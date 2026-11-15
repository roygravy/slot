//! Wear on a cart face: the plastic warms and rubs lighter where fingers hold it, with a polish
//! where the thumb sits and a few fine scratches; the sticker fades and frays. Baked into the
//! face (before the dust) once, with a seed per cart so the pattern never changes.

use crate::dust::{hash, noise};
use crate::CartFace;

const WEAR: [f32; 4] = [0.0, 0.35, 0.7, 1.0];

fn lighten(px: &mut [u8], a: f32, k: f32) {
    for c in px.iter_mut().take(3) {
        *c = (*c as f32 + (255.0 - *c as f32) * a * k).round().clamp(0.0, 255.0) as u8;
    }
}

/// Wear of the plastic (level 0..=3). The alpha of the face is the cart's outline.
pub fn wear_plastic(face: &mut CartFace, level: u8, seed: u64) {
    let w = WEAR[level.min(3) as usize];
    if w <= 0.0 {
        return;
    }
    let (fw, fh) = (face.w as i32, face.h as i32);
    let idx = |x: i32, y: i32| (y * fw + x) as usize * 4;
    let solid = |f: &CartFace, x: i32, y: i32| x >= 0 && y >= 0 && x < fw && y < fh && f.rgba[idx(x, y) + 3] > 128;
    let (mut top, mut bot, mut left, mut right) = (fh, 0, fw, 0);
    for y in 0..fh {
        for x in 0..fw {
            if solid(face, x, y) {
                top = top.min(y);
                bot = bot.max(y);
                left = left.min(x);
                right = right.max(x);
            }
        }
    }
    if bot <= top {
        return;
    }
    let span = (bot - top) as f32;
    let width = (right - left).max(1) as f32;
    let src = CartFace { rgba: face.rgba.clone(), w: face.w, h: face.h };
    // thumb polish blobs, lower middle
    let blobs: Vec<(f32, f32)> = (0..3)
        .map(|n| {
            let n = n as i64;
            (
                (left + right) as f32 / 2.0 + (hash(seed ^ 11, n, 0) - 0.5) * 0.32 * width,
                top as f32 + span * (0.66 + (hash(seed ^ 11, n, 1) - 0.5) * 0.14),
            )
        })
        .collect();
    for y in top..=bot {
        for x in left..=right {
            if !solid(&src, x, y) {
                continue;
            }
            let i = idx(x, y);
            let (fx, fy) = (x as f32, y as f32);
            // warmer plastic
            face.rgba[i + 1] = (face.rgba[i + 1] as f32 * (1.0 - 0.05 * w * 0.5)).round() as u8;
            face.rgba[i + 2] = (face.rgba[i + 2] as f32 * (1.0 - 0.05 * w * 1.4)).round() as u8;
            // polish
            let mut sheen = 0.0f32;
            for (bx, by) in &blobs {
                sheen += (-(((fx - bx) / 30.0).powi(2) + ((fy - by) / 20.0).powi(2))).exp();
            }
            for c in 0..3 {
                face.rgba[i + c] = (face.rgba[i + c] as f32 + 6.0 * sheen.min(1.0) * w).min(255.0) as u8;
            }
            // rubbed edges: only within 2 px of the outline
            let edge = (1..=2).any(|k| !solid(&src, x - k, y) || !solid(&src, x + k, y) || !solid(&src, x, y - k) || !solid(&src, x, y + k));
            if edge {
                let v = (fy - top as f32) / span;
                let corner = (-((fx - left as f32) / 40.0).powi(2)).exp() + (-((fx - right as f32) / 40.0).powi(2)).exp();
                let low = (0.25 + 0.75 * ((v - 0.3) / 0.7).clamp(0.0, 1.0)) * (0.6 + 0.9 * corner);
                let field = 0.62 * noise(seed ^ 12, fx, fy, 7.0) + 0.38 * noise(seed ^ 13, fx, fy, 3.0);
                let thr = 0.74 - 0.30 * w * low;
                let a = ((field - thr) * 4.5).clamp(0.0, 1.0) * 0.6;
                lighten(&mut face.rgba[i..i + 3], a.min(0.7), 0.6);
            }
        }
    }
    // fine scratches
    for n in 0..(w * 18.0) as i64 {
        let x0 = left as f32 + 6.0 + hash(seed ^ 14, n, 0) * (width - 12.0);
        let y0 = top as f32 + 6.0 + hash(seed ^ 14, n, 1) * (span - 12.0);
        let ang = (12.0 + (hash(seed ^ 14, n, 2) - 0.5) * 60.0f32).to_radians();
        let len = 8.0 + hash(seed ^ 14, n, 3) * 18.0;
        let k = 0.2 + 0.25 * hash(seed ^ 14, n, 4);
        for s in 0..(len as i32) {
            let (x, y) = ((x0 + ang.cos() * s as f32) as i32, (y0 + ang.sin() * s as f32) as i32);
            if solid(&src, x, y) {
                let i = idx(x, y);
                lighten(&mut face.rgba[i..i + 3], 1.0, k);
            }
        }
    }
}

/// Wear of the sticker at `rect` (x, y, w, h) in the face. Level 1 fades the ink, 2 frays one
/// edge and adds a thumb smudge, 3 frays two edges, adds a crease and scratches. The art stays
/// readable.
pub fn wear_label(face: &mut CartFace, rect: (u32, u32, u32, u32), level: u8, seed: u64) {
    if level == 0 {
        return;
    }
    let (lx, ly, lw, lh) = (rect.0 as i32, rect.1 as i32, rect.2 as i32, rect.3 as i32);
    let fw = face.w as i32;
    let idx = |x: i32, y: i32| (y * fw + x) as usize * 4;
    let inside = |x: i32, y: i32| x >= 0 && y >= 0 && x < face.w as i32 && y < face.h as i32;
    // the plastic bed shows at a frayed edge: take its colour from just outside the sticker
    let bed = |face: &CartFace, x: i32, y: i32| {
        let (bx, by) = (x.clamp(lx - 3, lx + lw + 2).max(0), y.clamp(ly - 3, ly + lh + 2).max(0));
        let (ox, oy) = if x < lx + 3 { (lx - 3, y) } else if x >= lx + lw - 3 { (lx + lw + 2, y) } else if y < ly + 3 { (x, ly - 3) } else { (x, ly + lh + 2) };
        let (ox, oy) = (ox.clamp(0, face.w as i32 - 1), oy.clamp(0, face.h as i32 - 1));
        let _ = (bx, by);
        let i = idx(ox, oy);
        [face.rgba[i], face.rgba[i + 1], face.rgba[i + 2]]
    };
    let fade = [0.0, 0.10, 0.14, 0.18][level.min(3) as usize];
    for y in ly..ly + lh {
        for x in lx..lx + lw {
            if !inside(x, y) {
                continue;
            }
            let i = idx(x, y);
            if face.rgba[i + 3] < 200 {
                continue;
            }
            // ink fades toward paper white
            for c in 0..3 {
                face.rgba[i + c] = (face.rgba[i + c] as f32 * (1.0 - fade) + 238.0 * fade).round() as u8;
            }
            if level >= 2 {
                // a faint thumb print low on the sticker
                let (dx, dy) = ((x - (lx + lw * 6 / 10)) as f32 / (lw as f32 * 0.16), (y - (ly + lh * 9 / 10)) as f32 / (lh as f32 * 0.22));
                let print = (-(dx * dx + dy * dy)).exp() * (0.5 + 0.5 * noise(seed ^ 21, x as f32, y as f32, 2.0));
                lighten(&mut face.rgba[i..i + 3], print, 0.22);
            }
        }
    }
    // fray: a ragged strip along an edge where the sticker has worn through to the plastic
    let frays = if level >= 3 { 2 } else { 1 };
    for n in 0..frays {
        let side = ((hash(seed ^ 22, n, 0) * 4.0) as i64).min(3);
        let along = 0.15 + 0.5 * hash(seed ^ 22, n, 1);
        let run = 0.18 + 0.12 * hash(seed ^ 22, n, 2);
        let (len, depth_max) = if side < 2 { (lw, 3.0 + 1.5 * hash(seed ^ 22, n, 3)) } else { (lh, 3.0 + 1.5 * hash(seed ^ 22, n, 3)) };
        let (a0, a1) = ((along * len as f32) as i32, ((along + run) * len as f32) as i32);
        for t in a0..a1 {
            let tt = (t - a0) as f32 / (a1 - a0).max(1) as f32;
            let env = (std::f32::consts::PI * tt).sin();
            let depth = depth_max * env * (0.5 + noise(seed ^ 23, t as f32, n as f32 * 9.0, 2.5));
            for d in 0..depth as i32 + 1 {
                let (x, y) = match side {
                    0 => (lx + t, ly + d),
                    1 => (lx + t, ly + lh - 1 - d),
                    2 => (lx + d, ly + t),
                    _ => (lx + lw - 1 - d, ly + t),
                };
                if !inside(x, y) {
                    continue;
                }
                let c = bed(face, x, y);
                let i = idx(x, y);
                let k = if d as f32 > depth - 1.0 { 0.55 } else { 0.9 };
                for ch in 0..3 {
                    face.rgba[i + ch] = (face.rgba[i + ch] as f32 * (1.0 - k) + c[ch] as f32 * k).round() as u8;
                }
            }
        }
    }
    if level >= 3 {
        // a fine crease and a few scratches across the art
        let x0 = lx as f32 + lw as f32 * (0.2 + 0.2 * hash(seed ^ 24, 0, 0));
        let slope = 0.25 + 0.2 * hash(seed ^ 24, 0, 1);
        for s in 0..lh {
            let (x, y) = ((x0 + s as f32 * slope) as i32, ly + s);
            for (dx, k, dark) in [(0, 0.35, false), (1, 0.12, true)] {
                if inside(x + dx, y) && x + dx >= lx && x + dx < lx + lw {
                    let i = idx(x + dx, y);
                    if dark {
                        for c in 0..3 {
                            face.rgba[i + c] = (face.rgba[i + c] as f32 * 0.9) as u8;
                        }
                    } else {
                        lighten(&mut face.rgba[i..i + 3], 1.0, k);
                    }
                }
            }
        }
        for n in 0..5i64 {
            let sx = lx as f32 + hash(seed ^ 25, n, 0) * lw as f32 * 0.8;
            let sy = ly as f32 + hash(seed ^ 25, n, 1) * lh as f32;
            let ang = (hash(seed ^ 25, n, 2) - 0.5) * 1.2;
            for s in 0..(6 + (hash(seed ^ 25, n, 3) * 10.0) as i32) {
                let (x, y) = ((sx + ang.cos() * s as f32) as i32, (sy + ang.sin() * s as f32) as i32);
                if inside(x, y) && x >= lx && x < lx + lw && y >= ly && y < ly + lh {
                    let i = idx(x, y);
                    lighten(&mut face.rgba[i..i + 3], 1.0, 0.3);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(w: u32, h: u32) -> CartFace {
        CartFace { rgba: [60, 70, 80, 255].repeat((w * h) as usize), w, h }
    }

    #[test]
    fn level_zero_changes_nothing() {
        let mut f = flat(60, 80);
        let before = f.rgba.clone();
        wear_plastic(&mut f, 0, 1);
        wear_label(&mut f, (10, 10, 40, 30), 0, 1);
        assert_eq!(f.rgba, before);
    }

    #[test]
    fn wear_is_fixed_per_seed_and_grows() {
        let run = |level, seed| {
            let mut f = flat(90, 100);
            wear_plastic(&mut f, level, seed);
            wear_label(&mut f, (15, 20, 60, 40), level, seed);
            f.rgba
        };
        assert_eq!(run(3, 5), run(3, 5));
        assert_ne!(run(3, 5), run(3, 6));
        let diff = |a: &[u8]| a.iter().zip(flat(90, 100).rgba).filter(|(x, y)| **x != *y).count();
        assert!(diff(&run(1, 5)) < diff(&run(3, 5)));
    }
}
