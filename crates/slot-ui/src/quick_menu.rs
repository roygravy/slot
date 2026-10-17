use crate::draw::{Canvas, Draw, TexId, OUT_H, OUT_W};
use crate::plate::{arrows_hint_face, centred_hints, hint_face, UndoFace, HINT_H, LEGEND_GAP};
use crate::power_menu::{MENU_H, MENU_INK, MENU_PAD};
use crate::slot_chrome::{edge, opening};
use crate::text;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum QuickRow {
    FastForward,
    FastForwardSound,
    Screen,
    Rumble,
    DateTime,
    About,
    GbaShader,
    GbShader,
    GbaColourCorrection,
    GbColourCorrection,
    Game,
    EjectSave,
    Turbo,
    Rewind,
    GbPalettes,
    RunAhead,
    Theme,
    Wallpaper,
    General,
    Recents,
    ShowTitles,
    WallpaperTint,
    GbcShader,
    GbcColourCorrection,
    Resume,
    States,
    Settings,
    ExitGame,
    PlayScreen,
    PlayGameplay,
    Shader,
    ColourCorrection,
    Palette,
    ScreenGba,
    GbBezel,
    GbcBezel,
    Bezel,
    GbaBezel,
    ScreenGbc,
    ScreenGb,
    AutoSleep,
    PixelTransparency,
    LcdGhosting,
    InGameMenu,
    RememberPosition,
    FavoriteMark,
}

impl QuickRow {
    pub const ALL: [QuickRow; 46] = [
        QuickRow::FastForward,
        QuickRow::FastForwardSound,
        QuickRow::Screen,
        QuickRow::Rumble,
        QuickRow::DateTime,
        QuickRow::About,
        QuickRow::GbaShader,
        QuickRow::GbShader,
        QuickRow::GbaColourCorrection,
        QuickRow::GbColourCorrection,
        QuickRow::Game,
        QuickRow::EjectSave,
        QuickRow::Turbo,
        QuickRow::Rewind,
        QuickRow::GbPalettes,
        QuickRow::RunAhead,
        QuickRow::Theme,
        QuickRow::Wallpaper,
        QuickRow::General,
        QuickRow::Recents,
        QuickRow::ShowTitles,
        QuickRow::WallpaperTint,
        QuickRow::GbcShader,
        QuickRow::GbcColourCorrection,
        QuickRow::Resume,
        QuickRow::States,
        QuickRow::Settings,
        QuickRow::ExitGame,
        QuickRow::PlayScreen,
        QuickRow::PlayGameplay,
        QuickRow::Shader,
        QuickRow::ColourCorrection,
        QuickRow::Palette,
        QuickRow::ScreenGba,
        QuickRow::GbBezel,
        QuickRow::GbcBezel,
        QuickRow::Bezel,
        QuickRow::GbaBezel,
        QuickRow::ScreenGbc,
        QuickRow::ScreenGb,
        QuickRow::AutoSleep,
        QuickRow::PixelTransparency,
        QuickRow::LcdGhosting,
        QuickRow::InGameMenu,
        QuickRow::RememberPosition,
        QuickRow::FavoriteMark,
    ];

    pub const MAIN: [QuickRow; 3] = [QuickRow::General, QuickRow::Screen, QuickRow::Game];

    pub const GENERAL: [QuickRow; 10] = [
        QuickRow::Wallpaper,
        QuickRow::WallpaperTint,
        QuickRow::Theme,
        QuickRow::Recents,
        QuickRow::FavoriteMark,
        QuickRow::ShowTitles,
        QuickRow::DateTime,
        QuickRow::AutoSleep,
        QuickRow::InGameMenu,
        QuickRow::RememberPosition,
    ];

    pub const SCREEN: [QuickRow; 5] = [
        QuickRow::ScreenGba,
        QuickRow::ScreenGbc,
        QuickRow::ScreenGb,
        QuickRow::PixelTransparency,
        QuickRow::LcdGhosting,
    ];

    pub const SCREEN_GBA: [QuickRow; 3] = [
        QuickRow::GbaShader,
        QuickRow::GbaColourCorrection,
        QuickRow::GbaBezel,
    ];

    pub const SCREEN_GBC: [QuickRow; 3] = [
        QuickRow::GbcShader,
        QuickRow::GbcColourCorrection,
        QuickRow::GbcBezel,
    ];

    pub const SCREEN_GB: [QuickRow; 4] = [
        QuickRow::GbShader,
        QuickRow::GbColourCorrection,
        QuickRow::GbPalettes,
        QuickRow::GbBezel,
    ];

    pub const GAME: [QuickRow; 7] = [
        QuickRow::FastForward,
        QuickRow::FastForwardSound,
        QuickRow::Rewind,
        QuickRow::RunAhead,
        QuickRow::Turbo,
        QuickRow::Rumble,
        QuickRow::EjectSave,
    ];

    pub const PLAY: [QuickRow; 4] = [
        QuickRow::Resume,
        QuickRow::States,
        QuickRow::Settings,
        QuickRow::ExitGame,
    ];

    pub const SETTINGS: [QuickRow; 2] = [QuickRow::PlayScreen, QuickRow::PlayGameplay];

    pub const PLAY_SCREEN: [QuickRow; 4] = [
        QuickRow::Shader,
        QuickRow::ColourCorrection,
        QuickRow::Palette,
        QuickRow::Bezel,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn page(self) -> &'static [QuickRow] {
        match self {
            QuickRow::ScreenGba | QuickRow::ScreenGbc | QuickRow::ScreenGb => &QuickRow::SCREEN,
            QuickRow::GbaShader | QuickRow::GbaColourCorrection | QuickRow::GbaBezel => {
                &QuickRow::SCREEN_GBA
            }
            QuickRow::GbcShader | QuickRow::GbcColourCorrection | QuickRow::GbcBezel => {
                &QuickRow::SCREEN_GBC
            }
            QuickRow::GbShader
            | QuickRow::GbColourCorrection
            | QuickRow::GbPalettes
            | QuickRow::GbBezel => &QuickRow::SCREEN_GB,
            QuickRow::PixelTransparency | QuickRow::LcdGhosting => &QuickRow::SCREEN,
            QuickRow::FastForward
            | QuickRow::FastForwardSound
            | QuickRow::Rewind
            | QuickRow::RunAhead
            | QuickRow::Turbo
            | QuickRow::Rumble
            | QuickRow::EjectSave => &QuickRow::GAME,
            QuickRow::Wallpaper
            | QuickRow::WallpaperTint
            | QuickRow::DateTime
            | QuickRow::Theme
            | QuickRow::Recents
            | QuickRow::FavoriteMark
            | QuickRow::ShowTitles
            | QuickRow::AutoSleep
            | QuickRow::InGameMenu
            | QuickRow::RememberPosition => &QuickRow::GENERAL,
            QuickRow::Resume | QuickRow::States | QuickRow::Settings | QuickRow::ExitGame => {
                &QuickRow::PLAY
            }
            QuickRow::PlayScreen | QuickRow::PlayGameplay => &QuickRow::SETTINGS,
            QuickRow::Shader | QuickRow::ColourCorrection | QuickRow::Palette | QuickRow::Bezel => {
                &QuickRow::PLAY_SCREEN
            }
            _ => &QuickRow::MAIN,
        }
    }

    pub fn parent(self) -> Option<QuickRow> {
        match self.page()[0] {
            QuickRow::ScreenGba => Some(QuickRow::Screen),
            QuickRow::GbaShader => Some(QuickRow::ScreenGba),
            QuickRow::GbcShader => Some(QuickRow::ScreenGbc),
            QuickRow::GbShader => Some(QuickRow::ScreenGb),
            QuickRow::FastForward => Some(QuickRow::Game),
            QuickRow::Wallpaper => Some(QuickRow::General),
            QuickRow::AutoSleep => Some(QuickRow::General),
            QuickRow::InGameMenu | QuickRow::RememberPosition => Some(QuickRow::General),
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
            &QuickRow::SCREEN,
        ]
        .contains(&self.page())
    }

    pub fn child(self) -> Option<QuickRow> {
        match self {
            QuickRow::Screen => Some(QuickRow::SCREEN[0]),
            QuickRow::Game => Some(QuickRow::GAME[0]),
            QuickRow::General => Some(QuickRow::GENERAL[0]),
            QuickRow::Settings => Some(QuickRow::SETTINGS[0]),
            QuickRow::PlayScreen => Some(QuickRow::PLAY_SCREEN[0]),
            QuickRow::PlayGameplay => Some(QuickRow::GAME[0]),
            QuickRow::ScreenGba => Some(QuickRow::SCREEN_GBA[0]),
            QuickRow::ScreenGbc => Some(QuickRow::SCREEN_GBC[0]),
            QuickRow::ScreenGb => Some(QuickRow::SCREEN_GB[0]),
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
            QuickRow::DateTime => "Date & Time",
            QuickRow::About => "About",
            QuickRow::GbaShader => "Shader",
            QuickRow::GbShader => "Shader",
            QuickRow::GbcShader => "Shader",
            QuickRow::GbaColourCorrection => "Color Correction",
            QuickRow::GbColourCorrection => "Color Correction",
            QuickRow::GbcColourCorrection => "Color Correction",
            QuickRow::Game => "Gameplay",
            QuickRow::EjectSave => "Auto Save on Eject",
            QuickRow::Turbo => "Turbo Buttons",
            QuickRow::Rewind => "Rewind",
            QuickRow::GbPalettes => "Palettes",
            QuickRow::RunAhead => "Run-Ahead",
            QuickRow::Theme => "Theme",
            QuickRow::Wallpaper => "Wallpaper",
            QuickRow::General => "General",
            QuickRow::Recents => "Recents",
            QuickRow::FavoriteMark => "Favorites",
            QuickRow::ShowTitles => "Game Titles",
            QuickRow::WallpaperTint => "Wallpaper Tint",
            QuickRow::AutoSleep => "Auto Sleep",
            QuickRow::PixelTransparency => "Pixel Transparency",
            QuickRow::LcdGhosting => "LCD Ghosting",
            QuickRow::InGameMenu => "In-Game Menu",
            QuickRow::RememberPosition => "Remember Shelf Position",
            QuickRow::Resume => "Resume",
            QuickRow::States => "States",
            QuickRow::Settings => "Settings",
            QuickRow::ExitGame => "Exit Game",
            QuickRow::PlayScreen => "Screen",
            QuickRow::PlayGameplay => "Gameplay",
            QuickRow::Shader => "Shader",
            QuickRow::ColourCorrection => "Color Correction",
            QuickRow::Palette => "Palette",
            QuickRow::ScreenGba => "Game Boy Advance",
            QuickRow::ScreenGbc => "Game Boy Color",
            QuickRow::ScreenGb => "Game Boy",
            QuickRow::GbBezel => "Bezel",
            QuickRow::GbcBezel => "Bezel",
            QuickRow::GbaBezel => "Bezel",
            QuickRow::Bezel => "Bezel",
        }
    }

    pub fn opens(self) -> bool {
        matches!(
            self,
            QuickRow::DateTime
                | QuickRow::About
                | QuickRow::Screen
                | QuickRow::Game
                | QuickRow::General
                | QuickRow::Resume
                | QuickRow::States
                | QuickRow::Settings
                | QuickRow::ExitGame
                | QuickRow::PlayScreen
                | QuickRow::PlayGameplay
                | QuickRow::ScreenGba
                | QuickRow::ScreenGbc
                | QuickRow::ScreenGb
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
    Show,
    Hide,
    Low,
    Medium,
    High,
    Sleep30s,
    Sleep1m,
    Sleep5m,
    GoldGlow,
    WhiteGlow,
    StarMark,
}

impl QuickValue {
    pub const ALL: [QuickValue; 25] = [
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
        QuickValue::Show,
        QuickValue::Hide,
        QuickValue::Low,
        QuickValue::Medium,
        QuickValue::High,
        QuickValue::Sleep30s,
        QuickValue::Sleep1m,
        QuickValue::Sleep5m,
        QuickValue::GoldGlow,
        QuickValue::WhiteGlow,
        QuickValue::StarMark,
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
            QuickValue::Show => "On",
            QuickValue::Hide => "Off",
            QuickValue::Low => "25%",
            QuickValue::Medium => "45%",
            QuickValue::High => "65%",
            QuickValue::Sleep30s => "30 sec",
            QuickValue::Sleep1m => "1 min",
            QuickValue::Sleep5m => "5 min",
            QuickValue::GoldGlow => "Gold Glow",
            QuickValue::WhiteGlow => "White Glow",
            QuickValue::StarMark => "Star",
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
const GROUND_SCRIM: f32 = 0.65;
const TYPE_DROP: f32 = 4.0;
const BAR_INSET: f32 = 4.0;
const QUICK_ROOM: f32 = 411.0;
const CARET_GAP: f32 = 14.0;
const CARET_PX: f32 = 24.0;
const LEGEND_Y: f32 = 427.0;
const DIM_INK: [u8; 3] = [0x9a, 0x9a, 0xa4];
const PANEL_GAP: f32 = 12.0;

pub fn quick_label_face(row: QuickRow, lit: bool) -> UndoFace {
    quick_text_face(row.label(), if lit { MENU_INK } else { DIM_INK })
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

pub fn quick_pointer_face() -> UndoFace {
    let s = POINTER_SCALE;
    let w = POINTER[0].len() * s;
    let h = (MENU_H + 2 * POINTER_BLEED) as usize;
    let centre = match text::label_font() {
        Some(label) => cap_centre(label, MENU_H),
        None => MENU_H as f32 / 2.0,
    } + POINTER_BLEED as f32;
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

pub fn quick_value_face(text: &str, lit: bool) -> UndoFace {
    quick_text_face(text, if lit { MENU_INK } else { DIM_INK })
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
            rgba[at..at + 4].copy_from_slice(&[MENU_INK[0], MENU_INK[1], MENU_INK[2], a]);
        }
    }
    UndoFace { rgba, w, h }
}

pub fn quick_legend_faces() -> [UndoFace; 4] {
    [
        hint_face("B", "Back"),
        arrows_hint_face("Change"),
        hint_face("A", "Open"),
        hint_face("B", "Resume"),
    ]
}

pub struct QuickMenuFaces {
    pub labels: Vec<[(TexId, u32, u32); 2]>,
    pub pointer: (TexId, u32, u32),
    pub values: Vec<[(TexId, u32, u32); 2]>,
    pub carets: [(TexId, u32, u32); 2],
    pub legend: [(TexId, u32); 4],
}

pub struct QuickMenu<'a> {
    pub row: QuickRow,
    pub hidden: &'a [QuickRow],
    pub values: [Option<QuickValue>; QuickRow::ALL.len()],
    pub clock: Option<[(TexId, u32, u32); 2]>,
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
}

impl QuickMenu<'_> {
    pub fn draw(&self, out: &mut Vec<Draw>) {
        match (self.over_game, self.row.page() == QuickRow::PLAY_SCREEN) {
            (false, _) => draw_menu_ground(self.canvas, self.ground, out),
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
                    colour: [0.0, 0.0, 0.0, GROUND_SCRIM],
                });
            }
        }
        let rows = self.row.shown(self.hidden).len();
        let pitch = quick_pitch(rows);
        out.push(Draw::Rect {
            x: 0.0,
            y: quick_row_top(self.row, self.hidden) + BAR_INSET,
            w: self.canvas.w,
            h: pitch - 2.0 * BAR_INSET,
            colour: edge(),
        });
        let Some(faces) = self.faces else {
            return;
        };
        let (right, pad) = (self.canvas.w - QUICK_EDGE, MENU_PAD as f32);
        let main_w = self
            .row
            .page()
            .iter()
            .filter_map(|r| faces.labels.get(r.index()))
            .map(|l| l[1].1)
            .max()
            .unwrap_or(0);
        for row in self.row.shown(self.hidden) {
            let y = quick_row_top(row, self.hidden) + TYPE_DROP + (pitch - QUICK_PITCH) / 2.0;
            let lit = row == self.row;
            if let Some(&(tex, w, h)) = faces.labels.get(row.index()).map(|l| &l[lit as usize]) {
                let x = match row.centred() {
                    true => self.canvas.centred(main_w as f32),
                    false => QUICK_LABEL_X - pad,
                };
                push(out, tex, x, y, w, h);
            }
            let value = match row {
                QuickRow::DateTime => self.clock.map(|c| c[lit as usize]),
                QuickRow::Wallpaper => self.wallpaper.map(|c| c[lit as usize]),
                QuickRow::Palette => self.palette.map(|c| c[lit as usize]),
                QuickRow::GbaBezel => self.gba_bezel.map(|c| c[lit as usize]),
                QuickRow::GbBezel => self.gb_bezel.map(|c| c[lit as usize]),
                QuickRow::GbcBezel => self.gbc_bezel.map(|c| c[lit as usize]),
                QuickRow::Bezel => self.bezel.map(|c| c[lit as usize]),
                _ => self.values[row.index()]
                    .and_then(|v| faces.values.get(v.index()))
                    .map(|v| v[lit as usize]),
            };
            let Some((tex, w, h)) = value else {
                continue;
            };
            if !lit || row.opens() {
                push(out, tex, right + pad - w as f32, y, w, h);
                continue;
            }
            let [(left_tex, lw, lh), (right_tex, rw, rh)] = faces.carets;
            let rx = right - rw as f32;
            push(out, right_tex, rx, y, rw, rh);
            let vx = rx - CARET_GAP + pad - w as f32;
            push(out, tex, vx, y, w, h);
            push(out, left_tex, vx + pad - CARET_GAP - lw as f32, y, lw, lh);
        }
        let [back, change, open, resume] = faces.legend;
        let back = match self.row.page() == QuickRow::PLAY {
            true => resume,
            false => back,
        };
        let other = if self.row.opens() { open } else { change };
        for (tex, w, x) in centred_hints(self.canvas, &[back, other], LEGEND_GAP) {
            push(out, tex, x, LEGEND_Y, w, HINT_H);
        }
    }
}

pub fn draw_menu_ground(canvas: Canvas, ground: Option<TexId>, out: &mut Vec<Draw>) {
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
                uv: canvas.window(OUT_W as f32),
            });
            out.push(Draw::Rect {
                x: 0.0,
                y: 0.0,
                w,
                h,
                colour: [0.0, 0.0, 0.0, GROUND_SCRIM],
            });
        }
        None => out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
            colour: opening(),
        }),
    }
}

/// The row height: the full pitch while the page fits, squeezed so a longer page still does.
pub fn quick_pitch(rows: usize) -> f32 {
    if rows == 0 {
        return QUICK_PITCH;
    }
    (QUICK_ROOM / rows as f32).floor().clamp(MENU_H as f32, QUICK_PITCH)
}

pub fn quick_row_top(row: QuickRow, hidden: &[QuickRow]) -> f32 {
    let pitch = quick_pitch(row.shown(hidden).len());
    let rows = pitch * row.shown(hidden).len() as f32;
    let top = match row.page() == QuickRow::PLAY_SCREEN {
        true => LEGEND_Y - PANEL_GAP - rows,
        false => (LEGEND_Y - rows) / 2.0,
    };
    top + pitch * row.position(hidden) as f32
}

fn push(out: &mut Vec<Draw>, tex: TexId, x: f32, y: f32, w: u32, h: u32) {
    out.push(Draw::Tex {
        x: x.round(),
        y: y.round(),
        w: w as f32,
        h: h as f32,
        tex,
        alpha: 1.0,
    });
}
