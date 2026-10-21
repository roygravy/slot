//! The Cheats page of the in-game menu: one row per cheat with a switch, scrolling when the
//! game has more cheats than fit, laid out like the other settings pages. Also the masked
//! robber that shows while any cheat is on: in the menu's corner beside the battery, and in
//! the bottom left while playing.

use crate::draw::{Canvas, Draw, TexId, OUT_H};
use crate::icon::{icon_glyph_face, Icon};
use crate::plate::{centred_hints, UndoFace, HINT_H, LEGEND_GAP};
use crate::theme::OVER_GAME;

/// Rows on screen at once, and their pitch: the settings pages' own.
pub const CHEAT_ROWS: usize = crate::quick_menu::SUB_ROWS;
const PITCH: f32 = crate::quick_menu::QUICK_PITCH;
const MORE_GAP: f32 = crate::quick_menu::MORE_GAP;
const BAR_INSET: f32 = 4.0;
const LABEL_X: f32 = 33.0;
const LABEL_DROP: f32 = 4.0;
const SWITCH_RIGHT: f32 = 32.0;
const SWITCH_DROP: f32 = 10.0;
const HELP_Y: f32 = 392.0;
const LEGEND_Y: f32 = 427.0;
const EMPTY_Y: f32 = 170.0;

/// The robber's height in the menu corner (a pixel taller than the battery) and in play.
pub const BADGE_MENU_H: u32 = 13;
pub const BADGE_GAME_H: u32 = 22;
/// In play the robber mirrors the fast-forward badge: this far in from the bottom left.
const BADGE_GAME_X: f32 = 14.0;
const BADGE_GAME_BOTTOM: f32 = 12.0;

pub struct CheatFaces {
    /// Each cheat's name, white.
    pub names: Vec<(TexId, u32, u32)>,
    /// "N of M cheats on…", rebuilt when the count changes.
    pub help: Option<(TexId, u32, u32)>,
    pub empty: (TexId, u32, u32),
    pub empty_help: (TexId, u32, u32),
    /// Back, Toggle, All Off; and Back alone for a game without cheats.
    pub legend: Vec<(TexId, u32)>,
    pub legend_empty: Vec<(TexId, u32)>,
    /// Off, on.
    pub switches: [(TexId, u32, u32); 2],
    /// Pointing up and down, for more rows above or below.
    pub more: [(TexId, u32, u32); 2],
}

/// The first row shown so that `sel` is on screen, keeping the list still while it can.
pub fn first_shown(sel: usize, count: usize, first: usize) -> usize {
    crate::quick_menu::window_first(sel, count, first, CHEAT_ROWS)
}

fn tint(out: &mut Vec<Draw>, f: (TexId, u32, u32), x: f32, y: f32, colour: [f32; 4]) {
    out.push(Draw::Tint {
        x: x.round(),
        y: y.round(),
        w: f.1 as f32,
        h: f.2 as f32,
        tex: f.0,
        colour,
    });
}

pub struct CheatPage<'a> {
    pub sel: usize,
    pub first: usize,
    pub on: &'a [bool],
    pub faces: Option<&'a CheatFaces>,
    pub canvas: Canvas,
}

impl CheatPage<'_> {
    pub fn draw(&self, out: &mut Vec<Draw>) {
        let ink = OVER_GAME;
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: self.canvas.w,
            h: OUT_H as f32,
            colour: ink.wash,
        });
        let Some(faces) = self.faces else {
            return;
        };
        let legend = |names: &[(TexId, u32)], out: &mut Vec<Draw>| {
            for (tex, w, x) in centred_hints(self.canvas, names, LEGEND_GAP) {
                out.push(Draw::Tint {
                    x,
                    y: LEGEND_Y,
                    w: w as f32,
                    h: HINT_H as f32,
                    tex,
                    colour: ink.text,
                });
            }
        };
        let count = self.on.len().min(faces.names.len());
        if count == 0 {
            let f = faces.empty;
            tint(out, f, self.canvas.centred(f.1 as f32), EMPTY_Y, ink.dim);
            let h = faces.empty_help;
            tint(out, h, self.canvas.centred(h.1 as f32), HELP_Y, ink.dim);
            legend(&faces.legend_empty, out);
            return;
        }
        let first = first_shown(self.sel, count, self.first);
        let last = (first + CHEAT_ROWS).min(count);
        // Fewer rows than a full page sit centred, as on the settings pages.
        let page_top = crate::quick_menu::MENU_CENTRE - PITCH * (last - first) as f32 / 2.0;
        for (row, i) in (first..last).enumerate() {
            let top = page_top + PITCH * row as f32;
            let lit = i == self.sel;
            if lit {
                out.push(Draw::Rect {
                    x: 0.0,
                    y: top + BAR_INSET,
                    w: self.canvas.w,
                    h: PITCH - 2.0 * BAR_INSET,
                    colour: ink.bar,
                });
            }
            let colour = if lit { ink.on_bar } else { ink.dim };
            tint(out, faces.names[i], LABEL_X, top + LABEL_DROP, colour);
            let sw = faces.switches[usize::from(self.on[i])];
            let x = self.canvas.w - SWITCH_RIGHT - sw.1 as f32;
            tint(out, sw, x, top + SWITCH_DROP, colour);
        }
        // Small arrows where the list carries on.
        let rows_bottom = page_top + PITCH * (last - first) as f32;
        if first > 0 {
            let f = faces.more[0];
            tint(out, f, self.canvas.centred(f.1 as f32), page_top - MORE_GAP - f.2 as f32, ink.dim);
        }
        if last < count {
            let f = faces.more[1];
            tint(out, f, self.canvas.centred(f.1 as f32), rows_bottom + MORE_GAP, ink.dim);
        }
        if let Some(h) = faces.help {
            tint(out, h, self.canvas.centred(h.1 as f32), HELP_Y, ink.dim);
        }
        legend(&faces.legend, out);
    }
}

/// How wide a cheat's name may run before it shrinks, leaving room for its switch.
pub const NAME_MAX_W: f32 = 500.0;

/// The robber in white with its dark one-pixel rim, `h` pixels tall.
pub fn cheat_badge_face(h: u32) -> UndoFace {
    // Find the size whose glyph is `h` pixels tall.
    let mut px = h as f32;
    for _ in 0..12 {
        let f = icon_glyph_face(Icon::Cheats, px, [0xff, 0xff, 0xff]);
        if f.h == 0 {
            break;
        }
        let glyph_h = f.h.saturating_sub(2);
        if glyph_h == h {
            break;
        }
        px *= h as f32 / glyph_h.max(1) as f32;
    }
    let f = icon_glyph_face(Icon::Cheats, px, [0xff, 0xff, 0xff]);
    UndoFace {
        rgba: f.rgba,
        w: f.w,
        h: f.h,
    }
}

/// The robber in play: bottom left, mirroring the fast-forward badge in the top right.
pub fn draw_cheat_badge_in_game(canvas: Canvas, face: (TexId, u32, u32), out: &mut Vec<Draw>) {
    let _ = canvas;
    out.push(Draw::Tex {
        x: BADGE_GAME_X - 1.0,
        y: (OUT_H as f32 - BADGE_GAME_BOTTOM - face.2 as f32 + 1.0).round(),
        w: face.1 as f32,
        h: face.2 as f32,
        tex: face.0,
        alpha: 1.0,
    });
}

/// The robber in the menu's corner, right after the clock.
pub fn draw_cheat_badge_in_menu(x: f32, face: (TexId, u32, u32), out: &mut Vec<Draw>) {
    // On the clock's digits, which run from 14 to 25 in the 40 px plate, sitting a
    // pixel low so the round badge reads as level with them.
    let centre = 20.0;
    out.push(Draw::Tex {
        x: x.round(),
        y: (centre - face.2 as f32 / 2.0 + 0.5).round(),
        w: face.1 as f32,
        h: face.2 as f32,
        tex: face.0,
        alpha: 1.0,
    });
}
