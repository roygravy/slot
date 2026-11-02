use std::io::Read;
use std::path::{Path, PathBuf};

use slot_store::{atomic_write, write_slot_state, Core, Platform, SlotState, StateRing};

pub trait Snapshot {
    fn state(&self) -> Option<Vec<u8>>;
    fn save_ram(&self) -> Option<Vec<u8>>;
    fn thumb(&self) -> Option<Vec<u8>>;
    fn load(&self, state: Vec<u8>);

    fn resume_trusted(&self) -> bool {
        true
    }

    fn save_ram_trusted(&self) -> bool {
        true
    }
}

pub fn flush(
    root: &Path,
    platform: Platform,
    core: Core,
    stem: &str,
    state: Option<&[u8]>,
    sav: Option<&[u8]>,
) -> std::io::Result<()> {
    if let Some(state) = state {
        let ring = StateRing::new(root, platform, core, stem);
        if !matches!(ring.read_resume(), Ok(Some(old)) if old == state) {
            ring.write_resume(state)?;
        }
    }
    if let Some(sav) = sav {
        write_sav(root, platform, stem, sav)?;
    }
    Ok(())
}

pub fn eject(
    root: &Path,
    platform: Platform,
    core: Core,
    stem: &str,
    state: Option<&[u8]>,
    sav: Option<&[u8]>,
    slot: &SlotState,
) -> std::io::Result<()> {
    flush(root, platform, core, stem, state, sav)?;
    let mut slot = slot.clone();
    slot.cart = None;
    slot.cart_platform = None;
    write_slot_state(root, &slot)
}

pub fn write_sav(root: &Path, platform: Platform, stem: &str, sav: &[u8]) -> std::io::Result<bool> {
    let path = sav_path(root, platform, stem);
    if let Ok(Some(old)) = read_sav(root, platform, stem) {
        if old == sav {
            return Ok(false);
        }
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    atomic_write(&path, sav)?;
    Ok(true)
}

pub fn read_sav(root: &Path, platform: Platform, stem: &str) -> std::io::Result<Option<Vec<u8>>> {
    for path in sav_files(root, platform, stem) {
        match std::fs::read(&path) {
            // A save RetroArch compressed is read, not taken for a blank one.
            Ok(bytes) if bytes.starts_with(RZIP_MAGIC) => return unrzip(&bytes).map(Some),
            Ok(bytes) => return Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(None)
}

const RZIP_MAGIC: &[u8] = b"#RZIPv\x01#";

fn unrzip(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    let bad = || std::io::Error::new(std::io::ErrorKind::InvalidData, "bad rzip save");
    let total = bytes.get(12..20).ok_or_else(bad)?;
    let total = u64::from_le_bytes(total.try_into().map_err(|_| bad())?);
    let mut rest = &bytes[20..];
    let mut out = Vec::new();
    while let Some((len, body)) = rest.split_first_chunk::<4>() {
        let len = u32::from_le_bytes(*len) as usize;
        let chunk = body.get(..len).ok_or_else(bad)?;
        flate2::read::ZlibDecoder::new(chunk).read_to_end(&mut out)?;
        rest = &body[len..];
    }
    if out.len() as u64 == total {
        Ok(out)
    } else {
        Err(bad())
    }
}

/// How much newer the battery save may be than the resume state before the resume is
/// taken for an old one. Both are written together, the save just after; the slack covers a
/// slow card and FAT's two-second clock.
const RESUME_SLACK: std::time::Duration = std::time::Duration::from_secs(60);

pub fn read_resume(
    root: &Path,
    platform: Platform,
    core: Core,
    stem: &str,
) -> std::io::Result<Option<Vec<u8>>> {
    let ring = StateRing::new(root, platform, core, stem);
    // A resume state carries the battery save as it was then. One older than the save was
    // left by another core (the Super Game Boy's mGBA, or a core picked since) or by an eject
    // that kept no state: loading it would bring back the old save and write it over the
    // newer one. The game starts from its save instead.
    if resume_outdated(&ring.resume_path(), &sav_files(root, platform, stem)) {
        eprintln!(
            "slot: resume: {stem}: its save is newer than its {} resume, starting from the save",
            core.as_str()
        );
        return Ok(None);
    }
    ring.read_resume()
}

fn resume_outdated(resume: &Path, savs: &[PathBuf]) -> bool {
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let Some(resume) = modified(resume) else {
        return false;
    };
    savs.iter()
        .find_map(|p| modified(p))
        .and_then(|sav| sav.duration_since(resume).ok())
        .is_some_and(|newer| newer > RESUME_SLACK)
}

/// The battery save's files, in the order `read_sav` tries them.
fn sav_files(root: &Path, platform: Platform, stem: &str) -> [PathBuf; 2] {
    [
        sav_path(root, platform, stem),
        crate::root::saves_dir(root)
            .join(platform.dir_name())
            .join(format!("{stem}.srm")),
    ]
}

fn sav_path(root: &Path, platform: Platform, stem: &str) -> PathBuf {
    crate::root::saves_dir(root)
        .join(platform.dir_name())
        .join(format!("{stem}.sav"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn age(path: &Path, secs: u64) {
        let f = std::fs::File::options().write(true).open(path).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(secs))
            .unwrap();
    }

    #[test]
    fn a_resume_older_than_the_save_is_not_loaded_over_it() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        let ring = StateRing::new(root, Platform::Gb, Core::Gambatte, "Game");
        ring.write_resume(b"old game").unwrap();
        write_sav(root, Platform::Gb, "Game", b"save").unwrap();
        // Written together, as a flush writes them: the resume stands.
        assert_eq!(
            read_resume(root, Platform::Gb, Core::Gambatte, "Game").unwrap(),
            Some(b"old game".to_vec())
        );
        // Saved an hour later by another core: the old resume would bring back the old save.
        age(&ring.resume_path(), 3600);
        assert_eq!(
            read_resume(root, Platform::Gb, Core::Gambatte, "Game").unwrap(),
            None
        );
        // The other core's own resume is as new as the save, and stands.
        let mgba = StateRing::new(root, Platform::Gb, Core::Mgba, "Game");
        mgba.write_resume(b"new game").unwrap();
        assert_eq!(
            read_resume(root, Platform::Gb, Core::Mgba, "Game").unwrap(),
            Some(b"new game".to_vec())
        );
    }

    #[test]
    fn a_game_without_a_save_resumes_as_before() {
        let d = tempfile::tempdir().unwrap();
        let ring = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Game");
        ring.write_resume(b"state").unwrap();
        age(&ring.resume_path(), 3600);
        assert_eq!(
            read_resume(d.path(), Platform::Gba, Core::Mgba, "Game").unwrap(),
            Some(b"state".to_vec())
        );
    }
}
