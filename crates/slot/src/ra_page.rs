//! The achievements screens, drawn in software into one 640x480 picture.
//! Monochrome like the other menus; only the badges, which are the content, carry colour.
use std::collections::HashMap;
use std::sync::Mutex;

use fontdue::Font;
use slot_achievements::{Achievement, Info};
use slot_ui::CartFace;

pub const W: u32 = 640;
pub const H: u32 = 480;
const COLS: usize = 4;

/// Badge pictures fetched from RetroAchievements: name -> 64x64 rgba.
pub static BADGES: Mutex<Option<HashMap<String, Vec<u8>>>> = Mutex::new(None);

/// Players who earned each achievement (id -> count), when the user gave a web API key.
pub static COUNTS: Mutex<Option<HashMap<u32, u32>>> = Mutex::new(None);

/// "12,345".
fn thousands(n: u32) -> String {
    let d = n.to_string();
    let mut out = String::new();
    for (i, ch) in d.chars().enumerate() {
        if i > 0 && (d.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// The user's picture: side length and rgba.
pub static AVATAR: Mutex<Option<(usize, Vec<u8>)>> = Mutex::new(None);

pub struct View {
    /// 0 is the first page (the header and the latest four), then twelve to a page.
    pub page: usize,
    /// The chosen card on that page.
    pub sel: usize,
    pub detail: bool,
}

struct Canvas {
    px: Vec<u8>,
}

impl Canvas {
    fn new() -> Self {
        Canvas {
            px: vec![0; (W * H * 4) as usize],
        }
    }

    fn blend(&mut self, x: i32, y: i32, c: [u8; 3], a: f32) {
        if x < 0 || y < 0 || x >= W as i32 || y >= H as i32 || a <= 0.0 {
            return;
        }
        let i = ((y as u32 * W + x as u32) * 4) as usize;
        let da = self.px[i + 3] as f32 / 255.0;
        let oa = a + da * (1.0 - a);
        for k in 0..3 {
            let dc = self.px[i + k] as f32;
            self.px[i + k] = ((c[k] as f32 * a + dc * da * (1.0 - a)) / oa.max(0.001)) as u8;
        }
        self.px[i + 3] = (oa * 255.0) as u8;
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: [u8; 3]) {
        for yy in y.round() as i32..(y + h).round() as i32 {
            for xx in x.round() as i32..(x + w).round() as i32 {
                self.blend(xx, yy, c, 1.0);
            }
        }
    }

    /// A smooth disc (or ring when `ring` > 0), 4x4 supersampled.
    fn disc(&mut self, cx: f32, cy: f32, r: f32, ring: f32, c: [u8; 3]) {
        for y in (cy - r - 1.0).floor() as i32..(cy + r + 1.0).ceil() as i32 {
            for x in (cx - r - 1.0).floor() as i32..(cx + r + 1.0).ceil() as i32 {
                let mut n = 0;
                for sy in 0..4 {
                    for sx in 0..4 {
                        let px = x as f32 + (sx as f32 + 0.5) / 4.0 - cx;
                        let py = y as f32 + (sy as f32 + 0.5) / 4.0 - cy;
                        let d = (px * px + py * py).sqrt();
                        if d <= r && (ring <= 0.0 || d >= r - ring) {
                            n += 1;
                        }
                    }
                }
                self.blend(x, y, c, n as f32 / 16.0);
            }
        }
    }

    /// The 64x64 badge scaled to a disc of radius `r`.
    fn badge(&mut self, cx: f32, cy: f32, r: f32, rgba: &[u8], _dim: bool) {
        self.picture_in(cx, cy, r, rgba, 64, BADGE_INSET);
    }

    /// A square picture of `side` pixels scaled into a disc of radius `r`.
    fn picture(&mut self, cx: f32, cy: f32, r: f32, rgba: &[u8], side: usize) {
        self.picture_in(cx, cy, r, rgba, side, 0.0);
    }

    /// Like `picture`, but only the part `inset` (0..0.5 of the side) inside the edges fills
    /// the disc, so a badge's pale border never shows in the circle.
    fn picture_in(&mut self, cx: f32, cy: f32, r: f32, rgba: &[u8], side: usize, inset: f32) {
        let d = r * 2.0;
        for y in 0..d.ceil() as i32 {
            for x in 0..d.ceil() as i32 {
                let (px, py) = (x as f32 + 0.5 - r, y as f32 + 0.5 - r);
                let dist = (px * px + py * py).sqrt();
                let edge = (r - dist + 0.5).clamp(0.0, 1.0);
                if edge <= 0.0 {
                    continue;
                }
                // Every source pixel under this one, averaged: a big picture scaled down
                // stays smooth instead of picking one pixel in many (Roy, ra8).
                let u = |v: f32| (inset + (v / d) * (1.0 - 2.0 * inset)) * side as f32;
                let span = |v: i32| {
                    let a = (u(v as f32).floor().max(0.0) as usize).min(side - 1);
                    let b = (u(v as f32 + 1.0).ceil() as usize).clamp(a + 1, side);
                    a..b
                };
                let (mut sum, mut n) = ([0.0f32; 4], 0.0f32);
                for sy in span(y) {
                    for sx in span(x) {
                        let i = (sy * side + sx) * 4;
                        if i + 3 >= rgba.len() {
                            continue;
                        }
                        let a = rgba[i + 3] as f32;
                        for k in 0..3 {
                            sum[k] += rgba[i + k] as f32 * a;
                        }
                        sum[3] += a;
                        n += 1.0;
                    }
                }
                if n == 0.0 || sum[3] <= 0.0 {
                    continue;
                }
                let c = [0, 1, 2].map(|k| (sum[k] / sum[3]).round() as u8);
                self.blend(
                    (cx - r) as i32 + x,
                    (cy - r) as i32 + y,
                    c,
                    edge * sum[3] / n / 255.0,
                );
            }
        }
    }

    fn text_width(font: &Font, s: &str, px: f32) -> f32 {
        s.chars().map(|c| font.metrics(c, px).advance_width).sum()
    }

    /// Letter-spaced width, as the concept sets its capitals.
    fn tracked_width(font: &Font, s: &str, px: f32, tr: f32) -> f32 {
        let n = s.chars().count();
        Self::text_width(font, s, px) + px * tr * n.saturating_sub(1) as f32
    }

    /// `s` with its top at `y` (the line box's top, as the concept's mockups place text),
    /// left edge `x`; returns its width.
    fn put(&mut self, font: &Font, s: &str, x: f32, y: f32, px: f32, tr: f32, c: [u8; 3]) -> f32 {
        let ascent = font
            .horizontal_line_metrics(px)
            .map_or(px * 0.9, |m| m.ascent);
        let base = y + ascent;
        let mut pen = x;
        for ch in s.chars() {
            let (m, bm) = font.rasterize(ch, px);
            let gx = (pen + m.xmin as f32).round() as i32;
            let gy = (base - m.height as f32 - m.ymin as f32).round() as i32;
            for yy in 0..m.height {
                for xx in 0..m.width {
                    let a = bm[yy * m.width + xx] as f32 / 255.0;
                    self.blend(gx + xx as i32, gy + yy as i32, c, a);
                }
            }
            pen += m.advance_width + px * tr;
        }
        pen - x - px * tr
    }

    fn ascent(font: &Font, px: f32) -> f32 {
        font.horizontal_line_metrics(px)
            .map_or(px * 0.9, |m| m.ascent)
    }

    /// The height of the capitals: what the eye measures when it lines text up.
    fn cap(font: &Font, px: f32) -> f32 {
        font.metrics('H', px).height as f32
    }

    /// Where the ink of a run starts after its pen position, and how wide the ink is.
    fn ink(font: &Font, s: &str, px: f32, tr: f32) -> (f32, f32) {
        let (Some(first), Some(last)) = (s.chars().next(), s.chars().last()) else {
            return (0.0, 0.0);
        };
        let lb = font.metrics(first, px).xmin as f32;
        let lm = font.metrics(last, px);
        let rb = lm.advance_width - (lm.xmin as f32 + lm.width as f32);
        (lb, Self::tracked_width(font, s, px, tr) - lb - rb)
    }

    /// Text placed by its ink: the capitals' top on `y`, the ink's left edge, centre or right
    /// edge on `x` ('l', 'c', 'r'). Returns the ink width.
    fn line(
        &mut self,
        font: &Font,
        s: &str,
        x: f32,
        y: f32,
        px: f32,
        tr: f32,
        c: [u8; 3],
        a: char,
    ) -> f32 {
        let (lb, w) = Self::ink(font, s, px, tr);
        let x = match a {
            'c' => x - w / 2.0,
            'r' => x - w,
            _ => x,
        };
        self.put(
            font,
            s,
            x - lb,
            y - (Self::ascent(font, px) - Self::cap(font, px)),
            px,
            tr,
            c,
        );
        w
    }

    /// `line` in capitals.
    fn caps(
        &mut self,
        font: &Font,
        s: &str,
        x: f32,
        y: f32,
        px: f32,
        tr: f32,
        c: [u8; 3],
        a: char,
    ) -> f32 {
        self.line(font, &s.to_uppercase(), x, y, px, tr, c, a)
    }

    /// Capitals with their ink centred on (`cx`, `cy`) both ways.
    fn mid(&mut self, font: &Font, s: &str, cx: f32, cy: f32, px: f32, tr: f32, c: [u8; 3]) {
        self.caps(font, s, cx, cy - Self::cap(font, px) / 2.0, px, tr, c, 'c');
    }

    /// Let what lies under a box show through: its pixels keep `a` of their cover.
    fn fade(&mut self, x: f32, y: f32, w: f32, h: f32, a: f32) {
        for yy in (y.round() as i32).max(0)..((y + h).round() as i32).min(H as i32) {
            for xx in (x.round() as i32).max(0)..((x + w).round() as i32).min(W as i32) {
                let i = ((yy as u32 * W + xx as u32) * 4 + 3) as usize;
                self.px[i] = (self.px[i] as f32 * a).round() as u8;
            }
        }
    }

    /// A share bar: round ends, the earned part in `ink`, the rest `ink` see-through
    /// (Roy 2026-11-12, ui2).
    fn bar(&mut self, x: f32, y: f32, w: f32, h: f32, share: f32, ink: [u8; 3]) {
        self.pill_alpha(x, y, w, h, ink, BAR_TRACK);
        let fill = (w * share.clamp(0.0, 1.0)).round();
        if fill > 0.0 {
            self.pill_alpha(x, y, fill.max(h), h, ink, 1.0);
        }
    }

    /// A pill at `alpha`, 4x4 supersampled, blended once per pixel.
    fn pill_alpha(&mut self, x: f32, y: f32, w: f32, h: f32, c: [u8; 3], alpha: f32) {
        let r = h / 2.0;
        let (l, rr) = (x + r, x + w - r);
        for py in y.floor() as i32..(y + h).ceil() as i32 {
            for px in x.floor() as i32..(x + w).ceil() as i32 {
                let mut n = 0;
                for sy in 0..4 {
                    for sx in 0..4 {
                        let fx = px as f32 + (sx as f32 + 0.5) / 4.0;
                        let fy = py as f32 + (sy as f32 + 0.5) / 4.0 - (y + r);
                        let dx = (l - fx).max(fx - rr).max(0.0);
                        if dx * dx + fy * fy <= r * r {
                            n += 1;
                        }
                    }
                }
                if n > 0 {
                    self.blend(px, py, c, alpha * n as f32 / 16.0);
                }
            }
        }
    }

    /// A pill: a bar with round ends.
    fn pill(&mut self, x: f32, y: f32, w: f32, h: f32, c: [u8; 3]) {
        let r = h / 2.0;
        self.rect(x + r, y, w - h, h, c);
        self.disc(x + r, y + r, r, 0.0, c);
        self.disc(x + w - r, y + r, r, 0.0, c);
    }

    /// Wrap capitals into at most `max` lines of width `w`, the last cut with an ellipsis. A
    /// word too long for a line may also break after a hyphen in it ("BATMAN-" over
    /// "UFACTURING", Roy 2026-11-13).
    fn wrap(font: &Font, s: &str, px: f32, tr: f32, w: f32, max: usize, up: bool) -> Vec<String> {
        let s = if up { s.to_uppercase() } else { s.to_string() };
        let mut lines: Vec<String> = Vec::new();
        let mut cur = String::new();
        // each piece, and whether a space goes before it (none inside a hyphenated word)
        let pieces = s.split_whitespace().flat_map(|word| {
            word.split_inclusive('-')
                .enumerate()
                .map(|(i, piece)| (piece.to_string(), i == 0))
                .collect::<Vec<_>>()
        });
        for (piece, spaced) in pieces {
            let t = match (cur.is_empty(), spaced) {
                (true, _) => piece.clone(),
                (false, true) => format!("{cur} {piece}"),
                (false, false) => format!("{cur}{piece}"),
            };
            if Self::tracked_width(font, &t, px, tr) <= w || cur.is_empty() {
                cur = t;
            } else {
                lines.push(std::mem::replace(&mut cur, piece));
            }
        }
        if !cur.is_empty() {
            lines.push(cur);
        }
        if lines.len() > max {
            lines.truncate(max);
            if let Some(l) = lines.last_mut() {
                l.push('…');
            }
        }
        lines
    }
}

const WHITE: [u8; 3] = [245, 245, 245];
const GREY: [u8; 3] = [152, 152, 152];
const FAINT: [u8; 3] = [112, 112, 112];
const BG: [u8; 3] = [22, 22, 22];
// Neutral greys, no blue: they sit on the in-game menu's black wash (Roy 2026-11-12).
const CARD: [u8; 3] = [40, 40, 40];
/// The ground of a card that is not earned yet: darker than an earned one.
const CARD_LOCKED: [u8; 3] = [22, 22, 22];
/// The share of a badge's edge left out so its circle is filled edge to edge.
const BADGE_INSET: f32 = 0.07;
/// How much of a bar's ink its empty part keeps.
const BAR_TRACK: f32 = 0.2;

/// Menu rules: the help line and legend sit where every other page has them.
const HELP_Y: f32 = 392.0;
const LEGEND_Y: f32 = 427.0;
/// One margin for every page: everything lines up on 32 and 608.
const LEFT: f32 = 32.0;
const RIGHT: f32 = 608.0;
/// Four cards fill the width between the margins exactly, 12 apart.
const CARD_W: f32 = 135.0;
const CARD_STEP: f32 = 147.0;
const CARD_H: f32 = 140.0;
/// The bottom row of cards, the same spot on every page.
const LOW_ROW: f32 = 208.0;
/// The pages after the first: two rows of four, the top row 12 above the bottom one.
pub const ROWS: usize = 2;
pub const PER_PAGE: usize = ROWS * COLS;
const HIGH_ROW: f32 = LOW_ROW - CARD_H - 12.0;
/// From a heading's capitals to the cards under it.
const HEADING_GAP: f32 = 12.0;
/// How far a card that is not earned yet fades away (it keeps 45%, like greyed-out rows).
const LOCKED_VEIL: f32 = 0.55;
/// The rarity pill.
const PILL_H: f32 = 16.0;
const PILL_PX: f32 = 9.0;
const PILL_TR: f32 = 0.14;
/// The rarity pill on the detail page: a size up, easier to read (Roy, ra7).
const PILL_BIG: (f32, f32) = (20.0, 11.0);

/// How rare an achievement is, from the share of players who earned it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
}

impl Rarity {
    pub fn of(percent: f32) -> Rarity {
        match percent {
            p if p >= 40.0 => Rarity::Common,
            p if p >= 20.0 => Rarity::Uncommon,
            p if p >= 5.0 => Rarity::Rare,
            _ => Rarity::Epic,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Rarity::Common => "Common",
            Rarity::Uncommon => "Uncommon",
            Rarity::Rare => "Rare",
            Rarity::Epic => "Epic",
        }
    }

    /// The pill's fill (white text sits on it) and the badge ring: grey, green, blue, purple.
    fn colour(self) -> [u8; 3] {
        match self {
            Rarity::Common => [104, 108, 116],
            Rarity::Uncommon => [40, 146, 88],
            Rarity::Rare => [48, 108, 206],
            Rarity::Epic => [134, 76, 206],
        }
    }
}

/// Every achievement in the order the set's makers gave them, earned or not, so each card
/// keeps its place.
pub fn ordered(list: &[Achievement]) -> Vec<&Achievement> {
    list.iter().collect()
}

/// The first page's four: the latest earned, or what is up next when nothing is earned yet.
pub fn recent(list: &[Achievement]) -> Vec<&Achievement> {
    let mut earned: Vec<&Achievement> = list.iter().filter(|a| a.unlocked).collect();
    earned.sort_by_key(|a| std::cmp::Reverse(a.unlock_time));
    if earned.is_empty() {
        return list.iter().take(COLS).collect();
    }
    earned.truncate(COLS);
    earned
}

/// Pages: the first with the header, then every achievement eight at a time.
pub fn pages(list: &[Achievement]) -> usize {
    1 + list.len().div_ceil(PER_PAGE)
}

/// The achievements on `page`, in the order they are drawn.
pub fn on_page(list: &[Achievement], page: usize) -> Vec<&Achievement> {
    if page == 0 {
        return recent(list);
    }
    ordered(list)
        .into_iter()
        .skip((page - 1) * PER_PAGE)
        .take(PER_PAGE)
        .collect()
}

/// Where a press takes the choice on the overview: left and right stay in the row, up and
/// down cross rows and pages. `dir` is 'l', 'r', 'u', 'd'; in the detail, 'p' and 'n' step
/// through every achievement in page order.
pub fn step(list: &[Achievement], page: usize, sel: usize, dir: char) -> (usize, usize) {
    let len = |p: usize| on_page(list, p).len();
    let n = len(page);
    let total = pages(list);
    let col = sel % COLS;
    match dir {
        'l' if col > 0 => (page, sel - 1),
        'r' if col + 1 < COLS && sel + 1 < n => (page, sel + 1),
        'u' if sel >= COLS => (page, sel - COLS),
        'u' if page > 0 => {
            let m = len(page - 1);
            let last_row = m.saturating_sub(1) / COLS * COLS;
            (page - 1, (last_row + col).min(m.saturating_sub(1)))
        }
        'd' if sel + COLS < n => (page, sel + COLS),
        'd' if sel / COLS < n.saturating_sub(1) / COLS => (page, n - 1),
        'd' if page + 1 < total => (page + 1, col.min(len(page + 1).saturating_sub(1))),
        'p' if sel > 0 => (page, sel - 1),
        'p' if page > 0 => (page - 1, len(page - 1).saturating_sub(1)),
        'n' if sel + 1 < n => (page, sel + 1),
        'n' if page + 1 < total => (page + 1, 0),
        _ => (page, sel),
    }
}

pub fn play_time(secs: u64) -> String {
    let (h, m) = (secs / 3600, secs % 3600 / 60);
    if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

fn badge_of(a: &Achievement) -> Option<Vec<u8>> {
    let name = if a.unlocked {
        a.badge.clone()
    } else {
        format!("{}_lock", a.badge)
    };
    BADGES.lock().ok()?.as_ref()?.get(&name).cloned()
}

/// The points chip: a white disc (black when its card is chosen) with the number centred.
fn chip(
    c: &mut Canvas,
    font: &Font,
    cx: f32,
    cy: f32,
    r: f32,
    px: f32,
    points: u32,
    sel: bool,
    rim: [u8; 3],
) {
    let (fill, fg) = if sel {
        ([14, 15, 18], WHITE)
    } else {
        (WHITE, BG)
    };
    c.disc(cx, cy, r + 2.0, 0.0, rim);
    c.disc(cx, cy, r, 0.0, fill);
    c.mid(font, &points.to_string(), cx, cy, px, 0.0, fg);
}

/// The ring (in the rarity's colour), the round badge and the points chip.
fn emblem(
    c: &mut Canvas,
    font: &Font,
    cx: f32,
    cy: f32,
    ring_r: f32,
    a: &Achievement,
    sel: bool,
    rim: [u8; 3],
) {
    let ring_w = if ring_r > 50.0 { 3.0 } else { 2.0 };
    let tone = Rarity::of(a.rarity).colour();
    c.disc(cx, cy, ring_r, ring_w, tone);
    let r = ring_r - ring_w - 3.0;
    match badge_of(a) {
        Some(px) => c.badge(cx, cy, r, &px, false),
        None => c.disc(
            cx,
            cy,
            r,
            0.0,
            if a.unlocked {
                [0x88, 0x88, 0x88]
            } else {
                [0x3a, 0x3a, 0x3a]
            },
        ),
    }
    // The chip sits on the ring at the lower right, 45 degrees round.
    let k = ring_r / 50.0;
    let d = ring_r * std::f32::consts::FRAC_1_SQRT_2;
    chip(
        c,
        font,
        cx + d,
        cy + d,
        (16.0 * k).max(11.0),
        (15.0 * k).max(11.0),
        a.points,
        sel,
        rim,
    );
}

/// The width of a rarity pill.
fn pill_width(font: &Font, rarity: Rarity) -> f32 {
    pill_width_at(font, rarity, (PILL_H, PILL_PX))
}

fn pill_width_at(font: &Font, rarity: Rarity, (h, px): (f32, f32)) -> f32 {
    let (_, w) = Canvas::ink(font, &rarity.label().to_uppercase(), px, PILL_TR);
    (w + 2.0 * h * 0.6).round()
}

/// The rarity as a pill in its colour, the word in white and centred on the ink.
/// Every pill's word takes the ground's colour: dark on a dark card, light on the chosen
/// white one.
fn rarity_pill(c: &mut Canvas, font: &Font, x: f32, y: f32, rarity: Rarity, ground: [u8; 3]) {
    rarity_pill_at(c, font, x, y, rarity, ground, (PILL_H, PILL_PX));
}

#[allow(clippy::too_many_arguments)]
fn rarity_pill_at(c: &mut Canvas, font: &Font, x: f32, y: f32, rarity: Rarity, ground: [u8; 3], (h, px): (f32, f32)) {
    let ink = ground;
    let w = pill_width_at(font, rarity, (h, px));
    c.pill(x, y, w, h, rarity.colour());
    c.mid(
        font,
        rarity.label(),
        x + w / 2.0,
        y + h / 2.0,
        px,
        PILL_TR,
        ink,
    );
}

/// A clock, `d` across, centred on (`cx`, `cy`).
fn clock(c: &mut Canvas, cx: f32, cy: f32, d: f32) {
    let r = d / 2.0;
    c.disc(cx, cy, r, 2.0, GREY);
    c.rect(cx - 1.0, cy - r + 4.0, 2.0, r - 4.0, GREY);
    c.rect(cx - 1.0, cy - 1.0, r - 3.0, 2.0, GREY);
}

/// A face rendered for the GPU, white on clear, blitted in `ink`.
fn blit(c: &mut Canvas, rgba: &[u8], w: u32, h: u32, x: f32, y: f32, ink: [u8; 3]) {
    for yy in 0..h as usize {
        for xx in 0..w as usize {
            let a = rgba[(yy * w as usize + xx) * 4 + 3] as f32 / 255.0;
            c.blend(
                x.round() as i32 + xx as i32,
                y.round() as i32 + yy as i32,
                ink,
                a,
            );
        }
    }
}

/// The box a face's ink covers: left, top, right, bottom (exclusive).
fn ink_box(rgba: &[u8], w: u32, h: u32) -> (f32, f32, f32, f32) {
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for y in 0..h {
        for x in 0..w {
            if rgba[((y * w + x) * 4 + 3) as usize] > 40 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    (x0 as f32, y0 as f32, x1 as f32, y1 as f32)
}

/// A full-colour face drawn at `x`, `y`.
#[allow(dead_code)]
fn blit_rgba(c: &mut Canvas, f: &CartFace, x: f32, y: f32) {
    for yy in 0..f.h as usize {
        for xx in 0..f.w as usize {
            let i = (yy * f.w as usize + xx) * 4;
            c.blend(
                x as i32 + xx as i32,
                y as i32 + yy as i32,
                [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2]],
                f.rgba[i + 3] as f32 / 255.0,
            );
        }
    }
}

fn hint(key: &str, label: &str) -> slot_ui::UndoFace {
    slot_ui::tint_mask(slot_ui::hint_face(key, label))
}

/// The legend along the foot, built from the menus' own hint faces.
fn legend(c: &mut Canvas, hints: &[(&str, &str)]) {
    legend_faces(c, hints.iter().map(|(k, l)| hint(k, l)).collect());
}

fn legend_faces(c: &mut Canvas, faces: Vec<slot_ui::UndoFace>) {
    let gap = slot_ui::LEGEND_GAP;
    let total: f32 = faces.iter().map(|f| f.w as f32).sum::<f32>() + gap * (faces.len() as f32 - 1.0);
    let mut x = (W as f32 / 2.0 - total / 2.0).round();
    for f in &faces {
        blit(c, &f.rgba, f.w, f.h, x, LEGEND_Y, WHITE);
        x += f.w as f32 + gap;
    }
}

/// The capitals' top of the help line.
fn help_cap(font: &Font) -> f32 {
    HELP_Y + Canvas::ascent(font, 16.0) - Canvas::cap(font, 16.0)
}

/// One line of help, 16 px, centred; cut rather than shrunk.
fn help(c: &mut Canvas, font: &Font, s: &str) {
    let lines = Canvas::wrap(font, s, 16.0, 0.0, 576.0, 1, true);
    if let Some(l) = lines.first() {
        c.line(
            font,
            l,
            W as f32 / 2.0,
            help_cap(font),
            16.0,
            0.0,
            GREY,
            'c',
        );
    }
}

/// The menus' own caret turned a quarter, as their lists show more above or below; its ink
/// centred on (`W` / 2, `cy`), in the menus' dim ink.
fn menu_arrow(c: &mut Canvas, cy: f32, up: bool) {
    let f = slot_ui::quick_caret_face(true);
    let (w, h) = (f.w, f.h);
    let mut rgba = vec![0u8; f.rgba.len()];
    for y in 0..h {
        for x in 0..w {
            let (nx, ny) = if up { (y, w - 1 - x) } else { (h - 1 - y, x) };
            let from = ((y * w + x) * 4) as usize;
            let to = ((ny * h + nx) * 4) as usize;
            rgba[to..to + 4].copy_from_slice(&f.rgba[from..from + 4]);
        }
    }
    let (x0, y0, x1, y1) = ink_box(&rgba, h, w);
    blit(c, &rgba, h, w, (W as f32 / 2.0 - (x0 + x1) / 2.0).round(), (cy - (y0 + y1) / 2.0).round(), GREY);
}

/// The arrow below the cards: midway between them and the help line.
fn arrow_down(c: &mut Canvas, font: &Font) {
    menu_arrow(c, (LOW_ROW + CARD_H + help_cap(font)) / 2.0, false);
}

/// The arrow above the heading: midway between the top of the screen and its capitals.
fn arrow_up(c: &mut Canvas, font: &Font) {
    menu_arrow(c, (HIGH_ROW - HEADING_GAP - Canvas::cap(font, 13.0)) / 2.0, true);
}

/// The avatar in a circle and the user's name, right on `right`, centred on `cy`.
fn profile(c: &mut Canvas, font: &Font, user: &str, right: f32, cy: f32, px: f32) {
    let w = c.caps(
        font,
        user,
        right,
        cy - Canvas::cap(font, px) / 2.0,
        px,
        0.12,
        WHITE,
        'r',
    );
    let r = 12.0;
    let cx = right - w - 10.0 - r;
    match AVATAR.lock().ok().and_then(|a| a.clone()) {
        Some((side, px)) => c.picture(cx, cy, r, &px, side),
        None => {
            c.disc(cx, cy, r, 0.0, CARD);
            let initial: String = user.chars().take(1).collect();
            c.mid(font, &initial, cx, cy, 12.0, 0.0, GREY);
        }
    }
    c.disc(cx, cy, r, 1.5, [92, 92, 92]);
}

/// The "beaten" stamp: a tick in a circle and the word, in a square box whose foot stands on
/// `bottom`, right on `right`.
fn beaten_stamp(c: &mut Canvas, font: &Font, right: f32, bottom: f32) {
    let px = 14.0;
    let (_, tw) = Canvas::ink(font, "BEATEN", px, 0.14);
    let (pad, d, gap, h) = (11.0, 18.0, 8.0, 30.0);
    let w = (pad + d + gap + tw + pad).round();
    let (x, y) = (right - w, bottom - h);
    c.rect(x, y, w, h, CARD);
    let (cx, cy) = (x + pad + d / 2.0, y + h / 2.0);
    c.disc(cx, cy, d / 2.0, 0.0, WHITE);
    for k in 0..=12 {
        let f = k as f32 / 12.0;
        c.disc(cx - 4.0 + 3.0 * f, cy + 0.5 + 3.0 * f, 1.1, 0.0, BG);
        c.disc(cx - 1.0 + 5.5 * f, cy + 3.5 - 7.0 * f, 1.1, 0.0, BG);
    }
    c.caps(
        font,
        "Beaten",
        x + pad + d + gap,
        cy - Canvas::cap(font, px) / 2.0,
        px,
        0.14,
        WHITE,
        'l',
    );
}

pub fn render(info: &Info, list: &[Achievement], view: &View, secs: u64, paused: bool) -> CartFace {
    let mut c = Canvas::new();
    let Some(font) = slot_ui::text::label_font() else {
        return CartFace {
            rgba: c.px,
            w: W,
            h: H,
        };
    };
    // No ground of its own: the app lays the in-game menu's black wash under the page.
    let items = on_page(list, view.page);
    if view.detail {
        if let Some(a) = items.get(view.sel) {
            detail(&mut c, font, info, a);
        }
    } else if view.page == 0 {
        first_page(&mut c, font, info, &items, view, secs, paused, pages(list));
    } else {
        list_page(&mut c, font, &items, view, pages(list));
    }
    CartFace {
        rgba: c.px,
        w: W,
        h: H,
    }
}

/// Two lines broken where they come out most even ("WAVY LINES / LOOK NICE", not
/// "WAVY LINES LOOK / NICE"), when the title fits in two without cutting.
fn balanced(font: &Font, title: &str, px: f32, tr: f32, w: f32, lines: Vec<String>) -> Vec<String> {
    if lines.len() != 2 || lines[1].ends_with('\u{2026}') {
        return lines;
    }
    let words: Vec<String> = title.to_uppercase().split_whitespace().map(str::to_string).collect();
    let mut best: Option<(f32, Vec<String>)> = None;
    for k in 1..words.len() {
        let (l1, l2) = (words[..k].join(" "), words[k..].join(" "));
        let widest = Canvas::tracked_width(font, &l1, px, tr).max(Canvas::tracked_width(font, &l2, px, tr));
        if widest <= w && best.as_ref().is_none_or(|b| widest < b.0) {
            best = Some((widest, vec![l1, l2]));
        }
    }
    best.map_or(lines, |b| b.1)
}

/// The smallest a card's title gets to fit a long word in.
const MIN_TITLE_PX: f32 = 9.0;

/// The left edge of card `col`.
fn card_x(col: usize) -> f32 {
    LEFT + col as f32 * CARD_STEP
}

/// A card: badge, title (one or two lines, centred between the chip and the pill), rarity
/// pill. 10 px inside at the top and the foot.
fn card(c: &mut Canvas, font: &Font, x: f32, y: f32, a: &Achievement, sel: bool) {
    let ground = if sel {
        WHITE
    } else if a.unlocked {
        CARD
    } else {
        CARD_LOCKED
    };
    c.rect(x, y, CARD_W, CARD_H, ground);
    let cx = x + CARD_W / 2.0;
    let ring = 30.0;
    emblem(c, font, cx, y + 10.0 + ring, ring, a, sel, ground);
    let ink = if sel { BG } else { WHITE };
    let (mut px, tr, step) = (12.0, 0.04, 15.0);
    let fit = CARD_W - 16.0;
    let mut lines = Canvas::wrap(font, &a.title, px, tr, fit, 2, true);
    // One word wider than the card and nowhere to break it: smaller, until it stays inside
    // the card's edge (Roy 2026-11-13).
    while px > MIN_TITLE_PX && lines.iter().any(|l| Canvas::tracked_width(font, l.trim_end_matches('\u{2026}'), px, tr) > fit) {
        px -= 0.5;
        lines = Canvas::wrap(font, &a.title, px, tr, fit, 2, true);
    }
    let lines = balanced(font, &a.title, px, tr, fit, lines);
    let pill_y = y + CARD_H - 10.0 - PILL_H;
    // The text block's middle sits midway between the chip's foot and the pill.
    let (top, bottom) = (y + 78.0, pill_y);
    let block = Canvas::cap(font, px) + step * (lines.len().max(1) as f32 - 1.0);
    let first = ((top + bottom) / 2.0 - block / 2.0).round();
    for (i, l) in lines.iter().enumerate() {
        c.line(font, l, cx, first + i as f32 * step, px, tr, ink, 'c');
    }
    let rarity = Rarity::of(a.rarity);
    rarity_pill(
        c,
        font,
        (cx - pill_width(font, rarity) / 2.0).round(),
        pill_y,
        rarity,
        ground,
    );
    // Not earned yet: the whole card turns see-through, like every greyed-out item (Roy, ui1).
    if !a.unlocked {
        c.fade(x, y, CARD_W, CARD_H, 1.0 - LOCKED_VEIL);
    }
}

#[allow(clippy::too_many_arguments)]
fn first_page(
    c: &mut Canvas,
    font: &Font,
    info: &Info,
    items: &[&Achievement],
    view: &View,
    secs: u64,
    paused: bool,
    pages: usize,
) {
    // Top line: the game on the left, the player on the right, on one middle.
    let top = 20.0;
    c.caps(font, &info.game_title, LEFT, top, 13.0, 0.12, GREY, 'l');
    profile(
        c,
        font,
        &info.user,
        RIGHT,
        top + Canvas::cap(font, 13.0) / 2.0,
        13.0,
    );
    c.caps(font, "Achievements", LEFT, 44.0, 28.0, 0.08, WHITE, 'l');
    // The count: the big number and the total on one baseline; Beaten stands on it too.
    let num_top = 80.0;
    let base = num_top + Canvas::cap(font, 56.0);
    let nw = c.line(
        font,
        &info.unlocked.to_string(),
        LEFT,
        num_top,
        56.0,
        0.0,
        WHITE,
        'l',
    );
    c.line(
        font,
        &format!("/{}", info.total),
        LEFT + nw + 8.0,
        base - Canvas::cap(font, 32.0),
        32.0,
        0.0,
        GREY,
        'l',
    );
    if info.beaten_time != 0 {
        beaten_stamp(c, font, RIGHT, base);
    }
    let by = base + 16.0;
    let share = if info.points_total > 0 {
        info.points_unlocked as f32 / info.points_total as f32
    } else {
        0.0
    };
    c.bar(LEFT, by, RIGHT - LEFT, 6.0, share, WHITE);
    let row = by + 16.0;
    let px = 13.0;
    c.caps(
        font,
        &format!("{} of {} points", info.points_unlocked, info.points_total),
        LEFT,
        row,
        px,
        0.12,
        GREY,
        'l',
    );
    let played = if paused {
        "Paused while cheats are on".to_string()
    } else {
        format!("{} played", play_time(secs))
    };
    let tw = c.caps(font, &played, RIGHT, row, px, 0.12, GREY, 'r');
    if !paused {
        clock(
            c,
            RIGHT - tw - 8.0 - 7.0,
            row + Canvas::cap(font, px) / 2.0,
            14.0,
        );
    }
    let heading = if items.iter().any(|a| a.unlocked) {
        "Recently earned"
    } else {
        "Up next"
    };
    c.caps(
        font,
        heading,
        LEFT,
        LOW_ROW - HEADING_GAP - Canvas::cap(font, px),
        px,
        0.14,
        WHITE,
        'l',
    );
    for (i, a) in items.iter().enumerate() {
        card(c, font, card_x(i), LOW_ROW, a, i == view.sel);
    }
    if pages > 1 {
        arrow_down(c, font);
    }
    if let Some(a) = items.get(view.sel) {
        help(c, font, &a.description);
    }
    legend(c, &[("A", "Details"), ("B", "Back")]);
}

fn list_page(c: &mut Canvas, font: &Font, items: &[&Achievement], view: &View, pages: usize) {
    arrow_up(c, font);
    // The heading, as "Recently earned" on the first page: the same gap above the cards,
    // and on one middle with the arrow.
    let px = 13.0;
    c.caps(font, "All achievements", LEFT, HIGH_ROW - HEADING_GAP - Canvas::cap(font, px), px, 0.14, WHITE, 'l');
    for (i, a) in items.iter().enumerate() {
        let y = if i < COLS { HIGH_ROW } else { LOW_ROW };
        card(c, font, card_x(i % COLS), y, a, i == view.sel);
    }
    if view.page + 1 < pages {
        arrow_down(c, font);
    }
    if let Some(a) = items.get(view.sel) {
        help(c, font, &a.description);
    }
    legend(c, &[("A", "Details"), ("B", "Back")]);
}

fn detail(c: &mut Canvas, font: &Font, info: &Info, a: &Achievement) {
    let mid = W as f32 / 2.0;
    c.caps(font, &info.game_title, mid, 18.0, 13.0, 0.14, GREY, 'c');
    let titles = Canvas::wrap(font, &a.title, 26.0, 0.1, RIGHT - LEFT, 2, true);
    let words = Canvas::wrap(font, &a.description, 16.0, 0.0, 520.0, 3, true);
    // The stack (badge, date, title, description, rarity row) measured first, then centred
    // between the game's name and the legend.
    let ring = 56.0;
    let (date_px, title_px, desc_px, row_px) = (13.0, 26.0, 16.0, 14.0);
    let title_h = Canvas::cap(font, title_px) + 34.0 * (titles.len() as f32 - 1.0);
    let desc_h = if words.is_empty() {
        0.0
    } else {
        Canvas::cap(font, desc_px) + 22.0 * (words.len() as f32 - 1.0)
    };
    let stack = 2.0 * ring
        + 8.0
        + 18.0
        + Canvas::cap(font, date_px)
        + 22.0
        + title_h
        + 18.0
        + desc_h
        + 34.0
        + Canvas::cap(font, row_px)
        + 12.0
        + PILL_BIG.0;
    let mut y = (40.0 + (LEGEND_Y - 24.0 - 40.0 - stack) / 2.0).round();
    emblem(c, font, mid, y + ring, ring, a, false, BG);
    y += 2.0 * ring + 8.0 + 18.0;
    let when = if a.unlocked {
        unlocked_on(a.unlock_time)
    } else {
        "Not earned yet".to_string()
    };
    c.caps(
        font,
        &when,
        mid,
        y,
        date_px,
        0.14,
        if a.unlocked { GREY } else { FAINT },
        'c',
    );
    y += Canvas::cap(font, date_px) + 22.0;
    for (i, l) in titles.iter().enumerate() {
        c.line(font, l, mid, y + 34.0 * i as f32, title_px, 0.1, WHITE, 'c');
    }
    y += title_h + 18.0;
    for (i, l) in words.iter().enumerate() {
        c.line(font, l, mid, y + 22.0 * i as f32, desc_px, 0.0, GREY, 'c');
    }
    y += desc_h + 34.0;
    // The share bar on the page's centre line, the share of players in front of it on its
    // middle, and the rarity pill centred under the bar.
    let rarity = Rarity::of(a.rarity);
    let cy = (y + Canvas::cap(font, row_px) / 2.0).round();
    let bw = 160.0;
    let bx = (mid - bw / 2.0).round();
    c.pill_alpha(bx, cy - 2.0, bw, 4.0, WHITE, BAR_TRACK);
    let fill = (bw * (a.rarity / 100.0).clamp(0.0, 1.0)).round();
    if fill > 0.0 {
        c.pill_alpha(bx, cy - 2.0, fill.max(4.0), 4.0, rarity.colour(), 1.0);
    }
    let pct = format!("{:.0}%", a.rarity);
    let ctop = cy - Canvas::cap(font, row_px) / 2.0;
    // The number of players on the left of the bar, the share on the right.
    let count = COUNTS.lock().ok().and_then(|c| c.as_ref()?.get(&a.id).copied());
    if let Some(n) = count {
        let right = bx - 14.0;
        let (_, ow) = Canvas::ink(font, "PLAYERS", row_px, 0.12);
        let space = (row_px * 0.45).round();
        c.caps(font, "players", right, ctop, row_px, 0.12, GREY, 'r');
        c.line(font, &thousands(n), right - ow - space, ctop, row_px, 0.12, WHITE, 'r');
    }
    c.line(font, &pct, bx + bw + 14.0, ctop, row_px, 0.12, WHITE, 'l');
    let pill_y = y + Canvas::cap(font, row_px) + 12.0;
    rarity_pill_at(c, font, (mid - pill_width_at(font, rarity, PILL_BIG) / 2.0).round(), pill_y, rarity, BG, PILL_BIG);
    legend_faces(c, vec![hint("B", "Back"), slot_ui::tint_mask(slot_ui::keys_hint_face(&["L1", "R1"], "Page"))]);
}

/// "7 Nov 2026" from unix seconds.
fn unlocked_on(t: i64) -> String {
    let days = t.div_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(m - 1) as usize];
    format!("{d} {month} {y}")
}

/// The trophy, solid white with a dark rim, `h` pixels tall (the glyph font has none).
pub fn trophy_face(h: u32) -> CartFace {
    const S: usize = 4;
    let g = (h as usize) * S;
    let unit = g as f32 / 24.0;
    let poly = [
        (6.0, 3.0),
        (18.0, 3.0),
        (18.0, 9.0),
        (15.6, 12.6),
        (13.0, 14.0),
        (13.0, 16.6),
        (16.0, 18.6),
        (16.0, 21.0),
        (8.0, 21.0),
        (8.0, 18.6),
        (11.0, 16.6),
        (11.0, 14.0),
        (8.4, 12.6),
        (6.0, 9.0),
    ];
    let inside = |x: f32, y: f32| -> bool {
        let mut c = false;
        let mut j = poly.len() - 1;
        for i in 0..poly.len() {
            let (xi, yi) = poly[i];
            let (xj, yj) = poly[j];
            if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                c = !c;
            }
            j = i;
        }
        c
    };
    let handle = |x: f32, y: f32, cx: f32, left: bool| -> bool {
        let (dx, dy) = (x - cx, y - 7.2);
        let d = (dx * dx + dy * dy).sqrt();
        d <= 3.6 && d >= 2.0 && if left { x < cx } else { x > cx }
    };
    let mut cov = vec![0u8; (h * h) as usize];
    for y in 0..h as usize {
        for x in 0..h as usize {
            let mut n = 0;
            for sy in 0..S {
                for sx in 0..S {
                    let (ux, uy) = (
                        ((x * S + sx) as f32 + 0.5) / unit,
                        ((y * S + sy) as f32 + 0.5) / unit,
                    );
                    if inside(ux, uy) || handle(ux, uy, 6.0, true) || handle(ux, uy, 18.0, false) {
                        n += 1;
                    }
                }
            }
            cov[y * h as usize + x] = (n * 255 / (S * S)) as u8;
        }
    }
    slot_ui::haloed(&cov, h, h, [0xff, 0xff, 0xff])
}

/// The big flat button on top of the in-game menu: trophy and name centred like the menu's
/// own rows, what is earned under it, a points bar, and the menus' caret. 640 x 116.
pub fn button_face(info: &Info, selected: bool, off: bool) -> CartFace {
    let mut c = Canvas::new();
    let h = 116u32;
    let Some(font) = slot_ui::text::label_font() else {
        return CartFace {
            rgba: c.px[..(W * h * 4) as usize].to_vec(),
            w: W,
            h,
        };
    };
    let (bg, ink, sub) = if selected {
        (WHITE, BG, [72, 72, 72])
    } else {
        (CARD, WHITE, GREY)
    };
    // Off (cheats on), chosen: the bar stays solid, its words faint like a locked row's. At
    // rest the app draws the whole button see-through instead.
    let (ink, sub) = match off && selected {
        true => (fade(ink, bg, 0.45), fade(sub, bg, 0.45)),
        false => (ink, sub),
    };
    if selected {
        c.rect(0.0, 0.0, W as f32, h as f32, bg);
    }
    let mid = W as f32 / 2.0;
    // Trophy and title share one middle; the block of title, line and bar sits in the middle
    // of the button.
    let (title_px, sub_px) = (26.0, 13.0);
    let (tcap, scap) = (Canvas::cap(font, title_px), Canvas::cap(font, sub_px));
    let block = tcap + 14.0 + scap + 16.0 + 6.0;
    let top = ((h as f32 - block) / 2.0).round();
    let t = trophy_face(34);
    let (tx0, ty0, tx1, ty1) = ink_box(&t.rgba, t.w, t.h);
    let (_, tw) = Canvas::ink(font, "ACHIEVEMENTS", title_px, 0.1);
    let gap = 14.0;
    let x = (mid - ((tx1 - tx0) + gap + tw) / 2.0).round();
    let tmid = top + tcap / 2.0;
    blit(
        &mut c,
        &t.rgba,
        t.w,
        t.h,
        x - tx0,
        (tmid - (ty0 + ty1) / 2.0).round(),
        ink,
    );
    c.line(
        font,
        "ACHIEVEMENTS",
        x + (tx1 - tx0) + gap,
        top,
        title_px,
        0.1,
        ink,
        'l',
    );
    let line = format!(
        "{} of {} earned \u{b7} {} of {} points",
        info.unlocked, info.total, info.points_unlocked, info.points_total
    );
    let sy = top + tcap + 14.0;
    c.caps(font, &line, mid, sy, sub_px, 0.1, sub, 'c');
    let share = if info.points_total > 0 {
        info.points_unlocked as f32 / info.points_total as f32
    } else {
        0.0
    };
    let (bw, by) = (360.0, sy + scap + 16.0);
    c.bar(mid - bw / 2.0, by, bw, 6.0, share, ink);
    // The caret every menu row that opens a page carries: its ink on the margin, centred.
    let caret = slot_ui::quick_caret_face(true);
    let (cx0, cy0, cx1, cy1) = ink_box(&caret.rgba, caret.w, caret.h);
    blit(
        &mut c,
        &caret.rgba,
        caret.w,
        caret.h,
        RIGHT - cx1,
        (h as f32 / 2.0 - (cy0 + cy1) / 2.0).round(),
        ink,
    );
    let _ = cx0;
    CartFace {
        rgba: c.px[..(W * h * 4) as usize].to_vec(),
        w: W,
        h,
    }
}

/// `c` at `a` over `ground`.
fn fade(c: [u8; 3], ground: [u8; 3], a: f32) -> [u8; 3] {
    std::array::from_fn(|k| (c[k] as f32 * a + ground[k] as f32 * (1.0 - a)).round() as u8)
}

pub const TOAST_H: u32 = 64;

/// The unlock: the black top bar of every toast, taller, with the badge in its rarity ring,
/// the title and the points; badge and chip the same distance from the edges. 640 x 64.
pub fn toast_face(a: &Achievement) -> CartFace {
    let mut c = Canvas::new();
    let h = TOAST_H;
    let Some(font) = slot_ui::text::label_font() else {
        return CartFace {
            rgba: c.px[..(W * h * 4) as usize].to_vec(),
            w: W,
            h,
        };
    };
    c.rect(0.0, 0.0, W as f32, h as f32, [0, 0, 0]);
    let cy = h as f32 / 2.0;
    let (ring, edge) = (24.0, 20.0);
    let bx = edge + ring;
    c.disc(bx, cy, ring, 2.0, Rarity::of(a.rarity).colour());
    match badge_of(a) {
        Some(px) => c.badge(bx, cy, ring - 5.0, &px, false),
        None => c.disc(bx, cy, ring - 5.0, 0.0, [0x88, 0x88, 0x88]),
    }
    let r = 17.0;
    let px_cx = W as f32 - edge - r;
    chip(&mut c, font, px_cx, cy, r, 15.0, a.points, false, [0, 0, 0]);
    // Two lines, their capitals centred as one block on the bar's middle.
    let (lpx, tpx, gap) = (11.0, 20.0, 9.0);
    let block = Canvas::cap(font, lpx) + gap + Canvas::cap(font, tpx);
    let top = (cy - block / 2.0).round();
    let tx = bx + ring + 16.0;
    c.caps(font, "Achievement unlocked", tx, top, lpx, 0.16, GREY, 'l');
    if let Some(l) =
        Canvas::wrap(font, &a.title, tpx, 0.04, px_cx - r - 16.0 - tx, 1, true).first()
    {
        c.line(
            font,
            l,
            tx,
            top + Canvas::cap(font, lpx) + gap,
            tpx,
            0.04,
            WHITE,
            'l',
        );
    }
    CartFace {
        rgba: c.px[..(W * h * 4) as usize].to_vec(),
        w: W,
        h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps() {
        let list: Vec<Achievement> = (0..19)
            .map(|i| Achievement {
                id: i,
                points: 5,
                unlocked: i < 6,
                unlock_time: i as i64,
                rarity: 50.0,
                title: String::new(),
                description: String::new(),
                badge: String::new(),
            })
            .collect();
        assert_eq!(pages(&list), 1 + 3);
        assert_eq!(step(&list, 0, 3, 'r'), (0, 3));
        assert_eq!(step(&list, 0, 2, 'd'), (1, 2));
        assert_eq!(step(&list, 1, 2, 'u'), (0, 2));
        assert_eq!(step(&list, 1, 6, 'd'), (2, 2));
        assert_eq!(step(&list, 3, 1, 'u'), (2, 5));
        assert_eq!(step(&list, 3, 2, 'r'), (3, 2));
        assert_eq!(step(&list, 1, 7, 'n'), (2, 0));
    }

    #[test]
    fn thousands_and_counts() {
        assert_eq!(thousands(1234567), "1,234,567");
        assert_eq!(thousands(12), "12");
        let m = crate::ra::parse_counts(r#"{"Achievements":{"1":{"ID":7,"NumAwarded":300,"Title":"x"},"2":{"ID":9,"NumAwarded":"41"}}}"#);
        assert_eq!((m[&7], m[&9]), (300, 41));
    }

    #[test]
    fn a_long_hyphenated_title_breaks_at_its_hyphen_inside_the_card() {
        let font = slot_ui::text::label_font().unwrap();
        let lines = Canvas::wrap(font, "Batman-ufacturing", 12.0, 0.04, CARD_W - 16.0, 2, true);
        assert_eq!(lines, ["BATMAN-", "UFACTURING"]);
        let lines = Canvas::wrap(font, "Up-to-date", 12.0, 0.04, CARD_W - 16.0, 2, true);
        assert_eq!(lines, ["UP-TO-DATE"]);
    }

    #[test]
    fn dates() {
        assert_eq!(unlocked_on(0), "1 Jan 1970");
        assert_eq!(unlocked_on(1_762_473_600), "7 Nov 2025");
    }
}

#[cfg(test)]
mod preview {
    use super::*;

    fn save(dir: &str, name: &str, f: &CartFace, ground: [u8; 3]) {
        let mut bg = vec![0u8; f.rgba.len()];
        for p in bg.chunks_mut(4) {
            p.copy_from_slice(&[ground[0], ground[1], ground[2], 255]);
        }
        for (d, s) in bg.chunks_mut(4).zip(f.rgba.chunks(4)) {
            let a = s[3] as u32;
            for k in 0..3 {
                d[k] = ((s[k] as u32 * a + d[k] as u32 * (255 - a)) / 255) as u8;
            }
        }
        let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
        let mut enc = png::Encoder::new(file, f.w, f.h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&bg).unwrap();
    }

    /// With RA_PREVIEW=<dir>, writes every screen as a png there. Badges come from
    /// RA_BADGES=<dir> (<name>.png and <name>_lock.png, 64x64), when given.
    #[test]
    fn preview() {
        let Ok(dir) = std::env::var("RA_PREVIEW") else {
            return;
        };
        *COUNTS.lock().unwrap() = Some((0..40).map(|i| (i, 1234 * (i + 1))).collect());
        // RA_AVATAR=<square png>: the player's picture in the corner.
        if let Ok(path) = std::env::var("RA_AVATAR") {
            let mut dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
            dec.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
            let mut r = dec.read_info().unwrap();
            let mut buf = vec![0; r.output_buffer_size()];
            let fr = r.next_frame(&mut buf).unwrap();
            buf.truncate(fr.buffer_size());
            *AVATAR.lock().unwrap() = Some((fr.width as usize, buf));
        }
        let mut list = Vec::new();
        let mut names = Vec::new();
        if let Ok(src) = std::env::var("RA_BADGES") {
            let json = std::fs::read_to_string(format!("{src}/list.tsv")).unwrap();
            for line in json.lines() {
                let f: Vec<&str> = line.split('\t').collect();
                names.push((
                    f[0].to_string(),
                    f[1].to_string(),
                    f[2].to_string(),
                    f[3].parse::<u32>().unwrap(),
                    f[4].parse::<f32>().unwrap(),
                ));
                for n in [f[0].to_string(), format!("{}_lock", f[0])] {
                    if let Ok(file) = std::fs::File::open(format!("{src}/{n}.png")) {
                        let mut dec = png::Decoder::new(std::io::BufReader::new(file));
                        dec.set_transformations(
                            png::Transformations::normalize_to_color8()
                                | png::Transformations::ALPHA,
                        );
                        let mut r = dec.read_info().unwrap();
                        let mut buf = vec![0; r.output_buffer_size()];
                        let fr = r.next_frame(&mut buf).unwrap();
                        let rgba: Vec<u8> = match fr.color_type {
                            png::ColorType::Rgba => buf[..64 * 64 * 4].to_vec(),
                            png::ColorType::GrayscaleAlpha => buf[..64 * 64 * 2]
                                .chunks(2)
                                .flat_map(|p| [p[0], p[0], p[0], p[1]])
                                .collect(),
                            _ => continue,
                        };
                        BADGES
                            .lock()
                            .unwrap()
                            .get_or_insert_with(Default::default)
                            .insert(n, rgba);
                    }
                }
            }
        }
        for (i, (badge, title, desc, points, rarity)) in names.into_iter().enumerate() {
            list.push(Achievement {
                id: i as u32,
                points,
                unlocked: [0, 1, 2, 5, 7, 9].contains(&i),
                unlock_time: 1_762_500_000 + i as i64 * 3600,
                rarity,
                title,
                description: desc,
                badge,
            });
        }
        let earned = list.iter().filter(|a| a.unlocked).count() as u32;
        let pts: u32 = list.iter().filter(|a| a.unlocked).map(|a| a.points).sum();
        let info = Info {
            state: slot_achievements::State::GameReady,
            game_id: 682,
            game_title: "Alleyway".into(),
            game_badge: String::new(),
            user: "roygravy".into(),
            error: String::new(),
            total: list.len() as u32,
            unlocked: earned,
            points_total: list.iter().map(|a| a.points).sum(),
            points_unlocked: pts,
            beaten_time: 1_762_500_000,
        };
        let shots = [
            ("1-overzicht", 0usize, 1usize, false),
            ("2-pagina-2", 1, 5, false),
            ("3-detail", 0, 1, true),
            ("2b-pagina-3", 2, 0, false),
        ];
        for (name, page, sel, detail) in shots {
            save(
                &dir,
                name,
                &render(&info, &list, &View { page, sel, detail }, 20520, false),
                [0, 0, 0],
            );
        }
        save(&dir, "4-knop-gekozen", &button_face(&info, true, false), [0, 0, 0]);
        save(&dir, "4-knop", &button_face(&info, false, false), [0, 0, 0]);
        save(&dir, "4-knop-uit-gekozen", &button_face(&info, true, true), [0, 0, 0]);
        save(&dir, "4-knop-uit", &button_face(&info, false, true), [0, 0, 0]);
        save(&dir, "5-melding", &toast_face(&list[1]), [60, 60, 60]);
        // The in-game menu, roughly: the game dimmed, the button and the four rows under it.
        let font = slot_ui::text::label_font().unwrap();
        let mut c = Canvas::new();
        c.rect(0.0, 0.0, W as f32, H as f32, [20, 30, 36]);
        c.rect(0.0, 0.0, W as f32, 40.0, [0, 0, 0]);
        c.put(font, "12:41", 16.0, 9.0, 18.0, 0.0, WHITE);
        let b = button_face(&info, true, false);
        blit_rgba(&mut c, &b, 0.0, 56.0);
        for (i, row) in ["States", "Cheats", "Settings", "Exit Game"]
            .iter()
            .enumerate()
        {
            c.caps(
                font,
                row,
                W as f32 / 2.0,
                202.0 + i as f32 * 48.0,
                26.0,
                0.0,
                GREY,
                'c',
            );
        }
        help(&mut c, font, "Your achievements for this game.");
        legend(&mut c, &[("A", "Open"), ("B", "Resume")]);
        save(
            &dir,
            "6-ingame-menu",
            &CartFace {
                rgba: c.px,
                w: W,
                h: H,
            },
            [0, 0, 0],
        );
    }
}
