//! Missing cartridge labels, prepared on a caller's worker thread. A ROM's checksum finds its
//! label in the artwork pack, when the card has one, then in an index of LaunchBox cart fronts
//! built into slot; only carts neither knows are searched for on LaunchBox by a cleaned
//! filename. No ROM data leaves the device.
mod artwork;
mod catalog;
pub mod cheats;
mod pack;
mod source;

/// What the examples need to build the LaunchBox index and the artwork pack the way the device
/// reads them.
#[doc(hidden)]
pub mod build {
    pub use crate::artwork::cart_label_jpeg;
    pub use crate::pack::{write as write_pack, FILE as PACK};
    pub use crate::source::{normalize, platforms, rank, title, PHOTO, RENDER};
}

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use slot_store::{Cart, Platform};

#[derive(Debug)]
pub enum Error {
    Network(String),
    Unavailable(String),
    Image(String),
    Storage(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(s) | Self::Unavailable(s) | Self::Image(s) => f.write_str(s),
            Self::Storage(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Storage(e)
    }
}

/// The size a label is prepared at: the label well slot draws on that platform's cart.
pub fn label_size(platform: Platform) -> (u32, u32) {
    match platform {
        Platform::Gba => (264, 136),
        Platform::Gb => (220, 195),
        // The NES label is tall; the SNES one is the NTSC sticker, which the shelf crops to the
        // flatter PAL sticker when the cart takes that mould.
        Platform::Nes => (141, 228),
        Platform::Snes => (221, 109),
        Platform::Gbc => (223, 199),
    }
}

trait Transport {
    fn get(&mut self, url: &str) -> Result<Vec<u8>, Error>;
}

struct Http(ureq::Agent);
impl Transport for Http {
    fn get(&mut self, url: &str) -> Result<Vec<u8>, Error> {
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        let mut url = url.to_owned();
        // Gallery IDs redirect to canonical titles. Follow a bounded number of redirects,
        // validating every destination and sharing one timeout across the whole transfer.
        for _ in 0..3 {
            if !source::allowed_url(&url) {
                return Err(Error::Unavailable("unsupported artwork URL".into()));
            }
            let remaining = deadline
                .checked_duration_since(std::time::Instant::now())
                .ok_or_else(|| Error::Network("artwork request timed out".into()))?;
            let mut response = self
                .0
                .get(&url)
                .config()
                .timeout_global(Some(remaining))
                .build()
                .call()
                .map_err(|e| match e {
                    ureq::Error::StatusCode(code)
                        if (400..500).contains(&code) && code != 408 && code != 429 =>
                    {
                        Error::Unavailable(e.to_string())
                    }
                    _ => Error::Network(e.to_string()),
                })?;
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get("location")
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| Error::Unavailable("artwork redirect has no location".into()))?;
                url = if location.starts_with('/') && !location.starts_with("//") {
                    let host = if url.starts_with("https://images.launchbox-app.com/") {
                        "https://images.launchbox-app.com"
                    } else {
                        "https://gamesdb.launchbox-app.com"
                    };
                    format!("{host}{location}")
                } else {
                    location.to_owned()
                };
                continue;
            }
            return response
                .body_mut()
                .with_config()
                .limit(16 * 1024 * 1024)
                .read_to_vec()
                .map_err(|e| Error::Network(e.to_string()));
        }
        Err(Error::Unavailable("too many artwork redirects".into()))
    }
}

pub struct Downloader {
    http: Http,
    // Cache resolved source URLs, not decoded images. A failed image transfer or SD write
    // should retry that image without repeating title searches and gallery downloads.
    sources: HashMap<String, Vec<String>>,
}

impl Default for Downloader {
    fn default() -> Self {
        Self {
            http: Http(
                ureq::Agent::config_builder()
                    .timeout_global(Some(Duration::from_secs(15)))
                    .max_redirects(0)
                    .user_agent(concat!(
                        "slot/",
                        env!("CARGO_PKG_VERSION"),
                        " cartridge-labels"
                    ))
                    .build()
                    .into(),
            ),
            sources: HashMap::new(),
        }
    }
}

/// One version of a cart's label: a cart front LaunchBox holds, cut to the label.
pub struct Version {
    /// A fanart render of the cart rather than a photo of one.
    pub render: bool,
    /// LaunchBox's name for the region, empty when it gives none.
    pub region: String,
    pub png: Vec<u8>,
}

/// At most this many versions are fetched for one cart.
const VERSIONS: usize = 8;

impl Downloader {
    /// Every version of the cart's label LaunchBox holds, best first. Blocking; call only on a
    /// background worker.
    pub fn versions(&mut self, cart: &Cart) -> Result<Vec<Version>, Error> {
        versions(&mut self.http, cart)
    }

    /// Blocking; call only on a background worker. Existing files are never replaced.
    pub fn prepare(&mut self, root: &Path, cart: &Cart) -> Result<PathBuf, Error> {
        self.prepare_identified(root, cart, None)
    }

    pub fn prepare_identified(
        &mut self,
        root: &Path,
        cart: &Cart,
        canonical_title: Option<&str>,
    ) -> Result<PathBuf, Error> {
        prepare(
            &mut self.http,
            &mut self.sources,
            catalog::INDEX,
            root,
            cart,
            canonical_title,
        )
    }
}

fn prepare(
    http: &mut impl Transport,
    sources: &mut HashMap<String, Vec<String>>,
    index: &str,
    root: &Path,
    cart: &Cart,
    canonical_title: Option<&str>,
) -> Result<PathBuf, Error> {
    // Use the actual filename, retaining every tag and punctuation mark.
    let filename = cart
        .rom
        .file_name()
        .ok_or_else(|| Error::Unavailable("missing ROM filename".into()))?;
    let target = root
        .join("Labels")
        .join(cart.platform.dir_name())
        .join(Path::new(filename).with_extension("png"));
    if target.try_exists()? {
        return Ok(target);
    }
    let key = format!("{}/{}", cart.platform.dir_name(), cart.stem);
    let urls = match sources.get(&key) {
        Some(urls) => urls.clone(),
        None => {
            let crc = catalog::crc32(&cart.rom).ok();
            if let Some(png) = crc.and_then(|crc| from_pack(root, crc, cart.platform)) {
                publish(&target, &png)?;
                return Ok(target);
            }
            let urls = match crc.map(|crc| catalog::lookup(index, crc)) {
                Some(urls) if !urls.is_empty() => urls,
                _ => source::resolve(http, cart, canonical_title)?,
            };
            sources.insert(key, urls.clone());
            urls
        }
    };
    // Candidates come best first. One that is gone or will not crop gives way to the next.
    let mut last = Error::Unavailable("no cartridge artwork".into());
    for url in &urls {
        match http
            .get(url)
            .and_then(|bytes| artwork::prepare(&bytes, cart.platform))
        {
            Ok(png) => {
                publish(&target, &png)?;
                return Ok(target);
            }
            Err(e @ (Error::Unavailable(_) | Error::Image(_))) => last = e,
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

fn versions(http: &mut impl Transport, cart: &Cart) -> Result<Vec<Version>, Error> {
    let fronts = match source::all_fronts(http, cart) {
        Ok(fronts) => fronts,
        // A title LaunchBox does not know may still have its checksum in slot's index.
        Err(Error::Unavailable(e)) => {
            let crc = catalog::crc32(&cart.rom).ok();
            let urls = crc.map(|crc| catalog::lookup(catalog::INDEX, crc));
            match urls.filter(|u| !u.is_empty()) {
                Some(urls) => urls.into_iter().map(|u| (false, String::new(), u)).collect(),
                None => return Err(Error::Unavailable(e)),
            }
        }
        Err(e) => return Err(e),
    };
    let mut found = Vec::new();
    let mut last = Error::Unavailable("no cartridge artwork".into());
    for (render, region, url) in fronts.into_iter().take(VERSIONS) {
        match http
            .get(&url)
            .and_then(|bytes| artwork::prepare(&bytes, cart.platform))
        {
            Ok(png) => found.push(Version { render, region, png }),
            Err(e @ Error::Network(_)) if found.is_empty() => return Err(e),
            Err(e) => last = e,
        }
    }
    if found.is_empty() {
        Err(last)
    } else {
        Ok(found)
    }
}

/// Where a cart's label is kept: the file it already wears, else one named after its ROM.
pub fn label_path(root: &Path, cart: &Cart) -> PathBuf {
    cart.label.clone().unwrap_or_else(|| {
        let file = cart.rom.file_name().map(Path::new).unwrap_or(Path::new(&cart.stem));
        root.join("Labels")
            .join(cart.platform.dir_name())
            .join(file.with_extension("png"))
    })
}

/// A label picked by hand: it replaces the one the cart wears.
pub fn save_label(root: &Path, cart: &Cart, png: &[u8]) -> Result<PathBuf, Error> {
    let target = label_path(root, cart);
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir)?;
    }
    slot_store::atomic_write(&target, png)?;
    Ok(target)
}

/// The pack's label for a ROM. A damaged pack or image is reported and passed over, so the cart
/// is still labelled online.
fn from_pack(root: &Path, crc: u32, platform: Platform) -> Option<Vec<u8>> {
    let path = root.join("Labels").join(pack::FILE);
    let label = pack::find(&path, crc)
        .map_err(Error::from)
        .and_then(|image| image.map(|i| artwork::fit(&i, platform)).transpose());
    label.unwrap_or_else(|e| {
        eprintln!("slot: labels: {}: {e}", path.display());
        None
    })
}

fn publish(target: &Path, bytes: &[u8]) -> Result<(), Error> {
    let dir = target
        .parent()
        .ok_or_else(|| Error::Unavailable("missing label directory".into()))?;
    std::fs::create_dir_all(dir)?;
    // The complete file appears in one operation. A custom label copied in during the
    // download wins, including on FAT/exFAT where hard-link based installation cannot work.
    let mut tmp = tempfile::Builder::new()
        .prefix(".slot-label-")
        .tempfile_in(dir)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    match tmp.persist_noclobber(target) {
        Ok(_) => {
            if let Ok(dir) = std::fs::File::open(dir) {
                let _ = dir.sync_all();
            }
            Ok(())
        }
        Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        // Older vendor exFAT drivers reject RENAME_NOREPLACE and hard links. Their
        // ordinary rename is atomic; the single label worker checks again immediately
        // before publishing so an already present custom label is preserved.
        Err(e) if matches!(e.error.raw_os_error(), Some(1 | 22 | 38 | 95)) => {
            publish_legacy(e.file, target)
        }
        Err(e) => Err(Error::Storage(e.error)),
    }
}

fn publish_legacy(tmp: tempfile::NamedTempFile, target: &Path) -> Result<(), Error> {
    if target.try_exists()? {
        return Ok(());
    }
    tmp.persist(target).map_err(|e| Error::Storage(e.error))?;
    if let Some(dir) = target
        .parent()
        .and_then(|dir| std::fs::File::open(dir).ok())
    {
        let _ = dir.sync_all();
    }
    Ok(())
}

#[cfg(test)]
mod tests;
