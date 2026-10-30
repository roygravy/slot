use std::path::{Path, PathBuf};

use crate::atomic::atomic_write;
use crate::gb_palette::GbPalette;
use crate::platform::Platform;

pub const BRIGHTNESS_MAX: u8 = 9;
pub const BLUE_LIGHT_MAX: u8 = 9;
pub const VOLUME_MAX: u8 = 100;

pub const UTC_OFFSET_MIN: i16 = -720;
pub const UTC_OFFSET_MAX: i16 = 840;

pub const FF_SPEEDS: [u8; 4] = [2, 3, 4, 6];

pub const FF_SPEED_DEFAULT: u8 = 6;

pub const RECENTS_MAX: usize = 10;
pub const AUTO_SLEEP_OPTIONS_SECS: [u16; 4] = [0, 30, 60, 300];
pub const AUTO_SLEEP_DEFAULT_SECS: u16 = 60;
/// From the light sleep into the deep one: 0 straight away.
pub const DEEP_SLEEP_OPTIONS_MINS: [u16; 4] = [0, 5, 10, 30];
pub const DEEP_SLEEP_DEFAULT_MINS: u16 = 5;
const AUTHENTIC_LCD_PROFILE: &str = "lcd_profile=authentic-v4";

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Recent {
    pub platform: Platform,
    pub stem: String,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Appearance {
    #[default]
    Dark,
    Light,
    /// Each console's own housing round its slot: the DMG's grey, the GBC's Grape, the GBA's
    /// Indigo, the NES's and SNES's light greys. Menus stay dark.
    Originals,
}

impl Appearance {
    pub fn light(self) -> bool {
        matches!(self, Appearance::Light)
    }

    pub fn name(self) -> &'static str {
        match self {
            Appearance::Dark => "dark",
            Appearance::Light => "light",
            Appearance::Originals => "originals",
        }
    }

    pub fn parse(name: &str) -> Option<Appearance> {
        match name {
            "dark" => Some(Appearance::Dark),
            // A card set to the DMG theme of a test build reads as light.
            "light" | "dmg" => Some(Appearance::Light),
            "originals" => Some(Appearance::Originals),
            _ => None,
        }
    }

    pub fn step(self, right: bool) -> Appearance {
        const ALL: [Appearance; 3] = [Appearance::Dark, Appearance::Light, Appearance::Originals];
        let at = ALL.iter().position(|a| *a == self).unwrap_or(0);
        ALL[(at + if right { 1 } else { ALL.len() - 1 }) % ALL.len()]
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum FavoriteMark {
    #[default]
    GoldGlow,
    /// A soft light on the ground behind the cart.
    Highlight,
    Star,
}

impl FavoriteMark {
    pub const ALL: [FavoriteMark; 3] = [
        FavoriteMark::GoldGlow,
        FavoriteMark::Highlight,
        FavoriteMark::Star,
    ];

    pub fn key(self) -> &'static str {
        match self {
            FavoriteMark::GoldGlow => "gold_glow",
            FavoriteMark::Highlight => "highlight",
            FavoriteMark::Star => "star",
        }
    }

    pub fn parse(value: &str) -> Option<FavoriteMark> {
        // The white glow is gone; a card set to it gets the spotlight.
        if value == "white_glow" {
            return Some(FavoriteMark::GoldGlow);
        }
        FavoriteMark::ALL.into_iter().find(|m| m.key() == value)
    }

    /// Spotlight, Highlight and Star, round and round.
    pub fn step(self, right: bool) -> FavoriteMark {
        let all = FavoriteMark::ALL;
        let at = all.iter().position(|m| *m == self).unwrap_or(0);
        all[if right { (at + 1) % all.len() } else { (at + all.len() - 1) % all.len() }]
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum WallpaperTint {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

impl WallpaperTint {
    pub const ALL: [WallpaperTint; 4] = [
        WallpaperTint::Off,
        WallpaperTint::Low,
        WallpaperTint::Medium,
        WallpaperTint::High,
    ];

    pub fn key(self) -> &'static str {
        match self {
            WallpaperTint::Off => "off",
            WallpaperTint::Low => "low",
            WallpaperTint::Medium => "medium",
            WallpaperTint::High => "high",
        }
    }

    pub fn parse(value: &str) -> Option<WallpaperTint> {
        WallpaperTint::ALL.into_iter().find(|t| t.key() == value)
    }

    pub fn step(self, right: bool) -> WallpaperTint {
        let n = WallpaperTint::ALL.len();
        let at = WallpaperTint::ALL
            .iter()
            .position(|t| *t == self)
            .unwrap_or(0);
        WallpaperTint::ALL[if right {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        }]
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum WallpaperPick {
    None,
    Random,
    /// The theme's own wallpaper (what older cards and a fresh card hold).
    #[default]
    Default,
    /// The dark theme's wallpaper, whatever the theme.
    Dark,
    /// The light theme's wallpaper, whatever the theme.
    Light,
    Blur,
    File(String),
}

impl WallpaperPick {
    pub fn key(&self) -> String {
        match self {
            WallpaperPick::None => "none".to_string(),
            WallpaperPick::Random => "random".to_string(),
            WallpaperPick::Default => "default".to_string(),
            WallpaperPick::Dark => "dark".to_string(),
            WallpaperPick::Light => "light".to_string(),
            WallpaperPick::Blur => "default_blur".to_string(),
            WallpaperPick::File(name) => name.clone(),
        }
    }

    pub fn label(&self) -> String {
        match self {
            WallpaperPick::None => "Off".to_string(),
            WallpaperPick::Random => "Random".to_string(),
            WallpaperPick::Default => "Default".to_string(),
            WallpaperPick::Dark => "Dark".to_string(),
            WallpaperPick::Light => "Light".to_string(),
            WallpaperPick::Blur => "DEFAULT BLUR".to_string(),
            WallpaperPick::File(name) => name
                .rsplit_once('.')
                .map(|(stem, _)| stem)
                .unwrap_or(name)
                .to_string(),
        }
    }

    pub fn parse(value: &str) -> WallpaperPick {
        match value {
            "none" => WallpaperPick::None,
            "random" => WallpaperPick::Random,
            "" | "default" => WallpaperPick::Default,
            "dark" => WallpaperPick::Dark,
            "light" => WallpaperPick::Light,
            "default_blur" => WallpaperPick::Blur,
            other => WallpaperPick::File(other.to_string()),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum BezelPick {
    None,
    Default,
    #[default]
    Ags102,
    File(String),
}

impl BezelPick {
    pub fn key(&self) -> String {
        match self {
            BezelPick::None => "none".to_string(),
            BezelPick::Default => "default".to_string(),
            BezelPick::Ags102 => "ags-102".to_string(),
            BezelPick::File(name) => name.clone(),
        }
    }

    pub fn label(&self) -> String {
        match self {
            BezelPick::None => "None".to_string(),
            BezelPick::Default => "Default".to_string(),
            BezelPick::Ags102 => "AGS-102".to_string(),
            BezelPick::File(name) => name
                .rsplit_once('.')
                .map(|(stem, _)| stem)
                .unwrap_or(name)
                .to_string(),
        }
    }

    pub fn parse(value: &str) -> BezelPick {
        match value {
            "none" => BezelPick::None,
            "default" => BezelPick::Default,
            "" | "ags-102" => BezelPick::Ags102,
            other => BezelPick::File(other.to_string()),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SlotState {
    pub cart: Option<String>,
    pub cart_platform: Option<Platform>,
    /// The cart in the slot is a Game Boy game in the Super Game Boy.
    pub cart_sgb: bool,
    /// The Super Game Boy's border: 0 auto, 1 slot.'s, 2 none, 11-18 the SGB's own 1-8.
    pub sgb_border: u8,
    pub brightness: u8,
    pub blue_light: u8,
    pub volume: u8,
    pub volume_hp: u8,
    pub muted: bool,
    pub muted_hp: bool,
    pub clock_set: bool,
    pub utc_offset_min: i16,
    /// A named zone with its own summer time; wins over `utc_offset_min`.
    pub time_zone: Option<&'static str>,
    /// The clock follows the network over Home Wi-Fi.
    pub auto_time: bool,
    /// 24-hour clock; off shows 6:00 PM.
    pub clock_24h: bool,
    /// Game Boy and Game Boy Color games fill the screen (L) rather than showing at their
    /// own size (R): one setting for both, kept between games (Roy).
    pub gb_fullscreen: bool,
    /// Between the two: the picture as tall as the screen, its width kept to its own shape.
    pub gb_fill_height: bool,
    pub rumble: bool,
    /// Home association only; Link holds its own radio lease.
    pub home_wifi_enabled: bool,
    pub ff_speed: u8,
    pub ff_sound: bool,
    pub shader_gba: Shader,
    pub shader_gb: Shader,
    pub shader_gbc: Shader,
    pub eject_save: bool,
    pub turbo: bool,
    pub rewind: bool,
    pub gb_palettes: bool,
    pub gb_palette: GbPalette,
    pub appearance: Appearance,
    pub wallpaper: WallpaperPick,
    pub wallpaper_tint: WallpaperTint,
    pub favorite_mark: FavoriteMark,
    pub show_recents: bool,
    pub show_titles: bool,
    pub bezel_gba: BezelPick,
    pub bezel_gb: BezelPick,
    pub bezel_gbc: BezelPick,
    pub recents: Vec<Recent>,
    pub favorites: Vec<Recent>,
    pub shelf: Option<String>,
    pub shelf_cart: Option<Recent>,
    pub auto_sleep_secs: u16,
    /// Minutes in the light sleep before the deep one.
    pub deep_sleep_mins: u16,
    /// Pale pixels let the panel through, for each system on its own: GB, GBC, GBA.
    pub pixel_transparency_gb: bool,
    pub pixel_transparency_gbc: bool,
    pub pixel_transparency_gba: bool,
    /// Show the masked robber while cheats are on.
    pub cheat_icon: bool,
    /// GB and GBC pictures framed in black rather than the LCD's own glass.
    pub gbc_black_border: bool,
    pub gb_black_border: bool,
    /// Moving pixels fade out slowly, for each system on its own: GB, GBC, GBA.
    pub lcd_ghosting_gb: bool,
    pub lcd_ghosting_gbc: bool,
    pub lcd_ghosting_gba: bool,
    pub in_game_menu: bool,
    pub remember_position: bool,
    /// Power on straight back into the game that was in the slot.
    pub resume_game: bool,
    /// Powering off in a game leaves its picture as the boot screen.
    pub boot_picture: bool,
    /// The effects SELECT + B switched off, to switch back on: GB, GBC, GBA. 0 for none.
    pub effects_saved_gb: u8,
    pub effects_saved_gbc: u8,
    pub effects_saved_gba: u8,
    /// The TV consoles' screen effects as chosen below, all on or off together (SELECT + B),
    /// for each on its own: NES, SNES.
    pub tv_filter_nes: bool,
    pub tv_filter_snes: bool,
    pub tv_cable_nes: TvCable,
    pub tv_cable_snes: TvCable,
    pub nes_colors: NesColors,
    pub tv_scanlines_nes: bool,
    pub tv_scanlines_snes: bool,
    pub tv_mask_nes: bool,
    pub tv_mask_snes: bool,
    pub tv_afterglow_nes: bool,
    pub tv_afterglow_snes: bool,
}

impl SlotState {
    pub fn played(&mut self, platform: Platform, stem: &str) {
        self.recents
            .retain(|r| !(r.platform == platform && r.stem == stem));
        self.recents.insert(
            0,
            Recent {
                platform,
                stem: stem.to_string(),
            },
        );
        self.recents.truncate(RECENTS_MAX);
    }

    pub fn is_favorite(&self, platform: Platform, stem: &str) -> bool {
        self.favorites
            .iter()
            .any(|f| f.platform == platform && f.stem == stem)
    }

    pub fn toggle_favorite(&mut self, platform: Platform, stem: &str) -> bool {
        if self.is_favorite(platform, stem) {
            self.favorites
                .retain(|f| !(f.platform == platform && f.stem == stem));
            return false;
        }
        self.favorites.push(Recent {
            platform,
            stem: stem.to_string(),
        });
        true
    }
}

/// The cable a TV console's picture travels to the TV by, as its core's NTSC filter draws it;
/// Off for the console's own sharp pixels.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TvCable {
    Off,
    Rf,
    Composite,
    SVideo,
    Rgb,
}

impl TvCable {
    pub const ALL: [TvCable; 5] = [
        TvCable::Off,
        TvCable::Rf,
        TvCable::Composite,
        TvCable::SVideo,
        TvCable::Rgb,
    ];

    pub fn name(self) -> &'static str {
        match self {
            TvCable::Off => "off",
            TvCable::Rf => "rf",
            TvCable::Composite => "composite",
            TvCable::SVideo => "svideo",
            TvCable::Rgb => "rgb",
        }
    }

    /// RF is no longer offered: a card that picked it gets composite, its nearest.
    pub fn parse(name: &str) -> Option<TvCable> {
        if name == "rf" {
            return Some(TvCable::Composite);
        }
        TvCable::ALL.into_iter().find(|c| c.name() == name)
    }
}

/// The colours an NES's picture came out in: as a Trinitron's decoder made them, or the
/// console's own. Only those two are offered (Roy); the core's own stays for the bare picture.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum NesColors {
    Standard,
    Sony,
    /// The NES's own composite colours, as measured off the console.
    Authentic,
}

impl NesColors {
    pub const ALL: [NesColors; 2] = [NesColors::Authentic, NesColors::Sony];

    pub fn name(self) -> &'static str {
        match self {
            NesColors::Standard => "standard",
            NesColors::Sony => "sony",
            NesColors::Authentic => "authentic",
        }
    }

    pub fn parse(name: &str) -> Option<NesColors> {
        NesColors::ALL.into_iter().find(|c| c.name() == name)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Shader {
    Off,
    Lcd3x,
    Grid,
    GbaLcd,
    GbcLcd,
    Dot,
    Simpletex,
}

impl Shader {
    pub const GBA: [Shader; 2] = [Shader::Off, Shader::GbaLcd];
    pub const GB: [Shader; 2] = [Shader::Off, Shader::Grid];
    pub const GBC: [Shader; 2] = [Shader::Off, Shader::GbcLcd];

    /// A GBC setting saved before the GBC had an LCD of its own: off stays off, any shader
    /// becomes the GBC LCD.
    pub fn for_gbc(self) -> Shader {
        match self {
            Shader::Off => Shader::Off,
            _ => Shader::GbcLcd,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Shader::Off => "off",
            Shader::Lcd3x => "lcd3x",
            Shader::Grid => "grid",
            Shader::GbaLcd => "gba_lcd",
            Shader::GbcLcd => "gbc_lcd",
            Shader::Dot => "dot",
            Shader::Simpletex => "simpletex",
        }
    }

    pub fn parse(name: &str) -> Option<Shader> {
        [
            Shader::Off,
            Shader::Lcd3x,
            Shader::Grid,
            Shader::GbaLcd,
            Shader::GbcLcd,
            Shader::Dot,
            Shader::Simpletex,
        ]
        .into_iter()
        .find(|s| s.name() == name)
    }

    pub fn step(self, choices: &[Shader], right: bool) -> Shader {
        let n = choices.len();
        let at = choices.iter().position(|&s| s == self).unwrap_or(0);
        choices[if right {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        }]
    }
}

impl Default for SlotState {
    fn default() -> Self {
        SlotState {
            cart: None,
            cart_platform: None,
            cart_sgb: false,
            sgb_border: 0,
            brightness: 5,
            blue_light: 0,
            volume: 60,
            volume_hp: 60,
            muted: false,
            muted_hp: false,
            clock_set: false,
            utc_offset_min: 0,
            time_zone: None,
            auto_time: true,
            clock_24h: true,
            gb_fullscreen: false,
            gb_fill_height: false,
            rumble: true,
            home_wifi_enabled: false,
            ff_speed: FF_SPEED_DEFAULT,
            ff_sound: false,
            shader_gba: Shader::GbaLcd,
            shader_gb: Shader::Grid,
            shader_gbc: Shader::GbcLcd,
            eject_save: true,
            turbo: true,
            rewind: true,
            gb_palettes: true,
            gb_palette: GbPalette::DEFAULT,
            appearance: Appearance::Dark,
            wallpaper: WallpaperPick::Default,
            wallpaper_tint: WallpaperTint::Off,
            favorite_mark: FavoriteMark::GoldGlow,
            show_recents: false,
            show_titles: false,
            bezel_gba: BezelPick::Ags102,
            bezel_gb: BezelPick::Ags102,
            bezel_gbc: BezelPick::Ags102,
            recents: Vec::new(),
            favorites: Vec::new(),
            shelf: None,
            shelf_cart: None,
            auto_sleep_secs: AUTO_SLEEP_DEFAULT_SECS,
            deep_sleep_mins: DEEP_SLEEP_DEFAULT_MINS,
            pixel_transparency_gb: true,
            pixel_transparency_gbc: true,
            pixel_transparency_gba: true,
            cheat_icon: true,
            gbc_black_border: false,
            gb_black_border: false,
            lcd_ghosting_gb: true,
            lcd_ghosting_gbc: true,
            lcd_ghosting_gba: true,
            in_game_menu: true,
            remember_position: false,
            resume_game: true,
            boot_picture: true,
            effects_saved_gb: 0,
            effects_saved_gbc: 0,
            effects_saved_gba: 0,
            tv_filter_nes: true,
            tv_filter_snes: true,
            tv_cable_nes: TvCable::Composite,
            tv_cable_snes: TvCable::Composite,
            nes_colors: NesColors::Sony,
            tv_scanlines_nes: true,
            tv_scanlines_snes: true,
            tv_mask_nes: true,
            tv_mask_snes: true,
            tv_afterglow_nes: true,
            tv_afterglow_snes: true,
        }
    }
}

fn state_path(root: &Path) -> PathBuf {
    root.join("Config").join("slot.state")
}

const RECENTS_FILE: &str = "recents.txt";
const FAVORITES_FILE: &str = "favorites.txt";

fn list_path(root: &Path, name: &str) -> PathBuf {
    root.join("Config").join(name)
}

pub fn read_slot_state(root: &Path) -> SlotState {
    let mut s: SlotState = std::fs::read(state_path(root))
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| parse(&s))
        .unwrap_or_default();
    // Read the lists before the profile below writes the state back: writing it writes them
    // too, and an unread list would be written out empty.
    s.recents = read_list(root, RECENTS_FILE);
    s.recents.truncate(RECENTS_MAX);
    s.favorites = read_list(root, FAVORITES_FILE);
    let text = std::fs::read_to_string(state_path(root)).unwrap_or_default();
    if !text
        .lines()
        .any(|line| line.trim() == AUTHENTIC_LCD_PROFILE)
    {
        let had_display_profile = text
            .lines()
            .any(|line| line.trim().starts_with("lcd_profile="));
        if !had_display_profile {
            s.shader_gba = Shader::GbaLcd;
            s.shader_gb = Shader::Grid;
            s.shader_gbc = Shader::GbcLcd;
            s.gb_palettes = true;
            if s.auto_sleep_secs == 0 {
                s.auto_sleep_secs = AUTO_SLEEP_DEFAULT_SECS;
            }
        }
        let _ = write_slot_state(root, &s);
    }
    s
}

fn read_list(root: &Path, name: &str) -> Vec<Recent> {
    let text = std::fs::read_to_string(list_path(root, name)).unwrap_or_default();
    let mut list: Vec<Recent> = Vec::new();
    for r in text.lines().filter_map(recent_value) {
        if !list.contains(&r) {
            list.push(r);
        }
    }
    list
}

fn write_list(root: &Path, name: &str, list: &[Recent]) -> std::io::Result<()> {
    let text: String = list
        .iter()
        .map(|r| format!("{}/{}\n", platform_key(r.platform), r.stem))
        .collect();
    let path = list_path(root, name);
    if std::fs::read(&path).map_or(text.is_empty(), |old| old == text.as_bytes()) {
        return Ok(());
    }
    atomic_write(&path, text.as_bytes())
}

pub fn write_slot_state(root: &Path, s: &SlotState) -> std::io::Result<()> {
    let mut text = format!(
        "cart={}\ncart_platform={}\nbrightness={}\nblue_light={}\nvolume={}\nvolume_hp={}\nmuted={}\nmuted_hp={}\nclock_set={}\nutc_offset_min={}\nrumble={}\nff_speed={}\nff_sound={}\nshader_gba={}\nshader_gb={}\neject_save={}\nturbo={}\nrewind={}\ngb_palettes={}\ngb_palette={}\nappearance={}\nwallpaper={}\n",
        s.cart.as_deref().unwrap_or(""),
        s.cart_platform.map_or(String::new(), platform_key),
        s.brightness,
        s.blue_light,
        s.volume,
        s.volume_hp,
        s.muted as u8,
        s.muted_hp as u8,
        s.clock_set as u8,
        s.utc_offset_min,
        s.rumble as u8,
        s.ff_speed,
        s.ff_sound as u8,
        s.shader_gba.name(),
        s.shader_gb.name(),
        s.eject_save as u8,
        s.turbo as u8,
        s.rewind as u8,
        s.gb_palettes as u8,
        s.gb_palette.core_name(),
        s.appearance.name(),
        s.wallpaper.key()
    );
    text.push_str(&format!("shader_gbc={}\n", s.shader_gbc.name()));
    text.push_str(&format!("wallpaper_tint={}\n", s.wallpaper_tint.key()));
    text.push_str(&format!("favorite_mark={}\n", s.favorite_mark.key()));
    text.push_str(&format!("auto_sleep_secs={}\n", s.auto_sleep_secs));
    text.push_str(&format!("deep_sleep_mins={}\n", s.deep_sleep_mins));
    text.push_str(&format!("home_wifi_enabled={}\n", s.home_wifi_enabled as u8));
    text.push_str(&format!("auto_time={}\n", s.auto_time as u8));
    text.push_str(&format!("clock_24h={}\n", s.clock_24h as u8));
    text.push_str(&format!("gb_fullscreen={}\n", s.gb_fullscreen as u8));
    text.push_str(&format!("gb_fill_height={}\n", s.gb_fill_height as u8));
    text.push_str(&format!("cart_sgb={}\n", s.cart_sgb as u8));
    text.push_str(&format!("sgb_border={}\n", s.sgb_border));
    if let Some(zone) = s.time_zone {
        text.push_str(&format!("time_zone={zone}\n"));
    }
    for (key, on) in [
        ("pixel_transparency_gb", s.pixel_transparency_gb),
        ("pixel_transparency_gbc", s.pixel_transparency_gbc),
        ("pixel_transparency_gba", s.pixel_transparency_gba),
        ("lcd_ghosting_gb", s.lcd_ghosting_gb),
        ("lcd_ghosting_gbc", s.lcd_ghosting_gbc),
        ("lcd_ghosting_gba", s.lcd_ghosting_gba),
        ("tv_filter_nes", s.tv_filter_nes),
        ("tv_filter_snes", s.tv_filter_snes),
        ("tv_scanlines_nes", s.tv_scanlines_nes),
        ("tv_scanlines_snes", s.tv_scanlines_snes),
        ("tv_mask_nes", s.tv_mask_nes),
        ("tv_mask_snes", s.tv_mask_snes),
        ("tv_afterglow_nes", s.tv_afterglow_nes),
        ("tv_afterglow_snes", s.tv_afterglow_snes),
    ] {
        text.push_str(&format!("{key}={}\n", on as u8));
    }
    text.push_str(&format!("tv_cable_nes={}\n", s.tv_cable_nes.name()));
    text.push_str(&format!("tv_cable_snes={}\n", s.tv_cable_snes.name()));
    text.push_str(&format!("nes_colors={}\n", s.nes_colors.name()));
    text.push_str(&format!("cheat_icon={}\n", s.cheat_icon as u8));
    text.push_str(&format!("gbc_black_border={}\n", s.gbc_black_border as u8));
    text.push_str(&format!("gb_black_border={}\n", s.gb_black_border as u8));
    text.push_str(&format!("resume_game={}\n", s.resume_game as u8));
    text.push_str(&format!("boot_picture={}\n", s.boot_picture as u8));
    for (key, saved) in [
        ("effects_saved_gb", s.effects_saved_gb),
        ("effects_saved_gbc", s.effects_saved_gbc),
        ("effects_saved_gba", s.effects_saved_gba),
    ] {
        text.push_str(&format!("{key}={saved}\n"));
    }
    text.push_str(&format!("in_game_menu={}\n", s.in_game_menu as u8));
    text.push_str(&format!(
        "remember_position={}\n",
        s.remember_position as u8
    ));
    text.push_str(&format!("show_recents={}\n", s.show_recents as u8));
    text.push_str(&format!("show_titles={}\n", s.show_titles as u8));
    text.push_str(&format!("bezel_gba={}\n", s.bezel_gba.key()));
    text.push_str(&format!("bezel_gb={}\n", s.bezel_gb.key()));
    text.push_str(&format!("bezel_gbc={}\n", s.bezel_gbc.key()));
    text.push_str(&format!("shelf={}\n", s.shelf.as_deref().unwrap_or("")));
    text.push_str(&format!(
        "shelf_cart={}\n",
        s.shelf_cart.as_ref().map_or(String::new(), |r| format!(
            "{}/{}",
            platform_key(r.platform),
            r.stem
        ))
    ));
    text.push_str(AUTHENTIC_LCD_PROFILE);
    text.push('\n');
    atomic_write(&state_path(root), text.as_bytes())?;
    write_list(root, RECENTS_FILE, &s.recents)?;
    write_list(root, FAVORITES_FILE, &s.favorites)
}

fn parse(text: &str) -> Option<SlotState> {
    let mut cart = None;
    let mut cart_platform = None;
    let mut brightness = None;
    let mut blue_light = None;
    let mut volume = None;
    let mut volume_hp = None;
    let mut muted = None;
    let mut muted_hp = None;
    let mut clock_set = None;
    let mut utc_offset_min = None;
    let mut rumble = None;
    let mut home_wifi_enabled = None;
    let mut time_zone = None;
    let mut auto_time = None;
    let mut clock_24h = None;
    let mut gb_fullscreen = None;
    let mut gb_fill_height = None;
    let mut cart_sgb = None;
    let mut sgb_border = None;
    let mut ff_speed = None;
    let mut ff_sound = None;
    let mut shader_gba = None;
    let mut shader_gb = None;
    let mut shader_gbc = None;
    let mut eject_save = None;
    let mut turbo = None;
    let mut rewind = None;
    let mut gb_palettes = None;
    let mut gb_palette = None;
    let mut appearance = None;
    let mut wallpaper = None;
    let mut wallpaper_tint = None;
    let mut favorite_mark = None;
    let mut show_recents = None;
    let mut show_titles = None;
    let mut bezel_gba = None;
    let mut bezel_gb = None;
    let mut bezel_gbc = None;
    let mut shelf = None;
    let mut shelf_cart = None;
    let mut auto_sleep_secs = None;
    let mut deep_sleep_mins = None;
    let mut pixel_transparency = None;
    let mut pixel_transparency_gb = None;
    let mut pixel_transparency_gbc = None;
    let mut pixel_transparency_gba = None;
    let mut cheat_icon = None;
    let mut gbc_black_border = None;
    let mut gb_black_border = None;
    let mut resume_game = None;
    let mut boot_picture = None;
    let mut lcd_ghosting = None;
    let mut lcd_ghosting_gb = None;
    let mut lcd_ghosting_gbc = None;
    let mut lcd_ghosting_gba = None;
    let mut in_game_menu = None;
    let mut remember_position = None;
    let mut effects_saved_gb = None;
    let mut effects_saved_gbc = None;
    let mut effects_saved_gba = None;
    let mut tv_filter_nes = None;
    let mut tv_filter_snes = None;
    let mut tv_cable_nes = None;
    let mut tv_cable_snes = None;
    let mut nes_colors = None;
    let mut tv_scanlines_nes = None;
    let mut tv_scanlines_snes = None;
    let mut tv_mask_nes = None;
    let mut tv_mask_snes = None;
    let mut tv_afterglow_nes = None;
    let mut tv_afterglow_snes = None;
    for line in text.lines().filter(|l| !l.is_empty()) {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "cart" => cart = Some(value.to_string()),
            "cart_platform" => cart_platform = platform_value(value),
            "brightness" => brightness = Some(level(value, BRIGHTNESS_MAX)?),
            "blue_light" => blue_light = Some(level(value, BLUE_LIGHT_MAX)?),
            "volume" => volume = Some(level(value, VOLUME_MAX)?),
            "volume_hp" => volume_hp = Some(level(value, VOLUME_MAX)?),
            "muted" => muted = Some(level(value, 1)? == 1),
            "muted_hp" => muted_hp = Some(level(value, 1)? == 1),
            "clock_set" => clock_set = Some(level(value, 1)? == 1),
            "utc_offset_min" => utc_offset_min = Some(offset(value)?),
            "rumble" => rumble = flag(value),
            "home_wifi_enabled" => home_wifi_enabled = flag(value),
            "auto_time" => auto_time = flag(value),
            "clock_24h" => clock_24h = flag(value),
            "gb_fullscreen" => gb_fullscreen = flag(value),
            "gb_fill_height" => gb_fill_height = flag(value),
            "cart_sgb" => cart_sgb = flag(value),
            "sgb_border" => sgb_border = value.parse::<u8>().ok(),
            "time_zone" => time_zone = crate::zone_named(value).map(|z| z.name),
            "ff_speed" => ff_speed = ff_speed_value(value),
            "ff_sound" => ff_sound = flag(value),
            "shader_gba" => shader_gba = Shader::parse(value).filter(|s| Shader::GBA.contains(s)),
            "eject_save" => eject_save = flag(value),
            "turbo" => turbo = flag(value),
            "rewind" => rewind = flag(value),
            "gb_palettes" => gb_palettes = flag(value),
            "gb_palette" => gb_palette = GbPalette::parse(value),
            "appearance" => appearance = Appearance::parse(value),
            "wallpaper" => wallpaper = Some(WallpaperPick::parse(value)),
            "wallpaper_tint" => wallpaper_tint = WallpaperTint::parse(value),
            "favorite_mark" => favorite_mark = FavoriteMark::parse(value),
            "show_recents" => show_recents = flag(value),
            "show_titles" => show_titles = flag(value),
            "bezel_gba" => bezel_gba = Some(BezelPick::parse(value)),
            "bezel_gb" => bezel_gb = Some(BezelPick::parse(value)),
            "bezel_gbc" => bezel_gbc = Some(BezelPick::parse(value)),
            "shelf" => shelf = Some(value.to_string()).filter(|v| !v.is_empty()),
            "shelf_cart" => shelf_cart = recent_value(value),
            "auto_sleep_secs" => auto_sleep_secs = auto_sleep_value(value),
            "deep_sleep_mins" => {
                deep_sleep_mins = value
                    .parse()
                    .ok()
                    .filter(|mins| DEEP_SLEEP_OPTIONS_MINS.contains(mins))
            }
            "pixel_transparency" => pixel_transparency = flag(value),
            "pixel_transparency_gb" => pixel_transparency_gb = flag(value),
            "pixel_transparency_gbc" => pixel_transparency_gbc = flag(value),
            "pixel_transparency_gba" => pixel_transparency_gba = flag(value),
            "cheat_icon" => cheat_icon = flag(value),
            "gbc_black_border" => gbc_black_border = flag(value),
            "gb_black_border" => gb_black_border = flag(value),
            "resume_game" => resume_game = flag(value),
            "boot_picture" => boot_picture = flag(value),
            "lcd_ghosting" => lcd_ghosting = flag(value),
            "lcd_ghosting_gb" => lcd_ghosting_gb = flag(value),
            "lcd_ghosting_gbc" => lcd_ghosting_gbc = flag(value),
            "lcd_ghosting_gba" => lcd_ghosting_gba = flag(value),
            "tv_filter_nes" => tv_filter_nes = flag(value),
            "tv_filter_snes" => tv_filter_snes = flag(value),
            "tv_cable_nes" => tv_cable_nes = TvCable::parse(value),
            "tv_cable_snes" => tv_cable_snes = TvCable::parse(value),
            "nes_colors" => nes_colors = NesColors::parse(value),
            "tv_scanlines_nes" => tv_scanlines_nes = flag(value),
            "tv_scanlines_snes" => tv_scanlines_snes = flag(value),
            "tv_mask_nes" => tv_mask_nes = flag(value),
            "tv_mask_snes" => tv_mask_snes = flag(value),
            "tv_afterglow_nes" => tv_afterglow_nes = flag(value),
            "tv_afterglow_snes" => tv_afterglow_snes = flag(value),
            "in_game_menu" => in_game_menu = flag(value),
            "remember_position" => remember_position = flag(value),
            "effects_saved_gb" => effects_saved_gb = value.parse().ok(),
            "effects_saved_gbc" => effects_saved_gbc = value.parse().ok(),
            "effects_saved_gba" => effects_saved_gba = value.parse().ok(),
            "shader_gb" => shader_gb = Shader::parse(value).filter(|s| Shader::GB.contains(s)),
            "shader_gbc" => shader_gbc = Shader::parse(value).map(Shader::for_gbc),
            _ => {}
        }
    }
    let cart = cart?;
    let fallback = SlotState::default();
    Some(SlotState {
        cart: (!cart.is_empty()).then_some(cart),
        cart_platform,
        brightness: brightness?,
        blue_light: blue_light?,
        volume: volume?,
        volume_hp: volume_hp.or(volume)?,
        muted: muted?,
        muted_hp: muted_hp.or(muted)?,
        clock_set: clock_set?,
        utc_offset_min: utc_offset_min?,
        time_zone,
        auto_time: auto_time.unwrap_or(true),
        clock_24h: clock_24h.unwrap_or(true),
        gb_fullscreen: gb_fullscreen.unwrap_or(false),
        gb_fill_height: gb_fill_height.unwrap_or(false),
        cart_sgb: cart_sgb.unwrap_or(false),
        sgb_border: sgb_border.unwrap_or(0),
        rumble: rumble.unwrap_or(fallback.rumble),
        home_wifi_enabled: home_wifi_enabled.unwrap_or(false),
        ff_speed: ff_speed.unwrap_or(fallback.ff_speed),
        ff_sound: ff_sound.unwrap_or(fallback.ff_sound),
        shader_gba: shader_gba.unwrap_or(fallback.shader_gba),
        shader_gb: shader_gb.unwrap_or(fallback.shader_gb),
        shader_gbc: shader_gbc
            .or(shader_gb.map(Shader::for_gbc))
            .unwrap_or(fallback.shader_gbc),
        eject_save: eject_save.unwrap_or(fallback.eject_save),
        turbo: turbo.unwrap_or(fallback.turbo),
        rewind: rewind.unwrap_or(fallback.rewind),
        gb_palettes: gb_palettes.unwrap_or(fallback.gb_palettes),
        gb_palette: gb_palette.unwrap_or(fallback.gb_palette),
        appearance: appearance.unwrap_or(fallback.appearance),
        wallpaper: wallpaper.unwrap_or(fallback.wallpaper),
        wallpaper_tint: wallpaper_tint.unwrap_or(fallback.wallpaper_tint),
        favorite_mark: favorite_mark.unwrap_or(fallback.favorite_mark),
        show_recents: show_recents.unwrap_or(fallback.show_recents),
        show_titles: show_titles.unwrap_or(fallback.show_titles),
        bezel_gba: bezel_gba.unwrap_or(fallback.bezel_gba),
        bezel_gb: bezel_gb.unwrap_or(fallback.bezel_gb),
        bezel_gbc: bezel_gbc.unwrap_or(fallback.bezel_gbc),
        recents: Vec::new(),
        favorites: Vec::new(),
        shelf,
        shelf_cart,
        auto_sleep_secs: auto_sleep_secs.unwrap_or(fallback.auto_sleep_secs),
        deep_sleep_mins: deep_sleep_mins.unwrap_or(fallback.deep_sleep_mins),
        // A card from before the screen settings went per system: the one switch, for all.
        pixel_transparency_gb: pixel_transparency_gb
            .or(pixel_transparency)
            .unwrap_or(fallback.pixel_transparency_gb),
        pixel_transparency_gbc: pixel_transparency_gbc
            .or(pixel_transparency)
            .unwrap_or(fallback.pixel_transparency_gbc),
        pixel_transparency_gba: pixel_transparency_gba
            .or(pixel_transparency)
            .unwrap_or(fallback.pixel_transparency_gba),
        cheat_icon: cheat_icon.unwrap_or(fallback.cheat_icon),
        gbc_black_border: gbc_black_border.unwrap_or(fallback.gbc_black_border),
        gb_black_border: gb_black_border.unwrap_or(fallback.gb_black_border),
        resume_game: resume_game.unwrap_or(fallback.resume_game),
        boot_picture: boot_picture.unwrap_or(fallback.boot_picture),
        lcd_ghosting_gb: lcd_ghosting_gb.or(lcd_ghosting).unwrap_or(fallback.lcd_ghosting_gb),
        lcd_ghosting_gbc: lcd_ghosting_gbc.or(lcd_ghosting).unwrap_or(fallback.lcd_ghosting_gbc),
        lcd_ghosting_gba: lcd_ghosting_gba.or(lcd_ghosting).unwrap_or(fallback.lcd_ghosting_gba),
        in_game_menu: in_game_menu.unwrap_or(fallback.in_game_menu),
        remember_position: remember_position.unwrap_or(fallback.remember_position),
        effects_saved_gb: effects_saved_gb.unwrap_or(fallback.effects_saved_gb),
        effects_saved_gbc: effects_saved_gbc.unwrap_or(fallback.effects_saved_gbc),
        effects_saved_gba: effects_saved_gba.unwrap_or(fallback.effects_saved_gba),
        tv_filter_nes: tv_filter_nes.unwrap_or(fallback.tv_filter_nes),
        tv_filter_snes: tv_filter_snes.unwrap_or(fallback.tv_filter_snes),
        tv_cable_nes: tv_cable_nes.unwrap_or(fallback.tv_cable_nes),
        tv_cable_snes: tv_cable_snes.unwrap_or(fallback.tv_cable_snes),
        nes_colors: nes_colors.unwrap_or(fallback.nes_colors),
        tv_scanlines_nes: tv_scanlines_nes.unwrap_or(fallback.tv_scanlines_nes),
        tv_scanlines_snes: tv_scanlines_snes.unwrap_or(fallback.tv_scanlines_snes),
        tv_mask_nes: tv_mask_nes.unwrap_or(fallback.tv_mask_nes),
        tv_mask_snes: tv_mask_snes.unwrap_or(fallback.tv_mask_snes),
        tv_afterglow_nes: tv_afterglow_nes.unwrap_or(fallback.tv_afterglow_nes),
        tv_afterglow_snes: tv_afterglow_snes.unwrap_or(fallback.tv_afterglow_snes),
    })
}

fn platform_key(platform: Platform) -> String {
    platform.dir_name().to_ascii_lowercase()
}

fn recent_value(value: &str) -> Option<Recent> {
    let (platform, stem) = value.split_once('/')?;
    Some(Recent {
        platform: platform_value(platform)?,
        stem: stem.to_string(),
    })
    .filter(|r| !r.stem.is_empty())
}

fn platform_value(value: &str) -> Option<Platform> {
    Platform::ALL
        .into_iter()
        .find(|p| value.eq_ignore_ascii_case(p.dir_name()))
}

fn offset(value: &str) -> Option<i16> {
    value
        .parse()
        .ok()
        .filter(|n| (UTC_OFFSET_MIN..=UTC_OFFSET_MAX).contains(n))
}

fn ff_speed_value(value: &str) -> Option<u8> {
    value.parse().ok().filter(|n| FF_SPEEDS.contains(n))
}

fn auto_sleep_value(value: &str) -> Option<u16> {
    value
        .parse()
        .ok()
        .filter(|secs| AUTO_SLEEP_OPTIONS_SECS.contains(secs))
}

fn level(value: &str, max: u8) -> Option<u8> {
    value.parse().ok().filter(|n| *n <= max)
}

fn flag(value: &str) -> Option<bool> {
    level(value, 1).map(|n| n == 1)
}
