use slot_gfx::{Draw, TexId, OUT_H, OUT_W};
use slot_power::Battery;
use slot_store::{parse_stamp, StateEntry};

use crate::art;
use crate::footer::{draw_corners, Printed};
use crate::hud::{PLATE, PLATE_H};
use crate::plate::{hint_quad, hint_row, hint_width, Hint, HINT_GAP, HINT_H, TITLE_H, TITLE_W};

pub const PHOTO_W: u32 = 240;
pub const PHOTO_H: u32 = 160;

const BLANK: [u8; 3] = [0x3a, 0x3a, 0x3e];

pub const DOT: f32 = 6.0;
const DOT_GAP: f32 = 10.0;
const DOT_DIM: f32 = 0.35;

const MARGIN: f32 = 16.0;

pub const LEGEND: [(&str, &str); 3] = [("B", "Back"), ("Y", "Delete"), ("A", "Load")];
const WAYS_OUT: usize = 2;

pub struct PhotoFace {
    pub rgba: Vec<u8>,
    pub w: u32,
    pub h: u32,
}

pub fn photo_face(entry: &StateEntry) -> PhotoFace {
    let rgba = art::cover(&entry.thumb, PHOTO_W, PHOTO_H).unwrap_or_else(|| {
        std::iter::repeat_n(
            [BLANK[0], BLANK[1], BLANK[2], 255],
            (PHOTO_W * PHOTO_H) as usize,
        )
        .flatten()
        .collect()
    });
    PhotoFace {
        rgba,
        w: PHOTO_W,
        h: PHOTO_H,
    }
}

/// Asking before a state is deleted: B cancels on the left, A deletes on the right, and the
/// question between them.
#[derive(Clone, Copy)]
pub struct ConfirmFaces {
    pub cancel: (TexId, u32),
    pub delete: (TexId, u32),
    pub line: (TexId, u32, u32),
}

pub struct Polaroids {
    pub entries: Vec<StateEntry>,
    pub index: usize,
    faces: Vec<TexId>,
    title: Option<TexId>,
    hint_faces: Vec<TexId>,
    pub confirm: Option<ConfirmFaces>,
}

impl Polaroids {
    pub fn new(entries: Vec<StateEntry>) -> Self {
        Polaroids {
            entries,
            index: 0,
            faces: Vec::new(),
            title: None,
            hint_faces: Vec::new(),
            confirm: None,
        }
    }

    pub fn set_faces(&mut self, faces: Vec<TexId>) {
        self.faces = faces;
    }

    pub fn set_title_face(&mut self, face: Option<TexId>) {
        self.title = face;
    }

    pub fn set_hint_faces(&mut self, faces: Vec<TexId>) {
        self.hint_faces = faces;
    }

    pub fn hints(&self) -> Vec<Hint> {
        hint_row(&LEGEND)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn selected(&self) -> Option<&StateEntry> {
        self.entries.get(self.index)
    }

    pub fn dots(&self) -> (usize, usize) {
        (self.entries.len(), self.index)
    }

    pub fn title(&self, now: &str) -> String {
        match self.selected() {
            Some(e) => Self::relative_time(&e.stamp, now),
            None => String::new(),
        }
    }

    pub fn remove_selected(&mut self) {
        if self.index >= self.entries.len() {
            return;
        }
        self.entries.remove(self.index);
        if self.index < self.faces.len() {
            self.faces.remove(self.index);
        }
        self.index = self.index.min(self.entries.len().saturating_sub(1));
    }

    pub fn left(&mut self) {
        self.index = self.index.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.index = (self.index + 1).min(self.entries.len().saturating_sub(1));
    }

    pub fn draw(
        &self,
        battery: Option<Battery>,
        battery_percent: Printed,
        bolt: Option<TexId>,
        clock: Printed,
        wifi: Option<TexId>,
        out: &mut Vec<Draw>,
    ) {
        self.draw_photo(out);
        self.draw_top(battery, battery_percent, bolt, clock, wifi, out);
        self.draw_bottom(out);
    }

    fn draw_photo(&self, out: &mut Vec<Draw>) {
        let (w, h) = (OUT_W as f32, OUT_H as f32);
        out.push(match self.faces.get(self.index) {
            Some(tex) => Draw::Shot { tex: *tex },
            None => Draw::Rect {
                x: 0.0,
                y: 0.0,
                w,
                h,
                colour: [
                    BLANK[0] as f32 / 255.0,
                    BLANK[1] as f32 / 255.0,
                    BLANK[2] as f32 / 255.0,
                    1.0,
                ],
            },
        });
    }

    fn draw_top(
        &self,
        battery: Option<Battery>,
        battery_percent: Printed,
        bolt: Option<TexId>,
        clock: Printed,
        wifi: Option<TexId>,
        out: &mut Vec<Draw>,
    ) {
        out.push(plate(0.0));
        if let Some(tex) = self.title {
            out.push(Draw::Tex {
                x: (OUT_W as f32 - TITLE_W as f32) / 2.0,
                y: (PLATE_H - TITLE_H as f32) / 2.0,
                w: TITLE_W as f32,
                h: TITLE_H as f32,
                tex,
                alpha: 1.0,
            });
        }

        draw_corners(
            slot_gfx::Canvas::FRAMED,
            battery,
            battery_percent,
            bolt,
            clock,
            wifi,
            out,
        );
    }

    fn draw_bottom(&self, out: &mut Vec<Draw>) {
        let top = OUT_H as f32 - PLATE_H;
        out.push(plate(top));
        let y = top + (PLATE_H - HINT_H as f32) / 2.0;
        if let Some(c) = self.confirm {
            let white = [1.0, 1.0, 1.0, 1.0];
            let tint = |tex, x: f32, y: f32, w: u32, h: u32| Draw::Tint {
                x: x.round(),
                y: y.round(),
                w: w as f32,
                h: h as f32,
                tex,
                colour: white,
            };
            out.push(tint(c.cancel.0, MARGIN, y, c.cancel.1, HINT_H));
            let dx = OUT_W as f32 - MARGIN - c.delete.1 as f32;
            out.push(tint(c.delete.0, dx, y, c.delete.1, HINT_H));
            let lx = (OUT_W as f32 - c.line.1 as f32) / 2.0;
            out.push(tint(c.line.0, lx, top + (PLATE_H - c.line.2 as f32) / 2.0, c.line.1, c.line.2));
            return;
        }
        let hints = self.hints();
        let widths: Vec<f32> = hints
            .iter()
            .map(|h| hint_width(h.key, &h.label) as f32)
            .collect();
        let tail = &widths[WAYS_OUT..];
        let tail_w = tail.iter().sum::<f32>() + HINT_GAP * (tail.len() - 1) as f32;
        let mut x = MARGIN;
        for (i, w) in widths.iter().enumerate() {
            if i == WAYS_OUT {
                x = OUT_W as f32 - MARGIN - tail_w;
            }
            out.push(hint_quad(x, y, *w, self.hint_faces.get(i).copied()));
            x += w + HINT_GAP;
        }
        self.draw_dots(top, out);
    }

    fn draw_dots(&self, top: f32, out: &mut Vec<Draw>) {
        let n = self.entries.len();
        let row = n as f32 * DOT + (n as f32 - 1.0).max(0.0) * DOT_GAP;
        let mut x = (OUT_W as f32 - row) / 2.0;
        for i in 0..n {
            out.push(Draw::Rect {
                x,
                y: top + (PLATE_H - DOT) / 2.0,
                w: DOT,
                h: DOT,
                colour: [1.0, 1.0, 1.0, if i == self.index { 1.0 } else { DOT_DIM }],
            });
            x += DOT + DOT_GAP;
        }
    }

    pub fn relative_time(stamp: &str, now: &str) -> String {
        const RELATIVE_WINDOW: i64 = 12 * 3600;
        let (Some(then), Some(parsed)) = (parse_stamp(stamp), parse_stamp(now)) else {
            return stamp.to_string();
        };
        let delta = parsed - then;
        if delta < 60 {
            return "just now".into();
        }
        if delta < 3600 {
            return format!("{} min ago", delta / 60);
        }
        if delta < RELATIVE_WINDOW {
            return format!("{} hr ago", delta / 3600);
        }
        format!("{} {}:{}", &stamp[..10], &stamp[11..13], &stamp[14..16])
    }
}

fn plate(y: f32) -> Draw {
    Draw::Rect {
        x: 0.0,
        y,
        w: OUT_W as f32,
        h: PLATE_H,
        colour: PLATE,
    }
}
