use slot_gfx::{OUT_H, OUT_W};

use crate::art::render_svg;
use crate::bitmaps;
use crate::cart::CartFace;
use crate::text;

/// Only the socket of the console strip, cut out and faded off at its sides and bottom like the
/// cable's own end (Roy 2026-11-12): no bar along the bottom, the legend has the floor.
pub const PORT_W: u32 = 240;
pub const PORT_H: u32 = bitmaps::PORT_H;
/// How far the whole link scene sits above where the strip used to be, so the port clears the
/// help line and the legend under it.
pub const LINK_LIFT: f32 = 100.0;
pub const PORT_Y: f32 = (OUT_H - PORT_H) as f32 - LINK_LIFT;
const PORT_FADE_X: f32 = 72.0;
const PORT_FADE_Y: f32 = 26.0;
pub const PLUG_W: u32 = 92;
pub const PLUG_H: u32 = 244;
pub const PLUG_TIP_X: f32 = 46.0;
pub const ADAPTER_W: u32 = bitmaps::ADAPTER_W;
pub const ADAPTER_H: u32 = bitmaps::ADAPTER_H;
/// Centre of the connector. It is the same width as the plug tip and sits on the screen centre.
pub const ADAPTER_BASE_X: f32 = 145.0;
/// Top of the connector. The connector is the bottom 31px, so a seated base puts the tip at `PORT_Y + 31`, the same depth as a plug.
pub const ADAPTER_BASE_Y: f32 = 149.0;
const CX: f32 = OUT_W as f32 / 2.0;
pub const ARCS: [(f32, f32, u32, u32); 3] = [
    (CX + 148.0, 324.0 - LINK_LIFT, 16, 56),
    (CX + 170.0, 308.0 - LINK_LIFT, 20, 88),
    (CX + 192.0, 292.0 - LINK_LIFT, 25, 120),
];
pub const CLICKS_X: f32 = CX - CLICKS_W as f32 / 2.0;
/// Just above the port lip. The port used to start at y=388 with the clicks at 360.
pub const CLICKS_Y: f32 = PORT_Y - 28.0;
pub const CLICKS_W: u32 = 180;
pub const CLICKS_H: u32 = 30;
pub const ARROW_W: u32 = 16;
pub const ARROW_H: u32 = 22;
pub const ARROW_LEFT_X: f32 = CX - 92.0;
pub const ARROW_RIGHT_X: f32 = CX + 76.0;
pub const ARROW_Y: f32 = 52.0;
/// The network notice: a line in the help line's place, centred just above the legend.
pub const NET_W: u32 = 240;
pub const NET_H: u32 = 22;
pub const NET_X: f32 = CX - NET_W as f32 / 2.0;
pub const NET_Y: f32 = crate::quick_menu::HELP_Y;

const ARC_INK: &str = "#f6f4ef";

pub struct LinkArt {
    pub port: CartFace,
    pub port_light: CartFace,
    /// The port's shading (lip, triangle, shadows) as white and black over its colour.
    pub port_shade: CartFace,
    pub plug_host: CartFace,
    pub plug_join: CartFace,
    /// The Game Boy's cable: the host's plug in black instead of purple (Roy 2026-11-12), and
    /// the joiner's black too, cable and all (Roy 2026-11-13).
    pub plug_gb: CartFace,
    pub plug_gb_join: CartFace,
    pub adapter: CartFace,
    pub arcs_right: [CartFace; 3],
    pub arcs_left: [CartFace; 3],
    pub clicks: CartFace,
    pub arrow_left: CartFace,
    pub arrow_right: CartFace,
    /// The plate for a link over the home network, and for the private one.
    pub net_home: CartFace,
    pub net_direct: CartFace,
}

pub fn link_art() -> LinkArt {
    let arcs_right = [arc_face(0), arc_face(1), arc_face(2)];
    let arcs_left = [
        mirror(&arcs_right[0]),
        mirror(&arcs_right[1]),
        mirror(&arcs_right[2]),
    ];
    LinkArt {
        port: port_cut(bitmaps::link_port()).0,
        port_light: port_cut(bitmaps::link_port()).0,
        port_shade: port_cut(bitmaps::link_port()).1,
        plug_host: bitmap_face(bitmaps::link_plug_host(), PLUG_W, PLUG_H),
        plug_join: bitmap_face(bitmaps::link_plug_join(), PLUG_W, PLUG_H),
        plug_gb: blacken(&bitmap_face(bitmaps::link_plug_host(), PLUG_W, PLUG_H)),
        plug_gb_join: blacken(&bitmap_face(bitmaps::link_plug_join(), PLUG_W, PLUG_H)),
        adapter: bitmap_face(bitmaps::link_adapter(), ADAPTER_W, ADAPTER_H),
        arcs_right,
        arcs_left,
        clicks: svg_face(
            &format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {CLICKS_W} {CLICKS_H}"><g stroke="{ARC_INK}" stroke-width="4" stroke-linecap="round"><path d="M34 20 L20 4"/><path d="M146 20 L160 4"/><path d="M26 26 L4 26"/><path d="M154 26 L176 26"/></g></svg>"#
            ),
            CLICKS_W,
            CLICKS_H,
        ),
        arrow_left: svg_face(&arrow_svg("M14 2 L2 11 L14 20 Z"), ARROW_W, ARROW_H),
        arrow_right: svg_face(&arrow_svg("M2 2 L14 11 L2 20 Z"), ARROW_W, ARROW_H),
        net_home: net_plate(true),
        net_direct: net_plate(false),
    }
}

/// A small die-cut plate, printed like the adapter's label: a glyph and two words, on the
/// console's own slate so it reads as something on the device and not as a control. The home
/// network is a Wi-Fi mark; the private one is two nodes joined, which is what it is.
fn net_plate(home: bool) -> CartFace {
    // The same glyphs as before, at the help line's size, white; the screen dims it like a help line.
    let glyph = if home {
        r#"<g fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"><path d="M3.5 8.6 Q10 3.4 16.5 8.6"/><path d="M6.4 12 Q10 9.2 13.6 12"/></g><circle cx="10" cy="15.6" r="1.8" fill="white"/>"#
    } else {
        r#"<path d="M5 11 H15" stroke="white" stroke-width="1.8" stroke-linecap="round"/><circle cx="3.2" cy="11" r="2.6" fill="white"/><circle cx="16.8" cy="11" r="2.6" fill="white"/>"#
    };
    let label = if home { "Home Wi-Fi" } else { "Direct Link" };
    let px = crate::quick_menu::HELP_PX;
    let width = text::label_font().map_or(0.0, |font| {
        let layout = text::fit(font, label, f32::MAX, 1, px, px);
        layout
            .lines
            .first()
            .map(|l| text::line_width(font, l, px, layout.tracking))
            .unwrap_or(0.0)
    });
    const GLYPH_W: f32 = 20.0;
    const GAP: f32 = 8.0;
    let left = ((NET_W as f32 - GLYPH_W - GAP - width) / 2.0).round();
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {NET_W} {NET_H}"><g transform="translate({left} 0)">{glyph}</g></svg>"#
    );
    let mut face = svg_face(&svg, NET_W, NET_H);
    stamp(&mut face, left + GLYPH_W + GAP, 16.5, label, px, [0xff, 0xff, 0xff]);
    face
}

/// The link port's place until the shelf's own slot is known: nothing. The second face is left
/// empty too.
fn port_cut(_strip: &[u8]) -> (CartFace, CartFace) {
    let blank = || CartFace { rgba: vec![0; (PORT_W * PORT_H * 4) as usize], w: PORT_W, h: PORT_H };
    (blank(), blank())
}

/// The link port: the plain piece of the slot on the shelf left of its mouth, as it is, in its
/// own colour, no mouth (Roy 2026-11-13), faded off at its sides and bottom. `back` is a slot
/// set's back image, `SLOT_W` wide; only its top `PORT_H` rows are used. The piece is read from
/// the side's first `SIDE_SPAN` columns, to and fro, so no slot's mouth (the Super Game Boy's
/// starts at 89) or clear case's board comes into it.
pub fn port_from_slot(back: &[u8]) -> Vec<u8> {
    let (w, h, sw) = (PORT_W as usize, PORT_H as usize, bitmaps::SLOT_W as usize);
    let smooth = |t: f32| {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let mut face = vec![0u8; w * h * 4];
    if back.len() < sw * h * 4 {
        return face;
    }
    for y in 0..h {
        let fy = smooth((h - 1 - y) as f32 / PORT_FADE_Y);
        for x in 0..w {
            let (period, i) = (2 * SIDE_SPAN, x % (2 * SIDE_SPAN));
            let src = SIDE_FROM + if i < SIDE_SPAN { i } else { period - 1 - i };
            let (s, d) = ((y * sw + src) * 4, (y * w + x) * 4);
            let fx = smooth(x.min(w - 1 - x) as f32 / PORT_FADE_X);
            face[d..d + 3].copy_from_slice(&back[s..s + 3]);
            face[d + 3] = (f32::from(back[s + 3]) * fx * fy).round() as u8;
        }
    }
    face
}

const SIDE_FROM: usize = 8;
const SIDE_SPAN: usize = 72;

/// The plug's cable and body run down to here; the metal tip starts below.
const BODY_ROWS: usize = 213;
/// Grey (the cable, the joiner's plastic) is three times as light as the host's purple: this
/// brings its black down to the purple's, so cable and plug are one black (Roy 2026-11-13).
const GREY_DARKEN: f32 = 0.335;

/// The plug's plastic and its cable turned black, the shading kept; the metal tip, grey
/// already, stays.
fn blacken(face: &CartFace) -> CartFace {
    let mut rgba = face.rgba.clone();
    let w = face.w as usize;
    for (i, px) in rgba.chunks_exact_mut(4).enumerate() {
        let (r, g, b) = (px[0] as f32, px[1] as f32, px[2] as f32);
        let (hi, lo) = (r.max(g).max(b), r.min(g).min(b));
        let grey = hi - lo < 40.0;
        if grey && i / w >= BODY_ROWS {
            continue;
        }
        let darken = if grey { GREY_DARKEN } else { 1.0 };
        let l = 0.3 * r + 0.59 * g + 0.11 * b;
        let v = (18.0 + l * 0.45 * darken).min(255.0) as u8;
        px[0] = v;
        px[1] = v;
        px[2] = v.saturating_add(2);
    }
    CartFace { rgba, w: face.w, h: face.h }
}

fn bitmap_face(rgba: &[u8], w: u32, h: u32) -> CartFace {
    CartFace {
        rgba: rgba.to_vec(),
        w,
        h,
    }
}

fn svg_face(svg: &str, w: u32, h: u32) -> CartFace {
    let rgba = render_svg(svg, w, h).unwrap_or_else(|| vec![0; (w * h * 4) as usize]);
    CartFace { rgba, w, h }
}

fn arrow_svg(path: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {ARROW_W} {ARROW_H}"><path d="{path}" fill="{ARC_INK}"/></svg>"#
    )
}

fn arc_face(ring: usize) -> CartFace {
    let (_, _, w, h) = ARCS[ring];
    let bulge = [16.0, 24.0, 34.0][ring];
    let path = format!("M4 4 Q{} {} 4 {}", 4.0 + bulge, h as f32 / 2.0, h - 4);
    svg_face(
        &format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}"><path d="{path}" fill="none" stroke="{ARC_INK}" stroke-width="5" stroke-linecap="round"/></svg>"#
        ),
        w,
        h,
    )
}

fn mirror(face: &CartFace) -> CartFace {
    let mut rgba = vec![0u8; face.rgba.len()];
    for y in 0..face.h {
        for x in 0..face.w {
            let s = ((y * face.w + x) * 4) as usize;
            let d = ((y * face.w + (face.w - 1 - x)) * 4) as usize;
            rgba[d..d + 4].copy_from_slice(&face.rgba[s..s + 4]);
        }
    }
    CartFace {
        rgba,
        w: face.w,
        h: face.h,
    }
}

fn stamp(face: &mut CartFace, x: f32, baseline: f32, line: &str, px: f32, ink: [u8; 3]) {
    let Some(font) = text::label_font() else {
        return;
    };
    let layout = text::fit(font, line, f32::MAX, 1, px, px);
    let width = layout
        .lines
        .first()
        .map(|l| text::line_width(font, l, px, layout.tracking))
        .unwrap_or(0.0);
    let (bw, bh) = ((width.ceil() as u32).max(1), (px * 1.6).ceil() as u32);
    let cov = text::coverage(bw, bh, &layout);
    let left = x.round() as i32;
    let top = (baseline - px * 1.2).round() as i32;
    for row in 0..bh as i32 {
        for col in 0..bw as i32 {
            let a = cov[(row as u32 * bw + col as u32) as usize] as u32;
            let (dx, dy) = (left + col, top + row);
            if a == 0 || dx < 0 || dy < 0 || dx >= face.w as i32 || dy >= face.h as i32 {
                continue;
            }
            let d = ((dy as u32 * face.w + dx as u32) * 4) as usize;
            for (k, c) in ink.iter().enumerate() {
                face.rgba[d + k] =
                    ((*c as u32 * a + face.rgba[d + k] as u32 * (255 - a)) / 255) as u8;
            }
            face.rgba[d + 3] = face.rgba[d + 3].max(a as u8);
        }
    }
}
