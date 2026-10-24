use slot_gfx::{Canvas, Draw, TexId, OUT_H};
use slot_store::Cart;

use crate::bitmaps::{Mould, NES_LID_W};
use crate::cart::{cart_box, label_colour, label_text, seated_box, SEATED_W};
use crate::footer::Printed;
use crate::icon::icon_box;
use crate::plate::HINT_H;
use crate::shelf::foot_y;

pub const ALERT_PX: f32 = 44.0;

pub const MOUTH_H: f32 = 58.0;
const BAND_Y: f32 = OUT_H as f32 - MOUTH_H;

pub const MOUTH_W: f32 = SEATED_W as f32 + 14.0;
const SLIT_H: f32 = 9.0;
const SLIT_Y: f32 = BAY_Y + 5.0;

pub const LIP_H: f32 = 2.0;

const BAY_W: f32 = MOUTH_W + 18.0;
const BAY_Y: f32 = BAND_Y + LIP_H;
const SCOOP_W: f32 = MOUTH_W * 0.88;
const SCOOP_D: f32 = RECESS_H - (SCOOP_Y - BAY_Y);
const SCOOP_Y: f32 = SLIT_Y + SLIT_H;

const RECESS_H: f32 = 42.0;
const RIM_W: f32 = 2.0;
const SCOOP_FLAT: f32 = 4.0;

fn mouth_x(canvas: Canvas) -> f32 {
    canvas.centred(MOUTH_W)
}

fn bay_x(canvas: Canvas) -> f32 {
    canvas.centred(BAY_W)
}

const HOUSING: [u8; 3] = [0x24, 0x24, 0x29];
const RECESS: [u8; 3] = [0x1a, 0x1a, 0x1d];
const OPENING: [u8; 3] = [0x05, 0x05, 0x08];
const EDGE: [u8; 3] = [0x4d, 0x4d, 0x57];

fn rgb(c: [u8; 3]) -> [f32; 4] {
    [
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
        1.0,
    ]
}

pub fn housing() -> [f32; 4] {
    rgb(HOUSING)
}

pub fn opening() -> [f32; 4] {
    rgb(OPENING)
}

pub fn edge() -> [f32; 4] {
    rgb(EDGE)
}

pub fn recess() -> [f32; 4] {
    rgb(RECESS)
}

const LIP_Y: f32 = BAND_Y;

const SEATED_Y: f32 = BAY_Y + 4.0;

fn catch_at(lip: f32, foot: f32, seated_foot: f32) -> f32 {
    (lip - foot) / (seated_foot - foot)
}

const CATCH_IN: f32 = 0.42;
const CATCH_OUT: f32 = 0.62;
const CREEP: f32 = 0.03;

pub struct SlotChrome<'a> {
    pub cart: &'a Cart,
    pub face: Option<TexId>,
    pub rest: f32,
    pub scale: f32,
    pub seat: f32,
    pub alert: Option<(TexId, f32)>,
    pub dim: f32,
    pub screen: f32,
    pub game: bool,
    /// When set, the picture opens behind a fading slot. A resumed boot, the picture opening, and the eject back to the shelf keep the cart between the slot back and front.
    pub over: bool,
    /// The back plastic and the board a see-through cart shows under its front shell.
    pub behind: [Option<TexId>; 2],
    pub slot: Option<(TexId, TexId)>,
    pub canvas: Canvas,
    /// How much higher the cart stands once seated.
    pub lift: f32,
    /// The NES lid, shut, in the theme's housing.
    pub lid: Option<TexId>,
    /// The light housing, for the NES lid's tab.
    pub light: bool,
}

/// The Game Boy slot seats a pak this much higher, so its top lines up with the housing's top
/// edge.
pub const GB_SEAT_LIFT: f32 = SEATED_Y - BAND_Y;

impl SlotChrome<'_> {
    pub fn draw(&self, out: &mut Vec<Draw>) {
        let canvas = self.canvas;
        let seat = self.seat.clamp(0.0, 1.0);
        let dim = self.dim.clamp(0.0, 1.0);
        if dim > 0.0 {
            out.push(Draw::Rect {
                x: 0.0,
                y: 0.0,
                w: canvas.w,
                h: OUT_H as f32,
                colour: [0.0, 0.0, 0.0, dim],
            });
        }

        let chrome = 1.0 - self.screen.clamp(0.0, 1.0);
        let opening = self.screen > 0.0;
        let held = opening && !self.over;
        let slot_alpha = if held { 1.0 } else { chrome };

        if self.game && opening && self.over {
            out.push(Draw::Game);
        }

        draw_slot_back(canvas, self.slot, slot_alpha, out);

        let mould = crate::cart::mould_of(self.cart);
        let (cw, ch) = cart_box(mould);
        let (cw, ch) = (cw as f32, ch as f32);

        let scale = self.scale.clamp(0.0, 1.0);
        let (w0, h0) = (cw * scale, ch * scale);
        let (sw, sh) = seated_box(mould);
        let (sw, sh) = (sw as f32, sh as f32);
        let foot0 = foot_y(ch);
        let (x, y, cw, ch, lid) = match tv_pose(mould, seat, canvas.w / 2.0, (sw, sh)) {
            Some(pose) => {
                // Off the shelf it shrinks to its seated size on the way to the slot.
                let k = pose.leave;
                let (x0, y0) = (self.rest, foot0 - h0);
                let lerp = |a: f32, b: f32| a + (b - a) * k;
                let (x, y) = (lerp(x0, pose.x), lerp(y0, pose.y));
                (x, y, lerp(w0, sw), lerp(h0, sh), pose.lid)
            }
            None => {
                // Into the Super Game Boy, the pak goes in at its top edge and sinks until its
                // own top is flush with it, as in the Game Boy slot (Roy).
                let (lip, seated_foot) = match rise() > 0.0 {
                    true => {
                        let top = BAND_Y - rise() + SGB_TOP;
                        (top, top + sh)
                    }
                    false => (LIP_Y, SEATED_Y + sh - self.lift),
                };
                let catch = catch_at(lip, foot0, seated_foot);
                let travel = travel(seat, catch);
                let k = (travel / catch).min(1.0);
                let (cw, ch) = (w0 + (sw - w0) * k, h0 + (sh - h0) * k);
                let centre = self.rest + w0 / 2.0;
                let x = centre + (canvas.w / 2.0 - centre) * k - cw / 2.0;
                let y = foot0 + (seated_foot - foot0) * travel - ch;
                (x, y, cw, ch, None)
            }
        };
        let cart_alpha = if seat >= 1.0 && !held { chrome } else { 1.0 };
        for tex in self.behind.into_iter().flatten() {
            out.push(Draw::Tex {
                x,
                y,
                w: cw,
                h: ch,
                tex,
                alpha: cart_alpha,
            });
        }
        out.push(match self.face {
            Some(tex) => Draw::Tex {
                x,
                y,
                w: cw,
                h: ch,
                tex,
                alpha: cart_alpha,
            },
            None => {
                let c = label_colour(&label_text(self.cart));
                Draw::Rect {
                    x,
                    y,
                    w: cw,
                    h: ch,
                    colour: [
                        c[0] as f32 / 255.0,
                        c[1] as f32 / 255.0,
                        c[2] as f32 / 255.0,
                        cart_alpha,
                    ],
                }
            }
        });

        if let Some((tex, alpha)) = self.alert {
            let (w, h) = icon_box(ALERT_PX);
            let (w, h) = (w as f32, h as f32);
            out.push(Draw::Tex {
                x: x + (cw - w) / 2.0,
                y: y + (ch - h) / 2.0,
                w,
                h,
                tex,
                alpha,
            });
        }

        let front_alpha = if self.over { chrome } else { slot_alpha };
        if let (Some(theta), Some(tex)) = (lid, self.lid) {
            draw_lid(canvas, tex, theta, self.light, front_alpha, out);
        }

        if self.over && chrome < 1.0 && self.slot.is_none() {
            front_bands(canvas, |x, y, w, h, _| {
                out.push(Draw::Rect {
                    x,
                    y,
                    w,
                    h,
                    colour: [0.0, 0.0, 0.0, 1.0],
                });
            });
        }
        draw_slot_front(canvas, self.slot, front_alpha, out);

        if self.game && opening && !self.over {
            out.push(Draw::Game);
        }
    }
}

fn band(x: f32, y: f32, w: f32, h: f32, c: [f32; 4], alpha: f32) -> Draw {
    Draw::Rect {
        x,
        y,
        w,
        h,
        colour: [c[0], c[1], c[2], c[3] * alpha],
    }
}

/// How far the Super Game Boy's top stands above the band while the Game Boy carts go into
/// it: half as tall again as the band shows, its top edge in it, and the Game Boy shelf moves
/// up out of its way (Roy).
pub const SGB_RISE: f32 = crate::bitmaps::SGB_BAND_H as f32 - MOUTH_H;
/// Where the Super Game Boy's top edge is in its band, as `tools/sgb/sgb_band.py` draws it.
const SGB_TOP: f32 = 6.0;

/// The Super Game Boy in its band, as `tools/sgb/sgb_band.py` draws it in the 720-wide strip:
/// its left edge and size.
fn sgb_in_band(model: crate::SgbModel) -> (f32, f32, f32) {
    match model {
        crate::SgbModel::Ntsc => (90.0, 540.0, 472.0),
        _ => (122.0, 473.0, 449.0),
    }
}

/// Where the Super Game Boy's whole face is while it is the band: (x, y, w, h).
pub fn sgb_band_rect(model: crate::SgbModel, canvas: Canvas) -> (f32, f32, f32, f32) {
    let (x, w, h) = sgb_in_band(model);
    let strip = crate::bitmaps::SLOT_W as f32;
    let cut = ((strip - canvas.w) / 2.0).max(0.0);
    let top = OUT_H as f32 - crate::bitmaps::SGB_BAND_H as f32;
    (x - cut, top + SGB_TOP, w, h)
}

/// Where the Super Game Boy stands once it is in the SNES: (x, y, w, h).
pub fn sgb_seated_rect(mould: Mould, canvas: Canvas) -> (f32, f32, f32, f32) {
    let (sw, sh) = seated_box(mould);
    let (sw, sh) = (sw as f32, sh as f32);
    match tv_pose(mould, 1.0, canvas.w / 2.0, (sw, sh)) {
        Some(pose) => (pose.x, pose.y, sw, sh),
        None => (canvas.w / 2.0 - sw / 2.0, BAND_Y - sh, sw, sh),
    }
}

static SGB_TALL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The band is the Super Game Boy's top, standing `SGB_RISE` above the console.
pub fn set_sgb_band(on: bool) {
    SGB_TALL.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn rise() -> f32 {
    match SGB_TALL.load(std::sync::atomic::Ordering::Relaxed) {
        true => SGB_RISE,
        false => 0.0,
    }
}

fn strip(canvas: Canvas, tex: TexId, alpha: f32) -> Draw {
    Draw::Part {
        x: 0.0,
        y: BAND_Y - rise(),
        w: canvas.w,
        h: MOUTH_H + rise(),
        tex,
        alpha,
        uv: canvas.window(crate::bitmaps::SLOT_W as f32),
    }
}

/// The console's shadow on the ground just above its top edge, for depth.
const CONSOLE_SHADOW: f32 = 16.0;
const CONSOLE_SHADOW_ALPHA: f32 = 0.18;

static LIGHT_GROUND: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// On a light wallpaper the console's shadow is lighter.
pub fn set_light_ground(light: bool) {
    LIGHT_GROUND.store(light, std::sync::atomic::Ordering::Relaxed);
}

fn draw_console_shadow(canvas: Canvas, alpha: f32, out: &mut Vec<Draw>) {
    let alpha = match LIGHT_GROUND.load(std::sync::atomic::Ordering::Relaxed) {
        true => alpha * 0.55,
        false => alpha,
    };
    const STEPS: usize = 8;
    let step = CONSOLE_SHADOW / STEPS as f32;
    for k in 0..STEPS {
        // Darkest against the housing, easing out to nothing.
        let t = (k as f32 + 1.0) / STEPS as f32;
        let a = CONSOLE_SHADOW_ALPHA * t * t / STEPS as f32 * 2.2;
        out.push(Draw::Rect {
            x: 0.0,
            y: BAND_Y - rise() - CONSOLE_SHADOW + k as f32 * step,
            w: canvas.w,
            h: CONSOLE_SHADOW - k as f32 * step,
            colour: [0.0, 0.0, 0.0, a * alpha],
        });
    }
}

fn draw_slot_back(canvas: Canvas, slot: Option<(TexId, TexId)>, alpha: f32, out: &mut Vec<Draw>) {
    if let Some((back, _)) = slot {
        // The Super Game Boy stands alone, no console around it to cast the shadow.
        if rise() == 0.0 {
            draw_console_shadow(canvas, alpha, out);
        }
        out.push(strip(canvas, back, alpha));
        return;
    }
    let (bay_x, mouth_x) = (bay_x(canvas), mouth_x(canvas));
    out.push(band(bay_x, BAND_Y, BAY_W, LIP_H, housing(), alpha));
    out.push(band(mouth_x, BAND_Y, MOUTH_W, LIP_H, edge(), alpha));
    out.push(band(bay_x, BAY_Y, BAY_W, RECESS_H, recess(), alpha));
    out.push(band(mouth_x, SLIT_Y, MOUTH_W, SLIT_H, opening(), alpha));
    for_each_scoop_span(canvas, |x, w, depth| {
        out.push(band(x, SCOOP_Y, w, depth, opening(), alpha));
    });
}

fn draw_slot_front(canvas: Canvas, slot: Option<(TexId, TexId)>, alpha: f32, out: &mut Vec<Draw>) {
    if let Some((_, front)) = slot {
        out.push(strip(canvas, front, alpha));
        return;
    }
    front_bands(canvas, |x, y, w, h, c| out.push(band(x, y, w, h, c, alpha)));
}

fn front_bands(canvas: Canvas, mut band: impl FnMut(f32, f32, f32, f32, [f32; 4])) {
    let w = canvas.w;
    let (bay_x, cx) = (bay_x(canvas), canvas.w / 2.0);
    let right = bay_x + BAY_W;
    let floor = SCOOP_Y + SCOOP_D + RIM_W;
    band(0.0, BAND_Y, bay_x, MOUTH_H, housing());
    band(right, BAND_Y, w - right, MOUTH_H, housing());

    let near = cx - SCOOP_W / 2.0;
    band(bay_x, SCOOP_Y, near - bay_x, floor - SCOOP_Y, housing());
    let far = cx + SCOOP_W / 2.0;
    band(far, SCOOP_Y, right - far, floor - SCOOP_Y, housing());

    for_each_scoop_span(canvas, |x, w, depth| {
        let top = SCOOP_Y + depth + RIM_W;
        band(x, top, w, floor - top, housing());
    });
    band(0.0, floor, w, OUT_H as f32 - floor, housing());
    for_each_scoop_span(canvas, |x, w, depth| {
        band(x, SCOOP_Y + depth, w, RIM_W, edge());
    });
}

fn for_each_scoop_span(canvas: Canvas, mut span: impl FnMut(f32, f32, f32)) {
    let cx = canvas.w / 2.0;
    let hw = SCOOP_W / 2.0;
    let depth = |x: f32| SCOOP_D * (1.0 - (x.abs() / hw).powf(SCOOP_FLAT)).max(0.0);
    let mut x = -hw;
    while x < hw {
        let d = depth(x + 0.5).round();
        let start = x;
        while x < hw && depth(x + 0.5).round() == d {
            x += 1.0;
        }
        span(cx + start, x - start, d);
    }
}

const SLOT_NAME_GAP: f32 = 6.0;

pub fn slot_name_y() -> f32 {
    OUT_H as f32 - MOUTH_H - rise() - HINT_H as f32 - SLOT_NAME_GAP
}

pub fn draw_slot_name(
    canvas: Canvas,
    name: Printed,
    alpha: f32,
    ink: [f32; 4],
    out: &mut Vec<Draw>,
) {
    let (Some(tex), true) = (name.face, alpha > 0.0) else {
        return;
    };
    let (w, h) = (name.w as f32, HINT_H as f32);
    out.push(Draw::Tint {
        x: canvas.centred(w),
        y: slot_name_y(),
        w,
        h,
        tex,
        colour: [ink[0], ink[1], ink[2], ink[3] * alpha],
    });
}

pub fn draw_slot_title(
    canvas: Canvas,
    title: Printed,
    alpha: f32,
    ink: [f32; 4],
    out: &mut Vec<Draw>,
) {
    let (Some(tex), true) = (title.face, alpha > 0.0) else {
        return;
    };
    let (w, h) = (title.w as f32, crate::plate::TITLE_LINE_H as f32);
    out.push(Draw::Tint {
        x: canvas.centred(w),
        y: slot_name_y() + HINT_H as f32 - h,
        w,
        h,
        tex,
        colour: [ink[0], ink[1], ink[2], ink[3] * alpha],
    });
}

/// The slot's back (behind a cart) or its front (over one), on their own.
pub fn draw_slot_part(canvas: Canvas, slot: Option<(TexId, TexId)>, front: bool, out: &mut Vec<Draw>) {
    match front {
        false => draw_slot_back(canvas, slot, 1.0, out),
        true => draw_slot_front(canvas, slot, 1.0, out),
    }
}

pub fn draw_empty_slot(canvas: Canvas, slot: Option<(TexId, TexId)>, out: &mut Vec<Draw>) {
    draw_slot_back(canvas, slot, 1.0, out);
    draw_slot_front(canvas, slot, 1.0, out);
}

fn travel(seat: f32, catch: f32) -> f32 {
    if seat < CATCH_IN {
        catch * ease(seat / CATCH_IN)
    } else if seat < CATCH_OUT {
        catch + CREEP * (seat - CATCH_IN) / (CATCH_OUT - CATCH_IN)
    } else {
        let caught = catch + CREEP;
        caught + (1.0 - caught) * ease((seat - CATCH_OUT) / (1.0 - CATCH_OUT))
    }
}

pub fn ease(u: f32) -> f32 {
    u * u * u * (u * (u * 6.0 - 15.0) + 10.0)
}

/// Seconds an NES or SNES cart takes from the shelf until it is seated, at its own, slower pace:
/// the NES lid swings open, the cart slides in against the spring, takes a last push and the lid
/// drops shut; the SNES cart slides down into its slot and takes the same push. None for the
/// Game Boys and the GBA, which keep the slot's own quick insert.
pub fn insert_seconds(mould: Mould) -> Option<f32> {
    match mould {
        Mould::Nes => Some(NES_T),
        Mould::SnesNtsc | Mould::SnesPal | Mould::SgbNtsc | Mould::SgbPal => Some(SNES_T),
        _ => None,
    }
}

/// How far into the insert, 0 to 1, the board snaps into the connector: the click.
pub fn click_at(mould: Mould) -> f32 {
    match mould {
        Mould::Nes => NES_CLICK / NES_T,
        Mould::SnesNtsc | Mould::SnesPal | Mould::SgbNtsc | Mould::SgbPal => SNES_CLICK / SNES_T,
        _ => 1.0,
    }
}

const NES_T: f32 = 1.60;
const NES_CLICK: f32 = 1.15;
const SNES_T: f32 = 1.15;
/// The European SNES game cart's seated height, for the Super Game Boys that go in as deep.
const SNES_PAL_SEATED_H: f32 = 152.0;
const SNES_CLICK: f32 = 1.08;

/// Where an NES or SNES cart is, `seat` into its insert: its seated-size box once it has left
/// the shelf (`leave` runs 0 to 1 on the way), and the NES lid's opening in degrees.
struct TvPose {
    x: f32,
    y: f32,
    leave: f32,
    lid: Option<f32>,
}

fn tv_pose(mould: Mould, seat: f32, cx: f32, (sw, sh): (f32, f32)) -> Option<TvPose> {
    let x = cx - sw / 2.0;
    let lerp = |a: f32, b: f32, u: f32| a + (b - a) * u;
    match mould {
        Mould::Nes => {
            let t = seat * NES_T;
            let wait = BAND_Y - 6.0 - sh;
            let slid = BAND_Y - 14.0;
            let home = BAND_Y + 6.0;
            // The lid swings open as the cart leaves the shelf, and the cart is already sliding
            // in while the lid finishes its swing.
            let open = 90.0 * smooth((t - 0.10) / 0.40);
            let (leave, y, lid) = match t {
                t if t < 0.40 => (smooth(t / 0.40), wait, open),
                t if t < 0.95 => (1.0, lerp(wait, slid, resist((t - 0.40) / 0.55)), open),
                t if t < 1.20 => (1.0, lerp(slid, home, seat_push((t - 0.95) / 0.25)), 90.0),
                t if t < 1.28 => (1.0, home, 90.0),
                t if t < 1.48 => {
                    let u = (t - 1.28) / 0.20;
                    (1.0, home, 90.0 * (1.0 - u * u))
                }
                t => {
                    // The lid lands and bounces once.
                    let u = ((t - 1.48) / 0.12).min(1.0);
                    (1.0, home, 7.0 * (u * std::f32::consts::PI).sin())
                }
            };
            Some(TvPose { x, y, leave, lid: Some(lid) })
        }
        Mould::SnesNtsc | Mould::SnesPal | Mould::SgbNtsc | Mould::SgbPal => {
            let t = seat * SNES_T;
            let pal = matches!(mould, Mould::SnesPal | Mould::SgbPal);
            let wait = BAND_Y - sh - 8.0;
            // NTSC: the slot's edge meets the third line on the cart's side grips. A PAL cart goes
            // in as deep and so stands a little prouder. A Super Game Boy goes in as deep as the
            // game cart it is shaped like, and its taller body stands out above.
            let out = if pal { (86.0 - 42.6) / 86.0 } else { 132.0 / 257.0 };
            let depth = match mould {
                // The American SGB stands prouder: the slot's edge meets its lowest groove but
                // one (Roy: it went in too deep).
                Mould::SgbNtsc => sh * (1.0 - 0.797),
                Mould::SgbPal => SNES_PAL_SEATED_H * (1.0 - out),
                _ => sh * (1.0 - out),
            };
            let home = BAND_Y - (sh - depth) - if pal { 5.0 } else { 0.0 };
            let slid = home - 14.0;
            let (leave, y) = match t {
                t if t < 0.40 => (smooth(t / 0.40), wait),
                t if t < 0.45 => (1.0, wait),
                t if t < 0.85 => (1.0, lerp(wait, slid, resist((t - 0.45) / 0.40))),
                t if t < 0.88 => (1.0, slid),
                t => (1.0, lerp(slid, home, seat_push((t - 0.88) / 0.25))),
            };
            Some(TvPose { x, y, leave, lid: None })
        }
        _ => None,
    }
}

fn smooth(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// Sliding in against the spring: slowing as it goes, with a few small catches.
fn resist(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    let base = 1.0 - (1.0 - u).powf(2.2);
    let catch = 0.012 * (u * std::f32::consts::PI * 6.0).sin() * (1.0 - u);
    (base - catch.abs()).clamp(0.0, 1.0)
}

/// The last push, as a hand does it: the thumb presses and the cart picks up speed, the board
/// bites the connector and stalls a moment, then it gives way, snaps home and settles.
fn seat_push(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    if u < 0.38 {
        let v = u / 0.38;
        0.45 * v * v
    } else if u < 0.50 {
        let v = (u - 0.38) / 0.12;
        0.45 + 0.04 * (1.0 - (1.0 - v).powi(2))
    } else if u < 0.80 {
        let v = (u - 0.50) / 0.30;
        0.49 + 0.54 * (1.0 - (1.0 - v).powi(3))
    } else {
        let v = (u - 0.80) / 0.20;
        1.03 - 0.03 * (1.0 - (1.0 - v).powi(2))
    }
}

/// The NES lid, seen from straight above: hinged on the screen's bottom edge, it swings up toward
/// the camera. Its top panel foreshortens down to the hinge while its front, as long as the
/// panel, turns its face to the camera upside down, with the small tab on its free edge.
fn draw_lid(canvas: Canvas, tex: TexId, theta: f32, light: bool, alpha: f32, out: &mut Vec<Draw>) {
    if alpha <= 0.0 {
        return;
    }
    let (w, l) = (NES_LID_W as f32, MOUTH_H);
    let x = canvas.centred(w);
    if theta <= 0.01 {
        out.push(Draw::Tex { x, y: BAND_Y, w, h: l, tex, alpha });
        return;
    }
    let hinge = OUT_H as f32;
    let (sin, cos) = theta.to_radians().sin_cos();
    // The well under the lid.
    out.push(band(x, BAND_Y, w, l, opening(), alpha));
    // Turning toward the camera lifts the plastic a touch, nothing more: one colour.
    let sheen = |y: f32, h: f32, k: f32, out: &mut Vec<Draw>| {
        out.push(band(x, y, w, h, [1.0, 1.0, 1.0, 0.06 * k], alpha));
    };
    let fold = hinge - l * cos;
    if hinge - fold > 0.5 {
        out.push(Draw::Tex { x, y: fold, w, h: hinge - fold, tex, alpha });
        sheen(fold, hinge - fold, sin, out);
    }
    let fh = l * sin;
    if fh <= 0.5 {
        return;
    }
    let top = fold - fh;
    // Its shadow on the ground above, a soft band that deepens as it stands up.
    let shadow = if light { 0.27 } else { 0.43 };
    for k in 0..4 {
        let spread = (7.0 * sin + 2.0) * (1.0 - k as f32 / 4.0);
        out.push(band(x, top - spread, w, spread + 1.0, [0.0, 0.0, 0.0, shadow / 4.0], alpha));
    }
    out.push(Draw::Part {
        x,
        y: top,
        w,
        h: fh,
        tex,
        alpha,
        uv: [0.0, 1.0, 1.0, -1.0],
    });
    sheen(top, fh, 1.0, out);
    // The tab: lighter, rounded, standing proud of the free edge.
    let base: [f32; 3] = if light { [182.0, 179.0, 173.0] } else { [41.0, 41.0, 44.0] };
    let shade = |k: f32, add: f32| {
        let c = base.map(|v| ((v * k + add) / 255.0).min(1.0));
        [c[0], c[1], c[2], 1.0]
    };
    let th = (4.0 * sin).max(1.0);
    let tw = (w * 0.17).round();
    let tx = canvas.w / 2.0 - tw / 2.0;
    out.push(band(tx, top - th + 1.0, tw, th + 1.0, shade(0.7, 0.0), alpha));
    out.push(band(tx + 1.0, top - th + 1.0, tw - 2.0, th, shade(1.18, 14.0), alpha));
    out.push(band(tx + 3.0, top - th + 1.0, tw - 6.0, 1.0, shade(1.18, 39.0), alpha));
}
