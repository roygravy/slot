//! Firmware updates from the GRAVY Manager: the newest GitHub release, or a zip sent from the
//! page, is unpacked beside System as System.new. launch_frontend.sh swaps it in on the next
//! start and keeps the old System to fall back on.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

const LATEST: &str = "https://api.github.com/repos/roygravy/slot/releases/latest";
const PREFIX: &str = "slot-x-gravy-";
pub const MAX_ZIP: u64 = 256 << 20;
/// Left beside System.new once every file is out of the zip; the start script only swaps a
/// System.new that carries it.
const COMPLETE: &str = ".complete";

#[derive(Clone)]
pub enum Status {
    Idle,
    Checking,
    Latest(String),
    Available { version: String, url: String },
    Downloading { version: String, done: u64, total: u64 },
    Ready(String),
    Failed(String),
}

pub struct Update {
    status: Mutex<Status>,
}

/// One for the daemon: a download carries on when the page is closed and opened again.
pub static UPDATE: Update = Update {
    status: Mutex::new(Status::Idle),
};

/// Left in System.new by Install on the page; the console sees it and restarts.
pub const INSTALL: &str = "System.new/.install";

impl Update {
    fn set(&self, s: Status) {
        *self.status.lock().unwrap() = s;
    }

    fn get(&self) -> Status {
        self.status.lock().unwrap().clone()
    }

    fn busy(&self) -> bool {
        matches!(self.get(), Status::Checking | Status::Downloading { .. })
    }

    pub fn json(&self, root: &Path) -> String {
        let installed = installed(root);
        let (state, version, done, total, error) = match self.get() {
            _ if ready(root).is_some() && !self.busy() => {
                ("ready", ready(root).unwrap_or_default(), 0, 0, String::new())
            }
            Status::Idle => ("idle", String::new(), 0, 0, String::new()),
            Status::Checking => ("checking", String::new(), 0, 0, String::new()),
            Status::Latest(v) => ("latest", v, 0, 0, String::new()),
            Status::Available { version, .. } => ("available", version, 0, 0, String::new()),
            Status::Downloading {
                version,
                done,
                total,
            } => ("downloading", version, done, total, String::new()),
            Status::Ready(v) => ("ready", v, 0, 0, String::new()),
            Status::Failed(e) => ("failed", String::new(), 0, 0, e),
        };
        format!(
            "{{\"installed\":{},\"state\":\"{state}\",\"version\":{},\"done\":{done},\"total\":{total},\"error\":{}}}",
            crate::web::quote(&installed),
            crate::web::quote(&version),
            crate::web::quote(&error),
        )
    }

    /// Asks GitHub for the newest release, on its own thread.
    pub fn check(&'static self, root: &Path) -> Result<(), &'static str> {
        if self.busy() {
            return Err("Already busy with an update");
        }
        self.set(Status::Checking);
        let installed = installed(root);
        std::thread::spawn(move || {
            self.set(match newest() {
                Ok((version, _)) if !newer(&version, &installed) => Status::Latest(installed.clone()),
                Ok((version, url)) => Status::Available { version, url },
                Err(e) => Status::Failed(e.into()),
            });
        });
        Ok(())
    }

    /// Fetches the release found by `check` and unpacks it, on its own thread.
    pub fn download(&'static self, root: &Path) -> Result<(), &'static str> {
        let Status::Available { version, url } = self.get() else {
            return Err("Check for an update first");
        };
        let root = root.to_path_buf();
        self.set(Status::Downloading {
            version: version.clone(),
            done: 0,
            total: 0,
        });
        std::thread::spawn(move || {
            let result = fetch(&url, &root, |done, total| {
                self.set(Status::Downloading {
                    version: version.clone(),
                    done,
                    total,
                })
            })
            .and_then(|zip| {
                let r = unpack(&zip, &root);
                let _ = fs::remove_file(&zip);
                r
            });
            self.set(match result {
                Ok(v) => Status::Ready(v),
                Err(e) => Status::Failed(e.into()),
            });
        });
        Ok(())
    }

    pub fn install(&self, root: &Path) -> Result<(), &'static str> {
        if ready(root).is_none() {
            return Err("No update is ready");
        }
        fs::write(root.join(INSTALL), b"").map_err(|_| "The card can't be written")
    }

    /// A release zip sent from the page.
    pub fn upload(&self, root: &Path, length: u64, body: &mut impl Read) -> Result<(), &'static str> {
        if self.busy() {
            return Err("Already busy with an update");
        }
        if length == 0 || length > MAX_ZIP {
            return Err("That is not a slot. x GRAVY zip");
        }
        if length * 3 > crate::web::free(root) {
            return Err("The card is full");
        }
        let zip = root.join(".update.zip");
        let written = (|| -> std::io::Result<()> {
            let mut f = File::create(&zip)?;
            if std::io::copy(&mut body.take(length), &mut f)? != length {
                return Err(std::io::ErrorKind::UnexpectedEof.into());
            }
            f.sync_all()
        })();
        let result = match written {
            Ok(()) => unpack(&zip, root),
            Err(_) => Err("The upload stopped halfway"),
        };
        let _ = fs::remove_file(&zip);
        self.set(match &result {
            Ok(v) => Status::Ready(v.clone()),
            Err(e) => Status::Failed((*e).into()),
        });
        result.map(|_| ())
    }
}

/// v1.2 is newer than v1.1.2; any release is newer than a System that doesn't know its own.
fn newer(release: &str, installed: &str) -> bool {
    let parts = |v: &str| -> Option<Vec<u32>> {
        v.trim_start_matches('v').split('.').map(|n| n.parse().ok()).collect()
    };
    match (parts(release), parts(installed)) {
        (Some(r), Some(i)) => r > i,
        (Some(_), None) => true,
        _ => false,
    }
}

/// The version the running System came with, from the release zip's name.
pub fn installed(root: &Path) -> String {
    version_in(&root.join("System"))
}

fn version_in(system: &Path) -> String {
    fs::read_to_string(system.join("version.txt"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// The version waiting in System.new, once it is whole.
pub fn ready(root: &Path) -> Option<String> {
    let new = root.join("System.new");
    new.join(COMPLETE).is_file().then(|| version_in(&new))
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .user_agent(concat!("slot/", env!("CARGO_PKG_VERSION"), " update"))
        .build()
        .into()
}

/// The newest release's version and the address of its slot-x-gravy zip.
fn newest() -> Result<(String, String), &'static str> {
    let body = agent()
        .get(LATEST)
        .header("Accept", "application/vnd.github+json")
        .config()
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .call()
        .map_err(|_| "GitHub can't be reached")?
        .body_mut()
        .with_config()
        .limit(1 << 20)
        .read_to_string()
        .map_err(|_| "GitHub can't be reached")?;
    release_zip(&body).ok_or("The newest release has no slot. x GRAVY zip")
}

fn release_zip(json: &str) -> Option<(String, String)> {
    strings(json, "browser_download_url").into_iter().find_map(|url| {
        let file = url.rsplit('/').next()?;
        let version = file.strip_prefix(PREFIX)?.strip_suffix(".zip")?;
        Some((version.to_string(), url))
    })
}

/// Every string value of `key` in a JSON text, without a JSON parser for one field.
fn strings(json: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    json.match_indices(&needle)
        .filter_map(|(at, _)| {
            let rest = json[at + needle.len()..].trim_start().strip_prefix(':')?;
            let rest = rest.trim_start().strip_prefix('"')?;
            let end = rest.find('"')?;
            Some(rest[..end].replace("\\/", "/"))
        })
        .collect()
}

fn fetch(url: &str, root: &Path, mut progress: impl FnMut(u64, u64)) -> Result<PathBuf, &'static str> {
    let mut response = agent()
        .get(url)
        .call()
        .map_err(|_| "The download didn't start")?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok()?.parse().ok())
        .unwrap_or(0u64);
    if total > MAX_ZIP {
        return Err("That release is too big");
    }
    if total * 3 > crate::web::free(root) {
        return Err("The card is full");
    }
    let zip = root.join(".update.zip");
    let written = (|| -> std::io::Result<()> {
        let mut f = File::create(&zip)?;
        let mut body = response.body_mut().with_config().limit(MAX_ZIP).reader();
        let mut buf = vec![0u8; 64 << 10];
        let mut done = 0u64;
        loop {
            let n = body.read(&mut buf)?;
            if n == 0 {
                break;
            }
            f.write_all(&buf[..n])?;
            done += n as u64;
            progress(done, total);
        }
        if total > 0 && done != total {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        f.sync_all()
    })();
    if written.is_err() {
        let _ = fs::remove_file(&zip);
        return Err("The download stopped halfway");
    }
    Ok(zip)
}

/// Unpacks the release's System folder into System.new and marks it whole. Only System is
/// taken: games, saves and settings on the card stay as they are.
fn unpack(zip: &Path, root: &Path) -> Result<String, &'static str> {
    let bad = "That is not a slot. x GRAVY zip";
    let mut archive = zip::ZipArchive::new(File::open(zip).map_err(|_| bad)?).map_err(|_| bad)?;
    // A zip from GitHub Actions wraps the release zip in one of its own: unpack the inner one.
    if archive.len() == 1 {
        let mut entry = archive.by_index(0).map_err(|_| bad)?;
        if entry.name().is_ok_and(|n| n.to_ascii_lowercase().ends_with(".zip")) {
            let inner = root.join(".update-inner.zip");
            let copied = File::create(&inner).and_then(|mut f| std::io::copy(&mut entry, &mut f));
            let result = match copied {
                Ok(_) => unpack(&inner, root),
                Err(_) => Err("The card can't be written"),
            };
            let _ = fs::remove_file(&inner);
            return result;
        }
    }
    let new = root.join("System.new");
    let _ = fs::remove_dir_all(&new);
    let mut has_slot = false;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|_| bad)?;
        let Some(path) = entry.enclosed_name() else {
            continue;
        };
        // slot-x-gravy-v1.1.2/System/slot, or System/slot when zipped without the folder.
        let mut parts = path.components().map(|c| c.as_os_str().to_string_lossy().into_owned());
        let first = parts.next().unwrap_or_default();
        let rest: Vec<String> = if first == "System" {
            parts.collect()
        } else if parts.next().as_deref() == Some("System") {
            parts.collect()
        } else {
            continue;
        };
        if rest.is_empty() {
            continue;
        }
        let to = rest.iter().fold(new.clone(), |p, part| p.join(part));
        if entry.is_dir() {
            fs::create_dir_all(&to).map_err(|_| "The card can't be written")?;
            continue;
        }
        has_slot |= rest == ["slot"];
        if let Some(dir) = to.parent() {
            fs::create_dir_all(dir).map_err(|_| "The card can't be written")?;
        }
        let copied = File::create(&to).and_then(|mut f| {
            std::io::copy(&mut entry, &mut f)?;
            f.sync_all()
        });
        if copied.is_err() {
            let _ = fs::remove_dir_all(&new);
            return Err("The card can't be written");
        }
    }
    if !has_slot {
        let _ = fs::remove_dir_all(&new);
        return Err(bad);
    }
    fs::write(new.join(COMPLETE), b"").map_err(|_| "The card can't be written")?;
    Ok(version_in(&new))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_newer_release_is_offered() {
        assert!(newer("v1.2", "v1.1.2"));
        assert!(newer("v1.1.10", "v1.1.9"));
        assert!(!newer("v1.1.1", "v1.1.2"));
        assert!(!newer("v1.1.2", "v1.1.2"));
        assert!(newer("v1.1.1", ""));
    }

    #[test]
    fn finds_the_zip_among_the_assets() {
        let json = r#"{"tag_name":"slot-x-gravy-v1.1","assets":[{"name":"notes.txt","browser_download_url":"https://github.com/roygravy/slot/releases/download/t/notes.txt"},{"name":"z","browser_download_url": "https://github.com/roygravy/slot/releases/download/slot-x-gravy-v1.1/slot-x-gravy-v1.1.zip"}]}"#;
        assert_eq!(
            release_zip(json),
            Some((
                "v1.1".into(),
                "https://github.com/roygravy/slot/releases/download/slot-x-gravy-v1.1/slot-x-gravy-v1.1.zip".into()
            ))
        );
    }

    #[test]
    fn unpacks_only_system_and_marks_it_whole() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        let zip = r.join("u.zip");
        let mut w = zip::ZipWriter::new(File::create(&zip).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        for (name, body) in [
            ("slot-x-gravy-v1.2/System/slot", "bin"),
            ("slot-x-gravy-v1.2/System/version.txt", "v1.2\n"),
            ("slot-x-gravy-v1.2/System/licenses/a.txt", "a"),
            ("slot-x-gravy-v1.2/Config/wifi.toml.example", "x"),
        ] {
            w.start_file(name, o).unwrap();
            w.write_all(body.as_bytes()).unwrap();
        }
        w.finish().unwrap();
        assert_eq!(unpack(&zip, r).unwrap(), "v1.2");
        assert_eq!(fs::read_to_string(r.join("System.new/licenses/a.txt")).unwrap(), "a");
        assert!(!r.join("System.new/wifi.toml.example").exists());
        assert!(!r.join("Config").exists());
        assert_eq!(ready(r).as_deref(), Some("v1.2"));
    }

    #[test]
    fn a_zip_inside_a_zip_is_opened() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        let mut inner = std::io::Cursor::new(Vec::new());
        let mut w = zip::ZipWriter::new(&mut inner);
        let o = zip::write::SimpleFileOptions::default();
        w.start_file("slot-x-gravy-v1.2/System/slot", o).unwrap();
        w.write_all(b"bin").unwrap();
        w.start_file("slot-x-gravy-v1.2/System/version.txt", o).unwrap();
        w.write_all(b"v1.2").unwrap();
        w.finish().unwrap();
        let zip = r.join("outer.zip");
        let mut w = zip::ZipWriter::new(File::create(&zip).unwrap());
        w.start_file("slot-x-gravy-v1.2.zip", o).unwrap();
        w.write_all(inner.get_ref()).unwrap();
        w.finish().unwrap();
        assert_eq!(unpack(&zip, r).unwrap(), "v1.2");
        assert!(!r.join(".update-inner.zip").exists());
    }

    #[test]
    fn a_zip_without_slot_is_refused() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        let zip = r.join("u.zip");
        let mut w = zip::ZipWriter::new(File::create(&zip).unwrap());
        w.start_file("game.gba", zip::write::SimpleFileOptions::default()).unwrap();
        w.finish().unwrap();
        assert!(unpack(&zip, r).is_err());
        assert!(ready(r).is_none());
        assert!(!r.join("System.new").exists());
    }
}
