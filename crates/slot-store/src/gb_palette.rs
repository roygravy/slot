/// The Game Boy's own screens only: slot.'s DMG, the Pocket and the Light. The GBC and SGB
/// palettes looked wrong under the Game Boy's LCD filter; with Color Palettes off a game
/// still gets its own GBC colours.
const NAMES: [&str; 3] = ["DMG Green", "GB Pocket", "GB Light"];

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct GbPalette(u8);

impl GbPalette {
    pub const COUNT: usize = NAMES.len();
    pub const DEFAULT: GbPalette = GbPalette(0);

    pub fn all() -> impl Iterator<Item = GbPalette> {
        (0..Self::COUNT as u8).map(GbPalette)
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub fn core_name(self) -> &'static str {
        NAMES[self.index()]
    }

    pub fn label(self) -> &'static str {
        let name = self.core_name();
        // slot.'s own DMG colours, named like the other Game Boy palettes.
        if name == "DMG Green" {
            return "GB DMG";
        }
        match name.find(['↑', '↓', '←', '→']) {
            Some(at) => name[..at].trim_end(),
            None => name,
        }
    }

    pub fn next(self) -> GbPalette {
        GbPalette(((self.index() + 1) % Self::COUNT) as u8)
    }

    pub fn prev(self) -> GbPalette {
        GbPalette(((self.index() + Self::COUNT - 1) % Self::COUNT) as u8)
    }

    pub fn parse(name: &str) -> Option<GbPalette> {
        Self::all().find(|p| p.core_name() == name)
    }
}
