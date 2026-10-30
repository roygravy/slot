//! RetroArch cheat files for carts that have none, from the libretro database on GitHub. The
//! database's file list per platform is asked for once; each file is then fetched by name.
//! Only names leave the device, never ROM data.
use std::time::Duration;

use slot_store::Platform;

use crate::Error;

const API: &str = "https://api.github.com/repos/libretro/libretro-database";
const RAW: &str = "https://raw.githubusercontent.com/libretro/libretro-database/master/cht";

/// The database's folder for a platform.
pub fn folder(platform: Platform) -> &'static str {
    match platform {
        Platform::Gb => "Nintendo - Game Boy",
        Platform::Gbc => "Nintendo - Game Boy Color",
        Platform::Gba => "Nintendo - Game Boy Advance",
        Platform::Nes => "Nintendo - Nintendo Entertainment System",
        Platform::Snes => "Nintendo - Super Nintendo Entertainment System",
    }
}

pub struct Cheats {
    agent: ureq::Agent,
}

impl Default for Cheats {
    fn default() -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(20)))
                .max_redirects(0)
                .user_agent(concat!("slot/", env!("CARGO_PKG_VERSION"), " cheats"))
                .build()
                .into(),
        }
    }
}

impl Cheats {
    fn get(&self, url: &str) -> Result<Vec<u8>, Error> {
        if !url.starts_with(&format!("{API}/")) && !url.starts_with(&format!("{RAW}/")) {
            return Err(Error::Unavailable("unsupported cheat URL".into()));
        }
        let mut response = self.agent.get(url).call().map_err(|e| match e {
            // GitHub answers 403 when its hourly allowance for this address is spent: that
            // passes, like a dropped connection, so it is tried again later.
            ureq::Error::StatusCode(code)
                if (400..500).contains(&code) && ![403, 408, 429].contains(&code) =>
            {
                Error::Unavailable(e.to_string())
            }
            _ => Error::Network(e.to_string()),
        })?;
        response
            .body_mut()
            .with_config()
            .limit(32 * 1024 * 1024)
            .read_to_vec()
            .map_err(|e| Error::Network(e.to_string()))
    }

    /// The names of the database's cheat files for a platform. Blocking; call only on a
    /// background worker.
    pub fn index(&self, platform: Platform) -> Result<Vec<String>, Error> {
        let listing = self.get(&format!("{API}/contents/cht?ref=master"))?;
        let listing = String::from_utf8_lossy(&listing);
        let want = folder(platform);
        let mut name = None;
        let mut sha = None;
        for (key, value) in json_pairs(&listing) {
            match key.as_str() {
                "name" => name = Some(value),
                "sha" if name.as_deref() == Some(want) => {
                    sha = Some(value);
                    break;
                }
                _ => {}
            }
        }
        let sha = sha.ok_or_else(|| Error::Unavailable(format!("no {want} cheats")))?;
        let tree = self.get(&format!("{API}/git/trees/{sha}"))?;
        Ok(json_pairs(&String::from_utf8_lossy(&tree))
            .into_iter()
            .filter(|(k, v)| k == "path" && v.to_lowercase().ends_with(".cht"))
            .map(|(_, v)| v)
            .collect())
    }

    /// One cheat file from the database. Blocking.
    pub fn file(&self, platform: Platform, name: &str) -> Result<Vec<u8>, Error> {
        self.get(&format!(
            "{RAW}/{}/{}",
            encode(folder(platform)),
            encode(name)
        ))
    }
}

/// The title before the first bracket, as slot's cheat reader matches it: "Pokemon - Emerald
/// Version (USA, Europe)" reads as "pokemon - emerald version".
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

/// The files in an index that belong to a ROM: named exactly like it (with or without a code
/// kind in brackets after), or when there are none, those with its title, the English
/// releases first among them if there are any, as their codes are the likeliest to fit.
pub fn matches(index: &[String], stem: &str) -> Vec<String> {
    let lower = stem.to_lowercase();
    let base = |n: &String| n[..n.len().saturating_sub(4)].to_lowercase();
    let exact: Vec<String> = index
        .iter()
        .filter(|n| {
            let b = base(n);
            b == lower || b.starts_with(&format!("{lower} ("))
        })
        .cloned()
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    let t = title(stem);
    if t.is_empty() {
        return Vec::new();
    }
    let titled: Vec<String> = index
        .iter()
        .filter(|n| title(&base(n)) == t)
        .cloned()
        .collect();
    let english: Vec<String> = titled
        .iter()
        .filter(|n| {
            ["(usa", "(europe", "(world"]
                .iter()
                .any(|r| n.to_lowercase().contains(r))
        })
        .cloned()
        .collect();
    if english.is_empty() {
        titled
    } else {
        english
    }
}

/// A path segment for a raw URL: everything but plain characters escaped.
fn encode(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len() * 3 / 2);
    for b in segment.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~()!,'+&".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The string values of a JSON document, each with the key it stands under, in order. Enough
/// for GitHub's listings without a JSON library: values that are not strings are passed over.
fn json_pairs(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    let mut pending: Option<String> = None;
    let mut after_colon = false;
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                let s = json_string(&mut chars);
                while chars.peek().is_some_and(|c| c.is_whitespace()) {
                    chars.next();
                }
                if chars.peek() == Some(&':') {
                    chars.next();
                    pending = Some(s);
                    after_colon = true;
                } else {
                    if after_colon {
                        if let Some(key) = pending.take() {
                            out.push((key, s));
                        }
                    }
                    after_colon = false;
                }
            }
            c if c.is_whitespace() => {}
            _ => {
                pending = None;
                after_colon = false;
            }
        }
    }
    out
}

fn json_string(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut s = String::new();
    let mut high: Option<u32> = None;
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => match chars.next() {
                Some('n') => s.push('\n'),
                Some('t') => s.push('\t'),
                Some('r') => s.push('\r'),
                Some('b') => s.push('\u{8}'),
                Some('f') => s.push('\u{c}'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    let Ok(unit) = u32::from_str_radix(&hex, 16) else {
                        continue;
                    };
                    match (high.take(), unit) {
                        (None, 0xd800..=0xdbff) => high = Some(unit),
                        (Some(h), 0xdc00..=0xdfff) => {
                            let code = 0x10000 + ((h - 0xd800) << 10) + (unit - 0xdc00);
                            s.extend(char::from_u32(code));
                        }
                        (_, unit) => s.extend(char::from_u32(unit)),
                    }
                }
                Some(other) => s.push(other),
                None => break,
            },
            c => s.push(c),
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_listings_read_without_a_json_library() {
        let listing = r#"[
          {"name": "Atari - 2600", "path": "cht/Atari - 2600", "sha": "aaa", "size": 0},
          {"name": "Nintendo - Game Boy Advance", "path": "cht/x", "sha": "b1c2", "size": 0,
           "_links": {"self": "https://example"}}
        ]"#;
        let pairs = json_pairs(listing);
        assert!(pairs.contains(&("sha".into(), "b1c2".into())));
        let tree = r#"{"sha":"b1c2","tree":[{"path":"Pokémon \"X\" (USA).cht","type":"blob","size":12},{"path":"README","type":"blob"}],"truncated":false}"#;
        let paths: Vec<String> = json_pairs(tree)
            .into_iter()
            .filter(|(k, _)| k == "path")
            .map(|(_, v)| v)
            .collect();
        assert_eq!(paths, ["Pokémon \"X\" (USA).cht", "README"]);
    }

    #[test]
    fn a_rom_takes_its_own_files_and_else_its_titles_english_ones() {
        let index: Vec<String> = [
            "Pokemon - Emerald Version (USA, Europe) (Code Breaker).cht",
            "Pokemon - Emerald Version (USA, Europe) (GameShark).cht",
            "Pokemon - Emerald Version (Japan).cht",
            "Tetris (World).cht",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(
            matches(&index, "Pokemon - Emerald Version (USA, Europe)").len(),
            2
        );
        assert_eq!(
            matches(&index, "Pokemon - Emerald Version (Europe) (Rev 1)"),
            index[..2].to_vec(),
            "same title, English releases first"
        );
        assert_eq!(matches(&index, "Tetris"), ["Tetris (World).cht"]);
        assert!(matches(&index, "Zelda").is_empty());
    }

    #[test]
    fn names_are_escaped_for_the_raw_url() {
        assert_eq!(
            encode("Pokémon #1 (USA, Europe).cht"),
            "Pok%C3%A9mon%20%231%20(USA,%20Europe).cht"
        );
    }
}
