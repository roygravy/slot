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

/// A small drop shadow one pixel under a status face, as a phone's status bar has: black under
/// light ink (dark slots), white under dark ink (light slots).
pub(crate) fn shade(out: &mut Vec<Draw>, from: usize, ink: [u8; 3]) {
    let light_ink = ink.iter().map(|&v| v as u32).sum::<u32>() > 3 * 128;
    let colour = if light_ink {
        [0.0, 0.0, 0.0, 0.55]
    } else {
        [1.0, 1.0, 1.0, 0.7]
    };
    let shadows: Vec<Draw> = out[from..]
        .iter()
        .filter_map(|d| match *d {
            // Something that is off is see-through, its shadow with it: never a dark grey
            // smudge where the shadow shows through it (Roy 2026-11-12).
            Draw::Tex { x, y, w, h, tex, alpha } => Some(Draw::Shade {
                x,
                y: y + 1.0,
                w,
                h,
                tex,
                colour: [colour[0], colour[1], colour[2], colour[3] * alpha * alpha],
            }),
            _ => None,
        })
        .collect();
    out.splice(from..from, shadows);
}

/// Clock on the left; Wi-Fi, then the battery, on the right, as on a phone.
#[allow(clippy::too_many_arguments)]
pub fn draw_footer(
    canvas: Canvas,
    battery: Option<Battery>,
    percent: Printed,
    bolt: Option<TexId>,
    clock: Printed,
    marks: impl Into<Marks>,
    ink: [u8; 3],
    out: &mut Vec<Draw>,
) {
    let marks = marks.into();
    let y = FOOTER_Y + (HINT_H as f32 - GAUGE_H) / 2.0;
    let x = gauge_x(canvas, FOOTER_MARGIN, battery, percent);
    let first = out.len();
    draw_gauge(x, y, battery, percent, bolt, ink, out);
    draw_marks(x, y, marks, out);
    printed(FOOTER_MARGIN, clock, out);
    shade(out, first, ink);
}

pub fn draw_corners(
    canvas: Canvas,
    battery: Option<Battery>,
    percent: Printed,
    bolt: Option<TexId>,
    clock: Printed,
    marks: impl Into<Marks>,
    out: &mut Vec<Draw>,
) {
    let marks = marks.into();
    let y = (PLATE_H - GAUGE_H) / 2.0;
    let x = gauge_x(canvas, CORNER_MARGIN, battery, percent);
    let first = out.len();
    draw_gauge(x, y, battery, percent, bolt, HUD_INK, out);
    draw_marks(x, y, marks, out);
    draw_printed(CORNER_MARGIN, (PLATE_H - HINT_H as f32) / 2.0, clock, out);
    shade(out, first, HUD_INK);
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

/// The home Wi-Fi mark as a phone draws it: three thick bands fanning up from a point.
pub const WIFI_H: u32 = 17;
/// Each band's share of the fan, and the clear gap between bands.
const WIFI_GAP_FRAC: f32 = 0.12;
/// Clear room between the Wi-Fi mark and the battery.
const WIFI_GAP: f32 = 11.0;

pub fn wifi_w() -> u32 {
    (WIFI_H as f32 * std::f32::consts::SQRT_2).round() as u32
}

/// Drawn smooth rather than taken from the symbol font, so it matches the battery's weight.
pub fn wifi_face(ink: [u8; 3]) -> crate::plate::UndoFace {
    const SS: u32 = 4;
    let (w, h) = (wifi_w(), WIFI_H);
    let r = h as f32 - 0.2;
    let (cx, cy) = (w as f32 / 2.0, h as f32 - 0.1);
    let band = r / 3.0;
    let gap = r * WIFI_GAP_FRAC;
    let tilt = std::f32::consts::FRAC_1_SQRT_2;
    let lit = |x: f32, y: f32| -> bool {
        let (dx, dy) = (x - cx, y - cy);
        let d = (dx * dx + dy * dy).sqrt();
        let within = dy < 0.0 && dx.abs() <= -dy;
        (0..3).any(|i| {
            let outer = r - i as f32 * band;
            let inner = if i == 2 { 0.0 } else { outer - band + gap };
            if within && d >= inner && d <= outer {
                return true;
            }
            // Round ends, as wide as the band, at 45 degrees either side of straight up.
            let (mid, half) = ((outer + inner) / 2.0, (outer - inner) / 2.0);
            i < 2
                && [-1.0f32, 1.0].iter().any(|side| {
                    let (ex, ey) = (side * mid * tilt, -mid * tilt);
                    ((dx - ex).powi(2) + (dy - ey).powi(2)).sqrt() <= half
                })
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

/// The marks left of the battery, each with how solid it is: Wi-Fi, then Bluetooth left of it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Marks {
    pub wifi: Option<(TexId, f32)>,
    pub bt: Option<(TexId, f32)>,
}

impl From<Option<(TexId, f32)>> for Marks {
    fn from(wifi: Option<(TexId, f32)>) -> Self {
        Marks { wifi, bt: None }
    }
}

/// The Wi-Fi fan beside the battery: solid when joined, see-through while Wi-Fi is on but not
/// joined (Roy, ui11), not there with Wi-Fi off. The Bluetooth rune the same way, left of it:
/// solid with headphones connected, see-through while on and waiting.
fn draw_marks(x: f32, gauge_y: f32, marks: Marks, out: &mut Vec<Draw>) {
    let mut x = x;
    for (icon, w) in [(marks.wifi, wifi_w()), (marks.bt, bt_w())] {
        let Some((tex, alpha)) = icon else {
            continue;
        };
        let w = w as f32;
        x = (x - WIFI_GAP - w).round();
        out.push(Draw::Tex {
            x,
            y: (gauge_y + (GAUGE_H - WIFI_H as f32) / 2.0).round(),
            w,
            h: WIFI_H as f32,
            tex,
            alpha,
        });
    }
}

pub fn bt_w() -> u32 {
    (WIFI_H as f32 * 0.76).round() as u32
}

/// The Bluetooth rune, drawn smooth at the Wi-Fi mark's height and stroke weight.
pub fn bt_face(ink: [u8; 3]) -> crate::plate::UndoFace {
    const SS: u32 = 4;
    let (w, h) = (bt_w(), WIFI_H);
    let (wf, hf) = (w as f32, h as f32);
    // Lighter than the Wi-Fi fan's line, so the narrow rune weighs the same (Roy 2026-11-14).
    let stroke = 2.0;
    let pad = stroke / 2.0;
    let (mid, left, right) = (wf / 2.0, pad + 0.2, wf - pad - 0.2);
    let (top, bottom) = (pad, hf - pad);
    let (q1, q3) = (hf * 0.29, hf * 0.71);
    let lines = [
        ((mid, top), (mid, bottom)),
        ((mid, top), (right, q1)),
        ((right, q1), (left, q3)),
        ((mid, bottom), (right, q3)),
        ((right, q3), (left, q1)),
    ];
    let near = |x: f32, y: f32| -> bool {
        lines.iter().any(|&((ax, ay), (bx, by))| {
            let (dx, dy) = (bx - ax, by - ay);
            let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            let (px, py) = (ax + t * dx - x, ay + t * dy - y);
            (px * px + py * py).sqrt() <= stroke / 2.0
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
                    hits += u32::from(near(x, y));
                }
            }
            let a = (hits * 255 / (SS * SS)) as u8;
            rgba.extend_from_slice(&[ink[0], ink[1], ink[2], a]);
        }
    }
    crate::plate::UndoFace { rgba, w, h }
}
