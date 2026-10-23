//! SELECT on a shelf cart: its shell's colour and finish on one tab, and when it has no label
//! and there is none to fetch, the marker its title is written in on a second. Y fetches the
//! label of a cart that has none.
use std::path::PathBuf;

use slot_input::Millis;
use slot_store::{Outline, ShellChoice, ShellFinish};

/// The core picker's pace: the shelf parts and the cart rises, then it lifts into place.
const OPEN_MS: f32 = crate::core_picker::OPEN_MS as f32;
const CLOSE_MS: f32 = crate::core_picker::CLOSE_MS as f32;

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

/// The markers a blank cart's title can be written in: the first suits a Game Boy pak, the
/// second a GBA cart, as `slot_ui`'s generated labels draw them.
pub const MARKERS: [(&str, [u8; 3]); 3] = [
    ("Black", [0x1b, 0x1c, 0x24]),
    ("White", [0xf2, 0xf0, 0xea]),
    ("Gold", [0xc9, 0xa2, 0x27]),
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
    /// The tab names themselves, to step between the tabs.
    Tabs,
    Colour,
    Finish,
    /// An SNES cart's shape: the American cart or the European and Japanese one.
    Model,
    Marker,
}

/// The SNES shapes the Model row offers, in its order.
pub const MODELS: [Outline; 2] = [Outline::Notched, Outline::Rounded];

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Tab {
    Colour,
    Marker,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Fetch {
    /// The cart has a label: no fetch is offered.
    Labelled,
    Idle,
    Working,
}

pub enum Outcome {
    None,
    /// The shell shown changed: draw the cart again with it.
    Preview(Option<ShellChoice>),
    /// The marker shown changed: write the title again in it.
    Marker(Option<[u8; 3]>),
    Save(Option<ShellChoice>, Option<[u8; 3]>),
    /// Closed without saving: the cart goes back to what it was.
    Revert(Option<ShellChoice>, Option<[u8; 3]>),
    Fetch,
}

pub enum Press {
    Left,
    Right,
    Up,
    Down,
    Keep,
    Fetch,
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
    tab: Tab,
    row: Row,
    fetch: Fetch,
    /// A label was looked for and none exists: the marker is offered instead of a fetch.
    no_label: bool,
    was_marker: Option<[u8; 3]>,
    /// The marker the cart's own shell is written in.
    own_marker: usize,
    marker: usize,
    opened: Millis,
    /// When it began to close, and how far open it was then.
    closed: Option<(Millis, f32)>,
    /// An SNES cart's shape picked, and the one its region gives it.
    model: Option<(usize, usize)>,
}

impl ShellPicker {
    #[allow(clippy::too_many_arguments)]
    pub fn open(
        rom: PathBuf,
        stem: String,
        was: Option<ShellChoice>,
        labelled: bool,
        no_label: bool,
        was_marker: Option<[u8; 3]>,
        gba: bool,
        now: Millis,
    ) -> Self {
        let was_colour = was.filter(|c| c.finish != ShellFinish::Own);
        let custom = was_colour
            .map(|c| c.colour)
            .filter(|c| PALETTE.iter().all(|(_, p)| p != c));
        let swatches: Vec<Swatch> = std::iter::once(Swatch::Original)
            .chain(custom.map(Swatch::Custom))
            .chain((0..PALETTE.len()).map(Swatch::Palette))
            .collect();
        let swatch = was_colour
            .and_then(|c| swatches.iter().position(|s| s.colour() == Some(c.colour)))
            .unwrap_or(0);
        let finish = was_colour
            .and_then(|c| FINISHES.iter().position(|f| *f == c.finish))
            .unwrap_or(0);
        let own_marker = usize::from(gba);
        let marker = was_marker
            .and_then(|m| MARKERS.iter().position(|(_, c)| *c == m))
            .unwrap_or(own_marker);
        ShellPicker {
            rom,
            stem,
            was,
            swatches,
            swatch,
            finish,
            tab: Tab::Colour,
            row: Row::Colour,
            fetch: if labelled {
                Fetch::Labelled
            } else {
                Fetch::Idle
            },
            no_label,
            was_marker,
            own_marker,
            marker,
            opened: now,
            closed: None,
            model: None,
        }
    }

    /// An SNES cart also picks its shape; `auto` is the one its region gives it, in `MODELS`.
    pub fn with_model(mut self, auto: usize) -> Self {
        let picked = self
            .was
            .and_then(|c| MODELS.iter().position(|o| *o == c.outline))
            .unwrap_or(auto);
        self.model = Some((picked, auto));
        self
    }

    pub fn model(&self) -> Option<usize> {
        self.model.map(|(picked, _)| picked)
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

    pub fn tab(&self) -> Tab {
        self.tab
    }

    /// Y fetches the label while the cart has none and it is not known to have none.
    pub fn offers_fetch(&self) -> bool {
        self.fetch != Fetch::Labelled && !self.no_label
    }

    pub fn marker(&self) -> usize {
        self.marker
    }

    pub fn was_marker(&self) -> Option<[u8; 3]> {
        self.was_marker
    }

    /// The marker's tab, for a blank cart whose label was looked for and is not there.
    pub fn offers_marker(&self) -> bool {
        self.fetch != Fetch::Labelled && self.no_label
    }

    /// The marker picked; None for the one the cart's shell is written in anyway.
    pub fn marker_choice(&self) -> Option<[u8; 3]> {
        (self.marker != self.own_marker).then(|| MARKERS[self.marker].1)
    }

    /// The finish only means something on a colour of one's own.
    pub fn finish_on(&self) -> bool {
        self.swatches[self.swatch] != Swatch::Original
    }

    pub fn choice(&self) -> Option<ShellChoice> {
        // The shape is only written down when it is not the one the region gives.
        let outline = match self.model {
            Some((picked, auto)) if picked != auto => MODELS[picked],
            Some(_) => Outline::Auto,
            None => self.was.map_or(Outline::Auto, |c| c.outline),
        };
        match self.swatches[self.swatch].colour() {
            Some(colour) => Some(ShellChoice {
                outline,
                colour,
                finish: FINISHES[self.finish],
            }),
            None if outline != Outline::Auto && self.model.is_some() => Some(ShellChoice {
                outline,
                colour: [0, 0, 0],
                finish: ShellFinish::Own,
            }),
            None => None,
        }
    }

    pub fn openness(&self, now: Millis) -> f32 {
        match self.closed {
            Some((at, from)) => (from - now.saturating_sub(at) as f32 / CLOSE_MS).max(0.0),
            None => (now.saturating_sub(self.opened) as f32 / OPEN_MS).min(1.0),
        }
    }

    pub fn closing(&self) -> bool {
        self.closed.is_some()
    }

    pub fn finished(&self, now: Millis) -> bool {
        self.closed.is_some() && self.openness(now) <= 0.0
    }

    /// A fetch came back: Some(found), or None when it could not be tried or failed.
    pub fn fetched(&mut self, found: Option<bool>) {
        let ok = found == Some(true);
        self.fetch = if ok { Fetch::Labelled } else { Fetch::Idle };
        if found == Some(false) {
            self.no_label = true;
        }
        // None to fetch: the marker's tab opens, as that is what is left to pick.
        if found == Some(false) && self.offers_marker() {
            self.tab = Tab::Marker;
            self.row = Row::Marker;
        }
    }

    fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        if self.offers_marker() {
            rows.push(Row::Tabs);
        }
        match self.tab {
            Tab::Colour => {
                rows.push(Row::Colour);
                if self.finish_on() {
                    rows.push(Row::Finish);
                }
                if self.model.is_some() {
                    rows.push(Row::Model);
                }
            }
            Tab::Marker => rows.push(Row::Marker),
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
                    Row::Tabs => {
                        self.tab = if by < 0 { Tab::Colour } else { Tab::Marker };
                        return Outcome::None;
                    }
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
                    Row::Model => {
                        let Some((picked, auto)) = self.model else {
                            return Outcome::None;
                        };
                        let to = step(picked, by, MODELS.len());
                        if to == picked {
                            return Outcome::None;
                        }
                        self.model = Some((to, auto));
                    }
                    Row::Marker => {
                        let to = step(self.marker, by, MARKERS.len());
                        if to == self.marker {
                            return Outcome::None;
                        }
                        self.marker = to;
                        return Outcome::Marker(self.marker_choice());
                    }
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
            Press::Fetch => {
                if !self.offers_fetch() || self.fetch != Fetch::Idle {
                    return Outcome::None;
                }
                self.fetch = Fetch::Working;
                Outcome::Fetch
            }
            Press::Keep => {
                self.closed = Some((now, self.openness(now)));
                Outcome::Save(self.choice(), self.marker_choice())
            }
            Press::Back => {
                self.closed = Some((now, self.openness(now)));
                if self.choice() == self.was && self.marker_choice() == self.was_marker {
                    Outcome::None
                } else {
                    Outcome::Revert(self.was, self.was_marker)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picker(was: Option<ShellChoice>, labelled: bool) -> ShellPicker {
        ShellPicker::open(
            "a.gb".into(),
            "a".into(),
            was,
            labelled,
            false,
            None,
            false,
            0,
        )
    }

    #[test]
    fn a_cart_with_no_pick_of_its_own_opens_on_original() {
        let p = picker(None, true);
        assert_eq!(p.swatches()[p.swatch()], Swatch::Original);
        assert_eq!(p.choice(), None);
        assert!(!p.finish_on());
    }

    #[test]
    fn an_snes_cart_picks_its_shape_and_keeps_its_own_colour() {
        let mut p = picker(None, true).with_model(0);
        assert_eq!(p.choice(), None, "the region's own shape is not written down");
        p.press(Press::Down, 0);
        assert_eq!(p.row(), Row::Model);
        p.press(Press::Right, 0);
        let c = p.choice().unwrap();
        assert_eq!((c.outline, c.finish), (Outline::Rounded, ShellFinish::Own));
        let again = picker(Some(c), true).with_model(0);
        assert_eq!(again.model(), Some(1));
        assert_eq!(again.swatches()[again.swatch()], Swatch::Original);
        assert_eq!(again.choice(), Some(c));
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
        assert!(matches!(p.press(Press::Fetch, 0), Outcome::Fetch));
        assert!(
            matches!(p.press(Press::Fetch, 0), Outcome::None),
            "one fetch at a time"
        );
    }

    #[test]
    fn back_puts_the_old_shell_back() {
        let was = ShellChoice::parse("auto f2c230 solid");
        let mut p = picker(was, true);
        p.press(Press::Right, 0);
        assert!(matches!(p.press(Press::Back, 0), Outcome::Revert(w, None) if w == was));
        assert!(p.finished(1000));
    }

    #[test]
    fn a_labelled_cart_offers_no_download() {
        let mut p = picker(ShellChoice::parse("auto f2c230 solid"), true);
        assert!(!p.offers_fetch());
        assert!(matches!(p.press(Press::Fetch, 0), Outcome::None));
        p.press(Press::Up, 0);
        assert_eq!(p.row(), Row::Colour, "one tab: no tab row");
    }

    #[test]
    fn a_cart_with_no_label_to_fetch_offers_a_marker_instead() {
        let mut p = picker(None, false);
        assert!(!p.offers_marker());
        p.press(Press::Fetch, 0);
        p.fetched(Some(false));
        assert!(!p.offers_fetch());
        assert_eq!(p.tab(), Tab::Marker, "the marker's tab opens");
        assert_eq!(p.row(), Row::Marker);
        assert_eq!(p.marker_choice(), None, "opens on the shell's own marker");
        let Outcome::Marker(Some(ink)) = p.press(Press::Right, 0) else {
            panic!("no marker preview");
        };
        assert_eq!(ink, MARKERS[1].1);
        assert!(matches!(p.press(Press::Keep, 0), Outcome::Save(None, Some(i)) if i == ink));
    }

    #[test]
    fn a_failed_fetch_can_be_tried_again() {
        let mut p = picker(None, false);
        p.press(Press::Fetch, 0);
        p.fetched(None);
        assert!(p.offers_fetch());
        assert!(matches!(p.press(Press::Fetch, 0), Outcome::Fetch));
    }

    #[test]
    fn up_reaches_the_tabs_and_sideways_steps_between_them() {
        let mut p = ShellPicker::open("a.gb".into(), "a".into(), None, false, true, None, false, 0);
        p.press(Press::Up, 0);
        assert_eq!(p.row(), Row::Tabs);
        p.press(Press::Right, 0);
        assert_eq!(p.tab(), Tab::Marker);
        p.press(Press::Down, 0);
        assert_eq!(p.row(), Row::Marker);
        p.press(Press::Up, 0);
        p.press(Press::Left, 0);
        p.press(Press::Down, 0);
        assert_eq!((p.tab(), p.row()), (Tab::Colour, Row::Colour));
    }

    #[test]
    fn a_gba_cart_opens_on_white_and_keeps_a_picked_marker() {
        let gold = MARKERS[2].1;
        let p = ShellPicker::open("a.gba".into(), "a".into(), None, false, true, None, true, 0);
        assert_eq!(p.marker(), 1);
        assert!(p.offers_marker());
        let p = ShellPicker::open(
            "a.gb".into(),
            "a".into(),
            None,
            false,
            true,
            Some(gold),
            false,
            0,
        );
        assert_eq!(p.marker(), 2);
        assert_eq!(p.marker_choice(), Some(gold));
    }
}
