use std::sync::OnceLock;

use slot_gfx::OUT_W;

use crate::art::decode_png_bytes;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Mould {
    Gba,
    Notched,
    Rounded,
    /// The NES's grey toaster cart.
    Nes,
    /// The American SNES cart, ridged sides and a raised middle.
    SnesNtsc,
    /// The European SNES and Super Famicom cart: smaller, rounded ends, an inset label.
    SnesPal,
}

impl Mould {
    pub const ALL: [Mould; 6] = [
        Mould::Gba,
        Mould::Notched,
        Mould::Rounded,
        Mould::Nes,
        Mould::SnesNtsc,
        Mould::SnesPal,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    /// The cart's box on the shelf, in pixels. Not to scale between consoles: an SNES cart
    /// stands a little wider than a Game Boy pak rather than twice as wide.
    pub fn size(self) -> (u32, u32) {
        match self {
            Mould::Gba => (GBA_CART_W, GBA_CART_H),
            Mould::Notched | Mould::Rounded | Mould::Nes => (GB_CART_W, GB_CART_H),
            Mould::SnesNtsc => (SNES_CART_W, SNES_CART_H),
            Mould::SnesPal => (SNES_PAL_CART_W, SNES_PAL_CART_H),
        }
    }

    /// A TV console's cart: its art comes as a full set of cart, neutral, clear, interior
    /// and board, all the cart's own size.
    pub fn tv(self) -> bool {
        matches!(self, Mould::Nes | Mould::SnesNtsc | Mould::SnesPal)
    }
}

pub const GBA_CART_W: u32 = 360;
pub const GBA_CART_H: u32 = 212;
pub const GB_CART_W: u32 = 312;
pub const GB_CART_H: u32 = 337;
pub const SNES_CART_W: u32 = 360;
pub const SNES_CART_H: u32 = 231;
pub const SNES_PAL_CART_W: u32 = 336;
pub const SNES_PAL_CART_H: u32 = 228;
pub const BOARD_W: u32 = GBA_CART_W;
pub const BOARD_H: u32 = GBA_CART_H;
pub const SOCKET_W: u32 = 64;
pub const SOCKET_H: u32 = 46;
pub const CHIP_W: u32 = 59;
pub const CHIP_H: u32 = 41;
pub const SLOT_W: u32 = OUT_W;
pub const SLOT_H: u32 = 58;
pub const PORT_W: u32 = OUT_W;
pub const PORT_H: u32 = SLOT_H;
pub const PLUG_W: u32 = 92;
pub const PLUG_H: u32 = 244;
pub const ADAPTER_W: u32 = 290;
pub const ADAPTER_H: u32 = 180;
pub const STICKER_W: u32 = 660;
pub const STICKER_H: u32 = 228;

const GBA_CART_PNG: &[u8] = include_bytes!("../../../assets/gba_cart.png");
const GB_CART_PNG: &[u8] = include_bytes!("../../../assets/gb_cart.png");
const GBC_CART_PNG: &[u8] = include_bytes!("../../../assets/gbc_cart.png");
const GBA_INTERIOR_PNG: &[u8] = include_bytes!("../../../assets/gba_interior.png");
const GB_INTERIOR_PNG: &[u8] = include_bytes!("../../../assets/gb_interior.png");
const GBC_INTERIOR_PNG: &[u8] = include_bytes!("../../../assets/gbc_interior.png");
const GB_BOARD_PNG: &[u8] = include_bytes!("../../../assets/gb_board.png");
const BOARD_PNG: &[u8] = include_bytes!("../../../assets/board.png");
const SOCKET_PNG: &[u8] = include_bytes!("../../../assets/socket.png");
const CHIP_PNG: &[u8] = include_bytes!("../../../assets/chip.png");
const SLOT_BACK_DARK_PNG: &[u8] = include_bytes!("../../../assets/slot_back_dark.png");
const SLOT_BACK_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/slot_back_light.png");
const SLOT_FRONT_DARK_PNG: &[u8] = include_bytes!("../../../assets/slot_front_dark.png");
const SLOT_FRONT_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/slot_front_light.png");
const SLOT_BACK_GB_DARK_PNG: &[u8] = include_bytes!("../../../assets/slot_back_gb_dark.png");
const SLOT_FRONT_GB_DARK_PNG: &[u8] = include_bytes!("../../../assets/slot_front_gb_dark.png");
const SLOT_BACK_GB_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/slot_back_gb_light.png");
const SLOT_FRONT_GB_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/slot_front_gb_light.png");
const SLOT_BACK_NES_DARK_PNG: &[u8] = include_bytes!("../../../assets/slot_back_nes_dark.png");
const SLOT_FRONT_NES_DARK_PNG: &[u8] = include_bytes!("../../../assets/slot_front_nes_dark.png");
const SLOT_BACK_NES_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/slot_back_nes_light.png");
const SLOT_FRONT_NES_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/slot_front_nes_light.png");
const SLOT_BACK_SNES_DARK_PNG: &[u8] = include_bytes!("../../../assets/slot_back_snes_dark.png");
const SLOT_FRONT_SNES_DARK_PNG: &[u8] = include_bytes!("../../../assets/slot_front_snes_dark.png");
const SLOT_BACK_SNES_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/slot_back_snes_light.png");
const SLOT_FRONT_SNES_LIGHT_PNG: &[u8] =
    include_bytes!("../../../assets/slot_front_snes_light.png");
const NES_LID_DARK_PNG: &[u8] = include_bytes!("../../../assets/nes_lid_dark.png");
const NES_LID_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/nes_lid_light.png");
const PORT_DARK_PNG: &[u8] = include_bytes!("../../../assets/link_port_dark.png");
const PORT_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/link_port_light.png");
const PLUG_HOST_PNG: &[u8] = include_bytes!("../../../assets/link_plug_host.png");
const PLUG_JOIN_PNG: &[u8] = include_bytes!("../../../assets/link_plug_join.png");
const ADAPTER_PNG: &[u8] = include_bytes!("../../../assets/link_adapter.png");
const STICKER_PNG: &[u8] = include_bytes!("../../../assets/sticker.png");

fn load(bytes: &'static [u8], w: u32, h: u32, slot: &'static OnceLock<Vec<u8>>) -> &'static [u8] {
    slot.get_or_init(|| {
        let (mut rgba, sw, sh) = decode_png_bytes(bytes).expect("a template png decodes");
        if sh == h && sw > w {
            rgba = crop_centre(&rgba, sw, sh, w);
        } else if (sw, sh) != (w, h) {
            rgba = scale(&rgba, sw, sh, w, h);
        }
        // Transparent pixels are often stored as white. The cart is sampled with a linear
        // filter, which would pull that white in along every edge.
        bleed(&mut rgba, w, h);
        rgba
    })
}

/// Copy an opaque neighbour's colour into each transparent pixel, leaving its alpha at 0.
fn bleed(rgba: &mut [u8], w: u32, h: u32) {
    let src = rgba.to_vec();
    let (w, h) = (w as i32, h as i32);
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            if src[i + 3] != 0 {
                continue;
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let n = ((ny * w + nx) * 4) as usize;
                if src[n + 3] == 0 {
                    continue;
                }
                rgba[i..i + 3].copy_from_slice(&src[n..n + 3]);
                break;
            }
        }
    }
}

fn crop_centre(src: &[u8], sw: u32, sh: u32, w: u32) -> Vec<u8> {
    let left = ((sw - w) / 2) as usize;
    let mut out = Vec::with_capacity((w * sh * 4) as usize);
    for row in src.chunks_exact(sw as usize * 4) {
        out.extend_from_slice(&row[left * 4..(left + w as usize) * 4]);
    }
    out
}

fn scale(src: &[u8], sw: u32, sh: u32, w: u32, h: u32) -> Vec<u8> {
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        let sy = y * sh / h;
        for x in 0..w {
            let sx = x * sw / w;
            let from = ((sy * sw + sx) * 4) as usize;
            let to = ((y * w + x) * 4) as usize;
            out[to..to + 4].copy_from_slice(&src[from..from + 4]);
        }
    }
    out
}

pub fn gba_cart() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(GBA_CART_PNG, GBA_CART_W, GBA_CART_H, &P)
}

pub fn gb_cart() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(GB_CART_PNG, GB_CART_W, GB_CART_H, &P)
}

pub fn gbc_cart() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(GBC_CART_PNG, GB_CART_W, GB_CART_H, &P)
}

/// The inside of a GBA cart, from `assets/gba_interior.png`. Same size as the cart and
/// the board, so the carousel can stack the three 1:1. Its colour is set by the game, with the
/// same overlay blend the shells use.
pub fn gba_interior() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(GBA_INTERIOR_PNG, BOARD_W, BOARD_H, &P)
}

/// The back of a Game Boy pak, from `assets/gb_interior.png`.
pub fn gb_interior() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(GB_INTERIOR_PNG, GB_CART_W, GB_CART_H, &P)
}

/// The back of a Game Boy Color pak, from `assets/gbc_interior.png`.
pub fn gbc_interior() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(GBC_INTERIOR_PNG, GB_CART_W, GB_CART_H, &P)
}

/// The Game Boy PCB, from `assets/gb_board.png`, drawn between the pak's back and front plastic.
pub fn gb_board() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(GB_BOARD_PNG, GB_CART_W, GB_CART_H, &P)
}

pub fn board() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(BOARD_PNG, BOARD_W, BOARD_H, &P)
}

pub fn socket() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(SOCKET_PNG, SOCKET_W, SOCKET_H, &P)
}

pub fn chip() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(CHIP_PNG, CHIP_W, CHIP_H, &P)
}

pub fn slot_back() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(SLOT_BACK_DARK_PNG, SLOT_W, SLOT_H, &P)
}

pub fn slot_back_light() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(SLOT_BACK_LIGHT_PNG, SLOT_W, SLOT_H, &P)
}

pub fn slot_front() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(SLOT_FRONT_DARK_PNG, SLOT_W, SLOT_H, &P)
}

/// The Game Boy and Game Boy Color slot, behind the cart: the original Game Boy's top edge,
/// in the dark or the light housing.
pub fn slot_back_gb(light: bool) -> &'static [u8] {
    static D: OnceLock<Vec<u8>> = OnceLock::new();
    static L: OnceLock<Vec<u8>> = OnceLock::new();
    match light {
        false => load(SLOT_BACK_GB_DARK_PNG, SLOT_W, SLOT_H, &D),
        true => load(SLOT_BACK_GB_LIGHT_PNG, SLOT_W, SLOT_H, &L),
    }
}

/// The Game Boy slot in front of the cart, with the mouth open.
pub fn slot_front_gb(light: bool) -> &'static [u8] {
    static D: OnceLock<Vec<u8>> = OnceLock::new();
    static L: OnceLock<Vec<u8>> = OnceLock::new();
    match light {
        false => load(SLOT_FRONT_GB_DARK_PNG, SLOT_W, SLOT_H, &D),
        true => load(SLOT_FRONT_GB_LIGHT_PNG, SLOT_W, SLOT_H, &L),
    }
}

/// The NES band (`nes`) or the SNES one, behind (`front` false) or in front of the cart, in the
/// dark or the light housing. The NES back has its lid shut; the front leaves the lid's mouth open.
pub fn slot_tv(nes: bool, front: bool, light: bool) -> &'static [u8] {
    static P: [OnceLock<Vec<u8>>; 8] = [const { OnceLock::new() }; 8];
    let bytes = match (nes, front, light) {
        (true, false, false) => SLOT_BACK_NES_DARK_PNG,
        (true, true, false) => SLOT_FRONT_NES_DARK_PNG,
        (true, false, true) => SLOT_BACK_NES_LIGHT_PNG,
        (true, true, true) => SLOT_FRONT_NES_LIGHT_PNG,
        (false, false, false) => SLOT_BACK_SNES_DARK_PNG,
        (false, true, false) => SLOT_FRONT_SNES_DARK_PNG,
        (false, false, true) => SLOT_BACK_SNES_LIGHT_PNG,
        (false, true, true) => SLOT_FRONT_SNES_LIGHT_PNG,
    };
    let i = usize::from(nes) * 4 + usize::from(front) * 2 + usize::from(light);
    load(bytes, SLOT_W, SLOT_H, &P[i])
}

pub const NES_LID_W: u32 = 256;

/// The NES lid, shut: the band's own plastic, as wide as the mouth and as deep as the band.
pub fn nes_lid(light: bool) -> &'static [u8] {
    static D: OnceLock<Vec<u8>> = OnceLock::new();
    static L: OnceLock<Vec<u8>> = OnceLock::new();
    match light {
        false => load(NES_LID_DARK_PNG, NES_LID_W, SLOT_H, &D),
        true => load(NES_LID_LIGHT_PNG, NES_LID_W, SLOT_H, &L),
    }
}

pub fn slot_front_light() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(SLOT_FRONT_LIGHT_PNG, SLOT_W, SLOT_H, &P)
}

pub fn link_port() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(PORT_DARK_PNG, PORT_W, PORT_H, &P)
}

pub fn link_port_light() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(PORT_LIGHT_PNG, PORT_W, PORT_H, &P)
}

pub fn link_plug_host() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(PLUG_HOST_PNG, PLUG_W, PLUG_H, &P)
}

pub fn link_plug_join() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(PLUG_JOIN_PNG, PLUG_W, PLUG_H, &P)
}

pub fn link_adapter() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(ADAPTER_PNG, ADAPTER_W, ADAPTER_H, &P)
}

pub fn sticker() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(STICKER_PNG, STICKER_W, STICKER_H, &P)
}

// Shells are hand drawn. There is no Game Boy clear file yet, so a clear Game Boy pak uses
// the neutral shell at 80%.
const GBA_SHELL_PNG: &[u8] = include_bytes!("../../../assets/gba_neutral.png");
const GB_SHELL_PNG: &[u8] = include_bytes!("../../../assets/gb_neutral.png");
const GBC_SHELL_PNG: &[u8] = include_bytes!("../../../assets/gbc_neutral.png");
const GBA_CLEAR_PNG: &[u8] = include_bytes!("../../../assets/gba_clear.png");
const GB_CLEAR_PNG: &[u8] = include_bytes!("../../../assets/gb_neutral.png");
const GBC_CLEAR_PNG: &[u8] = include_bytes!("../../../assets/gbc_clear.png");

/// The neutral shell asset for a mould, opaque or the separate clear one. The game's colour is
/// blended onto it by the caller.
pub fn shell_asset(mould: Mould, clear: bool) -> &'static [u8] {
    if mould.tv() {
        return tv_art(
            mould,
            if clear {
                TvPart::Clear
            } else {
                TvPart::Neutral
            },
        );
    }
    static GBA: OnceLock<Vec<u8>> = OnceLock::new();
    static GB: OnceLock<Vec<u8>> = OnceLock::new();
    static GBC: OnceLock<Vec<u8>> = OnceLock::new();
    static GBA_CLEAR: OnceLock<Vec<u8>> = OnceLock::new();
    static GB_CLEAR: OnceLock<Vec<u8>> = OnceLock::new();
    static GBC_CLEAR: OnceLock<Vec<u8>> = OnceLock::new();
    let (bytes, w, h, slot) = match (mould, clear) {
        (Mould::Gba, false) => (GBA_SHELL_PNG, GBA_CART_W, GBA_CART_H, &GBA),
        (Mould::Notched, false) => (GB_SHELL_PNG, GB_CART_W, GB_CART_H, &GB),
        (Mould::Rounded, false) => (GBC_SHELL_PNG, GB_CART_W, GB_CART_H, &GBC),
        (Mould::Gba, true) => (GBA_CLEAR_PNG, GBA_CART_W, GBA_CART_H, &GBA_CLEAR),
        (Mould::Notched, true) => (GB_CLEAR_PNG, GB_CART_W, GB_CART_H, &GB_CLEAR),
        (Mould::Rounded, true) => (GBC_CLEAR_PNG, GB_CART_W, GB_CART_H, &GBC_CLEAR),
        _ => unreachable!("a TV cart's shell is its own art"),
    };
    load(bytes, w, h, slot)
}

/// The layers of a TV console's cart, rendered from photos by `tools/carts`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TvPart {
    /// White where the cart is: its silhouette.
    Cart,
    Neutral,
    Clear,
    Interior,
    Board,
}

const NES_PNGS: [&[u8]; 5] = [
    include_bytes!("../../../assets/nes_cart.png"),
    include_bytes!("../../../assets/nes_neutral.png"),
    include_bytes!("../../../assets/nes_clear.png"),
    include_bytes!("../../../assets/nes_interior.png"),
    include_bytes!("../../../assets/nes_board.png"),
];
const SNES_PNGS: [&[u8]; 5] = [
    include_bytes!("../../../assets/snes_cart.png"),
    include_bytes!("../../../assets/snes_neutral.png"),
    include_bytes!("../../../assets/snes_clear.png"),
    include_bytes!("../../../assets/snes_interior.png"),
    include_bytes!("../../../assets/snes_board.png"),
];
const SNES_PAL_PNGS: [&[u8]; 5] = [
    include_bytes!("../../../assets/snes_pal_cart.png"),
    include_bytes!("../../../assets/snes_pal_neutral.png"),
    include_bytes!("../../../assets/snes_pal_clear.png"),
    include_bytes!("../../../assets/snes_pal_interior.png"),
    include_bytes!("../../../assets/snes_pal_board.png"),
];

pub fn tv_art(mould: Mould, part: TvPart) -> &'static [u8] {
    static SLOTS: [[OnceLock<Vec<u8>>; 5]; 3] = [const { [const { OnceLock::new() }; 5] }; 3];
    let (row, pngs) = match mould {
        Mould::Nes => (0, NES_PNGS),
        Mould::SnesNtsc => (1, SNES_PNGS),
        Mould::SnesPal => (2, SNES_PAL_PNGS),
        _ => unreachable!("only a TV cart has TV art"),
    };
    let (w, h) = mould.size();
    load(pngs[part as usize], w, h, &SLOTS[row][part as usize])
}
