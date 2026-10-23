//! Brandon Kowalski's own label set, the one his Cart Studio dresses carts from: flat scans of
//! just the sticker, found by the ROM's checksum. Its index is kept on the card for a week.
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::{Error, Transport};

pub(crate) const BASE: &str = "https://art.slot-cfw.fyi/";
const CACHE: &str = "System/Cache/Labels/slot-art.tsv";
const KEEP: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// The media the set calls a scan of the sticker.
const STICKER: &str = "support-texture";

/// ROM checksum to the sticker's path under `BASE`.
pub(crate) type Index = HashMap<u32, String>;

/// The index from the card while it is fresh, else from the set, else the stale one on the card.
pub(crate) fn index(http: &mut impl Transport, root: &Path) -> Result<Index, Error> {
    let cache = root.join(CACHE);
    let cached = std::fs::read_to_string(&cache).ok();
    let fresh = std::fs::metadata(&cache)
        .and_then(|m| m.modified())
        .is_ok_and(|at| SystemTime::now().duration_since(at).is_ok_and(|age| age < KEEP));
    if let (true, Some(text)) = (fresh, &cached) {
        return Ok(read_tsv(text));
    }
    match http.get(&format!("{BASE}index.json")) {
        Ok(bytes) => {
            let index = parse(&String::from_utf8_lossy(&bytes));
            if index.is_empty() {
                return Err(Error::Unavailable("label set index is empty".into()));
            }
            let mut text: Vec<String> =
                index.iter().map(|(crc, path)| format!("{crc:08X}\t{path}")).collect();
            text.sort();
            if let Some(dir) = cache.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = slot_store::atomic_write(&cache, (text.join("\n") + "\n").as_bytes()) {
                eprintln!("slot: labels: {}: {e}", cache.display());
            }
            Ok(index)
        }
        Err(e) => match cached {
            Some(text) => Ok(read_tsv(&text)),
            None => Err(e),
        },
    }
}

fn read_tsv(text: &str) -> Index {
    text.lines()
        .filter_map(|line| {
            let (crc, path) = line.split_once('\t')?;
            Some((u32::from_str_radix(crc, 16).ok()?, path.to_owned()))
        })
        .collect()
}

/// The set's index.json: each entry is keyed by a checksum, or by a name with its checksums
/// listed under "crcs".
pub(crate) fn parse(json: &str) -> Index {
    let mut index = Index::new();
    let Some(Json::Object(entries)) = Parser(json.as_bytes(), 0).value() else {
        return index;
    };
    for (key, entry) in entries {
        let Json::Object(fields) = entry else {
            continue;
        };
        let Some(Json::Str(path)) = fields.iter().find(|(k, _)| k == STICKER).map(|(_, v)| v)
        else {
            continue;
        };
        let crcs: Vec<&str> = match fields.iter().find(|(k, _)| k == "crcs") {
            Some((_, Json::Array(list))) => list
                .iter()
                .filter_map(|v| match v {
                    Json::Str(s) => Some(s.as_str()),
                    _ => None,
                })
                .collect(),
            _ => vec![key.as_str()],
        };
        for crc in crcs {
            if let Ok(crc) = u32::from_str_radix(crc, 16) {
                index.insert(crc, path.clone());
            }
        }
    }
    index
}

enum Json {
    Str(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
    Other,
}

/// Just enough JSON for the index: strings, arrays and objects; numbers and literals are
/// skipped.
struct Parser<'a>(&'a [u8], usize);

impl Parser<'_> {
    fn skip_space(&mut self) {
        while self.0.get(self.1).is_some_and(|b| b.is_ascii_whitespace()) {
            self.1 += 1;
        }
    }

    fn value(&mut self) -> Option<Json> {
        self.skip_space();
        match *self.0.get(self.1)? {
            b'"' => self.string().map(Json::Str),
            b'[' => {
                self.1 += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_space();
                    match *self.0.get(self.1)? {
                        b']' => {
                            self.1 += 1;
                            return Some(Json::Array(items));
                        }
                        b',' => self.1 += 1,
                        _ => items.push(self.value()?),
                    }
                }
            }
            b'{' => {
                self.1 += 1;
                let mut fields = Vec::new();
                loop {
                    self.skip_space();
                    match *self.0.get(self.1)? {
                        b'}' => {
                            self.1 += 1;
                            return Some(Json::Object(fields));
                        }
                        b',' => self.1 += 1,
                        b'"' => {
                            let key = self.string()?;
                            self.skip_space();
                            if *self.0.get(self.1)? != b':' {
                                return None;
                            }
                            self.1 += 1;
                            fields.push((key, self.value()?));
                        }
                        _ => return None,
                    }
                }
            }
            _ => {
                while self
                    .0
                    .get(self.1)
                    .is_some_and(|b| !matches!(b, b',' | b']' | b'}'))
                {
                    self.1 += 1;
                }
                Some(Json::Other)
            }
        }
    }

    fn string(&mut self) -> Option<String> {
        self.1 += 1;
        let mut out = Vec::new();
        loop {
            match *self.0.get(self.1)? {
                b'"' => {
                    self.1 += 1;
                    return Some(String::from_utf8_lossy(&out).into_owned());
                }
                b'\\' => {
                    self.1 += 1;
                    match *self.0.get(self.1)? {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let hex = std::str::from_utf8(self.0.get(self.1 + 1..self.1 + 5)?).ok()?;
                            let c = char::from_u32(u32::from_str_radix(hex, 16).ok()?)
                                .unwrap_or('\u{fffd}');
                            out.extend_from_slice(c.to_string().as_bytes());
                            self.1 += 4;
                        }
                        b => out.push(b),
                    }
                    self.1 += 1;
                }
                b => {
                    out.push(b);
                    self.1 += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stickers_are_found_by_every_checksum_an_entry_names() {
        let json = r#"{"000206C0":{"look":{"band":null,"by":"analysis","ground":"3067a3","sure":true},
            "support-texture":"GBA/support-texture/000206C0.png","wheel":"GBA/wheel/000206C0.png"},
            "00202DFA":{"wheel":"GB/wheel/00202DFA.png"},
            "kirby":{"crcs":["38217BDB","C0B5DC37"],"name":"Kirby's Dream Land DX","platform":"GBC",
            "support-texture":"GBC/support-texture/kirbys-dream-land-dx.png"}}"#;
        let index = parse(json);
        assert_eq!(index.len(), 3, "an entry with only a logo has no sticker");
        assert_eq!(index[&0x000206C0], "GBA/support-texture/000206C0.png");
        assert_eq!(index[&0xC0B5DC37], "GBC/support-texture/kirbys-dream-land-dx.png");
        assert_eq!(read_tsv("000206C0\tGBA/a.png\nbad line\n")[&0x000206C0], "GBA/a.png");
    }
}
