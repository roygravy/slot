//! The shell picker: a shelf cart lifted onto the menu's ground, a row of colours under it and
//! the finishes under those; for a blank cart with no label to fetch, a second tab holds the
//! markers its title can be written in.
use crate::board::{board_in, lift_of, slide_of, Placed, SLIDE_SHARE};
use crate::draw::{Canvas, Draw, TexId, OUT_H, OUT_W};
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
const CART_MAX_H: f32 = 200.0;
/// The Super Game Boy has only its model row, so its cart takes the room the others' rows use,
/// up to its size on the shelf.
const SGB_CART_MAX_W: f32 = 400.0;
const SGB_CART_MAX_H: f32 = 330.0;
/// How far under the Super Game Boy its model row stands.
const SGB_MODEL_GAP: f32 = 34.0;
const NAME_Y: f32 = 248.0;
const SWATCH_Y: f32 = 290.0;
const FINISH_Y: f32 = 324.0;
const FINISH_GAP: f32 = 34.0;
/// A cart's two shapes (SNES NTSC and PAL, Game Boy and Color), under the finishes.
const MODEL_Y: f32 = 358.0;
/// The tabs' names stand under the cart and the rows move down to make room.
const TAB_Y: f32 = 246.0;
const TAB_GAP: f32 = 34.0;
const TABBED_DROP: f32 = 34.0;
const MARKER_PITCH: f32 = 40.0;
const LEGEND_Y: f32 = 438.0;
/// How far the cart rises off the shelf while its neighbours part, before it lifts into place.
const HOP: f32 = 22.0;
/// The rows rise this far into place as the cart arrives, one after the other.
const RISE_IN: f32 = 26.0;
const STAGGER: f32 = 0.12;
/// The rows wait until the cart has nearly cleared them.
const ROWS_START: f32 = 0.3;
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
    /// PAL, NTSC: an SNES cart's two shapes; then GB, GBC: a Game Boy cart's two; then the
    /// three Super Game Boys.
    pub models: Vec<(TexId, u32)>,
    /// The tabs' names: the colour's, the label's, then the marker's.
    pub tabs: [(TexId, u32); 3],
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
    /// The finishes offered, as places in the faces' finishes.
    pub finishes: Vec<usize>,
    /// For a cart with a choice of shapes: where they start in the faces' models, how many,
    /// and the one picked.
    pub model: Option<(usize, usize, usize)>,
    pub row: PickerRow,
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
    /// The Super Game Boy: no rows but its model's, and the cart drawn larger for it.
    pub sgb: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PickerRow {
    Tabs,
    Colour,
    Finish,
    Model,
    Label,
    Marker,
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
        w: OUT_W as f32,
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
    let to = Placed {
        x: canvas.centred(cw * scale),
        y: CART_TOP,
        w: cw * scale,
        h: ch * scale,
    };
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
    let cart_from = out.len();
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

    let cart_to = out.len();
    let ink = |c: [f32; 4], k: f32| faded(c, k);
    let rise = |y: f32, k: f32| y + RISE_IN * (1.0 - k);
    let focus = |row: PickerRow| if v.row == row { v.ink.text } else { v.ink.dim };
    let word = |(tex, w): (TexId, u32), x: f32, y: f32, colour: [f32; 4]| {
        tint(tex, x.round(), y, w as f32, HINT_H as f32, colour)
    };
    // A row of words, the chosen one boxed: the bar's own colours while the row is current.
    let boxed_row = |faces: &[(TexId, u32)],
                     chosen: Option<usize>,
                     current: bool,
                     y: f32,
                     gap: f32,
                     k: f32,
                     out: &mut Vec<Draw>| {
        let y = rise(y, k);
        let widths: Vec<f32> = faces.iter().map(|&(_, w)| seen(w)).collect();
        let total = widths.iter().sum::<f32>() + gap * (widths.len().saturating_sub(1)) as f32;
        let mut x = canvas.centred(total);
        for (i, &face) in faces.iter().enumerate() {
            let w = widths[i];
            let colour = match (chosen == Some(i), current) {
                (true, on) => {
                    out.push(Draw::Rect {
                        x: (x - BOX_PAD_X).round(),
                        y: y - BOX_PAD_Y,
                        w: (w + 2.0 * BOX_PAD_X).round(),
                        h: HINT_H as f32 + 2.0 * BOX_PAD_Y,
                        colour: ink(if on { v.ink.bar } else { RESTING_BOX }, k),
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
    };

    let drop = TABBED_DROP;
    let tab_faces: Vec<(TexId, u32)> = v.tabs.iter().map(|&t| v.faces.tabs[t]).collect();
    boxed_row(
        &tab_faces,
        v.tabs.iter().position(|&t| t == v.tab),
        v.row == PickerRow::Tabs,
        TAB_Y,
        TAB_GAP,
        stage(0.0),
        out,
    );

    if v.tab == 1 {
        // The label's tab: where the version shown comes from, over a row of them all.
        let caption = match v.label {
            LabelTab::Looking => Some(v.faces.looking),
            LabelTab::Missing => Some(v.faces.none_found),
            LabelTab::Found { captions, at, .. } => at.and_then(|i| captions.get(i).copied()),
        };
        if let Some(face) = caption {
            out.push(word(
                face,
                canvas.centred(seen(face.1)),
                rise(NAME_Y + drop, stage(0.0)),
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
            let mut x = canvas.centred(total).round();
            let y = rise(SWATCH_Y + drop + THUMB_DROP, k).round();
            for (i, (&(tex, _, _), &(w, h))) in thumbs.iter().zip(&sizes).enumerate() {
                if at == Some(i) {
                    let c = ink(focus(PickerRow::Label), k);
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
    } else if v.tab == 2 {
        // The marker's tab: the ink's name over a row of inks.
        if let Some(&face) = v.faces.inks.get(v.marker) {
            out.push(word(
                face,
                canvas.centred(seen(face.1)),
                rise(NAME_Y + drop, stage(0.0)),
                ink(focus(PickerRow::Marker), stage(0.0)),
            ));
        }
        let n = v.inks.len() as f32;
        let first = canvas.w / 2.0 - MARKER_PITCH * (n - 1.0) / 2.0;
        for (i, &c) in v.inks.iter().enumerate() {
            swatch(
                v,
                first + MARKER_PITCH * i as f32,
                rise(SWATCH_Y + drop, stage(1.0)),
                Some(c),
                i == v.marker,
                focus(PickerRow::Marker),
                &|c| ink(c, stage(1.0)),
                out,
            );
        }
    } else {
        // The name of the colour at the cursor, over the row.
        if let Some(&(_, name)) = v.swatches.get(v.swatch) {
            if let Some(&face) = v.faces.names.get(name) {
                out.push(word(
                    face,
                    canvas.centred(seen(face.1)),
                    rise(NAME_Y + drop, stage(0.0)),
                    ink(focus(PickerRow::Colour), stage(0.0)),
                ));
            }
        }
        // More colours than fit slide along under a window kept around the cursor; the end
        // swatches fade where the row goes on past them.
        let all = v.swatches.len();
        let shown = all.min(SWATCHES_SHOWN);
        let from = v.swatch.saturating_sub(shown / 2).min(all - shown);
        let first = canvas.w / 2.0 - SWATCH_PITCH * (shown as f32 - 1.0) / 2.0;
        for (at, i) in (from..from + shown).enumerate() {
            let more = (at == 0 && from > 0) || (at == shown - 1 && from + shown < all);
            let k = stage(1.0) * if more { SWATCH_MORE } else { 1.0 };
            swatch(
                v,
                first + SWATCH_PITCH * at as f32,
                rise(SWATCH_Y + drop, stage(1.0)),
                v.swatches[i].0,
                i == v.swatch,
                focus(PickerRow::Colour),
                &|c| ink(c, k),
                out,
            );
        }
        let finishes: Vec<(TexId, u32)> = v.finishes.iter().map(|&f| v.faces.finishes[f]).collect();
        boxed_row(
            &finishes,
            v.finish_on.then_some(v.finish),
            v.row == PickerRow::Finish,
            FINISH_Y + drop,
            FINISH_GAP,
            stage(2.0),
            out,
        );
        if let Some((first, count, model)) = v.model {
            boxed_row(
                &v.faces.models[first..first + count],
                Some(model),
                v.row == PickerRow::Model,
                if v.sgb { (to.y + to.h + SGB_MODEL_GAP).round() } else { MODEL_Y + drop },
                FINISH_GAP,
                stage(2.5),
                out,
            );
        }
    }

    let legend = if v.confirming {
        v.faces.confirm.clone()
    } else {
        let mut legend = v.faces.legend.clone();
        // Before Save, as A stays last on every legend.
        let at = legend.len().saturating_sub(1);
        if v.remove {
            legend.insert(at, v.faces.remove);
        } else if v.search {
            legend.insert(at, v.faces.search);
        }
        legend
    };
    for (tex, w, x) in centred_hints(canvas, &legend, LEGEND_GAP) {
        out.push(tint(
            tex,
            x,
            rise(LEGEND_Y, stage(3.0)),
            w as f32,
            HINT_H as f32,
            ink(v.ink.text, stage(3.0)),
        ));
    }
    // The cart goes over the rows, so they slip out from under it as it moves.
    let cart: Vec<Draw> = out.drain(cart_from..cart_to).collect();
    out.extend(cart);
}

/// One disc on a row, with a faint rim so the darkest still stands off the ground, and the
/// ring around the chosen one.
#[allow(clippy::too_many_arguments)]
fn swatch(
    v: &ShellPickerView,
    cx: f32,
    cy: f32,
    colour: Option<[u8; 3]>,
    chosen: bool,
    ring: [f32; 4],
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
    if chosen {
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
