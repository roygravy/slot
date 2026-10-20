//! The Buttons pages in the menu: every shortcut slot answers to, grouped the way the guide at
//! slot-cfw.fyi groups them. Faces are white and tinted with the theme's ink.

use crate::draw::{Canvas, Draw, TexId};
use crate::plate::{centred_hints, UndoFace, HINT_H, LEGEND_GAP};
use crate::text;
use crate::theme::Ink;

pub const BUTTON_PAGES: [(&str, &[(&str, &str)]); 4] = [
    (
        "Everywhere",
        &[
            ("SELECT + Up / Down", "Brightness"),
            ("SELECT + Left / Right", "Blue light filter"),
            ("VOL+ / VOL-", "Volume"),
            ("VOL+ and VOL- together", "Mute"),
            ("SELECT + VOL+ or VOL-", "Mute"),
            ("Hold POWER", "Restart or power off"),
        ],
    ),
    (
        "Shelf",
        &[
            ("Left / Right", "Move along the shelf"),
            ("Up / Down", "Jump to the next letter"),
            ("L1 / R1", "Switch shelf"),
            ("Tap A", "Play from your last save state"),
            ("Hold A", "Start the game fresh"),
            ("Y", "Favorite or unfavorite"),
            ("MENU", "Settings"),
            ("START", "Choose the emulator (GBA)"),
        ],
    ),
    (
        "In Game",
        &[
            ("Hold MENU", "Save, eject, back to the shelf"),
            ("Tap MENU", "In-game menu, when it is on"),
            ("Double tap MENU", "Save state switcher"),
            ("SELECT + MENU", "Link cable to another slot"),
            ("SELECT + R1", "Save state"),
            ("SELECT + L1", "Load the newest save state"),
            ("SELECT + B / A", "LCD grid on / off"),
            ("SELECT + Y", "Original colors on / off"),
            ("SELECT + L2 / R2", "Game Boy palette"),
            ("Hold L2", "Rewind"),
            ("Hold R2", "Fast-forward"),
            ("Double tap R2", "Keep fast-forwarding"),
            ("Hold X / Y", "Turbo A / turbo B"),
            ("L1 / R1", "Stretch / original size (GB)"),
        ],
    ),
    (
        "Save States",
        &[
            ("Left / Right", "Flick through the states"),
            ("A", "Load the one on screen"),
            ("Y", "Delete the one on screen"),
            ("X", "Undo the last save or load"),
            ("B", "Back to the game"),
        ],
    ),
];

pub const BODY_W: u32 = 592;
const TAB_H: u32 = 30;
const TAB_PX: f32 = 18.0;
const TAB_Y: f32 = 22.0;
const TAB_GAP: f32 = 22.0;
const BODY_Y: f32 = 72.0;
const BODY_ROOM: f32 = 344.0;
const ROW_MAX: f32 = 40.0;
const KEY_PX: f32 = 17.0;
const DO_PX: f32 = 17.0;
const KEY_W: f32 = 270.0;
pub const LEGEND_Y: f32 = 427.0;
/// How strongly the description column reads against the key column.
const DO_ALPHA: f32 = 0.72;

pub struct ButtonsFaces {
    /// Each tab's name, white.
    pub tabs: Vec<(TexId, u32, u32)>,
    /// Each page's body: keys in white, what they do in a fainter white.
    pub bodies: Vec<(TexId, u32, u32)>,
    pub legend: [(TexId, u32); 2],
}

fn word(font: &fontdue::Font, s: &str, px: f32, max_w: f32, h: u32) -> UndoFace {
    let layout = text::fit(font, s, max_w, 1, px, px * 0.7);
    let set = layout
        .lines
        .iter()
        .map(|l| text::line_width(font, l, layout.px, layout.tracking))
        .fold(0.0, f32::max);
    let w = (set.ceil() as u32 + 2).max(1);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    text::draw_centred(&mut rgba, w, h, &layout, [0xff, 0xff, 0xff]);
    UndoFace { rgba, w, h }
}

pub fn buttons_tab_face(page: usize) -> UndoFace {
    let Some(font) = text::label_font() else {
        return UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    word(font, BUTTON_PAGES[page].0, TAB_PX, 200.0, TAB_H)
}

fn pitch(rows: usize) -> f32 {
    (BODY_ROOM / rows.max(1) as f32).floor().min(ROW_MAX)
}

pub fn buttons_body_face(page: usize) -> UndoFace {
    let rows = BUTTON_PAGES[page].1;
    let p = pitch(rows.len());
    let h = (p * rows.len() as f32).ceil() as u32;
    let w = BODY_W;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let Some(font) = text::label_font() else {
        return UndoFace { rgba, w, h };
    };
    let line_h = p as u32;
    for (i, (key, what)) in rows.iter().enumerate() {
        let y = (i as f32 * p) as u32;
        let k = word(font, key, KEY_PX, KEY_W - 8.0, line_h);
        stamp(&mut rgba, w, h, &k, 0, y, 1.0);
        let d = word(font, what, DO_PX, w as f32 - KEY_W, line_h);
        stamp(&mut rgba, w, h, &d, KEY_W as u32, y, DO_ALPHA);
    }
    UndoFace { rgba, w, h }
}

fn stamp(dst: &mut [u8], dw: u32, dh: u32, src: &UndoFace, x: u32, y: u32, alpha: f32) {
    for sy in 0..src.h {
        let ty = y + sy;
        if ty >= dh {
            break;
        }
        for sx in 0..src.w {
            let tx = x + sx;
            if tx >= dw {
                break;
            }
            let from = ((sy * src.w + sx) * 4) as usize;
            let to = ((ty * dw + tx) * 4) as usize;
            let a = (src.rgba[from + 3] as f32 * alpha).round() as u8;
            if a > dst[to + 3] {
                dst[to..to + 4].copy_from_slice(&[0xff, 0xff, 0xff, a]);
            }
        }
    }
}

pub struct ButtonsHelp<'a> {
    pub page: usize,
    pub faces: Option<&'a ButtonsFaces>,
    pub ground: Option<TexId>,
    pub canvas: Canvas,
    pub ink: Ink,
}

impl ButtonsHelp<'_> {
    pub fn draw(&self, out: &mut Vec<Draw>) {
        crate::quick_menu::draw_menu_ground(self.canvas, self.ground, self.ink, out);
        let Some(faces) = self.faces else {
            return;
        };
        let total: f32 = faces.tabs.iter().map(|t| t.1 as f32).sum::<f32>()
            + TAB_GAP * 2.0 * faces.tabs.len() as f32;
        let mut x = self.canvas.centred(total).round();
        for (i, &(tex, w, h)) in faces.tabs.iter().enumerate() {
            let lit = i == self.page;
            let cell = w as f32 + 2.0 * TAB_GAP;
            if lit {
                out.push(Draw::Rect {
                    x,
                    y: TAB_Y,
                    w: cell,
                    h: h as f32,
                    colour: self.ink.bar,
                });
            }
            out.push(Draw::Tint {
                x: (x + TAB_GAP).round(),
                y: TAB_Y,
                w: w as f32,
                h: h as f32,
                tex,
                colour: if lit { self.ink.on_bar } else { self.ink.dim },
            });
            x += cell;
        }
        if let Some(&(tex, w, h)) = faces.bodies.get(self.page) {
            out.push(Draw::Tint {
                x: self.canvas.centred(w as f32).round(),
                y: BODY_Y + ((BODY_ROOM - h as f32) / 2.0).max(0.0).round(),
                w: w as f32,
                h: h as f32,
                tex,
                colour: self.ink.text,
            });
        }
        for (tex, w, x) in centred_hints(self.canvas, &faces.legend, LEGEND_GAP) {
            out.push(Draw::Tint {
                x,
                y: LEGEND_Y,
                w: w as f32,
                h: HINT_H as f32,
                tex,
                colour: self.ink.text,
            });
        }
    }
}
