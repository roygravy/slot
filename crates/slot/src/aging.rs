//! Wear on the cart faces, from the wear book in Config/wear.tsv: a hidden easter egg, always
//! on, no setting (Roy, xg1). Dust is gone for now (xg2); only `SLOT_FAKE_AGE` still draws it.

use std::path::Path;

use slot_store::wear::{key, WearBook};
use slot_store::Cart;
use slot_ui::{cart_face_aged, CartFace};

/// A fixed seed per cart, so a cart's dust never changes between builds.
pub fn seed_of(stem: &str) -> u64 {
    stem.bytes().fold(0xCBF2_9CE4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01B3))
}

/// `SLOT_FAKE_AGE=dust,wear`, for simulator previews.
pub fn forced() -> Option<(u8, u8)> {
    let v = std::env::var("SLOT_FAKE_AGE").ok()?;
    let (d, w) = v.split_once(',')?;
    Some((d.trim().parse().unwrap_or(0), w.trim().parse().unwrap_or(0)))
}

/// `(dust, wear)` of a cart. `SLOT_FAKE_AGE=dust,wear` forces both, for simulator previews.
pub fn levels(root: &Path, cart: &Cart) -> (u8, u8) {
    if let Some(forced) = forced() {
        return forced;
    }
    let book = WearBook::read(root);
    (0, book.wear_level(&key(cart.platform, &cart.stem)))
}

/// The cart's face with its dust and wear baked in.
pub fn face(root: &Path, cart: &Cart, art: Option<Vec<u8>>) -> CartFace {
    let (dust, wear) = levels(root, cart);
    cart_face_aged(cart, art, dust, wear, seed_of(&cart.stem))
}
