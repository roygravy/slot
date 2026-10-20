//! Cheats for the game being played, read from RetroArch `.cht` files.
//!
//! A game's cheats live in `Cheats/<GB|GBC|GBA>/`, named like its ROM, often one file per code
//! kind: `Pokemon - Emerald Version (USA, Europe) (Code Breaker).cht`. Every file that belongs
//! to the ROM is read and merged into one list, the first copy of each name winning. Which
//! cheats are on is kept per game in `Config/Cheats/<GB|GBC|GBA>/<rom>.txt`, one name a line.

use std::path::{Path, PathBuf};

use slot_store::Platform;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cheat {
    pub name: String,
    pub code: String,
    pub on: bool,
}

fn cheat_dir(root: &Path, platform: Platform) -> PathBuf {
    root.join("Cheats").join(platform.dir_name())
}

fn state_path(root: &Path, platform: Platform, stem: &str) -> PathBuf {
    root.join("Config")
        .join("Cheats")
        .join(platform.dir_name())
        .join(format!("{stem}.txt"))
}

/// The title before the first bracket: "Pokemon - Emerald Version (USA, Europe)" reads as
/// "pokemon - emerald version", so a ROM finds its cheats whatever its region tags say.
fn title(name: &str) -> String {
    name.split(" (")
        .next()
        .unwrap_or(name)
        .split(" [")
        .next()
        .unwrap_or(name)
        .trim()
        .to_lowercase()
}

/// The `.cht` files for a ROM: those named exactly like it (with or without a trailing code
/// kind in brackets), or, when there are none, those sharing its title.
fn files_for(dir: &Path, stem: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.to_lowercase().ends_with(".cht") && !n.starts_with('.'))
        .collect();
    names.sort();
    let lower = stem.to_lowercase();
    let exact: Vec<&String> = names
        .iter()
        .filter(|n| {
            let base = n[..n.len() - 4].to_lowercase();
            base == lower || base.starts_with(&format!("{lower} ("))
        })
        .collect();
    let picked: Vec<&String> = if exact.is_empty() {
        let t = title(stem);
        names.iter().filter(|n| title(&n[..n.len() - 4]) == t).collect()
    } else {
        exact
    };
    picked.into_iter().map(|n| dir.join(n)).collect()
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    let v = v.strip_prefix('"').unwrap_or(v);
    let v = v.strip_suffix('"').unwrap_or(v);
    v.to_string()
}

/// The cheats in one `.cht` file, in its order.
pub fn parse(text: &str) -> Vec<(String, String)> {
    let mut slots: Vec<(Option<String>, Option<String>)> = Vec::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let Some(rest) = key.strip_prefix("cheat") else {
            continue;
        };
        let Some((index, field)) = rest.split_once('_') else {
            continue;
        };
        let Ok(i) = index.parse::<usize>() else {
            continue;
        };
        if i > 4096 {
            continue;
        }
        if slots.len() <= i {
            slots.resize(i + 1, (None, None));
        }
        match field {
            "desc" => slots[i].0 = Some(unquote(value)),
            "code" => slots[i].1 = Some(unquote(value)),
            _ => {}
        }
    }
    slots
        .into_iter()
        .filter_map(|(d, c)| {
            let code = c?.trim().to_string();
            if code.is_empty() {
                return None;
            }
            let name = d.filter(|d| !d.trim().is_empty()).unwrap_or_else(|| code.clone());
            Some((name.trim().to_string(), code))
        })
        .collect()
}

/// Every cheat for the ROM, merged across its files, with the ones this game left on.
pub fn load(root: &Path, platform: Platform, stem: &str) -> Vec<Cheat> {
    let mut out: Vec<Cheat> = Vec::new();
    for path in files_for(&cheat_dir(root, platform), stem) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        for (name, code) in parse(&String::from_utf8_lossy(&bytes)) {
            if out.iter().any(|c| c.name.eq_ignore_ascii_case(&name)) {
                continue;
            }
            out.push(Cheat {
                name,
                code,
                on: false,
            });
        }
    }
    if let Ok(saved) = std::fs::read_to_string(state_path(root, platform, stem)) {
        for line in saved.lines().map(str::trim).filter(|l| !l.is_empty()) {
            if let Some(c) = out.iter_mut().find(|c| c.name == line) {
                c.on = true;
            }
        }
    }
    out
}

/// Remember which cheats are on for this game; with none on, the file goes.
pub fn save(root: &Path, platform: Platform, stem: &str, cheats: &[Cheat]) {
    let path = state_path(root, platform, stem);
    let on: Vec<&str> = cheats.iter().filter(|c| c.on).map(|c| c.name.as_str()).collect();
    if on.is_empty() {
        let _ = std::fs::remove_file(&path);
        return;
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut text = on.join("\n");
    text.push('\n');
    if let Err(e) = slot_store::atomic_write(&path, text.as_bytes()) {
        eprintln!("slot: cheats: could not save {}: {e}", path.display());
    }
}

/// The codes of the cheats that are on, for the core.
pub fn codes(cheats: &[Cheat]) -> Vec<String> {
    cheats.iter().filter(|c| c.on).map(|c| c.code.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_cht_file_and_skips_empty_codes() {
        let text = "cheats = 3\n\ncheat0_desc = \"Master Code\"\ncheat0_code = \"AAAA+BBBB\"\ncheat0_enable = false\n\ncheat1_desc = \"Nothing\"\ncheat1_code = \"\"\n\ncheat2_desc = \"Max Money\"\ncheat2_code = \"1234\"\n";
        assert_eq!(
            parse(text),
            vec![
                ("Master Code".to_string(), "AAAA+BBBB".to_string()),
                ("Max Money".to_string(), "1234".to_string())
            ]
        );
    }

    #[test]
    fn titles_ignore_region_and_kind() {
        assert_eq!(title("Pokemon - Emerald Version (USA, Europe) (Code Breaker)"), "pokemon - emerald version");
        assert_eq!(title("Tetris (World)"), "tetris");
    }
}
