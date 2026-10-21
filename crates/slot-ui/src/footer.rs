use slot_gfx::{Canvas, Draw, TexId, OUT_H};
use slot_power::Battery;

use crate::battery::{draw_gauge, GAUGE_H};
use crate::hud::{HUD_INK, PLATE_H};
use crate::plate::HINT_H;
use crate::slot_chrome::MOUTH_H;

pub const FOOTER_Y: f32 = OUT_H as f32 - MOUTH_H + (MOUTH_H - HINT_H as f32) / 2.0;
const FOOTER_MARGIN: f32 = 24.0;
const CORNER_MARGIN: f32 = 16.0;

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Printed {
    pub face: Option<TexId>,
    pub w: u32,
}

impl Printed {
    pub fn new(face: TexId, w: u32) -> Self {
        Printed {
            face: Some(face),
            w,
        }
    }
}

/// Clock on the left; Wi-Fi, then the battery, on the right, as on a phone.
#[allow(clippy::too_many_arguments)]
pub fn draw_footer(
    canvas: Canvas,
    battery: Option<Battery>,
    percent: Printed,
    bolt: Option<TexId>,
    clock: Printed,
    wifi: Option<TexId>,
    ink: [u8; 3],
    out: &mut Vec<Draw>,
) {
    let y = FOOTER_Y + (HINT_H as f32 - GAUGE_H) / 2.0;
    let x = gauge_x(canvas, FOOTER_MARGIN, battery, percent);
    draw_gauge(x, y, battery, percent, bolt, ink, out);
    draw_wifi(x, y, wifi, out);
    printed(FOOTER_MARGIN, clock, out);
}

pub fn draw_corners(
    canvas: Canvas,
    battery: Option<Battery>,
    percent: Printed,
    bolt: Option<TexId>,
    clock: Printed,
    wifi: Option<TexId>,
    out: &mut Vec<Draw>,
) {
    let y = (PLATE_H - GAUGE_H) / 2.0;
    let x = gauge_x(canvas, CORNER_MARGIN, battery, percent);
    draw_gauge(x, y, battery, percent, bolt, HUD_INK, out);
    draw_wifi(x, y, wifi, out);
    draw_printed(CORNER_MARGIN, (PLATE_H - HINT_H as f32) / 2.0, clock, out);
}

/// Where the gauge starts so its percentage ends at the right margin.
fn gauge_x(canvas: Canvas, margin: f32, battery: Option<Battery>, percent: Printed) -> f32 {
    match battery {
        Some(_) => (canvas.w - margin - crate::battery::gauge_width(battery, percent.w)).round(),
        None => canvas.w - margin + WIFI_GAP,
    }
}

/// Where the corner clock ends, for anything set beside it.
pub fn corner_clock_end(clock: Printed) -> f32 {
    CORNER_MARGIN + clock.w as f32
}

pub(crate) fn draw_printed(x: f32, y: f32, p: Printed, out: &mut Vec<Draw>) {
    if p.w == 0 {
        return;
    }
    let (w, h) = (p.w as f32, HINT_H as f32);
    out.push(match p.face {
        Some(tex) => Draw::Tex {
            x,
            y,
            w,
            h,
            tex,
            alpha: 1.0,
        },
        None => Draw::Rect {
            x,
            y,
            w,
            h,
            colour: [1.0, 1.0, 1.0, 0.08],
        },
    });
}

fn printed(x: f32, p: Printed, out: &mut Vec<Draw>) {
    draw_printed(x, FOOTER_Y, p, out);
}

/// The home Wi-Fi mark: two arcs over a dot, as tall as the battery gauge.
pub const WIFI_H: u32 = 12;
const WIFI_RADII: [f32; 2] = [5.6, 10.2];
const WIFI_STROKE: f32 = 2.0;
const WIFI_DOT: f32 = 1.5;
/// The same gap the gauge keeps before its percentage.
const WIFI_GAP: f32 = 9.0;

pub fn wifi_w() -> u32 {
    let outer = WIFI_RADII[1] + WIFI_STROKE / 2.0;
    (2.0 * outer * std::f32::consts::FRAC_1_SQRT_2).ceil() as u32 + 2
}

/// Drawn rather than taken from the symbol font, so its strokes match the gauge's walls.
pub fn wifi_face(ink: [u8; 3]) -> crate::plate::UndoFace {
    const SS: u32 = 4;
    let (w, h) = (wifi_w(), WIFI_H);
    let (cx, cy) = (w as f32 / 2.0, h as f32 - WIFI_DOT * 0.9);
    let half = WIFI_STROKE / 2.0;
    let lit = |x: f32, y: f32| -> bool {
        let (dx, dy) = (x - cx, y - cy);
        let d = (dx * dx + dy * dy).sqrt();
        if d <= WIFI_DOT {
            return true;
        }
        WIFI_RADII.iter().any(|&r| {
            if dy < 0.0 && dx.abs() <= -dy {
                return (d - r).abs() <= half;
            }
            // Round ends at 45 degrees either side of straight up.
            let e = r * std::f32::consts::FRAC_1_SQRT_2;
            [(-e, -e), (e, -e)]
                .iter()
                .any(|(ex, ey)| ((dx - ex).powi(2) + (dy - ey).powi(2)).sqrt() <= half)
        })
    };
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for py in 0..h {
        for px in 0..w {
            let mut hits = 0;
            for sy in 0..SS {
                for sx in 0..SS {
                    let x = px as f32 + (sx as f32 + 0.5) / SS as f32;
                    let y = py as f32 + (sy as f32 + 0.5) / SS as f32;
                    hits += u32::from(lit(x, y));
                }
            }
            let a = (hits * 255 / (SS * SS)) as u8;
            rgba.extend_from_slice(&[ink[0], ink[1], ink[2], a]);
        }
    }
    crate::plate::UndoFace { rgba, w, h }
}

fn draw_wifi(x: f32, gauge_y: f32, icon: Option<TexId>, out: &mut Vec<Draw>) {
    let Some(tex) = icon else {
        return;
    };
    let w = wifi_w() as f32;
    out.push(Draw::Tex {
        x: (x - WIFI_GAP - w).round(),
        y: (gauge_y + (GAUGE_H - WIFI_H as f32) / 2.0).round(),
        w,
        h: WIFI_H as f32,
        tex,
        alpha: 1.0,
    });
}
