//! The shell picker: a shelf cart lifted onto the menu's ground, a row of colours under it,
//! the finishes under those, and a line to fetch its label when it has none, or when there is
//! none to fetch, the markers its title can be written in.
use crate::board::{board_in, Placed};
use crate::draw::{Canvas, Draw, TexId, OUT_H, OUT_W};
use crate::plate::{centred_hints, tint_mask, word_face, UndoFace, HINT_EDGE, HINT_H, LEGEND_GAP};
use crate::slot_chrome::ease;
use crate::theme::Ink;

const DISC: u32 = 24;
const RING: u32 = 36;
const STROKE: f32 = 2.0;
const SWATCH_PITCH: f32 = 34.0;

const CART_TOP: f32 = 36.0;
const CART_MAX_W: f32 = 250.0;
const CART_MAX_H: f32 = 206.0;
const NAME_Y: f32 = 262.0;
const SWATCH_Y: f32 = 304.0;
const FINISH_Y: f32 = 338.0;
const FINISH_GAP: f32 = 34.0;
const DOWNLOAD_Y: f32 = 376.0;
const MARKER_DISC: f32 = 16.0;
const MARKER_RING: f32 = 26.0;
const MARKER_PITCH: f32 = 28.0;
const MARKER_GAP: f32 = 20.0;
const LEGEND_Y: f32 = 430.0;
const BOX_PAD_X: f32 = 10.0;
const BOX_PAD_Y: f32 = 2.0;
/// The finish chosen while the row is elsewhere: a quiet box, the ink on it.
const RESTING_BOX: [f32; 4] = [1.0, 1.0, 1.0, 0.16];

pub struct ShellPickerFaces {
    pub disc: TexId,
    pub ring: TexId,
    pub original: TexId,
    /// Original, Custom, then the palette's names.
    pub names: Vec<(TexId, u32)>,
    pub finishes: Vec<(TexId, u32)>,
    /// The fetch line idle, then at work.
    pub download: [(TexId, u32); 2],
    pub marker: (TexId, u32),
    pub legend: Vec<(TexId, u32)>,
}

pub struct ShellPickerView<'a> {
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
    /// 0 colours, 1 finish, 2 the fetch line or the markers.
    pub row: usize,
    /// None when the cart has a label; Some(at work) otherwise.
    pub download: Option<bool>,
    /// In the fetch line's place for a cart with no label to fetch: the marker chosen and
    /// the inks offered.
    pub markers: Option<(usize, [[u8; 3]; 3])>,
}

/// A white disc, a white ring for the chosen swatch and the cart's-own swatch (a ring with a
/// slash), all drawn with `Draw::Tint`.
pub fn shell_picker_shapes() -> [UndoFace; 3] {
    [disc(DISC), ring(RING, STROKE), original(DISC)]
}

pub fn shell_picker_word(text: &str) -> UndoFace {
    tint_mask(word_face(text))
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

pub fn draw_shell_picker(canvas: Canvas, v: &ShellPickerView, out: &mut Vec<Draw>) {
    let open = ease(v.openness.clamp(0.0, 1.0));
    out.push(Draw::Rect {
        x: 0.0,
        y: 0.0,
        w: OUT_W as f32,
        h: OUT_H as f32,
        colour: faded(v.ground, open),
    });

    let (cw, ch) = (v.cart_box.0 as f32, v.cart_box.1 as f32);
    let scale = (CART_MAX_W / cw).min(CART_MAX_H / ch);
    let to = Placed {
        x: canvas.centred(cw * scale),
        y: CART_TOP,
        w: cw * scale,
        h: ch * scale,
    };
    let at = |a: f32, b: f32| a + (b - a) * open;
    let cart = Placed {
        x: at(v.from.x, to.x),
        y: at(v.from.y, to.y),
        w: at(v.from.w, to.w),
        h: at(v.from.h, to.h),
    };
    if let Some(tex) = v.cart {
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

    let ink = |c: [f32; 4]| faded(c, open);
    let focus = |row: usize| if v.row == row { v.ink.text } else { v.ink.dim };
    let word = |(tex, w): (TexId, u32), x: f32, y: f32, colour: [f32; 4]| {
        tint(tex, x.round(), y, w as f32, HINT_H as f32, colour)
    };

    // The name of the colour at the cursor, over the row.
    if let Some(&(_, name)) = v.swatches.get(v.swatch) {
        if let Some(&face) = v.faces.names.get(name) {
            out.push(word(
                face,
                canvas.centred(seen(face.1)),
                NAME_Y,
                ink(focus(0)),
            ));
        }
    }

    let n = v.swatches.len() as f32;
    let first = canvas.w / 2.0 - SWATCH_PITCH * (n - 1.0) / 2.0;
    for (i, &(colour, _)) in v.swatches.iter().enumerate() {
        let cx = first + SWATCH_PITCH * i as f32;
        let d = DISC as f32;
        // A faint rim, so the darkest swatch still stands off the ground.
        let rim = d + 2.0;
        out.push(tint(
            v.faces.disc,
            cx - rim / 2.0,
            SWATCH_Y - rim / 2.0,
            rim,
            rim,
            ink(faded(v.ink.dim, 0.45)),
        ));
        match colour {
            Some(c) => out.push(tint(
                v.faces.disc,
                cx - d / 2.0,
                SWATCH_Y - d / 2.0,
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
                SWATCH_Y - d / 2.0,
                d,
                d,
                ink(v.ink.text),
            )),
        }
        if i == v.swatch {
            let r = RING as f32;
            out.push(tint(
                v.faces.ring,
                cx - r / 2.0,
                SWATCH_Y - r / 2.0,
                r,
                r,
                ink(focus(0)),
            ));
        }
    }

    // The finishes, the chosen one boxed: the bar's own colours while the row is current.
    let widths: Vec<f32> = v.faces.finishes.iter().map(|&(_, w)| seen(w)).collect();
    let total = widths.iter().sum::<f32>() + FINISH_GAP * (widths.len().saturating_sub(1)) as f32;
    let mut x = canvas.centred(total);
    for (i, &face) in v.faces.finishes.iter().enumerate() {
        let w = widths[i];
        let chosen = i == v.finish && v.finish_on;
        let colour = match (chosen, v.row == 1) {
            (true, true) => {
                out.push(Draw::Rect {
                    x: (x - BOX_PAD_X).round(),
                    y: FINISH_Y - BOX_PAD_Y,
                    w: (w + 2.0 * BOX_PAD_X).round(),
                    h: HINT_H as f32 + 2.0 * BOX_PAD_Y,
                    colour: ink(v.ink.bar),
                });
                v.ink.on_bar
            }
            (true, false) => {
                out.push(Draw::Rect {
                    x: (x - BOX_PAD_X).round(),
                    y: FINISH_Y - BOX_PAD_Y,
                    w: (w + 2.0 * BOX_PAD_X).round(),
                    h: HINT_H as f32 + 2.0 * BOX_PAD_Y,
                    colour: ink(RESTING_BOX),
                });
                v.ink.text
            }
            _ => v.ink.dim,
        };
        // Off for the cart's own shell: the finish waits, faint, for a colour.
        let alpha = if v.finish_on { 1.0 } else { 0.35 };
        out.push(word(face, x, FINISH_Y, ink(faded(colour, alpha))));
        x += w + FINISH_GAP;
    }

    if let Some((chosen, inks)) = v.markers {
        let face = v.faces.marker;
        let w = seen(face.1);
        let n = inks.len() as f32;
        let total = w + MARKER_GAP + MARKER_PITCH * (n - 1.0) + MARKER_DISC;
        let x = canvas.centred(total);
        out.push(word(face, x, DOWNLOAD_Y, ink(focus(2))));
        let cy = DOWNLOAD_Y + HINT_H as f32 / 2.0;
        let first = x + w + MARKER_GAP + MARKER_DISC / 2.0;
        for (i, c) in inks.iter().enumerate() {
            let cx = first + MARKER_PITCH * i as f32;
            let rim = MARKER_DISC + 2.0;
            out.push(tint(
                v.faces.disc,
                cx - rim / 2.0,
                cy - rim / 2.0,
                rim,
                rim,
                ink(faded(v.ink.dim, 0.45)),
            ));
            out.push(tint(
                v.faces.disc,
                cx - MARKER_DISC / 2.0,
                cy - MARKER_DISC / 2.0,
                MARKER_DISC,
                MARKER_DISC,
                ink([
                    c[0] as f32 / 255.0,
                    c[1] as f32 / 255.0,
                    c[2] as f32 / 255.0,
                    1.0,
                ]),
            ));
            if i == chosen {
                out.push(tint(
                    v.faces.ring,
                    cx - MARKER_RING / 2.0,
                    cy - MARKER_RING / 2.0,
                    MARKER_RING,
                    MARKER_RING,
                    ink(focus(2)),
                ));
            }
        }
    } else if let Some(working) = v.download {
        let face = v.faces.download[usize::from(working)];
        let w = seen(face.1);
        let x = canvas.centred(w);
        let colour = if v.row == 2 {
            out.push(Draw::Rect {
                x: (x - BOX_PAD_X).round(),
                y: DOWNLOAD_Y - BOX_PAD_Y,
                w: (w + 2.0 * BOX_PAD_X).round(),
                h: HINT_H as f32 + 2.0 * BOX_PAD_Y,
                colour: ink(v.ink.bar),
            });
            v.ink.on_bar
        } else {
            v.ink.dim
        };
        out.push(word(face, x, DOWNLOAD_Y, ink(colour)));
    }

    for (tex, w, x) in centred_hints(canvas, &v.faces.legend, LEGEND_GAP) {
        out.push(tint(
            tex,
            x,
            LEGEND_Y,
            w as f32,
            HINT_H as f32,
            ink(v.ink.text),
        ));
    }
}
