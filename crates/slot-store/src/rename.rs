//! A game renamed, and everything kept under its name moved along: battery saves, save
//! states, label, the cart's colour and marker. Favorites and recents live in slot's own
//! state: a journal line tells slot.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::platform::Platform;

/// The journal slot follows on its next start or rescan: `dir \t old \t new` per line.
pub const RENAMED: &str = "Config/.renamed";

/// Moves what is kept under the game's stem `old` to `new`. The game file itself is the
/// caller's to rename, after this.
pub fn move_game(root: &Path, p: Platform, old: &str, new: &str) -> Result<(), &'static str> {
    if old == new {
        return Ok(());
    }
    let taken = |path: PathBuf| path.exists() && !old.eq_ignore_ascii_case(new);
    let saves = root.join("Saves").join(p.dir_name());
    let labels = root.join("Labels").join(p.dir_name());
    let states: Vec<PathBuf> = fs::read_dir(root.join("States").join(p.dir_name()))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    let clash = ["sav", "srm"]
        .iter()
        .any(|e| taken(saves.join(format!("{new}.{e}"))))
        || states.iter().any(|core| taken(core.join(new)));
    if clash {
        return Err("Saves for a game with that name are already there");
    }
    for ext in ["sav", "srm"] {
        move_if_there(&saves.join(format!("{old}.{ext}")), &saves.join(format!("{new}.{ext}")));
    }
    for core in &states {
        move_if_there(&core.join(old), &core.join(new));
    }
    if let Some(label) = crate::listing(&labels).get(&format!("{old}.png").to_lowercase()) {
        move_if_there(&labels.join(label), &labels.join(format!("{new}.png")));
    }
    // The cart's own colour and marker are kept by name too.
    for file in [crate::CART_SHELL_FILE, crate::LABELS_SHELL_FILE, crate::CART_MARKER_FILE] {
        if let Err(e) = rekey(&root.join(file), old, new) {
            eprintln!("slot-store: rename: {file}: {e}");
        }
    }
    let line = format!("{}\t{old}\t{new}\n", p.dir_name());
    let journal = root.join(RENAMED);
    if let Some(dir) = journal.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(journal)
        .and_then(|mut f| f.write_all(line.as_bytes()));
    Ok(())
}

/// A game's name ending in a space or a dot trips the card's file system (the red banner):
/// such games are renamed without it, everything kept under the name moving along
/// (upstream 1.15, Roy up4). Returns how many were renamed.
pub fn tidy_game_names(root: &Path) -> usize {
    let mut done = 0;
    for p in Platform::ALL {
        let dir = root.join("Games").join(p.dir_name());
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let path = e.path();
            let (Some(stem), ext) = (
                path.file_stem().and_then(|s| s.to_str()),
                path.extension().and_then(|s| s.to_str()),
            ) else {
                continue;
            };
            let clean = stem.trim_end_matches([' ', '.']);
            if clean == stem || clean.is_empty() || clean.starts_with('.') {
                continue;
            }
            let to = match ext {
                Some(ext) => format!("{clean}.{ext}"),
                None => clean.to_string(),
            };
            if dir.join(&to).exists() || move_game(root, p, stem, clean).is_err() {
                continue;
            }
            if fs::rename(&path, dir.join(&to)).is_ok() {
                done += 1;
            }
        }
    }
    done
}

/// `old`'s line in a `name = value` file, moved to `new`.
fn rekey(path: &Path, old: &str, new: &str) -> std::io::Result<()> {
    let Ok(text) = fs::read_to_string(path) else {
        return Ok(());
    };
    let wanted = crate::cart_shell::key(old);
    let Some((was, value)) = crate::ini::parse(&text)
        .into_iter()
        .find(|(k, _)| crate::cart_shell::key(k) == wanted)
    else {
        return Ok(());
    };
    let text = crate::ini::set(&text, &was, None)?;
    let text = crate::ini::set(&text, new, Some(&value))?;
    crate::atomic_write(path, text.as_bytes())
}

fn move_if_there(from: &Path, to: &Path) {
    if from.exists() {
        if let Err(e) = fs::rename(from, to) {
            eprintln!("slot-store: rename: {}: {e}", from.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_ending_in_a_space_loses_it_and_keeps_its_save() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        for (dir, file) in [("Games/GB", "Super Mario Land .gb"), ("Saves/GB", "Super Mario Land .sav")] {
            fs::create_dir_all(r.join(dir)).unwrap();
            fs::write(r.join(dir).join(file), b"x").unwrap();
        }
        assert_eq!(tidy_game_names(r), 1);
        assert!(r.join("Games/GB/Super Mario Land.gb").is_file());
        assert!(r.join("Saves/GB/Super Mario Land.sav").is_file());
        assert_eq!(fs::read_to_string(r.join(RENAMED)).unwrap(), "GB\tSuper Mario Land \tSuper Mario Land\n");
        assert_eq!(tidy_game_names(r), 0);
    }
}
