use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use fontdue::{Font, FontSettings};

use crate::CartFace;

const SYMBOLS_TTF: &[u8] = include_bytes!("../assets/SymbolsNerdFontMono-Regular.ttf");

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Icon {
    Volume,
    VolumeZero,
    VolumeMuted,
    Brightness,
    BlueLight,
    FastForward,
    FastForwardLatched,
    Rewind,
    Alert,
    Charging,
    Headphones,
    HeadphonesMuted,
    /// A robber's masked face: cheats are on.
    Cheats,
    Power,
    Wifi,
}

impl Icon {
    pub const ALL: [Icon; 15] = [
        Icon::Volume,
        Icon::VolumeZero,
        Icon::VolumeMuted,
        Icon::Brightness,
        Icon::BlueLight,
        Icon::FastForward,
        Icon::FastForwardLatched,
        Icon::Rewind,
        Icon::Alert,
        Icon::Charging,
        Icon::Headphones,
        Icon::HeadphonesMuted,
        Icon::Cheats,
        Icon::Power,
        Icon::Wifi,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn glyph(self) -> char {
        match self {
            Icon::Volume => '\u{f028}',
            Icon::VolumeZero => '\u{f026}',
            Icon::VolumeMuted => '\u{f075f}',
            Icon::Brightness => '\u{f185}',
            Icon::BlueLight => '\u{f186}',
            Icon::FastForward => '\u{f06d2}',
            Icon::FastForwardLatched => '\u{f0211}',
            Icon::Rewind => '\u{f04a}',
            Icon::Alert => '\u{f0026}',
            Icon::Charging => '\u{f0e7}',
            Icon::Headphones => '\u{f025}',
            Icon::HeadphonesMuted => '\u{f07ce}',
            Icon::Cheats => '\u{f1058}',
            Icon::Power => '\u{f011}',
            Icon::Wifi => '\u{f1eb}',
        }
    }
}

const HALO: [u8; 3] = [0x08, 0x08, 0x0a];
pub const HALO_PX: u32 = 1;

pub fn haloed(cov: &[u8], cw: u32, ch: u32, colour: [u8; 3]) -> CartFace {
    let pad = HALO_PX as usize;
    let (cw, ch) = (cw as usize, ch as usize);
    let (w, h) = (cw + 2 * pad, ch + 2 * pad);
    let at = |x: isize, y: isize| -> u8 {
        if x < 0 || y < 0 || x >= cw as isize || y >= ch as isize {
            0
        } else {
            cov[y as usize * cw + x as usize]
        }
    };

    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let (gx, gy) = (x as isize - pad as isize, y as isize - pad as isize);
            let ink = at(gx, gy);
            let mut halo = 0u8;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    halo = halo.max(at(gx + dx, gy + dy));
                }
            }
            if ink > 0 {
                let a = ink as u32;
                let inv = 255 - a;
                let mix = |c: u8, s: u8| ((c as u32 * a + s as u32 * inv) / 255) as u8;
                rgba.extend_from_slice(&[
                    mix(colour[0], HALO[0]),
                    mix(colour[1], HALO[1]),
                    mix(colour[2], HALO[2]),
                    ink.max(halo),
                ]);
            } else {
                rgba.extend_from_slice(&[HALO[0], HALO[1], HALO[2], halo]);
            }
        }
    }
    CartFace {
        rgba,
        w: w as u32,
        h: h as u32,
    }
}

pub fn icon_face(icon: Icon, px: f32, colour: [u8; 3]) -> CartFace {
    let Some(r) = raster(icon, px) else {
        return CartFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    haloed(&r.cov, r.w, r.h, colour)
}

pub fn icon_glyph_face(icon: Icon, px: f32, colour: [u8; 3]) -> CartFace {
    let Some(font) = symbols_font() else {
        return CartFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let (metrics, coverage) = font.rasterize(icon.glyph(), px);
    haloed(
        &coverage,
        metrics.width as u32,
        metrics.height as u32,
        colour,
    )
}

pub fn icon_glyph_box(icon: Icon, px: f32) -> (u32, u32) {
    let Some(font) = symbols_font() else {
        return (0, 0);
    };
    let metrics = font.metrics(icon.glyph(), px);
    (
        metrics.width as u32 + 2 * HALO_PX,
        metrics.height as u32 + 2 * HALO_PX,
    )
}

pub fn icon_box(px: f32) -> (u32, u32) {
    match raster(Icon::Volume, px) {
        Some(r) => (r.w + 2 * HALO_PX, r.h + 2 * HALO_PX),
        None => (0, 0),
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Badge {
    Link,
    LinkBroken,
}

impl Badge {
    pub fn glyph(self) -> char {
        match self {
            Badge::Link => '\u{f0339}',
            Badge::LinkBroken => '\u{f033a}',
        }
    }
}

/// A highlight for a favourite cart, added as light on the ground behind it: a soft warm pool
/// centred on the cart that shows round its edges and fades out over the ground, with no edge
/// and no beam. `cw` x `ch` is the cart; the face is `margin` px larger all round,
/// built at half resolution.
pub fn halo_face(cw: u32, ch: u32, margin: u32, colour: [u8; 3], peak: f32, fall: f32) -> CartFace {
    let (fw, fh) = (cw + 2 * margin, ch + 2 * margin);
    let (w, h) = (fw.div_ceil(2), fh.div_ceil(2));
    let (cx, cy) = (fw as f32 / 2.0, fh as f32 / 2.0);
    // Full just inside the cart's edges, so it shows right round them.
    // Its corners round off over `round` px so the light never shows a square corner (Roy).
    let round = 28.0;
    let (hw, hh) = (
        cw as f32 / 2.0 - 12.0 - round,
        ch as f32 / 2.0 - 12.0 - round,
    );
    // Close round the cart and soft (Roy): well inside the face's own edge.
    let reach = margin as f32 * 0.6 + 12.0;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for py in 0..h {
        for px in 0..w {
            let (x, y) = ((px as f32 + 0.5) * 2.0 - cx, (py as f32 + 0.5) * 2.0 - cy);
            let (qx, qy) = (x.abs() - hw, y.abs() - hh);
            let out = ((qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() - round).max(0.0);
            let t = (out / reach).min(1.0);
            let a = (1.0 - t).powf(fall) * peak;
            let o = ((py * w + px) * 4) as usize;
            rgba[o..o + 4].copy_from_slice(&[
                colour[0],
                colour[1],
                colour[2],
                (a * 255.0).round().min(255.0) as u8,
            ]);
        }
    }
    CartFace { rgba, w, h }
}

/// A soft drop shadow for a cart: a black rounded box `cw` x `ch` blurred over `margin` px,
/// built at half resolution and smoothed when stretched.
pub fn drop_shadow_face(cw: u32, ch: u32, margin: u32) -> CartFace {
    let (fw, fh) = (cw + 2 * margin, ch + 2 * margin);
    let (w, h) = (fw.div_ceil(2), fh.div_ceil(2));
    let (m, radius) = (margin as f32, 14.0_f32);
    let (cx, cy) = (fw as f32 / 2.0, fh as f32 / 2.0);
    let (hw, hh) = (
        cw as f32 / 2.0 - radius - 4.0,
        ch as f32 / 2.0 - radius - 4.0,
    );
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for py in 0..h {
        for px in 0..w {
            let (x, y) = ((px as f32 + 0.5) * 2.0, (py as f32 + 0.5) * 2.0);
            let (qx, qy) = ((x - cx).abs() - hw, (y - cy).abs() - hh);
            let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
            let d = outside + qx.max(qy).min(0.0) - radius;
            // A smooth fall from full at the box's edge to nothing `margin` px out.
            let t = ((d + m * 0.35) / (m * 1.2)).clamp(0.0, 1.0);
            let a = 1.0 - t * t * (3.0 - 2.0 * t);
            let o = ((py * w + px) * 4) as usize;
            rgba[o..o + 4].copy_from_slice(&[0, 0, 0, (a * 255.0).round() as u8]);
        }
    }
    CartFace { rgba, w, h }
}

/// A smooth white five-point star with a soft dark rim, `px` wide, anti-aliased.
pub fn star_mark_face(px: u32) -> CartFace {
    let pad = 3.0_f32;
    let size = px as f32 + 2.0 * pad;
    let (w, h) = (size.ceil() as u32, size.ceil() as u32);
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0 + px as f32 * 0.04);
    let (outer, inner) = (px as f32 / 2.0, px as f32 / 2.0 * 0.46);
    let pts: Vec<(f32, f32)> = (0..10)
        .map(|k| {
            let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
            let r = if k % 2 == 0 { outer } else { inner };
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect();
    let dist = |x: f32, y: f32| -> f32 {
        let mut best = f32::MAX;
        let mut inside = false;
        for k in 0..10 {
            let (a, b) = (pts[k], pts[(k + 1) % 10]);
            let (ex, ey) = (b.0 - a.0, b.1 - a.1);
            let t = (((x - a.0) * ex + (y - a.1) * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
            let (qx, qy) = (a.0 + t * ex - x, a.1 + t * ey - y);
            best = best.min((qx * qx + qy * qy).sqrt());
            if (a.1 > y) != (b.1 > y) && x < a.0 + (y - a.1) / (b.1 - a.1) * ex {
                inside = !inside;
            }
        }
        if inside {
            -best
        } else {
            best
        }
    };
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for py in 0..h {
        for pxl in 0..w {
            let d = dist(pxl as f32 + 0.5, py as f32 + 0.5);
            let fill = (0.5 - d).clamp(0.0, 1.0);
            let rim = ((2.4 - d) / 1.8).clamp(0.0, 1.0) * 0.5;
            let a = fill + rim * (1.0 - fill);
            if a <= 0.0 {
                continue;
            }
            let c = 255.0 * fill / a;
            let o = ((py * w + pxl) * 4) as usize;
            rgba[o..o + 4].copy_from_slice(&[c as u8, c as u8, c as u8, (a * 255.0).round() as u8]);
        }
    }
    CartFace { rgba, w, h }
}

/// A soft four-point sparkle, `px` square: a white-hot core, four long thin rays that taper
/// to nothing, four short ones between them and a warm gold haze round it, anti-aliased
/// (no square dots, Roy).
pub fn sparkle_face(px: u32) -> CartFace {
    let (w, h) = (px, px);
    let c = px as f32 / 2.0;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for py in 0..h {
        for pxl in 0..w {
            let (x, y) = ((pxl as f32 + 0.5 - c) / c, (py as f32 + 0.5 - c) / c);
            let r = (x * x + y * y).sqrt();
            // A ray along an axis: thin, its width and light falling off with the distance out.
            let ray = |along: f32, across: f32, reach: f32| {
                let t = (1.0 - along.abs() / reach).max(0.0);
                let width = 0.02 + 0.10 * t.powi(3);
                (1.0 - (across.abs() / width)).clamp(0.0, 1.0) * t.powf(0.75)
            };
            let (u, v) = ((x + y) * std::f32::consts::FRAC_1_SQRT_2, (x - y) * std::f32::consts::FRAC_1_SQRT_2);
            let rays = ray(x, y, 1.0).max(ray(y, x, 1.0)).max(0.6 * ray(u, v, 0.34)).max(0.6 * ray(v, u, 0.34));
            let core = (-(r / 0.14).powi(2)).exp();
            let haze = 0.42 * (-(r / 0.30).powi(2)).exp();
            let white = (rays + core).min(1.0);
            let a = (white + haze * (1.0 - white)).min(1.0);
            if a <= 0.0 {
                continue;
            }
            // White where the light is, gold in the haze.
            let k = white / (white + haze).max(1e-6);
            let gold = [255.0, 208.0, 110.0];
            let o = ((py * w + pxl) * 4) as usize;
            for ch in 0..3 {
                rgba[o + ch] = (gold[ch] + (255.0 - gold[ch]) * k).round() as u8;
            }
            rgba[o + 3] = (a * 255.0).round() as u8;
        }
    }
    CartFace { rgba, w, h }
}

/// The favourite's star as a shiny sticker (Roy, sh12): a gold-foil five-point star with
/// rounded points, a bright streak of shine across the foil, a darker gold edge and a faint
/// shadow under it where it lies on the label. No white die-cut edge (Roy 2026-11-12).
pub fn sticker_star_face(px: u32) -> CartFace {
    let edge = 0.0_f32;
    let pad = edge + 3.0;
    let size = px as f32 + 2.0 * pad;
    let (w, h) = (size.ceil() as u32, size.ceil() as u32);
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0 + px as f32 * 0.04);
    let (outer, inner) = (px as f32 / 2.0, px as f32 / 2.0 * 0.5);
    let pts: Vec<(f32, f32)> = (0..10)
        .map(|k| {
            let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
            let r = if k % 2 == 0 { outer } else { inner };
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect();
    let dist = |x: f32, y: f32| -> f32 {
        let mut best = f32::MAX;
        let mut inside = false;
        for k in 0..10 {
            let (a, b) = (pts[k], pts[(k + 1) % 10]);
            let (ex, ey) = (b.0 - a.0, b.1 - a.1);
            let t = (((x - a.0) * ex + (y - a.1) * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
            let (qx, qy) = (a.0 + t * ex - x, a.1 + t * ey - y);
            best = best.min((qx * qx + qy * qy).sqrt());
            if (a.1 > y) != (b.1 > y) && x < a.0 + (y - a.1) / (b.1 - a.1) * ex {
                inside = !inside;
            }
        }
        if inside {
            -best
        } else {
            best
        }
    };
    // The points rounded off a little.
    let round = 0.8;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for py in 0..h {
        for pxl in 0..w {
            let (x, y) = (pxl as f32 + 0.5, py as f32 + 0.5);
            let d = dist(x, y) + round;
            let foil = (0.5 - d).clamp(0.0, 1.0);
            let paper = (0.5 - (d - edge)).clamp(0.0, 1.0);
            // The sticker's shadow, a pixel down, soft.
            let ds = dist(x, y - 1.0) + round - edge;
            let shadow = ((1.6 - ds) / 1.6).clamp(0.0, 1.0) * 0.30;
            // Foil: light at the top left, deep at the bottom right, a bright streak across.
            let u = ((x - cx) * 0.55 + (y - cy) * 0.85) / outer;
            let streak = (-((u + 0.18) / 0.16).powi(2)).exp();
            let t = ((u + 1.0) / 2.0).clamp(0.0, 1.0);
            let lo = [0xFF as f32, 0xE6 as f32, 0x8A as f32];
            let hi = [0xC2 as f32, 0x7C as f32, 0x12 as f32];
            let rim = 1.0 - 0.30 * ((d + 1.6) / 1.2).clamp(0.0, 1.0);
            let mut gold = [0.0f32; 3];
            for c in 0..3 {
                let base = lo[c] + (hi[c] - lo[c]) * t;
                gold[c] = (base * rim + (255.0 - base * rim) * 0.85 * streak).min(255.0);
            }
            let white = [250.0, 248.0, 242.0];
            let mut col = [0.0f32; 3];
            for c in 0..3 {
                col[c] = white[c] + (gold[c] - white[c]) * foil;
            }
            // Paper over its shadow.
            let a = paper + shadow * (1.0 - paper);
            if a <= 0.0 {
                continue;
            }
            let o = ((py * w + pxl) * 4) as usize;
            let k = paper / a;
            rgba[o..o + 4].copy_from_slice(&[
                (col[0] * k).round() as u8,
                (col[1] * k).round() as u8,
                (col[2] * k).round() as u8,
                (a * 255.0).round() as u8,
            ]);
        }
    }
    CartFace { rgba, w, h }
}

pub fn badge_face(badge: Badge, px: f32, colour: [u8; 3]) -> CartFace {
    let Some(font) = symbols_font() else {
        return CartFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let f = frame(font, px);
    let (m, cov) = font.rasterize(badge.glyph(), px);
    let x0 = m.xmin - f.left;
    let y0 = f.top - (m.ymin + m.height as i32);
    let mut out = vec![0u8; (f.w * f.h) as usize];
    for gy in 0..m.height {
        for gx in 0..m.width {
            let (dx, dy) = (x0 + gx as i32, y0 + gy as i32);
            if dx < 0 || dy < 0 || dx >= f.w as i32 || dy >= f.h as i32 {
                continue;
            }
            out[(dy as u32 * f.w + dx as u32) as usize] = cov[gy * m.width + gx];
        }
    }
    haloed(&out, f.w, f.h, colour)
}

struct Raster {
    cov: Vec<u8>,
    w: u32,
    h: u32,
}

type Cache = Mutex<HashMap<(Icon, u32), Arc<Raster>>>;

fn raster(icon: Icon, px: f32) -> Option<Arc<Raster>> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(Mutex::default);
    let key = (icon, px.to_bits());
    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(r) = cache.get(&key) {
        return Some(r.clone());
    }
    let r = Arc::new(rasterise(icon, px)?);
    cache.insert(key, r.clone());
    Some(r)
}

fn rasterise(icon: Icon, px: f32) -> Option<Raster> {
    let font = symbols_font()?;
    let f = frame(font, px);
    if icon == Icon::Power {
        return Some(power_symbol(&f, px));
    }
    let (m, cov) = font.rasterize(icon.glyph(), px);
    let x0 = m.xmin - f.left;
    let y0 = f.top - (m.ymin + m.height as i32);

    let mut out = vec![0u8; (f.w * f.h) as usize];
    for gy in 0..m.height {
        for gx in 0..m.width {
            let (dx, dy) = ((x0 + gx as i32) as u32, (y0 + gy as i32) as u32);
            out[(dy * f.w + dx) as usize] = cov[gy * m.width + gx];
        }
    }
    Some(Raster {
        cov: out,
        w: f.w,
        h: f.h,
    })
}

/// The power symbol, drawn rather than taken from the font, which does not carry it: a ring
/// open at the top with a bar down through the gap, centred in the icons' frame.
fn power_symbol(f: &Frame, px: f32) -> Raster {
    let (cx, cy) = (f.w as f32 / 2.0, f.h as f32 / 2.0 + px * 0.04);
    let radius = px * 0.36;
    let stroke = (px * 0.11).max(1.5);
    let gap = 0.62; // half the opening at the top, in radians
    let mut cov = vec![0u8; (f.w * f.h) as usize];
    for y in 0..f.h {
        for x in 0..f.w {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            // The ring, cut open around straight up.
            let angle = dx.atan2(-dy).abs();
            let ring = if angle > gap {
                (dx.hypot(dy) - radius).abs() - stroke / 2.0
            } else {
                // Distance to the nearer cut end, so the ends are rounded.
                let end = |s: f32| {
                    let (ex, ey) = (s * radius * gap.sin(), -radius * gap.cos());
                    (dx - ex).hypot(dy - ey) - stroke / 2.0
                };
                end(1.0).min(end(-1.0))
            };
            // The bar, from above the ring's top down to near its middle, rounded too.
            let (top, bottom) = (-radius - stroke * 0.3, -radius * 0.05);
            let by = dy.clamp(top, bottom);
            let bar = dx.hypot(dy - by) - stroke / 2.0;
            let d = ring.min(bar);
            let a = (0.5 - d).clamp(0.0, 1.0);
            cov[(y * f.w + x) as usize] = (a * 255.0) as u8;
        }
    }
    Raster {
        cov,
        w: f.w,
        h: f.h,
    }
}

struct Frame {
    w: u32,
    h: u32,
    left: i32,
    top: i32,
}

fn frame(font: &Font, px: f32) -> Frame {
    let (mut left, mut right, mut top, mut bottom) = (i32::MAX, i32::MIN, i32::MIN, i32::MAX);
    for icon in Icon::ALL.into_iter().filter(|i| *i != Icon::Power) {
        let m = font.metrics(icon.glyph(), px);
        left = left.min(m.xmin);
        right = right.max((m.xmin + m.width as i32).max(m.advance_width.ceil() as i32));
        top = top.max(m.ymin + m.height as i32);
        bottom = bottom.min(m.ymin);
    }
    Frame {
        w: (right - left).max(1) as u32,
        h: (top - bottom).max(1) as u32,
        left,
        top,
    }
}

pub fn warm_fonts() {
    let _ = symbols_font();
    let _ = crate::text::label_font();
}

pub(crate) fn symbols_font() -> Option<&'static Font> {
    static FONT: OnceLock<Option<Font>> = OnceLock::new();
    FONT.get_or_init(|| Font::from_bytes(SYMBOLS_TTF, FontSettings::default()).ok())
        .as_ref()
}

/// A symbol in plain white, no halo, for tinting: the WPS screen's Wi-Fi sign.
pub fn icon_plain_face(icon: Icon, px: f32) -> crate::plate::UndoFace {
    let Some(font) = symbols_font() else {
        return crate::plate::UndoFace { rgba: vec![0; 4], w: 1, h: 1 };
    };
    let (m, cov) = font.rasterize(icon.glyph(), px);
    let rgba = cov.iter().flat_map(|&a| [255, 255, 255, a]).collect();
    crate::plate::UndoFace { rgba, w: m.width.max(1) as u32, h: m.height.max(1) as u32 }
}

/// The Deck's spotlight (Roy, sh10): a warm cone of light falling from the top of the screen
/// onto the middle of the shelf, with faint dusty streaks in it, and the pool it throws on the
/// ground round the cart's foot. Drawn as added light over a 640 x 480 screen, behind the carts;
/// built at half resolution, for the ground under the middle cart at SPOT_FOOT.
pub const SPOT_FOOT: f32 = 380.0;

/// Half the spotlight's width at height y: narrow at the lamp, as wide as the cart where it lands.
fn spot_half(y: f32) -> f32 {
    // A straight cone: its sides run from the lamp to the edge of the oval (Roy 2026-11-14).
    36.0 + y * 0.45
}

pub fn spot_beam_face() -> CartFace {
    let foot = SPOT_FOOT;
    const W: f32 = 640.0;
    let (w, h) = (320u32, 240u32);
    let apex = -70.0_f32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for py in 0..h {
        for px in 0..w {
            let (x, y) = ((px as f32 + 0.5) * 2.0, (py as f32 + 0.5) * 2.0);
            let dx = x - W / 2.0;
            // The cone: narrow at the top, as wide as the cart where it lands.
            // Narrower than at first, so it reads as a beam on every wallpaper (Roy, B 2026-11-14).
            let half = spot_half(y);
            let across = (dx.abs() / half).min(2.0);
            // A clear edge, so the cone reads as one shape down to its oval (Roy 2026-11-14).
            let edge = 1.0 - smooth(0.5, 1.08, across);
            // Bright high up, softening as it comes down, gone a little below the foot.
            let down = ((y - apex) / (foot - apex)).clamp(0.0, 1.3);
            let along = (1.0 - 0.3 * down).max(0.0) * (1.0 - smooth(1.0, 1.08, down));
            // Streaks: faint, fanning from the top as dust in the beam does.
            let angle = dx / (y - apex).max(1.0);
            // Clearer rays than at first, as the favourite's glow fans its own (Roy 2026-11-14).
            let streak = 0.9 + 0.07 * (angle * 23.0).sin() + 0.03 * (angle * 41.0 + 1.3).sin();
            let core = 1.0 + 0.35 * (-(dx / (half * 0.35)).powi(2)).exp();
            let beam = 0.68 * edge * along * streak * core;
            // The pool on the ground: a flat warm oval round the foot.
            // A clear oval on the floor, as a stage spot throws (Roy 2026-11-14: less vague).
            // The cone's own width where it meets the floor, so the oval sits under its point.
            let (ex, ey) = (dx / spot_half(foot), (y - foot - 2.0) / 30.0);
            let r = (ex * ex + ey * ey).sqrt();
            let pool = 0.9 * (0.5 + 0.5 * (1.0 - r * r).max(0.0)) * (1.0 - smooth(0.4, 1.12, r));
            let a = (beam + pool).min(1.0);
            if a <= 0.002 {
                continue;
            }
            // Creamy white light with a warm edge, as in Roy's picture (2026-11-14).
            let heart = beam * (-(dx / (half * 0.30)).powi(2)).exp() * (1.0 - 0.6 * down.min(1.0));
            let colours = [
                ([255.0, 238.0, 196.0], heart),
                ([255.0, 212.0, 140.0], beam - heart + pool * 0.6),
                ([255.0, 186.0, 110.0], pool * 0.4),
            ];
            let sum: f32 = colours.iter().map(|c| c.1).sum::<f32>().max(1e-6);
            let o = ((py * w + px) * 4) as usize;
            for ch in 0..3 {
                rgba[o + ch] =
                    (colours.iter().map(|k| k.0[ch] * k.1).sum::<f32>() / sum).round() as u8;
            }
            rgba[o + 3] = (a * 255.0).round() as u8;
        }
    }
    CartFace { rgba, w, h }
}

/// Thin rays for the Deck spotlight (Roy 2026-11-14, after a stage spotlight loop): three
/// sets, each fanning a few thin gold rays from the lamp at random angles inside the cone.
/// The shelf fades them in and out and sways them apart, so the light shimmers and moves.
/// Built at half resolution, drawn over 640 x 480 like the beam.
pub fn spot_rays_face(seed: u32) -> CartFace {
    let foot = SPOT_FOOT;
    const W: f32 = 640.0;
    let (w, h) = (320u32, 240u32);
    let apex = -70.0_f32;
    let mut state = 0x9E37_79B9u32 ^ seed.wrapping_mul(0x85EB_CA6B).wrapping_add(1);
    let mut rnd = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state % 10_000) as f32 / 10_000.0
    };
    // Each ray: its slope (dx per dy, inside the cone), width and strength.
    let rays: Vec<(f32, f32, f32)> = (0..5)
        .map(|_| {
            let slope = (rnd() * 2.0 - 1.0) * 0.26;
            // Soft, hazy rays rather than sharp lines (Roy 2026-11-14).
            let width = 0.030 + rnd() * 0.030;
            let power = 0.55 + rnd() * 0.45;
            (slope, width, power)
        })
        .collect();
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for py in 0..h {
        for px in 0..w {
            let (x, y) = ((px as f32 + 0.5) * 2.0, (py as f32 + 0.5) * 2.0);
            let dx = x - W / 2.0;
            let slope = dx / (y - apex).max(1.0);
            let down = ((y - apex) / (foot - apex)).clamp(0.0, 1.3);
            let along = (1.0 - 0.6 * down).max(0.0) * (1.0 - smooth(0.9, 1.1, down));
            let mut ray = 0.0f32;
            for &(s, wd, pw) in &rays {
                let d = (slope - s) / wd;
                ray += pw * (-d * d).exp();
            }
            let a = (0.32 * ray * along).min(1.0);
            if a <= 0.002 {
                continue;
            }
            let o = ((py * w + px) * 4) as usize;
            let hot = (ray.min(1.0) * (1.0 - down.min(1.0))).clamp(0.0, 1.0);
            let c = [255.0, 212.0 + 26.0 * hot, 140.0 + 56.0 * hot];
            for ch in 0..3 {
                rgba[o + ch] = c[ch].round() as u8;
            }
            rgba[o + 3] = (a * 255.0).round() as u8;
        }
    }
    CartFace { rgba, w, h }
}

/// The dark room round the Deck's spotlight (Roy 2026-11-14, after his own picture): black,
/// deepest at the top and the edges, open where the beam falls and round the pool of light on
/// the floor. The shelf lays it over the wallpaper, behind the carts. Half resolution, drawn
/// over 640 x 480.
pub fn spot_room_face() -> CartFace {
    let foot = SPOT_FOOT;
    let (w, h) = (320u32, 240u32);
    let apex = -70.0_f32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for py in 0..h {
        for px in 0..w {
            let (x, y) = ((px as f32 + 0.5) * 2.0, (py as f32 + 0.5) * 2.0);
            let dx = x - 320.0;
            let half = spot_half(y);
            let cone = 1.0 - smooth(0.7, 1.3, dx.abs() / half);
            let down = ((y - apex) / (foot - apex)).clamp(0.0, 1.0);
            let cone = cone * (0.35 + 0.4 * down) * (1.0 - smooth(1.0, 1.25, (y - apex) / (foot - apex)));
            let (ex, ey) = (dx / (spot_half(foot) * 1.1), (y - foot) / 32.0);
            let pool = 1.0 - smooth(0.7, 1.05, (ex * ex + ey * ey).sqrt());
            let open = cone.max(pool * 0.85);
            // Deeper towards the top and the edges.
            let (vx, vy) = (dx / 360.0, (y - foot) / 420.0);
            let edge = (0.7 + 0.3 * (vx * vx + vy * vy).sqrt().min(1.0)).min(1.0);
            let a = ((1.0 - open) * edge).clamp(0.0, 1.0);
            let o = ((py * w + px) * 4) as usize;
            rgba[o + 3] = (a * 255.0).round() as u8;
        }
    }
    CartFace { rgba, w, h }
}

/// The shadow right under the cart in the Deck's spotlight: a soft dark oval, densest in the
/// middle. White with alpha, tinted black by the shelf.
pub fn spot_shadow_face() -> CartFace {
    // A straight, flat band rather than an oval (Roy 2026-11-14, after his picture): as wide
    // as the cart with only its ends softened, darkest along the top where the cart stands.
    let (w, h) = (128u32, 16u32);
    let mut rgba = vec![255u8; (w * h * 4) as usize];
    for py in 0..h {
        for px in 0..w {
            let ex = ((px as f32 + 0.5) / w as f32 * 2.0 - 1.0).abs();
            let ey = (py as f32 + 0.5) / h as f32;
            let a = (1.0 - smooth(0.86, 1.0, ex)) * (1.0 - smooth(0.25, 1.0, ey));
            rgba[((py * w + px) * 4 + 3) as usize] = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    CartFace { rgba, w, h }
}

fn smooth(a: f32, b: f32, t: f32) -> f32 {
    let t = ((t - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The Recents shelf as a memory (Roy 2026-11-12): over the screen, a soft haze in the middle
/// and a dark, round edge, as when thinking back. Drawn over the shelf (faded by then),
/// under the footer; built at half resolution, drawn over 640 x 480.
pub fn memory_veil_face() -> CartFace {
    let (w, h) = (320u32, 240u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for py in 0..h {
        for px in 0..w {
            let x = (px as f32 + 0.5) / w as f32 * 2.0 - 1.0;
            let y = (py as f32 + 0.5) / h as f32 * 2.0 - 1.0;
            let r = (x * x + (y * 0.92) * (y * 0.92)).sqrt();
            let edge = ((r - 0.55) / 0.7).clamp(0.0, 1.0).powf(1.6);
            // Middle: white haze, a lift of the darks. Edge: black, up to 55%.
            let haze = 0.10 * (1.0 - edge);
            let dark = 0.55 * edge;
            let a = haze + dark;
            let white = if a > 0.0 { haze / a } else { 0.0 };
            let i = ((py * w + px) * 4) as usize;
            let v = (white * 255.0).round() as u8;
            rgba[i..i + 4].copy_from_slice(&[v, v, v, (a * 255.0).round() as u8]);
        }
    }
    CartFace { rgba, w, h }
}
