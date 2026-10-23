//! The marker a blank cart's title is written in, and the carts known to have no label to
//! fetch, which are the ones offered a marker.
use std::collections::HashMap;
use std::path::Path;

use crate::cart_shell::key;
use crate::Platform;

pub const CART_MARKER_FILE: &str = "Config/cart_marker.txt";
pub const NO_LABEL_FILE: &str = "System/Cache/Labels/none.txt";
/// The carts whose label the player took off, kept with the labels rather than the cache so
/// clearing the cache does not bring them back.
pub const REMOVED_LABEL_FILE: &str = "Labels/removed.txt";

pub fn parse_marker(value: &str) -> Option<[u8; 3]> {
    let hex = value.trim();
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some([0, 2, 4].map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0)))
}

pub fn markers(text: &str) -> HashMap<String, [u8; 3]> {
    crate::ini::parse(text)
        .into_iter()
        .filter_map(|(stem, value)| Some((key(&stem), parse_marker(&value)?)))
        .collect()
}

/// A marker picked on the shelf; None hands the cart back to the one its shell suits.
pub fn write_marker(root: &Path, stem: &str, ink: Option<[u8; 3]>) -> std::io::Result<()> {
    let path = root.join(CART_MARKER_FILE);
    let mut text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let wanted = key(stem);
    for other in crate::ini::parse(&text).into_keys() {
        if other != stem && key(&other) == wanted {
            text = crate::ini::set(&text, &other, None)?;
        }
    }
    let value = ink.map(|[r, g, b]| format!("{r:02x}{g:02x}{b:02x}"));
    let text = crate::ini::set(&text, stem, value.as_deref())?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    crate::atomic::atomic_write(&path, text.as_bytes())
}

fn no_label_line(platform: Platform, stem: &str) -> String {
    format!("{}/{}", platform.dir_name(), key(stem))
}

fn listed(root: &Path, file: &str, platform: Platform, stem: &str) -> bool {
    let line = no_label_line(platform, stem);
    std::fs::read_to_string(root.join(file)).is_ok_and(|text| text.lines().any(|l| l == line))
}

fn unlist(root: &Path, file: &str, platform: Platform, stem: &str) {
    if !listed(root, file, platform, stem) {
        return;
    }
    let path = root.join(file);
    let line = no_label_line(platform, stem);
    let text: String = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .lines()
        .filter(|l| *l != line)
        .map(|l| format!("{l}\n"))
        .collect();
    if let Err(e) = crate::atomic::atomic_write(&path, text.as_bytes()) {
        eprintln!("slot: labels: {}: {e}", path.display());
    }
}

fn list(root: &Path, file: &str, platform: Platform, stem: &str) {
    if listed(root, file, platform, stem) {
        return;
    }
    let path = root.join(file);
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    text.push_str(&no_label_line(platform, stem));
    text.push('\n');
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = crate::atomic::atomic_write(&path, text.as_bytes()) {
        eprintln!("slot: labels: {}: {e}", path.display());
    }
}

/// Whether a label was looked for and none exists for this cart.
pub fn has_no_label(root: &Path, platform: Platform, stem: &str) -> bool {
    listed(root, NO_LABEL_FILE, platform, stem)
}

/// A label turned up after all: the cart is no longer noted as having none.
pub fn forget_no_label(root: &Path, platform: Platform, stem: &str) {
    unlist(root, NO_LABEL_FILE, platform, stem)
}

pub fn note_no_label(root: &Path, platform: Platform, stem: &str) {
    list(root, NO_LABEL_FILE, platform, stem)
}

/// Whether the player took this cart's label off: none is fetched for it again.
pub fn label_removed(root: &Path, platform: Platform, stem: &str) -> bool {
    listed(root, REMOVED_LABEL_FILE, platform, stem)
}

pub fn note_label_removed(root: &Path, platform: Platform, stem: &str) {
    list(root, REMOVED_LABEL_FILE, platform, stem)
}

/// A label was picked for the cart again.
pub fn forget_label_removed(root: &Path, platform: Platform, stem: &str) {
    unlist(root, REMOVED_LABEL_FILE, platform, stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marker_is_kept_per_cart_and_none_takes_it_away() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_marker(root, "Homebrew", Some([0xc9, 0xa2, 0x27])).unwrap();
        write_marker(root, "Other", Some([0xf2, 0xf0, 0xea])).unwrap();
        let text = std::fs::read_to_string(root.join(CART_MARKER_FILE)).unwrap();
        assert_eq!(markers(&text)["Homebrew"], [0xc9, 0xa2, 0x27]);
        write_marker(root, "Homebrew", None).unwrap();
        let text = std::fs::read_to_string(root.join(CART_MARKER_FILE)).unwrap();
        assert!(!markers(&text).contains_key("Homebrew"));
        assert!(markers(&text).contains_key("Other"));
    }

    #[test]
    fn a_cart_with_no_label_is_noted_once() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        assert!(!has_no_label(root, Platform::Gb, "Homebrew"));
        note_no_label(root, Platform::Gb, "Homebrew");
        note_no_label(root, Platform::Gb, "Homebrew");
        assert!(has_no_label(root, Platform::Gb, "Homebrew"));
        assert!(!has_no_label(root, Platform::Gbc, "Homebrew"));
        let text = std::fs::read_to_string(root.join(NO_LABEL_FILE)).unwrap();
        assert_eq!(text.lines().count(), 1);
    }

    #[test]
    fn a_removed_label_is_remembered_until_one_is_picked_again() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        note_label_removed(root, Platform::Snes, "Game");
        assert!(label_removed(root, Platform::Snes, "Game"));
        assert!(!has_no_label(root, Platform::Snes, "Game"));
        forget_label_removed(root, Platform::Snes, "Game");
        assert!(!label_removed(root, Platform::Snes, "Game"));
    }
}
