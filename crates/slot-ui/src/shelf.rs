use slot_gfx::{Canvas, Draw, TexId, OUT_H};
use slot_store::{Cart, FavoriteMark};
use std::sync::OnceLock;

use crate::bitmaps::Mould;
use crate::board::{board_in, Placed};
use crate::cart::{
    cart_box, is_clear, label_colour, label_rect, label_text, mould_of, CART_H, CART_W,
};
use crate::hud::Millis;
use crate::slot_chrome::{draw_empty_slot, MOUTH_H};

const SIDE_SCALE: f32 = 0.78;
const MOULDS: usize = Mould::ALL.len();
/// How large the carts stand on the carousel, the middle one included.
const CAROUSEL: f32 = 0.88;
const SIDE_ALPHA: f32 = 1.0;

pub fn rest_y(h: f32) -> f32 {
    // A tall pak (Game Boy, NES) centres in the gap; a wide cart (GBA, SNES) stands on the
    // GBA's ground, so every wide cart sits at the same height.
    let y = if h > TALL_H {
        // Above a Super Game Boy the whole shelf stands as much higher as its top does, the
        // shelf's name still just above it (Roy).
        (OUT_H as f32 - MOUTH_H - h) / 2.0 - crate::slot_chrome::rise()
    } else {
        (OUT_H + CART_H) as f32 / 2.0 - h
    };
    // A Game Boy pak is 337px in a 422px gap, which centres at 42.5. Snap to a
    // whole pixel so the 312×337 bitmap hits the RG SP panel 1:1.
    y.round()
}

/// Taller than any wide cart, shorter than any pak.
const TALL_H: f32 = 300.0;

pub fn foot_y(h: f32) -> f32 {
    rest_y(h) + h
}
const OMEGA: f32 = 16.0;
const PART: f32 = 130.0;

const SLOTS: i32 = 3;

pub const REPEAT_DELAY_MS: Millis = 400;

pub const GLOW_MARGIN: u32 = 130;
/// A favourite stands this much higher than the carts beside it.
const LIFT: f32 = 10.0;
/// The Favorites light waits for the cart to come this close to standing still...
const SETTLED: f32 = 0.04;
/// ...then rises over this long. It fades out quicker, as soon as the carts move.
const LIGHT_UP_S: f32 = 0.22;
const LIGHT_DOWN_S: f32 = 0.12;
/// How far a cart's drop shadow spreads, and how far below the cart it falls.
pub const DROP_MARGIN: u32 = 22;
const DROP_FALL: f32 = 10.0;
const DROP_ALPHA: f32 = 0.22;
pub const GLOW_GOLD: [u8; 3] = [0xff, 0xdc, 0x9a];
/// The glow's amber, and how much stronger it is laid on than the face's own alpha.
const GLOW_AMBER: ([f32; 3], f32) = ([0.88, 0.63, 0.25], 1.6);
pub const GLOW_WHITE: [u8; 3] = [0xff, 0xff, 0xff];
pub const STAR_PX: u32 = 22;
const STAR_INSET: f32 = 5.0;
const REPEAT_MS: [Millis; 4] = [110, 85, 65, 50];

pub fn repeat_gap(fired: usize) -> Millis {
    REPEAT_MS[fired.min(REPEAT_MS.len() - 1)]
}

pub struct Shelf {
    pub carts: Vec<Cart>,
    pub index: usize,
    pub scroll: f32,
    faces: Vec<Option<TexId>>,
    interiors: Vec<Option<TexId>>,
    /// Per mould, in `Mould::ALL` order: the cart's shadow, and the board behind a clear one.
    shadows: [Option<TexId>; MOULDS],
    backings: [Option<TexId>; MOULDS],
    moulds: Vec<OnceLock<Mould>>,
    clears: Vec<OnceLock<bool>>,
    starred: Vec<bool>,
    glows: [[Option<TexId>; MOULDS]; 2],
    star: Option<(TexId, f32, f32)>,
    mark: FavoriteMark,
    /// One still backlight behind the middle of the shelf, for the Favorites shelf.
    spotlight: bool,
    drops: [Option<TexId>; MOULDS],
    /// Light grounds take the glow as a tint; dark ones take it as added light.
    light_ground: bool,
    ride: f32,
    vel: f32,
    /// How far the Favorites shelf's still light has come up: it goes out while the carts move
    /// and rises again once one stands still.
    lit: f32,
    held: Option<(i32, Millis, usize)>,
}

impl Shelf {
    pub fn new(carts: Vec<Cart>) -> Self {
        Shelf {
            moulds: carts.iter().map(|_| OnceLock::new()).collect(),
            clears: carts.iter().map(|_| OnceLock::new()).collect(),
            faces: vec![None; carts.len()],
            interiors: vec![None; carts.len()],
            starred: vec![false; carts.len()],
            glows: [[None; MOULDS]; 2],
            star: None,
            mark: FavoriteMark::default(),
            spotlight: false,
            drops: [None; MOULDS],
            light_ground: false,
            carts,
            index: 0,
            scroll: 0.0,
            shadows: [None; MOULDS],
            backings: [None; MOULDS],
            ride: 0.0,
            vel: 0.0,
            lit: 1.0,
            held: None,
        }
    }

    fn mould(&self, i: usize) -> Mould {
        let (Some(cell), Some(cart)) = (self.moulds.get(i), self.carts.get(i)) else {
            return Mould::Gba;
        };
        *cell.get_or_init(|| mould_of(cart))
    }

    fn clear(&self, i: usize) -> bool {
        let (Some(cell), Some(cart)) = (self.clears.get(i), self.carts.get(i)) else {
            return false;
        };
        *cell.get_or_init(|| is_clear(cart))
    }

    pub fn restock(&mut self, carts: Vec<Cart>) -> Vec<TexId> {
        let old: Vec<TexId> = self
            .faces
            .drain(..)
            .chain(self.interiors.drain(..))
            .flatten()
            .collect();
        self.moulds = carts.iter().map(|_| OnceLock::new()).collect();
        self.clears = carts.iter().map(|_| OnceLock::new()).collect();
        self.faces = vec![None; carts.len()];
        self.interiors = vec![None; carts.len()];
        self.starred = vec![false; carts.len()];
        self.carts = carts;
        self.held = None;
        self.select(0);
        old
    }

    pub fn select(&mut self, i: usize) {
        self.index = i;
        self.scroll = i as f32;
        self.ride = i as f32;
        self.vel = 0.0;
    }

    pub fn set_glow(&mut self, style: FavoriteMark, mould: Mould, face: TexId) {
        let row = (style == FavoriteMark::Highlight) as usize;
        self.glows[row][mould.index()] = Some(face);
    }

    pub fn set_star(&mut self, face: TexId, w: u32, h: u32) {
        self.star = Some((face, w as f32, h as f32));
    }

    pub fn set_drop(&mut self, mould: Mould, face: TexId) {
        self.drops[mould.index()] = Some(face);
    }

    pub fn set_light_ground(&mut self, light: bool) {
        self.light_ground = light;
    }

    pub fn set_spotlight(&mut self, on: bool) {
        self.spotlight = on;
    }

    /// The glow is laid on as amber colour on every ground (Roy): added light vanishes on a
    /// light one.
    fn glow_draw(&self, light: Draw) -> Draw {
        let Draw::Light { x, y, w, h, tex, alpha } = light else {
            return light;
        };
        let (c, boost) = GLOW_AMBER;
        let g = GLOW_GOLD.map(|v| v as f32 / 255.0);
        Draw::Tint {
            x,
            y,
            w,
            h,
            tex,
            colour: [c[0] / g[0], c[1] / g[1], c[2] / g[2], (alpha * boost).min(1.0)],
        }
    }

    fn glow_for(&self, i: usize) -> Option<TexId> {
        let row = (self.mark == FavoriteMark::Highlight) as usize;
        self.glows[row][self.mould(i).index()]
    }

    pub fn set_mark(&mut self, mark: FavoriteMark) {
        self.mark = mark;
    }

    pub fn mark_starred(&mut self, starred: impl Fn(&Cart) -> bool) {
        self.starred = self.carts.iter().map(starred).collect();
    }

    pub fn set_shadow(&mut self, mould: Mould, face: TexId) {
        self.shadows[mould.index()] = Some(face);
    }

    pub fn set_backing(&mut self, mould: Mould, face: TexId) {
        self.backings[mould.index()] = Some(face);
    }

    pub fn set_faces(&mut self, faces: Vec<TexId>) {
        let mut faces = faces.into_iter();
        for face in &mut self.faces {
            *face = faces.next();
        }
    }

    pub fn face(&self, i: usize) -> Option<TexId> {
        self.faces.get(i).copied().flatten()
    }

    pub fn set_face(&mut self, i: usize, tex: TexId) -> Option<TexId> {
        self.faces.get_mut(i).and_then(|f| f.replace(tex))
    }

    pub fn set_interior(&mut self, i: usize, tex: TexId) -> Option<TexId> {
        self.interiors.get_mut(i).and_then(|f| f.replace(tex))
    }

    pub fn take_interior(&mut self, i: usize) -> Option<TexId> {
        self.interiors.get_mut(i).and_then(Option::take)
    }

    /// A cart drawn again in another shell: its mould and see-through plastic are worked out
    /// afresh, so a cart turned clear shows its board on the shelf and in the slot.
    pub fn restyled(&mut self, i: usize) {
        if let Some(cell) = self.moulds.get_mut(i) {
            *cell = OnceLock::new();
        }
        if let Some(cell) = self.clears.get_mut(i) {
            *cell = OnceLock::new();
        }
    }

    pub fn take_face(&mut self, i: usize) -> Option<TexId> {
        self.faces.get_mut(i).and_then(Option::take)
    }

    pub fn on_screen(&self) -> Vec<usize> {
        self.around(SLOTS)
    }

    pub fn around(&self, radius: i32) -> Vec<usize> {
        let mut out = Vec::new();
        for step in 0..=radius {
            for off in [step, -step] {
                if let Some(i) = self.cart_at_offset(off) {
                    if !out.contains(&i) {
                        out.push(i);
                    }
                }
            }
        }
        out
    }

    /// What a see-through cart shows behind its front shell, back plastic first and then the board.
    /// The shelf draws these under the face. The slot has to as well, or the board vanishes
    /// as the cart goes in or comes out.
    pub fn behind(&self, stem: &str) -> [Option<TexId>; 2] {
        let Some(i) = self.carts.iter().position(|c| c.stem == stem) else {
            return [None, None];
        };
        if !self.clear(i) || self.face(i).is_none() {
            return [None, None];
        }
        let board = self.backings[self.mould(i).index()];
        [self.interiors.get(i).copied().flatten(), board]
    }

    /// The shadow a cart standing at `at` throws on the shelf, as a raised favourite's does
    /// when `rise` lifts it off; `fade` takes it away as the cart leaves the shelf.
    pub fn draw_drop(
        &self,
        stem: &str,
        at: Placed,
        scale: f32,
        rise: f32,
        fade: f32,
        out: &mut Vec<Draw>,
    ) {
        let Some(i) = self.carts.iter().position(|c| c.stem == stem) else {
            return;
        };
        let Some(tex) = self.drops[self.mould(i).index()] else {
            return;
        };
        let m = DROP_MARGIN as f32 * scale;
        out.push(Draw::Tex {
            x: at.x - m,
            y: at.y - m + DROP_FALL * scale + rise * 0.5,
            w: at.w + 2.0 * m,
            h: at.h + 2.0 * m,
            tex,
            alpha: DROP_ALPHA
                * fade
                * (1.0 + rise / LIFT * 0.3).min(1.6)
                * if self.light_ground { 0.55 } else { 1.0 },
        });
    }

    pub fn find(&self, stem: &str) -> Option<(&Cart, Option<TexId>)> {
        let i = self.carts.iter().position(|c| c.stem == stem)?;
        Some((&self.carts[i], self.face(i)))
    }

    pub fn left(&mut self) {
        self.step(-1);
    }

    pub fn right(&mut self) {
        self.step(1);
    }

    pub fn jump_next_letter(&mut self) {
        self.jump(1);
    }

    pub fn jump_prev_letter(&mut self) {
        self.jump(-1);
    }

    fn start_of_letter(&self, from: usize) -> usize {
        let n = self.carts.len();
        let letter = slot_store::initial(&self.carts[from].stem);
        let mut at = from;
        for _ in 0..n {
            let before = (at as i32 - 1).rem_euclid(n as i32) as usize;
            if slot_store::initial(&self.carts[before].stem) != letter {
                break;
            }
            at = before;
        }
        at
    }

    fn jump(&mut self, dir: i32) {
        let n = self.carts.len();
        if n < 2 {
            return;
        }
        let wrap = |i: i32| i.rem_euclid(n as i32) as usize;
        let here = slot_store::initial(&self.carts[self.index].stem);
        if self
            .carts
            .iter()
            .all(|c| slot_store::initial(&c.stem) == here)
        {
            return;
        }
        let target = match dir > 0 {
            true => {
                let mut at = self.index;
                for _ in 0..n {
                    at = wrap(at as i32 + 1);
                    if slot_store::initial(&self.carts[at].stem) != here {
                        break;
                    }
                }
                at
            }
            false => {
                let start = self.start_of_letter(self.index);
                match start == self.index {
                    true => self.start_of_letter(wrap(start as i32 - 1)),
                    false => start,
                }
            }
        };
        let ahead = (target as i32 - self.index as i32).rem_euclid(n as i32);
        let delta = match dir > 0 {
            true => ahead,
            false => ahead - n as i32,
        };
        self.index = target;
        self.ride += delta as f32;
        let to = self.scroll_target();
        self.scroll = to - (to - self.scroll).clamp(-1.0, 1.0);
    }

    pub fn hold_left(&mut self, now: Millis) {
        self.hold(-1, now);
    }

    pub fn hold_right(&mut self, now: Millis) {
        self.hold(1, now);
    }

    fn hold(&mut self, by: i32, now: Millis) {
        self.step(by);
        self.held = Some((by, now + REPEAT_DELAY_MS, 0));
    }

    pub fn release_left(&mut self) {
        self.release(-1);
    }

    pub fn release_right(&mut self) {
        self.release(1);
    }

    fn release(&mut self, by: i32) {
        if matches!(self.held, Some((held, _, _)) if held == by) {
            self.held = None;
        }
    }

    pub fn release_hold(&mut self) {
        self.held = None;
    }

    pub fn tick(&mut self, now: Millis) {
        let Some((by, due, fired)) = self.held else {
            return;
        };
        if now < due {
            return;
        }
        self.step(by);
        self.held = Some((by, now + repeat_gap(fired), fired + 1));
    }

    fn step(&mut self, by: i32) {
        let n = self.carts.len();
        if n < 2 {
            return;
        }
        self.index = (self.index as i32 + by).rem_euclid(n as i32) as usize;
        self.ride += by as f32;
    }

    pub fn scroll_target(&self) -> f32 {
        let n = self.carts.len();
        if n == 0 {
            return 0.0;
        }
        let from = self.ride;
        let n = n as f32;
        from + (self.index as f32 - from + n / 2.0).rem_euclid(n) - n / 2.0
    }

    pub fn cart_at_offset(&self, off: i32) -> Option<usize> {
        let n = self.carts.len() as i32;
        if n == 0 {
            return None;
        }
        if n == 1 {
            return (off == 0).then_some(self.index);
        }
        Some((self.index as i32 + off).rem_euclid(n) as usize)
    }

    pub fn selected_at(&self, canvas: Canvas) -> (f32, f32) {
        let (w, _) = self.slot_width(0);
        let scale = shrink(self.scroll_target() - self.scroll);
        (self.centre(canvas, 0) - w * scale / 2.0, scale)
    }

    fn slot_width(&self, slot: i32) -> (f32, bool) {
        match self.cart_at_offset(slot).map(|i| self.mould(i)) {
            Some(Mould::Gba) | None => (CART_W as f32, true),
            Some(m) => (cart_box(m).0 as f32, false),
        }
    }

    fn pitch(&self, a: i32, u: f32) -> f32 {
        let ((wa, gba_a), (wb, gba_b)) = (self.slot_width(a), self.slot_width(a + 1));
        let (sa, sb) = (shrink(a as f32 - u), shrink((a + 1) as f32 - u));
        let gap_of = if gba_a || gba_b { CART_W as f32 } else { wa };
        wa * sa / 2.0 + gap_of * (CAROUSEL - (sa + sb) / 2.0) + wb * sb / 2.0
    }

    fn centre(&self, canvas: Canvas, slot: i32) -> f32 {
        let u = self.scroll - self.scroll_target();
        let a = u.floor() as i32;
        let mut c = canvas.w / 2.0 - (u - a as f32) * self.pitch(a, u);
        if slot >= a {
            for s in a..slot {
                c += self.pitch(s, u);
            }
        } else {
            for s in (slot..a).rev() {
                c -= self.pitch(s, u);
            }
        }
        c
    }

    pub fn update(&mut self, dt: f32) {
        let accel = -2.0 * OMEGA * self.vel - OMEGA * OMEGA * (self.scroll - self.scroll_target());
        self.vel += accel * dt;
        self.scroll += self.vel * dt;
        self.lit = if (self.scroll - self.scroll_target()).abs() > SETTLED {
            (self.lit - dt / LIGHT_DOWN_S).max(0.0)
        } else {
            (self.lit + dt / LIGHT_UP_S).min(1.0)
        };
    }

    pub fn draw(
        &self,
        canvas: Canvas,
        shake: f32,
        slot: Option<(TexId, TexId)>,
        out: &mut Vec<Draw>,
    ) {
        self.draw_row(canvas, None, shake, 0.0, 1.0, out);
        draw_empty_slot(canvas, slot, out);
    }

    pub fn draw_row(
        &self,
        canvas: Canvas,
        hidden: Option<&str>,
        shake: f32,
        recede: f32,
        dim: f32,
        out: &mut Vec<Draw>,
    ) {
        let recede = recede.clamp(0.0, 1.0);
        let dim = dim.clamp(0.0, 1.0);
        let target = self.scroll_target();
        // The lights go behind every cart: gathered on the way, laid down before the first.
        let behind = out.len();
        let mut lights = Vec::new();
        for slot in -SLOTS..=SLOTS {
            let Some(i) = self.cart_at_offset(slot) else {
                continue;
            };
            let cart = &self.carts[i];
            if hidden == Some(cart.stem.as_str()) {
                continue;
            }
            let offset = target + slot as f32 - self.scroll;
            let t = offset.abs().min(1.0);
            let scale = shrink(offset);
            let alpha = (1.0 + (SIDE_ALPHA - 1.0) * t) * (1.0 - recede);
            let mould = self.mould(i);
            let (cw, ch) = cart_box(mould);
            let (w, h) = (cw as f32 * scale, ch as f32 * scale);
            let away = offset.signum() * (1.0 + offset.abs());
            let x = self.centre(canvas, slot) - w / 2.0 + away * PART * recede;
            if x + w <= 0.0 || x >= canvas.w || alpha <= 0.0 {
                continue;
            }
            let x = x + shake;
            // A favourite stands raised; on the Favorites shelf, the cart under the light does.
            let lift = if self.starred.get(i) == Some(&true) {
                LIFT
            } else if self.spotlight {
                LIFT * (1.0 - t)
            } else {
                0.0
            } * scale;
            let ground = foot_y(ch as f32) - h;
            let y = ground - lift;
            if alpha < 1.0 {
                if let Some(tex) = self.shadows[mould.index()] {
                    out.push(Draw::Tex {
                        x,
                        y,
                        w,
                        h,
                        tex,
                        alpha: recede_alpha(alpha),
                    });
                }
            }
            // A cart in its own glow casts no shadow (Roy): the shadow goes as the glow comes up.
            let glowing = if self.mark == FavoriteMark::Star {
                0.0
            } else if self.starred.get(i) == Some(&true) {
                1.0
            } else if self.spotlight && slot == 0 {
                smoothstep(self.lit) * (1.0 - t)
            } else {
                0.0
            };
            if let (Some(tex), true) = (self.drops[mould.index()], glowing < 1.0) {
                let m = DROP_MARGIN as f32 * scale;
                out.push(Draw::Tex {
                    x: x - m,
                    y: ground - m + DROP_FALL * scale + lift * 0.5,
                    w: w + 2.0 * m,
                    h: h + 2.0 * m,
                    tex,
                    alpha: DROP_ALPHA
                        * alpha
                        * dim
                        * (1.0 + lift / LIFT * 0.3)
                        * if self.light_ground { 0.55 } else { 1.0 }
                        * (1.0 - glowing),
                });
            }
            if self.clear(i) && self.face(i).is_some() {
                let board = self.backings[mould.index()];
                let gba = mould == Mould::Gba;
                // A GBA cart's board and back shell fill the cart's box, exactly where the
                // picker lifts them from; a Game Boy pak's backing fills its box.
                let Placed {
                    x: bx,
                    y: by,
                    w: bw,
                    h: bh,
                } = if gba {
                    board_in(Placed { x, y, w, h })
                } else {
                    Placed { x, y, w, h }
                };
                // Back plastic, then the board, then the front shell.
                if let Some(tex) = self.interiors.get(i).copied().flatten() {
                    out.push(Draw::Tex {
                        x: bx,
                        y: by,
                        w: bw,
                        h: bh,
                        tex,
                        alpha: alpha * dim,
                    });
                }
                if let Some(tex) = board {
                    out.push(Draw::Tex {
                        x: bx,
                        y: by,
                        w: bw,
                        h: bh,
                        tex,
                        alpha: alpha * dim,
                    });
                }
            }
            out.push(match self.face(i) {
                Some(tex) => Draw::Tex {
                    x,
                    y,
                    w,
                    h,
                    tex,
                    alpha: alpha * dim,
                },
                None => {
                    let c = label_colour(&label_text(cart));
                    Draw::Rect {
                        x,
                        y,
                        w,
                        h,
                        colour: [
                            c[0] as f32 / 255.0,
                            c[1] as f32 / 255.0,
                            c[2] as f32 / 255.0,
                            alpha * dim,
                        ],
                    }
                }
            });
            // The spotlight falls on the cart, over its face; the highlight lights the ground
            // behind it.
            if self.starred.get(i) == Some(&true) && self.mark != FavoriteMark::Star {
                if let Some(tex) = self.glow_for(i) {
                    let m = GLOW_MARGIN as f32 * scale;
                    let to = if self.mark == FavoriteMark::Highlight {
                        &mut lights
                    } else {
                        &mut *out
                    };
                    to.push(self.glow_draw(Draw::Light {
                        x: x - m,
                        y: y - m,
                        w: w + 2.0 * m,
                        h: h + 2.0 * m,
                        tex,
                        alpha: alpha * dim,
                    }));
                }
            }
            if let (FavoriteMark::Star, Some(true), Some((tex, sw, sh))) =
                (self.mark, self.starred.get(i).copied(), self.star)
            {
                let (lx, ly, lw, _) = label_rect(mould);
                out.push(Draw::Tex {
                    x: (x + (lx + lw) * w - sw - STAR_INSET).round(),
                    y: (y + ly * h + STAR_INSET).round(),
                    w: sw,
                    h: sh,
                    tex,
                    alpha: alpha * dim,
                });
            }
        }
        if self.spotlight && self.mark != FavoriteMark::Star && !self.carts.is_empty() {
            // Still, over the middle: the carts slide through the light rather than carry it.
            let i = self.index.min(self.carts.len() - 1);
            if let Some(tex) = self.glow_for(i) {
                let (cw, ch) = cart_box(self.mould(i));
                let (w, h) = (cw as f32 * CAROUSEL, ch as f32 * CAROUSEL);
                let m = GLOW_MARGIN as f32 * CAROUSEL;
                let x = canvas.w / 2.0 - w / 2.0;
                let y = foot_y(ch as f32) - h - LIFT * CAROUSEL;
                let to = if self.mark == FavoriteMark::Highlight {
                    &mut lights
                } else {
                    &mut *out
                };
                to.push(self.glow_draw(Draw::Light {
                    x: x - m,
                    y: y - m,
                    w: w + 2.0 * m,
                    h: h + 2.0 * m,
                    tex,
                    alpha: (1.0 - recede) * dim * smoothstep(self.lit),
                }));
            }
        }
        out.splice(behind..behind, lights);
    }
}

fn shrink(offset: f32) -> f32 {
    CAROUSEL * (1.0 + (SIDE_SCALE - 1.0) * offset.abs().min(1.0))
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn recede_alpha(face_alpha: f32) -> f32 {
    (face_alpha / SIDE_ALPHA).clamp(0.0, 1.0)
}
