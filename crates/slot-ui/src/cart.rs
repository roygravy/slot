use slot_store::{Cart, Outline, Platform};

use crate::art;
use crate::bitmaps::{self, Mould, SgbModel, TvPart, SNES_CART_W};
use crate::board::{BOARD_H, BOARD_W};
use crate::shell::{shell_for, Finish, Shell};
use crate::silhouette::{mould_mask, GbShell};
use crate::text;

pub const CART_W: u32 = 360;
pub const CART_H: u32 = (CART_W * SEATED_H + SEATED_W / 2) / SEATED_W;

pub const GB_CART_W: u32 = 312;

pub const GB_CART_H: u32 = (GB_CART_W * GB_SEATED_H + SEATED_W / 2) / SEATED_W;

/// The GBA cart's label box, as drawn in `assets/gba_neutral.png`: x 48..312, y 50..186
/// of 360x212.
pub const fn label_panel(w: u32, h: u32) -> (u32, u32, u32, u32) {
    (
        (w * 133 + 500) / 1000,
        (h * 236 + 500) / 1000,
        (w * 867 + 500) / 1000,
        (h * 877 + 500) / 1000,
    )
}

pub const LABEL_WELL: [u8; 3] = [0x8d, 0x8d, 0x90];
/// Stickers have rounded corners, like the printed ones: each console's as round as its real
/// stickers', measured on LaunchBox cart photos. The GBA's are the roundest.
const GBA_LABEL_RADIUS: f32 = 11.0;
const GB_LABEL_RADIUS: f32 = 5.0;
const NES_LABEL_RADIUS: f32 = 4.0;
/// The American SNES sticker's corners run round with its well's (radius 7), 2px inside it.
const SNES_LABEL_RADIUS: f32 = 5.0;
const SNES_PAL_LABEL_RADIUS: f32 = 9.0;

pub const LABEL_X: u32 = label_panel(CART_W, CART_H).0;
pub const LABEL_Y: u32 = label_panel(CART_W, CART_H).1;
pub const LABEL_W: u32 = label_panel(CART_W, CART_H).2 - LABEL_X;
pub const LABEL_H: u32 = label_panel(CART_W, CART_H).3 - LABEL_Y;

pub const fn gb_label_panel(w: u32, h: u32) -> (u32, u32, u32, u32) {
    (
        (w * 147 + 500) / 1000,
        (h * 291 + 500) / 1000,
        (w * 854 + 500) / 1000,
        (h * 868 + 500) / 1000,
    )
}

pub const GB_LABEL_X: u32 = gb_label_panel(GB_CART_W, GB_CART_H).0;
pub const GB_LABEL_Y: u32 = gb_label_panel(GB_CART_W, GB_CART_H).1;
pub const GB_LABEL_W: u32 = gb_label_panel(GB_CART_W, GB_CART_H).2 - GB_LABEL_X;
pub const GB_LABEL_H: u32 = gb_label_panel(GB_CART_W, GB_CART_H).3 - GB_LABEL_Y;

/// The Game Boy Color pak's label box: exactly the flat label well drawn in
/// `assets/gbc_neutral.png`, x 45..267, y 110..308 of 312x337. It is the same 223x199
/// well as the Game Boy's (x 44..266, y 95..293), 1px right and 15px lower.
pub const GBC_LABEL_X: u32 = 45;
pub const GBC_LABEL_Y: u32 = 110;
pub const GBC_LABEL_W: u32 = 223;
pub const GBC_LABEL_H: u32 = 199;

const PAD: u32 = 10;
const MAX_LINES: usize = 3;
const MIN_PX: f32 = 10.0;

pub struct CartFace {
    pub rgba: Vec<u8>,
    pub w: u32,
    pub h: u32,
}

struct Spec {
    w: u32,
    h: u32,
    label: (u32, u32, u32, u32),
    /// The sticker wraps over the cart's top edge: square at the top, rounded below.
    square_top: bool,
    /// The sticker's corner radius, as round as the real ones.
    radius: f32,
    mask: &'static [u8],
    mould: Mould,
    paint: &'static [u8],
}

/// The NES label well, in `assets/nes_neutral.png`: x 126..267, y 0..228 of 312x337. The
/// sticker runs over the top edge.
const NES_LABEL: (u32, u32, u32, u32) = (126, 0, 141, 228);
/// The American SNES label well, x 68..292, y 0..102 of 360x231, over the top edge too:
/// measured on Roy's photo of an American cart (Dracula X), 2.2 wide to tall.
const SNES_LABEL: (u32, u32, u32, u32) = (68, 0, 224, 102);
/// The European and Japanese SNES label sits inset, with plastic above it: x 29..306,
/// y 13..110 of 336x228.
const SNES_PAL_LABEL: (u32, u32, u32, u32) = (29, 13, 277, 97);

/// The mould a cart is drawn in: its console's, and for the Game Boys and the SNES the one
/// its region or colour picks, unless the player picked another.
pub fn mould_of(cart: &Cart) -> Mould {
    let outline = cart.shell.map(|c| c.outline);
    match cart.platform {
        Platform::Gba => Mould::Gba,
        Platform::Nes => Mould::Nes,
        Platform::Snes if cart.is_sgb() => sgb_model_of(cart).mould(),
        Platform::Snes => match outline {
            Some(Outline::Notched) => Mould::SnesNtsc,
            Some(Outline::Rounded) => Mould::SnesPal,
            // A sticker fits only the cart it was printed for: a wide PAL/SFC one goes on the
            // flatter cart, a taller NTSC one on the American cart, whatever the game's region.
            _ => match cart.label.as_deref().and_then(art::is_wide_snes_label) {
                Some(true) => Mould::SnesPal,
                Some(false) => Mould::SnesNtsc,
                None => match slot_store::snes::cached_region(&cart.rom) {
                    slot_store::snes::Region::Ntsc => Mould::SnesNtsc,
                    slot_store::snes::Region::Pal => Mould::SnesPal,
                },
            },
        },
        // An original Game Boy game only ever wears the Game Boy cart, even when an older
        // studio pick gave it the Color's shape (Roy 2026-11-06).
        Platform::Gb => Mould::Notched,
        // A Game Boy Color game's studio Model row can give it the Game Boy's shape, which
        // changes only how it looks.
        Platform::Gbc => match outline {
            Some(Outline::Notched) => Mould::Notched,
            Some(Outline::Rounded) => Mould::Rounded,
            _ => gbc_folder_mould(cart),
        },
    }
}

/// The shape a game in the Game Boy Color folder shipped in: only a Color-only game came in
/// the Color's own cart; a dual-mode one (Pokémon Gold, Silver, Yellow, Link's Awakening DX)
/// and an original Game Boy game came in the Game Boy's (Roy 2026-11-07).
pub fn gbc_folder_mould(cart: &Cart) -> Mould {
    match slot_store::gb::cached_header(&cart.rom) {
        Some(h) if h.cgb != 0xc0 => Mould::Notched,
        _ => Mould::Rounded,
    }
}

/// Which Super Game Boy the SGB cart is, as the studio set it: kept in its shell choice, the
/// notched outline for the American one, the rounded for the others, told apart by finish.
/// The European one is the original.
pub fn sgb_model_of(cart: &Cart) -> SgbModel {
    use slot_store::ShellFinish;
    match cart.shell {
        Some(c) if c.outline == Outline::Notched => SgbModel::Ntsc,
        Some(c) if c.outline == Outline::Rounded && c.finish == ShellFinish::Clear => SgbModel::Sgb2,
        Some(c) if c.outline == Outline::Rounded && c.finish == ShellFinish::Solid => SgbModel::Sfc,
        _ => SgbModel::Pal,
    }
}

pub fn gb_shell_of(cart: &Cart) -> Option<GbShell> {
    match mould_of(cart) {
        Mould::Notched => Some(GbShell::Notched),
        Mould::Rounded => Some(GbShell::Rounded),
        _ => None,
    }
}

pub fn label_rect(mould: Mould) -> (f32, f32, f32, f32) {
    let s = spec(mould);
    let (x, y, w, h) = s.label;
    (
        x as f32 / s.w as f32,
        y as f32 / s.h as f32,
        w as f32 / s.w as f32,
        h as f32 / s.h as f32,
    )
}

fn spec(mould: Mould) -> Spec {
    let (w, h) = mould.size();
    let (label, square_top) = match mould {
        Mould::Gba => ((LABEL_X, LABEL_Y, LABEL_W, LABEL_H), false),
        Mould::Notched => ((GB_LABEL_X, GB_LABEL_Y, GB_LABEL_W, GB_LABEL_H), false),
        Mould::Rounded => ((GBC_LABEL_X, GBC_LABEL_Y, GBC_LABEL_W, GBC_LABEL_H), false),
        Mould::Nes => (NES_LABEL, true),
        Mould::SnesNtsc => (SNES_LABEL, true),
        Mould::SnesPal => (SNES_PAL_LABEL, false),
        // The Super Game Boys' stickers are in their art.
        Mould::SgbNtsc | Mould::SgbPal => ((0, 0, w, h), false),
    };
    Spec {
        w,
        h,
        label,
        square_top,
        radius: match mould {
            Mould::Gba => GBA_LABEL_RADIUS,
            Mould::Notched | Mould::Rounded => GB_LABEL_RADIUS,
            Mould::Nes => NES_LABEL_RADIUS,
            Mould::SnesNtsc => SNES_LABEL_RADIUS,
            Mould::SnesPal => SNES_PAL_LABEL_RADIUS,
            Mould::SgbNtsc | Mould::SgbPal => 0.0,
        },
        mask: mould_mask(mould),
        mould,
        paint: match mould {
            Mould::Gba => bitmaps::gba_cart(),
            Mould::Notched => bitmaps::gb_cart(),
            Mould::Rounded => bitmaps::gbc_cart(),
            tv => bitmaps::tv_art(tv, TvPart::Cart),
        },
    }
}

pub const SEATED_W: u32 = 240;
const SEATED_H: u32 = 141;
const GB_SEATED_H: u32 = 259;

/// The cart seated in the slot, as wide as the mouth: each mould shrinks to the same 240.
/// The European SNES cart, smaller than the American one, sits 224 wide in the same slot.
pub fn seated_box(mould: Mould) -> (u32, u32) {
    let (w, h) = mould.size();
    let seated_w = match mould {
        Mould::SnesPal | Mould::SgbPal => SEATED_W * w / SNES_CART_W,
        _ => SEATED_W,
    };
    let full = match mould {
        Mould::SnesPal | Mould::SgbPal => SNES_CART_W,
        _ => w,
    };
    (seated_w, (h * SEATED_W + full / 2) / full)
}

pub fn cart_box(mould: Mould) -> (u32, u32) {
    mould.size()
}

/// A cart's shadow: its silhouette in black.
pub fn cart_shadow(mould: Mould) -> CartFace {
    let (w, h) = mould.size();
    shadow(w, h, mould_mask(mould))
}

/// How far a favourite's glow reaches past the cart's box, and how far its edge light does.
pub const FAV_GLOW_MARGIN: u32 = 60;
pub const FAV_EDGE_MARGIN: u32 = 12;

/// The distance in px from every pixel of a `w` x `h` plane to the nearest set one (two-pass
/// chamfer, good to a fraction of a pixel over these reaches).
fn distances(set: &[bool], w: usize, h: usize) -> Vec<f32> {
    const A: f32 = 1.0;
    const B: f32 = std::f32::consts::SQRT_2;
    let mut d: Vec<f32> = set.iter().map(|&s| if s { 0.0 } else { 1e6 }).collect();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut v = d[i];
            if x > 0 {
                v = v.min(d[i - 1] + A);
            }
            if y > 0 {
                v = v.min(d[i - w] + A);
                if x > 0 {
                    v = v.min(d[i - w - 1] + B);
                }
                if x + 1 < w {
                    v = v.min(d[i - w + 1] + B);
                }
            }
            d[i] = v;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            let mut v = d[i];
            if x + 1 < w {
                v = v.min(d[i + 1] + A);
            }
            if y + 1 < h {
                v = v.min(d[i + w] + A);
                if x + 1 < w {
                    v = v.min(d[i + w + 1] + B);
                }
                if x > 0 {
                    v = v.min(d[i + w - 1] + B);
                }
            }
            d[i] = v;
        }
    }
    d
}

/// The cart's silhouette on a plane `m` px larger all round: how far each pixel lies outside
/// it, and how far inside.
fn mould_distances(mould: Mould, m: usize) -> (Vec<f32>, Vec<f32>, usize, usize) {
    let (cw, ch) = mould.size();
    let mask = mould_mask(mould);
    let (w, h) = (cw as usize + 2 * m, ch as usize + 2 * m);
    let mut inside = vec![false; w * h];
    for y in 0..ch as usize {
        for x in 0..cw as usize {
            inside[(y + m) * w + x + m] = mask[y * cw as usize + x] > 127;
        }
    }
    let outside: Vec<bool> = inside.iter().map(|&s| !s).collect();
    (distances(&inside, w, h), distances(&outside, w, h), w, h)
}

/// A favourite's glow behind the cart (Roy, sh9, after his target picture): light that hugs
/// the cart's own outline, near white-yellow against the edge, gold close by and orange
/// further out, with a fan of soft rays rising from behind the cart. `FAV_GLOW_MARGIN` px
/// larger than the cart's box all round, built at half resolution.
pub fn fav_glow_face(mould: Mould) -> CartFace {
    let m = FAV_GLOW_MARGIN as usize;
    let (out_d, _, fw, fh) = mould_distances(mould, m);
    let (w, h) = (fw.div_ceil(2), fh.div_ceil(2));
    let (cx, cy) = (fw as f32 / 2.0, fh as f32 / 2.0);
    let reach = m as f32;
    let mut rgba = vec![0u8; w * h * 4];
    for py in 0..h {
        for px in 0..w {
            let (x, y) = ((px * 2).min(fw - 1), (py * 2).min(fh - 1));
            let d = out_d[y * fw + x];
            // Three lights laid over each other, the hottest closest.
            let hot = (-d / 3.0).exp();
            let gold = 0.62 * (-d / 9.0).exp();
            let far = 0.42 * (-d / 22.0).exp();
            // Rays: a fan about the cart's middle, strongest straight up, gone at the sides.
            let (dx, dy) = (x as f32 - cx, y as f32 - cy);
            let ang = dx.atan2(-dy);
            let up = (ang.cos()).max(0.0).powf(0.8);
            let fan = (0.5 + 0.5 * (ang * 17.0 + 0.6).sin()) * (0.5 + 0.5 * (ang * 29.0 + 2.1).sin());
            let fan = 0.25 + 0.75 * fan.powf(1.5);
            let rays = 0.7 * up * fan * (-d / 26.0).exp() * (1.0 - (-d / 6.0).exp());
            let a = hot + gold + far + rays;
            // Nothing at the face's own edge.
            let fade = ((reach - d) / (reach * 0.6)).clamp(0.0, 1.0);
            let fade = fade * fade * (3.0 - 2.0 * fade);
            let a = (a.min(1.0) * fade).clamp(0.0, 1.0);
            let colours = [
                ([255.0, 244.0, 190.0], hot),
                ([255.0, 200.0, 84.0], gold + rays),
                ([255.0, 150.0, 48.0], far),
            ];
            let sum: f32 = colours.iter().map(|c| c.1).sum::<f32>().max(1e-6);
            let o = (py * w + px) * 4;
            for c in 0..3 {
                rgba[o + c] = (colours.iter().map(|k| k.0[c] * k.1).sum::<f32>() / sum).round() as u8;
            }
            rgba[o + 3] = (a * 255.0).round() as u8;
        }
    }
    CartFace { rgba, w: w as u32, h: h as u32 }
}

/// A favourite's edge light, over the cart: a fine bright line along its outline, hottest
/// along the top, and a warm wash a few pixels into the plastic, as if the glow behind it
/// wraps round the edge. `FAV_EDGE_MARGIN` px larger than the cart's box all round.
pub fn fav_edge_face(mould: Mould) -> CartFace {
    let m = FAV_EDGE_MARGIN as usize;
    let (out_d, in_d, w, h) = mould_distances(mould, m);
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            // Signed: negative inside the cart.
            let d = if out_d[i] > 0.0 { out_d[i] - 0.5 } else { 0.5 - in_d[i] };
            let top = 1.0 - 0.45 * (y as f32 / h as f32);
            let line = (-(d / 1.3).powi(2)).exp() * top;
            let wash = if d < 0.0 { 0.42 * (d / 5.0).exp() * top } else { (-(d / 2.0).powi(2)).exp() * 0.5 };
            let a = (line + wash * (1.0 - line)).clamp(0.0, 1.0);
            if a < 0.004 {
                continue;
            }
            let k = line / (line + wash).max(1e-6);
            let (hot, warm) = ([255.0, 247.0, 205.0], [255.0, 196.0, 84.0]);
            let o = i * 4;
            for c in 0..3 {
                rgba[o + c] = (warm[c] + (hot[c] - warm[c]) * k).round() as u8;
            }
            rgba[o + 3] = (a * 255.0).round() as u8;
        }
    }
    CartFace { rgba, w: w as u32, h: h as u32 }
}

/// The Deck spotlight where it lands on the cart (Roy, sh10): light from straight above, so it
/// follows the cart's own silhouette. A fine hot line along every edge that faces up, a warm
/// wash a little way down into the plastic under it, and a soft halo just above the top where
/// the beam scatters; strongest in the beam's middle, in the favourite glow's colours.
/// `SPOT_EDGE_MARGIN` px larger than the cart's box all round.
pub fn spot_edge_face(mould: Mould) -> CartFace {
    let m = SPOT_EDGE_MARGIN as usize;
    let (cw, ch) = mould.size();
    let mask = mould_mask(mould);
    let (w, h) = (cw as usize + 2 * m, ch as usize + 2 * m);
    let inside = |x: usize, y: usize| {
        x >= m && y >= m && x < m + cw as usize && y < m + ch as usize
            && mask[(y - m) * cw as usize + (x - m)] > 127
    };
    let mut rgba = vec![0u8; w * h * 4];
    for x in 0..w {
        // Across the cart: the beam's middle brightest, the sides softer.
        let u = (x as f32 - w as f32 / 2.0) / (cw as f32 / 2.0);
        let across = (1.0 - 0.45 * u * u).max(0.0);
        // Down the column: how far below the lit edge (inside), or above the cart (outside).
        let mut lit_at: Option<usize> = None;
        let first = (0..h).find(|&y| inside(x, y));
        for y in 0..h {
            let (line, wash, halo) = if inside(x, y) {
                let top = *lit_at.get_or_insert(y);
                let t = (y - top) as f32;
                // Only the first surface the light meets: a lower ledge lies in the cart's
                // shade, a little lit.
                let k = if Some(top) == first { 1.0 } else { 0.35 };
                (k * (-(t / 1.8).powi(2)).exp(), k * 0.42 * (-t / 12.0).exp(), 0.0)
            } else {
                lit_at = None;
                match first {
                    Some(f) if y < f => (0.0, 0.0, 0.55 * (-((f - y) as f32) / 5.0).exp()),
                    _ => (0.0, 0.0, 0.0),
                }
            };
            let a = ((line + wash * (1.0 - line) + halo) * across).clamp(0.0, 1.0);
            if a < 0.004 {
                continue;
            }
            let k = line / (line + wash + halo).max(1e-6);
            let (hot, warm) = ([255.0, 244.0, 190.0], [255.0, 200.0, 84.0]);
            let o = (y * w + x) * 4;
            for c in 0..3 {
                rgba[o + c] = (warm[c] + (hot[c] - warm[c]) * k).round() as u8;
            }
            rgba[o + 3] = (a * 255.0).round() as u8;
        }
    }
    CartFace { rgba, w: w as u32, h: h as u32 }
}

pub const SPOT_EDGE_MARGIN: u32 = 14;

/// How far a pak's drop shadow on a shallow slot spreads past its box, in the cart's own pixels.
pub const PERCH_MARGIN: u32 = 36;

/// The soft drop shadow a pak casts on a shallow slot (Pocket, Light, Game Boy Color): its own
/// silhouette, blurred and fallen right and down as if lit from the top left, with the pak itself
/// cut out so it lands only on the slot around it (Roy 2026-11-05: "gewoon een zachte
/// slagschaduw"). `PERCH_MARGIN` px larger than the cart's box on every side.
pub fn perch_shadow_face(mould: Mould) -> CartFace {
    let (cw, ch) = mould.size();
    let mask = mould_mask(mould);
    let m = PERCH_MARGIN as usize;
    let (w, h) = (cw as usize + 2 * m, ch as usize + 2 * m);
    // the silhouette, fallen right and down
    const DX: usize = 10;
    const DY: usize = 12;
    let mut a = vec![0f32; w * h];
    for y in 0..ch as usize {
        for x in 0..cw as usize {
            a[(y + m + DY) * w + x + m + DX] = mask[y * cw as usize + x] as f32 / 255.0;
        }
    }
    // three box blurs: close to a gaussian
    for _ in 0..3 {
        box_blur(&mut a, w, h, 7);
    }
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let (cx, cy) = (x as i64 - m as i64, y as i64 - m as i64);
            // The silhouette runs a few px past the pak's visible side (clear pixels in the face
            // art): cut out a slightly shrunk one, or a bright unshaded line shows beside the pak.
            let at = |x: i64, y: i64| {
                if x >= 0 && y >= 0 && x < cw as i64 && y < ch as i64 {
                    mask[y as usize * cw as usize + x as usize] as f32 / 255.0
                } else {
                    0.0
                }
            };
            const IN: i64 = 4;
            let inside = at(cx, cy).min(at(cx - IN, cy)).min(at(cx + IN, cy)).min(at(cx, cy - IN));
            let v = a[y * w + x] * (1.0 - inside);
            rgba.extend_from_slice(&[0, 0, 0, (v * 255.0).round() as u8]);
        }
    }
    CartFace { rgba, w: w as u32, h: h as u32 }
}

fn box_blur(a: &mut [f32], w: usize, h: usize, r: usize) {
    let n = (2 * r + 1) as f32;
    let mut tmp = vec![0f32; a.len()];
    for y in 0..h {
        for x in 0..w {
            let (lo, hi) = (x.saturating_sub(r), (x + r).min(w - 1));
            tmp[y * w + x] = a[y * w + lo..=y * w + hi].iter().sum::<f32>() / n;
        }
    }
    for y in 0..h {
        for x in 0..w {
            let (lo, hi) = (y.saturating_sub(r), (y + r).min(h - 1));
            a[y * w + x] = (lo..=hi).map(|yy| tmp[yy * w + x]).sum::<f32>() / n;
        }
    }
}

fn shadow(w: u32, h: u32, mask: &[u8]) -> CartFace {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for cover in mask {
        rgba.extend_from_slice(&[0, 0, 0, *cover]);
    }
    CartFace { rgba, w, h }
}

pub fn cart_face(cart: &Cart) -> CartFace {
    let (lw, lh) = label_size(cart);
    let art = cart.label.as_deref().and_then(|p| art::cover(p, lw, lh));
    cart_face_with(cart, art)
}

pub fn is_clear(cart: &Cart) -> bool {
    // A Super Game Boy 2's clear plastic is in its finished art already.
    !cart.is_sgb() && shell_for(cart).finish.clear()
}

/// Whether the shelf draws this cart's coloured interior under its plastic: any clear cart.
pub fn shows_interior(cart: &Cart) -> bool {
    is_clear(cart)
}

/// The back plastic, with the game's colour blended onto it by the same overlay as the shells, at
/// the clear shells' opacity. A GBA cart uses `gba_interior.png`, a Game Boy pak `gb_interior.png`, and a Game
/// Boy Color pak `gbc_interior.png`. Same size as that cart's board and face.
pub fn cart_interior(cart: &Cart) -> CartFace {
    let colour = shell_for(cart).colour;
    let mould = mould_of(cart);
    let (src, w, h, mask) = match mould {
        Mould::Gba => (bitmaps::gba_interior(), BOARD_W, BOARD_H, None),
        mould => {
            let s = spec(mould);
            let src = match mould {
                Mould::Notched => bitmaps::gb_interior(),
                Mould::Rounded => bitmaps::gbc_interior(),
                tv => bitmaps::tv_art(tv, TvPart::Interior),
            };
            (src, s.w, s.h, Some(s.mask))
        }
    };
    let mut rgba = src.to_vec();
    for (i, px) in rgba.chunks_exact_mut(4).enumerate() {
        if let Some(mask) = mask {
            px[3] = ((px[3] as u32 * mask[i] as u32 + 127) / 255) as u8;
        }
        if px[3] == 0 {
            continue;
        }
        for c in 0..3 {
            px[c] = overlay(px[c], colour[c]);
        }
        px[3] = ((u32::from(px[3]) * u32::from(CLEAR_ALPHA) + 127) / 255) as u8;
    }
    CartFace { rgba, w, h }
}

/// The board a clear cart's plastic is drawn over, between the back and front shells. A GBA cart
/// gets `board.png`. A pak gets `gb_board.png`, cut to the notched or rounded outline.
pub fn cart_backing(mould: Mould) -> CartFace {
    if mould == Mould::Gba {
        return CartFace {
            rgba: bitmaps::board().to_vec(),
            w: BOARD_W,
            h: BOARD_H,
        };
    }
    let s = spec(mould);
    let mut rgba = match mould.tv() {
        true => bitmaps::tv_art(mould, TvPart::Board).to_vec(),
        false => bitmaps::gb_board().to_vec(),
    };
    for (px, cover) in rgba.chunks_exact_mut(4).zip(s.mask) {
        px[3] = ((px[3] as u32 * *cover as u32 + 127) / 255) as u8;
    }
    CartFace {
        rgba,
        w: s.w,
        h: s.h,
    }
}

pub fn label_size(cart: &Cart) -> (u32, u32) {
    let (_, _, lw, lh) = spec(mould_of(cart)).label;
    (lw, lh)
}

pub fn cart_face_with(cart: &Cart, art: Option<Vec<u8>>) -> CartFace {
    cart_face_aged(cart, art, 0, 0, 0)
}

/// The cart's face with `dust` and `wear` (0..=3 each) baked in: plastic and sticker wear first,
/// then the dust film over everything. The same seed always gives the same marks.
pub fn cart_face_aged(cart: &Cart, art: Option<Vec<u8>>, dust: u8, wear: u8, seed: u64) -> CartFace {
    let mould = mould_of(cart);
    if mould.sgb() {
        let (w, h) = mould.size();
        return CartFace {
            rgba: bitmaps::sgb_face(sgb_model_of(cart)).to_vec(),
            w,
            h,
        };
    }
    let s = spec(mould);
    let shell = shell_for(cart);
    let mut face = shell_face(&s, &shell);
    // The sticker's rounded corners show the recess under it, in the shell's own plastic.
    plastic_label_bed(&s, &shell, &mut face);
    sparkle(&s, shell.finish, &mut face);
    let paper = art.is_some();
    let label = art.unwrap_or_else(|| generated_label(&s, &shell, &label_text(cart), cart.marker));
    paste_label(&s, &mut face, &label, paper);
    clip_to_silhouette(&s, &mut face);
    if wear > 0 {
        crate::wear::wear_plastic(&mut face, wear, seed);
        crate::wear::wear_label(&mut face, sticker_rect(&s), wear, seed);
    }
    crate::dust::dust_face(&mut face, dust, seed);
    face
}

fn clip_to_silhouette(s: &Spec, face: &mut CartFace) {
    for (px, cover) in face.rgba.chunks_exact_mut(4).zip(s.mask) {
        px[3] = ((px[3] as u32 * *cover as u32 + 127) / 255) as u8;
    }
}

pub fn label_colour(title: &str) -> [u8; 3] {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in title.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hsv_to_rgb((h % 360) as f32, 0.52, 0.74)
}

pub fn label_text(cart: &Cart) -> String {
    clean_label(&cart.stem)
}

pub fn clean_label(stem: &str) -> String {
    let mut bare = String::with_capacity(stem.len());
    let mut depth = 0u32;
    for ch in stem.chars() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => bare.push(ch),
            _ => {}
        }
    }

    let mut out = String::with_capacity(bare.len());
    for word in bare.split_whitespace().filter(|w| *w != "-") {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    if out.is_empty() {
        stem.to_string()
    } else {
        out
    }
}

pub fn label_tags(stem: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0u32;
    let mut cur = String::new();
    for ch in stem.chars() {
        match ch {
            '(' | '[' => {
                depth += 1;
                if depth == 1 {
                    cur.clear();
                    continue;
                }
            }
            ')' | ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let t = cur.trim();
                    if !t.is_empty() {
                        out.push(t.to_string());
                    }
                    continue;
                }
            }
            _ => {}
        }
        if depth >= 1 {
            cur.push(ch);
        }
    }
    out
}

fn luma(px: &[u8]) -> u32 {
    (px[0] as u32 * 77 + px[1] as u32 * 150 + px[2] as u32 * 29) >> 8
}

fn shade_of(paint: &[u8], w: u32, h: u32, label: (u32, u32, u32, u32)) -> Vec<u32> {
    let (lx, ly, lw, lh) = label;
    let mut sum = 0u64;
    let mut n = 0u64;
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            if paint[i + 3] < 128 {
                continue;
            }
            let on_label = (lx..lx + lw).contains(&x) && (ly..ly + lh).contains(&y);
            if on_label {
                continue;
            }
            sum += u64::from(luma(&paint[i..]));
            n += 1;
        }
    }
    let mean = (sum / n.max(1)).max(1) as u32;
    paint
        .chunks_exact(4)
        .map(|px| (luma(px) * 256 / mean).clamp(176, 256))
        .collect()
}

/// The neutral shell asset for a mould: 50% grey plastic with the console name embossed on the
/// shoulder and the grey label box. `clear` makes the see-through variant, which is its own
/// asset with a rim and a sheen. The game's colour is blended on by `overlay`.
pub fn neutral_shell(mould: Mould, clear: bool) -> Vec<u8> {
    let s = spec(mould);
    assert_eq!(
        s.paint.len(),
        (s.w * s.h * 4) as usize,
        "cartridge template is not {}x{}",
        s.w,
        s.h
    );
    let shade = shade_of(s.paint, s.w, s.h, s.label);
    let mut rgba = vec![0u8; (s.w * s.h * 4) as usize];
    let inside = |x: i64, y: i64| {
        (0..i64::from(s.w)).contains(&x)
            && (0..i64::from(s.h)).contains(&y)
            && s.mask[(y * i64::from(s.w) + x) as usize] >= 128
    };
    for (i, (dst, shade)) in rgba.chunks_exact_mut(4).zip(shade).enumerate() {
        let (x, y) = ((i as u32 % s.w) as i64, (i as u32 / s.w) as i64);
        let grey = if clear {
            // Glassy plastic: flat 50% grey body, a darker rim where the walls are thick, and a
            // faint lighter diagonal sheen across the face.
            let rim = (1..=CLEAR_RIM as i64).find(|d| {
                ![(x - d, y), (x + d, y), (x, y - d), (x, y + d)]
                    .iter()
                    .all(|&(px, py)| inside(px, py))
            });
            let sheen = (x + y - i64::from(s.w) / 3).abs() < i64::from(s.w) / 14;
            match (rim, sheen) {
                (Some(_), _) => NEUTRAL - 28,
                (None, true) => NEUTRAL + 24,
                (None, false) => NEUTRAL,
            }
        } else {
            // The mould's shading (256 is unshaded) falls below 50% grey.
            NEUTRAL - ((256 - shade.min(256)) / 2) as u8
        };
        dst.copy_from_slice(&[grey, grey, grey, 255]);
    }
    let mut face = CartFace {
        rgba,
        w: s.w,
        h: s.h,
    };
    clip_to_silhouette(&s, &mut face);
    dress(&s, &mut face.rgba);
    face.rgba
}

fn dress(s: &Spec, rgba: &mut [u8]) {
    let (x0, y0, lw, lh) = s.label;
    for y in y0..y0 + lh {
        for x in x0..x0 + lw {
            let i = ((y * s.w + x) * 4) as usize;
            if rgba[i + 3] == 0 {
                continue;
            }
            rgba[i..i + 3].copy_from_slice(&LABEL_WELL);
        }
    }
    let title = match s.mould {
        Mould::Gba => "GAME BOY ADVANCE",
        Mould::Notched => "GAME BOY",
        Mould::Rounded => "GAME BOY COLOR",
        // The TV carts carry no name above a label that runs to their top edge.
        Mould::Nes | Mould::SnesNtsc | Mould::SnesPal | Mould::SgbNtsc | Mould::SgbPal => return,
    };
    stamp_console(rgba, s.w, s.h, title, y0);
}

fn stamp_console(rgba: &mut [u8], w: u32, h: u32, title: &str, label_top: u32) {
    let Some(font) = text::label_font() else {
        return;
    };
    let band = label_top.max(1);
    let layout = text::fit_box(font, title, w as f32 * 0.72, band as f32, 2, 26.0, 9.0);
    let cov = text::coverage(w, band, &layout);
    let mut ink_top = band;
    let mut ink_bottom = 0u32;
    for y in 0..band {
        let row = &cov[(y * w) as usize..(y * w + w) as usize];
        if row.iter().any(|a| *a > 0) {
            ink_top = ink_top.min(y);
            ink_bottom = y;
        }
    }
    if ink_bottom < ink_top {
        return;
    }
    let text_h = ink_bottom - ink_top + 1;
    let dest_top = label_top.saturating_sub(3 + text_h);
    for y in ink_top..=ink_bottom {
        let dy = dest_top + (y - ink_top);
        if dy >= h {
            continue;
        }
        for x in 0..w {
            let a = cov[(y * w + x) as usize] as u32;
            if a == 0 {
                continue;
            }
            let i = ((dy * w + x) * 4) as usize;
            if rgba[i + 3] == 0 {
                continue;
            }
            let base = [rgba[i], rgba[i + 1], rgba[i + 2]];
            let ink = relief(base);
            for c in 0..3 {
                rgba[i + c] = ((ink[c] as u32 * a + base[c] as u32 * (255 - a)) / 255) as u8;
            }
        }
    }
}

/// The embossed name is a darker grey than the plastic around it; the overlay then darkens
/// whatever colour the game sets by the same proportion.
fn relief(px: [u8; 3]) -> [u8; 3] {
    px.map(|c| (c as u32 * 3 / 5) as u8)
}

const CLEAR_RIM: u32 = 5;

/// How much of a clear shell is opaque: 80%, the board a little more in view again (Roy 2026-11-13;
/// 88% on 2026-11-12, st7).
const CLEAR_ALPHA: u8 = 204;

/// The neutral shell's flat plastic: 50% grey, the level that leaves the game's colour as it is.
const NEUTRAL: u8 = 128;

/// Photoshop's Overlay blend of one channel: the neutral shell is the base layer and the game's
/// colour the blend layer. A base of 50% grey gives the colour exactly, a darker base darkens it
/// toward black and a lighter base lightens it toward white, so the shell's highlights, shadows
/// and embossed console name all follow whatever colour the game sets.
fn overlay(base: u8, colour: u8) -> u8 {
    let (base, colour) = (i32::from(base), i32::from(colour));
    let out = if base >= i32::from(NEUTRAL) {
        colour + ((255 - colour) * (base - i32::from(NEUTRAL)) + 63) / 127
    } else {
        colour - (colour * (i32::from(NEUTRAL) - base) + 64) / i32::from(NEUTRAL)
    };
    out.clamp(0, 255) as u8
}

/// The SNES carts' security screws are steel whatever the plastic's colour (Roy 2026-11-15).
const SCREW_STEEL: [u8; 3] = [0xa4, 0xa6, 0xaa];

/// How much of a pixel is the head of one of the SNES carts' screws, 0 to 255: where
/// tools/carts/snes_render.py and snes_pal_render.py put them (gamebit), in asset pixels.
fn screw_cover(mould: Mould, x: u32, y: u32) -> u32 {
    let (heads, r): (&[(f32, f32)], f32) = match mould {
        Mould::SnesNtsc => (&[(31.5, 211.0), (328.5, 211.0)], 4.6),
        Mould::SnesPal => (&[(21.5, 211.0), (314.5, 211.0)], 4.6),
        _ => return 0,
    };
    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
    heads
        .iter()
        .map(|&(cx, cy)| ((r + 0.5 - ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()).clamp(0.0, 1.0) * 255.0) as u32)
        .max()
        .unwrap_or(0)
}

fn shell_face(s: &Spec, shell: &Shell) -> CartFace {
    let clear = shell.finish.clear();
    let src = bitmaps::shell_asset(s.mould, clear);
    assert_eq!(
        src.len(),
        (s.w * s.h * 4) as usize,
        "shell bitmap is not {}x{}",
        s.w,
        s.h
    );
    let mut rgba = src.to_vec();
    // The neutral asset is blended with the game's colour everywhere but the label box, which
    // stays the grey of the asset. A clear shell is its own asset with real transparency: what the
    // compositor drew underneath (the board) shows through and stays put while the shell moves.
    // The label box stays opaque.
    let (lx, ly, lw, lh) = s.label;
    for y in 0..s.h {
        for x in 0..s.w {
            let on_label = (lx..lx + lw).contains(&x) && (ly..ly + lh).contains(&y);
            let i = ((y * s.w + x) * 4) as usize;
            if !on_label {
                let metal = screw_cover(s.mould, x, y);
                for c in 0..3 {
                    let plastic = overlay(rgba[i + c], shell.colour[c]) as u32;
                    let steel = overlay(rgba[i + c], SCREW_STEEL[c]) as u32;
                    rgba[i + c] = ((steel * metal + plastic * (255 - metal) + 127) / 255) as u8;
                }
            }
            // The asset's own alpha is kept: its transparent corners and sides hold stray RGB
            // (often white), and only the alpha stops that being drawn.
            let own = u32::from(rgba[i + 3]);
            rgba[i + 3] = if on_label {
                255
            } else if clear {
                ((own * u32::from(CLEAR_ALPHA) + 127) / 255) as u8
            } else {
                own as u8
            };
        }
    }
    if shell.finish == Finish::Metallic {
        metal(&mut rgba, src, s, shell.colour);
    }
    CartFace {
        rgba,
        w: s.w,
        h: s.h,
    }
}

/// Gold paint, as on Nintendo's gold NES carts: the moulding's light and shade pushed harder,
/// a broad soft highlight running down the cart at a slant with a duller band after it, and a
/// faint fine grain in the paint. The label box is left as it is.
/// METAL is how hard all that is pushed over the plain moulding (1.0 was the first, too strong
/// for Roy, 2026-11-07).
const METAL: f32 = 0.6;

fn metal(rgba: &mut [u8], neutral: &[u8], s: &Spec, colour: [u8; 3]) {
    let k = METAL;
    let (lx, ly, lw, lh) = s.label;
    let shine = lerp(colour, [0xff; 3], 150, 255);
    for y in 0..s.h {
        for x in 0..s.w {
            if (lx..lx + lw).contains(&x) && (ly..ly + lh).contains(&y) {
                continue;
            }
            let i = ((y * s.w + x) * 4) as usize;
            if rgba[i + 3] == 0 {
                continue;
            }
            let shade = (neutral[i..i + 3].iter().map(|&c| c as f32).sum::<f32>() / 3.0 / 128.0)
                .clamp(0.0, 2.0);
            let lit = shade + (0.12 + 0.95 * shade.powf(2.2) - shade) * k;
            let u = x as f32 / s.w as f32 * 0.75 + y as f32 / s.h as f32 * 0.25;
            let band = |at: f32, width: f32| (-((u - at) / width).powi(2)).exp();
            let highlight = (0.75 * band(0.30, 0.08) + 0.30 * band(0.56, 0.06)) * (0.4 + 0.6 * k);
            let dull = 1.0 - (0.32 * band(0.76, 0.10) + 0.15 * band(0.12, 0.06)) * k;
            let grain = 1.0 + ((fleck_noise(x, y) % 9) as f32 - 4.0) * 0.006;
            for c in 0..3 {
                let body = colour[c] as f32 * lit * dull * grain;
                let v = body + (shine[c] as f32 - body).max(0.0) * highlight * shade.min(1.2);
                rgba[i + c] = v.clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// Glitter's flecks, over the shell and its bare well alike. In clear plastic they are sparse
/// white specks; in solid glitter plastic (Pokémon Gold and Silver) they are dense and fine,
/// most a lighter shade of the plastic catching the light, some a darker one.
fn sparkle(s: &Spec, finish: Finish, face: &mut CartFace) {
    for y in 0..s.h {
        for x in 0..s.w {
            let i = ((y * s.w + x) * 4) as usize;
            if face.rgba[i + 3] == 0 {
                continue;
            }
            let body = [face.rgba[i], face.rgba[i + 1], face.rgba[i + 2]];
            match finish {
                Finish::Glitter if fleck(x, y) => {
                    face.rgba[i..i + 3].copy_from_slice(&lerp(body, [0xff; 3], 100, 255));
                    face.rgba[i + 3] = 255;
                }
                Finish::SolidGlitter => {
                    let h = fleck_noise(x, y) % 1000;
                    let to = match h {
                        0..=59 => lerp(body, [0xff; 3], 40 + (h % 5) * 25, 255),
                        60..=89 => lerp(body, [0; 3], 50, 255),
                        _ => continue,
                    };
                    face.rgba[i..i + 3].copy_from_slice(&to);
                }
                _ => {}
            }
        }
    }
}

fn fleck_noise(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x9e37_79b9) ^ y.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h.wrapping_mul(0x2c1b_3c6d) >> 7
}

fn fleck(x: u32, y: u32) -> bool {
    let mut h = x.wrapping_mul(0x9e37_79b9) ^ y.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h.is_multiple_of(100)
}

fn lerp(a: [u8; 3], b: [u8; 3], num: u32, den: u32) -> [u8; 3] {
    let mut out = [0u8; 3];
    for c in 0..3 {
        out[c] = ((a[c] as u32 * (den - num) + b[c] as u32 * num) / den) as u8;
    }
    out
}

/// With no sticker, the label recess is bare plastic: the shell's art under the label, its
/// well's walls and floor and round corners, in the shell's own colour, with the mark the
/// moulding leaves there. A clear shell's well is as see-through as the rest of it.
fn plastic_label_bed(s: &Spec, shell: &Shell, face: &mut CartFace) {
    let (lx, ly, lw, lh) = s.label;
    let on_label = |x: u32, y: u32| (lx..lx + lw).contains(&x) && (ly..ly + lh).contains(&y);
    // The plastic's own opacity: the most common one on the shell, so the see-through bits of
    // its art (vents, gaps) never thin out a solid cart's recess (an average did: 97%).
    let mut count = [0u32; 256];
    for y in 0..s.h {
        for x in 0..s.w {
            let i = (y * s.w + x) as usize;
            if on_label(x, y) || s.mask[i] < 250 {
                continue;
            }
            count[face.rgba[i * 4 + 3] as usize] += 1;
        }
    }
    let Some((alpha, _)) = count.iter().enumerate().filter(|(_, n)| **n > 0).max_by_key(|(_, n)| **n) else {
        return;
    };
    let alpha = alpha as u8;
    // The round sink mark the moulding leaves in a cart's well, where it was screwed or
    // pinned shut, as a fraction of the well and of the cart's width. The Game Boy Color's is
    // a small real hole; the SNES carts show none.
    let mark = match s.mould {
        Mould::Notched => Some((
            0.5,
            0.522 * s.h as f32 / lh as f32 - ly as f32 / lh as f32,
            8.0 / 312.0,
            Mark::Sink(0.30),
        )),
        Mould::Gba => Some((0.48, 0.44, 0.026, Mark::Sink(0.40))),
        Mould::Rounded => Some((0.5, 0.65, 0.012, Mark::Hole)),
        // The NES cart's art has its own, from the photo it was drawn from.
        Mould::Nes | Mould::SnesNtsc | Mould::SnesPal | Mould::SgbNtsc | Mould::SgbPal => None,
    };
    let (dimple_x, dimple_y, dimple_r, kind) = match mark {
        Some((fx, fy, fr, kind)) => (lw as f32 * fx, lh as f32 * fy, s.w as f32 * fr, kind),
        None => (-1e4, -1e4, 1.0, Mark::Sink(0.0)),
    };
    for y in 0..lh {
        for x in 0..lw {
            let (dx, dy) = (x as f32 - dimple_x, y as f32 - dimple_y);
            let dimple = match kind {
                Mark::Sink(strength) => screw_dimple(dx, dy, dimple_r, strength),
                Mark::Hole => screw_hole(dx, dy, dimple_r),
            };
            let d = (((y + ly) * s.w + x + lx) * 4) as usize;
            for c in 0..3 {
                let px = overlay(face.rgba[d + c], shell.colour[c]) as f32 * dimple;
                face.rgba[d + c] = px.clamp(0.0, 255.0) as u8;
            }
            face.rgba[d + 3] = alpha;
        }
    }
}

/// A Game Boy pak's bare label recess shows a small round sink mark from the moulding: a
/// shallow dish with a soft rim and a tiny bump in its middle. Returns how much a pixel at
/// (dx, dy) from its centre is lit, lit from above like the rest of the shell, averaged over
/// 4x4 samples.
fn screw_dimple(dx: f32, dy: f32, radius: f32, strength: f32) -> f32 {
    if dx.abs() > radius * 1.5 || dy.abs() > radius * 1.5 {
        return 1.0;
    }
    let profile = |t: f32| {
        let dish = if t < 1.0 {
            -(1.0 - t * t).max(0.0).powf(0.6) + 0.25 * (-(t / 0.28).powi(2)).exp()
        } else {
            0.0
        };
        dish + 0.18 * (-((t - 1.02) / 0.10).powi(2)).exp()
    };
    let depth = radius * 0.22;
    let (lx, ly, lz) = {
        let (x, y, z) = (-0.25f32, -0.75f32, 0.62f32);
        let n = (x * x + y * y + z * z).sqrt();
        (x / n, y / n, z / n)
    };
    let mut sum = 0.0;
    for sy in 0..4 {
        for sx in 0..4 {
            let px = dx + (sx as f32 + 0.5) / 4.0 - 0.5;
            let py = dy + (sy as f32 + 0.5) / 4.0 - 0.5;
            let r = (px * px + py * py).sqrt().max(1e-3);
            let t = r / radius;
            let slope = (profile(t + 0.02) - profile(t - 0.02)) / 0.04 * depth / radius;
            let (gx, gy) = (slope * px / r, slope * py / r);
            let lit = (-gx * lx - gy * ly + lz) / (gx * gx + gy * gy + 1.0).sqrt();
            // Faint: it catches the light, it does not draw the eye.
            sum += 1.0 + (lit / lz - 1.0) * strength;
        }
    }
    sum / 16.0
}

/// A Game Boy Color pak's well has a small hole right through: dark inside, its lower rim
/// catching the light. Returns how much a pixel at (dx, dy) from its centre is lit, averaged
/// over 4x4 samples.
fn screw_hole(dx: f32, dy: f32, radius: f32) -> f32 {
    if dx.abs() > radius * 2.0 || dy.abs() > radius * 2.0 {
        return 1.0;
    }
    let mut sum = 0.0;
    for sy in 0..4 {
        for sx in 0..4 {
            let px = dx + (sx as f32 + 0.5) / 4.0 - 0.5;
            let py = dy + (sy as f32 + 0.5) / 4.0 - 0.5;
            let t = (px * px + py * py).sqrt() / radius;
            sum += if t < 1.0 {
                0.18
            } else if t < 1.35 && py > 0.0 {
                1.0 + 0.10 * (py / (t * radius))
            } else {
                1.0
            };
        }
    }
    sum / 16.0
}

#[derive(Clone, Copy)]
enum Mark {
    /// A shallow dish, this lit.
    Sink(f32),
    Hole,
}

/// A cart with no label art has no sticker at all: its title is written straight onto the
/// plastic in marker, dark on a Game Boy pak and white on a GBA cart or any near-black pak,
/// unless another marker was picked for it.
fn generated_label(s: &Spec, shell: &Shell, title: &str, marker: Option<[u8; 3]>) -> Vec<u8> {
    let (_, _, lw, lh) = s.label;
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    for b in title.as_bytes() {
        seed = (seed ^ *b as u64).wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut rgba = vec![0u8; (lw * lh * 4) as usize];
    let Some(font) = text::marker_font() else {
        return rgba;
    };
    let room_w = (lw - 2 * PAD) as f32;
    let room_h = (lh - 2 * PAD) as f32;
    let max_px = match s.mould {
        Mould::Gba | Mould::SnesNtsc | Mould::SnesPal | Mould::SgbNtsc | Mould::SgbPal => {
            lh as f32 / 2.1
        }
        Mould::Notched | Mould::Rounded | Mould::Nes => room_h / 2.6,
    };
    let mut layout = text::fit_box(font, title, room_w, room_h, MAX_LINES, max_px, MIN_PX);
    layout.tracking = 0.0;
    let near_black = shell.colour.iter().map(|&c| u32::from(c)).sum::<u32>() < 3 * 0x30;
    let (ink, strength) = match s.mould {
        Mould::Notched
        | Mould::Rounded
        | Mould::Nes
        | Mould::SnesNtsc
        | Mould::SnesPal
        | Mould::SgbNtsc
        | Mould::SgbPal
            if !near_black =>
        {
            ([0x1b, 0x1c, 0x24], 235)
        }
        _ => ([0xf2, 0xf0, 0xea], 245),
    };
    let ink = marker.unwrap_or(ink);
    for (i, a) in text::marker_coverage(lw, lh, &layout, seed)
        .into_iter()
        .enumerate()
    {
        if a == 0 {
            continue;
        }
        let a = (a as u32 * strength / 255) as u8;
        rgba[i * 4..i * 4 + 4].copy_from_slice(&[ink[0], ink[1], ink[2], a]);
    }
    rgba
}

/// The stickers sit a little inside their well, so its edge shows round them (Roy 2026-11-15):
/// left, right, top and bottom, in the art's pixels. The NES and American SNES ones still run
/// over the top edge. The American SNES well (x 70..=291, floor down to y 101, made 4px deeper so
/// the sticker fits whole, Roy 2026-11-15): its sticker leaves 1px each side and 2px below,
/// its proportions the original label's.
fn label_inset(mould: Mould) -> (u32, u32, u32, u32) {
    match mould {
        Mould::Nes => (2, 2, 0, 2),
        Mould::SnesNtsc => (3, 1, 0, 2),
        Mould::SnesPal => (1, 1, 0, 2),
        // Set a little high in the well: even margins read as sitting too low (Roy 2026-11-15).
        Mould::Gba | Mould::Notched | Mould::Rounded => (1, 1, 0, 2),
        Mould::SgbNtsc | Mould::SgbPal => (0, 0, 0, 0),
    }
}

/// How dark a sticker's drop shadow falls on the plastic, row by row below its bottom edge:
/// small and sharp, as a sheet of paper lying on the cart (Roy 2026-11-15).
const STICKER_SHADOW: [u32; 1] = [100];

/// Where the sticker itself lies: its well less the margin round it.
fn sticker_rect(s: &Spec) -> (u32, u32, u32, u32) {
    let (wx, wy, ww, wh) = s.label;
    let (il, ir, it, ib) = label_inset(s.mould);
    (wx + il, wy + it, ww - il - ir, wh - it - ib)
}

fn paste_label(s: &Spec, face: &mut CartFace, label: &[u8], paper: bool) {
    let (_, _, ww, wh) = s.label;
    let (lx, ly, lw, lh) = sticker_rect(s);
    let shape = |x: u32, y: u32| match s.square_top {
        true => corner_cover(x, y + lh, lw, 2 * lh, s.radius),
        false => corner_cover(x, y, lw, lh, s.radius),
    };
    // The American SNES well's floor a touch lighter than the plastic round it, so the margin
    // round its sticker shows (Roy 2026-11-15). Its visible well: x 70..=291, floor down to
    // y 101, its lower corners round (radius 7); it runs out over the top edge.
    if s.mould == Mould::SnesNtsc {
        let (fx, fw, fh) = (70u32, 222u32, 102u32);
        for y in 0..fh {
            for x in 0..fw {
                let k = corner_cover(x, y + fh, fw, 2 * fh, 7.0);
                if k == 0 {
                    continue;
                }
                let d = ((y * s.w + x + fx) * 4) as usize;
                for c in 0..3 {
                    let v = face.rgba[d + c] as u32;
                    let lit = (v * 107 / 100 + 4).min(255);
                    face.rgba[d + c] = ((lit * k + v * (255 - k)) / 255) as u8;
                }
            }
        }
        // its side walls barely show in the art: a fine shadowed edge down each side, down to
        // where the lower corners start to round
        for x in [fx - 1, fx + fw] {
            for y in 0..fh - 7 {
                let d = ((y * s.w + x) * 4) as usize;
                for c in 0..3 {
                    face.rgba[d + c] = (face.rgba[d + c] as u32 * 60 / 100) as u8;
                }
            }
        }
    }
    // A printed sticker casts a small sharp shadow below it; the marker's ink does not.
    if paper {
        for (k, strength) in STICKER_SHADOW.iter().enumerate() {
            let y = lh + k as u32;
            if y + ly >= s.h {
                break;
            }
            for x in 0..lw {
                // the sticker's own bottom row, its round corners included, dropped k + 1 rows
                let cast = shape(x, lh - 1) * strength / 255;
                let d = (((y + ly) * s.w + x + lx) * 4) as usize;
                for c in 0..3 {
                    face.rgba[d + c] = (face.rgba[d + c] as u32 * (255 - cast) / 255) as u8;
                }
            }
        }
    }
    // The art is made for the whole well: drawn smaller into the inset sticker, its proportions
    // kept as the original label's (Roy 2026-11-15): scaled evenly to cover the sticker, the bit
    // over shared out between both sides, and on a sticker running over the top edge all off its
    // foot, so the top stays whole.
    let k = (lw as f32 / ww as f32).max(lh as f32 / wh as f32);
    let ox = (lw as f32 - ww as f32 * k) / 2.0;
    let oy = if s.square_top { 0.0 } else { (lh as f32 - wh as f32 * k) / 2.0 };
    for y in 0..lh {
        for x in 0..lw {
            let px = sample(label, ww, wh, (x as f32 + 0.5 - ox) / k, (y as f32 + 0.5 - oy) / k);
            // A sticker that wraps over the top edge is square there: measure its corners
            // as if it went on above.
            let a = px[3] as u32 * shape(x, y) / 255;
            if a == 0 {
                continue;
            }
            let d = (((y + ly) * s.w + x + lx) * 4) as usize;
            for c in 0..3 {
                face.rgba[d + c] =
                    ((px[c] as u32 * a + face.rgba[d + c] as u32 * (255 - a) + 127) / 255)
                        as u8;
            }
            // Paper is opaque: on a clear shell the recess under it is see-through plastic,
            // and the sticker must not take that over.
            face.rgba[d + 3] = (a + (face.rgba[d + 3] as u32 * (255 - a) + 127) / 255) as u8;
        }
    }
}

/// The `w`x`h` RGBA image at (`fx`, `fy`), bilinear, colour weighted by its alpha.
fn sample(img: &[u8], w: u32, h: u32, fx: f32, fy: f32) -> [u8; 4] {
    if w == 0 || h == 0 {
        return [0; 4];
    }
    let (x, y) = ((fx - 0.5).clamp(0.0, (w - 1) as f32), (fy - 0.5).clamp(0.0, (h - 1) as f32));
    let (x0, y0) = (x as u32, y as u32);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (tx, ty) = (x - x0 as f32, y - y0 as f32);
    let mut acc = [0f32; 4];
    for (xx, yy, k) in [(x0, y0, (1.0 - tx) * (1.0 - ty)), (x1, y0, tx * (1.0 - ty)), (x0, y1, (1.0 - tx) * ty), (x1, y1, tx * ty)] {
        let p = &img[((yy * w + xx) * 4) as usize..][..4];
        let a = p[3] as f32 * k;
        for c in 0..3 {
            acc[c] += p[c] as f32 * a;
        }
        acc[3] += a;
    }
    let a = acc[3].max(1e-6);
    [(acc[0] / a).round() as u8, (acc[1] / a).round() as u8, (acc[2] / a).round() as u8, acc[3].round().min(255.0) as u8]
}

/// How much of a pixel a sticker with rounded corners covers, 0 to 255.
fn corner_cover(x: u32, y: u32, w: u32, h: u32, r: f32) -> u32 {
    if r <= 0.0 {
        return 255;
    }
    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
    let cx = px.clamp(r, w as f32 - r);
    let cy = py.clamp(r, h as f32 - r);
    let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
    ((r + 0.5 - d).clamp(0.0, 1.0) * 255.0) as u32
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    ]
}
