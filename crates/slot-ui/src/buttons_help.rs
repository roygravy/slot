//! The Buttons pages in the menu: every shortcut slot answers to, grouped the way the guide at
//! slot-cfw.fyi groups them. Faces are white and tinted with the theme's ink.

use crate::draw::{Canvas, Draw, TexId};
use crate::plate::{centred_hints, UndoFace, HINT_H, LEGEND_GAP};
use crate::text;
use crate::theme::Ink;

/// Every page keeps to a few rows; a tab with more runs over pages of its own, ‹ and ›.
pub const BUTTON_PAGES: [(&str, &[(&str, &str)]); 7] = [
    (
        "Everywhere",
        &[
            ("SELECT + Up / Down", "Brightness"),
            ("SELECT + Left / Right", "Blue light filter"),
            ("VOL+ / VOL-", "Volume"),
            ("VOL+ and VOL-", "Mute"),
        ],
    ),
    (
        "Everywhere",
        &[
            ("POWER", "Sleep / wake up"),
            ("Hold POWER", "Power off"),
            ("VOL+ and POWER", "Screenshot"),
        ],
    ),
    (
        "Shelf",
        &[
            ("Left / Right", "Move along the shelf"),
            ("Up / Down", "Jump to the next letter"),
            ("L1 / R1", "Switch shelf"),
            ("MENU", "Settings"),
        ],
    ),
    (
        "Shelf",
        &[
            ("Tap A", "Play from the last state"),
            ("Hold A", "Start the game fresh"),
            ("Y", "Favorite or unfavorite"),
            ("START", "Emulator choice (GBA)"),
        ],
    ),
    (
        "In Game",
        &[
            ("Tap MENU", "In-game menu"),
            ("Hold MENU", "Save and eject"),
            ("Double tap MENU", "Save state switcher"),
            ("SELECT + MENU", "Link cable"),
            ("START + SELECT", "Reset the game"),
        ],
    ),
    (
        "In Game",
        &[
            ("SELECT + R1", "Save state"),
            ("SELECT + L1", "Load the newest state"),
            ("SELECT + B", "Screen effects on / off"),
            ("SELECT + L2 / R2", "Game Boy palette"),
            ("L1 / R1", "Stretch or not (GB)"),
        ],
    ),
    (
        "In Game",
        &[
            ("Hold L2", "Rewind"),
            ("Hold R2", "Fast-forward"),
            ("Double tap R2", "Keep fast-forwarding"),
            ("Hold X / Y", "Turbo A / turbo B"),
        ],
    ),
];

/// The rows sit between the ‹ and ›, clear of them.
pub const BODY_W: u32 = 528;
const TAB_H: u32 = 30;
const TAB_PX: f32 = 17.0;
const TAB_Y: f32 = 22.0;
const TAB_GAP: f32 = 22.0;
/// Room kept free at each side of the tab row.
const TAB_EDGE: f32 = 8.0;
const BODY_Y: f32 = 72.0;
const BODY_ROOM: f32 = 344.0;
const ROW_MAX: f32 = 40.0;
const KEY_PX: f32 = 17.0;
const DO_PX: f32 = 17.0;
const KEY_W: f32 = 244.0;
pub const LEGEND_Y: f32 = 427.0;
/// How strongly the description column reads against the key column.
const DO_ALPHA: f32 = 0.72;

pub struct ButtonsFaces {
    /// Each tab's name, white.
    pub tabs: Vec<(TexId, u32, u32)>,
    /// Each page's body: keys in white, what they do in a fainter white.
    pub bodies: Vec<(TexId, u32, u32)>,
    pub legend: [(TexId, u32); 2],
    /// ‹ and ›: a tab's earlier and later page, in white.
    pub arrows: [(TexId, u32, u32); 2],
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

/// Whether two pages share a tab: In Game runs over two.
fn same_tab(a: usize, b: usize) -> bool {
    BUTTON_PAGES.get(a).zip(BUTTON_PAGES.get(b)).is_some_and(|(a, b)| a.0 == b.0)
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
        // The tabs share the width: as many as there are, the gaps between them narrow.
        // A tab's later pages share its place in the row.
        let first = |i: usize| i == 0 || !same_tab(i - 1, i);
        let shown = || faces.tabs.iter().enumerate().filter(|(i, _)| first(*i));
        let words: f32 = shown().map(|(_, t)| t.1 as f32).sum();
        let n = shown().count().max(1) as f32;
        let gap = ((self.canvas.w - 2.0 * TAB_EDGE - words) / (2.0 * n)).clamp(4.0, TAB_GAP).floor();
        let total = words + gap * 2.0 * n;
        let mut x = self.canvas.centred(total).round();
        for (i, &(tex, w, h)) in shown() {
            let lit = (i..faces.tabs.len())
                .take_while(|&j| j == i || same_tab(j - 1, j))
                .any(|j| j == self.page);
            let cell = w as f32 + 2.0 * gap;
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
                x: (x + gap).round(),
                y: TAB_Y,
                w: w as f32,
                h: h as f32,
                tex,
                colour: if lit { self.ink.on_bar } else { self.ink.dim },
            });
            x += cell;
        }
        // ‹ and ›: this tab has a page before or after this one.
        let more = [
            self.page > 0 && same_tab(self.page - 1, self.page),
            same_tab(self.page, self.page + 1),
        ];
        for (right, &(tex, w, h)) in faces.arrows.iter().enumerate() {
            if !more[right] {
                continue;
            }
            // Where the menus' own carets sit, and as dim as their scroll arrows.
            let x = match right {
                1 => self.canvas.w - crate::quick_menu::QUICK_EDGE - w as f32,
                _ => crate::quick_menu::QUICK_EDGE,
            };
            out.push(Draw::Tint {
                x: x.round(),
                y: (BODY_Y + (BODY_ROOM - h as f32) / 2.0).round(),
                w: w as f32,
                h: h as f32,
                tex,
                colour: self.ink.dim,
            });
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
