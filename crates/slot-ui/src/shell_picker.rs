//! The shell picker: a shelf cart lifted onto the menu's ground, a row of colours under it and
//! the finishes under those; for a blank cart with no label to fetch, a second tab holds the
//! markers its title can be written in.
use crate::board::{board_in, lift_of, slide_of, Placed, SLIDE_SHARE};
use crate::draw::{Canvas, Draw, TexId, OUT_H, OUT_W};

const OUT_W_F: f32 = OUT_W as f32;
use crate::plate::{centred_hints, tint_mask, word_face, UndoFace, HINT_EDGE, HINT_H, LEGEND_GAP};
use crate::slot_chrome::ease;
use crate::theme::Ink;

const DISC: u32 = 24;
const RING: u32 = 36;
const STROKE: f32 = 2.0;
const SWATCH_PITCH: f32 = 34.0;
/// The most swatches the row shows at once, and how faint an end one is with more past it.
const SWATCHES_SHOWN: usize = 15;
const SWATCH_MORE: f32 = 0.35;

const CART_TOP: f32 = 30.0;
const CART_MAX_W: f32 = 250.0;
const CART_MAX_H: f32 = 192.0;
/// The Super Game Boy has only its model row, so its cart takes the room the others' rows use,
/// up to its size on the shelf.
const SGB_CART_MAX_W: f32 = 400.0;
const SGB_CART_MAX_H: f32 = 330.0;
/// How far under the Super Game Boy its model row stands.
const SGB_MODEL_GAP: f32 = 34.0;
const NAME_Y: f32 = 237.0;
const SWATCH_Y: f32 = 279.0;
const FINISH_Y: f32 = 313.0;
const FINISH_GAP: f32 = 34.0;
/// A cart's two shapes (SNES NTSC and PAL, Game Boy and Color), under the finishes.
const MODEL_Y: f32 = 347.0;
/// The tabs' names stand under the cart and the rows move down to make room.
const TAB_Y: f32 = 235.0;
const TAB_GAP: f32 = 34.0;
const TABBED_DROP: f32 = 34.0;
/// The studios' tabs, Cart and Slot, along the top; the cart moves down under them.
const STUDIO_TAB_Y: f32 = 12.0;
const STUDIO_DROP: f32 = 18.0;
/// The slot's studio: its name and swatches higher up, the legend over the slot.
const SLOT_ROWS_DY: f32 = -60.0;
const SLOT_LEGEND_Y: f32 = 372.0;
const SLOT_MODEL_GAP: f32 = 52.0;
const MARKER_PITCH: f32 = 40.0;
/// The cart's studio sets the cart at full size on the left, up to `SIDE_CART_W` by the room
/// over the legend, and its rows in a column on the right (Roy: the cart was too small).
const SIDE_EDGE: f32 = 16.0;
const SIDE_CART_W: f32 = 320.0;
const SIDE_GUTTER: f32 = 12.0;
const SIDE_TOP: f32 = 40.0;
const SIDE_TABS_DROP: f32 = 34.0;
/// X New Cart stands this far under a worn cart, in room kept for it.
const NEW_CART_ROOM: f32 = 40.0;
const NEW_CART_GAP: f32 = 10.0;
const SIDE_BOTTOM: f32 = 412.0;
/// The column's rows: the tabs, then each row this far under the last; rows too wide for the
/// column carry on on a line under, `SIDE_LINE` lower.
const SIDE_ROWS_TOP: f32 = 10.0;
const SIDE_STEP: f32 = 46.0;
const SIDE_LINE: f32 = 32.0;
/// The colours stand as a grid in the column, this many to a line.
pub const SWATCH_COLS: usize = 7;
const GRID_PITCH: f32 = 38.0;
/// The legend sits where every menu has it.
const LEGEND_Y: f32 = crate::quick_menu::LEGEND_Y;
/// How far the cart rises off the shelf while its neighbours part, before it lifts into place.
const HOP: f32 = 22.0;
/// On its way up the cart turns a little, as if lifted to be looked at.
const LIFT_TURN: f32 = 0.07;
/// A new look: the cart hops this high and rocks the way it was stepped, settling in POKE_S.
const POKE_HOP: f32 = 12.0;
const POKE_TURN: f32 = 0.06;
const POKE_S: f32 = 0.45;
/// Held up, it breathes like the cart in the middle of the shelf.
const SWAY: f32 = 0.016;
const SWAY_S: f32 = 3.4;
const BOB: f32 = 2.0;
/// The rows rise this far into place as the cart arrives, one after the other.
const RISE_IN: f32 = 26.0;
const STAGGER: f32 = 0.12;
/// The rows wait until the cart has nearly cleared them.
const ROWS_START: f32 = 0.3;
const BOX_PAD_X: f32 = 10.0;
const BOX_PAD_Y: f32 = 2.0;
/// The finish chosen while the row is elsewhere: a quiet box of the theme's ink, the ink on
/// it, so it reads on a light wallpaper as on a dark one.
const RESTING_BOX_ALPHA: f32 = 0.16;

fn resting_box(ink: &Ink) -> [f32; 4] {
    [ink.text[0], ink.text[1], ink.text[2], RESTING_BOX_ALPHA]
}

pub struct ShellPickerFaces {
    pub disc: TexId,
    pub ring: TexId,
    pub original: TexId,
    /// Original, Custom, then the palette's names.
    pub names: Vec<(TexId, u32)>,
    pub finishes: Vec<(TexId, u32)>,
    /// PAL, NTSC: an SNES cart's two shapes; then GB, GBC: a Game Boy cart's two; then the
    /// three Super Game Boys.
    pub models: Vec<(TexId, u32)>,
    /// The tabs' names: the colour's, the label's, the marker's, then the studios' Slot and
    /// Cart.
    pub tabs: [(TexId, u32); 6],
    /// The label tab while its versions are looked for, and when there are none.
    pub looking: (TexId, u32),
    pub none_found: (TexId, u32),
    /// The markers' names, in the order of their inks.
    pub inks: Vec<(TexId, u32)>,
    pub legend: Vec<(TexId, u32)>,
    /// Y taking the label off, shown while the cart wears one.
    pub remove: (TexId, u32),
    /// The legend while Y asks whether to take the label off.
    pub confirm: Vec<(TexId, u32)>,
    /// Y searching for the label by name, on the label's tab.
    pub search: (TexId, u32),
    /// The wear words: Dust, Wear, the four dust states, the four wear states, New Cart,
    /// the question and what the swap does, on two lines.
    pub wear: Vec<(TexId, u32)>,
    /// X New Cart under a worn cart, then the line asking X again to be sure.
    pub wear_legend: Vec<(TexId, u32)>,
}

/// A worn cart: its dust and wear (0 to 3 each), whether a new cart can be swapped in, and
/// whether that is being asked.
#[derive(Copy, Clone)]
pub struct WearView {
    pub dust: u8,
    pub wear: u8,
    pub new_cart: bool,
    pub asking: bool,
}

pub struct ShellPickerView<'a> {
    /// Seconds since the cart last changed its look and the way it was stepped (-1 or 1).
    pub poke: Option<(f32, f32)>,
    /// Seconds since the picker opened.
    pub clock: f32,
    /// How the cart was turned on the shelf when it was picked up.
    pub rest_turn: f32,
    pub faces: &'a ShellPickerFaces,
    pub ink: Ink,
    pub ground: [f32; 4],
    pub cart: Option<TexId>,
    /// A clear cart's back plastic and board, drawn under its face as the shelf draws them.
    pub behind: [Option<TexId>; 2],
    /// A GBA cart's board fills its board's box, not the cart's.
    pub gba: bool,
    pub cart_box: (u32, u32),
    /// Where the cart stands on the shelf; it rises from there as the picker opens.
    pub from: Placed,
    pub openness: f32,
    /// Each swatch's colour (None for the cart's own) and its name's place in the faces.
    pub swatches: &'a [(Option<[u8; 3]>, usize)],
    pub swatch: usize,
    pub finish: usize,
    pub finish_on: bool,
    /// The finishes offered, as places in the faces' finishes.
    pub finishes: Vec<usize>,
    /// For a cart with a choice of shapes: where they start in the faces' models, how many,
    /// and the one picked.
    pub model: Option<(usize, usize, usize)>,
    pub row: PickerRow,
    /// Where the cursor stands on the current row, in its order: the arrows move it, A
    /// chooses it (Roy 2026-11-07).
    pub cursor: usize,
    /// The open tab, when there are two.
    pub tabs: Vec<usize>,
    /// The open tab, by its name's place in the faces: colour, label, marker.
    pub tab: usize,
    pub label: LabelTab<'a>,
    pub marker: usize,
    pub inks: [[u8; 3]; 3],
    /// Y can take the cart's label off.
    pub remove: bool,
    /// Y asked: A takes the label off, B keeps it.
    pub confirming: bool,
    /// Y searches for the label by name.
    pub search: bool,
    /// The Wear tab, while it is open.
    pub wear: Option<WearView>,
    /// The Super Game Boy: no rows but its model's, and the cart drawn larger for it.
    pub sgb: bool,
    /// The studios' tabs along the top, Cart and Slot, when the slot is offered: whether the
    /// slot's is open.
    pub studio: Option<bool>,
    /// How far the cart has flown off for the slot's studio: 0 held up, 1 gone; a little past 0
    /// as it springs back.
    pub away: f32,
    /// The cart's drop shadow from the shelf: it keeps it in the studio (Roy 2026-11-12, st2).
    pub drop: Option<TexId>,
    /// A light ground: the shadow goes lighter, as on the shelf.
    pub light: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PickerRow {
    Tabs,
    Colour,
    Finish,
    Model,
    Label,
    Marker,
    Wear,
}

/// The label tab's versions of the label.
#[derive(Copy, Clone)]
pub enum LabelTab<'a> {
    Looking,
    Missing,
    /// Each version's thumbnail (texture, width, height) and caption, and the one shown.
    Found {
        thumbs: &'a [(TexId, u32, u32)],
        captions: &'a [(TexId, u32)],
        at: Option<usize>,
    },
}

/// A version's thumbnail is this tall, its top this far under the swatches' middle; the row of
/// them is spaced by `THUMB_GAP`.
const THUMB_H: f32 = 42.0;
const THUMB_DROP: f32 = -6.0;
const THUMB_GAP: f32 = 12.0;

/// A white disc, a white ring for the chosen swatch and the cart's-own swatch (a ring with a
/// slash), all drawn with `Draw::Tint`.
pub fn shell_picker_shapes() -> [UndoFace; 3] {
    [disc(DISC), ring(RING, STROKE), original(DISC)]
}

pub fn shell_picker_word(text: &str) -> UndoFace {
    tint_mask(word_face(text))
}

/// The studio's tabs at the size every tab row uses.
pub fn shell_picker_tab(text: &str) -> UndoFace {
    tint_mask(crate::buttons_help::tab_face_in(text, crate::plate::HINT_H))
}

fn shape(size: u32, cover: impl Fn(f32, f32) -> f32) -> UndoFace {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let c = size as f32 / 2.0;
    for y in 0..size {
        for x in 0..size {
            // Four samples a pixel smooth the edge.
            let mut a = 0.0;
            for (sx, sy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                a += cover(x as f32 + sx - c, y as f32 + sy - c);
            }
            let at = ((y * size + x) * 4) as usize;
            rgba[at..at + 4].copy_from_slice(&[255, 255, 255, (a / 4.0 * 255.0) as u8]);
        }
    }
    UndoFace {
        rgba,
        w: size,
        h: size,
    }
}

fn disc(size: u32) -> UndoFace {
    let r = size as f32 / 2.0;
    shape(size, |x, y| f32::from(x * x + y * y <= r * r))
}

fn ring(size: u32, stroke: f32) -> UndoFace {
    let r = size as f32 / 2.0;
    shape(size, |x, y| {
        let d = (x * x + y * y).sqrt();
        f32::from(d <= r && d >= r - stroke)
    })
}

fn original(size: u32) -> UndoFace {
    let r = size as f32 / 2.0;
    shape(size, |x, y| {
        let d = (x * x + y * y).sqrt();
        let edge = d <= r && d >= r - STROKE;
        // The slash runs from the lower left to the upper right, inside the ring.
        let slash = d < r - STROKE && ((x + y).abs() / std::f32::consts::SQRT_2) <= STROKE / 2.0;
        f32::from(edge || slash)
    })
}

fn tint(tex: TexId, x: f32, y: f32, w: f32, h: f32, colour: [f32; 4]) -> Draw {
    Draw::Tint {
        x,
        y,
        w,
        h,
        tex,
        colour,
    }
}

fn faded(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], c[3] * a]
}

fn seen(w: u32) -> f32 {
    w.saturating_sub(HINT_EDGE) as f32
}

/// How far the cart stands above its place on the shelf as the picker opens.
pub fn shell_picker_hop(openness: f32) -> f32 {
    HOP * slide_of(openness.clamp(0.0, 1.0))
}

pub fn draw_shell_picker(canvas: Canvas, v: &ShellPickerView, out: &mut Vec<Draw>) {
    let p = v.openness.clamp(0.0, 1.0);
    let open = lift_of(p);
    // The rows arrive one after another as the cart settles, each a beat behind the last.
    let raw = ((p - SLIDE_SHARE) / (1.0 - SLIDE_SHARE)).clamp(0.0, 1.0);
    let stage = |i: f32| {
        let start = ROWS_START + i * STAGGER;
        ease(((raw - start) / (1.0 - ROWS_START - 3.0 * STAGGER)).clamp(0.0, 1.0))
    };
    out.push(Draw::Rect {
        x: 0.0,
        y: 0.0,
        w: canvas.w,
        h: OUT_H as f32,
        colour: faded(v.ground, open),
    });

    let (cw, ch) = (v.cart_box.0 as f32, v.cart_box.1 as f32);
    let (max_w, max_h) = match v.sgb {
        true => (SGB_CART_MAX_W, SGB_CART_MAX_H),
        false => (CART_MAX_W, CART_MAX_H),
    };
    let mut scale = (max_w / cw).min(max_h / ch);
    if v.sgb {
        // Never larger than it stands on the shelf (Roy).
        scale = scale.min(v.from.w / cw);
    }
    let studio_drop = if v.studio.is_some() { STUDIO_DROP } else { 0.0 };
    if v.studio.is_some() {
        scale = scale.min((max_h - studio_drop) / ch);
    }
    let slot_mode = v.studio == Some(true);
    // The cart's studio: the cart full size on the left, the rows in a column on the right.
    let side = !v.sgb && !slot_mode;
    let shift = canvas.centred(OUT_W_F);
    // The tabs along the top, under the studios' when there are those; cart and column under.
    let side_tabs_y = STUDIO_TAB_Y + if v.studio.is_some() { SIDE_TABS_DROP } else { 0.0 };
    let area_top = side_tabs_y + SIDE_TOP;
    // The cart keeps its place on the left in the slot studio too: it flies up from there and
    // comes back down there, not from the middle (Roy 2026-11-06).
    let to = if !v.sgb {
        // Room kept under the cart for New Cart, so it never changes size.
        let bottom = SIDE_BOTTOM - NEW_CART_ROOM;
        let s = (SIDE_CART_W / cw).min((bottom - area_top) / ch).min(1.0);
        Placed {
            x: (shift + SIDE_EDGE + (SIDE_CART_W - cw * s) / 2.0).round(),
            y: (area_top + (bottom - area_top - ch * s) / 2.0).round(),
            w: cw * s,
            h: ch * s,
        }
    } else {
        Placed {
            x: canvas.centred(cw * scale),
            y: CART_TOP + studio_drop,
            w: cw * scale,
            h: ch * scale,
        }
    };
    let col_left = shift + SIDE_EDGE + SIDE_CART_W + SIDE_GUTTER;
    let col_w = OUT_W_F - SIDE_EDGE - (col_left - shift);
    let (cx, max_w) = match side {
        true => (col_left + col_w / 2.0, col_w),
        false => (canvas.w / 2.0, f32::INFINITY),
    };
    let centred = |w: f32| cx - w / 2.0;
    let from = Placed {
        y: v.from.y - shell_picker_hop(p),
        ..v.from
    };
    let at = |a: f32, b: f32| a + (b - a) * open;
    let cart = Placed {
        x: at(from.x, to.x),
        y: at(from.y, to.y),
        w: at(from.w, to.w),
        h: at(from.h, to.h),
    };
    // Lifted off the shelf it turns a little; held up it breathes; a new look makes it hop
    // and rock the way it was stepped.
    let tau = std::f32::consts::TAU;
    let calm = (v.clock - 0.6).clamp(0.0, 1.0) * open;
    let phase = tau * v.clock / SWAY_S;
    let mut turn = v.rest_turn * (1.0 - open)
        + LIFT_TURN * (std::f32::consts::PI * open).sin()
        + SWAY * phase.sin() * calm;
    let mut hop = BOB * (1.0 - (2.0 * phase).cos()) * 0.5 * calm;
    if let Some((s, dir)) = v.poke.filter(|(s, _)| *s < POKE_S) {
        let u = s / POKE_S;
        hop += POKE_HOP * (std::f32::consts::PI * u).sin() * (1.0 - u);
        turn += dir * POKE_TURN * (tau * u).sin() * (1.0 - u);
    }
    // Off to the slot's studio it flies up and out; back, it drops in and springs (Roy).
    let cart = Placed {
        y: cart.y - hop * open - v.away * (cart.y + cart.h + 24.0),
        ..cart
    };
    turn += 0.10 * v.away;
    // The shadow turns with the cart, as on the shelf, so its corners never stick out (Roy 14-nov).
    let cart_from = out.len();
    if let (Some(tex), Some(_)) = (v.drop, v.cart.filter(|_| v.away < 1.0)) {
        let k = cart.w / cw;
        let m = crate::shelf::DROP_MARGIN as f32 * k;
        let fade = if v.light { 0.55 } else { 1.0 };
        out.push(Draw::Tex {
            x: cart.x - m,
            y: cart.y - m + crate::shelf::DROP_FALL * k,
            w: cart.w + 2.0 * m,
            h: cart.h + 2.0 * m,
            tex,
            // It takes over from the shelf's shadow as that one fades, so there is only ever
            // one shadow, never two stacked (Roy 14-nov).
            alpha: crate::shelf::DROP_ALPHA
                * fade
                * (1.0 - v.away).clamp(0.0, 1.0)
                * slide_of(p),
        });
    }
    if let Some(tex) = v.cart.filter(|_| v.away < 1.0) {
        let inside = if v.gba { board_in(cart) } else { cart };
        for tex in v.behind.into_iter().flatten() {
            out.push(Draw::Tex {
                x: inside.x,
                y: inside.y,
                w: inside.w,
                h: inside.h,
                tex,
                alpha: 1.0,
            });
        }
        out.push(Draw::Tex {
            x: cart.x,
            y: cart.y,
            w: cart.w,
            h: cart.h,
            tex,
            alpha: 1.0,
        });
    }

    let cart_to = out.len();
    if turn.abs() > 1e-4 {
        for d in &mut out[cart_from..cart_to] {
            if let Draw::Tex { x, y, w, h, tex, alpha } = *d {
                *d = Draw::Turned { x, y, w, h, tex, alpha, turn };
            }
        }
    }
    let ink = |c: [f32; 4], k: f32| faded(c, k);
    let rise = |y: f32, k: f32| y + RISE_IN * (1.0 - k);
    let focus = |row: PickerRow| if v.row == row { v.ink.text } else { v.ink.dim };
    // The cursor on a row, if it is the current one.
    let hover = |row: PickerRow| (v.row == row).then_some(v.cursor);
    // The ring of a swatch: the cursor's bright, the chosen one's faint while the cursor is elsewhere.
    let ring_of = |row: PickerRow, i: usize, chosen: bool| match hover(row) {
        Some(c) if c == i => Some(v.ink.text),
        _ if chosen => Some(v.ink.dim),
        _ => None,
    };
    let word = |(tex, w): (TexId, u32), x: f32, y: f32, colour: [f32; 4]| {
        tint(tex, x.round(), y, w as f32, HINT_H as f32, colour)
    };
    // A row of words, the chosen one boxed: the bar's own colours while the row is current.
    let tabs_row = |faces: &[(TexId, u32)], chosen: Option<usize>, current: bool, y: f32, gap: f32, k: f32, column: bool, out: &mut Vec<Draw>| {
        let y = rise(y, k);
        let widths: Vec<f32> = faces.iter().map(|&(_, w)| seen(w)).collect();
        let total = widths.iter().sum::<f32>() + gap * (widths.len().saturating_sub(1)) as f32;
        let mut x = if column { centred(total) } else { canvas.centred(total) };
        for (i, &face) in faces.iter().enumerate() {
            let w = widths[i];
            let colour = if chosen == Some(i) {
                out.push(Draw::Rect {
                    x: (x - BOX_PAD_X).round(),
                    y: y - BOX_PAD_Y,
                    w: (w + 2.0 * BOX_PAD_X).round(),
                    h: HINT_H as f32 + 2.0 * BOX_PAD_Y,
                    colour: ink(if current { v.ink.bar } else { resting_box(&v.ink) }, k),
                });
                if current { v.ink.on_bar } else { v.ink.text }
            } else {
                v.ink.dim
            };
            out.push(word(face, x, y, ink(colour, k)));
            x += w + gap;
        }
    };
    let studio_row = |faces: &[(TexId, u32)], chosen: Option<usize>, current: bool, y: f32, gap: f32, k: f32, out: &mut Vec<Draw>| {
        tabs_row(faces, chosen, current, y, gap, k, false, out)
    };
    // The cart's own tabs stand over the column of colours, not under the studios' (Roy 2026-11-07).
    let column_row = |faces: &[(TexId, u32)], chosen: Option<usize>, current: bool, y: f32, gap: f32, k: f32, out: &mut Vec<Draw>| {
        tabs_row(faces, chosen, current, y, gap, k, true, out)
    };
    let boxed_row = |faces: &[(TexId, u32)],
                     chosen: Option<usize>,
                     cursor: Option<usize>,
                     y: f32,
                     gap: f32,
                     k: f32,
                     out: &mut Vec<Draw>| {
        let widths: Vec<f32> = faces.iter().map(|&(_, w)| seen(w)).collect();
        // In the column more than two words stand two to a line, each line centred under the
        // colours (Roy 2026-11-06: the glitters beside Solid and Clear, all centred).
        let paired = side && faces.len() > 2;
        let lines = match paired {
            true => (0..faces.len()).step_by(2).map(|i| i..(i + 2).min(faces.len())).collect(),
            false => wrap(&widths, gap, max_w),
        };
        for (line, range) in lines.iter().enumerate() {
        let y = rise(y + SIDE_LINE * line as f32, k);
        let total = widths[range.clone()].iter().sum::<f32>() + gap * (range.len().saturating_sub(1)) as f32;
        let mut x = centred(total);

        for i in range.clone() {
            let (face, w) = (faces[i], widths[i]);
            // The cursor's box black, the chosen one's resting grey (Roy 2026-11-07).
            let on = cursor == Some(i);
            let colour = match on || chosen == Some(i) {
                true => {
                    out.push(Draw::Rect {
                        x: (x - BOX_PAD_X).round(),
                        y: y - BOX_PAD_Y,
                        w: (w + 2.0 * BOX_PAD_X).round(),
                        h: HINT_H as f32 + 2.0 * BOX_PAD_Y,
                        colour: ink(if on { v.ink.bar } else { resting_box(&v.ink) }, k),
                    });
                    if on {
                        v.ink.on_bar
                    } else {
                        v.ink.text
                    }
                }
                _ => v.ink.dim,
            };
            // Off for the cart's own shell: the finish waits, faint, for a colour.
            let alpha = if chosen.is_none() { 0.35 } else { 1.0 };
            out.push(word(face, x, y, ink(faded(colour, alpha), k)));
            x += w + gap;
        }
        }
        lines.len()
    };

    if let Some(slot) = v.studio {
        studio_row(
            &[v.faces.tabs[4], v.faces.tabs[3]],
            Some(usize::from(slot)),
            true,
            STUDIO_TAB_Y,
            TAB_GAP,
            stage(0.0),
            out,
        );
    }
    let drop = match slot_mode {
        true => SLOT_ROWS_DY,
        false => TABBED_DROP,
    };
    // Where each row stands: under the cart, or down the column beside it.
    let tab_y = if side { side_tabs_y } else { TAB_Y };
    // In the column the cart's shape comes first, over the colours (Roy).
    let model_first = side && v.tab == 0 && v.model.is_some();
    let model_y = area_top + SIDE_ROWS_TOP;
    let name_y = match side {
        true if model_first => model_y + SIDE_STEP,
        true => model_y,
        false => NAME_Y + drop,
    };
    let swatch_y = if side { name_y + SIDE_STEP } else { SWATCH_Y + drop };
    let grid_lines = v.swatches.len().div_ceil(SWATCH_COLS).max(1) as f32;
    let finish_y = match side {
        true if v.tab == 0 => swatch_y + GRID_PITCH * (grid_lines - 1.0) + SIDE_STEP - HINT_H as f32 / 2.0,
        true => swatch_y + SIDE_STEP - HINT_H as f32 / 2.0,
        false => FINISH_Y + drop,
    };
    if !slot_mode {
        let tab_faces: Vec<(TexId, u32)> = v.tabs.iter().map(|&t| v.faces.tabs[t]).collect();
        let row = if side { &column_row as &dyn Fn(&[(TexId, u32)], Option<usize>, bool, f32, f32, f32, &mut Vec<Draw>) } else { &|f: &[(TexId, u32)], c: Option<usize>, o: bool, y: f32, g: f32, k: f32, out: &mut Vec<Draw>| { boxed_row(f, c, if o { c } else { None }, y, g, k, out); } };
        row(
            &tab_faces,
            v.tabs.iter().position(|&t| t == v.tab),
            // In the column the tabs are a row: the open one black while the row is current,
            // the grey resting box else, as every row of choices (Roy 2026-11-07).
            !side || v.row == PickerRow::Tabs,
            tab_y,
            TAB_GAP,
            stage(0.0),
            out,
        );
    }

    if v.tab == 1 && !slot_mode {
        // The label's tab: where the version shown comes from, over a row of them all.
        let caption = match v.label {
            LabelTab::Looking => Some(v.faces.looking),
            LabelTab::Missing => Some(v.faces.none_found),
            LabelTab::Found { captions, at, .. } => {
                hover(PickerRow::Label).or(at).and_then(|i| captions.get(i).copied())
            }
        };
        if let Some(face) = caption {
            out.push(word(
                face,
                centred(seen(face.1)),
                rise(name_y, stage(0.0)),
                ink(focus(PickerRow::Label), stage(0.0)),
            ));
        }
        if let LabelTab::Found { thumbs, at, .. } = v.label {
            let k = stage(1.0);
            let sizes: Vec<(f32, f32)> = thumbs
                .iter()
                .map(|&(_, w, h)| (THUMB_H * w as f32 / h.max(1) as f32, THUMB_H))
                .collect();
            let total = sizes.iter().map(|s| s.0).sum::<f32>()
                + THUMB_GAP * sizes.len().saturating_sub(1) as f32;
            let widths: Vec<f32> = sizes.iter().map(|s| s.0).collect();
            let lines = wrap(&widths, THUMB_GAP, max_w);
            let line_of = |i: usize| lines.iter().position(|r| r.contains(&i)).unwrap_or(0);
            let mut x = centred(total).round();
            let y0 = rise(swatch_y + THUMB_DROP, k).round();
            let mut y = y0;
            for (i, (&(tex, _, _), &(w, h))) in thumbs.iter().zip(&sizes).enumerate() {
                if lines.len() > 1 && lines[line_of(i)].start == i {
                    let r = lines[line_of(i)].clone();
                    let t = widths[r.clone()].iter().sum::<f32>() + THUMB_GAP * (r.len() - 1) as f32;
                    x = centred(t).round();
                    y = y0 + (THUMB_H + THUMB_GAP) * line_of(i) as f32;
                }
                if let Some(ring) = ring_of(PickerRow::Label, i, at == Some(i)) {
                    let c = ink(ring, k);
                    let (fx, fy, fw, fh, t) = (x - 4.0, y - 4.0, w + 8.0, h + 8.0, 2.0);
                    for (rx, ry, rw, rh) in [
                        (fx, fy, fw, t),
                        (fx, fy + fh - t, fw, t),
                        (fx, fy, t, fh),
                        (fx + fw - t, fy, t, fh),
                    ] {
                        out.push(Draw::Rect {
                            x: rx,
                            y: ry,
                            w: rw,
                            h: rh,
                            colour: c,
                        });
                    }
                }
                out.push(Draw::Tex {
                    x,
                    y,
                    w: w.round(),
                    h,
                    tex,
                    alpha: k,
                });
                x += (w + THUMB_GAP).round();
            }
        }
    } else if v.tab == 2 && !slot_mode {
        // The marker's tab: the ink's name over a row of inks.
        if let Some(&face) = v.faces.inks.get(hover(PickerRow::Marker).unwrap_or(v.marker)) {
            out.push(word(
                face,
                centred(seen(face.1)),
                rise(name_y, stage(0.0)),
                ink(focus(PickerRow::Marker), stage(0.0)),
            ));
        }
        let n = v.inks.len() as f32;
        let first = cx - MARKER_PITCH * (n - 1.0) / 2.0;
        for (i, &c) in v.inks.iter().enumerate() {
            swatch(
                v,
                first + MARKER_PITCH * i as f32,
                rise(swatch_y, stage(1.0)),
                Some(c),
                ring_of(PickerRow::Marker, i, i == v.marker),
                &|c| ink(c, stage(1.0)),
                out,
            );
        }
    } else {
        // The name of the colour at the cursor, over the row.
        let at_colour = hover(PickerRow::Colour).unwrap_or(v.swatch);
        if let Some(&(_, name)) = v.swatches.get(at_colour) {
            if let Some(&face) = v.faces.names.get(name) {
                out.push(word(
                    face,
                    centred(seen(face.1)),
                    rise(name_y, stage(0.0)),
                    ink(focus(PickerRow::Colour), stage(0.0)),
                ));
            }
        }
        // More colours than fit slide along under a window kept around the cursor; the end
        // swatches fade where the row goes on past them.
        let all = v.swatches.len();
        if side {
            // All the colours at once, a grid of them.
            let first = cx - GRID_PITCH * (SWATCH_COLS as f32 - 1.0) / 2.0;
            for (i, s) in v.swatches.iter().enumerate() {
                let (line, col) = (i / SWATCH_COLS, i % SWATCH_COLS);
                swatch(
                    v,
                    first + GRID_PITCH * col as f32,
                    rise(swatch_y + GRID_PITCH * line as f32, stage(1.0)),
                    s.0,
                    ring_of(PickerRow::Colour, i, i == v.swatch),
                    &|c| ink(c, stage(1.0)),
                    out,
                );
            }
        }
        let shown = if side { 0 } else { all.min(SWATCHES_SHOWN) };
        let from = at_colour.saturating_sub(shown / 2).min(all - shown);
        let first = canvas.w / 2.0 - SWATCH_PITCH * (shown as f32 - 1.0) / 2.0;
        for (at, i) in (from..from + shown).enumerate() {
            let more = (at == 0 && from > 0) || (at == shown - 1 && from + shown < all);
            let k = stage(1.0) * if more { SWATCH_MORE } else { 1.0 };
            swatch(
                v,
                first + SWATCH_PITCH * at as f32,
                rise(swatch_y, stage(1.0)),
                v.swatches[i].0,
                ring_of(PickerRow::Colour, i, i == v.swatch),
                &|c| ink(c, k),
                out,
            );
        }
        let finishes: Vec<(TexId, u32)> = v.finishes.iter().map(|&f| v.faces.finishes[f]).collect();
        boxed_row(
            &finishes,
            v.finish_on.then_some(v.finish),
            hover(PickerRow::Finish),
            finish_y,
            FINISH_GAP,
            stage(2.0),
            out,
        );
        if let Some((first, count, model)) = v.model {
            boxed_row(
                &v.faces.models[first..first + count],
                Some(model),
                hover(PickerRow::Model),
                match (v.sgb, slot_mode) {
                    (true, _) => (to.y + to.h + SGB_MODEL_GAP).round(),
                    // the slot's models stand over its colour's name, as tabs
                    (_, true) => NAME_Y + drop - SLOT_MODEL_GAP,
                    _ if side => model_y,
                    _ => MODEL_Y + drop,
                },
                FINISH_GAP,
                stage(2.5),
                out,
            );
        }
    }

    let legend = if v.confirming {
        v.faces.confirm.clone()
    } else {
        // A Select, then Y, B Back, L1 R1 Tab last (Roy 2026-11-14).
        let faces = &v.faces.legend;
        let mut legend = vec![faces[1]];
        if v.remove {
            legend.push(v.faces.remove);
        } else if v.search {
            legend.push(v.faces.search);
        }
        legend.push(faces[0]);
        // L1 R1 only turn Cart and Slot: without them, no L1 R1 Tab (Roy, sg2 and ui8).
        if !(v.sgb || v.studio.is_none()) {
            legend.push(faces[2]);
        }
        legend
    };
    // A worn cart: X New Cart under it (Roy); pressed, the line asks X again to be sure.
    let new_cart = v.wear.filter(|w| side && (w.new_cart || w.asking));
    if let Some(&(tex, w)) = new_cart.and_then(|w| v.faces.wear_legend.get(usize::from(w.asking))) {
        let k = stage(3.0) * (1.0 - v.away.clamp(0.0, 1.0));
        out.push(tint(
            tex,
            (to.x + (to.w - seen(w)) / 2.0).round(),
            rise(to.y + to.h + NEW_CART_GAP, stage(3.0)).round(),
            w as f32,
            HINT_H as f32,
            ink(v.ink.text, k),
        ));
    }
    for (tex, w, x) in centred_hints(canvas, &legend, LEGEND_GAP) {
        let y = rise(LEGEND_Y + (SLOT_LEGEND_Y - LEGEND_Y) * v.away.clamp(0.0, 1.0), stage(3.0));
        crate::plate::shaded_hint(out, tex, x, y, w as f32, HINT_H as f32, ink(v.ink.text, stage(3.0)), v.ink.wash);
    }
    // The cart goes over the rows, so they slip out from under it as it moves.
    let cart: Vec<Draw> = out.drain(cart_from..cart_to).collect();
    out.extend(cart);
}

/// The label versions' thumbnails split into their lines in the studio's column, as drawn: up
/// and down step through them as through the colours (Roy 2026-11-14).
pub fn label_lines(sizes: &[(u32, u32)]) -> Vec<std::ops::Range<usize>> {
    let widths: Vec<f32> = sizes
        .iter()
        .map(|&(w, h)| THUMB_H * w as f32 / h.max(1) as f32)
        .collect();
    let col_w = OUT_W_F - SIDE_EDGE - (SIDE_EDGE + SIDE_CART_W + SIDE_GUTTER);
    wrap(&widths, THUMB_GAP, col_w)
}

/// The words of a row split into lines no wider than `max`, each as the range of its words.
fn wrap(widths: &[f32], gap: f32, max: f32) -> Vec<std::ops::Range<usize>> {
    let mut lines = vec![];
    let (mut start, mut run) = (0, 0.0);
    for (i, &w) in widths.iter().enumerate() {
        if i > start && run + gap + w > max {
            lines.push(start..i);
            (start, run) = (i, w);
        } else {
            run += if i > start { gap + w } else { w };
        }
    }
    if start < widths.len() || lines.is_empty() {
        lines.push(start..widths.len());
    }
    lines
}

/// One disc on a row, with a faint rim so the darkest still stands off the ground, and the
/// ring around the chosen one.
#[allow(clippy::too_many_arguments)]
fn swatch(
    v: &ShellPickerView,
    cx: f32,
    cy: f32,
    colour: Option<[u8; 3]>,
    ring: Option<[f32; 4]>,
    ink: &dyn Fn([f32; 4]) -> [f32; 4],
    out: &mut Vec<Draw>,
) {
    let d = DISC as f32;
    let rim = d + 2.0;
    out.push(tint(
        v.faces.disc,
        cx - rim / 2.0,
        cy - rim / 2.0,
        rim,
        rim,
        ink(faded(v.ink.dim, 0.45)),
    ));
    match colour {
        Some(c) => out.push(tint(
            v.faces.disc,
            cx - d / 2.0,
            cy - d / 2.0,
            d,
            d,
            ink([
                c[0] as f32 / 255.0,
                c[1] as f32 / 255.0,
                c[2] as f32 / 255.0,
                1.0,
            ]),
        )),
        None => out.push(tint(
            v.faces.original,
            cx - d / 2.0,
            cy - d / 2.0,
            d,
            d,
            ink(v.ink.text),
        )),
    }
    if let Some(ring) = ring {
        let r = RING as f32;
        out.push(tint(
            v.faces.ring,
            cx - r / 2.0,
            cy - r / 2.0,
            r,
            r,
            ink(ring),
        ));
    }
}
