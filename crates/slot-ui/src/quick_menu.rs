use crate::draw::{Canvas, Draw, TexId, OUT_H, OUT_W};
use crate::plate::{
    arrows_hint_face, centred_hints, hint_face, tint_mask, UndoFace, HINT_H, LEGEND_GAP,
};
use crate::power_menu::{MENU_H, MENU_INK, MENU_PAD};
use crate::text;
use crate::theme::{self, Ink};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum QuickRow {
    FastForward,
    FastForwardSound,
    Screen,
    Rumble,
    CheatIcon,
    GbcBlackBorder,
    GbBlackBorder,
    BlackBorder,
    GbTransparency,
    GbcTransparency,
    GbaTransparency,
    GbGhosting,
    GbcGhosting,
    GbaGhosting,
    DateTime,
    About,
    GbaShader,
    GbShader,
    Game,
    EjectSave,
    Turbo,
    Rewind,
    GbPalettes,
    Theme,
    Wallpaper,
    General,
    Recents,
    ShowTitles,
    WallpaperTint,
    GbcShader,
    Resume,
    States,
    Cheats,
    Settings,
    ExitGame,
    Screenshots,
    GameScreenshots,
    PlayScreen,
    PlayGameplay,
    Shader,
    Palette,
    ScreenGba,
    GbBezel,
    GbcBezel,
    Bezel,
    GbaBezel,
    ScreenGbc,
    ScreenGb,
    AutoSleep,
    DeepSleep,
    PixelTransparency,
    LcdGhosting,
    InGameMenu,
    RememberPosition,
    ResumeGame,
    BootPicture,
    FavoriteMark,
    Buttons,
    Sleep,
    Restart,
    PowerOff,
    HomeWifi,
    Network,
    ConnectedTo,
    AddNetwork,
    SavedNetworks,
    Wps,
    RomManager,
    AutoTime,
    TimeZone,
    Clock24,
    ScreenNes,
    ScreenSnes,
    NesCable,
    NesColors,
    NesScanlines,
    NesMask,
    NesAfterglow,
    SnesCable,
    SnesScanlines,
    SnesMask,
    SnesAfterglow,
    Cable,
    Colors,
    Scanlines,
    Mask,
    Afterglow,
    SgbBorder,
}

impl QuickRow {
    pub const ALL: [QuickRow; 88] = [
        QuickRow::FastForward,
        QuickRow::FastForwardSound,
        QuickRow::Screen,
        QuickRow::Rumble,
        QuickRow::CheatIcon,
        QuickRow::GbcBlackBorder,
        QuickRow::GbBlackBorder,
        QuickRow::BlackBorder,
        QuickRow::GbTransparency,
        QuickRow::GbcTransparency,
        QuickRow::GbaTransparency,
        QuickRow::GbGhosting,
        QuickRow::GbcGhosting,
        QuickRow::GbaGhosting,
        QuickRow::DateTime,
        QuickRow::About,
        QuickRow::GbaShader,
        QuickRow::GbShader,
        QuickRow::Game,
        QuickRow::EjectSave,
        QuickRow::Turbo,
        QuickRow::Rewind,
        QuickRow::GbPalettes,
        QuickRow::Theme,
        QuickRow::Wallpaper,
        QuickRow::General,
        QuickRow::Recents,
        QuickRow::ShowTitles,
        QuickRow::WallpaperTint,
        QuickRow::GbcShader,
        QuickRow::Resume,
        QuickRow::States,
        QuickRow::Cheats,
        QuickRow::Settings,
        QuickRow::ExitGame,
        QuickRow::Screenshots,
        QuickRow::GameScreenshots,
        QuickRow::PlayScreen,
        QuickRow::PlayGameplay,
        QuickRow::Shader,
        QuickRow::Palette,
        QuickRow::ScreenGba,
        QuickRow::GbBezel,
        QuickRow::GbcBezel,
        QuickRow::Bezel,
        QuickRow::GbaBezel,
        QuickRow::ScreenGbc,
        QuickRow::ScreenGb,
        QuickRow::AutoSleep,
        QuickRow::DeepSleep,
        QuickRow::PixelTransparency,
        QuickRow::LcdGhosting,
        QuickRow::InGameMenu,
        QuickRow::RememberPosition,
        QuickRow::ResumeGame,
        QuickRow::BootPicture,
        QuickRow::FavoriteMark,
        QuickRow::Buttons,
        QuickRow::Sleep,
        QuickRow::Restart,
        QuickRow::PowerOff,
        QuickRow::HomeWifi,
        QuickRow::Network,
        QuickRow::ConnectedTo,
        QuickRow::AddNetwork,
        QuickRow::SavedNetworks,
        QuickRow::Wps,
        QuickRow::RomManager,
        QuickRow::AutoTime,
        QuickRow::TimeZone,
        QuickRow::Clock24,
        QuickRow::ScreenNes,
        QuickRow::ScreenSnes,
        QuickRow::NesCable,
        QuickRow::NesColors,
        QuickRow::NesScanlines,
        QuickRow::NesMask,
        QuickRow::NesAfterglow,
        QuickRow::SnesCable,
        QuickRow::SnesScanlines,
        QuickRow::SnesMask,
        QuickRow::SnesAfterglow,
        QuickRow::Cable,
        QuickRow::Colors,
        QuickRow::Scanlines,
        QuickRow::Mask,
        QuickRow::Afterglow,
        QuickRow::SgbBorder,
    ];

    /// The power menu, held POWER.
    pub const POWER: [QuickRow; 3] = [QuickRow::Sleep, QuickRow::Restart, QuickRow::PowerOff];

    pub const MAIN: [QuickRow; 7] = [
        QuickRow::General,
        QuickRow::Screen,
        QuickRow::Game,
        QuickRow::Network,
        QuickRow::Screenshots,
        QuickRow::Buttons,
        QuickRow::About,
    ];

    pub const GENERAL: [QuickRow; 13] = [
        QuickRow::Wallpaper,
        QuickRow::Theme,
        QuickRow::Recents,
        QuickRow::FavoriteMark,
        QuickRow::ShowTitles,
        QuickRow::AutoTime,
        QuickRow::TimeZone,
        QuickRow::Clock24,
        QuickRow::DateTime,
        QuickRow::AutoSleep,
        QuickRow::DeepSleep,
        QuickRow::RememberPosition,
        QuickRow::ResumeGame,
    ];

    /// Home Wi-Fi and what it is connected to.
    pub const NETWORK: [QuickRow; 6] = [
        QuickRow::HomeWifi,
        QuickRow::ConnectedTo,
        QuickRow::AddNetwork,
        QuickRow::SavedNetworks,
        QuickRow::Wps,
        QuickRow::RomManager,
    ];

    /// Every screen setting belongs to one system: nothing here reaches across all three.
    pub const SCREEN: [QuickRow; 5] = [
        QuickRow::ScreenGb,
        QuickRow::ScreenGbc,
        QuickRow::ScreenGba,
        QuickRow::ScreenNes,
        QuickRow::ScreenSnes,
    ];

    pub const SCREEN_NES: [QuickRow; 5] = [
        QuickRow::NesCable,
        QuickRow::NesColors,
        QuickRow::NesScanlines,
        QuickRow::NesMask,
        QuickRow::NesAfterglow,
    ];

    pub const SCREEN_SNES: [QuickRow; 4] = [
        QuickRow::SnesCable,
        QuickRow::SnesScanlines,
        QuickRow::SnesMask,
        QuickRow::SnesAfterglow,
    ];

    pub const SCREEN_GBA: [QuickRow; 3] = [
        QuickRow::GbaShader,
        QuickRow::GbaTransparency,
        QuickRow::GbaGhosting,
    ];

    pub const SCREEN_GBC: [QuickRow; 4] = [
        QuickRow::GbcShader,
        QuickRow::GbcTransparency,
        QuickRow::GbcGhosting,
        QuickRow::GbcBlackBorder,
    ];

    pub const SCREEN_GB: [QuickRow; 5] = [
        QuickRow::GbShader,
        QuickRow::GbPalettes,
        QuickRow::GbTransparency,
        QuickRow::GbGhosting,
        QuickRow::GbBlackBorder,
    ];

    pub const GAME: [QuickRow; 8] = [
        QuickRow::FastForward,
        QuickRow::FastForwardSound,
        QuickRow::Rewind,
        QuickRow::Turbo,
        QuickRow::Rumble,
        QuickRow::InGameMenu,
        QuickRow::EjectSave,
        QuickRow::CheatIcon,
    ];

    pub const PLAY: [QuickRow; 6] = [
        QuickRow::Resume,
        QuickRow::States,
        QuickRow::GameScreenshots,
        QuickRow::Cheats,
        QuickRow::Settings,
        QuickRow::ExitGame,
    ];

    pub const SETTINGS: [QuickRow; 2] = [QuickRow::PlayScreen, QuickRow::PlayGameplay];

    /// The in-game Screen page: the playing system's own settings, through rows that stand
    /// for whichever system is in the slot.
    /// The TV consoles' rows come after the handhelds' and only one set is shown.
    pub const PLAY_SCREEN: [QuickRow; 11] = [
        QuickRow::SgbBorder,
        QuickRow::Shader,
        QuickRow::Palette,
        QuickRow::PixelTransparency,
        QuickRow::LcdGhosting,
        QuickRow::BlackBorder,
        QuickRow::Cable,
        QuickRow::Colors,
        QuickRow::Scanlines,
        QuickRow::Mask,
        QuickRow::Afterglow,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn page(self) -> &'static [QuickRow] {
        match self {
            QuickRow::ScreenGba
            | QuickRow::ScreenGbc
            | QuickRow::ScreenGb
            | QuickRow::ScreenNes
            | QuickRow::ScreenSnes => &QuickRow::SCREEN,
            QuickRow::NesCable
            | QuickRow::NesColors
            | QuickRow::NesScanlines
            | QuickRow::NesMask
            | QuickRow::NesAfterglow => &QuickRow::SCREEN_NES,
            QuickRow::SnesCable
            | QuickRow::SnesScanlines
            | QuickRow::SnesMask
            | QuickRow::SnesAfterglow => &QuickRow::SCREEN_SNES,
            QuickRow::GbaShader
            | QuickRow::GbaTransparency
            | QuickRow::GbaGhosting
            | QuickRow::GbaBezel => &QuickRow::SCREEN_GBA,
            QuickRow::GbcShader
            | QuickRow::GbcTransparency
            | QuickRow::GbcGhosting
            | QuickRow::GbcBlackBorder
            | QuickRow::GbcBezel => &QuickRow::SCREEN_GBC,
            QuickRow::GbShader
            | QuickRow::GbTransparency
            | QuickRow::GbGhosting
            | QuickRow::GbPalettes
            | QuickRow::GbBlackBorder
            | QuickRow::GbBezel => &QuickRow::SCREEN_GB,
            QuickRow::FastForward
            | QuickRow::FastForwardSound
            | QuickRow::Rewind
            | QuickRow::Turbo
            | QuickRow::Rumble
            | QuickRow::CheatIcon
            | QuickRow::InGameMenu
            | QuickRow::EjectSave => &QuickRow::GAME,
            QuickRow::Wallpaper
            | QuickRow::WallpaperTint
            | QuickRow::DateTime
            | QuickRow::AutoTime
            | QuickRow::TimeZone
            | QuickRow::Clock24
            | QuickRow::Theme
            | QuickRow::Recents
            | QuickRow::FavoriteMark
            | QuickRow::ShowTitles
            | QuickRow::AutoSleep
            | QuickRow::DeepSleep
            | QuickRow::RememberPosition
            | QuickRow::ResumeGame
            | QuickRow::BootPicture => &QuickRow::GENERAL,
            QuickRow::HomeWifi
            | QuickRow::ConnectedTo
            | QuickRow::AddNetwork
            | QuickRow::SavedNetworks
            | QuickRow::Wps
            | QuickRow::RomManager => &QuickRow::NETWORK,
            QuickRow::Resume
            | QuickRow::States
            | QuickRow::Cheats
            | QuickRow::GameScreenshots
            | QuickRow::Settings
            | QuickRow::ExitGame => &QuickRow::PLAY,
            QuickRow::PlayScreen | QuickRow::PlayGameplay => &QuickRow::SETTINGS,
            QuickRow::Sleep | QuickRow::Restart | QuickRow::PowerOff => &QuickRow::POWER,
            QuickRow::Shader
            | QuickRow::PixelTransparency
            | QuickRow::LcdGhosting
            | QuickRow::Palette
            | QuickRow::BlackBorder
            | QuickRow::Bezel
            | QuickRow::Cable
            | QuickRow::Colors
            | QuickRow::Scanlines
            | QuickRow::Mask
            | QuickRow::Afterglow
            | QuickRow::SgbBorder => &QuickRow::PLAY_SCREEN,
            _ => &QuickRow::MAIN,
        }
    }

    pub fn parent(self) -> Option<QuickRow> {
        match self.page()[0] {
            QuickRow::ScreenGb => Some(QuickRow::Screen),
            QuickRow::GbaShader => Some(QuickRow::ScreenGba),
            QuickRow::GbcShader => Some(QuickRow::ScreenGbc),
            QuickRow::GbShader => Some(QuickRow::ScreenGb),
            QuickRow::NesCable => Some(QuickRow::ScreenNes),
            QuickRow::SnesCable => Some(QuickRow::ScreenSnes),
            QuickRow::FastForward => Some(QuickRow::Game),
            QuickRow::Wallpaper => Some(QuickRow::General),
            QuickRow::HomeWifi => Some(QuickRow::Network),
            QuickRow::AutoSleep => Some(QuickRow::General),
            QuickRow::RememberPosition => Some(QuickRow::General),
            QuickRow::PlayScreen => Some(QuickRow::Settings),
            QuickRow::Shader => Some(QuickRow::PlayScreen),
            _ => None,
        }
    }

    pub fn parent_in_game(self) -> Option<QuickRow> {
        match self.parent() {
            Some(QuickRow::Game) => Some(QuickRow::PlayGameplay),
            parent => parent,
        }
    }

    pub fn centred(self) -> bool {
        [
            &QuickRow::MAIN[..],
            &QuickRow::PLAY,
            &QuickRow::SETTINGS,
            &QuickRow::POWER,
        ]
        .contains(&self.page())
    }

    pub fn child(self) -> Option<QuickRow> {
        match self {
            QuickRow::Screen => Some(QuickRow::SCREEN[0]),
            QuickRow::Game => Some(QuickRow::GAME[0]),
            QuickRow::General => Some(QuickRow::GENERAL[0]),
            QuickRow::Network => Some(QuickRow::NETWORK[0]),
            QuickRow::Settings => Some(QuickRow::SETTINGS[0]),
            QuickRow::PlayScreen => Some(QuickRow::PLAY_SCREEN[0]),
            QuickRow::PlayGameplay => Some(QuickRow::GAME[0]),
            QuickRow::ScreenGba => Some(QuickRow::SCREEN_GBA[0]),
            QuickRow::ScreenGbc => Some(QuickRow::SCREEN_GBC[0]),
            QuickRow::ScreenGb => Some(QuickRow::SCREEN_GB[0]),
            QuickRow::ScreenNes => Some(QuickRow::SCREEN_NES[0]),
            QuickRow::ScreenSnes => Some(QuickRow::SCREEN_SNES[0]),
            _ => None,
        }
    }

    pub fn shown(self, hidden: &[QuickRow]) -> Vec<QuickRow> {
        self.page()
            .iter()
            .copied()
            .filter(|r| !hidden.contains(r))
            .collect()
    }

    pub fn position(self, hidden: &[QuickRow]) -> usize {
        self.shown(hidden)
            .iter()
            .position(|r| *r == self)
            .unwrap_or(0)
    }

    pub fn label(self) -> &'static str {
        match self {
            QuickRow::FastForward => "Fast Forward",
            QuickRow::FastForwardSound => "Fast Forward Sound",
            QuickRow::Screen => "Screen",
            QuickRow::Rumble => "Rumble",
            QuickRow::CheatIcon => "Cheat Icon",
            QuickRow::GbcBlackBorder | QuickRow::GbBlackBorder | QuickRow::BlackBorder => {
                "Black Border"
            }
            QuickRow::DateTime => "Date & Time",
            QuickRow::AutoTime => "Auto Time",
            QuickRow::TimeZone => "Time Zone",
            QuickRow::Clock24 => "24-Hour Time",
            QuickRow::About => "About",
            QuickRow::Screenshots | QuickRow::GameScreenshots => "Screenshots",
            QuickRow::GbaShader | QuickRow::GbcShader | QuickRow::Shader => "LCD Filter",
            // The Game Boy's own screen; with its game's GBC colors it takes the GBC's filter.
            QuickRow::GbShader => "Dot Matrix",
            QuickRow::Game => "Gameplay",
            QuickRow::EjectSave => "Auto Save on Eject",
            QuickRow::Turbo => "Turbo Buttons",
            QuickRow::Rewind => "Rewind",
            QuickRow::GbPalettes => "Palette",
            QuickRow::Theme => "Slot Color",
            QuickRow::Wallpaper => "Theme",
            QuickRow::General => "General",
            QuickRow::Recents => "Recents",
            QuickRow::FavoriteMark => "Favorites",
            QuickRow::ShowTitles => "Game Titles",
            QuickRow::WallpaperTint => "Theme Tint",
            QuickRow::AutoSleep => "Light Sleep",
            QuickRow::DeepSleep => "Deep Sleep",
            QuickRow::PixelTransparency
            | QuickRow::GbTransparency
            | QuickRow::GbcTransparency
            | QuickRow::GbaTransparency => "Pixel Transparency",
            QuickRow::LcdGhosting
            | QuickRow::GbGhosting
            | QuickRow::GbcGhosting
            | QuickRow::GbaGhosting => "Ghosting",
            QuickRow::InGameMenu => "In-Game Menu",
            QuickRow::RememberPosition => "Remember Shelf Position",
            QuickRow::ResumeGame => "Resume Game",
            QuickRow::BootPicture => "Game as Boot Screen",
            QuickRow::Resume => "Resume",
            QuickRow::States => "States",
            QuickRow::Cheats => "Cheats",
            QuickRow::Settings => "Settings",
            QuickRow::ExitGame => "Exit Game",
            QuickRow::PlayScreen => "Screen",
            QuickRow::PlayGameplay => "Gameplay",
            QuickRow::Palette => "Palette",
            QuickRow::ScreenGba => "Game Boy Advance",
            QuickRow::ScreenGbc => "Game Boy Color",
            QuickRow::ScreenGb => "Game Boy",
            QuickRow::GbBezel | QuickRow::GbcBezel | QuickRow::GbaBezel | QuickRow::Bezel => {
                "Bezel"
            }
            QuickRow::Buttons => "Buttons",
            QuickRow::Sleep => "Sleep",
            QuickRow::Restart => "Restart",
            QuickRow::PowerOff => "Power Off",
            QuickRow::HomeWifi => "Home Wi-Fi",
            QuickRow::Network => "Network",
            QuickRow::ConnectedTo => "Connected To",
            QuickRow::AddNetwork => "Add Network",
            QuickRow::SavedNetworks => "My Networks",
            QuickRow::Wps => "WPS",
            QuickRow::RomManager => "GRAVY Manager",
            QuickRow::ScreenNes => "NES",
            QuickRow::ScreenSnes => "Super Nintendo",
            QuickRow::NesCable | QuickRow::SnesCable | QuickRow::Cable => "Cable",
            QuickRow::NesColors | QuickRow::Colors => "Colors",
            QuickRow::NesScanlines | QuickRow::SnesScanlines | QuickRow::Scanlines => "Scanlines",
            QuickRow::NesMask | QuickRow::SnesMask | QuickRow::Mask => "Mask",
            QuickRow::NesAfterglow | QuickRow::SnesAfterglow | QuickRow::Afterglow => "Afterglow",
            QuickRow::SgbBorder => "Border",
        }
    }

    /// One plain line under the menu saying what the selected row does.
    pub fn help(self) -> Option<&'static str> {
        Some(match self {
            QuickRow::General => "Theme, slot color, shelves and sleep.",
            QuickRow::Screen => "How the game picture looks.",
            QuickRow::Game => "Speed, rewind, turbo and saving.",
            QuickRow::Buttons => "Every button and shortcut, explained.",
            QuickRow::About => "Who made this, and what it runs on.",
            QuickRow::Screenshots => "Every screenshot, newest first. VOL+ and POWER takes one.",
            QuickRow::GameScreenshots => "This game's screenshots. VOL+ and POWER takes one.",
            QuickRow::NesCable | QuickRow::SnesCable | QuickRow::Cable => {
                "How the console reached the TV. SELECT + B turns every effect off."
            }
            QuickRow::NesColors | QuickRow::Colors => "The colors a TV made of the NES's picture.",
            QuickRow::SgbBorder => {
                "The Super Game Boy's frame. Auto: the game's own, else the Classic one."
            }
            QuickRow::NesScanlines | QuickRow::SnesScanlines | QuickRow::Scanlines => {
                "A faint dark line between the picture's lines."
            }
            QuickRow::NesMask | QuickRow::SnesMask | QuickRow::Mask => {
                "The tube's red, green and blue, faintly over the picture."
            }
            QuickRow::NesAfterglow | QuickRow::SnesAfterglow | QuickRow::Afterglow => {
                "Moving things leave a brief glow, as on a tube."
            }
            QuickRow::ScreenGb
            | QuickRow::ScreenGbc
            | QuickRow::ScreenGba
            | QuickRow::ScreenNes
            | QuickRow::ScreenSnes => "Settings for this console only.",
            QuickRow::GbShader => "The Game Boy's dot grid. SELECT + B turns every effect off.",
            QuickRow::GbaShader | QuickRow::GbcShader | QuickRow::Shader => {
                "The real screen's lines. SELECT + B turns every effect off."
            }
            QuickRow::GbPalettes | QuickRow::Palette => {
                "Off: the game's own Game Boy Color colors. SELECT + L2 / R2 picks one."
            }
            QuickRow::PixelTransparency
            | QuickRow::GbTransparency
            | QuickRow::GbcTransparency
            | QuickRow::GbaTransparency => {
                "Pale pixels let the light through and cast soft shadows."
            }
            QuickRow::LcdGhosting
            | QuickRow::GbGhosting
            | QuickRow::GbcGhosting
            | QuickRow::GbaGhosting => "Moving pixels fade out slowly, like an old LCD.",
            QuickRow::FastForward => "How fast the game runs while you hold R2.",
            QuickRow::FastForwardSound => "Hear the game while it fast-forwards.",
            QuickRow::Rewind => "Hold L2 in a game to go back in time.",
            QuickRow::Turbo => "Hold X for rapid A, hold Y for rapid B.",
            QuickRow::Rumble => "Let games that support it shake the device.",
            QuickRow::CheatIcon => "Show a masked robber while any cheat is on.",
            QuickRow::GbcBlackBorder | QuickRow::GbBlackBorder | QuickRow::BlackBorder => {
                "Black around the picture, instead of the screen's own glass."
            }
            QuickRow::InGameMenu => "Tap MENU in a game to pause and open a menu.",
            QuickRow::EjectSave => "Save your place when you eject a cart.",
            QuickRow::Wallpaper => "Dark, Light, or your own from the Wallpapers folder.",
            QuickRow::Theme => "Dark, light or each console's own colour round the slot.",
            QuickRow::Recents => "A shelf with the last 10 games you played.",
            QuickRow::FavoriteMark => "How favorites stand out. Press Y on a cart to mark it.",
            QuickRow::ShowTitles => "Show the selected game's name under the shelf.",
            QuickRow::DateTime => "Set the date and time yourself.",
            QuickRow::AutoTime => "Set by the internet over Home Wi-Fi.",
            QuickRow::TimeZone => "Your city, for summer and winter time. A shows them all.",
            QuickRow::Clock24 => "Off shows the time as 6:00 PM.",
            QuickRow::HomeWifi => "Join your saved networks. Sets the clock too.",
            QuickRow::AddNetwork => "Find a network nearby and type its password.",
            QuickRow::SavedNetworks => "The device joins these by itself. Y forgets one.",
            QuickRow::Wps => "Join by pressing the WPS button on your router.",
            QuickRow::RomManager => "Games, screenshots and more from a web browser.",
            QuickRow::Network => "Home Wi-Fi and the network you are on.",
            QuickRow::ConnectedTo => "The home network this device is on now.",
            QuickRow::AutoSleep => {
                "Screen off after this long without a button press. Wi-Fi stays on."
            }
            QuickRow::DeepSleep => {
                "From Light Sleep to Deep Sleep after this long. Saves the most battery."
            }
            QuickRow::RememberPosition => "Start where you left off on the shelf.",
            QuickRow::ResumeGame => {
                "Power on straight back into the game, its picture as boot screen."
            }
            QuickRow::BootPicture => "Power off in a game and it shows while the device starts.",
            QuickRow::Resume => "Back to the game, right where you paused.",
            QuickRow::States => "Save, load and flick through save states.",
            QuickRow::Cheats => "Turn this game's cheats on and off.",
            QuickRow::Settings => "Screen and gameplay settings, while you play.",
            QuickRow::ExitGame => "Save and put the cart back on the shelf.",
            QuickRow::PlayScreen => "How the game picture looks.",
            QuickRow::PlayGameplay => "Speed, rewind, turbo and saving.",
            QuickRow::Sleep => "Screen off, game paused. Press POWER to wake.",
            QuickRow::Restart => "Save your place and start again.",
            QuickRow::PowerOff => "Save your place and switch off.",
            _ => return None,
        })
    }

    /// A row that shows a value and takes no input.
    pub fn read_only(self) -> bool {
        self == QuickRow::ConnectedTo
    }

    pub fn opens(self) -> bool {
        matches!(
            self,
            QuickRow::DateTime
                | QuickRow::AddNetwork
                | QuickRow::SavedNetworks
                | QuickRow::Wps
                | QuickRow::RomManager
                | QuickRow::About
                | QuickRow::Screenshots
                | QuickRow::Screen
                | QuickRow::Game
                | QuickRow::General
                | QuickRow::Network
                | QuickRow::Resume
                | QuickRow::States
                | QuickRow::Cheats
                | QuickRow::GameScreenshots
                | QuickRow::Settings
                | QuickRow::ExitGame
                | QuickRow::PlayScreen
                | QuickRow::PlayGameplay
                | QuickRow::ScreenGba
                | QuickRow::ScreenGbc
                | QuickRow::ScreenGb
                | QuickRow::ScreenNes
                | QuickRow::ScreenSnes
                | QuickRow::Buttons
                | QuickRow::Sleep
                | QuickRow::Restart
                | QuickRow::PowerOff
        )
    }

    pub fn up(self, hidden: &[QuickRow]) -> QuickRow {
        let rows = self.shown(hidden);
        rows[(self.position(hidden) + rows.len() - 1) % rows.len()]
    }

    pub fn down(self, hidden: &[QuickRow]) -> QuickRow {
        let rows = self.shown(hidden);
        rows[(self.position(hidden) + 1) % rows.len()]
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum QuickValue {
    Speed2,
    Speed3,
    Speed4,
    Speed6,
    On,
    Off,
    Lcd3x,
    GbaLcd,
    Grid,
    Dot,
    Simpletex,
    Frames1,
    Dark,
    Light,
    Originals,
    Show,
    Hide,
    Low,
    Medium,
    High,
    Sleep30s,
    Sleep1m,
    Sleep5m,
    SleepNow,
    Sleep10m,
    Sleep30m,
    GoldGlow,
    WhiteGlow,
    StarMark,
    Authentic,
    Clean,
    Custom,
    Rf,
    Composite,
    SVideo,
    Rgb,
    Standard,
    Sony,
    BorderAuto,
    BorderGravy,
    /// No border: a word on the row, not a switch, since the row goes round its borders.
    BorderOff,
    Border1,
    Border2,
    Border3,
    Border4,
    Border5,
    Border6,
    Border7,
    Border8,
    /// The Super Game Boy 2's own borders after its Classic (Border1).
    Border2Circuit,
    Border2Beach,
    Border2Stone,
    Border2Gears,
    Border2Jungle,
    Border2Dolphins,
    Border2Temple,
}

impl QuickValue {
    pub const ALL: [QuickValue; 56] = [
        QuickValue::Speed2,
        QuickValue::Speed3,
        QuickValue::Speed4,
        QuickValue::Speed6,
        QuickValue::On,
        QuickValue::Off,
        QuickValue::Lcd3x,
        QuickValue::GbaLcd,
        QuickValue::Grid,
        QuickValue::Dot,
        QuickValue::Simpletex,
        QuickValue::Frames1,
        QuickValue::Dark,
        QuickValue::Light,
        QuickValue::Originals,
        QuickValue::Show,
        QuickValue::Hide,
        QuickValue::Low,
        QuickValue::Medium,
        QuickValue::High,
        QuickValue::Sleep30s,
        QuickValue::Sleep1m,
        QuickValue::Sleep5m,
        QuickValue::SleepNow,
        QuickValue::Sleep10m,
        QuickValue::Sleep30m,
        QuickValue::GoldGlow,
        QuickValue::WhiteGlow,
        QuickValue::StarMark,
        QuickValue::Authentic,
        QuickValue::Clean,
        QuickValue::Custom,
        QuickValue::Rf,
        QuickValue::Composite,
        QuickValue::SVideo,
        QuickValue::Rgb,
        QuickValue::Standard,
        QuickValue::Sony,
        QuickValue::BorderAuto,
        QuickValue::BorderGravy,
        QuickValue::BorderOff,
        QuickValue::Border1,
        QuickValue::Border2,
        QuickValue::Border3,
        QuickValue::Border4,
        QuickValue::Border5,
        QuickValue::Border6,
        QuickValue::Border7,
        QuickValue::Border8,
        QuickValue::Border2Circuit,
        QuickValue::Border2Beach,
        QuickValue::Border2Stone,
        QuickValue::Border2Gears,
        QuickValue::Border2Jungle,
        QuickValue::Border2Dolphins,
        QuickValue::Border2Temple,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn text(self) -> &'static str {
        match self {
            QuickValue::Speed2 => "2×",
            QuickValue::Speed3 => "3×",
            QuickValue::Speed4 => "4×",
            QuickValue::Speed6 => "6×",
            QuickValue::On => "On",
            QuickValue::Off => "Off",
            QuickValue::Lcd3x => "LCD3x",
            QuickValue::GbaLcd => "GBA LCD",
            QuickValue::Grid => "Grid",
            QuickValue::Dot => "Dot",
            QuickValue::Simpletex => "Simpletex",
            QuickValue::Frames1 => "1 Frame",
            QuickValue::Dark => "Dark",
            QuickValue::Light => "Light",
            QuickValue::Originals => "Originals",
            QuickValue::Show => "On",
            QuickValue::Hide => "Off",
            QuickValue::Low => "25%",
            QuickValue::Medium => "45%",
            QuickValue::High => "65%",
            QuickValue::Sleep30s => "30 sec",
            QuickValue::Sleep1m => "1 min",
            QuickValue::Sleep5m => "5 min",
            QuickValue::SleepNow => "Direct",
            QuickValue::Sleep10m => "10 min",
            QuickValue::Sleep30m => "30 min",
            QuickValue::GoldGlow => "Spotlight",
            QuickValue::WhiteGlow => "Highlight",
            QuickValue::StarMark => "Star",
            QuickValue::Authentic => "Authentic",
            QuickValue::Clean => "Clean",
            QuickValue::Custom => "Custom",
            QuickValue::Rf => "RF",
            QuickValue::Composite => "Composite",
            QuickValue::SVideo => "S-Video",
            QuickValue::Rgb => "RGB",
            QuickValue::Standard => "Standard",
            // A Trinitron's own colour decoder (Roy chose the name).
            QuickValue::Sony => "Trinitron",
            QuickValue::BorderAuto => "Auto",
            QuickValue::BorderGravy => "GRAVY",
            QuickValue::BorderOff => "Off",
            QuickValue::Border1 => "Classic",
            QuickValue::Border2 => "Windows",
            QuickValue::Border3 => "Cork",
            QuickValue::Border4 => "Forest",
            QuickValue::Border5 => "Theatre",
            QuickValue::Border6 => "Cats",
            QuickValue::Border7 => "Desk",
            QuickValue::Border8 => "Shapes",
            QuickValue::Border2Circuit => "Circuit",
            QuickValue::Border2Beach => "Beach",
            QuickValue::Border2Stone => "Stone",
            QuickValue::Border2Gears => "Gears",
            QuickValue::Border2Jungle => "Jungle",
            QuickValue::Border2Dolphins => "Dolphins",
            QuickValue::Border2Temple => "Temple",
        }
    }

    pub fn speed(frames: u8) -> Option<QuickValue> {
        match frames {
            2 => Some(QuickValue::Speed2),
            3 => Some(QuickValue::Speed3),
            4 => Some(QuickValue::Speed4),
            6 => Some(QuickValue::Speed6),
            _ => None,
        }
    }

    pub fn frames(frames: u8) -> Option<QuickValue> {
        match frames {
            0 => Some(QuickValue::Off),
            1 => Some(QuickValue::Frames1),
            _ => None,
        }
    }

    /// Values that read as a switch rather than a word.
    pub fn switch(self) -> Option<bool> {
        match self {
            QuickValue::On | QuickValue::Show => Some(true),
            QuickValue::Off | QuickValue::Hide => Some(false),
            _ => None,
        }
    }

    pub fn flag(on: bool) -> QuickValue {
        if on {
            QuickValue::On
        } else {
            QuickValue::Off
        }
    }
}

pub const QUICK_PITCH: f32 = 52.0;
pub const QUICK_EDGE: f32 = 32.0;
pub const QUICK_LABEL_X: f32 = QUICK_EDGE;

/// The 1.6 menu look: a smaller face than the power menu, so a label and its value never run into each other.
const QUICK_PX: f32 = 26.0;
const POINTER_SCALE: usize = 4;
const POINTER: [&str; 11] = [
    "X.......", "XX......", "XoX.....", "XooX....", "XoooX...", "XooooX..", "XoooX...", "XooX....",
    "XoX.....", "XX......", "X.......",
];
const POINTER_BLEED: u32 = 4;
const TYPE_DROP: f32 = 4.0;
const BAR_INSET: f32 = 4.0;
const CARET_GAP: f32 = 14.0;
const CARET_PX: f32 = 24.0;
pub(crate) const LEGEND_Y: f32 = 427.0;
/// The help line sits between the rows and the legend.
const HELP_Y: f32 = 392.0;
const HELP_H: u32 = 24;
const HELP_PX: f32 = 17.0;
const HELP_MIN_PX: f32 = 12.0;
const HELP_MAX_W: f32 = 600.0;
/// Rows fill the space above the help line.
const PANEL_GAP: f32 = 12.0;
/// Every menu sits centred on this line, between the top and the help line.
pub(crate) const MENU_CENTRE: f32 = 202.0;
/// A sub-page shows this many rows at once and scrolls the rest.
pub const SUB_ROWS: usize = 6;
/// Room between a scroll arrow and the rows it points past.
pub(crate) const MORE_GAP: f32 = 8.0;

/// The first row of a scrolling list shown so that `sel` is on screen, keeping the list still
/// while it can.
pub fn window_first(sel: usize, count: usize, first: usize, rows: usize) -> usize {
    let rows = rows.min(count);
    let mut first = first.min(count.saturating_sub(rows));
    if sel < first {
        first = sel;
    }
    if sel >= first + rows {
        first = sel + 1 - rows;
    }
    first
}

/// Which row of `row`'s page shows first, given the one shown first last time.
pub fn quick_first(row: QuickRow, hidden: &[QuickRow], prev: usize) -> usize {
    if row.centred() || row.page() == QuickRow::PLAY_SCREEN {
        return 0;
    }
    let n = row.shown(hidden).len();
    window_first(row.position(hidden), n, prev, SUB_ROWS)
}

/// The top of the first row shown, and how many rows show, for `row`'s page.
fn page_layout(row: QuickRow, hidden: &[QuickRow]) -> (f32, usize) {
    let n = row.shown(hidden).len();
    let page = row.page();
    if page == QuickRow::PLAY_SCREEN {
        return (HELP_Y - PANEL_GAP - QUICK_PITCH * n as f32, n);
    }
    let shown = if row.centred() { n } else { n.min(SUB_ROWS) };
    (MENU_CENTRE - QUICK_PITCH * shown as f32 / 2.0, shown)
}
/// Faces are rendered white and tinted with the theme's ink when drawn.
const FACE_INK: [u8; 3] = [0xff, 0xff, 0xff];

/// The on/off switch: a flat pill in one colour, as on a phone.
pub const SWITCH_W: u32 = 52;
pub const SWITCH_H: u32 = 30;
const SWITCH_STROKE: f32 = 2.5;

pub fn quick_label_face(row: QuickRow, _lit: bool) -> UndoFace {
    quick_text_face(row.label(), FACE_INK)
}

fn cap_centre(label: &fontdue::Font, h: u32) -> f32 {
    match label.horizontal_line_metrics(QUICK_PX) {
        Some(v) => {
            (h as f32 - v.new_line_size) / 2.0 + v.ascent
                - label.metrics('H', QUICK_PX).height as f32 / 2.0
        }
        None => h as f32 / 2.0,
    }
}

fn row_cap_centre() -> f32 {
    match text::label_font() {
        Some(label) => cap_centre(label, MENU_H),
        None => MENU_H as f32 / 2.0,
    }
}

pub fn quick_pointer_face() -> UndoFace {
    let s = POINTER_SCALE;
    let w = POINTER[0].len() * s;
    let h = (MENU_H + 2 * POINTER_BLEED) as usize;
    let centre = row_cap_centre() + POINTER_BLEED as f32;
    let tip = POINTER.len() / 2 * s + s / 2;
    let top = (centre - tip as f32).round() as usize;
    let mut rgba = vec![0u8; w * h * 4];
    for (y, row) in POINTER.iter().enumerate() {
        for (x, _) in row.chars().enumerate().filter(|(_, ch)| *ch != '.') {
            for py in top + y * s..top + (y + 1) * s {
                for px in x * s..(x + 1) * s {
                    let at = (py * w + px) * 4;
                    rgba[at..at + 4].copy_from_slice(&[
                        MENU_INK[0],
                        MENU_INK[1],
                        MENU_INK[2],
                        0xff,
                    ]);
                }
            }
        }
    }
    UndoFace {
        rgba,
        w: w as u32,
        h: h as u32,
    }
}

pub fn quick_value_face(text: &str, _lit: bool) -> UndoFace {
    quick_text_face(text, FACE_INK)
}

fn quick_text_face(label: &str, colour: [u8; 3]) -> UndoFace {
    let Some(font) = text::label_font() else {
        return UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let layout = text::fit(font, label, OUT_W as f32, 1, QUICK_PX, QUICK_PX);
    let set = layout
        .lines
        .iter()
        .map(|l| text::line_width(font, l, layout.px, layout.tracking))
        .fold(0.0, f32::max);
    let w = set.ceil() as u32 + 2 * MENU_PAD;
    let mut rgba = vec![0u8; (w * MENU_H * 4) as usize];
    text::draw_centred(&mut rgba, w, MENU_H, &layout, colour);
    UndoFace { rgba, w, h: MENU_H }
}

/// The help line for a row, in white for tinting. Rows with nothing to say have none.
pub fn quick_help_face(row: QuickRow) -> Option<UndoFace> {
    let line = row.help()?;
    let font = text::label_font()?;
    let layout = text::fit(font, line, HELP_MAX_W, 1, HELP_PX, HELP_MIN_PX);
    let set = layout
        .lines
        .iter()
        .map(|l| text::line_width(font, l, layout.px, layout.tracking))
        .fold(0.0, f32::max);
    let w = (set.ceil() as u32 + 4).min(HELP_MAX_W as u32);
    let mut rgba = vec![0u8; (w * HELP_H * 4) as usize];
    text::draw_centred(&mut rgba, w, HELP_H, &layout, FACE_INK);
    Some(UndoFace { rgba, w, h: HELP_H })
}

/// The switch in white: off is an outlined pill with the knob on the left, on is a filled pill
/// with the knob cut out on the right.
pub fn quick_switch_face(on: bool) -> UndoFace {
    const SS: usize = 4;
    let (w, h) = (SWITCH_W as usize, SWITCH_H as usize);
    let r = h as f32 / 2.0;
    let pill = |x: f32, y: f32| -> f32 {
        // Signed distance to the pill's outline, negative inside.
        let cx = x.clamp(r, w as f32 - r);
        ((x - cx).powi(2) + (y - r).powi(2)).sqrt() - r
    };
    let knob_r = if on { r - 4.0 } else { r - 5.5 };
    let knob_x = if on { w as f32 - r } else { r };
    let mut rgba = vec![0u8; w * h * 4];
    for py in 0..h {
        for px in 0..w {
            let mut cover = 0.0f32;
            for sy in 0..SS {
                for sx in 0..SS {
                    let x = px as f32 + (sx as f32 + 0.5) / SS as f32;
                    let y = py as f32 + (sy as f32 + 0.5) / SS as f32;
                    let d = pill(x, y);
                    let knob = ((x - knob_x).powi(2) + (y - r).powi(2)).sqrt() - knob_r;
                    let ink = if on {
                        d <= 0.0 && knob > 0.0
                    } else {
                        (d <= 0.0 && d > -SWITCH_STROKE) || knob <= 0.0
                    };
                    if ink {
                        cover += 1.0;
                    }
                }
            }
            let a = (cover / (SS * SS) as f32 * 255.0).round() as u8;
            let at = (py * w + px) * 4;
            rgba[at..at + 4].copy_from_slice(&[0xff, 0xff, 0xff, a]);
        }
    }
    UndoFace {
        rgba,
        w: SWITCH_W,
        h: SWITCH_H,
    }
}

pub fn quick_caret_face(right: bool) -> UndoFace {
    let glyph = if right { '\u{f0da}' } else { '\u{f0d9}' };
    let (Some(symbols), Some(label)) = (crate::icon::symbols_font(), text::label_font()) else {
        return UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let (m, cov) = symbols.rasterize(glyph, CARET_PX);
    let (w, h) = (m.width as u32, MENU_H);
    let centre = cap_centre(label, h);
    let top = (centre - m.height as f32 / 2.0).round() as i32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for gy in 0..m.height {
        let y = top + gy as i32;
        if y < 0 || y >= h as i32 {
            continue;
        }
        for gx in 0..m.width {
            let at = ((y as u32 * w + gx as u32) * 4) as usize;
            let a = cov[gy * m.width + gx];
            rgba[at..at + 4].copy_from_slice(&[FACE_INK[0], FACE_INK[1], FACE_INK[2], a]);
        }
    }
    UndoFace { rgba, w, h }
}

pub fn quick_legend_faces() -> [UndoFace; 5] {
    [
        tint_mask(hint_face("B", "Back")),
        tint_mask(arrows_hint_face("Change")),
        tint_mask(hint_face("A", "Open")),
        tint_mask(hint_face("B", "Resume")),
        tint_mask(hint_face("A", "Select")),
    ]
}

pub struct QuickMenuFaces {
    pub labels: Vec<[(TexId, u32, u32); 2]>,
    pub pointer: (TexId, u32, u32),
    pub values: Vec<[(TexId, u32, u32); 2]>,
    pub carets: [(TexId, u32, u32); 2],
    pub legend: [(TexId, u32); 5],
    /// Off, on.
    pub switches: [(TexId, u32, u32); 2],
    pub help: Vec<Option<(TexId, u32, u32)>>,
    /// Pointing up and down, where a page carries on.
    pub more: [(TexId, u32, u32); 2],
}

pub struct QuickMenu<'a> {
    pub row: QuickRow,
    pub hidden: &'a [QuickRow],
    pub values: [Option<QuickValue>; QuickRow::ALL.len()],
    pub clock: Option<[(TexId, u32, u32); 2]>,
    /// The connected network's name, or that there is none.
    pub network: Option<[(TexId, u32, u32); 2]>,
    /// The clock's city, on Time Zone.
    pub zone: Option<[(TexId, u32, u32); 2]>,
    /// A row shown faded that takes no input, like Date & Time while Auto Time is on.
    pub locked: Option<QuickRow>,
    pub wallpaper: Option<[(TexId, u32, u32); 2]>,
    pub faces: Option<&'a QuickMenuFaces>,
    pub ground: Option<TexId>,
    pub palette: Option<[(TexId, u32, u32); 2]>,
    pub gba_bezel: Option<[(TexId, u32, u32); 2]>,
    pub gb_bezel: Option<[(TexId, u32, u32); 2]>,
    pub gbc_bezel: Option<[(TexId, u32, u32); 2]>,
    pub bezel: Option<[(TexId, u32, u32); 2]>,
    pub over_game: bool,
    pub canvas: Canvas,
    pub ink: Ink,
    /// The first row shown on a page that scrolls (see `quick_first`).
    pub first: usize,
    /// Rows shown under another row's name: in game, LCD Filter reads Dot Matrix on a GBA.
    pub named_as: &'a [(QuickRow, QuickRow)],
}

impl QuickMenu<'_> {
    pub fn draw(&self, out: &mut Vec<Draw>) {
        let ink = if self.over_game {
            theme::OVER_GAME
        } else {
            self.ink
        };
        match (self.over_game, self.row.page() == QuickRow::PLAY_SCREEN) {
            (false, _) => draw_menu_ground(self.canvas, self.ground, ink, out),
            (true, panel) => {
                let y = match panel {
                    true => quick_row_top(self.row.page()[0], self.hidden) - PANEL_GAP,
                    false => 0.0,
                };
                out.push(Draw::Rect {
                    x: 0.0,
                    y,
                    w: self.canvas.w,
                    h: OUT_H as f32 - y,
                    colour: ink.wash,
                });
            }
        }
        let shown_rows = self.row.shown(self.hidden);
        let pitch = QUICK_PITCH;
        let (top, visible) = page_layout(self.row, self.hidden);
        let first = quick_first(self.row, self.hidden, self.first);
        let sel = self.row.position(self.hidden);
        out.push(Draw::Rect {
            x: 0.0,
            y: top + pitch * (sel - first) as f32 + BAR_INSET,
            w: self.canvas.w,
            h: pitch - 2.0 * BAR_INSET,
            colour: ink.bar,
        });
        let Some(faces) = self.faces else {
            return;
        };
        let (right, pad) = (self.canvas.w - QUICK_EDGE, MENU_PAD as f32);
        let centre = row_cap_centre();
        // Arrows where the page carries on above or below.
        let [up, down] = faces.more;
        if first > 0 {
            let y = top - MORE_GAP - up.2 as f32;
            tint(
                out,
                up.0,
                self.canvas.centred(up.1 as f32),
                y,
                up.1,
                up.2,
                ink.dim,
            );
        }
        if first + visible < shown_rows.len() {
            let y = top + pitch * visible as f32 + MORE_GAP;
            tint(
                out,
                down.0,
                self.canvas.centred(down.1 as f32),
                y,
                down.1,
                down.2,
                ink.dim,
            );
        }
        for (i, row) in shown_rows.iter().copied().enumerate() {
            if i < first || i >= first + visible {
                continue;
            }
            let y = top + pitch * (i - first) as f32 + TYPE_DROP;
            let lit = row == self.row;
            let colour = match (lit, self.locked == Some(row)) {
                (true, true) => [
                    ink.on_bar[0],
                    ink.on_bar[1],
                    ink.on_bar[2],
                    ink.on_bar[3] * 0.45,
                ],
                (true, false) => ink.on_bar,
                (false, true) => [ink.dim[0], ink.dim[1], ink.dim[2], ink.dim[3] * 0.45],
                (false, false) => ink.dim,
            };
            let named = self
                .named_as
                .iter()
                .find(|(r, _)| *r == row)
                .map_or(row, |&(_, as_row)| as_row);
            if let Some(&(tex, w, h)) = faces.labels.get(named.index()).map(|l| &l[lit as usize]) {
                let x = match row.centred() {
                    true => self.canvas.centred(w as f32),
                    false => QUICK_LABEL_X - pad,
                };
                tint(out, tex, x, y, w, h, colour);
            }
            if row.centred() {
                continue;
            }
            let picked = self.values[row.index()];
            if let Some(on) = picked.and_then(QuickValue::switch) {
                let (tex, w, h) = faces.switches[on as usize];
                let sy = y + centre - h as f32 / 2.0;
                tint(out, tex, right - w as f32, sy, w, h, colour);
                continue;
            }
            let value = match row {
                QuickRow::DateTime => self.clock.map(|c| c[lit as usize]),
                QuickRow::ConnectedTo => self.network.map(|c| c[lit as usize]),
                QuickRow::TimeZone => self.zone.map(|c| c[lit as usize]),
                QuickRow::Wallpaper => self.wallpaper.map(|c| c[lit as usize]),
                QuickRow::Palette | QuickRow::GbPalettes => self.palette.map(|c| c[lit as usize]),
                QuickRow::GbaBezel => self.gba_bezel.map(|c| c[lit as usize]),
                QuickRow::GbBezel => self.gb_bezel.map(|c| c[lit as usize]),
                QuickRow::GbcBezel => self.gbc_bezel.map(|c| c[lit as usize]),
                QuickRow::Bezel => self.bezel.map(|c| c[lit as usize]),
                _ => picked
                    .and_then(|v| faces.values.get(v.index()))
                    .map(|v| v[lit as usize]),
            };
            let [(left_tex, lw, lh), (right_tex, rw, rh)] = faces.carets;
            let Some((tex, w, h)) = value else {
                // A row that opens a page of its own: a caret says there is more inside.
                if row.opens() {
                    tint(out, right_tex, right - rw as f32, y, rw, rh, colour);
                }
                continue;
            };
            if !lit || row.opens() || row.read_only() || self.locked == Some(row) {
                tint(out, tex, right + pad - w as f32, y, w, h, colour);
                continue;
            }
            let rx = right - rw as f32;
            tint(out, right_tex, rx, y, rw, rh, colour);
            let vx = rx - CARET_GAP + pad - w as f32;
            tint(out, tex, vx, y, w, h, colour);
            tint(
                out,
                left_tex,
                vx + pad - CARET_GAP - lw as f32,
                y,
                lw,
                lh,
                colour,
            );
        }
        let help_row = self
            .named_as
            .iter()
            .find(|(r, _)| *r == self.row)
            .map_or(self.row, |&(_, as_row)| as_row);
        if let Some(&Some((tex, w, h))) = faces.help.get(help_row.index()) {
            tint(
                out,
                tex,
                self.canvas.centred(w as f32),
                HELP_Y,
                w,
                h,
                ink.dim,
            );
        }
        let [back, change, open, resume, select] = faces.legend;
        let back = match self.row.page() == QuickRow::PLAY {
            true => resume,
            false => back,
        };
        let other = match (self.row.page() == QuickRow::POWER, self.row.opens()) {
            (true, _) => select,
            (false, true) => open,
            (false, false) => change,
        };
        // A row that only tells, like Connected To, has nothing to change.
        let hints: &[(TexId, u32)] = match self.row.read_only() || self.locked == Some(self.row) {
            true => &[back],
            false => &[back, other],
        };
        for (tex, w, x) in centred_hints(self.canvas, hints, LEGEND_GAP) {
            tint(out, tex, x, LEGEND_Y, w, HINT_H, ink.text);
        }
    }
}

/// The menu's ground: the theme's wallpaper under a light wash, or plain black when Wallpaper
/// is Off.
pub fn draw_menu_ground(canvas: Canvas, ground: Option<TexId>, ink: Ink, out: &mut Vec<Draw>) {
    let (w, h) = (canvas.w, OUT_H as f32);
    match ground {
        Some(tex) => {
            out.push(Draw::Part {
                x: 0.0,
                y: 0.0,
                w,
                h,
                tex,
                alpha: 1.0,
                uv: canvas.cover(OUT_W as f32, OUT_H as f32),
            });
            out.push(Draw::Rect {
                x: 0.0,
                y: 0.0,
                w,
                h,
                colour: ink.wash,
            });
        }
        None => out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
            colour: [0.0, 0.0, 0.0, 1.0],
        }),
    }
}

/// The row height: every menu uses the full pitch; longer pages scroll instead.
pub fn quick_pitch(_rows: usize) -> f32 {
    QUICK_PITCH
}

/// Where `row` sits with its page scrolled to the top.
pub fn quick_row_top(row: QuickRow, hidden: &[QuickRow]) -> f32 {
    let (top, _) = page_layout(row, hidden);
    top + QUICK_PITCH * row.position(hidden) as f32
}

fn tint(out: &mut Vec<Draw>, tex: TexId, x: f32, y: f32, w: u32, h: u32, colour: [f32; 4]) {
    out.push(Draw::Tint {
        x: x.round(),
        y: y.round(),
        w: w as f32,
        h: h as f32,
        tex,
        colour,
    });
}
