//! SELECT on a shelf cart: its shell's colour and finish, and its label fetched when it has none.
use std::path::PathBuf;

use slot_input::Millis;
use slot_store::{Outline, ShellChoice, ShellFinish};

const OPEN_MS: f32 = 260.0;
const CLOSE_MS: f32 = 200.0;

/// The colours offered, in the order they sit along the row.
pub const PALETTE: [(&str, [u8; 3]); 11] = [
    ("Black", [0x2a, 0x2a, 0x2e]),
    ("White", [0xec, 0xeb, 0xe6]),
    ("Grey", [0x9a, 0x97, 0x8f]),
    ("Red", [0xc2, 0x3a, 0x3a]),
    ("Orange", [0xe0, 0x7a, 0x2e]),
    ("Yellow", [0xf2, 0xc2, 0x30]),
    ("Green", [0x3f, 0x9a, 0x4f]),
    ("Teal", [0x2f, 0xa5, 0xa0]),
    ("Blue", [0x3f, 0x6f, 0xd8]),
    ("Purple", [0x7a, 0x4f, 0xc0]),
    ("Pink", [0xd8, 0x50, 0x8a]),
];

pub const FINISHES: [ShellFinish; 3] =
    [ShellFinish::Solid, ShellFinish::Clear, ShellFinish::Glitter];

/// A swatch on the row: the cart's own shell, a colour set elsewhere (the studio) that the
/// palette does not hold, or one of the palette's.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Swatch {
    Original,
    Custom([u8; 3]),
    Palette(usize),
}

impl Swatch {
    pub fn colour(self) -> Option<[u8; 3]> {
        match self {
            Swatch::Original => None,
            Swatch::Custom(c) => Some(c),
            Swatch::Palette(i) => Some(PALETTE[i].1),
        }
    }

    /// Its name's place among the faces: Original, Custom, then the palette.
    pub fn name_index(self) -> usize {
        match self {
            Swatch::Original => 0,
            Swatch::Custom(_) => 1,
            Swatch::Palette(i) => 2 + i,
        }
    }
}

pub fn swatch_names() -> Vec<&'static str> {
    ["Original", "Custom"]
        .into_iter()
        .chain(PALETTE.iter().map(|(name, _)| *name))
        .collect()
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Row {
    Colour,
    Finish,
    Download,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Fetch {
    /// The cart has a label: the row is not offered.
    Labelled,
    Idle,
    Working,
}

pub enum Outcome {
    None,
    /// The shell shown changed: draw the cart again with it.
    Preview(Option<ShellChoice>),
    Save(Option<ShellChoice>),
    /// Closed without saving: the cart goes back to what it was.
    Revert(Option<ShellChoice>),
    Fetch,
}

pub enum Press {
    Left,
    Right,
    Up,
    Down,
    Keep,
    Back,
}

#[derive(Clone, Debug)]
pub struct ShellPicker {
    pub rom: PathBuf,
    pub stem: String,
    was: Option<ShellChoice>,
    swatches: Vec<Swatch>,
    swatch: usize,
    finish: usize,
    row: Row,
    fetch: Fetch,
    opened: Millis,
    closed: Option<Millis>,
}

impl ShellPicker {
    pub fn open(
        rom: PathBuf,
        stem: String,
        was: Option<ShellChoice>,
        labelled: bool,
        now: Millis,
    ) -> Self {
        let custom = was
            .map(|c| c.colour)
            .filter(|c| PALETTE.iter().all(|(_, p)| p != c));
        let swatches: Vec<Swatch> = std::iter::once(Swatch::Original)
            .chain(custom.map(Swatch::Custom))
            .chain((0..PALETTE.len()).map(Swatch::Palette))
            .collect();
        let swatch = was
            .and_then(|c| swatches.iter().position(|s| s.colour() == Some(c.colour)))
            .unwrap_or(0);
        let finish = was
            .and_then(|c| FINISHES.iter().position(|f| *f == c.finish))
            .unwrap_or(0);
        ShellPicker {
            rom,
            stem,
            was,
            swatches,
            swatch,
            finish,
            row: Row::Colour,
            fetch: if labelled {
                Fetch::Labelled
            } else {
                Fetch::Idle
            },
            opened: now,
            closed: None,
        }
    }

    pub fn swatches(&self) -> &[Swatch] {
        &self.swatches
    }

    pub fn swatch(&self) -> usize {
        self.swatch
    }

    pub fn finish(&self) -> usize {
        self.finish
    }

    pub fn row(&self) -> Row {
        self.row
    }

    pub fn fetch(&self) -> Fetch {
        self.fetch
    }

    /// The finish only means something on a colour of one's own.
    pub fn finish_on(&self) -> bool {
        self.swatches[self.swatch] != Swatch::Original
    }

    pub fn choice(&self) -> Option<ShellChoice> {
        let colour = self.swatches[self.swatch].colour()?;
        Some(ShellChoice {
            outline: self.was.map_or(Outline::Auto, |c| c.outline),
            colour,
            finish: FINISHES[self.finish],
        })
    }

    pub fn openness(&self, now: Millis) -> f32 {
        match self.closed {
            Some(at) => 1.0 - (now.saturating_sub(at) as f32 / CLOSE_MS).min(1.0),
            None => (now.saturating_sub(self.opened) as f32 / OPEN_MS).min(1.0),
        }
    }

    pub fn closing(&self) -> bool {
        self.closed.is_some()
    }

    pub fn finished(&self, now: Millis) -> bool {
        self.closed.is_some() && self.openness(now) <= 0.0
    }

    pub fn fetched(&mut self, ok: bool) {
        self.fetch = if ok { Fetch::Labelled } else { Fetch::Idle };
        if ok && self.row == Row::Download {
            self.row = Row::Colour;
        }
    }

    fn rows(&self) -> Vec<Row> {
        let mut rows = vec![Row::Colour];
        if self.finish_on() {
            rows.push(Row::Finish);
        }
        if self.fetch != Fetch::Labelled {
            rows.push(Row::Download);
        }
        rows
    }

    pub fn press(&mut self, press: Press, now: Millis) -> Outcome {
        if self.closed.is_some() {
            return Outcome::None;
        }
        let step =
            |at: usize, by: isize, n: usize| (at as isize + by).clamp(0, n as isize - 1) as usize;
        match press {
            Press::Left | Press::Right => {
                let by = if matches!(press, Press::Left) { -1 } else { 1 };
                match self.row {
                    Row::Colour => {
                        let to = step(self.swatch, by, self.swatches.len());
                        if to == self.swatch {
                            return Outcome::None;
                        }
                        self.swatch = to;
                    }
                    Row::Finish => {
                        let to = step(self.finish, by, FINISHES.len());
                        if to == self.finish {
                            return Outcome::None;
                        }
                        self.finish = to;
                    }
                    Row::Download => return Outcome::None,
                }
                Outcome::Preview(self.choice())
            }
            Press::Up | Press::Down => {
                let rows = self.rows();
                let at = rows.iter().position(|r| *r == self.row).unwrap_or(0);
                let by = if matches!(press, Press::Up) { -1 } else { 1 };
                self.row = rows[step(at, by, rows.len())];
                Outcome::None
            }
            Press::Keep if self.row == Row::Download => {
                if self.fetch != Fetch::Idle {
                    return Outcome::None;
                }
                self.fetch = Fetch::Working;
                Outcome::Fetch
            }
            Press::Keep => {
                self.closed = Some(now);
                Outcome::Save(self.choice())
            }
            Press::Back => {
                self.closed = Some(now);
                if self.choice() == self.was {
                    Outcome::None
                } else {
                    Outcome::Revert(self.was)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picker(was: Option<ShellChoice>, labelled: bool) -> ShellPicker {
        ShellPicker::open("a.gb".into(), "a".into(), was, labelled, 0)
    }

    #[test]
    fn a_cart_with_no_pick_of_its_own_opens_on_original() {
        let p = picker(None, true);
        assert_eq!(p.swatches()[p.swatch()], Swatch::Original);
        assert_eq!(p.choice(), None);
        assert!(!p.finish_on());
    }

    #[test]
    fn a_colour_from_elsewhere_gets_a_swatch_of_its_own() {
        let was = ShellChoice::parse("rounded 123456 glitter");
        let p = picker(was, true);
        assert_eq!(p.swatches()[p.swatch()], Swatch::Custom([0x12, 0x34, 0x56]));
        assert_eq!(p.choice(), was);
    }

    #[test]
    fn stepping_along_the_row_previews_and_down_reaches_the_finish() {
        let mut p = picker(None, false);
        assert!(matches!(
            p.press(Press::Right, 0),
            Outcome::Preview(Some(_))
        ));
        p.press(Press::Down, 0);
        assert_eq!(p.row(), Row::Finish);
        let Outcome::Preview(Some(c)) = p.press(Press::Right, 0) else {
            panic!("no preview");
        };
        assert_eq!(c.finish, ShellFinish::Clear);
        p.press(Press::Down, 0);
        assert_eq!(p.row(), Row::Download);
        assert!(matches!(p.press(Press::Keep, 0), Outcome::Fetch));
        assert!(
            matches!(p.press(Press::Keep, 0), Outcome::None),
            "one fetch at a time"
        );
    }

    #[test]
    fn back_puts_the_old_shell_back() {
        let was = ShellChoice::parse("auto f2c230 solid");
        let mut p = picker(was, true);
        p.press(Press::Right, 0);
        assert!(matches!(p.press(Press::Back, 0), Outcome::Revert(w) if w == was));
        assert!(p.finished(1000));
    }

    #[test]
    fn a_labelled_cart_offers_no_download() {
        let mut p = picker(ShellChoice::parse("auto f2c230 solid"), true);
        p.press(Press::Down, 0);
        p.press(Press::Down, 0);
        assert_eq!(p.row(), Row::Finish);
    }
}
