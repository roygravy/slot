//! The screenshots on the card, for the Screenshots page: one game's while playing it, every
//! one from the main menu. Names are "<game> <stamp>.png" in `Screenshots/<GB|GBC|GBA>/`, or
//! "slot <stamp>.png" in `Screenshots/` for those taken outside a game; a second shot in the
//! same second ends in " 2", " 3" and so on.

use std::path::{Path, PathBuf};

use slot_store::Platform;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shot {
    pub path: PathBuf,
    /// The game it was taken in, or None for the shelf and menus.
    pub game: Option<String>,
    /// Seconds since the epoch, local time, as the name says.
    pub secs: i64,
    /// 1 for the first shot of its second, 2 for the next, ...
    pub n: u32,
}

const PLATFORMS: [Platform; 3] = [Platform::Gb, Platform::Gbc, Platform::Gba];

fn dir(root: &Path) -> PathBuf {
    root.join("Screenshots")
}

/// "Goodboy Galaxy (Demo) 2026-10-20_09-14-02 2" into its game, time and count.
pub fn parse(stem: &str) -> Option<(String, i64, u32)> {
    let (rest, n) = match stem.rsplit_once(' ') {
        Some((rest, tail)) if tail.len() <= 3 && tail.bytes().all(|b| b.is_ascii_digit()) => {
            (rest, tail.parse().ok()?)
        }
        _ => (stem, 1),
    };
    let (game, stamp) = rest.rsplit_once(' ')?;
    let secs = slot_store::parse_stamp(stamp)?;
    Some((game.to_string(), secs, n))
}

fn read(folder: &Path, in_game: bool, out: &mut Vec<Shot>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        let is_png = path
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("png"));
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if !is_png || stem.starts_with('.') {
            continue;
        }
        let Some((game, secs, n)) = parse(stem) else {
            continue;
        };
        out.push(Shot {
            game: in_game.then_some(game),
            path,
            secs,
            n,
        });
    }
}

fn newest_first(shots: &mut [Shot]) {
    shots.sort_by(|a, b| (b.secs, b.n, &b.path).cmp(&(a.secs, a.n, &a.path)));
}

/// This game's screenshots, newest first.
pub fn for_game(root: &Path, platform: Platform, stem: &str) -> Vec<Shot> {
    let mut out = Vec::new();
    read(&dir(root).join(platform.dir_name()), true, &mut out);
    out.retain(|s| s.game.as_deref() == Some(stem));
    newest_first(&mut out);
    out
}

/// Every screenshot on the card, newest first.
pub fn all(root: &Path) -> Vec<Shot> {
    let mut out = Vec::new();
    read(&dir(root), false, &mut out);
    for p in PLATFORMS {
        read(&dir(root).join(p.dir_name()), true, &mut out);
    }
    newest_first(&mut out);
    out
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// "20 Oct 2026, 09:14" for the date line.
pub fn when(secs: i64) -> String {
    let (y, m, d) = slot_store::civil_from_days(secs.div_euclid(86_400));
    let rem = secs.rem_euclid(86_400);
    let month = MONTHS[(m.clamp(1, 12) - 1) as usize];
    format!("{d} {month} {y}, {:02}:{:02}", rem / 3600, rem / 60 % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_read_back() {
        let (g, s, n) = parse("Goodboy Galaxy (Demo) 2026-10-20_09-14-02").unwrap();
        assert_eq!((g.as_str(), n), ("Goodboy Galaxy (Demo)", 1));
        assert_eq!(when(s), "20 Oct 2026, 09:14");
        let (g, _, n) = parse("slot 2026-10-20_09-14-02 2").unwrap();
        assert_eq!((g.as_str(), n), ("slot", 2));
        assert!(parse("holiday photo").is_none());
    }
}

/// The Screenshots page while it is open.
pub struct ShotView {
    pub shots: Vec<Shot>,
    pub sel: usize,
    /// The carousel's position, easing after `sel`.
    pub pos: f32,
    pub full: bool,
    /// When the full-screen hints last showed.
    pub hints_at: u64,
    /// Y asked to delete the chosen picture: A deletes, B cancels.
    pub confirm: bool,
    /// Opened from the in-game menu (this game's shots) rather than the main menu (all).
    pub in_game: bool,
    /// Bumped whenever the list changes, so pictures and names are fetched again.
    pub rev: u64,
}

impl ShotView {
    pub fn new(shots: Vec<Shot>, in_game: bool, now: u64) -> Self {
        ShotView {
            shots,
            sel: 0,
            pos: 0.0,
            full: false,
            hints_at: now,
            confirm: false,
            in_game,
            rev: 0,
        }
    }

    pub fn selected(&self) -> Option<&Shot> {
        self.shots.get(self.sel)
    }

    pub fn step(&mut self, right: bool) -> bool {
        let to = match right {
            true => (self.sel + 1).min(self.shots.len().saturating_sub(1)),
            false => self.sel.saturating_sub(1),
        };
        let moved = to != self.sel;
        self.sel = to;
        // Never trail more than one picture behind, however fast the flicks come.
        self.pos = self.pos.clamp(self.sel as f32 - 1.0, self.sel as f32 + 1.0);
        moved
    }

    /// Delete the chosen picture from the card and the list.
    pub fn delete_selected(&mut self) {
        let Some(shot) = self.shots.get(self.sel) else {
            return;
        };
        if let Err(e) = std::fs::remove_file(&shot.path) {
            eprintln!("slot: screenshots: {}: {e}", shot.path.display());
            return;
        }
        self.shots.remove(self.sel);
        self.sel = self.sel.min(self.shots.len().saturating_sub(1));
        self.pos = self.sel as f32;
        self.rev += 1;
        if self.shots.is_empty() {
            self.full = false;
        }
    }

    pub fn ease(&mut self, dt: f32) {
        let to = self.sel as f32;
        let k = (dt * 16.0).min(1.0);
        self.pos += (to - self.pos) * k;
        if (to - self.pos).abs() < 0.002 {
            self.pos = to;
        }
    }

    /// The name above the picture: the game without its region tags, or slot. for the menus.
    pub fn title(&self) -> String {
        match self.selected().and_then(|s| s.game.as_deref()) {
            Some(game) => game.split(" (").next().unwrap_or(game).trim().to_string(),
            None if self.selected().is_some() => "slot.".to_string(),
            None => String::new(),
        }
    }
}
