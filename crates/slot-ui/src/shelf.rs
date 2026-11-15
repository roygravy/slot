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
/// Below 1 the slide runs a touch past the next cart and swings back, as a cart pushed along
/// with some weight would.
const DAMPING: f32 = 0.66;
/// A cart on the move hops: this high at the middle of its way from one place to the next.
const HOP: f32 = 16.0;
/// ...and leans into where it is going, this much per cart per second (radians), up to TILT_MAX.
const TILT: f32 = 0.012;
const TILT_MAX: f32 = 0.07;
/// The cart that comes to rest in the middle pops up this far, on a bouncy spring.
const POP: f32 = 9.0;
const POP_OMEGA: f32 = 15.0;
const POP_DAMPING: f32 = 0.6;
/// Standing still, the middle cart breathes: a slow sway and a small bob.
const SWAY: f32 = 0.016;
const SWAY_S: f32 = 3.4;
/// Blowing on a cart: the puff lasts PUFF_S, sheds PUFF_N soft grains and gives the cart a small
/// shove (a turn of SHOVE) that rocks out as the sway does.
const PUFF_S: f32 = 0.9;
const PUFF_N: u32 = 64;
const SHOVE: f32 = 0.05;
const PUFF_COLOUR: [f32; 3] = [0.92, 0.89, 0.83];
const BOB: f32 = 2.0;
const CALM_IN_S: f32 = 0.6;
/// The Deck's spotlight comes up over this long after a shelf change.
const SPOT_UP_S: f32 = 0.35;
/// And goes down over this long as the Deck leaves.
const SPOT_DOWN_S: f32 = 0.25;
/// How strong the spotlight's beam and its light on the cart's top are.
const SPOT_BEAM: f32 = 1.0;
/// How dark the room goes round the Deck's spotlight, and on the light wallpaper.
const SPOT_ROOM: f32 = 0.6;
const SPOT_ROOM_LIGHT: f32 = 0.35;
/// How dark the carts beside the lit one go.
const SPOT_SIDE_DARK: f32 = 0.4;
/// How dark the shadow right under the lit cart is.
const SPOT_UNDER: f32 = 1.0;
/// How much higher the Deck's middle cart is lifted on its way in and out of the spotlight.
const SPOT_HOP: f32 = 0.8;
const SPOT_EDGE: f32 = 0.9;
/// The beam on the theme's dark wallpaper, against your own.
const SPOT_DARK: f32 = 0.6;
/// The beam over your own wallpaper: added as light, and laid on as warm colour.
const SPOT_OWN: f32 = 0.35;
const SPOT_OWN_TINT: f32 = 0.7;
/// The rays in the beam: each layer comes and goes and sways over its own time.
const SPOT_RAY_S: [f32; 3] = [3.1, 4.3, 5.7];
const SPOT_SWAY_PX: f32 = 7.0;
/// The most one frame moves the shelf: a slower frame (start-up, a label, the deep sleep) does not
/// make the carts jump.
const MAX_FRAME_S: f32 = 0.05;
/// A favourite's glow breathes this deep over GLOW_PULSE_S, and flares this much brighter as
/// the middle cart pops up.
const GLOW_PULSE: f32 = 0.6;
const GLOW_PULSE_S: f32 = 2.4;
const GLOW_FLARE: f32 = 0.5;
/// Glints around a highlighted favourite: how many at once, how long each lives, how long
/// its arms reach at their brightest.
const SPARKLES: u32 = 9;
const SPARKLE_S: f32 = 1.4;
pub const SPARKLE_PX: f32 = 42.0;
/// Changing shelves: the old carts go off one after another, up or down, and the new ones
/// come in from the other side the same way. Each cart takes SWAP_S, a beat
/// of SWAP_STAGGER behind the one before; the new shelf starts SWAP_IN_AT in.
const SWAP_S: f32 = 0.18;
const SWAP_STAGGER: f32 = 0.04;
const SWAP_IN_AT: f32 = 0.1;
const SWAP_DIST: f32 = OUT_H as f32;
const SWAP_TURN: f32 = 0.04;
const SWAP_DONE: f32 = SWAP_IN_AT + SWAP_S + SLOTS as f32 * SWAP_STAGGER;
/// The shadow stays on the ground while a cart jumps: smaller and fainter the higher it goes.
const SHADOW_FADE_PX: f32 = 60.0;
const PART: f32 = 130.0;

const SLOTS: i32 = 3;

pub const REPEAT_DELAY_MS: Millis = 400;

pub const GLOW_MARGIN: u32 = 130;
/// A favourite stands this much higher than the carts beside it.
const LIFT: f32 = 10.0;
/// The Favorites light waits for the cart to come this close to standing still...
/// ...then rises over this long. It fades out quicker, as soon as the carts move.
/// How far a cart's drop shadow spreads, and how far below the cart it falls.
pub const DROP_MARGIN: u32 = 22;
pub(crate) const DROP_FALL: f32 = 10.0;
pub(crate) const DROP_ALPHA: f32 = 0.22;
pub const GLOW_GOLD: [u8; 3] = [0xff, 0xdc, 0x9a];
/// The favourite's sparkles' own warm white; the glow's colours are in its faces (cart.rs).
const GLOW_CHAMPAGNE: ([f32; 3], f32) = ([1.0, 0.80, 0.42], 1.0);
pub const GLOW_WHITE: [u8; 3] = [0xff, 0xff, 0xff];
pub const STAR_PX: u32 = 44;
const STAR_INSET: f32 = 6.0;
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
    glows: [Option<TexId>; MOULDS],
    star: Option<(TexId, f32, f32)>,
    mark: FavoriteMark,
    drops: [Option<TexId>; MOULDS],
    /// Light grounds take the glow as a tint; dark ones take it as added light.
    light_ground: bool,
    /// The player's own wallpaper is behind: the spotlight at its own strength there (Roy).
    own_ground: bool,
    ride: f32,
    vel: f32,
    /// The middle cart's pop (0 down, 1 up) and its speed; the running clock for the sway, and
    /// how far the sway has come in since the carts last stood still.
    pop: f32,
    pop_vel: f32,
    clock: f32,
    calm: f32,
    /// A shelf change under way: seconds in, the side the carts go to or come from (-1 left,
    /// 1 right), and whether they are leaving.
    swap: Option<(f32, f32, bool)>,
    held: Option<(i32, Millis, usize)>,
    /// A puff under way: seconds in, and whether there was dust to blow off.
    puff: Option<(f32, bool)>,
    puff_dot: Option<TexId>,
    /// The favourite's hot rim light per mould, close round the cart under the wide glow, and
    /// the soft four-point sparkle round it.
    rims: [Option<TexId>; MOULDS],
    sparkle: Option<TexId>,
    /// The Deck's spotlight (sh10): still over the middle, the carts slide through it. Its
    /// beam and the wash it lays on the cart, and how far it has come up: it goes out while
    /// the carts move and rises again once one stands still.
    spotlight: bool,
    spot: Option<TexId>,
    spot_rays: [Option<TexId>; 3],
    spot_room: Option<TexId>,
    spot_under: Option<TexId>,
    spot_edges: [Option<TexId>; MOULDS],
    lit: f32,
}

impl Shelf {
    pub fn new(carts: Vec<Cart>) -> Self {
        Shelf {
            moulds: carts.iter().map(|_| OnceLock::new()).collect(),
            clears: carts.iter().map(|_| OnceLock::new()).collect(),
            faces: vec![None; carts.len()],
            interiors: vec![None; carts.len()],
            starred: vec![false; carts.len()],
            glows: [None; MOULDS],
            star: None,
            mark: FavoriteMark::default(),
            drops: [None; MOULDS],
            light_ground: false,
            own_ground: false,
            carts,
            index: 0,
            pop: 0.0,
            pop_vel: 0.0,
            clock: 0.0,
            calm: 0.0,
            swap: None,
            scroll: 0.0,
            shadows: [None; MOULDS],
            backings: [None; MOULDS],
            ride: 0.0,
            vel: 0.0,
            held: None,
            puff: None,
            puff_dot: None,
            rims: [None; MOULDS],
            sparkle: None,
            spotlight: false,
            spot: None,
            spot_rays: [None; 3],
            spot_room: None,
            spot_under: None,
            spot_edges: [None; MOULDS],
            lit: 0.0,
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

    pub fn set_glow(&mut self, mould: Mould, face: TexId) {
        self.glows[mould.index()] = Some(face);
    }

    pub fn set_rim(&mut self, mould: Mould, face: TexId) {
        self.rims[mould.index()] = Some(face);
    }

    pub fn set_sparkle(&mut self, tex: TexId) {
        self.sparkle = Some(tex);
    }

    pub fn set_spotlight(&mut self, on: bool) {
        self.spotlight = on;
    }

    pub fn set_spot_faces(
        &mut self,
        beam: TexId,
        rays: [TexId; 3],
        room: TexId,
        under: TexId,
        edges: [TexId; MOULDS],
    ) {
        self.spot_room = Some(room);
        self.spot_under = Some(under);
        self.spot = Some(beam);
        self.spot_rays = rays.map(Some);
        self.spot_edges = edges.map(Some);
    }

    pub fn set_puff_dot(&mut self, tex: TexId) {
        self.puff_dot = Some(tex);
    }

    /// B on the middle cart: it takes a small shove and, with `dusty`, grains of dust lift off it.
    pub fn start_puff(&mut self, dusty: bool) {
        self.puff = Some((0.0, dusty));
    }

    pub fn stop_puff(&mut self) {
        self.puff = None;
    }

    /// Seconds into the puff, while there is one.
    pub fn puff_age(&self) -> Option<f32> {
        self.puff.map(|(t, _)| t)
    }

    pub fn set_star(&mut self, face: TexId, w: u32, h: u32) {
        self.star = Some((face, w as f32, h as f32));
    }

    pub fn set_drop(&mut self, mould: Mould, face: TexId) {
        self.drops[mould.index()] = Some(face);
    }

    /// The drop shadow a cart of `mould` casts on the shelf.
    pub fn drop_face(&self, mould: Mould) -> Option<TexId> {
        self.drops[mould.index()]
    }

    pub fn set_own_ground(&mut self, own: bool) {
        self.own_ground = own;
    }

    pub fn set_light_ground(&mut self, light: bool) {
        self.light_ground = light;
    }

    fn glow_for(&self, i: usize) -> Option<TexId> {
        self.glows[self.mould(i).index()]
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

    /// This shelf comes on, its carts from the side `from` (-1 above, 1 below).
    pub fn arrive(&mut self, from: f32) {
        self.swap = Some((0.0, from, false));
        self.lit = 0.0;
        self.pop = 0.0;
        self.pop_vel = 0.0;
        self.calm = 0.0;
    }

    /// This shelf goes off, its carts to the side `to`.
    pub fn depart(&mut self, to: f32) {
        self.swap = Some((0.0, to, true));
    }

    /// True while this shelf's carts are still on their way off.
    pub fn departing(&self) -> bool {
        matches!(self.swap, Some((_, _, true)))
    }

    /// Runs a shelf change on, for a shelf that is otherwise standing still.
    pub fn run_swap(&mut self, dt: f32) {
        if let Some((t, side, leaving)) = self.swap {
            let t = t + dt.min(MAX_FRAME_S);
            self.swap = (t < SWAP_DONE).then_some((t, side, leaving));
            // The Deck's spotlight goes down with the shelf as it leaves, not all at once at
            // the end (Roy 2026-11-12).
            if leaving {
                self.lit = (self.lit - dt.min(MAX_FRAME_S) / SPOT_DOWN_S).max(0.0);
            }
        }
    }

    /// How far up and how turned a cart `slot` from the middle is in a shelf change: the old
    /// shelf goes up and away (or down), the new one rises from the other side (Roy: vertical),
    /// the outer carts first and the middle one last, each tipped a little outward.
    fn swap_pose(swap: Option<(f32, f32, bool)>, slot: i32) -> (f32, f32) {
        let Some((t, side, leaving)) = swap else {
            return (0.0, 0.0);
        };
        let order = (SLOTS - slot.abs()) as f32;
        let start = order * SWAP_STAGGER + if leaving { 0.0 } else { SWAP_IN_AT };
        let u = ((t - start) / SWAP_S).clamp(0.0, 1.0);
        let away = if leaving { u * u } else { (1.0 - u) * (1.0 - u) };
        let arc = (std::f32::consts::PI * u).sin();
        (
            -side * SWAP_DIST * away,
            slot.signum() as f32 * SWAP_TURN * arc,
        )
    }

    pub fn update(&mut self, dt: f32) {
        self.run_swap(dt);
        // A long frame (start-up, a label being drawn, waking from the deep sleep) moves the
        // carts no further than a short one: the motion waits a moment instead of jumping, and
        // the springs, stepped at most 1/60 s at a time, never shoot the carts off screen.
        let dt = dt.min(MAX_FRAME_S);
        let steps = (dt * 60.0).ceil().clamp(1.0, 60.0);
        let h = dt / steps;
        let target = self.scroll_target();
        let mut resting = false;
        for _ in 0..steps as u32 {
            let accel = -2.0 * DAMPING * OMEGA * self.vel - OMEGA * OMEGA * (self.scroll - target);
            self.vel += accel * h;
            self.scroll += self.vel * h;
            resting = (self.scroll - target).abs() < 0.12 && self.vel.abs() < 1.5;
            let goal = if resting { 1.0 } else { 0.0 };
            let pop_accel = -2.0 * POP_DAMPING * POP_OMEGA * self.pop_vel
                - POP_OMEGA * POP_OMEGA * (self.pop - goal);
            self.pop_vel += pop_accel * h;
            self.pop += self.pop_vel * h;
        }
        if !(self.scroll.is_finite() && self.vel.is_finite()) {
            (self.scroll, self.vel) = (target, 0.0);
        }
        if !(self.pop.is_finite() && self.pop_vel.is_finite()) {
            (self.pop, self.pop_vel) = (1.0, 0.0);
        }
        if let Some((t, dusty)) = self.puff {
            self.puff = (t + dt < PUFF_S).then_some((t + dt, dusty));
        }
        self.clock = (self.clock + dt) % (SWAY_S * 100.0);
        // The spotlight stays on while the carts go by (Roy); it only comes up anew after a
        // shelf change.
        if self.swap.is_none() {
            self.lit = (self.lit + dt / SPOT_UP_S).min(1.0);
        }
        self.calm = if resting {
            (self.calm + dt / CALM_IN_S).min(1.0)
        } else {
            0.0
        };
    }

    /// On the move a cart hops and leans into its way; once in the middle it pops up and then
    /// breathes: how much higher it stands and how far it is turned, `offset` from the middle.
    fn play(&self, offset: f32, scale: f32) -> (f32, f32) {
        let middle = 1.0 - offset.abs().min(1.0);
        let phase = std::f32::consts::TAU * self.clock / SWAY_S;
        let calm = smoothstep(self.calm) * middle;
        // On the Deck the cart in the spotlight stands still on the floor; it is picked up and
        // set down in a higher arc as the carts pass, and the carts beside it hover and sway
        // in the dark, each on its own time (Roy 2026-11-14).
        let lit = self.spot_lit();
        let stand = lit * middle;
        let side = lit * (1.0 - middle);
        let own = offset * 1.9;
        // The middle one lands and stays down: no second little hop as the slide swings past
        // and back (Roy 2026-11-14).
        let land = 1.0 - stand * (1.0 - smoothstep((offset.abs() / 0.25).min(1.0)));
        let lift = (HOP * (1.0 + SPOT_HOP * lit) * (std::f32::consts::PI * offset).sin().abs() * land
            + POP * self.pop * middle * (1.0 - stand)
            + BOB * (1.0 - (2.0 * phase).cos()) * 0.5 * calm * (1.0 - stand)
            + BOB * (1.0 - (2.0 * phase + own).cos()) * 0.5 * side)
            * scale;
        let shove = self.puff.map_or(0.0, |(t, _)| {
            SHOVE * (std::f32::consts::TAU * t / 0.5).sin() * (-t * 4.5).exp() * middle
        });
        let turn = (-self.vel * TILT).clamp(-TILT_MAX, TILT_MAX) * (1.0 - stand)
            + SWAY * phase.sin() * calm * (1.0 - stand)
            + SWAY * (phase + own).sin() * side
            + shove;
        (lift, turn)
    }

    /// How far the Deck's spotlight is up (0 off, 1 full).
    fn spot_lit(&self) -> f32 {
        if self.spotlight && self.spot.is_some() && !self.carts.is_empty() {
            smoothstep(self.lit)
        } else {
            0.0
        }
    }

    /// The grains of a puff: they let go from the top of the cart to the bottom, drift up and to
    /// the right as if blown, and fade.
    fn puff_grains(&self, (x, y, w, h): (f32, f32, f32, f32), alpha: f32, out: &mut Vec<Draw>) {
        let (Some((age, true)), Some(tex)) = (self.puff, self.puff_dot) else {
            return;
        };
        for n in 0..PUFF_N {
            let seed = n * 97 + 13;
            let (gx, gy) = (hash(seed), hash(seed + 1));
            let delay = gy * 0.30 + hash(seed + 2) * 0.15;
            let t = ((age / PUFF_S - delay) / 0.65).clamp(0.0, 1.0);
            if t <= 0.0 || t >= 1.0 {
                continue;
            }
            let dx = t * (18.0 + 72.0 * hash(seed + 3));
            let dy = -t * (6.0 + 40.0 * hash(seed + 4)) + t * t * 18.0 * hash(seed + 5);
            let size = (4.0 + 6.0 * hash(seed + 6)) * (0.7 + 0.6 * t);
            let a = 0.8 * (1.0 - t).powf(1.3) * alpha;
            out.push(Draw::Tint {
                x: x + gx * w + dx - size / 2.0,
                y: y + gy * h + dy - size / 2.0,
                w: size,
                h: size,
                tex,
                colour: [PUFF_COLOUR[0], PUFF_COLOUR[1], PUFF_COLOUR[2], a],
            });
        }
    }

    /// A favourite in the highlight sparkles (Roy, sh9: as the target picture): soft
    /// four-point stars with long thin rays come and go round it, each in its own spot and
    /// time, a little way off its edges.
    fn sparkle(&self, i: usize, (x, y, w, h): (f32, f32, f32, f32), alpha: f32, out: &mut Vec<Draw>) {
        let Some(tex) = self.sparkle else { return };
        // The sparkle's face carries its own colours; on a light ground it is laid on gold.
        let colour = if self.light_ground { GLOW_CHAMPAGNE.0 } else { [1.0; 3] };
        // Two big ones sit on the cart's own corners, as light catches there (the target
        // picture): top left and bottom right, slowly swelling, never quite out.
        for (n, (fx, fy, big)) in [(0.045, 0.035, 1.9), (0.965, 0.90, 1.5)].into_iter().enumerate() {
            let phase = std::f32::consts::TAU * self.clock / 2.6 + n as f32 * 2.2 + i as f32;
            let k = 0.62 + 0.38 * phase.sin();
            let size = SPARKLE_PX * big * (0.75 + 0.25 * k);
            out.push(Draw::Tint {
                x: x + fx * w - size / 2.0,
                y: y + fy * h - size / 2.0,
                w: size,
                h: size,
                tex,
                colour: [1.0, 1.0, 1.0, alpha * k],
            });
        }
        for n in 0..SPARKLES {
            // Each glint lives SPARKLE_S, its next life somewhere else round the cart.
            let offset = n as f32 / SPARKLES as f32 + hash(i as u32 * 7 + n) * 0.5;
            let age = self.clock / SPARKLE_S + offset;
            let life = age.floor() as u32;
            let u = age.fract();
            let k = (std::f32::consts::PI * u).sin().powi(2);
            if k < 0.02 {
                continue;
            }
            let seed = i as u32 * 131 + n * 17 + life * 7919;
            // A spot round the cart's outline, off it by a little to a lot; most above it,
            // where the light rises.
            let along = hash(seed) * 2.0 * (w + h);
            let out_by = 2.0 + 22.0 * hash(seed + 1).powf(1.5);
            let (sx, sy) = if along < w {
                (x + along, y - out_by)
            } else if along < w + h {
                (x + w + out_by, y + along - w)
            } else if along < 2.0 * w + h {
                (x + w - (along - w - h), y + h + out_by * 0.4)
            } else {
                (x - out_by, y + h - (along - 2.0 * w - h))
            };
            let size = SPARKLE_PX * (0.45 + 0.55 * hash(seed + 2)) * (0.55 + 0.45 * k);
            out.push(Draw::Tint {
                x: sx - size / 2.0,
                y: sy - size / 2.0,
                w: size,
                h: size,
                tex,
                colour: [colour[0], colour[1], colour[2], alpha * k],
            });
        }
    }

    /// A favourite's light lives: it breathes slowly, each cart a little out of step with its
    /// neighbours, and flares up as the cart in the middle pops up.
    fn glow_pulse(&self, slot: i32) -> f32 {
        let phase = std::f32::consts::TAU * self.clock / GLOW_PULSE_S + slot as f32 * 1.3;
        let flare = if slot == 0 {
            GLOW_FLARE * (self.pop_vel / POP_OMEGA).max(0.0).min(1.0)
        } else {
            0.0
        };
        1.0 - GLOW_PULSE * 0.5 * (1.0 - phase.sin()) + flare
    }

    /// The selected cart's own lift and turn just now, for the insert and the studio to pick it
    /// up from where it stands.
    pub fn selected_play(&self) -> (f32, f32) {
        let offset = self.scroll_target() - self.scroll;
        self.play(offset, shrink(offset))
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
        self.draw_row_swapped(canvas, hidden, shake, recede, dim, self.swap, out);
    }

    /// The row coming down from above, `come` of the way (0 to 1), its carts one after another
    /// as in a shelf change: for the Game Boy shelf that comes down onto a Super Game Boy (Roy).
    pub fn draw_row_coming_down(&self, canvas: Canvas, come: f32, out: &mut Vec<Draw>) {
        let t = come.clamp(0.0, 1.0) * (SWAP_DONE - SWAP_IN_AT) + SWAP_IN_AT;
        self.draw_row_swapped(canvas, None, 0.0, 0.0, 1.0, Some((t, -1.0, false)), out);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_row_swapped(
        &self,
        canvas: Canvas,
        hidden: Option<&str>,
        shake: f32,
        recede: f32,
        dim: f32,
        swap: Option<(f32, f32, bool)>,
        out: &mut Vec<Draw>,
    ) {
        let recede = recede.clamp(0.0, 1.0);
        let dim = dim.clamp(0.0, 1.0);
        let target = self.scroll_target();
        // The lights go behind every cart: gathered on the way, laid down before the first.
        let behind = out.len();
        let mut lights = Vec::new();
        let spot = (self.spotlight && !self.carts.is_empty())
            .then_some(self.spot)
            .flatten()
            .map(|s| (s, smoothstep(self.lit) * (1.0 - recede) * dim))
            .filter(|(_, a)| *a > 0.0);
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
            let (swap_lift, swap_turn) = Self::swap_pose(swap, slot);
            let x = x + shake;
            // A favourite stands raised.
            let lift = if self.starred.get(i) == Some(&true) {
                LIFT
            } else {
                0.0
            } * scale;
            let (play, turn) = self.play(offset, scale);
            let turn = turn * (1.0 - recede) + swap_turn;
            let ground = foot_y(ch as f32) - h;
            // A shelf change carries the cart and its shadow off together.
            let ground = ground - swap_lift;
            let y = ground - lift - play;
            let first = out.len();
            // Only a see-through side cart gets the dark backing; a cart fading out as the
            // studio opens or closes must not, or it darkens into a grey ghost (Roy 14-nov).
            let side = 1.0 + (SIDE_ALPHA - 1.0) * t;
            if side < 1.0 {
                if let Some(tex) = self.shadows[mould.index()] {
                    out.push(Draw::Tex {
                        x,
                        y,
                        w,
                        h,
                        tex,
                        alpha: recede_alpha(side) * (1.0 - recede),
                    });
                }
            }
            // A cart in its own glow casts no shadow (Roy): the shadow goes as the glow comes up.
            let glowing = if self.mark != FavoriteMark::Highlight {
                0.0
            } else if self.starred.get(i) == Some(&true) {
                1.0
            } else {
                0.0
            };
            if let (Some(tex), true) = (self.drops[mould.index()], glowing < 1.0) {
                let m = DROP_MARGIN as f32 * scale;
                let air = (play / (SHADOW_FADE_PX * scale)).clamp(0.0, 1.0);
                let (dw, dh) = (w * (1.0 - 0.12 * air), h * (1.0 - 0.06 * air));
                out.push(Draw::Tex {
                    x: x + (w - dw) / 2.0 - m,
                    y: ground + (h - dh) - m + DROP_FALL * scale + lift * 0.5,
                    w: dw + 2.0 * m,
                    h: dh + 2.0 * m,
                    tex,
                    alpha: DROP_ALPHA
                        * alpha
                        * dim
                        * (1.0 + lift / LIFT * 0.3)
                        * (1.0 - 0.5 * air)
                        * if self.light_ground { 0.55 } else { 1.0 }
                        * (1.0 - glowing),
                });
            }
            // The spotlight from straight above throws a short, dark shadow right under the
            // cart, in the middle of its pool (Roy 2026-11-14, after his picture).
            if let (Some((_, a)), Some(tex)) = (spot, self.spot_under) {
                let under = 1.0 - offset.abs().min(1.0);
                let air = (play / (SHADOW_FADE_PX * scale)).clamp(0.0, 1.0);
                // A favourite in its own glow casts none, as on the other shelves (Roy 2026-11-14).
                let alpha = a * under * SPOT_UNDER * (1.0 - 0.6 * air) * dim * (1.0 - glowing);
                if alpha > 0.0 {
                    let (sw, sh) = (w * 1.0, w * 0.045);
                    out.push(Draw::Tint {
                        x: x + (w - sw) / 2.0,
                        y: ground + h - sh * 0.35,
                        w: sw,
                        h: sh,
                        tex,
                        colour: [0.0, 0.0, 0.0, alpha],
                    });
                }
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
            // The highlight lights the ground behind the cart.
            if self.starred.get(i) == Some(&true) && self.mark == FavoriteMark::Highlight {
                let pulse = self.glow_pulse(slot);
                // The glow hugs the cart's outline, so it turns with the cart.
                let place = |tex, m: f32, a: f32| {
                    let (x, y, w, h) = (x - m, y - m, w + 2.0 * m, h + 2.0 * m);
                    let alpha = (alpha * dim * a).min(1.0);
                    if turn.abs() > 1e-4 {
                        Draw::Turned { x, y, w, h, tex, alpha, turn }
                    } else {
                        Draw::Tex { x, y, w, h, tex, alpha }
                    }
                };
                // w is the cart's box at this scale; the faces are drawn in the cart's pixels.
                let px = w / cart_box(mould).0 as f32;
                if let Some(tex) = self.glow_for(i) {
                    lights.push(place(tex, crate::cart::FAV_GLOW_MARGIN as f32 * px, pulse));
                }
                if let Some(tex) = self.rims[mould.index()] {
                    out.push(place(tex, crate::cart::FAV_EDGE_MARGIN as f32 * px, 0.6 + 0.4 * pulse));
                }
            }
            if slot == 0 {
                self.puff_grains((x, y, w, h), alpha * dim, out);
            }
            // Where the spotlight lands: on the cart in the middle, handed from cart to cart as
            // they pass under it. The beam is drawn for a foot at SPOT_FOOT and stretched down
            // to this cart's, so a tall pak and a wide cart each get it to their own ground.
            let under = 1.0 - offset.abs().min(1.0);
            if let (Some((beam, a)), true) = (spot, under > 0.0) {
                let k = foot_y(ch as f32) / crate::icon::SPOT_FOOT;
                let at = Placed {
                    x: (canvas.w - 640.0) / 2.0,
                    y: 0.0,
                    w: 640.0,
                    h: 480.0 * k,
                };
                self.spot_draw(&mut lights, at, beam, a * under * SPOT_BEAM);
                // Rays move in it (Roy 2026-11-14, as a stage spotlight): three layers of thin
                // rays fading in and out and swaying a little, each on its own time.
                for (j, tex) in self.spot_rays.iter().enumerate() {
                    let Some(tex) = *tex else { continue };
                    let tau = std::f32::consts::TAU;
                    let t = self.clock / SPOT_RAY_S[j];
                    let come = 0.5 + 0.5 * (tau * t + j as f32 * 2.1).sin();
                    let sway = SPOT_SWAY_PX * (tau * t * 0.6 + j as f32 * 1.7).sin();
                    let ray_at = Placed { x: at.x + sway, ..at };
                    self.spot_draw(&mut lights, ray_at, tex, a * under * (0.25 + 0.75 * come));
                }
                if let Some(tex) = self.spot_edges[mould.index()] {
                    let m = crate::cart::SPOT_EDGE_MARGIN as f32 * w / cw as f32;
                    let (ex, ey, ew, eh) = (x - m, y - m, w + 2.0 * m, h + 2.0 * m);
                    let alpha = a * under * SPOT_EDGE;
                    // Laid on as colour over the cart's face, as the favourite's rim is, and
                    // turned with the cart.
                    out.push(if turn.abs() > 1e-4 {
                        Draw::Turned { x: ex, y: ey, w: ew, h: eh, tex, alpha, turn }
                    } else {
                        Draw::Tex { x: ex, y: ey, w: ew, h: eh, tex, alpha }
                    });
                }
            }
            if self.mark == FavoriteMark::Highlight && self.starred.get(i) == Some(&true) {
                self.sparkle(i, (x, y, w, h), alpha * dim, out);
            }
            if let (FavoriteMark::Star, Some(true), Some((tex, sw, sh))) =
                (self.mark, self.starred.get(i).copied(), self.star)
            {
                let (lx, ly, lw, _) = label_rect(mould);
                // A sticker on the cart (Roy, sh12): it turns with the cart round the cart's
                // middle, not round its own, so it never slides on the label. Unrounded while
                // the cart turns, so it does not step against it.
                // ...and is as much smaller as the cart is, on the carts at the sides.
                let k = scale / CAROUSEL;
                let (sw, sh, inset) = (sw * k, sh * k, STAR_INSET * k);
                let (mut sx, mut sy) = (x + (lx + lw) * w - sw - inset, y + ly * h + inset);
                if turn.abs() > 1e-4 {
                    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
                    let (ox, oy) = (sx + sw / 2.0 - cx, sy + sh / 2.0 - cy);
                    let (c, s) = (turn.cos(), turn.sin());
                    sx = cx + c * ox - s * oy - sw / 2.0;
                    sy = cy + s * ox + c * oy - sh / 2.0;
                } else {
                    (sx, sy) = (sx.round(), sy.round());
                }
                out.push(Draw::Tex {
                    x: sx,
                    y: sy,
                    w: sw,
                    h: sh,
                    tex,
                    alpha: alpha * dim,
                });
            }
            // Only the cart in the spotlight stands in the light (Roy 2026-11-14): the others
            // are in the dark room with the wallpaper.
            if let (Some((_, a)), Some(tex)) = (spot, self.shadows[mould.index()]) {
                let under = 1.0 - offset.abs().min(1.0);
                let dark = a * (1.0 - under) * SPOT_SIDE_DARK;
                if dark > 0.0 {
                    out.push(Draw::Tex {
                        x,
                        y,
                        w,
                        h,
                        tex,
                        alpha: dark * alpha,
                    });
                }
            }
            if turn.abs() > 1e-4 {
                for d in &mut out[first..] {
                    if let Draw::Tex {
                        x,
                        y,
                        w,
                        h,
                        tex,
                        alpha,
                    } = *d
                    {
                        *d = Draw::Turned {
                            x,
                            y,
                            w,
                            h,
                            tex,
                            alpha,
                            turn,
                        };
                    }
                }
            }
        }
        // The room goes dark round the spotlight, as when the house lights go out (Roy
        // 2026-11-14); it comes up with the beam.
        let mut behind = behind;
        if let (Some((_, a)), Some(tex)) = (spot, self.spot_room) {
            let alpha = a * self.room_dark();
            out.insert(behind, Draw::Tex { x: 0.0, y: 0.0, w: canvas.w, h: canvas.h, tex, alpha });
            behind += 1;
        }
        out.splice(behind..behind, lights);
    }
}

impl Shelf {
    /// How dark the room goes round the Deck's spotlight.
    fn room_dark(&self) -> f32 {
        if self.light_ground {
            SPOT_ROOM_LIGHT
        } else {
            SPOT_ROOM
        }
    }

    /// Light added on a dark ground; on a light one, where added light would vanish, laid on
    /// as a warm tint instead.
    fn spot_draw(&self, out: &mut Vec<Draw>, at: Placed, tex: TexId, alpha: f32) {
        let Placed { x, y, w, h } = at;
        // Full on the light wallpaper, where it was hard to see; softer on the dark one.
        if self.light_ground {
            out.push(Draw::Tex { x, y, w, h, tex, alpha: alpha.min(1.0) });
        } else if self.own_ground {
            // On your own wallpaper half laid on as warm colour, half added as light: the
            // ground stays visible under it and lights up, and it stays warm instead of
            // turning a blue wallpaper white (Roy 2026-11-14).
            out.push(Draw::Tex { x, y, w, h, tex, alpha: (alpha * SPOT_OWN_TINT).min(1.0) });
            out.push(Draw::Light { x, y, w, h, tex, alpha: alpha * SPOT_OWN });
        } else {
            out.push(Draw::Light { x, y, w, h, tex, alpha: alpha * SPOT_DARK });
        }
    }
}

fn shrink(offset: f32) -> f32 {
    CAROUSEL * (1.0 + (SIDE_SCALE - 1.0) * offset.abs().min(1.0))
}

/// A steady pseudo-random number 0..1 for a seed, so a glint keeps its spot through its life.
fn hash(seed: u32) -> f32 {
    let mut v = seed.wrapping_mul(0x9e37_79b9) ^ 0x85eb_ca6b;
    v ^= v >> 15;
    v = v.wrapping_mul(0x2c1b_3c6d);
    v ^= v >> 12;
    (v & 0xffff) as f32 / 65535.0
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn recede_alpha(face_alpha: f32) -> f32 {
    (face_alpha / SIDE_ALPHA).clamp(0.0, 1.0)
}
