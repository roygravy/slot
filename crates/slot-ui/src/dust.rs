//! Dust on a cart face: a fine matte film that mutes the plastic and the label, thicker on top
//! and on ridges and ledges, with a few soft motes, lint fibres and (from level 2) a thumb swipe
//! through it. Baked into the face once, with a seed per cart so the pattern never changes.

use crate::CartFace;

/// How much film each dust level lays down (level 0 is clean).
const DUST: [f32; 4] = [0.0, 0.35, 0.65, 1.0];
const DUST_COLOUR: [f32; 3] = [204.0, 200.0, 192.0];

pub(crate) fn hash(seed: u64, a: i64, b: i64) -> f32 {
    let mut h = seed ^ (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (b as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^= h >> 33;
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// Smooth value noise with `cell` pixels between lattice points.
pub(crate) fn noise(seed: u64, x: f32, y: f32, cell: f32) -> f32 {
    let (fx, fy) = (x / cell, y / cell);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let (tx, ty) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let (x0, y0) = (x0 as i64, y0 as i64);
    let a = hash(seed, x0, y0) * (1.0 - tx) + hash(seed, x0 + 1, y0) * tx;
    let b = hash(seed, x0, y0 + 1) * (1.0 - tx) + hash(seed, x0 + 1, y0 + 1) * tx;
    a * (1.0 - ty) + b * ty
}

/// Dust level 0..=3 laid over the face in place. The face's alpha is the cart's outline.
pub fn dust_face(face: &mut CartFace, level: u8, seed: u64) {
    let d = DUST[level.min(3) as usize];
    if d <= 0.0 {
        return;
    }
    let (w, h) = (face.w as i32, face.h as i32);
    let at = |x: i32, y: i32| -> usize { (y.clamp(0, h - 1) * w + x.clamp(0, w - 1)) as usize * 4 };
    let solid = |f: &CartFace, x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h && f.rgba[at(x, y) + 3] > 128;
    let (mut top, mut bot, mut left, mut right) = (h, 0, w, 0);
    for y in 0..h {
        for x in 0..w {
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
    let span = (bot - top).max(1) as f32;
    let lum = |f: &CartFace, x: i32, y: i32| {
        let i = at(x, y);
        (f.rgba[i] as f32 + f.rgba[i + 1] as f32 + f.rgba[i + 2] as f32) / 765.0
    };
    // The thumb swipe: a soft stroke through the dust, a little below the middle.
    let swipe = (level >= 2).then(|| {
        let cx = (left + right) as f32 / 2.0 + (hash(seed, 7, 1) - 0.5) * 0.16 * (right - left) as f32;
        let cy = top as f32 + span * (0.55 + 0.15 * hash(seed, 7, 2));
        let half = (right - left) as f32 * 0.27;
        let ang = (-14.0 + 20.0 * hash(seed, 7, 3)).to_radians();
        (cx, cy, half, ang)
    });
    let src = face.rgba.clone();
    let probe = CartFace { rgba: src.clone(), w: face.w, h: face.h };
    let mut film = vec![0.0f32; (w * h) as usize];
    for y in top..=bot {
        for x in left..=right {
            let i = at(x, y);
            let a = src[i + 3] as f32 / 255.0;
            if a <= 0.0 {
                continue;
            }
            let (fx, fy) = (x as f32, y as f32);
            let v = ((fy - top as f32) / span).clamp(0.0, 1.0);
            let top_heavy = (1.0 - v).powf(1.7);
            // ridges: brighter than the surroundings a few pixels out
            let around = (lum(&probe, x - 3, y) + lum(&probe, x + 3, y) + lum(&probe, x, y - 3) + lum(&probe, x, y + 3)) / 4.0;
            let ridge = ((lum(&probe, x, y) - around) * 5.0).clamp(0.0, 1.0);
            // ledges: the first rows under an edge that faces up
            let ledge = (1..=3).any(|k| !solid(&probe, x, y - k)) as u8 as f32;
            let g = 0.75 * noise(seed, fx, fy, 2.0) + 0.25 * noise(seed ^ 1, fx, fy, 9.0);
            let gn = ((g - 0.5) * 1.5 + 0.5).clamp(0.0, 1.0);
            let clumps = noise(seed ^ 2, fx, fy, 22.0);
            let mut va = d
                * (0.34 * (0.55 + 0.45 * top_heavy) * (0.65 + 0.7 * gn) * (0.85 + 0.3 * clumps)
                    + 0.12 * ridge * (0.5 + top_heavy)
                    + 0.28 * ledge * (0.5 + gn));
            if let Some((cx, cy, half, ang)) = swipe {
                let (dx, dy) = (fx - cx, fy - cy);
                let along = (dx * ang.cos() + dy * ang.sin()).clamp(-half, half);
                let (px, py) = (cx + along * ang.cos(), cy + along * ang.sin());
                let dist = ((fx - px).powi(2) + (fy - py).powi(2)).sqrt();
                let s = (1.0 - (dist - 3.0) / 6.0).clamp(0.0, 1.0);
                va *= 1.0 - 0.8 * s;
            }
            film[(y * w + x) as usize] = va.clamp(0.0, 0.6) * a;
        }
    }
    // A few soft motes, more on the top half and on ledges.
    let area = (right - left) as f32 * span;
    let motes = (d * 90.0 * area / (256.0 * 297.0)) as i32;
    for n in 0..motes {
        let x = left as f32 + hash(seed ^ 3, n as i64, 0) * (right - left) as f32;
        let y = top as f32 + hash(seed ^ 3, n as i64, 1).powf(1.6) * span;
        let r = 0.8 + 1.1 * hash(seed ^ 3, n as i64, 2);
        let p = 0.3 + 0.4 * hash(seed ^ 3, n as i64, 3);
        for yy in (y - 3.0) as i32..=(y + 3.0) as i32 {
            for xx in (x - 3.0) as i32..=(x + 3.0) as i32 {
                if !solid(&probe, xx, yy) {
                    continue;
                }
                let k = (1.0 - (((xx as f32 - x).powi(2) + (yy as f32 - y).powi(2)).sqrt() - r * 0.4) / (r + 0.8)).clamp(0.0, 1.0);
                let f = &mut film[(yy * w + xx) as usize];
                *f = f.max(p * k);
            }
        }
    }
    // Lint fibres at the heavier levels.
    let fibres = [0, 0, 2, 4][level.min(3) as usize];
    for n in 0..fibres {
        let mut x = left as f32 + hash(seed ^ 4, n, 0) * (right - left) as f32;
        let mut y = top as f32 + hash(seed ^ 4, n, 1).powf(1.5) * span;
        let mut ang = (hash(seed ^ 4, n, 2) - 0.5) * 1.6;
        for step in 0..(6 + (hash(seed ^ 4, n, 3) * 6.0) as i64) {
            ang += (hash(seed ^ 5, n, step) - 0.5) * 0.9;
            x += ang.cos() * 1.7;
            y += ang.sin() * 1.7;
            let (xx, yy) = (x as i32, y as i32);
            if solid(&probe, xx, yy) {
                let f = &mut film[(yy * w + xx) as usize];
                *f = f.max(0.5);
            }
        }
    }
    for y in top..=bot {
        for x in left..=right {
            let va = film[(y * w + x) as usize].min(0.65);
            if va <= 0.0 {
                continue;
            }
            let i = at(x, y);
            for c in 0..3 {
                face.rgba[i + c] = (src[i + c] as f32 * (1.0 - va) + DUST_COLOUR[c] * va).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// A soft round grain for the blow animation: white, fading from the middle out.
pub fn puff_dot_face() -> CartFace {
    let n = 16u32;
    let mut rgba = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (x as f32 + 0.5 - n as f32 / 2.0, y as f32 + 0.5 - n as f32 / 2.0);
            let d = (dx * dx + dy * dy).sqrt() / (n as f32 / 2.0);
            let a = (1.0 - d).clamp(0.0, 1.0);
            rgba.extend_from_slice(&[255, 255, 255, (a * a * (3.0 - 2.0 * a) * 255.0) as u8]);
        }
    }
    CartFace { rgba, w: n, h: n }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(w: u32, h: u32) -> CartFace {
        CartFace { rgba: [60, 70, 80, 255].repeat((w * h) as usize), w, h }
    }

    #[test]
    fn clean_level_changes_nothing() {
        let mut f = flat(60, 80);
        let before = f.rgba.clone();
        dust_face(&mut f, 0, 1);
        assert_eq!(f.rgba, before);
    }

    #[test]
    fn dust_is_fixed_per_seed_and_grows_with_level() {
        let lift = |level, seed| {
            let mut f = flat(80, 100);
            dust_face(&mut f, level, seed);
            f
        };
        assert_eq!(lift(2, 9).rgba, lift(2, 9).rgba);
        assert_ne!(lift(2, 9).rgba, lift(2, 10).rgba);
        let sum = |f: &CartFace| f.rgba.chunks(4).map(|p| p[0] as u64).sum::<u64>();
        assert!(sum(&lift(1, 9)) < sum(&lift(2, 9)) && sum(&lift(2, 9)) < sum(&lift(3, 9)));
    }

    #[test]
    fn transparent_pixels_stay_untouched() {
        let mut f = flat(40, 40);
        for p in f.rgba.chunks_mut(4).take(40 * 10) {
            p.copy_from_slice(&[0, 0, 0, 0]);
        }
        dust_face(&mut f, 3, 4);
        assert!(f.rgba.chunks(4).take(40 * 10).all(|p| p == [0, 0, 0, 0]));
    }
}
