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
}

impl Icon {
    pub const ALL: [Icon; 12] = [
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


/// A soft golden halo for a favourite cart: a rounded box `cw` x `ch` with `margin` px of glow
/// around it, drawn behind the cart. Built at half resolution and smoothed when stretched.
pub fn glow_face(cw: u32, ch: u32, margin: u32, colour: [u8; 3]) -> CartFace {
    let (w, h) = ((cw + 2 * margin).div_ceil(2), (ch + 2 * margin).div_ceil(2));
    let (m, radius) = (margin as f32, 16.0_f32);
    let (cx, cy) = (m + cw as f32 / 2.0, m + ch as f32 / 2.0);
    let (hw, hh) = (cw as f32 / 2.0 - radius, ch as f32 / 2.0 - radius);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for py in 0..h {
        for px in 0..w {
            let (x, y) = ((px as f32 + 0.5) * 2.0, (py as f32 + 0.5) * 2.0);
            let (qx, qy) = ((x - cx).abs() - hw, (y - cy).abs() - hh);
            let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
            let d = outside + qx.max(qy).min(0.0) - radius;
            let fall = (d.max(0.0) / (m * 0.42)).powi(2);
            let t = ((d - m * 0.6) / (m * 0.4)).clamp(0.0, 1.0);
            let window = 1.0 - t * t * (3.0 - 2.0 * t);
            let a = 0.9 * (-fall).exp() * window;
            let o = ((py * w + px) * 4) as usize;
            rgba[o..o + 4].copy_from_slice(&[colour[0], colour[1], colour[2], (a * 255.0).round() as u8]);
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

struct Frame {
    w: u32,
    h: u32,
    left: i32,
    top: i32,
}

fn frame(font: &Font, px: f32) -> Frame {
    let (mut left, mut right, mut top, mut bottom) = (i32::MAX, i32::MIN, i32::MIN, i32::MAX);
    for icon in Icon::ALL {
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
