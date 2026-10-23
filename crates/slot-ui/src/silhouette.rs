use std::sync::OnceLock;

use crate::bitmaps;
use crate::cart::{CART_H, CART_W, GB_CART_H, GB_CART_W};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum GbShell {
    Notched,
    Rounded,
}

pub fn silhouette(w: u32, h: u32) -> Vec<u8> {
    alpha(bitmaps::gba_cart(), w, h)
}

pub fn gb_silhouette(shell: GbShell, w: u32, h: u32) -> Vec<u8> {
    let src = match shell {
        GbShell::Notched => bitmaps::gb_cart(),
        GbShell::Rounded => bitmaps::gbc_cart(),
    };
    alpha(src, w, h)
}

fn alpha(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
    debug_assert_eq!(rgba.len(), (w * h * 4) as usize);
    rgba.iter().skip(3).step_by(4).copied().collect()
}

pub(crate) fn cart_mask() -> &'static [u8] {
    static MASK: OnceLock<Vec<u8>> = OnceLock::new();
    MASK.get_or_init(|| silhouette(CART_W, CART_H))
}

pub(crate) fn gb_cart_mask(shell: GbShell) -> &'static [u8] {
    static NOTCHED: OnceLock<Vec<u8>> = OnceLock::new();
    static ROUNDED: OnceLock<Vec<u8>> = OnceLock::new();
    let lock = match shell {
        GbShell::Notched => &NOTCHED,
        GbShell::Rounded => &ROUNDED,
    };
    lock.get_or_init(|| gb_silhouette(shell, GB_CART_W, GB_CART_H))
}

/// A mould's silhouette: how much of each pixel of its box the cart covers.
pub(crate) fn mould_mask(mould: bitmaps::Mould) -> &'static [u8] {
    use bitmaps::Mould;
    static TV: [OnceLock<Vec<u8>>; 3] = [const { OnceLock::new() }; 3];
    let row = match mould {
        Mould::Gba => return cart_mask(),
        Mould::Notched => return gb_cart_mask(GbShell::Notched),
        Mould::Rounded => return gb_cart_mask(GbShell::Rounded),
        Mould::Nes => 0,
        Mould::SnesNtsc => 1,
        Mould::SnesPal => 2,
    };
    let (w, h) = mould.size();
    TV[row].get_or_init(|| alpha(bitmaps::tv_art(mould, bitmaps::TvPart::Cart), w, h))
}
