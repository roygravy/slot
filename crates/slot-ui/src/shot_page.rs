//! The Screenshots page: a carousel of framed pictures on the menu's flat ground, the chosen
//! one in a white frame between its neighbours, and A to see it fill the screen. Deleting asks
//! first: A deletes, B cancels.

use crate::draw::{Canvas, Draw, TexId, OUT_H, OUT_W};
use crate::hud::{PLATE, PLATE_H};
use crate::plate::{centred_hints, HINT_H, LEGEND_GAP};
use crate::theme::Ink;

/// The picture in the middle, its frame, and the step to its neighbours.
pub const SHOT_W: f32 = 400.0;
pub const SHOT_H: f32 = 300.0;
const SHOT_TOP: f32 = 60.0;
const FRAME: f32 = 6.0;
const PITCH: f32 = SHOT_W + 24.0;
const DOTS_Y: f32 = SHOT_TOP + SHOT_H + 18.0;
const LINE_Y: f32 = 392.0;
const LEGEND_Y: f32 = 427.0;
const EMPTY_Y: f32 = 170.0;
const DOT: f32 = 6.0;
const DOT_GAP: f32 = 10.0;
const DOT_DIM: f32 = 0.35;
const DOTS_MAX: usize = 15;
const MARGIN: f32 = 16.0;
const PLATE_GAP: f32 = 16.0;

pub type Face = (TexId, u32, u32);
pub type Hint = (TexId, u32);

pub struct ShotFaces {
    /// The game the chosen picture is from; the date and time it was taken.
    pub title: Option<Face>,
    pub date: Option<Face>,
    pub confirm: Face,
    pub empty: Face,
    pub empty_help: Face,
    /// Back, View, Delete; Back alone; Cancel, Delete.
    pub legend: Vec<Hint>,
    pub legend_empty: Vec<Hint>,
    pub legend_confirm: Vec<Hint>,
    /// Full screen: Close and Delete on the left, Flick on the right.
    pub full_left: Vec<Hint>,
    pub full_right: Vec<Hint>,
}

pub struct ShotPage<'a> {
    pub count: usize,
    pub sel: usize,
    /// Where the carousel is, easing towards `sel` after a flick.
    pub pos: f32,
    /// The pictures loaded so far, by index.
    pub pics: &'a [(usize, TexId)],
    pub full: bool,
    /// How much of the full-screen hint plate shows.
    pub hints: f32,
    pub confirm: bool,
    pub faces: Option<&'a ShotFaces>,
    pub ink: Ink,
    pub ground: [f32; 4],
    pub canvas: Canvas,
}

fn tint(out: &mut Vec<Draw>, f: Face, x: f32, y: f32, colour: [f32; 4]) {
    out.push(Draw::Tint {
        x: x.round(),
        y: y.round(),
        w: f.1 as f32,
        h: f.2 as f32,
        tex: f.0,
        colour,
    });
}

fn rect(out: &mut Vec<Draw>, x: f32, y: f32, w: f32, h: f32, colour: [f32; 4]) {
    out.push(Draw::Rect { x, y, w, h, colour });
}

fn fade(mut c: [f32; 4], a: f32) -> [f32; 4] {
    c[3] *= a;
    c
}

impl ShotPage<'_> {
    fn pic(&self, i: usize) -> Option<TexId> {
        self.pics.iter().find(|(k, _)| *k == i).map(|(_, t)| *t)
    }

    pub fn draw(&self, out: &mut Vec<Draw>) {
        if self.full && self.count > 0 {
            return self.draw_full(out);
        }
        let w = self.canvas.w;
        rect(out, 0.0, 0.0, w, OUT_H as f32, self.ground);
        let Some(faces) = self.faces else {
            return;
        };
        let ink = self.ink;
        if self.count == 0 {
            tint(out, faces.empty, self.canvas.centred(faces.empty.1 as f32), EMPTY_Y, ink.dim);
            let h = faces.empty_help;
            tint(out, h, self.canvas.centred(h.1 as f32), LINE_Y, ink.dim);
            self.legend(&faces.legend_empty, out);
            return;
        }
        let centre_x = self.canvas.centred(SHOT_W);
        let lo = (self.pos.floor() as isize - 2).max(0) as usize;
        let hi = ((self.pos.ceil() as usize) + 2).min(self.count - 1);
        for i in lo..=hi {
            let x = centre_x + (i as f32 - self.pos) * PITCH;
            if x + SHOT_W + FRAME < 0.0 || x - FRAME > w {
                continue;
            }
            let off = (i as f32 - self.pos).abs().min(1.0);
            let (fx, fy, fw, fh) = (x - FRAME, SHOT_TOP - FRAME, SHOT_W + 2.0 * FRAME, SHOT_H + 2.0 * FRAME);
            rect(out, fx, fy, fw, fh, fade(ink.dim, 0.3));
            rect(out, fx, fy, fw, fh, fade(ink.bar, 1.0 - off));
            match self.pic(i) {
                Some(tex) => out.push(Draw::Tex {
                    x: x.round(),
                    y: SHOT_TOP,
                    w: SHOT_W,
                    h: SHOT_H,
                    tex,
                    alpha: 1.0,
                }),
                None => rect(out, x, SHOT_TOP, SHOT_W, SHOT_H, fade(ink.dim, 0.2)),
            }
            if off > 0.0 {
                rect(out, fx, fy, fw, fh, fade(self.ground, 0.6 * off));
            }
        }
        self.dots(DOTS_Y, ink.text, out);
        if let Some(t) = faces.title {
            tint(out, t, self.canvas.centred(t.1 as f32), (PLATE_H - t.2 as f32) / 2.0, ink.text);
        }
        if self.confirm {
            let c = faces.confirm;
            tint(out, c, self.canvas.centred(c.1 as f32), LINE_Y, ink.text);
            self.legend(&faces.legend_confirm, out);
        } else {
            if let Some(d) = faces.date {
                tint(out, d, self.canvas.centred(d.1 as f32), LINE_Y, ink.dim);
            }
            self.legend(&faces.legend, out);
        }
    }

    fn legend(&self, hints: &[Hint], out: &mut Vec<Draw>) {
        for (tex, w, x) in centred_hints(self.canvas, hints, LEGEND_GAP) {
            out.push(Draw::Tint {
                x,
                y: LEGEND_Y,
                w: w as f32,
                h: HINT_H as f32,
                tex,
                colour: self.ink.text,
            });
        }
    }

    fn dots(&self, y: f32, colour: [f32; 4], out: &mut Vec<Draw>) {
        if self.count < 2 {
            return;
        }
        let shown = self.count.min(DOTS_MAX);
        let first = self
            .sel
            .saturating_sub(shown / 2)
            .min(self.count - shown);
        let row = shown as f32 * DOT + (shown as f32 - 1.0) * DOT_GAP;
        let mut x = self.canvas.centred(row).round();
        for i in first..first + shown {
            let a = if i == self.sel { 1.0 } else { DOT_DIM };
            rect(out, x, y, DOT, DOT, fade(colour, a));
            x += DOT + DOT_GAP;
        }
    }

    fn draw_full(&self, out: &mut Vec<Draw>) {
        let w = self.canvas.w;
        rect(out, 0.0, 0.0, w, OUT_H as f32, [0.0, 0.0, 0.0, 1.0]);
        if let Some(tex) = self.pic(self.sel) {
            out.push(Draw::Tex {
                x: self.canvas.centred(OUT_W as f32),
                y: 0.0,
                w: OUT_W as f32,
                h: OUT_H as f32,
                tex,
                alpha: 1.0,
            });
        }
        let a = if self.confirm { 1.0 } else { self.hints };
        let Some(faces) = self.faces.filter(|_| a > 0.0) else {
            return;
        };
        let top = OUT_H as f32 - PLATE_H;
        rect(out, 0.0, top, w, PLATE_H, fade(PLATE, a));
        let white = [1.0, 1.0, 1.0, a];
        let y = top + (PLATE_H - HINT_H as f32) / 2.0;
        let (left, right): (&[Hint], &[Hint]) = match self.confirm {
            true => (&faces.legend_confirm[..1], &faces.legend_confirm[1..]),
            false => (&faces.full_left, &faces.full_right),
        };
        let mut x = MARGIN;
        for &(tex, hw) in left {
            tint(out, (tex, hw, HINT_H), x, y, white);
            x += hw as f32 + PLATE_GAP;
        }
        let right_w: f32 =
            right.iter().map(|&(_, hw)| hw as f32).sum::<f32>() + PLATE_GAP * right.len().saturating_sub(1) as f32;
        let mut x = w - MARGIN - right_w;
        for &(tex, hw) in right {
            tint(out, (tex, hw, HINT_H), x, y, white);
            x += hw as f32 + PLATE_GAP;
        }
        if self.confirm {
            let c = faces.confirm;
            tint(out, c, self.canvas.centred(c.1 as f32), top + (PLATE_H - c.2 as f32) / 2.0, white);
        } else {
            self.dots(top + (PLATE_H - DOT) / 2.0, white, out);
        }
    }
}
