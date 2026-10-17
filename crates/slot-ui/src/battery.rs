use std::sync::OnceLock;

use slot_gfx::{Draw, TexId};
use slot_power::{Battery, Charge};

use crate::footer::Printed;
use crate::icon::{icon_glyph_box, Icon};
use crate::plate::HINT_H;

pub const BOLT_PX: f32 = 18.0;

pub const GAUGE_W: f32 = 24.0;
pub const GAUGE_H: f32 = 12.0;
const _: () = assert!(GAUGE_H < GAUGE_W);
const NUB_W: f32 = 3.0;
const NUB_H: f32 = 6.0;
pub const WALL: f32 = 2.0;
const GAP: f32 = 7.0;

const BOLT_MAX_SIDE: f32 = 14.0;
const BOLT_GAP: f32 = 5.0;
const _: () = assert!(BOLT_GAP > 0.0);

pub fn draw_gauge(
    x: f32,
    y: f32,
    battery: Option<Battery>,
    percent: Printed,
    bolt: Option<TexId>,
    ink: [u8; 3],
    out: &mut Vec<Draw>,
) {
    let Some(b) = battery else {
        return;
    };
    // The panel is 1:1, so every edge sits on a whole pixel.
    let (x, y) = (x.round(), y.round());
    let colour = [
        ink[0] as f32 / 255.0,
        ink[1] as f32 / 255.0,
        ink[2] as f32 / 255.0,
        1.0,
    ];

    let rect = |x: f32, y: f32, w: f32, h: f32, out: &mut Vec<Draw>| {
        out.push(Draw::Rect { x, y, w, h, colour });
    };

    let (bolt_w, bolt_h) = bolt_size();
    let cx = x + BOLT_MAX_SIDE + BOLT_GAP;

    rect(cx, y, GAUGE_W, WALL, out);
    rect(cx, y + GAUGE_H - WALL, GAUGE_W, WALL, out);
    rect(cx, y, WALL, GAUGE_H, out);
    rect(cx + GAUGE_W - WALL, y, WALL, GAUGE_H, out);
    rect(cx + GAUGE_W, y + (GAUGE_H - NUB_H) / 2.0, NUB_W, NUB_H, out);

    let inner = GAUGE_W - 4.0 * WALL;
    let fill = (inner * f32::from(b.percent.min(100)) / 100.0).round();
    if fill > 0.0 {
        rect(
            cx + 2.0 * WALL,
            y + 2.0 * WALL,
            fill,
            GAUGE_H - 4.0 * WALL,
            out,
        );
    }

    if let (Charge::Charging, Some(tex)) = (b.charge, bolt) {
        out.push(Draw::Tex {
            x: (x + (BOLT_MAX_SIDE - bolt_w) / 2.0).round(),
            y: (y + (GAUGE_H - bolt_h) / 2.0).round(),
            w: bolt_w,
            h: bolt_h,
            tex,
            alpha: 1.0,
        });
    }

    if percent.w > 0 {
        let px = cx + GAUGE_W + NUB_W + GAP;
        let py = (y + (GAUGE_H - HINT_H as f32) / 2.0).round();
        out.push(match percent.face {
            Some(tex) => Draw::Tex {
                x: px,
                y: py,
                w: percent.w as f32,
                h: HINT_H as f32,
                tex,
                alpha: 1.0,
            },
            None => Draw::Rect {
                x: px,
                y: py,
                w: percent.w as f32,
                h: HINT_H as f32,
                colour: [1.0, 1.0, 1.0, 0.08],
            },
        });
    }
}

fn bolt_size() -> (f32, f32) {
    static SIZE: OnceLock<(f32, f32)> = OnceLock::new();
    *SIZE.get_or_init(|| {
        let (w, h) = icon_glyph_box(Icon::Charging, BOLT_PX);
        if w == 0 || h == 0 {
            return (BOLT_MAX_SIDE, BOLT_MAX_SIDE);
        }
        let scale = (BOLT_MAX_SIDE / w as f32).min(BOLT_MAX_SIDE / h as f32);
        ((w as f32 * scale).round(), (h as f32 * scale).round())
    })
}
