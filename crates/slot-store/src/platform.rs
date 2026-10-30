use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Platform {
    #[default]
    Gba,
    Gb,
    Gbc,
    Nes,
    Snes,
}

/// A TV console's frame: its 256 columns as the composite signal smears them, which the
/// core's NTSC filter draws 602 wide.
pub const TV_CANVAS: (u32, u32) = (602, 224);

impl Platform {
    pub const ALL: [Platform; 5] = [
        Platform::Gb,
        Platform::Gbc,
        Platform::Gba,
        Platform::Nes,
        Platform::Snes,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Platform::Gba => "Game Boy Advance",
            Platform::Gb => "Game Boy",
            Platform::Gbc => "Game Boy Color",
            Platform::Nes => "NES",
            Platform::Snes => "Super Nintendo",
        }
    }

    pub fn dir_name(self) -> &'static str {
        match self {
            Platform::Gba => "GBA",
            Platform::Gb => "GB",
            Platform::Gbc => "GBC",
            Platform::Nes => "NES",
            Platform::Snes => "SNES",
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Platform::Gba => &["gba"],
            Platform::Gb | Platform::Gbc => &["gb", "gbc"],
            Platform::Nes => &["nes"],
            Platform::Snes => &["sfc", "smc"],
        }
    }

    pub fn picture(self) -> (u32, u32) {
        match self {
            Platform::Gba => (240, 160),
            Platform::Gb | Platform::Gbc => (160, 144),
            Platform::Nes | Platform::Snes => TV_CANVAS,
        }
    }

    /// The frame every picture is drawn into: the Game Boys share the GBA's, centred in it;
    /// the TV consoles have their own, a 4:3 picture that fills the screen.
    pub fn canvas(self) -> (u32, u32) {
        match self {
            Platform::Gba | Platform::Gb | Platform::Gbc => (240, 160),
            Platform::Nes | Platform::Snes => TV_CANVAS,
        }
    }

    /// A TV console: its picture fills the screen, with no handheld's LCD around it.
    pub fn tv(self) -> bool {
        matches!(self, Platform::Nes | Platform::Snes)
    }

    pub fn accepts(self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                self.extensions()
                    .iter()
                    .any(|k| ext.eq_ignore_ascii_case(k))
            })
    }
}
