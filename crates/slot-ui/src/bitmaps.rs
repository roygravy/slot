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
    /// The American Super Game Boy: the ridged SNES cart, taller, a Game Boy bay in its top.
    SgbNtsc,
    /// The European and Japanese Super Game Boy, and the Super Game Boy 2.
    SgbPal,
}

impl Mould {
    pub const ALL: [Mould; 8] = [
        Mould::Gba,
        Mould::Notched,
        Mould::Rounded,
        Mould::Nes,
        Mould::SnesNtsc,
        Mould::SnesPal,
        Mould::SgbNtsc,
        Mould::SgbPal,
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
            Mould::SgbNtsc => (SGB_CART_W, SGB_CART_H),
            Mould::SgbPal => (SGB_PAL_CART_W, SGB_PAL_CART_H),
        }
    }

    /// A TV console's cart: its art comes as a full set of cart, neutral, clear, interior
    /// and board, all the cart's own size.
    pub fn tv(self) -> bool {
        matches!(
            self,
            Mould::Nes | Mould::SnesNtsc | Mould::SnesPal | Mould::SgbNtsc | Mould::SgbPal
        )
    }

    /// A Super Game Boy: its art is finished, sticker and plastic, one picture per model.
    pub fn sgb(self) -> bool {
        matches!(self, Mould::SgbNtsc | Mould::SgbPal)
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
pub const SGB_CART_W: u32 = 360;
pub const SGB_CART_H: u32 = 315;
pub const SGB_PAL_CART_W: u32 = 336;
pub const SGB_PAL_CART_H: u32 = 319;
pub const BOARD_W: u32 = GBA_CART_W;
pub const BOARD_H: u32 = GBA_CART_H;
pub const SOCKET_W: u32 = 64;
pub const SOCKET_H: u32 = 46;
pub const CHIP_W: u32 = 59;
pub const CHIP_H: u32 = 41;
/// The slot strips are drawn 720 wide: a 4:3 canvas shows their middle, a 3:2 one all of it.
pub const SLOT_W: u32 = 720;
pub const SLOT_H: u32 = 58;
/// The band while the Super Game Boy is in: its top stands above the console's.
pub const SGB_BAND_H: u32 = 87;
pub const PORT_W: u32 = OUT_W;
pub const PORT_H: u32 = SLOT_H;
pub const PLUG_W: u32 = 92;
pub const PLUG_H: u32 = 244;
pub const ADAPTER_W: u32 = 290;
pub const ADAPTER_H: u32 = 180;

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
const POWER_KNOB_DARK_PNG: &[u8] = include_bytes!("../../../assets/power_knob_dark.png");
const POWER_KNOB_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/power_knob_light.png");
const POWER_TAB_DARK_PNG: &[u8] = include_bytes!("../../../assets/power_tab_dark.png");
const POWER_TAB_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/power_tab_light.png");
/// The original Game Boy's power switch, from `tools/power_switch.py`: the knob on the band's
/// top edge (its plate's left edge `POWER_KNOB_PAD` px in) and the lock tab.
pub const POWER_KNOB_W: u32 = 23;
pub const POWER_KNOB_H: u32 = 11;
pub const POWER_KNOB_PAD: u32 = 3;
pub const POWER_TAB_W: u32 = 17;
pub const POWER_TAB_H: u32 = 7;
const SLOT_BACK_NES_DARK_PNG: &[u8] = include_bytes!("../../../assets/slot_back_nes_dark.png");
const SLOT_FRONT_NES_DARK_PNG: &[u8] = include_bytes!("../../../assets/slot_front_nes_dark.png");
const SLOT_BACK_NES_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/slot_back_nes_light.png");
const SLOT_FRONT_NES_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/slot_front_nes_light.png");
const SLOT_BACK_SNES_DARK_PNG: &[u8] = include_bytes!("../../../assets/slot_back_snes_dark.png");
const SLOT_FRONT_SNES_DARK_PNG: &[u8] = include_bytes!("../../../assets/slot_front_snes_dark.png");
const SLOT_BACK_SNES_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/slot_back_snes_light.png");
const SLOT_FRONT_SNES_LIGHT_PNG: &[u8] =
    include_bytes!("../../../assets/slot_front_snes_light.png");
const SLOT_BACK_NES_GREY_PNG: &[u8] = include_bytes!("../../../assets/slot_back_nes_grey.png");
const SLOT_FRONT_NES_GREY_PNG: &[u8] = include_bytes!("../../../assets/slot_front_nes_grey.png");
const SLOT_BACK_SNES_GREY_PNG: &[u8] = include_bytes!("../../../assets/slot_back_snes_grey.png");
const SLOT_FRONT_SNES_GREY_PNG: &[u8] = include_bytes!("../../../assets/slot_front_snes_grey.png");
const NES_LID_GREY_PNG: &[u8] = include_bytes!("../../../assets/nes_lid_grey.png");
const NES_LID_DARK_PNG: &[u8] = include_bytes!("../../../assets/nes_lid_dark.png");
const NES_LID_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/nes_lid_light.png");
const NES_LID_ORIG_PNG: &[u8] = include_bytes!("../../../assets/nes_lid_orig.png");
/// The Originals housings, back and front: GBA, GB, GBC, NES, SNES.
const SLOT_ORIG_PNG: [[&[u8]; 2]; 5] = [
    [
        include_bytes!("../../../assets/slot_back_orig.png"),
        include_bytes!("../../../assets/slot_front_orig.png"),
    ],
    [
        include_bytes!("../../../assets/slot_back_gb_orig.png"),
        include_bytes!("../../../assets/slot_front_gb_orig.png"),
    ],
    [
        include_bytes!("../../../assets/slot_back_gb_orig_gbc.png"),
        include_bytes!("../../../assets/slot_front_gb_orig_gbc.png"),
    ],
    [
        include_bytes!("../../../assets/slot_back_nes_orig.png"),
        include_bytes!("../../../assets/slot_front_nes_orig.png"),
    ],
    [
        include_bytes!("../../../assets/slot_back_snes_orig.png"),
        include_bytes!("../../../assets/slot_front_snes_orig.png"),
    ],
];
const PORT_DARK_PNG: &[u8] = include_bytes!("../../../assets/link_port_dark.png");
const PORT_LIGHT_PNG: &[u8] = include_bytes!("../../../assets/link_port_light.png");
const PLUG_HOST_PNG: &[u8] = include_bytes!("../../../assets/link_plug_host.png");
const PLUG_JOIN_PNG: &[u8] = include_bytes!("../../../assets/link_plug_join.png");
const ADAPTER_PNG: &[u8] = include_bytes!("../../../assets/link_adapter.png");

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

/// The original Game Boy's power switch knob, in the dark or the light housing's grey.
pub fn power_knob(light: bool) -> &'static [u8] {
    static D: OnceLock<Vec<u8>> = OnceLock::new();
    static L: OnceLock<Vec<u8>> = OnceLock::new();
    match light {
        false => load(POWER_KNOB_DARK_PNG, POWER_KNOB_W, POWER_KNOB_H, &D),
        true => load(POWER_KNOB_LIGHT_PNG, POWER_KNOB_W, POWER_KNOB_H, &L),
    }
}

/// The lock tab on the power switch, which slides over the cart's notch.
pub fn power_tab(light: bool) -> &'static [u8] {
    static D: OnceLock<Vec<u8>> = OnceLock::new();
    static L: OnceLock<Vec<u8>> = OnceLock::new();
    match light {
        false => load(POWER_TAB_DARK_PNG, POWER_TAB_W, POWER_TAB_H, &D),
        true => load(POWER_TAB_LIGHT_PNG, POWER_TAB_W, POWER_TAB_H, &L),
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

/// The NES band (`nes`) or the SNES one in the DMG's grey, for the slot studio's DMG Grey.
pub fn slot_tv_grey(nes: bool, front: bool) -> &'static [u8] {
    static P: [OnceLock<Vec<u8>>; 4] = [const { OnceLock::new() }; 4];
    let bytes = match (nes, front) {
        (true, false) => SLOT_BACK_NES_GREY_PNG,
        (true, true) => SLOT_FRONT_NES_GREY_PNG,
        (false, false) => SLOT_BACK_SNES_GREY_PNG,
        (false, true) => SLOT_FRONT_SNES_GREY_PNG,
    };
    load(bytes, SLOT_W, SLOT_H, &P[usize::from(nes) * 2 + usize::from(front)])
}

/// The NES lid in the DMG's grey.
pub fn nes_lid_grey() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(NES_LID_GREY_PNG, NES_LID_W, SLOT_H, &P)
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

/// A slot in its console's own housing (Originals): `kind` 0 GBA, 1 GB, 2 GBC, 3 NES, 4 SNES.
pub fn slot_orig(kind: usize, front: bool) -> &'static [u8] {
    static P: [OnceLock<Vec<u8>>; 10] = [const { OnceLock::new() }; 10];
    let i = kind * 2 + usize::from(front);
    load(SLOT_ORIG_PNG[kind][usize::from(front)], SLOT_W, SLOT_H, &P[i])
}

/// How a slot case's plastic is finished (Roy 2026-11-06: the slots as the carts, Solid, Clear,
/// Pearl and Metallic).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SlotFinish {
    Solid,
    Clear,
    Pearl,
    Metallic,
}

impl SlotFinish {
    pub const ALL: [SlotFinish; 4] = [SlotFinish::Solid, SlotFinish::Clear, SlotFinish::Pearl, SlotFinish::Metallic];
}

/// The slots' neutral images (tools/slot_cases/neutral.py), coloured when drawn as the carts
/// are. Solid, Pearl and Metallic: per mouth K (what is not plastic: plate, shadow,
/// catch-lights) and T (the plastic's shading and coverage, x T_SCALE; its alpha the coverage
/// alone), back then front.
const SLOT_NEUTRAL_PNG: [[[&[u8]; 2]; 2]; 4] = [
    [
        [include_bytes!("../../../assets/slot_back_n_dmg_k.png"), include_bytes!("../../../assets/slot_back_n_dmg_t.png")],
        [include_bytes!("../../../assets/slot_front_n_dmg_k.png"), include_bytes!("../../../assets/slot_front_n_dmg_t.png")],
    ],
    [
        [include_bytes!("../../../assets/slot_back_n_flat_k.png"), include_bytes!("../../../assets/slot_back_n_flat_t.png")],
        [include_bytes!("../../../assets/slot_front_n_flat_k.png"), include_bytes!("../../../assets/slot_front_n_flat_t.png")],
    ],
    [
        [include_bytes!("../../../assets/slot_back_n_gbc_k.png"), include_bytes!("../../../assets/slot_back_n_gbc_t.png")],
        [include_bytes!("../../../assets/slot_front_n_gbc_k.png"), include_bytes!("../../../assets/slot_front_n_gbc_t.png")],
    ],
    [
        [include_bytes!("../../../assets/slot_back_n_gba_k.png"), include_bytes!("../../../assets/slot_back_n_gba_t.png")],
        [include_bytes!("../../../assets/slot_front_n_gba_k.png"), include_bytes!("../../../assets/slot_front_n_gba_t.png")],
    ],
];
const T_SCALE: f32 = 170.0;

/// Each model's clear case in colourless plastic over its own board, back and front, and the
/// back's mask of what takes the tint (the metal plate stays silver, Roy 2026-11-06).
const SLOT_CLEAR_PNG: [([&[u8]; 2], Option<&[u8]>); 4] = [
    ([include_bytes!("../../../assets/slot_back_gb_clear.png"), include_bytes!("../../../assets/slot_front_gb_clear.png")], None),
    ([include_bytes!("../../../assets/slot_back_gb_pocket_clear.png"), include_bytes!("../../../assets/slot_front_gb_pocket_clear.png")], None),
    ([include_bytes!("../../../assets/slot_back_gbc_neutral.png"), include_bytes!("../../../assets/slot_front_gbc_neutral.png")], None),
    ([include_bytes!("../../../assets/slot_back_gba_clear.png"), include_bytes!("../../../assets/slot_front_gba_clear.png")], Some(include_bytes!("../../../assets/slot_back_c_gba_m.png"))),
];

/// How many of GB_SLOT_CASES each slot model has, in order: DMG, Pocket/Light.
pub const GB_SLOT_MODELS: [usize; 2] = [SLOT_COLOURS, SLOT_COLOURS];

/// A slot case: its name, swatch, whether it is light (dark ink on it), its finish, whether it
/// has the DMG's lock, and its paint: the plastic's colour, or for a clear one what the plastic
/// lets through (255 lets all through).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct SlotCase {
    pub name: &'static str,
    pub swatch: [u8; 3],
    pub light: bool,
    pub finish: SlotFinish,
    pub lock: bool,
    pub paint: [u8; 3],
}

const fn rgb(v: u32) -> [u8; 3] {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

const fn case(name: &'static str, swatch: u32, light: bool, finish: SlotFinish, lock: bool, paint: u32) -> SlotCase {
    SlotCase { name, swatch: rgb(swatch), light, finish, lock, paint: rgb(paint) }
}

use SlotFinish::{Clear, Metallic, Pearl, Solid};

/// The slot cases the studio offers (Roy 2026-11-12): every Game Boy slot the same colours,
/// all that any of them was sold in plus Clear Black (no Kiwi, Cobalt, Graphite: 2026-11-13),
/// one hex each, the warm Play It Loud tints throughout. Each model names them as Nintendo
/// did for it (Flame on the SP, Play It Loud Red on the DMG), else by their best-known name. DMG Grey first; then the Solid, Clear
/// and Metallic ones (the Pearls among the Metallic, Pearl still their own sheen). One block
/// per model: the DMG, the Pocket/Light, the Game Boy Color (from GBC_SLOT_FIRST), the GBA SP
/// (from GBA_SLOT_FIRST).
pub const GB_SLOT_CASES: [SlotCase; 4 * SLOT_COLOURS] = [
    // The DMG (its Original is the grey and has the lock).
    case("Original", 0xC4C0B7, true, Solid, true, 0xC4C0B7),
    case("Grape", 0x684E92, false, Solid, true, 0x684E92),
    case("Indigo", 0x544A98, false, Solid, true, 0x544A98),
    case("Play It Loud Red", 0xCC3E30, false, Solid, true, 0xCC3E30),
    case("Berry", 0xB8243C, false, Solid, true, 0xB8243C),
    case("Pink", 0xE59BB5, true, Solid, true, 0xE59BB5),
    case("Play It Loud Yellow", 0xE6BC32, true, Solid, true, 0xE6BC32),
    case("Lime Green", 0xA4CC3A, true, Solid, true, 0xA4CC3A),
    case("Play It Loud Green", 0x4E9E5C, false, Solid, true, 0x4E9E5C),
    case("Teal", 0x1E9AA6, false, Solid, true, 0x1E9AA6),
    case("Surf Blue", 0x4E9CD4, false, Solid, true, 0x4E9CD4),
    case("Play It Loud Blue", 0x406CB8, false, Solid, true, 0x406CB8),
    case("Purple", 0x6A4C9C, false, Solid, true, 0x6A4C9C),
    case("Play It Loud Black", 0x2E2C2C, false, Solid, true, 0x2E2C2C),
    case("Play It Loud White", 0xECE8DE, true, Solid, true, 0xECE8DE),
    case("Play It Loud Clear", 0xE8ECEC, false, Clear, true, 0xFFFFFF),
    case("Clear Black", 0x4C4C54, false, Clear, true, 0x4D4D54),
    case("Atomic Purple", 0x8E76C4, false, Clear, true, 0xD6BDFF),
    case("Fuchsia", 0xD0509C, false, Clear, true, 0xFFB3E0),
    case("Clear Yellow", 0xF0D870, false, Clear, true, 0xFFE59E),
    case("Clear Green", 0x8CD08C, false, Clear, true, 0x9EEB9E),
    case("Ice Blue", 0xB0D4EC, false, Clear, true, 0xCCEBFF),
    case("Glacier", 0x94ACE2, false, Clear, true, 0xB8C8FF),
    case("Water Blue", 0x8CC0E4, false, Clear, true, 0xA8DBFF),
    case("Midnight Blue", 0x4C5CB4, false, Clear, true, 0x667AE0),
    case("Silver", 0xB8BABE, true, Metallic, true, 0xB8BABE),
    case("Gold", 0xBAA371, true, Metallic, true, 0xBAA371),
    // Gold, Pearl Pink, Blue and Green at 60% of their colour: fuller they read too loud on livelier
    // panels (RG SP tester, Roy 2026-11-17). Pearl Blue after Roy's SP photos: cyan, not silver.
    case("Pearl White", 0xEEEDE8, true, Pearl, true, 0xEEEDE8),
    case("Pearl Pink", 0xDEC1CB, true, Pearl, true, 0xDEC1CB),
    case("Pearl Blue", 0x93B9D0, true, Pearl, true, 0x93B9D0),
    case("Pearl Green", 0xB3CCB1, true, Pearl, true, 0xB3CCB1),
    // The Pocket and the Light: one flat slot.
    case("DMG Grey", 0xC4C0B7, true, Solid, false, 0xC4C0B7),
    case("Grape", 0x684E92, false, Solid, false, 0x684E92),
    case("Indigo", 0x544A98, false, Solid, false, 0x544A98),
    case("Red", 0xCC3E30, false, Solid, false, 0xCC3E30),
    case("Berry", 0xB8243C, false, Solid, false, 0xB8243C),
    case("Pink", 0xE59BB5, true, Solid, false, 0xE59BB5),
    case("Yellow", 0xE6BC32, true, Solid, false, 0xE6BC32),
    case("Lime Green", 0xA4CC3A, true, Solid, false, 0xA4CC3A),
    case("Green", 0x4E9E5C, false, Solid, false, 0x4E9E5C),
    case("Teal", 0x1E9AA6, false, Solid, false, 0x1E9AA6),
    case("Surf Blue", 0x4E9CD4, false, Solid, false, 0x4E9CD4),
    case("Blue", 0x406CB8, false, Solid, false, 0x406CB8),
    case("Purple", 0x6A4C9C, false, Solid, false, 0x6A4C9C),
    case("Black", 0x2E2C2C, false, Solid, false, 0x2E2C2C),
    case("White", 0xECE8DE, true, Solid, false, 0xECE8DE),
    case("Clear", 0xE8ECEC, false, Clear, false, 0xFFFFFF),
    case("Clear Black", 0x4C4C54, false, Clear, false, 0x4D4D54),
    case("Clear Purple", 0x8E76C4, false, Clear, false, 0xD6BDFF),
    case("Fuchsia", 0xD0509C, false, Clear, false, 0xFFB3E0),
    case("Clear Yellow", 0xF0D870, false, Clear, false, 0xFFE59E),
    case("Clear Green", 0x8CD08C, false, Clear, false, 0x9EEB9E),
    case("Ice Blue", 0xB0D4EC, false, Clear, false, 0xCCEBFF),
    case("Glacier", 0x94ACE2, false, Clear, false, 0xB8C8FF),
    case("Water Blue", 0x8CC0E4, false, Clear, false, 0xA8DBFF),
    case("Midnight Blue", 0x4C5CB4, false, Clear, false, 0x667AE0),
    case("Silver", 0xB8BABE, true, Metallic, false, 0xB8BABE),
    case("Gold", 0xBAA371, true, Metallic, false, 0xBAA371),
    case("Pearl White", 0xEEEDE8, true, Pearl, false, 0xEEEDE8),
    case("Pearl Pink", 0xDEC1CB, true, Pearl, false, 0xDEC1CB),
    case("Pearl Blue", 0x93B9D0, true, Pearl, false, 0x93B9D0),
    case("Pearl Green", 0xB3CCB1, true, Pearl, false, 0xB3CCB1),
    // The Game Boy Color.
    case("DMG Grey", 0xC4C0B7, true, Solid, false, 0xC4C0B7),
    case("Original", 0x684E92, false, Solid, false, 0x684E92),
    case("Indigo", 0x544A98, false, Solid, false, 0x544A98),
    case("Red", 0xCC3E30, false, Solid, false, 0xCC3E30),
    case("Berry", 0xB8243C, false, Solid, false, 0xB8243C),
    case("Pink", 0xE59BB5, true, Solid, false, 0xE59BB5),
    case("Dandelion", 0xE6BC32, true, Solid, false, 0xE6BC32),
    case("Lime Green", 0xA4CC3A, true, Solid, false, 0xA4CC3A),
    case("Green", 0x4E9E5C, false, Solid, false, 0x4E9E5C),
    case("Teal", 0x1E9AA6, false, Solid, false, 0x1E9AA6),
    case("Surf Blue", 0x4E9CD4, false, Solid, false, 0x4E9CD4),
    case("Blue", 0x406CB8, false, Solid, false, 0x406CB8),
    case("Purple", 0x6A4C9C, false, Solid, false, 0x6A4C9C),
    case("Black", 0x2E2C2C, false, Solid, false, 0x2E2C2C),
    case("White", 0xECE8DE, true, Solid, false, 0xECE8DE),
    case("Clear", 0xE8ECEC, false, Clear, false, 0xF5F7F7),
    case("Clear Black", 0x4C4C54, false, Clear, false, 0x4D4D54),
    case("Atomic Purple", 0x8E76C4, false, Clear, false, 0xD6BDFF),
    case("Fuchsia", 0xD0509C, false, Clear, false, 0xFFB3E0),
    case("Clear Yellow", 0xF0D870, false, Clear, false, 0xFFE59E),
    case("Clear Green", 0x8CD08C, false, Clear, false, 0x9EEB9E),
    case("Ice Blue", 0xB0D4EC, false, Clear, false, 0xCCEBFF),
    case("Glacier", 0x94ACE2, false, Clear, false, 0xB8C8FF),
    case("Water Blue", 0x8CC0E4, false, Clear, false, 0xA8DBFF),
    case("Midnight Blue", 0x4C5CB4, false, Clear, false, 0x667AE0),
    case("Silver", 0xB8BABE, true, Metallic, false, 0xB8BABE),
    case("Gold", 0xBAA371, true, Metallic, false, 0xBAA371),
    case("Pearl White", 0xEEEDE8, true, Pearl, false, 0xEEEDE8),
    case("Pearl Pink", 0xDEC1CB, true, Pearl, false, 0xDEC1CB),
    case("Pearl Blue", 0x93B9D0, true, Pearl, false, 0x93B9D0),
    case("Pearl Green", 0xB3CCB1, true, Pearl, false, 0xB3CCB1),
    // The Game Boy Advance SP.
    case("DMG Grey", 0xC4C0B7, true, Solid, false, 0xC4C0B7),
    case("Grape", 0x684E92, false, Solid, false, 0x684E92),
    case("Original", 0x544A98, false, Solid, false, 0x544A98),
    case("Flame", 0xCC3E30, false, Solid, false, 0xCC3E30),
    case("Berry", 0xB8243C, false, Solid, false, 0xB8243C),
    case("Pink", 0xE59BB5, true, Solid, false, 0xE59BB5),
    case("Yellow", 0xE6BC32, true, Solid, false, 0xE6BC32),
    case("Lime Green", 0xA4CC3A, true, Solid, false, 0xA4CC3A),
    case("Green", 0x4E9E5C, false, Solid, false, 0x4E9E5C),
    case("Teal", 0x1E9AA6, false, Solid, false, 0x1E9AA6),
    case("Surf Blue", 0x4E9CD4, false, Solid, false, 0x4E9CD4),
    case("Blue", 0x406CB8, false, Solid, false, 0x406CB8),
    case("Purple", 0x6A4C9C, false, Solid, false, 0x6A4C9C),
    case("Onyx", 0x2E2C2C, false, Solid, false, 0x2E2C2C),
    case("Arctic", 0xECE8DE, true, Solid, false, 0xECE8DE),
    case("Clear", 0xE8ECEC, false, Clear, false, 0xFFFFFF),
    case("Clear Black", 0x4C4C54, false, Clear, false, 0x4D4D54),
    case("Atomic Purple", 0x8E76C4, false, Clear, false, 0xD6BDFF),
    case("Fuchsia", 0xD0509C, false, Clear, false, 0xFFB3E0),
    case("Clear Yellow", 0xF0D870, false, Clear, false, 0xFFE59E),
    case("Clear Green", 0x8CD08C, false, Clear, false, 0x9EEB9E),
    case("Ice Blue", 0xB0D4EC, false, Clear, false, 0xCCEBFF),
    case("Glacier", 0x94ACE2, false, Clear, false, 0xB8C8FF),
    case("Water Blue", 0x8CC0E4, false, Clear, false, 0xA8DBFF),
    case("Midnight Blue", 0x4C5CB4, false, Clear, false, 0x667AE0),
    case("Platinum", 0xB8BABE, true, Metallic, false, 0xB8BABE),
    case("Gold", 0xBAA371, true, Metallic, false, 0xBAA371),
    case("Pearl White", 0xEEEDE8, true, Pearl, false, 0xEEEDE8),
    case("Pearl Pink", 0xDEC1CB, true, Pearl, false, 0xDEC1CB),
    case("Pearl Blue", 0x93B9D0, true, Pearl, false, 0x93B9D0),
    case("Pearl Green", 0xB3CCB1, true, Pearl, false, 0xB3CCB1),
];

/// How many colours each Game Boy slot model offers.
pub const SLOT_COLOURS: usize = 31;

/// The NES and SNES slot studio (Roy 2026-11-12, st5): the console's own Original, the dark
/// housing and the DMG's grey, each a finished band of its own (tools/carts/tv_slots.py).
pub const TV_SLOT_CASES: [SlotCase; 3] = [
    case("Original", 0xC0BFBD, true, Solid, false, 0xC0BFBD),
    case("Dark", 0x2A2A2D, false, Solid, false, 0x2A2A2D),
    case("DMG Grey", 0xC4C0B7, true, Solid, false, 0xC4C0B7),
];

/// Where the Game Boy Color slot's cases start in GB_SLOT_CASES, and how many it has (one
/// model).
pub const GBC_SLOT_FIRST: usize = 2 * SLOT_COLOURS;
pub const GBC_SLOT_MODELS: [usize; 1] = [SLOT_COLOURS];

/// Where the Game Boy Advance slot's cases start in GB_SLOT_CASES, and how many each of its
/// models has: one, the SP (no plain GBA, Roy 2026-11-06).
pub const GBA_SLOT_FIRST: usize = 3 * SLOT_COLOURS;
pub const GBA_SLOT_MODELS: [usize; 1] = [SLOT_COLOURS];

/// A case's model: 0 DMG, 1 Pocket/Light, 2 Game Boy Color, 3 GBA SP.
pub fn slot_model(case: usize) -> usize {
    match case {
        c if c >= GBA_SLOT_FIRST => 3,
        c if c >= GBC_SLOT_FIRST => 2,
        c if c >= GB_SLOT_MODELS[0] => 1,
        _ => 0,
    }
}

/// A Game Boy slot case past the grey (1.. in GB_SLOT_CASES, here from 0), drawn once from its
/// model's neutral image in its paint and finish. Not kept: the frontend paints a case only
/// when it is first shown (136 of them).
pub fn slot_gb_case(case: usize, front: bool) -> Vec<u8> {
    let mut rgba = paint_slot(&GB_SLOT_CASES[case + 1], slot_model(case + 1), front);
    bleed(&mut rgba, SLOT_W, SLOT_H);
    rgba
}

/// The Game Boy Color slot in the dark or the light theme: the Color's own shallow mouth
/// (the slot studio's GBC model), in the theme's housing colour (Roy 2026-11-07).
pub fn slot_gbc_theme(light: bool, front: bool) -> &'static [u8] {
    static P: [OnceLock<Vec<u8>>; 4] = [const { OnceLock::new() }; 4];
    P[usize::from(light) * 2 + usize::from(front)].get_or_init(|| {
        let paint = if light { 0xB2ADA3 } else { 0x2A2A2D };
        let case = case("Theme", paint, light, SlotFinish::Solid, false, paint);
        let mut rgba = paint_slot(&case, 2, front);
        bleed(&mut rgba, SLOT_W, SLOT_H);
        rgba
    })
}

/// The gold of the Favorites shelf's slots (Roy, sh11): its own, deeper and warmer than the
/// studio's two Golds, and not on offer in the studio.
pub const FAV_GOLD: u32 = 0xE3A72F;
pub const FAV_GOLD_KINDS: usize = 6;

/// A slot on the Favorites shelf, in gold metal: `kind` 0 DMG, 1 Pocket/Light, 2 Game Boy
/// Color, 3 GBA SP (their neutral images under the metal's ramp), 4 NES, 5 SNES (their grey
/// bands turned to gold).
pub fn slot_fav_gold(kind: usize, front: bool) -> &'static [u8] {
    static P: [OnceLock<Vec<u8>>; FAV_GOLD_KINDS * 2] = [const { OnceLock::new() }; FAV_GOLD_KINDS * 2];
    P[kind * 2 + usize::from(front)].get_or_init(|| {
        if kind >= 4 {
            let mut rgba = gilded(slot_tv_grey(kind == 4, front), u32::from(front), 0.0);
            fav_light(&mut rgba);
            return rgba;
        }
        let case = case("Favorites", FAV_GOLD, true, SlotFinish::Metallic, kind == 0, FAV_GOLD);
        let mut rgba = paint_slot(&case, kind, front);
        fav_light(&mut rgba);
        bleed(&mut rgba, SLOT_W, SLOT_H);
        rgba
    })
}

/// The Favorites gold lit from above: brighter at the top, down to a deeper, warmer gold at
/// the bottom, so it reads as metal rather than flat yellow (Roy 2026-11-17, A).
fn fav_light(rgba: &mut [u8]) {
    let rows = (rgba.len() / 4 / SLOT_W as usize).max(2);
    for (i, px) in rgba.chunks_exact_mut(4).enumerate() {
        let t = (i / SLOT_W as usize) as f32 / (rows - 1) as f32;
        let g = 1.06 - 0.28 * t;
        let warm = ((t - 0.5) * 0.25).max(0.0);
        let k = [g, g * (1.0 - warm * 0.5), g * (1.0 - warm)];
        for c in 0..3 {
            px[c] = (f32::from(px[c]) * k[c]).round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// The NES lid in the Favorites gold.
pub fn nes_lid_gold() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    P.get_or_init(|| gilded(nes_lid_grey(), 2, (SLOT_W - NES_LID_W) as f32 / 2.0))
}

/// DMG-grey plastic turned to the Favorites gold: the grey's own light and shade carried
/// over the metal's ramp (deep shadow, the paint, a pale highlight); what is darker than
/// plastic (the mouth, the seams) stays dark.
fn gilded(grey: &[u8], seed: u32, x_off: f32) -> Vec<u8> {
    let paint = rgb(FAV_GOLD).map(f32::from);
    let base = rgb(0xC4C0B7).map(f32::from);
    let base = (base[0] + base[1] + base[2]) / 3.0;
    let mut rgba = grey.to_vec();
    let w = SLOT_W as f32;
    let row = (rgba.len() / 4 / SLOT_H as usize) as u32;
    for (i, px) in rgba.chunks_exact_mut(4).enumerate() {
        let v = (f32::from(px[0]) + f32::from(px[1]) + f32::from(px[2])) / 3.0 / base;
        // the same metal as the Game Boys' gold slots (Roy 2026-11-12): their broad sheen high
        // on the face and their fine flake, on the plastic only
        let (x, y) = ((i as u32 % row) as f32 + x_off, (i as u32 / row) as f32);
        let plastic = (v / 0.8).clamp(0.0, 1.0);
        let sheen = 0.16
            * (-((y - 9.0) / 7.0).powi(2)).exp()
            * (0.75 + 0.25 * (x / w * std::f32::consts::TAU * 1.5 + 0.6).cos());
        let (nz, u) = noise(i as u32, 7 + seed);
        let flake = nz * 0.022 + if u > 0.992 { 0.10 } else { 0.0 };
        // the grey's own rim is lit already: no sheen on top of it
        let lit = ((1.12 - v) / 0.12).clamp(0.0, 1.0);
        let v = v + (v * sheen * lit + flake) * plastic;
        let t_lo = ((1.0 - v) / 0.45).clamp(0.0, 1.0);
        let t_hi = ((v - 1.0) / 0.28).clamp(0.0, 1.0).powf(0.9);
        // Below the ramp's foot the grey's own darkness takes over.
        let dark = (v / 0.55).min(1.0);
        for c in 0..3 {
            let p = paint[c];
            let out = p + (p * 0.6 - p) * t_lo;
            let out = out + (p + (255.0 - p) * 0.72 - out) * t_hi;
            px[c] = (out * dark).round().clamp(0.0, 255.0) as u8;
        }
    }
    rgba
}

fn decoded(bytes: &[u8]) -> Vec<u8> {
    let (rgba, w, h) = decode_png_bytes(bytes).expect("a slot png decodes");
    assert_eq!((w, h), (SLOT_W, SLOT_H), "slot png is not {SLOT_W}x{SLOT_H}");
    rgba
}

/// A slot case in its paint: K + paint x T for Solid; the metal's ramp over T for Pearl and
/// Metallic (the metal ramp the Pocket's silver and gold were drawn with: deep shadow, the paint, a pale highlight, a
/// broad sheen high on the face and fine flake; Pearl softer, milkier and with less flake); the
/// colourless clear case through the paint for Clear, its plate kept.
pub fn paint_slot(case: &SlotCase, model: usize, front: bool) -> Vec<u8> {
    let paint = case.paint.map(f32::from);
    if case.finish == SlotFinish::Clear {
        let (pngs, mask) = SLOT_CLEAR_PNG[model];
        let mut rgba = decoded(pngs[usize::from(front)]);
        let mask = mask.filter(|_| !front).map(decoded);
        for (i, px) in rgba.chunks_exact_mut(4).enumerate() {
            let m = mask.as_ref().map_or(1.0, |m| f32::from(m[i * 4]) / 255.0);
            for c in 0..3 {
                let k = 1.0 + (paint[c] / 255.0 - 1.0) * m;
                px[c] = (f32::from(px[c]) * k).round().clamp(0.0, 255.0) as u8;
            }
        }
        return rgba;
    }
    let [k, t] = SLOT_NEUTRAL_PNG[model][usize::from(front)];
    let mut rgba = decoded(k);
    let t = decoded(t);
    if !matches!(case.finish, SlotFinish::Pearl | SlotFinish::Metallic) {
        for (px, t) in rgba.chunks_exact_mut(4).zip(t.chunks_exact(4)) {
            let tone = f32::from(t[0]) / T_SCALE;
            for c in 0..3 {
                px[c] = (f32::from(px[c]) + paint[c] * tone).round().clamp(0.0, 255.0) as u8;
            }
        }
        return rgba;
    }
    // Pearl and Metallic: T's alpha is the plastic's coverage, so the shading is T / coverage.
    // The metal's ramp is not linear, and on a 1 px line (the lip's catch-light, the wall's
    // edge) it turns the edge's smoothing into steps; so the ramp follows the shading softened
    // a little, the fine detail above it is added back linearly, and all of it is laid down by
    // coverage, as Solid's edges are.
    let n = rgba.len() / 4;
    let cov: Vec<f32> = (0..n).map(|i| f32::from(t[i * 4 + 3]) / 255.0).collect();
    let shade: Vec<f32> =
        (0..n).map(|i| if cov[i] > 0.0 { f32::from(t[i * 4]) / T_SCALE / cov[i] } else { 0.0 }).collect();
    let weighted: Vec<f32> = shade.iter().zip(&cov).map(|(s, c)| s * c).collect();
    let (soft_w, soft_c) = (soften(&weighted), soften(&cov));
    let pearl = case.finish == SlotFinish::Pearl;
    let (w, seed) = (SLOT_W as f32, 7 + u32::from(front));
    for (i, px) in rgba.chunks_exact_mut(4).enumerate() {
        if cov[i] <= 0.0 {
            continue;
        }
        let base = soft_w[i] / soft_c[i].max(1e-6);
        let detail = shade[i] - base;
        let (x, y) = ((i as u32 % SLOT_W) as f32, (i as u32 / SLOT_W) as f32);
        let sheen = 0.16
            * (-((y - 9.0) / 7.0).powi(2)).exp()
            * (0.75 + 0.25 * (x / w * std::f32::consts::TAU * 1.5 + 0.6).cos());
        let fk = if pearl { 0.35 } else { 1.0 };
        let (nz, u) = noise(i as u32, seed);
        let flake = fk * (nz * 0.022 + if u > 0.992 { 0.10 } else { 0.0 });
        let v = base * (1.0 + sheen) + flake;
        let t_lo = ((1.0 - v) / 0.45).clamp(0.0, 1.0);
        let t_hi = ((v - 1.0) / 0.28).clamp(0.0, 1.0).powf(0.9);
        let hi_k = if pearl { 0.88 } else { 0.72 };
        let glow = if pearl { 0.10 * (-((y - 12.0) / 10.0).powi(2)).exp() } else { 0.0 };
        for c in 0..3 {
            let p = paint[c];
            let (lo, hi) = (p * 0.6, p + (255.0 - p) * hi_k);
            let out = p + (lo - p) * t_lo;
            let out = out + (hi - out) * t_hi;
            let out = out + (255.0 - out) * glow;
            let out = if detail > 0.0 { out + (255.0 - out) * detail * 1.5 } else { out + out * detail };
            px[c] = (f32::from(px[c]) + out * cov[i]).round().clamp(0.0, 255.0) as u8;
        }
    }
    rgba
}

/// A slot-sized plane softened by a 5-tap binomial blur across, then down (edges held).
fn soften(src: &[f32]) -> Vec<f32> {
    const K: [f32; 5] = [1.0 / 16.0, 4.0 / 16.0, 6.0 / 16.0, 4.0 / 16.0, 1.0 / 16.0];
    let (w, h) = (SLOT_W as i32, (src.len() / SLOT_W as usize) as i32);
    let pass = |src: &[f32], dx: i32, dy: i32| -> Vec<f32> {
        (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                K.iter()
                    .enumerate()
                    .map(|(k, f)| {
                        let o = k as i32 - 2;
                        let (sx, sy) = ((x + o * dx).clamp(0, w - 1), (y + o * dy).clamp(0, h - 1));
                        f * src[(sy * w + sx) as usize]
                    })
                    .sum()
            })
            .collect()
    };
    pass(&pass(src, 1, 0), 0, 1)
}

/// Per pixel: about normal noise, and an even one in 0..1.
fn noise(i: u32, seed: u32) -> (f32, f32) {
    let mut h = i.wrapping_mul(0x9e37_79b9) ^ seed.wrapping_mul(0x85eb_ca6b);
    let mut next = || {
        h ^= h >> 15;
        h = h.wrapping_mul(0x2c1b_3c6d);
        h ^= h >> 12;
        h = h.wrapping_mul(0x297a_2d39);
        h ^= h >> 15;
        (h >> 8) as f32 / (1u32 << 24) as f32
    };
    let n = (next() + next() + next() - 1.5) * 2.0;
    (n, next())
}

/// The NES's lid in its own light grey (Originals).
pub fn nes_lid_orig() -> &'static [u8] {
    static P: OnceLock<Vec<u8>> = OnceLock::new();
    load(NES_LID_ORIG_PNG, NES_LID_W, SLOT_H, &P)
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

// The Super Game Boys have no plain plastic of their own to dye: every part is the finished
// cart, and only the silhouette is its own.
const SGB_PNGS: [&[u8]; 5] = [
    include_bytes!("../../../assets/sgb_ntsc_cart.png"),
    include_bytes!("../../../assets/sgb_face_ntsc.png"),
    include_bytes!("../../../assets/sgb_face_ntsc.png"),
    include_bytes!("../../../assets/sgb_face_ntsc.png"),
    include_bytes!("../../../assets/sgb_face_ntsc.png"),
];
const SGB_PAL_PNGS: [&[u8]; 5] = [
    include_bytes!("../../../assets/sgb_pal_cart.png"),
    include_bytes!("../../../assets/sgb_face_pal.png"),
    include_bytes!("../../../assets/sgb_face_pal.png"),
    include_bytes!("../../../assets/sgb_face_pal.png"),
    include_bytes!("../../../assets/sgb_face_pal.png"),
];

/// The Super Game Boy models, each its own finished cart.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SgbModel {
    /// The American one: ridged, grey, the red and blue sticker.
    Ntsc,
    /// The European one: light grey, the Super NES dots on its sticker.
    Pal,
    /// The Japanese one: as the European, the Super Famicom seal on its sticker.
    Sfc,
    /// The Super Game Boy 2: clear blue, its own purple sticker and the link port.
    Sgb2,
}

impl SgbModel {
    pub const ALL: [SgbModel; 4] = [SgbModel::Ntsc, SgbModel::Pal, SgbModel::Sfc, SgbModel::Sgb2];

    pub fn mould(self) -> Mould {
        match self {
            SgbModel::Ntsc => Mould::SgbNtsc,
            _ => Mould::SgbPal,
        }
    }
}

pub fn sgb_face(model: SgbModel) -> &'static [u8] {
    static SLOTS: [OnceLock<Vec<u8>>; 4] = [const { OnceLock::new() }; 4];
    let bytes: &[u8] = match model {
        SgbModel::Ntsc => include_bytes!("../../../assets/sgb_face_ntsc.png"),
        SgbModel::Pal => include_bytes!("../../../assets/sgb_face_pal.png"),
        SgbModel::Sfc => include_bytes!("../../../assets/sgb_face_sfc.png"),
        SgbModel::Sgb2 => include_bytes!("../../../assets/sgb_face_sgb2.png"),
    };
    let (w, h) = model.mould().size();
    load(bytes, w, h, &SLOTS[model as usize])
}

/// The Super Game Boy as the camera closes in on it: its face at the band's size, the top the
/// band's own art (`tools/sgb/sgb_zoom_face.py`), so the zoom stays as sharp as the slots and
/// lands on the band unchanged (Roy, xh2). (rgba, w, h)
pub fn sgb_zoom_face(model: SgbModel) -> (&'static [u8], u32, u32) {
    static SLOTS: [OnceLock<Vec<u8>>; 4] = [const { OnceLock::new() }; 4];
    let bytes: &[u8] = match model {
        SgbModel::Ntsc => include_bytes!("../../../assets/sgb_zoom_ntsc.png"),
        SgbModel::Pal => include_bytes!("../../../assets/sgb_zoom_pal.png"),
        SgbModel::Sfc => include_bytes!("../../../assets/sgb_zoom_sfc.png"),
        SgbModel::Sgb2 => include_bytes!("../../../assets/sgb_zoom_sgb2.png"),
    };
    let (w, h) = match model {
        SgbModel::Ntsc => (543, 475),
        _ => (472, 448),
    };
    (load(bytes, w, h, &SLOTS[model as usize]), w, h)
}

/// The slot band while the Super Game Boy is in: its top around the Game Boy cart, behind
/// (`front` false) or in front of it.
pub fn sgb_band(model: SgbModel, front: bool) -> &'static [u8] {
    static SLOTS: [[OnceLock<Vec<u8>>; 2]; 4] = [const { [const { OnceLock::new() }; 2] }; 4];
    let bytes: [&[u8]; 2] = match model {
        SgbModel::Ntsc => [
            include_bytes!("../../../assets/sgb_band_ntsc_back.png"),
            include_bytes!("../../../assets/sgb_band_ntsc_front.png"),
        ],
        SgbModel::Pal => [
            include_bytes!("../../../assets/sgb_band_pal_back.png"),
            include_bytes!("../../../assets/sgb_band_pal_front.png"),
        ],
        SgbModel::Sfc => [
            include_bytes!("../../../assets/sgb_band_sfc_back.png"),
            include_bytes!("../../../assets/sgb_band_sfc_front.png"),
        ],
        SgbModel::Sgb2 => [
            include_bytes!("../../../assets/sgb_band_sgb2_back.png"),
            include_bytes!("../../../assets/sgb_band_sgb2_front.png"),
        ],
    };
    load(bytes[usize::from(front)], SLOT_W, SGB_BAND_H, &SLOTS[model as usize][usize::from(front)])
}

pub fn tv_art(mould: Mould, part: TvPart) -> &'static [u8] {
    static SLOTS: [[OnceLock<Vec<u8>>; 5]; 5] = [const { [const { OnceLock::new() }; 5] }; 5];
    let (row, pngs) = match mould {
        Mould::Nes => (0, NES_PNGS),
        Mould::SnesNtsc => (1, SNES_PNGS),
        Mould::SnesPal => (2, SNES_PAL_PNGS),
        Mould::SgbNtsc => (3, SGB_PNGS),
        Mould::SgbPal => (4, SGB_PAL_PNGS),
        _ => unreachable!("only a TV cart has TV art"),
    };
    let (w, h) = mould.size();
    load(pngs[part as usize], w, h, &SLOTS[row][part as usize])
}
