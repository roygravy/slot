//! SELECT on a shelf cart: its shell's colour and finish on one tab, the versions of its label
//! LaunchBox holds on a second, and when it has no label and there is none to fetch, the marker
//! its title is written in on a third. Y fetches the label of a cart that has none.
use std::path::PathBuf;

use slot_input::Millis;
use slot_store::{Outline, ShellChoice, ShellFinish};

/// The core picker's pace: the shelf parts and the cart rises, then it lifts into place.
const OPEN_MS: f32 = crate::core_picker::OPEN_MS as f32;
const CLOSE_MS: f32 = crate::core_picker::CLOSE_MS as f32;

/// The colours offered, in the order they sit along the row: each one a real cart's or
/// console's plastic (Roy: the real colours, Pokémon Red's red), with a Nintendo name.
pub const PALETTE: [(&str, [u8; 3]); 23] = [
    ("Famicom White", [0xec, 0xee, 0xe8]),
    ("Super Famicom Grey", [0xc6, 0xc6, 0xc9]),
    ("Pokémon Silver", [0xa9, 0xaa, 0xa7]),
    ("Game Boy Grey", [0x9a, 0x97, 0x8f]),
    ("NES Grey", [0x86, 0x86, 0x8a]),
    ("WarioWare Smoke", [0x5f, 0x62, 0x64]),
    ("Advance Charcoal", [0x35, 0x35, 0x3a]),
    ("Zelda Black", [0x26, 0x26, 0x2a]),
    ("Pokémon Red", [0xc0, 0x28, 0x2c]),
    ("Donkey Kong Brown", [0xa9, 0x51, 0x3b]),
    ("Pokémon FireRed", [0xd8, 0x52, 0x24]),
    ("Daisy Orange", [0xf0, 0xa9, 0x5e]),
    ("Pokémon Yellow", [0xe9, 0xa8, 0x26]),
    ("Zelda Gold", [0xc9, 0xa2, 0x3f]),
    ("Pokémon Gold", [0xb3, 0x8b, 0x3a]),
    ("Pokémon LeafGreen", [0x63, 0xb0, 0x44]),
    ("Pokémon Emerald", [0x24, 0x9c, 0x60]),
    ("Yoshi Green", [0x2f, 0x8f, 0x4e]),
    ("Zero Suit Turquoise", [0x1f, 0x9f, 0xb6]),
    ("Pokémon Crystal", [0x86, 0xb9, 0xbf]),
    ("Pokémon Sapphire", [0x2f, 0x5c, 0xc0]),
    ("Pokémon Blue", [0x2b, 0x3a, 0x88]),
    ("Kirby Pink", [0xec, 0x94, 0xb4]),
];

/// The markers a blank cart's title can be written in: the first suits a Game Boy pak, the
/// second a GBA cart, as `slot_ui`'s generated labels draw them.
pub const MARKERS: [(&str, [u8; 3]); 3] = [
    ("Black", [0x1b, 0x1c, 0x24]),
    ("White", [0xf2, 0xf0, 0xea]),
    ("Gold", [0xc9, 0xa2, 0x27]),
];

/// Two to a line in the studio: Solid and Clear, then their glitters under them (Roy).
pub const FINISHES: &[ShellFinish] = &[
    ShellFinish::Solid,
    ShellFinish::Clear,
    ShellFinish::SolidGlitter,
    ShellFinish::Glitter,
];
/// An NES cart is painted metallic, as the gold ones were, instead of glittering.
pub const NES_FINISHES: &[ShellFinish] = &[
    ShellFinish::Solid,
    ShellFinish::Clear,
    ShellFinish::Metallic,
];

/// Every finish's name, in the order of its face: the ones a cart is offered index into it.
pub const FINISH_NAMES: [(ShellFinish, &str); 5] = [
    (ShellFinish::Solid, "Solid"),
    (ShellFinish::SolidGlitter, "Glitter"),
    (ShellFinish::Clear, "Clear"),
    (ShellFinish::Glitter, "Clear Glitter"),
    (ShellFinish::Metallic, "Metallic"),
];

/// A slot finish's name among the finishes' faces: the carts' ones, then Pearl after them.
pub fn slot_finish_face(finish: slot_ui::SlotFinish) -> usize {
    let cart = |f: ShellFinish| FINISH_NAMES.iter().position(|(n, _)| *n == f).unwrap_or(0);
    match finish {
        slot_ui::SlotFinish::Solid => cart(ShellFinish::Solid),
        slot_ui::SlotFinish::Clear => cart(ShellFinish::Clear),
        slot_ui::SlotFinish::Metallic => cart(ShellFinish::Metallic),
        slot_ui::SlotFinish::Pearl => FINISH_NAMES.len(),
    }
}

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
    /// The versions of the label, side by side.
    Label,
    Marker,
    /// The Game Boy slot's case.
    Slot,
    /// The slot studio's models: DMG, Pocket/Light.
    SlotModel,
    /// The slot studio's finish: Solid or Clear, as the carts' (Roy 2026-11-05).
    SlotFinish,
}

/// The shapes the Model row offers, in its order: Game Boy and Color.
pub const MODELS: [Outline; 2] = [Outline::Notched, Outline::Rounded];
/// An SNES cart's, Europe first (Roy): PAL, then NTSC.
pub const SNES_MODELS: [Outline; 2] = [Outline::Rounded, Outline::Notched];

/// The Super Game Boys the studio offers for the SGB cart, in its order; the European one is
/// the original.
pub const SGB_MODELS: [slot_ui::SgbModel; 3] = [
    slot_ui::SgbModel::Pal,
    slot_ui::SgbModel::Ntsc,
    slot_ui::SgbModel::Sgb2,
];
const SGB_ORIGINAL: usize = 0;

/// How a Super Game Boy is written down: in its cart's shell choice (see
/// `slot_ui::sgb_model_of`). None for the original.
pub fn sgb_choice(model: slot_ui::SgbModel) -> Option<ShellChoice> {
    let (outline, finish) = match model {
        slot_ui::SgbModel::Pal => return None,
        slot_ui::SgbModel::Ntsc => (Outline::Notched, ShellFinish::Own),
        slot_ui::SgbModel::Sfc => (Outline::Rounded, ShellFinish::Solid),
        slot_ui::SgbModel::Sgb2 => (Outline::Rounded, ShellFinish::Clear),
    };
    Some(ShellChoice {
        outline,
        colour: [0x80, 0x80, 0x80],
        finish,
    })
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Tab {
    Colour,
    Label,
    Marker,
}

/// The label tab's versions: asked for the first time the tab opens.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Versions {
    Unasked,
    Looking,
    /// None were found, or they could not be looked for.
    Missing,
    Found(usize),
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
    /// Y, then A: the cart's label comes off, and its title is written in marker.
    RemoveLabel,
    /// The label tab opened for the first time: look up the label's versions.
    Versions,
    /// Show the cart in this version of its label.
    Version(usize),
    /// Y on the label's tab: search for the game by name, to find its label that way.
    Search,
    /// Y, then A, on the Wear tab: swap the cart for a fresh one. The wear starts over; the dust stays.
    NewCart,
}

pub enum Press {
    Left,
    Right,
    /// L1 and R1: the tab before or after.
    TabLeft,
    TabRight,
    Up,
    Down,
    Keep,
    /// Y: ask to take the cart's label off, or on the label's tab, search for one.
    Remove,
    /// X: ask to swap a worn cart for a new one.
    NewCart,
    Back,
}

#[derive(Clone, Debug)]
pub struct ShellPicker {
    pub rom: PathBuf,
    pub stem: String,
    was: Option<ShellChoice>,
    swatches: Vec<Swatch>,
    swatch: usize,
    /// The shapes the Model row offers, in its order.
    shapes: [Outline; 2],
    finish: usize,
    /// The finishes the row offers: an NES cart's has metallic in glitter's place.
    finishes: &'static [ShellFinish],
    tab: Tab,
    row: Row,
    /// Where the cursor stands on the row, when it has been moved off the chosen item: the
    /// arrows move it, A chooses what it stands on (Roy 2026-11-07).
    cursor: Option<usize>,
    fetch: Fetch,
    /// A label was looked for and none exists: the marker is offered instead of a fetch.
    no_label: bool,
    was_marker: Option<[u8; 3]>,
    /// The marker the cart's own shell is written in.
    own_marker: usize,
    marker: usize,
    opened: Millis,
    /// When the cart last changed its look, and which way it was stepped (-1 left, 1 right).
    poked: Option<(Millis, f32)>,
    /// When it began to close, and how far open it was then.
    closed: Option<(Millis, f32)>,
    /// An SNES cart's shape picked, and the one its region gives it.
    model: Option<(usize, usize)>,
    versions: Versions,
    /// The version of the label shown, once one is.
    version: Option<usize>,
    /// The versions' thumbnails by line, as the studio draws them (slot_ui::label_lines).
    label_lines: Vec<std::ops::Range<usize>>,
    /// Y asked to take the label off: A does, B does not.
    confirm: bool,
    /// The Super Game Boy: only its model to pick, from `SGB_MODELS`.
    sgb: bool,
    /// An original Game Boy game: no shape to pick, an older pick of the Color's dropped.
    no_shape: bool,
    /// The Game Boy slot's case picked and the one it had, when the Slot tab is offered.
    slot: Option<(usize, usize)>,
    /// How many cases the slot studio offers, per model (DMG, Pocket, Light), in order.
    slot_groups: Vec<usize>,
    /// Which of the slot studio's cases are clear, in the same order.
    slot_finishes: Vec<slot_ui::SlotFinish>,
    /// The slot studio is open instead of the cart's (L1/R1, Roy).
    slot_mode: bool,
    /// When the studios last switched.
    mode_at: Option<Millis>,
    /// When the slot last changed its case, and how hard it hops for it.
    slot_poke: Option<(Millis, f32)>,
    /// The cart's dust and wear levels (0 to 3), shown on the Wear tab.
    wear: (u8, u8),
    /// Y asked to swap the cart for a new one: A does, B does not.
    swap: bool,
    /// Where the cart's own shell, the one it really shipped in, sits on the colour and finish
    /// rows: kept there, nothing is written down (Roy 2026-11-07).
    real: Option<(usize, usize)>,
    /// Original's plastic: the console's standard cart, per shape the Model row offers (one
    /// when there is no row).
    standards: Vec<([u8; 3], ShellFinish)>,
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
            finishes: FINISHES,
            tab: Tab::Colour,
            wear: (0, 0),
            swap: false,
            // Every menu starts at its top: here the tabs' row, Color/Label/Marker (Roy 2026-11-07).
            row: Row::Tabs,
            cursor: None,
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
            poked: None,
            closed: None,
            model: None,
            versions: Versions::Unasked,
            version: None,
            label_lines: Vec::new(),
            confirm: false,
            sgb: false,
            no_shape: false,
            shapes: MODELS,
            slot: None,
            slot_groups: Vec::new(),
            slot_finishes: Vec::new(),
            slot_mode: false,
            mode_at: None,
            slot_poke: None,
            real: None,
            standards: Vec::new(),
        }
    }

    /// The cart's own shell and its console's standard one per shape (`standards`, in the
    /// Model row's order, or just one): the studio opens on the swatch of its own shell, a
    /// Pokémon Red on Pokémon Red, and Original is the plain standard cart.
    pub fn with_original(
        mut self,
        real: ([u8; 3], ShellFinish),
        standards: Vec<([u8; 3], ShellFinish)>,
    ) -> Self {
        if self.sgb || standards.is_empty() {
            return self;
        }
        self.standards = standards;
        let standard = self.standard();
        let finish_of = |finishes: &[ShellFinish], f: ShellFinish| {
            finishes.iter().position(|x| *x == f).unwrap_or(0)
        };
        let real_at = if real == standard {
            (0, 0)
        } else {
            let swatch = match self.swatches.iter().position(|s| s.colour() == Some(real.0)) {
                Some(i) => i,
                None => {
                    // Not a colour of the palette's: it gets a swatch of its own, after Original.
                    if let Some(Swatch::Custom(_)) = self.swatches.get(1) {
                        self.swatches.insert(2, Swatch::Custom(real.0));
                        if self.swatch >= 2 {
                            self.swatch += 1;
                        }
                        2
                    } else {
                        self.swatches.insert(1, Swatch::Custom(real.0));
                        if self.swatch >= 1 {
                            self.swatch += 1;
                        }
                        1
                    }
                }
            };
            (swatch, finish_of(self.finishes, real.1))
        };
        self.real = Some(real_at);
        match self.was.filter(|c| c.finish != ShellFinish::Own) {
            None => (self.swatch, self.finish) = real_at,
            // A pick of the standard cart itself is Original.
            Some(c) if (c.colour, c.finish) == standard => self.swatch = 0,
            Some(_) => {}
        }
        self
    }

    /// Original's plastic for the shape shown.
    fn standard(&self) -> ([u8; 3], ShellFinish) {
        let at = self.model.map_or(0, |(picked, _)| picked);
        self.standards
            .get(at)
            .or(self.standards.first())
            .copied()
            .unwrap_or(([0, 0, 0], ShellFinish::Own))
    }

    /// Offers the slot studio: the Game Boy slot's case `now`, of cases grouped per model.
    /// `finishes` is each case's finish: the studio offers them as their own row.
    pub fn with_slot_case(mut self, now: usize, groups: &[usize], finishes: &[slot_ui::SlotFinish]) -> Self {
        let count: usize = groups.iter().sum();
        self.slot = Some((now.min(count.saturating_sub(1)), now));
        self.slot_groups = groups.to_vec();
        self.slot_finishes = finishes.to_vec();
        self
    }

    fn slot_finish_of(&self, case: usize) -> slot_ui::SlotFinish {
        self.slot_finishes.get(case).copied().unwrap_or(slot_ui::SlotFinish::Solid)
    }

    /// The finishes the model shown has cases in, in the row's order.
    pub fn slot_finishes(&self) -> Vec<slot_ui::SlotFinish> {
        let Some((_, first, n)) = self.slot_group() else {
            return Vec::new();
        };
        slot_ui::SlotFinish::ALL
            .into_iter()
            .filter(|f| (first..first + n).any(|c| self.slot_finish_of(c) == *f))
            .collect()
    }

    /// The slot's finish shown, when its model has cases in more than one. None when the model
    /// has only the one kind.
    pub fn slot_finish(&self) -> Option<slot_ui::SlotFinish> {
        let (picked, _) = self.slot?;
        (self.slot_finishes().len() > 1).then(|| self.slot_finish_of(picked))
    }

    /// The cases on the colour row: the model's own, of the finish shown.
    pub fn slot_shown(&self) -> Vec<usize> {
        let (Some((picked, _)), Some((_, first, n))) = (self.slot, self.slot_group()) else {
            return Vec::new();
        };
        let finish = self.slot_finish_of(picked);
        (first..first + n).filter(|&c| self.slot_finish_of(c) == finish).collect()
    }

    /// How many models the slot studio offers: its model row shows only when more than one.
    pub fn slot_models(&self) -> usize {
        self.slot_groups.len()
    }

    /// The slot's model shown, where its cases start and how many it has.
    pub fn slot_group(&self) -> Option<(usize, usize, usize)> {
        let (picked, _) = self.slot?;
        let mut first = 0;
        for (g, &n) in self.slot_groups.iter().enumerate() {
            if picked < first + n {
                return Some((g, first, n));
            }
            first += n;
        }
        None
    }

    /// The slot's case shown: the one picked while the slot studio is offered.
    pub fn slot_case(&self) -> Option<usize> {
        self.slot.map(|(picked, _)| picked)
    }

    /// Seconds since the studios last switched, and since the slot last changed its case with
    /// how hard it hops: for the playful moves (Roy).
    pub fn slot_motion(&self, now: Millis) -> (Option<f32>, Option<(f32, f32)>) {
        let secs = |at: Millis| now.saturating_sub(at) as f32 / 1000.0;
        (self.mode_at.map(secs), self.slot_poke.map(|(at, k)| (secs(at), k)))
    }

    /// Whether the slot studio is open: None when it is not offered.
    pub fn slot_mode(&self) -> Option<bool> {
        self.slot.map(|_| self.slot_mode)
    }

    /// The Super Game Boy cart: its model is all there is to pick.
    pub fn with_sgb(mut self, now: slot_ui::SgbModel) -> Self {
        let picked = SGB_MODELS
            .iter()
            .position(|m| *m == now)
            .unwrap_or(SGB_ORIGINAL);
        self.model = Some((picked, SGB_ORIGINAL));
        self.sgb = true;
        self.row = Row::Model;
        self
    }

    pub fn sgb(&self) -> bool {
        self.sgb
    }

    fn model_count(&self) -> usize {
        if self.sgb {
            SGB_MODELS.len()
        } else {
            self.shapes.len()
        }
    }

    /// An SNES or Game Boy cart also picks its shape; `auto` is the one its region or folder
    /// gives it, in `MODELS`.
    pub fn with_model(mut self, auto: usize) -> Self {
        let picked = self
            .was
            .and_then(|c| self.shapes.iter().position(|o| *o == c.outline))
            .unwrap_or(auto);
        self.model = Some((picked, auto));
        self
    }

    /// A cart with no choice of shape: an older pick of another shape is dropped.
    pub fn without_model(mut self) -> Self {
        self.no_shape = true;
        self
    }

    /// An SNES cart: its shapes in `SNES_MODELS`, `auto` among them.
    pub fn with_snes_model(mut self, auto: usize) -> Self {
        self.shapes = SNES_MODELS;
        self.with_model(auto)
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

    /// An NES cart: metallic instead of glitter.
    pub fn with_metallic(mut self) -> Self {
        self.finishes = NES_FINISHES;
        if let Some(c) = self.was.filter(|c| c.finish != ShellFinish::Own) {
            self.finish = self
                .finishes
                .iter()
                .position(|f| *f == c.finish)
                .unwrap_or(0);
        }
        self
    }

    /// The offered finishes' names, as places in `FINISH_NAMES`.
    pub fn finish_faces(&self) -> Vec<usize> {
        if self.sgb {
            return Vec::new();
        }
        self.finishes
            .iter()
            .map(|f| FINISH_NAMES.iter().position(|(n, _)| n == f).unwrap_or(0))
            .collect()
    }

    pub fn row(&self) -> Row {
        self.row
    }

    /// The item the cursor stands on in the current row, in that row's own order.
    pub fn cursor(&self) -> usize {
        self.cursor.unwrap_or_else(|| self.chosen_at(self.row))
    }

    /// The chosen item of a row, in its order.
    fn chosen_at(&self, row: Row) -> usize {
        match row {
            Row::Tabs => self.tabs().iter().position(|t| *t == self.tab).unwrap_or(0),
            Row::Colour => self.swatch,
            Row::Finish => self.finish,
            Row::Model => self.model.map_or(0, |m| m.0),
            Row::Marker => self.marker,
            Row::Label => self.version.unwrap_or(0),
            Row::Slot => {
                let picked = self.slot.map_or(0, |s| s.0);
                self.slot_shown().iter().position(|&c| c == picked).unwrap_or(0)
            }
            Row::SlotFinish => match self.slot_finish() {
                Some(f) => self.slot_finishes().iter().position(|k| *k == f).unwrap_or(0),
                None => 0,
            },
            Row::SlotModel => self.slot_group().map_or(0, |g| g.0),
        }
    }

    /// How many items a row has to choose from.
    fn count(&self, row: Row) -> usize {
        match row {
            Row::Tabs => self.tabs().len(),
            Row::Colour => self.swatches.len(),
            Row::Finish => self.finishes.len(),
            Row::Model => self.model.map_or(0, |_| self.model_count()),
            Row::Marker => MARKERS.len(),
            Row::Label => match self.versions {
                Versions::Found(n) => n,
                _ => 0,
            },
            Row::Slot => self.slot_shown().len(),
            Row::SlotFinish => self.slot_finishes().len(),
            Row::SlotModel => self.slot_groups.len(),
        }
    }

    pub fn fetch(&self) -> Fetch {
        self.fetch
    }

    pub fn tab(&self) -> Tab {
        self.tab
    }

    /// The tabs along the top: colour and label, and the marker for a cart with no label.
    pub fn tabs(&self) -> Vec<Tab> {
        // The Super Game Boy has its sticker: no colour or label to pick.
        if self.sgb {
            return Vec::new();
        }
        let mut tabs = vec![Tab::Colour, Tab::Label];
        if self.offers_marker() {
            tabs.push(Tab::Marker);
        }
        tabs
    }

    pub fn versions(&self) -> Versions {
        self.versions
    }

    /// The version of the label shown in place of the cart's own, if one is.
    pub fn version(&self) -> Option<usize> {
        self.version
    }

    /// The versions came back: how many, or None when they could not be looked for. The
    /// first is shown at once, as the tab is there to pick one.
    pub fn found(&mut self, count: Option<usize>) -> Option<usize> {
        self.versions = match count {
            Some(n) if n > 0 => Versions::Found(n),
            _ => Versions::Missing,
        };
        if matches!(self.versions, Versions::Found(_)) && self.tab == Tab::Label {
            self.version = Some(0);
        }
        self.version
    }

    /// The versions' thumbnails by line, as the studio draws them.
    pub fn set_label_lines(&mut self, lines: Vec<std::ops::Range<usize>>) {
        self.label_lines = lines;
    }

    /// The versions on offer changed, as an SNES cart's shape picked anew offers only the
    /// labels made for it: how many there are now, and whether the first must be shown, as
    /// the label the cart wears does not fit the shape. The version shown, if one is.
    pub fn refound(&mut self, n: usize, show_first: bool) -> Option<usize> {
        if matches!(self.versions, Versions::Unasked | Versions::Looking) {
            return self.version;
        }
        self.versions = if n > 0 {
            Versions::Found(n)
        } else {
            Versions::Missing
        };
        self.version = (n > 0 && (show_first || self.tab == Tab::Label)).then_some(0);
        self.version
    }

    /// The label's versions are looked up now, unasked for by the tab: true when they were
    /// not yet.
    pub fn look_up_versions(&mut self) -> bool {
        if self.versions != Versions::Unasked {
            return false;
        }
        self.versions = Versions::Looking;
        true
    }

    /// For an SNES cart, whether the shape picked is the PAL cart.
    pub fn snes_pal(&self) -> Option<bool> {
        let (picked, _) = self
            .model
            .filter(|_| !self.sgb && self.shapes == SNES_MODELS)?;
        Some(SNES_MODELS[picked] == Outline::Rounded)
    }

    /// Y takes off the label the cart wears; a label shown from the label's tab is not on yet,
    /// and on that tab Y searches instead.
    pub fn offers_remove(&self) -> bool {
        self.fetch == Fetch::Labelled
            && self.version.is_none()
            && !matches!(self.tab, Tab::Label)
    }

    /// Y on the label's tab, once its versions are in or none were: search by name.
    pub fn offers_search(&self) -> bool {
        !self.slot_mode && self.tab == Tab::Label && !matches!(self.versions, Versions::Unasked | Versions::Looking)
    }

    /// A game was picked from a search: its versions are looked for, and the cart wears its
    /// own label again until they come.
    pub fn searched(&mut self) {
        self.versions = Versions::Looking;
        self.version = None;
        self.tab = Tab::Label;
        self.row = Row::Label;
    }

    /// Asking whether to take the label off.
    pub fn confirming(&self) -> bool {
        self.confirm
    }

    /// The cart springs up and rocks, as for a new look: it was swapped for a fresh one.
    pub fn hop(&mut self, now: Millis) {
        self.poked = Some((now, 1.0));
    }

    pub fn set_wear(&mut self, dust: u8, wear: u8) {
        self.wear = (dust, wear);
    }

    pub fn wear(&self) -> (u8, u8) {
        self.wear
    }

    /// X, under a worn cart, asks to swap it for a new one.
    pub fn offers_new_cart(&self) -> bool {
        (self.wear.0 > 0 || self.wear.1 > 0) && !self.sgb && !self.slot_mode
    }

    /// Y asked: A swaps the cart, B does not.
    pub fn swapping(&self) -> bool {
        self.swap
    }

    pub fn marker(&self) -> usize {
        self.marker
    }

    pub fn was_marker(&self) -> Option<[u8; 3]> {
        self.was_marker
    }

    /// The marker's tab, for any cart without a label, looked for yet or not.
    pub fn offers_marker(&self) -> bool {
        self.fetch != Fetch::Labelled
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
        if let (true, Some((picked, _))) = (self.sgb, self.model) {
            return sgb_choice(SGB_MODELS[picked]);
        }
        // The shape is only written down when it is not the one the region gives.
        let outline = match self.model {
            Some((picked, auto)) if picked != auto => self.shapes[picked],
            Some(_) => Outline::Auto,
            None if self.no_shape => Outline::Auto,
            None => self.was.map_or(Outline::Auto, |c| c.outline),
        };
        let mut colour = self.swatches[self.swatch].colour();
        let mut finish = self.finishes[self.finish];
        if !self.standards.is_empty() {
            let auto_shape = self.model.is_none_or(|(picked, auto)| picked == auto);
            let at = (self.swatch, self.finish);
            let own = match self.real {
                // Original is the cart's own when that is the standard one, whatever finish.
                Some((0, _)) => self.swatch == 0,
                Some(real) => at == real,
                None => false,
            };
            if own && auto_shape {
                colour = None;
            } else if self.swatch == 0 {
                let (c, f) = self.standard();
                colour = Some(c);
                finish = f;
            }
        }
        match colour {
            Some(colour) => Some(ShellChoice {
                outline,
                colour,
                finish,
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
            if self.row != Row::Tabs {
                self.row = Row::Marker;
            }
        }
    }

    fn rows(&self) -> Vec<Row> {
        if self.sgb {
            return vec![Row::Model];
        }
        if self.slot_mode {
            // A model row with only one model to pick is left out (Roy: GBC, GBA).
            let mut rows = Vec::new();
            if self.slot_groups.len() > 1 {
                rows.push(Row::SlotModel);
            }
            rows.push(Row::Slot);
            if self.slot_finish().is_some() {
                rows.push(Row::SlotFinish);
            }
            return rows;
        }
        // The cart's own tabs (Color, Label, Marker) are a row over the colours, reached with
        // the arrows; L1/R1 turn the studios' tabs, Cart and Slot (Roy 2026-11-07).
        let mut rows = vec![];
        if self.tabs().len() > 1 {
            rows.push(Row::Tabs);
        }
        match self.tab {
            Tab::Colour => {
                // The cart's shape first, over the colours (Roy).
                if self.model.is_some() {
                    rows.push(Row::Model);
                }
                rows.push(Row::Colour);
                if self.finish_on() {
                    rows.push(Row::Finish);
                }
            }
            Tab::Label => rows.push(Row::Label),
            Tab::Marker => rows.push(Row::Marker),
        }
        rows
    }

    /// The top row: where every studio starts (Roy 2026-11-07).
    fn top_row(&self) -> Row {
        self.rows().first().copied().unwrap_or(Row::Tabs)
    }

    fn changed(&self) -> bool {
        self.choice() != self.was
            || self.marker_choice() != self.was_marker
            || self.version.is_some()
            || self.slot.is_some_and(|(picked, was)| picked != was)
    }

    pub fn press(&mut self, press: Press, now: Millis) -> Outcome {
        let look = |p: &Self| {
            (p.swatch(), p.finish(), p.model(), p.version(), p.marker(), p.slot_case())
        };
        let before = look(self);
        let dir = if matches!(press, Press::Left) { -1.0 } else { 1.0 };
        let outcome = self.press_step(press, now);
        if look(self) != before {
            self.poked = Some((now, dir));
        }
        outcome
    }

    /// Seconds since the cart last changed its look and the way it was stepped, and seconds
    /// since the picker opened: for the cart to answer a change and to breathe.
    pub fn motion(&self, now: Millis) -> (Option<(f32, f32)>, f32) {
        let secs = |at: Millis| now.saturating_sub(at) as f32 / 1000.0;
        (self.poked.map(|(at, dir)| (secs(at), dir)), secs(self.opened))
    }

    fn step_tab(&mut self, by: isize) -> Outcome {
        let tabs = self.tabs();
        let at = tabs.iter().position(|t| *t == self.tab).unwrap_or(0);
        self.tab = tabs[(at as isize + by).clamp(0, tabs.len() as isize - 1) as usize];
        if self.tab == Tab::Label && self.versions == Versions::Unasked {
            self.versions = Versions::Looking;
            return Outcome::Versions;
        }
        if self.tab == Tab::Label && self.version.is_none() {
            if let Versions::Found(_) = self.versions {
                self.version = Some(0);
                return Outcome::Version(0);
            }
        }
        Outcome::None
    }

    fn press_step(&mut self, press: Press, now: Millis) -> Outcome {
        if self.closed.is_some() {
            return Outcome::None;
        }
        if self.swap || self.confirm {
            return self.act(press, now);
        }
        let row = self.row;
        let n = self.count(row);
        let at = self.cursor();
        let cols = match row {
            Row::Colour if !self.sgb => slot_ui::SWATCH_COLS,
            Row::Finish if !self.sgb && self.finishes.len() > 2 => 2,
            _ => 0,
        };
        // The arrows only move the cursor along a row of choices; A chooses (Roy 2026-11-07).
        let to = match press {
            _ if row == Row::Tabs || n == 0 => None,
            Press::Left => Some(at.saturating_sub(1)),
            Press::Right => Some((at + 1).min(n - 1)),
            // A grid's lines first, as before (Roy 2026-11-06).
            Press::Up if cols > 0 && at >= cols => Some(at - cols),
            Press::Down if cols > 0 && at / cols < (n - 1) / cols => Some((at + cols).min(n - 1)),
            // The labels wrap into lines of their own widths: up and down go to the one as far
            // along on the line above or below, as through the colours (Roy 2026-11-14).
            Press::Up | Press::Down if row == Row::Label => {
                let lines = &self.label_lines;
                lines.iter().position(|r| r.contains(&at)).and_then(|l| {
                    let to = match press {
                        Press::Up => l.checked_sub(1)?,
                        _ => Some(l + 1).filter(|&t| t < lines.len())?,
                    };
                    let r = &lines[to];
                    Some((r.start + (at - lines[l].start)).min(r.end - 1).min(n - 1))
                })
            }
            _ => None,
        };
        if let Some(to) = to {
            self.cursor = (to != self.chosen_at(row)).then_some(to);
            return Outcome::None;
        }
        match press {
            Press::Keep if row != Row::Tabs && n > 0 => return self.choose(at, now),
            Press::Keep => return Outcome::None,
            // B goes back with what was chosen kept (Roy 2026-11-07).
            Press::Back => return self.act(Press::Keep, now),
            Press::Left | Press::Right if row != Row::Tabs => return Outcome::None,
            Press::Up | Press::Down => {
                // Off the row: what the cursor stood on and was not chosen is let go.
                self.cursor = None;
                let rows = self.rows();
                let i = rows.iter().position(|r| *r == row).unwrap_or(0) as isize;
                let by = if matches!(press, Press::Up) { -1 } else { 1 };
                self.row = rows[(i + by).clamp(0, rows.len() as isize - 1) as usize];
                return Outcome::None;
            }
            _ => {}
        }
        self.cursor = None;
        self.act(press, now)
    }

    /// A chooses the item under the cursor: the row steps there the way the arrows once did,
    /// so each row picks just as before.
    fn choose(&mut self, to: usize, now: Millis) -> Outcome {
        self.cursor = None;
        let mut outcome = Outcome::None;
        for _ in 0..self.count(self.row) {
            let at = self.chosen_at(self.row);
            if at == to {
                break;
            }
            let press = if to < at { Press::Left } else { Press::Right };
            outcome = self.act(press, now);
            if self.chosen_at(self.row) == at {
                break;
            }
        }
        outcome
    }

    fn act(&mut self, press: Press, now: Millis) -> Outcome {
        if self.swap {
            // X again swaps the cart; anything else lets the question go.
            self.swap = false;
            match press {
                Press::NewCart => {
                    self.wear = (0, 0);
                    return Outcome::NewCart;
                }
                Press::Back => return Outcome::None,
                _ => {}
            }
        }
        if self.confirm {
            match press {
                Press::Keep => {
                    self.confirm = false;
                    self.fetch = Fetch::Idle;
                    self.no_label = true;
                    return Outcome::RemoveLabel;
                }
                Press::Back => self.confirm = false,
                _ => {}
            }
            return Outcome::None;
        }
        let step =
            |at: usize, by: isize, n: usize| (at as isize + by).clamp(0, n as isize - 1) as usize;
        match press {
            Press::Left | Press::Right => {
                let by = if matches!(press, Press::Left) { -1 } else { 1 };
                match self.row {
                    Row::Tabs => return self.step_tab(by),
                    Row::Slot => {
                        let Some((picked, was)) = self.slot else {
                            return Outcome::None;
                        };
                        // Along the model's own colours of the finish shown only.
                        let shown = self.slot_shown();
                        let at = shown.iter().position(|&c| c == picked).unwrap_or(0);
                        let to = shown.get(step(at, by, shown.len())).copied().unwrap_or(picked);
                        if to != picked {
                            self.slot = Some((to, was));
                            self.slot_poke = Some((now, 1.0));
                        }
                        return Outcome::None;
                    }
                    Row::SlotFinish => {
                        let (Some((picked, was)), Some(shown_f), Some((_, first, n))) =
                            (self.slot, self.slot_finish(), self.slot_group())
                        else {
                            return Outcome::None;
                        };
                        let kinds = self.slot_finishes();
                        let at = kinds.iter().position(|f| *f == shown_f).unwrap_or(0);
                        let to = kinds[step(at, by, kinds.len())];
                        if to != shown_f {
                            // The finish opens on the case kept, if it is one, else its first.
                            let pick = match (first..first + n).contains(&was) && self.slot_finish_of(was) == to {
                                true => was,
                                false => (first..first + n).find(|&c| self.slot_finish_of(c) == to).unwrap_or(picked),
                            };
                            self.slot = Some((pick, was));
                            self.slot_poke = Some((now, 1.4));
                        }
                        return Outcome::None;
                    }
                    Row::SlotModel => {
                        let (Some((_, was)), Some((g, _, _))) = (self.slot, self.slot_group())
                        else {
                            return Outcome::None;
                        };
                        let to = step(g, by, self.slot_groups.len());
                        if to != g {
                            // A model opens on the one kept, else on its first finish's first
                            // colour: Solid first, never Metallic Gold (Roy, st4).
                            let first: usize = self.slot_groups[..to].iter().sum();
                            let n = self.slot_groups[to];
                            let rank = |c: usize| {
                                let f = self.slot_finish_of(c);
                                slot_ui::SlotFinish::ALL.iter().position(|k| *k == f).unwrap_or(0)
                            };
                            let pick = match (first..first + n).contains(&was) {
                                true => was,
                                false => (first..first + n).min_by_key(|&c| rank(c)).unwrap_or(first),
                            };
                            self.slot = Some((pick, was));
                            self.slot_poke = Some((now, 1.8));
                        }
                        return Outcome::None;
                    }
                    Row::Label => {
                        let Versions::Found(n) = self.versions else {
                            return Outcome::None;
                        };
                        let at = self.version.unwrap_or(0);
                        let to = step(at, by, n);
                        if Some(to) == self.version {
                            return Outcome::None;
                        }
                        self.version = Some(to);
                        return Outcome::Version(to);
                    }
                    Row::Colour => {
                        let to = step(self.swatch, by, self.swatches.len());
                        if to == self.swatch {
                            return Outcome::None;
                        }
                        self.swatch = to;
                    }
                    Row::Finish => {
                        let to = step(self.finish, by, self.finishes.len());
                        if to == self.finish {
                            return Outcome::None;
                        }
                        self.finish = to;
                    }
                    Row::Model => {
                        let Some((picked, auto)) = self.model else {
                            return Outcome::None;
                        };
                        let to = step(picked, by, self.model_count());
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
            Press::TabLeft | Press::TabRight if self.slot.is_some() => {
                // L1 the cart's studio, R1 the slot's.
                let slot = matches!(press, Press::TabRight);
                if slot != self.slot_mode {
                    self.slot_mode = slot;
                    self.mode_at = Some(now);
                    // The other studio opens at its top row (Roy 2026-11-07).
                    self.row = self.top_row();
                }
                Outcome::None
            }
            // L1 R1 only turn the main tabs, Cart and Slot; Color, Label and Marker are a row
            // the arrows reach (Roy 2026-11-12, ui8).
            Press::TabLeft | Press::TabRight => Outcome::None,
            Press::Up | Press::Down => {
                // The colours are a grid: up and down go through its lines first, and only
                // leave it from its top or bottom line (Roy 2026-11-06).
                if self.row == Row::Colour && !self.sgb {
                    let cols = slot_ui::SWATCH_COLS;
                    let last = self.swatches.len().saturating_sub(1);
                    let to = match press {
                        Press::Up if self.swatch >= cols => Some(self.swatch - cols),
                        Press::Down if self.swatch / cols < last / cols => {
                            Some((self.swatch + cols).min(last))
                        }
                        _ => None,
                    };
                    if let Some(to) = to {
                        self.swatch = to;
                        return Outcome::Preview(self.choice());
                    }
                }
                // The finishes stand two to a line when there are more than two: the same.
                if self.row == Row::Finish && !self.sgb && self.finishes.len() > 2 {
                    let last = self.finishes.len() - 1;
                    let to = match press {
                        Press::Up if self.finish >= 2 => Some(self.finish - 2),
                        Press::Down if self.finish / 2 < last / 2 => Some((self.finish + 2).min(last)),
                        _ => None,
                    };
                    if let Some(to) = to {
                        self.finish = to;
                        return Outcome::Preview(self.choice());
                    }
                }
                let rows = self.rows();
                let at = rows.iter().position(|r| *r == self.row).unwrap_or(0);
                let by = if matches!(press, Press::Up) { -1 } else { 1 };
                self.row = rows[step(at, by, rows.len())];
                Outcome::None
            }
            Press::NewCart => {
                self.swap = self.offers_new_cart();
                Outcome::None
            }
            Press::Remove if self.offers_search() => Outcome::Search,
            Press::Remove => {
                self.confirm = self.offers_remove();
                Outcome::None
            }
            Press::Keep => {
                self.closed = Some((now, self.openness(now)));
                Outcome::Save(self.choice(), self.marker_choice())
            }
            Press::Back => {
                self.closed = Some((now, self.openness(now)));
                let changed = self.changed();
                // The slot goes back to its case as the studio closes.
                if let Some((_, was)) = self.slot {
                    self.slot = Some((was, was));
                }
                if !changed {
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
    fn a_cart_opens_on_its_own_shell_and_original_is_the_plain_cart() {
        let grey = ([0x9a, 0x97, 0x8f], ShellFinish::Solid);
        let red = ([0xc0, 0x28, 0x2c], ShellFinish::Solid);
        let mut p = picker(None, true).with_original(red, vec![grey]);
        assert_eq!(p.swatches()[p.swatch()].colour(), Some(red.0));
        assert_eq!(p.choice(), None);
        p.swatch = 0;
        let c = p.choice().expect("the plain grey cart is written down");
        assert_eq!((c.colour, c.finish), grey);
        let again = picker(Some(c), true).with_original(red, vec![grey]);
        assert_eq!(again.swatches()[again.swatch()], Swatch::Original);
        // A cart whose own shell is the plain one: Original keeps nothing written down.
        let plain = picker(None, true).with_original(grey, vec![grey]);
        assert_eq!(plain.swatches()[plain.swatch()], Swatch::Original);
        assert_eq!(plain.choice(), None);
        // A shell the palette lacks gets a swatch of its own.
        let odd = ([1, 2, 3], ShellFinish::Clear);
        let q = picker(None, true).with_original(odd, vec![grey]);
        assert_eq!(q.swatches()[q.swatch()], Swatch::Custom(odd.0));
        assert_eq!(q.choice(), None);
    }

    #[test]
    fn a_game_boy_game_drops_an_older_color_shape() {
        let was = ShellChoice {
            outline: Outline::Rounded,
            colour: [200, 40, 40],
            finish: ShellFinish::Solid,
        };
        let p = picker(Some(was), true).without_model();
        assert_eq!(p.model(), None, "no Model row");
        assert_eq!(p.choice().map(|c| c.outline), Some(Outline::Auto));
    }

    #[test]
    fn an_snes_cart_picks_its_shape_and_keeps_its_own_colour() {
        let mut p = picker(None, true).with_model(0);
        assert_eq!(
            p.choice(),
            None,
            "the region's own shape is not written down"
        );
        assert_eq!(p.row(), Row::Tabs, "the studio starts at its top");
        p.press(Press::Down, 0);
        assert_eq!(p.row(), Row::Model, "the shape stands over the colours");
        p.press(Press::Right, 0);
        assert_eq!(p.choice(), None, "the arrows only move the cursor");
        p.press(Press::Keep, 0);
        let c = p.choice().unwrap();
        assert_eq!((c.outline, c.finish), (Outline::Rounded, ShellFinish::Own));
        let again = picker(Some(c), true).with_model(0);
        assert_eq!(again.model(), Some(1));
        assert_eq!(again.swatches()[again.swatch()], Swatch::Original);
        assert_eq!(again.choice(), Some(c));
    }

    #[test]
    fn x_under_a_worn_cart_asks_and_x_again_swaps_it() {
        let mut p = picker(None, false);
        assert!(!p.offers_new_cart(), "a cart with no wear has nothing to swap");
        p.press(Press::NewCart, 0);
        assert!(!p.swapping());
        p.set_wear(2, 1);
        p.press(Press::NewCart, 0);
        assert!(p.swapping());
        p.press(Press::Back, 0);
        assert!(!p.swapping() && !p.closing());
        p.press(Press::NewCart, 0);
        assert!(matches!(p.press(Press::NewCart, 0), Outcome::NewCart));
        assert!(!p.offers_new_cart(), "new now");
    }

    #[test]
    fn a_colour_from_elsewhere_gets_a_swatch_of_its_own() {
        let was = ShellChoice::parse("rounded 123456 glitter");
        let p = picker(was, true);
        assert_eq!(p.swatches()[p.swatch()], Swatch::Custom([0x12, 0x34, 0x56]));
        assert_eq!(p.choice(), was);
    }

    #[test]
    fn every_studio_starts_at_its_top_row() {
        // Roy 2026-11-07: every menu starts at its top.
        let mut p = picker(None, true).with_slot_case(0, &[2, 2], &[]);
        assert_eq!(p.row(), Row::Tabs);
        p.press(Press::TabRight, 0);
        assert_eq!(p.row(), Row::SlotModel, "the slot's studio starts on its model row");
        p.press(Press::TabLeft, 0);
        assert_eq!(p.row(), Row::Tabs);
    }

    #[test]
    fn up_and_down_step_through_the_colours_grid() {
        let mut p = picker(None, false);
        p.row = Row::Colour;
        let cols = slot_ui::SWATCH_COLS;
        p.press(Press::Right, 0);
        assert!(matches!(p.press(Press::Down, 0), Outcome::None));
        assert_eq!((p.row(), p.cursor(), p.swatch()), (Row::Colour, 1 + cols, 0));
        p.press(Press::Up, 0);
        assert_eq!((p.row(), p.cursor()), (Row::Colour, 1));
        assert!(matches!(p.press(Press::Keep, 0), Outcome::Preview(Some(_))), "A chooses");
        assert_eq!(p.swatch(), 1);
        p.press(Press::Up, 0);
        assert_eq!(p.row(), Row::Tabs, "the cart's tabs are a row over the colours (Roy 2026-11-07)");
    }

    #[test]
    fn stepping_along_the_row_previews_and_down_reaches_the_finish() {
        let mut p = picker(None, false);
        p.row = Row::Colour;
        assert!(matches!(p.press(Press::Right, 0), Outcome::None));
        assert!(matches!(p.press(Press::Keep, 0), Outcome::Preview(Some(_))));
        assert!(!p.closing(), "A chooses and stays");
        // down the colours' grid line by line, then off its bottom onto the finish
        let lines = p.swatches().len().div_ceil(slot_ui::SWATCH_COLS);
        for _ in 0..lines {
            p.press(Press::Down, 0);
        }
        assert_eq!(p.row(), Row::Finish);
        p.press(Press::Right, 0);
        let Outcome::Preview(Some(c)) = p.press(Press::Keep, 0) else {
            panic!("no preview");
        };
        assert_eq!(c.finish, ShellFinish::Clear, "Solid, then Clear beside it");
    }

    #[test]
    fn back_keeps_what_was_chosen_and_not_what_the_cursor_is_on() {
        let was = ShellChoice::parse("auto f2c230 solid");
        let mut p = picker(was, true);
        p.row = Row::Colour;
        p.press(Press::Right, 0);
        p.press(Press::Keep, 0);
        let picked = p.choice();
        assert_ne!(picked, was);
        p.press(Press::Right, 0);
        assert!(matches!(p.press(Press::Back, 0), Outcome::Save(c, None) if c == picked));
        assert!(p.finished(1000));
    }

    #[test]
    fn a_labelled_cart_offers_no_marker() {
        let mut p = picker(ShellChoice::parse("auto f2c230 solid"), true);
        assert_eq!(
            p.tabs(),
            [Tab::Colour, Tab::Label],
            "no marker on a labelled cart"
        );
    }

    #[test]
    fn a_cart_with_no_label_offers_a_marker_before_any_is_looked_for() {
        let mut p = picker(None, false);
        assert!(p.offers_marker());
        assert_eq!(p.tabs(), [Tab::Colour, Tab::Label, Tab::Marker]);
        assert_eq!(p.row(), Row::Tabs);
        p.press(Press::Right, 0);
        p.press(Press::Right, 0);
        p.press(Press::Down, 0);
        assert_eq!((p.tab(), p.row()), (Tab::Marker, Row::Marker));
        assert_eq!(p.marker_choice(), None, "opens on the shell's own marker");
        p.press(Press::Right, 0);
        let Outcome::Marker(Some(ink)) = p.press(Press::Keep, 0) else {
            panic!("no marker preview");
        };
        assert_eq!(ink, MARKERS[1].1);
        assert!(matches!(p.press(Press::Back, 0), Outcome::Save(None, Some(i)) if i == ink));
    }

    #[test]
    fn y_asks_before_the_label_comes_off_and_b_keeps_it() {
        let mut p = picker(None, true);
        assert!(p.offers_remove());
        assert!(matches!(p.press(Press::Remove, 0), Outcome::None));
        assert!(p.confirming());
        assert!(
            matches!(p.press(Press::Right, 0), Outcome::None),
            "nothing else moves"
        );
        assert!(matches!(p.press(Press::Back, 0), Outcome::None));
        assert!(!p.confirming() && !p.closing(), "B only keeps the label");
        p.press(Press::Remove, 0);
        assert!(matches!(p.press(Press::Keep, 0), Outcome::RemoveLabel));
        assert!(!p.closing(), "the studio stays open");
        assert!(!p.offers_remove());
        assert_eq!(
            p.tabs(),
            [Tab::Colour, Tab::Label, Tab::Marker],
            "the marker is offered"
        );
    }

    #[test]
    fn y_on_the_label_tab_searches_once_the_versions_are_in() {
        let mut p = picker(None, true);
        assert!(matches!(p.press(Press::Right, 0), Outcome::Versions));
        assert!(
            matches!(p.press(Press::Remove, 0), Outcome::None),
            "still looking"
        );
        p.found(Some(0));
        assert!(!p.offers_remove());
        assert!(matches!(p.press(Press::Remove, 0), Outcome::Search));
        assert!(!p.confirming());
        p.searched();
        assert_eq!(p.versions(), Versions::Looking);
    }

    #[test]
    fn a_cart_with_no_label_has_nothing_to_remove() {
        let mut p = picker(None, false);
        assert!(!p.offers_remove());
        p.press(Press::Remove, 0);
        assert!(!p.confirming());
    }

    #[test]
    fn without_a_slot_l1_r1_leave_the_tabs_and_the_arrows_turn_them() {
        let mut p = ShellPicker::open("a.gb".into(), "a".into(), None, false, true, None, false, 0);
        p.press(Press::Up, 0);
        assert_eq!(p.row(), Row::Tabs);
        p.press(Press::TabRight, 0);
        assert_eq!(p.tab(), Tab::Colour, "L1 R1 are for Cart and Slot only");
        p.press(Press::Right, 0);
        p.press(Press::Right, 0);
        assert_eq!((p.tab(), p.row()), (Tab::Marker, Row::Tabs));
        p.press(Press::Left, 0);
        p.press(Press::Left, 0);
        assert_eq!(p.tab(), Tab::Colour);
    }

    #[test]
    fn up_and_down_step_through_the_label_lines() {
        let mut p = picker(None, true);
        p.press(Press::Right, 0);
        p.found(Some(5));
        p.set_label_lines(vec![0..3, 3..5]);
        p.press(Press::Down, 0);
        assert_eq!(p.row(), Row::Label);
        p.press(Press::Right, 0);
        p.press(Press::Right, 0);
        p.press(Press::Down, 0);
        assert_eq!(p.row(), Row::Label, "down to the next line, not off the row");
        assert!(matches!(p.press(Press::Keep, 0), Outcome::Version(4)), "as far along, kept in the line");
        p.press(Press::Left, 0);
        p.press(Press::Up, 0);
        assert!(matches!(p.press(Press::Keep, 0), Outcome::Version(0)));
        p.press(Press::Up, 0);
        assert_eq!(p.row(), Row::Tabs, "the top line leaves the row");
    }

    #[test]
    fn the_label_tab_looks_up_versions_once_and_shows_the_first() {
        let mut p = picker(None, true);
        assert_eq!(p.row(), Row::Tabs);
        assert!(matches!(p.press(Press::Right, 0), Outcome::Versions));
        assert_eq!((p.tab(), p.versions()), (Tab::Label, Versions::Looking));
        assert_eq!(p.found(Some(3)), Some(0), "the first is shown at once");
        p.press(Press::Down, 0);
        assert_eq!(p.row(), Row::Label);
        p.press(Press::Right, 0);
        assert!(matches!(p.press(Press::Keep, 0), Outcome::Version(1)));
        p.press(Press::Up, 0);
        p.press(Press::Left, 0);
        assert!(
            matches!(p.press(Press::Right, 0), Outcome::None),
            "asked once, still shown"
        );
        assert_eq!(p.version(), Some(1));
        assert!(
            matches!(p.press(Press::Back, 0), Outcome::Save(None, None)),
            "B keeps the version chosen"
        );
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

    #[test]
    fn the_label_tab_is_reached_with_the_arrows() {
        // With the slot's studio offered L1/R1 turn Cart and Slot; the cart's own tabs are a
        // row over the colours (Roy 2026-11-07: the label tab could not be reached).
        let mut p = picker(None, true).with_slot_case(0, &[2], &[]);
        assert_eq!(p.rows().first(), Some(&Row::Tabs));
        assert_eq!(p.row, Row::Tabs, "the studio starts on its tabs (Roy 2026-11-07)");
        p.press(Press::Up, 0);
        assert_eq!(p.row, Row::Tabs);
        assert!(matches!(p.press(Press::Right, 0), Outcome::Versions));
        assert_eq!(p.tab(), Tab::Label);
        p.press(Press::TabRight, 0);
        assert!(p.slot_mode);
        p.press(Press::TabLeft, 0);
        assert_eq!((p.tab(), p.slot_mode), (Tab::Label, false));
    }
}
