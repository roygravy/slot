//! Play time, blow time and wear per game, for the dust-and-wear look of the carts.
//!
//! Stored in `Config/wear.tsv`, one line per game:
//! `key  play_secs  blown_at  first_seen  wear_base` (times are unix seconds, 0 = never).
//! Dust grows with days since the last blow (or first sighting) and is removed by blowing or playing.
//! Wear grows with play time and is removed only by a new cart; play time itself is kept.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{atomic_write, Platform, CONFIG_DIR};

pub const WEAR_FILE: &str = "wear.tsv";
/// Days without blowing at which the dust goes to level 1, 2 and 3.
pub const DUST_DAYS: [u64; 3] = [7, 30, 90];
/// Played seconds at which the wear goes to level 1, 2 and 3 (2 h, 10 h, 30 h).
pub const WEAR_SECS: [u64; 3] = [2 * 3600, 10 * 3600, 30 * 3600];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    pub play_secs: u64,
    pub blown_at: u64,
    pub first_seen: u64,
    pub wear_base: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WearBook {
    entries: BTreeMap<String, Entry>,
}

pub fn key(platform: Platform, stem: &str) -> String {
    let clean: String = stem.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    format!("{}/{}", platform.dir_name(), clean)
}

pub fn wear_path(root: &Path) -> PathBuf {
    root.join(CONFIG_DIR).join(WEAR_FILE)
}

fn level(value: u64, steps: &[u64; 3]) -> u8 {
    steps.iter().filter(|&&s| value >= s).count() as u8
}

impl WearBook {
    pub fn parse(text: &str) -> Self {
        let mut entries = BTreeMap::new();
        for line in text.lines() {
            let mut f = line.split('\t');
            let Some(k) = f.next().filter(|k| !k.is_empty()) else { continue };
            let mut n = || f.next().and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(0);
            let e = Entry { play_secs: n(), blown_at: n(), first_seen: n(), wear_base: n() };
            entries.insert(k.to_string(), e);
        }
        WearBook { entries }
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        for (k, e) in &self.entries {
            out.push_str(&format!("{k}\t{}\t{}\t{}\t{}\n", e.play_secs, e.blown_at, e.first_seen, e.wear_base));
        }
        out
    }

    pub fn read(root: &Path) -> Self {
        std::fs::read_to_string(wear_path(root)).map(|t| Self::parse(&t)).unwrap_or_default()
    }

    pub fn write(&self, root: &Path) -> std::io::Result<()> {
        let path = wear_path(root);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        atomic_write(&path, self.render().as_bytes())
    }

    pub fn get(&self, key: &str) -> Entry {
        self.entries.get(key).copied().unwrap_or_default()
    }

    /// Remember when a game first showed up on the shelf. `now` 0 (clock not set) does nothing.
    pub fn see(&mut self, key: &str, now: u64) {
        if now == 0 {
            return;
        }
        let e = self.entries.entry(key.to_string()).or_default();
        if e.first_seen == 0 {
            e.first_seen = now;
        }
    }

    pub fn add_play(&mut self, key: &str, secs: u64) {
        self.entries.entry(key.to_string()).or_default().play_secs += secs;
    }

    /// Blowing clears the dust only.
    pub fn blow(&mut self, key: &str, now: u64) {
        if now == 0 {
            return;
        }
        self.entries.entry(key.to_string()).or_default().blown_at = now;
    }

    /// A fresh cart comes clean: no wear and no dust (Roy); the play time stays.
    pub fn new_cart(&mut self, key: &str, now: u64) {
        let e = self.entries.entry(key.to_string()).or_default();
        e.wear_base = e.play_secs;
        if now > 0 {
            e.blown_at = now;
        }
    }

    /// 0..=3. Without a set clock (`now` 0) or a starting point there is no dust.
    pub fn dust_level(&self, key: &str, now: u64) -> u8 {
        let e = self.get(key);
        let since = e.blown_at.max(e.first_seen);
        if now == 0 || since == 0 || now < since {
            return 0;
        }
        level((now - since) / 86_400, &DUST_DAYS)
    }

    /// 0..=3.
    pub fn wear_level(&self, key: &str) -> u8 {
        let e = self.get(key);
        level(e.play_secs.saturating_sub(e.wear_base), &WEAR_SECS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DAY: u64 = 86_400;

    #[test]
    fn round_trip() {
        let mut b = WearBook::default();
        b.see("GBA/Game", 1000);
        b.add_play("GBA/Game", 99);
        b.blow("GBA/Game", 2000);
        b.new_cart("GBA/Game", 0);
        let back = WearBook::parse(&b.render());
        assert_eq!(back, b);
        assert_eq!(back.get("GBA/Game"), Entry { play_secs: 99, blown_at: 2000, first_seen: 1000, wear_base: 99 });
    }

    #[test]
    fn dust_levels_and_blow() {
        let mut b = WearBook::default();
        let t0 = 10 * DAY;
        b.see("a", t0);
        assert_eq!(b.dust_level("a", t0 + 6 * DAY), 0);
        assert_eq!(b.dust_level("a", t0 + 7 * DAY), 1);
        assert_eq!(b.dust_level("a", t0 + 30 * DAY), 2);
        assert_eq!(b.dust_level("a", t0 + 90 * DAY), 3);
        b.add_play("a", 99 * 3600);
        assert_eq!(b.dust_level("a", t0 + 90 * DAY), 3, "playing does not clean");
        b.blow("a", t0 + 90 * DAY);
        assert_eq!(b.dust_level("a", t0 + 90 * DAY), 0);
    }

    #[test]
    fn no_clock_no_dust() {
        let mut b = WearBook::default();
        b.see("a", 0);
        assert_eq!(b.dust_level("a", 100 * DAY), 0);
        assert_eq!(b.dust_level("a", 0), 0);
    }

    #[test]
    fn wear_levels_and_new_cart() {
        let mut b = WearBook::default();
        b.add_play("a", 2 * 3600 - 1);
        assert_eq!(b.wear_level("a"), 0);
        b.add_play("a", 1);
        assert_eq!(b.wear_level("a"), 1);
        b.add_play("a", 40 * 3600);
        assert_eq!(b.wear_level("a"), 3);
        b.see("a", DAY);
        assert!(b.dust_level("a", 200 * DAY) > 0);
        b.new_cart("a", 200 * DAY);
        assert_eq!(b.wear_level("a"), 0);
        assert_eq!(b.dust_level("a", 200 * DAY), 0, "a new cart has no dust");
        assert!(b.get("a").play_secs > 40 * 3600, "play time stays");
    }

    #[test]
    fn disk_round_trip() {
        let dir = std::env::temp_dir().join(format!("slot-wear-{}", std::process::id()));
        let mut b = WearBook::default();
        b.add_play("GB/x", 5);
        b.write(&dir).unwrap();
        assert_eq!(WearBook::read(&dir), b);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
