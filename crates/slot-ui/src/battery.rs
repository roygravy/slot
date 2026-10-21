//! The battery as a phone draws it: a rounded capsule filled to the charge, with the
//! percentage cut out of it and a bolt beside the number while charging, all in one ink.

use slot_gfx::{Draw, TexId};
use slot_power::Battery;

use crate::footer::Printed;
use crate::plate::UndoFace;
use crate::text::{self, Layout};

pub const BOLT_PX: f32 = 18.0;

/// The capsule; the nub sits past it.
pub const GAUGE_W: f32 = 38.0;
pub const GAUGE_H: f32 = 18.0;
const _: () = assert!(GAUGE_H < GAUGE_W);
const NUB_GAP: f32 = 1.0;
const NUB_W: f32 = 2.5;
const NUB_H: f32 = 7.0;
const RADIUS: f32 = 5.0;
/// The capsule's own corner, kept for callers that size against the old outline.
pub const WALL: f32 = 2.0;

const NUMBER_PX: f32 = 14.5;
const BOLT_W: f32 = 8.0;
const BOLT_H: f32 = 13.0;
const BOLT_GAP: f32 = 1.0;

/// How much of the ink the empty part of the capsule and the nub keep.
const EMPTY_ALPHA: f32 = 0.35;

const SS: u32 = 4;

/// The whole face is this wide: capsule, gap and nub.
pub fn battery_w() -> u32 {
    (GAUGE_W + NUB_GAP + NUB_W).ceil() as u32
}

/// How wide the gauge draws, from where it is placed to its nub.
pub fn gauge_width(battery: Option<Battery>, _face_w: u32) -> f32 {
    match battery {
        Some(_) => battery_w() as f32,
        None => 0.0,
    }
}

/// The battery face, in the ink alone (the bolt says it is charging), drawn smooth rather than from whole pixels so it reads as a drawing.
pub fn battery_face(percent: u8, charging: bool, ink: [u8; 3]) -> UndoFace {
    let (w, h) = (battery_w(), GAUGE_H as u32);
    let fill_to = GAUGE_W * f32::from(percent.min(100)) / 100.0;

    let number = percent.min(100).to_string();
    let cut = cut_out(&number, charging, w, h);

    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for py in 0..h {
        for px in 0..w {
            let (mut body, mut full, mut nub) = (0.0f32, 0.0f32, 0.0f32);
            for sy in 0..SS {
                for sx in 0..SS {
                    let x = px as f32 + (sx as f32 + 0.5) / SS as f32;
                    let y = py as f32 + (sy as f32 + 0.5) / SS as f32;
                    if in_rounded(x, y, 0.0, 0.0, GAUGE_W, GAUGE_H, RADIUS) {
                        body += 1.0;
                        if x < fill_to {
                            full += 1.0;
                        }
                    }
                    let nx = GAUGE_W + NUB_GAP;
                    let ny = (GAUGE_H - NUB_H) / 2.0;
                    if in_rounded(x, y, nx - NUB_W, ny, 2.0 * NUB_W, NUB_H, NUB_W) && x >= nx {
                        nub += 1.0;
                    }
                }
            }
            let n = (SS * SS) as f32;
            let (body, full, nub) = (body / n, full / n, nub / n);
            let empty = body - full;
            let keep = 1.0 - f32::from(cut[(py * w + px) as usize]) / 255.0;
            let a_full = full * keep;
            let a_empty = (empty + nub) * EMPTY_ALPHA * keep;
            let a = (a_full + a_empty).min(1.0);
            rgba.extend_from_slice(&[ink[0], ink[1], ink[2], (a * 255.0).round() as u8]);
        }
    }
    UndoFace { rgba, w, h }
}

/// The number, with the bolt after it while charging, as coverage to cut from the capsule.
fn cut_out(number: &str, charging: bool, w: u32, h: u32) -> Vec<u8> {
    let mut cut = vec![0u8; (w * h) as usize];
    let Some(font) = text::label_font() else {
        return cut;
    };
    let text_w = text::line_width(font, number, NUMBER_PX, 0.0);
    let bolt = if charging { BOLT_GAP + BOLT_W } else { 0.0 };
    let left = ((GAUGE_W - text_w - bolt) / 2.0).round();
    let box_w = text_w.ceil() as u32 + 2;
    let layout = Layout {
        lines: vec![number.to_string()],
        px: NUMBER_PX,
        tracking: 0.0,
    };
    let number_cov = text::coverage(box_w, h, &layout);
    let dx = left as i32 - 1;
    for y in 0..h {
        for x in 0..box_w {
            let tx = x as i32 + dx;
            if tx < 0 || tx >= w as i32 {
                continue;
            }
            let i = (y * w) as usize + tx as usize;
            cut[i] = cut[i].max(number_cov[(y * box_w + x) as usize]);
        }
    }
    if charging {
        let bx = left + text_w + BOLT_GAP;
        let by = (GAUGE_H - BOLT_H) / 2.0;
        for py in 0..h {
            for px in 0..w {
                let mut hits = 0u32;
                for sy in 0..SS {
                    for sx in 0..SS {
                        let x = px as f32 + (sx as f32 + 0.5) / SS as f32 - bx;
                        let y = py as f32 + (sy as f32 + 0.5) / SS as f32 - by;
                        hits += u32::from(in_bolt(x, y));
                    }
                }
                let i = (py * w + px) as usize;
                cut[i] = cut[i].max((hits * 255 / (SS * SS)) as u8);
            }
        }
    }
    cut
}

fn in_rounded(x: f32, y: f32, left: f32, top: f32, w: f32, h: f32, r: f32) -> bool {
    if x < left || y < top || x > left + w || y > top + h {
        return false;
    }
    let cx = x.clamp(left + r, left + w - r);
    let cy = y.clamp(top + r, top + h - r);
    (x - cx).powi(2) + (y - cy).powi(2) <= r * r
}

/// A lightning bolt in a `BOLT_W` by `BOLT_H` box.
fn in_bolt(x: f32, y: f32) -> bool {
    // Drawn in a 6 by 10 box, scaled to the bolt's own.
    let (x, y) = (x * 6.0 / BOLT_W, y * 10.0 / BOLT_H);
    const P: [(f32, f32); 6] = [
        (3.8, 0.0),
        (0.2, 5.8),
        (2.7, 5.8),
        (2.0, 10.0),
        (5.8, 4.0),
        (3.3, 4.0),
    ];
    let mut inside = false;
    let mut j = P.len() - 1;
    for i in 0..P.len() {
        let (xi, yi) = P[i];
        let (xj, yj) = P[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Draws the battery face made by `battery_face`, which the caller keeps up to date.
pub fn draw_gauge(
    x: f32,
    y: f32,
    battery: Option<Battery>,
    face: Printed,
    _bolt: Option<TexId>,
    _ink: [u8; 3],
    out: &mut Vec<Draw>,
) {
    if battery.is_none() {
        return;
    }
    let (x, y, w, h) = (x.round(), y.round(), battery_w() as f32, GAUGE_H);
    out.push(match face.face {
        Some(tex) => Draw::Tex {
            x,
            y,
            w,
            h,
            tex,
            alpha: 1.0,
        },
        None => Draw::Rect {
            x,
            y,
            w,
            h,
            colour: [1.0, 1.0, 1.0, 0.08],
        },
    });
}
