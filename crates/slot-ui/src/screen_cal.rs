//! General > Screen Calibration: the panel's colours set by eye on the console itself, no phone needed.
//! The test picture fills the screen; the tabs (the steps), the sliders, the help line and the
//! legend lie over it and fade away when no button is pressed, so the picture is judged whole.

use crate::draw::{Canvas, Draw, TexId, OUT_H};
use crate::plate::{
    arrows_hint_face, centred_hints, hint_face, keys_hint_face, tint_mask, UndoFace, HINT_H,
    LEGEND_GAP,
};
use crate::power_menu::MENU_H;
use crate::quick_menu::{help_line_face, quick_value_face, HELP_Y, LEGEND_Y, QUICK_EDGE};
use crate::text;
use crate::theme;

/// The steps, as the GRAVY Manager's Screen tab has them.
pub const CAL_STEPS: [&str; 6] = ["White", "Red", "Green", "Blue", "Check", "Color"];
pub const CAL_SLIDERS: [&str; 4] = ["Warmth", "Tint", "Curve", "Saturation"];
const CAL_HINTS: [&str; 6] = [
    "Slide until white looks like clean paper.",
    "Slide until the square melts into the lines.",
    "Slide until the square melts into the lines.",
    "Slide until the square melts into the lines.",
    "The grey square melts in and looks neutral.",
    "Colors pop, skin stays skin.",
];
const GLYPHS: &str = "0123456789+-.%";
const LABEL_PX: f32 = 26.0;

/// The top band holding the tabs.
pub const CAL_TOP: f32 = 74.0;
const TAB_Y: f32 = 22.0;
const TAB_PAD: f32 = 16.0;
const ROW_H: f32 = 44.0;
const PANEL_GAP: f32 = 12.0;
const TRACK_X: f32 = 268.0;
const TRACK_W: f32 = 230.0;
const KNOB: f32 = 18.0;
/// The ground under the bars: the menus' black, solid so the picture never shows through.
const GROUND: [f32; 4] = [5.0 / 255.0, 5.0 / 255.0, 5.0 / 255.0, 1.0];

/// Where the bars end and the picture shows on its own, for a step with `rows` sliders.
pub fn cal_panel_top(rows: usize) -> f32 {
    HELP_Y - PANEL_GAP - ROW_H * rows as f32 - if rows > 0 { PANEL_GAP } else { 0.0 }
}

pub struct ScreenCalUiFaces {
    pub tabs: Vec<UndoFace>,
    pub labels: Vec<UndoFace>,
    pub glyphs: Vec<UndoFace>,
    pub hints: Vec<UndoFace>,
    pub legend: Vec<UndoFace>,
}

pub struct ScreenCalFaces {
    pub tabs: Vec<(TexId, u32, u32)>,
    pub labels: Vec<(TexId, u32, u32)>,
    pub glyphs: Vec<(TexId, u32, u32)>,
    pub hints: Vec<(TexId, u32, u32)>,
    pub legend: Vec<(TexId, u32)>,
}

fn glyph_face(ch: char) -> UndoFace {
    let Some(font) = text::label_font() else {
        return UndoFace { rgba: vec![0; 4], w: 1, h: 1 };
    };
    let layout = text::fit(font, &ch.to_string(), 100.0, 1, LABEL_PX, LABEL_PX);
    let w = text::line_width(font, &layout.lines.concat(), layout.px, 0.0).ceil() as u32 + 2;
    let mut rgba = vec![0u8; (w * MENU_H * 4) as usize];
    text::draw_centred(&mut rgba, w, MENU_H, &layout, [0xff, 0xff, 0xff]);
    UndoFace { rgba, w, h: MENU_H }
}

pub fn screen_cal_faces() -> ScreenCalUiFaces {
    ScreenCalUiFaces {
        tabs: CAL_STEPS.iter().map(|s| crate::buttons_help::tab_face(s)).collect(),
        labels: CAL_SLIDERS.iter().map(|s| quick_value_face(s, false)).collect(),
        glyphs: GLYPHS.chars().map(glyph_face).collect(),
        hints: CAL_HINTS
            .iter()
            .map(|h| help_line_face(h).unwrap_or(UndoFace { rgba: vec![0; 4], w: 1, h: 1 }))
            .collect(),
        legend: vec![
            // Arrows first, then Save, Reset, Back, L1 R1 last: as on every menu.
            tint_mask(arrows_hint_face("Change")),
            tint_mask(hint_face("A", "Save")),
            // Y puts the chosen slider back to its standard.
            tint_mask(hint_face("Y", "Reset")),
            tint_mask(hint_face("B", "Back")),
            tint_mask(keys_hint_face(&["L1", "R1"], "Tab")),
        ],
    }
}

/// One slider: which label, where the knob sits and where its fill starts (0 to 1), and the
/// value as written beside it.
pub struct CalSlider {
    pub label: usize,
    pub at: f32,
    pub from: f32,
    pub value: String,
}

pub struct ScreenCalView<'a> {
    pub canvas: Canvas,
    pub step: usize,
    pub row: usize,
    pub sliders: Vec<CalSlider>,
    /// 1 while buttons are pressed, fading to 0 when they are left alone.
    pub shown: f32,
    pub faces: &'a ScreenCalFaces,
}

fn fade(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], c[3] * a]
}

impl ScreenCalView<'_> {
    pub fn draw(&self, out: &mut Vec<Draw>) {
        let a = self.shown;
        if a <= 0.0 {
            return;
        }
        let ink = theme::DARK;
        let w = self.canvas.w;
        let tint = |out: &mut Vec<Draw>, (tex, tw, th): (TexId, u32, u32), x: f32, y: f32, c| {
            out.push(Draw::Tint {
                x: x.round(),
                y: y.round(),
                w: tw as f32,
                h: th as f32,
                tex,
                colour: fade(c, a),
            })
        };
        // The tabs' band.
        out.push(Draw::Rect { x: 0.0, y: 0.0, w, h: CAL_TOP, colour: fade(GROUND, a) });
        let total: f32 = self.faces.tabs.iter().map(|t| t.1 as f32 + 2.0 * TAB_PAD).sum();
        let mut x = self.canvas.centred(total).round();
        for (i, &tab) in self.faces.tabs.iter().enumerate() {
            let cell = tab.1 as f32 + 2.0 * TAB_PAD;
            let lit = i == self.step;
            if lit {
                out.push(Draw::Rect { x, y: TAB_Y, w: cell, h: tab.2 as f32, colour: fade(ink.bar, a) });
            }
            tint(out, tab, x + TAB_PAD, TAB_Y, if lit { ink.on_bar } else { ink.dim });
            x += cell;
        }
        // The sliders, the help line and the legend.
        let top = cal_panel_top(self.sliders.len());
        out.push(Draw::Rect { x: 0.0, y: top, w, h: OUT_H as f32 - top, colour: fade(GROUND, a) });
        let right = w - QUICK_EDGE;
        for (i, s) in self.sliders.iter().enumerate() {
            let y = top + PANEL_GAP + ROW_H * i as f32;
            let lit = i == self.row;
            if lit {
                out.push(Draw::Rect { x: 0.0, y, w, h: ROW_H, colour: fade(ink.bar, a) });
            }
            let c = if lit { ink.on_bar } else { ink.dim };
            let face_y = y + (ROW_H - MENU_H as f32) / 2.0;
            if let Some(&label) = self.faces.labels.get(s.label) {
                // The label faces carry the menus' padding on both sides.
                tint(out, label, QUICK_EDGE - crate::MENU_PAD as f32, face_y, c);
            }
            // The track, its fill from the neutral point to the knob, and the knob.
            let mid = (y + ROW_H / 2.0).round();
            let track = if lit { fade(theme::DARK.on_bar, 0.25) } else { fade(ink.dim, 0.45) };
            out.push(Draw::Rect { x: TRACK_X, y: mid - 2.0, w: TRACK_W, h: 4.0, colour: fade(track, a) });
            let (kx, fx) = (TRACK_X + TRACK_W * s.at, TRACK_X + TRACK_W * s.from);
            out.push(Draw::Rect {
                x: kx.min(fx).round(),
                y: mid - 2.0,
                w: (kx - fx).abs().round(),
                h: 4.0,
                colour: fade(c, a),
            });
            crate::quick_menu::round_dot(out, (kx - KNOB / 2.0).round(), mid - KNOB / 2.0, KNOB, fade(c, a));
            if lit {
                crate::quick_menu::round_dot(out, (kx - 4.0).round(), mid - 4.0, 8.0, fade(ink.bar, a));
            }
            self.number(out, &s.value, right, face_y, fade(c, a));
        }
        if let Some(&(tex, tw, th)) = self.faces.hints.get(self.step) {
            let x = self.canvas.centred(tw as f32).round();
            out.push(Draw::Tint { x, y: HELP_Y.round(), w: tw as f32, h: th as f32, tex, colour: fade(ink.dim, a) });
        }
        // Check has nothing to slide: no Change, no Reset.
        let legend: Vec<_> = self
            .faces
            .legend
            .iter()
            .enumerate()
            .filter(|(i, _)| !matches!(i, 0 | 2) || !self.sliders.is_empty())
            .map(|(_, &h)| h)
            .collect();
        for (tex, hw, x) in centred_hints(self.canvas, &legend, LEGEND_GAP) {
            out.push(Draw::Tint {
                x: x.round(),
                y: LEGEND_Y.round(),
                w: hw as f32,
                h: HINT_H as f32,
                tex,
                colour: fade(ink.text, a),
            });
        }
    }

    /// A value, set right-aligned at `right` from the digit faces.
    fn number(&self, out: &mut Vec<Draw>, value: &str, right: f32, y: f32, colour: [f32; 4]) {
        let track = (LABEL_PX * 0.10).round();
        let faces: Vec<_> = value
            .chars()
            .filter_map(|c| GLYPHS.find(c).and_then(|i| self.faces.glyphs.get(i)))
            .collect();
        let total: f32 = faces.iter().map(|g| g.1 as f32 - 2.0 + track).sum::<f32>() - track;
        let mut x = right - total;
        for &&(tex, gw, gh) in &faces {
            out.push(Draw::Tint { x: (x - 1.0).round(), y: y.round(), w: gw as f32, h: gh as f32, tex, colour });
            x += gw as f32 - 2.0 + track;
        }
    }
}
