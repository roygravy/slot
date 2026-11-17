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
    /// On its way out: an NES cart then comes straight out, without the insert's push and
    /// the lid's bounce played backwards.
    pub ejecting: bool,
    /// The original Game Boy's power switch, on the Game Boy slot only.
    pub power: Option<PowerSwitch>,
    /// The slot's mouth is shallow (Pocket, Light, Game Boy Color): the pak lies on it rather
    /// than in it, so it casts a soft drop shadow on the slot around it (Roy): the pak's
    /// `perch_shadow_face`.
    pub perched: Option<TexId>,
    /// The cart's drop shadow on the shelf (`drop_shadow_face`): an NES or SNES cart keeps it
    /// while it hangs over the console, until it sinks into the mouth (Roy 2026-11-12, sh7).
    pub drop: Option<TexId>,
}

/// The original Game Boy's power switch: one piece of plastic, a knob on the console's top and
/// a lock tab that slides out of the slot's right wall over the cart's notch, so the cart can't
/// come out while it is on (Roy). `on` runs 0 (off) to 1 (on, locked).
#[derive(Clone, Copy)]
pub struct PowerSwitch {
    pub knob: TexId,
    pub tab: TexId,
    pub on: f32,
    pub light: bool,
    /// The clear slot: the tab's hole is shallow, with the mechanism's dark grey plastic just
    /// behind it (Roy, after a clear Play It Loud).
    pub clear: bool,
    /// The DMG's lock: its tab and the guide hole behind it. The Pocket and Light have the
    /// same slide switch without them (Roy).
    pub lock: bool,
}

/// How much deeper than the band's top a Game Boy or Game Boy Color pak sits in its slot (Roy;
/// 4 px since 2026-11-05: a touch closer to the top, the power switch's tab moves up with it).
pub const GB_SINK: f32 = 4.0;
/// The same in a shallow slot (Pocket, Light, Game Boy Color): just under the top (Roy).
pub const GB_SINK_SHALLOW: f32 = 2.0;
/// The Game Boy slot's right wall, from the middle: where the seated pak's notch is.
const POWER_WALL_DX: f32 = 111.0;
/// The tab, and the knob with it, travel this far; both are as long.
const POWER_TRAVEL: f32 = crate::bitmaps::POWER_TAB_W as f32;
/// The tab fills the lower half of the notch: this far under the seated pak's top.
const POWER_TAB_DY: f32 = 6.0;
/// How much of the lock tab shows through the clear slot's wall.
const CLEAR_WALL_SEEN: f32 = 0.8;
/// How much further right the clear slot's tab reaches than the knob.
const CLEAR_TAB_EXTRA: f32 = 10.0;
const POWER_TAB_ROWS: f32 = 6.0;
/// The knob stands a little right of the tab's left edge (Roy).
const POWER_KNOB_DX: f32 = 2.0;

/// How far on the switch is, snapped to whole pixels of its travel: knob, stem, slider and tab
/// are one solid part and must step together, never a sub-pixel apart (Roy).
fn slid(power: PowerSwitch) -> f32 {
    (POWER_TRAVEL * power.on.clamp(0.0, 1.0)).round() / POWER_TRAVEL
}

fn power_wall(canvas: Canvas, power: PowerSwitch) -> f32 {
    // The Pocket's and Light's switch sits further in, over the right end of the mouth, as on
    // Roy's photo of a clear Pocket (it has no lock reaching into the cart's notch)
    canvas.w / 2.0 + POWER_WALL_DX - if power.lock { 0.0 } else { POCKET_SWITCH_IN }
}
/// How much further left the Pocket's and Light's switch sits than the DMG's.
const POCKET_SWITCH_IN: f32 = 20.0;

/// The guide hole in the slot's back wall that the tab runs in: an opening ~0.8 mm deep in the
/// plastic, a row of shade under its top edge, and through it the switch's dark grey slider
/// moving with the knob, on every DMG case, solid or clear (Roy).
fn draw_power_hole(canvas: Canvas, power: PowerSwitch, alpha: f32, out: &mut Vec<Draw>) {
    if !power.lock {
        return;
    }
    let wall = power_wall(canvas, power);
    let (x, y) = (wall - POWER_TRAVEL + 2.0, BAND_Y + GB_SINK + POWER_TAB_DY);
    let lip: [f32; 3] = match power.light || power.clear {
        true => [150.0, 148.0, 144.0],
        false => [70.0, 69.0, 74.0],
    };
    let rgb = |c: [f32; 3], a: f32| [c[0] / 255.0, c[1] / 255.0, c[2] / 255.0, a];
    let (sx0, sx1) = slider_span(power, canvas);
    for row in 0..POWER_TAB_ROWS as usize {
        let ry = y + row as f32;
        // deep behind the slider: dark
        out.push(Draw::Rect { x, y: ry, w: POWER_TRAVEL, h: 1.0, colour: rgb([14.0, 14.0, 16.0], alpha) });
        if let Some((inset, v)) = slider_row(ry) {
            let (a0, a1) = ((sx0 + inset).max(x), (sx1 - inset).min(x + POWER_TRAVEL));
            if a1 > a0 {
                // in the hole it catches less light than out on the open board
                let v = v * 0.82;
                out.push(Draw::Rect { x: a0, y: ry, w: a1 - a0, h: 1.0, colour: rgb([v, v - 2.0, v - 5.0], alpha) });
            }
        }
        // the hole's top edge shades its first rows: a shallow hole, a short shadow
        if row < 2 {
            let k = if row == 0 { 0.55 } else { 0.22 };
            out.push(Draw::Rect { x, y: ry, w: POWER_TRAVEL, h: 1.0, colour: [0.0, 0.0, 0.0, k * alpha] });
        }
    }
    // the lower edge, quiet (Roy 2026-11-13: it stood out too sharp)
    out.push(Draw::Rect {
        x,
        y: y + POWER_TAB_ROWS,
        w: POWER_TRAVEL - 1.0,
        h: 1.0,
        colour: rgb(lip, alpha * LIP_ALPHA),
    });
}

const LIP_ALPHA: f32 = 0.4;

/// The slider bar's left and right ends where it stands now (moving with the knob).
fn slider_span(power: PowerSwitch, canvas: Canvas) -> (f32, f32) {
    let cx = power_wall(canvas, power) + 11.5 - POWER_TRAVEL * slid(power);
    (cx - SLIDER_W / 2.0, cx + SLIDER_W / 2.0)
}

const SLIDER_W: f32 = 54.0;
const SLIDER_TOP: f32 = 9.0;
const SLIDER_BOTTOM: f32 = 18.0;

/// One row of the slider bar at canvas row `y`: how far its ends are cut in (the rail notches)
/// and its grey, lit on top and falling to a shaded foot.
fn slider_row(y: f32) -> Option<(f32, f32)> {
    let (top, bottom) = (BAND_Y + SLIDER_TOP, BAND_Y + SLIDER_BOTTOM);
    if y < top || y >= bottom {
        return None;
    }
    let rows = (bottom - top) as usize;
    let row = (y - top) as usize;
    let u = row as f32 / (rows - 1) as f32;
    let v = SLIDER_DARK
        * match row {
            0 => 196.0,
            _ if row == rows - 1 => 104.0,
            _ => 170.0 - 38.0 * u,
        };
    let inset = if (0.34..0.6).contains(&u) { 3.0 } else { 0.0 };
    Some((inset, v))
}

/// The tab, out of the wall as far as the switch is on.
fn draw_power_tab(canvas: Canvas, power: PowerSwitch, alpha: f32, out: &mut Vec<Draw>) {
    draw_power_tab_in(canvas, power, alpha, out);
}

/// The part of the tab still inside the wall, only seen on the clear slot.
fn draw_power_tab_in(canvas: Canvas, power: PowerSwitch, alpha: f32, out: &mut Vec<Draw>) {
    if !power.lock {
        return;
    }
    let on = slid(power);
    if power.clear && on < 1.0 {
        // On the clear slot the rest of the tab shows through the wall it hides in (Roy).
        let w = POWER_TRAVEL * (1.0 - on);
        out.push(Draw::Part {
            x: power_wall(canvas, power) + 1.0,
            y: BAND_Y + GB_SINK + POWER_TAB_DY,
            w,
            h: crate::bitmaps::POWER_TAB_H as f32,
            tex: power.tab,
            alpha: alpha * CLEAR_WALL_SEEN,
            uv: [on, 0.0, 1.0 - on, 1.0],
        });
    }
    if power.clear {
        // The real tab is wider than the knob and reaches further right; that extra length
        // always stays inside the wall, so it only shows on the clear slot (Roy).
        out.push(Draw::Part {
            x: power_wall(canvas, power) + 1.0 + POWER_TRAVEL * (1.0 - on),
            y: BAND_Y + GB_SINK + POWER_TAB_DY,
            w: CLEAR_TAB_EXTRA,
            h: crate::bitmaps::POWER_TAB_H as f32,
            tex: power.tab,
            alpha: alpha * CLEAR_WALL_SEEN,
            uv: [0.7, 0.0, 0.3, 1.0],
        });
    }
    draw_power_tab_out(canvas, power, alpha, out);
}

/// The part of the tab that has come out of the wall, over the pak's notch.
fn draw_power_tab_out(canvas: Canvas, power: PowerSwitch, alpha: f32, out: &mut Vec<Draw>) {
    let on = slid(power);
    if on <= 0.0 || !power.lock {
        return;
    }
    let w = POWER_TRAVEL * on;
    out.push(Draw::Part {
        x: power_wall(canvas, power) + 1.0 - w,
        y: BAND_Y + GB_SINK + POWER_TAB_DY,
        w,
        h: crate::bitmaps::POWER_TAB_H as f32,
        tex: power.tab,
        alpha,
        uv: [0.0, 0.0, on, 1.0],
    });
}

/// The clear slot shows the switch's plastic slider (Roy's photo of the part): a wide bar with
/// notched rail ends, standing on the board's switch, behind the lock tab and under the knob,
/// sliding with them. Seen through the clear shell.
/// How much of what lies under it the clear shell's top rim hides, per row from the band's top
/// (assets/slot_back_gb_orig.png alpha).
const RIM_ALPHA: [f32; 10] = [0.85, 0.82, 0.71, 0.74, 0.67, 0.64, 0.38, 0.22, 0.18, 0.18];

/// The slider and its stem are dark grey plastic (Roy).
const SLIDER_DARK: f32 = 0.42;
/// The Pocket's and Light's: the same dark grey, its bar's rows, and where its stem meets the board's
/// switch (tools/slot_cases/clear_pocket_preview.py: SW1's top).
const POCKET_SLIDER_DARK: f32 = SLIDER_DARK;
const POCKET_SLIDER_TOP: f32 = 7.0;
const POCKET_SLIDER_BOTTOM: f32 = 17.0;
const POCKET_STEM_BOTTOM: f32 = POCKET_SLIDER_BOTTOM;

fn draw_power_slider(canvas: Canvas, power: PowerSwitch, alpha: f32, out: &mut Vec<Draw>) {
    if !power.clear {
        return;
    }
    let on = slid(power);
    let cx = power_wall(canvas, power) + 11.5 - POWER_TRAVEL * on;
    // The Pocket's and Light's bar sits right under the shell's top rim and stands straight on
    // the board's switch, no gap either side (Roy 2026-11-06)
    let pocket = !power.lock;
    let (top, bottom) = match pocket {
        true => (POCKET_SLIDER_TOP, POCKET_SLIDER_BOTTOM),
        false => (SLIDER_TOP, SLIDER_BOTTOM),
    };
    let dark = if pocket { POCKET_SLIDER_DARK } else { SLIDER_DARK };
    let (w, top, bottom) = (SLIDER_W, BAND_Y + top, BAND_Y + bottom);
    let x0 = cx - w / 2.0;
    let rows = (bottom - top) as usize;
    let a = alpha * CLEAR_WALL_SEEN;
    // the knob runs on down through the shell into the bar: its square stem
    let sw = 14.0_f32;
    for row in 0..(top - BAND_Y) as usize + 1 {
        let v = (176.0 - 2.0 * row as f32) * dark;
        // seen through the shell's clear top rim: only what its plastic lets through (Roy)
        let a = a * (1.0 - RIM_ALPHA.get(row).copied().unwrap_or(0.0));
        for (x, ww, k) in [(cx - sw / 2.0, sw, 1.0), (cx - sw / 2.0, 1.0, 0.86), (cx + sw / 2.0 - 1.0, 1.0, 0.7)] {
            out.push(Draw::Rect {
                x,
                y: BAND_Y + row as f32,
                w: ww,
                h: 1.0,
                colour: [v * k / 255.0, (v - 2.0) * k / 255.0, (v - 5.0) * k / 255.0, a],
            });
        }
    }
    for row in 0..rows {
        let u = row as f32 / (rows - 1) as f32;
        // lit top edge, a soft fall to the shaded foot, as the moulded part in the photo
        let v = dark
            * match row {
                0 => 196.0,
                _ if row == rows - 1 => 104.0,
                _ => 170.0 - 38.0 * u,
            };
        // the rail notches cut into both ends, a third of the way down
        let notch = (0.34..0.6).contains(&u);
        let (x, ww) = if notch { (x0 + 3.0, w - 6.0) } else { (x0, w) };
        out.push(Draw::Rect {
            x,
            y: top + row as f32,
            w: ww,
            h: 1.0,
            colour: [v / 255.0, (v - 2.0) / 255.0, (v - 5.0) / 255.0, a],
        });
        // the ends' edges catch less light
        for (ex, shade) in [(x, 0.86), (x + ww - 1.0, 0.7)] {
            out.push(Draw::Rect {
                x: ex,
                y: top + row as f32,
                w: 1.0,
                h: 1.0,
                colour: [v * shade / 255.0, (v - 2.0) * shade / 255.0, (v - 5.0) * shade / 255.0, a],
            });
        }
    }
    if pocket {
        // the stem from the bar's foot down to the switch on the board
        let sw = 6.0_f32;
        for row in bottom as usize..(BAND_Y + POCKET_STEM_BOTTOM) as usize {
            let v = 150.0 * dark;
            for (x, ww, k) in [(cx - sw / 2.0, sw, 1.0), (cx - sw / 2.0, 1.0, 1.25), (cx + sw / 2.0 - 1.0, 1.0, 0.7)] {
                out.push(Draw::Rect {
                    x,
                    y: row as f32,
                    w: ww,
                    h: 1.0,
                    colour: [v * k / 255.0, v * k / 255.0, (v + 2.0) * k / 255.0, a],
                });
            }
        }
    }
}

fn draw_power_knob(canvas: Canvas, power: PowerSwitch, alpha: f32, out: &mut Vec<Draw>) {
    let on = slid(power);
    out.push(Draw::Tex {
        x: power_wall(canvas, power) + 1.0 + POWER_KNOB_DX
            - crate::bitmaps::POWER_KNOB_PAD as f32
            - POWER_TRAVEL * on,
        y: BAND_Y - crate::bitmaps::POWER_KNOB_H as f32,
        w: crate::bitmaps::POWER_KNOB_W as f32,
        h: crate::bitmaps::POWER_KNOB_H as f32,
        tex: power.knob,
        alpha,
    });
}

/// The switch on an empty Game Boy slot: the guide hole and the knob, off.
pub fn draw_power_switch(canvas: Canvas, power: PowerSwitch, out: &mut Vec<Draw>) {
    draw_power_hole(canvas, power, 1.0, out);
    draw_power_slider(canvas, power, 1.0, out);
    draw_power_tab(canvas, power, 1.0, out);
    draw_power_knob(canvas, power, 1.0, out);
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
        if let Some(power) = self.power.filter(|p| !p.clear) {
            draw_power_hole(canvas, power, slot_alpha, out);
        }
        let front_alpha = if self.over { chrome } else { slot_alpha };
        // The clear slot: the slider and the tab in the wall sit under the clear plastic and the
        // pak goes in front of all of it; only the knob's top and the tab once it comes out over
        // the pak's notch are drawn over the pak (Roy).
        let clear = self.power.filter(|p| p.clear);
        if let Some(power) = clear {
            draw_power_slider(canvas, power, front_alpha, out);
            draw_slot_front(canvas, self.slot, front_alpha, out);
            // the hole is cut in the clear plastic: opaque, over it (Roy)
            draw_power_hole(canvas, power, front_alpha, out);
            // the tab inside the wall, seen through the wall's own clear plastic (Roy)
            draw_power_tab_in(canvas, power, front_alpha, out);
        }

        let mould = crate::cart::mould_of(self.cart);
        let (cw, ch) = cart_box(mould);
        let (cw, ch) = (cw as f32, ch as f32);

        let scale = self.scale.clamp(0.0, 1.0);
        let (w0, h0) = (cw * scale, ch * scale);
        let (sw, sh) = seated_box(mould);
        let (sw, sh) = (sw as f32, sh as f32);
        let foot0 = foot_y(ch);
        let (rest_lift, rest_turn) = rest_pose();
        let turn;
        let (x, y, cw, ch, lid) = match tv_pose(mould, seat, self.ejecting, canvas.w / 2.0, (sw, sh)) {
            Some(pose) => {
                // Off the shelf it shrinks to its seated size on the way to the slot.
                let k = pose.leave;
                let (x0, y0) = (self.rest, foot0 - h0 - rest_lift);
                let lerp = |a: f32, b: f32| a + (b - a) * k;
                let (x, y) = (lerp(x0, pose.x), lerp(y0, pose.y));
                // Picked up on the way over, as a pak is: up in an arc with a small turn, straight
                // again by the time it waits over the lid or slot, which then go as they were.
                let arc = (std::f32::consts::PI * k).sin();
                let y = y - PICK_UP * arc;
                turn = rest_turn * (1.0 - k) + PICK_TURN * arc;
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
                let push = (PUSH_PX / (seated_foot - foot0)).min(1.0 - catch - CREEP);
                let travel = travel(seat, catch, push, self.ejecting);
                let k = (travel / catch).min(1.0);
                let (cw, ch) = (w0 + (sw - w0) * k, h0 + (sh - h0) * k);
                let centre = self.rest + w0 / 2.0;
                let x = centre + (canvas.w / 2.0 - centre) * k - cw / 2.0;
                let y = foot0 + (seated_foot - foot0) * travel - ch;
                // Picked up off the shelf: an arc up and a small turn, gone by the catch at the
                // lip, so the pak meets the slot straight and the catch and push stay as they were.
                let u = (seat / CATCH_IN).min(1.0);
                let arc = (std::f32::consts::PI * u.sqrt()).sin();
                let y = y - PICK_UP * arc - rest_lift * (1.0 - u);
                turn = rest_turn * (1.0 - u) + PICK_TURN * (std::f32::consts::PI * u).sin();
                (x, y, cw, ch, None)
            }
        };
        let cart_alpha = if seat >= 1.0 && !held { chrome } else { 1.0 };
        // A shallow slot (Pocket, Light, Game Boy Color): the pak lies on the mouth, over its
        // rims (Roy 2026-11-05), so the slot's front, the switch and the pak's soft drop shadow
        // go down first and the pak over them.
        let perched = self.perched.is_some() && clear.is_none();
        if perched {
            match self.power {
                Some(power) if power.clear => draw_power_tab_out(canvas, power, front_alpha, out),
                Some(power) => {
                    draw_power_slider(canvas, power, front_alpha, out);
                    draw_power_tab(canvas, power, front_alpha, out);
                }
                None => {}
            }
            draw_slot_front(canvas, self.slot, front_alpha, out);
        }
        if let Some(tex) = self.perched.filter(|_| perched) {
            // A plain soft drop shadow, lit from the top left, growing in as the pak settles.
            let k = ((seat - 0.6) / 0.4).clamp(0.0, 1.0);
            let k = k * k * (3.0 - 2.0 * k) * cart_alpha * front_alpha;
            let (bw, bh) = cart_box(mould);
            let (sx, sy) = (cw / bw as f32, ch / bh as f32);
            let m = crate::cart::PERCH_MARGIN as f32;
            if k > 0.0 {
                out.push(Draw::Tex {
                    x: x - m * sx,
                    y: y - m * sy,
                    w: cw + 2.0 * m * sx,
                    h: ch + 2.0 * m * sy,
                    tex,
                    alpha: PERCH_ALPHA * k,
                });
            }
        }
        // Without the DMG's lock (Pocket, Light) the switch is the case's own, beside the mouth:
        // the pak goes in front of it (Roy 2026-11-06).
        if let Some(power) = self.power.filter(|p| !p.lock) {
            draw_power_knob(canvas, power, front_alpha, out);
        }
        let tv = matches!(
            mould,
            Mould::Nes | Mould::SnesNtsc | Mould::SnesPal | Mould::SgbNtsc | Mould::SgbPal
        );
        if let (Some(tex), true) = (self.drop, tv) {
            // Gone as the cart's foot goes down into the mouth.
            let sunk = ((y + ch - BAND_Y) / ch).clamp(0.0, 1.0);
            let k = (1.0 - sunk) * cart_alpha * front_alpha;
            if k > 0.0 {
                let (bw, _) = cart_box(mould);
                let m = crate::shelf::DROP_MARGIN as f32 * cw / bw as f32;
                let fall = crate::shelf::DROP_FALL * cw / bw as f32;
                out.push(Draw::Tex {
                    x: x - m,
                    y: y - m + fall,
                    w: cw + 2.0 * m,
                    h: ch + 2.0 * m,
                    tex,
                    alpha: crate::shelf::DROP_ALPHA
                        * k
                        * if LIGHT_GROUND.load(std::sync::atomic::Ordering::Relaxed) { 0.55 } else { 1.0 },
                });
            }
        }
        let cart_from = out.len();
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

        if turn.abs() > 1e-4 {
            for d in &mut out[cart_from..] {
                if let Draw::Tex { x, y, w, h, tex, alpha } = *d {
                    *d = Draw::Turned { x, y, w, h, tex, alpha, turn };
                }
            }
        }

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

        match self.power.filter(|_| !perched) {
            Some(power) if power.clear => draw_power_tab_out(canvas, power, front_alpha, out),
            Some(power) => {
                draw_power_slider(canvas, power, front_alpha, out);
                draw_power_tab(canvas, power, front_alpha, out);
            }
            None => {}
        }
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
        if clear.is_none() && !perched {
            draw_slot_front(canvas, self.slot, front_alpha, out);
        }
        if let Some(power) = self.power.filter(|p| p.lock) {
            draw_power_knob(canvas, power, front_alpha, out);
        }

        if self.game && opening && !self.over {
            out.push(Draw::Game);
        }
    }
}

/// How dark the pak's drop shadow on a shallow slot is at its darkest.
const PERCH_ALPHA: f32 = 0.38;

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
/// How wide `tools/sgb/sgb_band.py` draws the band.
const SGB_ART_W: f32 = 720.0;

/// The Super Game Boy in its band, as `tools/sgb/sgb_band.py` draws it in the 720-wide strip:
/// its left edge and size.
fn sgb_in_band(model: crate::SgbModel) -> (f32, f32, f32) {
    match model {
        crate::SgbModel::Ntsc => (89.0, 543.0, 475.0),
        _ => (123.0, 472.0, 448.0),
    }
}

/// Where the Super Game Boy's whole face is while it is the band: (x, y, w, h).
pub fn sgb_band_rect(model: crate::SgbModel, canvas: Canvas) -> (f32, f32, f32, f32) {
    let (x, w, h) = sgb_in_band(model);
    // The art is drawn 720 wide and loaded cut to the strip's middle, which the canvas cuts
    // again to its own width.
    let strip = crate::bitmaps::SLOT_W as f32;
    let cut = (SGB_ART_W - strip) / 2.0 + ((strip - canvas.w) / 2.0).max(0.0);
    let top = OUT_H as f32 - crate::bitmaps::SGB_BAND_H as f32;
    (x - cut, top + SGB_TOP, w, h)
}

/// Where the Super Game Boy stands once it is in the SNES: (x, y, w, h).
pub fn sgb_seated_rect(mould: Mould, canvas: Canvas) -> (f32, f32, f32, f32) {
    let (sw, sh) = seated_box(mould);
    let (sw, sh) = (sw as f32, sh as f32);
    match tv_pose(mould, 1.0, false, canvas.w / 2.0, (sw, sh)) {
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

/// The selected cart's lift and turn on the shelf (bits of two f32s), where an insert picks it up
/// and an eject sets it down.
static REST_POSE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn set_rest_pose(lift: f32, turn: f32) {
    let bits = (u64::from(lift.to_bits()) << 32) | u64::from(turn.to_bits());
    REST_POSE.store(bits, std::sync::atomic::Ordering::Relaxed);
}

fn rest_pose() -> (f32, f32) {
    let bits = REST_POSE.load(std::sync::atomic::Ordering::Relaxed);
    (f32::from_bits((bits >> 32) as u32), f32::from_bits(bits as u32))
}

/// On the way to the slot a pak is picked up: it rises this far over the first part of its way
/// and turns a little as it is lifted, straight again before it reaches the slot's lip.
const PICK_UP: f32 = 26.0;
const PICK_TURN: f32 = 0.06;

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

/// How dark the Super Game Boy's slot lies below its top, and the shade its rim throws in it.
const SGB_MOUTH_DIM: f32 = 0.22;
const SGB_MOUTH_SHADE: f32 = 14.0;
const SGB_MOUTH_SHADE_ALPHA: f32 = 0.7;

/// The Super Game Boy's slot for the Game Boy pak lies deeper than its top, as the Game Boy
/// slot's mouth does: darker, with a soft shade from the rim falling over it. A pak in it
/// covers it, so it stays light around the pak (Roy).
fn draw_sgb_mouth(canvas: Canvas, alpha: f32, out: &mut Vec<Draw>) {
    let w = SEATED_W as f32 + 4.0;
    let (x, top) = (canvas.centred(w), BAND_Y - rise() + SGB_TOP);
    out.push(band(x, top, w, OUT_H as f32 - top, [0.0, 0.0, 0.0, SGB_MOUTH_DIM], alpha));
    const STEPS: usize = 8;
    let step = SGB_MOUTH_SHADE / STEPS as f32;
    for k in 0..STEPS {
        // Darkest under the rim, easing out to nothing.
        let t = (STEPS - k) as f32 / STEPS as f32;
        let a = SGB_MOUTH_SHADE_ALPHA * t * t / STEPS as f32 * 2.2;
        out.push(band(x, top, w, step * (k + 1) as f32, [0.0, 0.0, 0.0, a], alpha));
    }
}

/// The Super Game Boy's soft shadow on the wallpaper round it: its own outline in rings.
const SGB_SHADOW_RINGS: [(f32, f32); 3] = [(2.0, 0.10), (5.0, 0.06), (9.0, 0.035)];

fn draw_sgb_shadow(canvas: Canvas, back: TexId, alpha: f32, out: &mut Vec<Draw>) {
    let alpha = match LIGHT_GROUND.load(std::sync::atomic::Ordering::Relaxed) {
        true => alpha * 0.55,
        false => alpha,
    };
    let strip_w = crate::bitmaps::SLOT_W as f32;
    let (x0, y0) = ((canvas.w - strip_w) / 2.0, BAND_Y - rise());
    let [r, g, b] = shadow_tone(canvas.w / 2.0, y0);
    for (radius, a) in SGB_SHADOW_RINGS {
        for k in 0..8 {
            let turn = k as f32 * std::f32::consts::FRAC_PI_4;
            out.push(Draw::Shade {
                x: x0 + radius * turn.cos(),
                // Lit from above: the shadow leans down a touch.
                y: y0 + radius * turn.sin() + radius * 0.3,
                w: strip_w,
                h: MOUTH_H + rise(),
                tex: back,
                colour: [r, g, b, a * alpha],
            });
        }
    }
}

fn draw_slot_back(canvas: Canvas, slot: Option<(TexId, TexId)>, alpha: f32, out: &mut Vec<Draw>) {
    if let Some((back, _)) = slot {
        // The Super Game Boy stands alone, no console around it to cast the shadow.
        match rise() == 0.0 {
            true => draw_console_shadow(canvas, alpha, out),
            // It still sits a little in the wallpaper: a soft shadow round its own outline
            // (Roy 2026-11-12, sh7).
            false => draw_sgb_shadow(canvas, back, alpha, out),
        }
        out.push(strip(canvas, back, alpha));
        if rise() > 0.0 {
            draw_sgb_mouth(canvas, alpha, out);
        }
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

/// Lifts the shelf line so its letters sit midway between the centre cart and the slot (Roy).
const SLOT_NAME_GAP: f32 = 13.0;

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
    shadowed(tex, canvas.centred(w), slot_name_y(), w, h, ink, alpha, out);
}

/// A soft shadow hugging the shelf's text, then the text: readable over busy and light
/// wallpapers (Roy). Dark ink (light themes) gets a pale shadow instead, so it lifts off the
/// wallpaper the same way.
fn shadowed(tex: TexId, x: f32, y: f32, w: f32, h: f32, ink: [f32; 4], alpha: f32, out: &mut Vec<Draw>) {
    // On whole pixels: a name of odd width centred lands half a pixel over and goes soft.
    let (x, y) = (x.round(), y.round());
    // The shadow is a dozen stacked layers, so at half alpha it still looks nearly whole while
    // the words are half gone. Cubed, it fades out ahead of the words and comes in after
    // them: never a shadow without its text (Roy 2026-11-13). At full alpha nothing changes.
    soft_shadow(tex, x, y, w, h, ink, alpha.clamp(0.0, 1.0).powi(3), out);
    out.push(Draw::Tint {
        x,
        y,
        w,
        h,
        tex,
        colour: [ink[0], ink[1], ink[2], ink[3] * alpha],
    });
}

/// Only the shadow of a white face tinted `ink`: drawn before the face, wherever text sits on
/// the wallpaper (the shelf's names, the menus' help line and legend). Close and full but soft:
/// a dense core one pixel down, a ring that fades within two pixels (Roy 2026-11-07).
pub fn soft_shadow(tex: TexId, x: f32, y: f32, w: f32, h: f32, ink: [f32; 4], alpha: f32, out: &mut Vec<Draw>) {
    soft_shadow_over(tex, x, y, w, h, ink, alpha, true, out);
}

/// `soft_shadow`, over the wallpaper (its colour, ui6) or over a flat ground (plain black).
pub fn soft_shadow_over(
    tex: TexId,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    ink: [f32; 4],
    alpha: f32,
    wallpaper: bool,
    out: &mut Vec<Draw>,
) {
    let luma = 0.299 * ink[0] + 0.587 * ink[1] + 0.114 * ink[2];
    // Light words get a shadow in the wallpaper's own colour, burnt deep: no grey haze over
    // a light or colourful ground (Roy 2026-11-12, ui6). Dark words keep their pale one.
    let [r, g, b] = match (luma > 0.5, wallpaper) {
        (true, true) => shadow_tone(x + w / 2.0, y + h / 2.0),
        (true, false) => [0.0; 3],
        (false, _) => [1.0; 3],
    };
    // On a dark ground a black shadow barely shows: twice as dense there, so the keys' white
    // caps and the words stand off it in the dark theme too (Roy 2026-11-12, ui7).
    let dark = luma > 0.5 && 0.299 * r + 0.587 * g + 0.114 * b < 0.06;
    let a = ink[3] * alpha * if dark { 2.0 } else { 1.0 };
    if a <= 0.0 {
        return;
    }
    for (dx, dy, k) in SHADOW {
        out.push(Draw::Tint {
            x: x + dx,
            y: y + dy,
            w,
            h,
            tex,
            colour: [r, g, b, a * k],
        });
    }
}

/// The wallpaper in a coarse grid of burnt colours, for shadows over it (ui6).
pub struct ShadowGround {
    tones: Vec<[f32; 3]>,
    h: f32,
    /// How light the wallpaper is where the shelf's names sit, under the carts (0 to 1).
    band_luma: f32,
}

/// Where the shelf's console name and game title sit on a 480-line wallpaper, the middle half
/// of its width.
const NAME_BAND: (usize, usize) = (360, 425);

const TONE_COLS: usize = 20;
const TONE_ROWS: usize = 15;
/// How deep a burnt tone goes, and how much its colour is pushed.
const BURN_VALUE: f32 = 0.14;
const BURN_CHROMA: f32 = 2.0;

impl ShadowGround {
    /// `rgba` is the wallpaper as drawn, `w` x `h`.
    pub fn new(rgba: &[u8], w: u32, h: u32) -> Self {
        let (w, h) = (w as usize, h as usize);
        let mut sum = vec![[0.0f32; 4]; TONE_COLS * TONE_ROWS];
        let (mut band, mut band_n) = (0.0f32, 0.0f32);
        let (b0, b1) = (NAME_BAND.0 * h / 480, NAME_BAND.1 * h / 480);
        for (i, px) in rgba.chunks_exact(4).enumerate().step_by(3) {
            let (x, y) = (i % w, i / w);
            if (b0..b1).contains(&y) && (w / 4..w * 3 / 4).contains(&x) {
                band += (0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32) / 255.0;
                band_n += 1.0;
            }
            let cell = &mut sum[(y * TONE_ROWS / h) * TONE_COLS + x * TONE_COLS / w];
            for k in 0..3 {
                cell[k] += px[k] as f32 / 255.0;
            }
            cell[3] += 1.0;
        }
        let tones = sum
            .iter()
            .map(|c| match c[3] > 0.0 {
                true => burn([c[0] / c[3], c[1] / c[3], c[2] / c[3]]),
                false => [0.0; 3],
            })
            .collect();
        Self { tones, h: h as f32, band_luma: if band_n > 0.0 { band / band_n } else { 0.0 } }
    }

    /// The wallpaper is light under the shelf's names, with `dim` of black over it (Wallpaper
    /// Tint): they go black there, white on a dark one (Roy 2026-11-17), whatever the menus do.
    pub fn light(&self, dim: f32) -> bool {
        self.band_luma * (1.0 - dim) > 0.55
    }
}

/// A colour taken down to a deep tone of itself, its hue pushed: color burn, not black.
fn burn(c: [f32; 3]) -> [f32; 3] {
    let top = c[0].max(c[1]).max(c[2]);
    if top <= 0.0 {
        return [0.0; 3];
    }
    let v = top * BURN_VALUE;
    c.map(|k| (1.0 - (1.0 - k / top) * BURN_CHROMA).clamp(0.0, 1.0) * v)
}

thread_local! {
    static SHADOW_GROUND: std::cell::RefCell<Option<(std::rc::Rc<ShadowGround>, f32)>> =
        const { std::cell::RefCell::new(None) };
}

/// The wallpaper under this frame's shadows, stretched over `canvas_w`; None on black: black
/// shadows.
pub fn set_shadow_ground(ground: Option<std::rc::Rc<ShadowGround>>, canvas_w: f32) {
    SHADOW_GROUND.with(|g| *g.borrow_mut() = ground.map(|g| (g, canvas_w)));
}

fn shadow_tone(x: f32, y: f32) -> [f32; 3] {
    SHADOW_GROUND.with(|g| {
        let g = g.borrow();
        let Some((g, canvas_w)) = g.as_ref() else {
            return [0.0; 3];
        };
        let col = ((x / canvas_w * TONE_COLS as f32) as usize).min(TONE_COLS - 1);
        let row = ((y / g.h * TONE_ROWS as f32) as usize).min(TONE_ROWS - 1);
        g.tones[row * TONE_COLS + col]
    })
}

/// (dx, dy, opacity) of each copy, centred one pixel below the text: outer ring first.
const SHADOW: [(f32, f32, f32); 13] = [
    (-2.0, 1.0, 0.10),
    (2.0, 1.0, 0.10),
    (0.0, -1.0, 0.10),
    (0.0, 3.0, 0.10),
    (-1.0, 0.0, 0.18),
    (1.0, 0.0, 0.18),
    (-1.0, 2.0, 0.18),
    (1.0, 2.0, 0.18),
    (-1.0, 1.0, 0.26),
    (1.0, 1.0, 0.26),
    (0.0, 0.0, 0.26),
    (0.0, 2.0, 0.26),
    (0.0, 1.0, 0.45),
];

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
    let y = slot_name_y() + HINT_H as f32 - h;
    shadowed(tex, canvas.centred(w), y, w, h, ink, alpha, out);
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

/// The empty slot with its power switch, off. On the clear slot the slider, the knob's stem and
/// the tab sit behind the shell's plastic, so they go between its back and front (Roy).
pub fn draw_empty_slot_power(
    canvas: Canvas,
    slot: Option<(TexId, TexId)>,
    power: Option<PowerSwitch>,
    out: &mut Vec<Draw>,
) {
    let off = power.map(|p| PowerSwitch { on: 0.0, ..p });
    draw_slot_back(canvas, slot, 1.0, out);
    if let Some(p) = off.filter(|p| p.clear) {
        draw_power_slider(canvas, p, 1.0, out);
    }
    draw_slot_front(canvas, slot, 1.0, out);
    if let Some(p) = off.filter(|p| p.clear) {
        draw_power_hole(canvas, p, 1.0, out);
    }
    match off {
        Some(p) if p.clear => {
            // the tab inside the wall, seen through the wall's own clear plastic (Roy)
            draw_power_tab(canvas, p, 1.0, out);
            draw_power_knob(canvas, p, 1.0, out);
        }
        Some(p) => draw_power_switch(canvas, p, out),
        None => {}
    }
}

/// Where a Game Boy pak is, 0 at the shelf to 1 seated: in to the catch at the lip, a short
/// creep, then down against the spring and, the last `push` of the way, the same thumb push
/// the NES and SNES carts take (Roy). Out again it comes straight up without the push.
fn travel(seat: f32, catch: f32, push: f32, ejecting: bool) -> f32 {
    let caught = catch + CREEP;
    if seat < CATCH_IN {
        catch * ease(seat / CATCH_IN)
    } else if ejecting {
        if seat < CATCH_OUT {
            catch + CREEP * (seat - CATCH_IN) / (CATCH_OUT - CATCH_IN)
        } else {
            caught + (1.0 - caught) * ease((seat - CATCH_OUT) / (1.0 - CATCH_OUT))
        }
    } else if seat < SLIDE_IN {
        catch + CREEP * (seat - CATCH_IN) / (SLIDE_IN - CATCH_IN)
    } else if seat < PUSH_IN {
        let slid = 1.0 - push;
        caught + (slid - caught) * resist((seat - SLIDE_IN) / (PUSH_IN - SLIDE_IN))
    } else {
        1.0 - push + push * seat_push((seat - PUSH_IN) / (1.0 - PUSH_IN))
    }
}

/// The Game Boy insert's steps: the creep at the catch ends at `SLIDE_IN`, the last push starts
/// at `PUSH_IN`, over the last `PUSH_PX` as for the NES and SNES carts.
const SLIDE_IN: f32 = 0.50;
const PUSH_IN: f32 = 0.70;
const PUSH_PX: f32 = 14.0;
/// The board bites the connector: where the push's snap home is, as for NES and SNES.
const GB_CLICK: f32 = PUSH_IN + (1.0 - PUSH_IN) * 0.8;

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
        Mould::Nes => NES_CLICK / NES_STORY,
        Mould::SnesNtsc | Mould::SnesPal | Mould::SgbNtsc | Mould::SgbPal => SNES_CLICK / SNES_STORY,
        _ => GB_CLICK,
    }
}

/// How long an insert takes. The poses below are timed on a slower story (`*_STORY`, in its own
/// seconds) that plays out in this time, so every step speeds up alike (Roy).
const NES_T: f32 = 0.90;
const NES_STORY: f32 = 1.60;
const NES_CLICK: f32 = 1.15;
const SNES_T: f32 = 0.70;
const SNES_STORY: f32 = 1.15;
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

fn tv_pose(
    mould: Mould,
    seat: f32,
    ejecting: bool,
    cx: f32,
    (sw, sh): (f32, f32),
) -> Option<TvPose> {
    let x = cx - sw / 2.0;
    let lerp = |a: f32, b: f32, u: f32| a + (b - a) * u;
    match mould {
        Mould::Nes if ejecting => {
            // Out in one go: the lid swings open, the cart rises out of the mouth and heads
            // back to the shelf while the lid falls shut behind it.
            let e = 1.0 - seat;
            let wait = BAND_Y - 6.0 - sh;
            let home = BAND_Y + 6.0;
            let lid = 90.0 * smooth(e / 0.30) * (1.0 - smooth((e - 0.70) / 0.30));
            let y = lerp(home, wait, smooth((e - 0.15) / 0.50));
            let leave = 1.0 - smooth((e - 0.60) / 0.40);
            Some(TvPose { x, y, leave, lid: Some(lid) })
        }
        Mould::Nes => {
            let t = seat * NES_STORY;
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
            let t = seat * SNES_STORY;
            let pal = matches!(mould, Mould::SnesPal | Mould::SgbPal);
            let wait = BAND_Y - sh - 8.0;
            // NTSC: the slot's edge meets the third line on the cart's side grips. A PAL cart goes
            // in as deep and so stands a little prouder. A Super Game Boy goes in as deep as the
            // game cart it is shaped like, and its taller body stands out above.
            let out = if pal { (86.0 - 42.6) / 86.0 } else { 132.0 / 257.0 };
            let depth = match mould {
                // The American SGB goes in like an American game: the slot's edge meets the
                // groove just under its label.
                Mould::SgbNtsc => sh * (1.0 - 0.693),
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
    // Its shadow on the wallpaper above, a soft band that deepens as it stands up. Drawn one whole
    // pixel row at a time with a smooth fall-off measured from the lid's true edge, so it fades
    // without steps and glides instead of jumping a row as the lid turns (Roy).
    let shadow = if light { 0.27 } else { 0.43 };
    let spread = 7.0 * sin + 2.0;
    let edge = top + 1.0;
    let first = (edge - spread).floor();
    let mut r = first;
    while r < edge {
        // Coverage-weighted distance of this row's middle from the edge, 0 at the lid, 1 at the
        // far end of the shadow; the row the lid edge sits in only counts the part above it.
        let bottom = (r + 1.0).min(edge);
        let d = ((edge - (r + bottom) / 2.0) / spread).clamp(0.0, 1.0);
        let fall = 1.0 - d * d * (3.0 - 2.0 * d);
        let a = shadow * fall * (bottom - r);
        if a > 0.002 {
            // Its ends fade out too, softer the further the shadow falls from the lid, so it
            // never stops in a hard corner on the wallpaper.
            // The fade starts a little outside the lid and runs well inside it (Roy: the ends
            // were cut off too sharply).
            let soft = (14.0 + 14.0 * d).round() as usize;
            let out_by = soft as f32 * 0.3;
            for j in 0..soft {
                let inset = j as f32 - out_by;
                let k = a / soft as f32;
                out.push(band(x + inset, r, w - 2.0 * inset, 1.0, [0.0, 0.0, 0.0, k], alpha));
            }
        }
        r += 1.0;
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
