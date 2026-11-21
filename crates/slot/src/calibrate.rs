//! Setting the panel's colours by eye, on the console itself: General > Screen Calibration
//! fills the screen with a test picture and puts its sliders on the panel at once (`Console`).
//! A keeps them in `Config/display.cal`, which is put on the panel at every start.

use std::path::Path;

use slot_gfx::{calibration, saturation_default, Calibration, Draw, OUT_H, OUT_W};

/// What fills the screen while calibrating.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Picture {
    /// Full white, held beside the phone's white.
    White,
    /// Colour bars and a grey ramp, to judge the saturation by.
    Colours,
    /// Fine lines of a channel at full and black, round a solid patch that matches them in
    /// brightness when the curve is right. 0, 1, 2 are red, green, blue; 3 is grey.
    Lines(usize),
}

pub struct Calibrate {
    booted: bool,
    saturation: f32,
    /// What the final blit puts on the panel: white point, curves and contrast.
    shown: Calibration,
    /// Screen Calibration is open: its sliders are on the panel.
    console: Option<Console>,
    /// Switched off on General's Screen Calibration row: the panel as the console makes it,
    /// no table and no extra colour, to compare with. Screen Calibration itself still shows
    /// its sliders while open.
    off: bool,
}

/// Screen Calibration: the sliders, worked with the buttons. Values as GRAVY Manager kept them: warmth and tint -100 to 100, each curve as 100 x log2 of its exponent,
/// saturation in percent of the standard (`saturation_default()` reads 100%).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Console {
    /// White, Red, Green, Blue, Check, Color (`slot_ui::CAL_STEPS`).
    pub step: usize,
    /// The slider chosen on the step.
    pub row: usize,
    pub warm: i32,
    pub tint: i32,
    pub curve: [i32; 3],
    pub sat: i32,
    /// Contrast in whole steps, -4 to 4, 0 as the curve leaves it.
    pub contrast: i32,
    /// When a button was last pressed: the bars fade a while after.
    pub touched: u64,
}

/// One press moves a slider this far; held, it keeps going.
const WARM_STEP: i32 = 2;
const CURVE_STEP: i32 = 1;
const CONTRAST_STEP: i32 = 1;
/// The contrast slider, -4 to 4: big steps, easy to see (`slot_gfx` keeps -1 to 1).
const CONTRAST_MAX: i32 = 4;
/// The saturation slider's nine stops, -4 to 4, in percent of the standard: big steps, easy to
/// see, inside `slot_gfx::saturation_range()` (40 to 250), each about a quarter more than the last.
const SAT_STOPS: [i32; 9] = [40, 50, 64, 80, 100, 125, 156, 195, 250];
/// The stop that is the standard, shown as 0.
const SAT_ZERO: usize = 4;

/// The stop nearest a saturation in percent.
fn sat_stop(sat: i32) -> usize {
    (0..SAT_STOPS.len()).min_by_key(|&i| (SAT_STOPS[i] - sat).abs()).unwrap_or(SAT_ZERO)
}
/// How long the bars stay after the last press, and how long they take to fade.
const BARS_MS: u64 = 2000;
const BARS_FADE_MS: u64 = 300;

/// Which slider a step's row is: 0 Warmth, 1 Tint, 2 Curve, 3 Saturation, 4 Contrast.
pub fn sliders(step: usize) -> &'static [usize] {
    match step {
        0 => &[0, 1],
        1..=3 => &[2],
        5 => &[3, 4],
        _ => &[],
    }
}

/// The Manager's gains from warmth and tint (index.html `gains`), the brightest at 1.
fn gains(warm: i32, tint: i32) -> [f32; 3] {
    let (w, t) = (warm as f32 / 100.0, tint as f32 / 100.0);
    let mut g = [1.0 + w.min(0.0) * 0.4, 1.0 - w.abs() * 0.12, 1.0 - w.max(0.0) * 0.4];
    if t > 0.0 {
        g[1] *= 1.0 - t * 0.25;
    } else {
        g[0] *= 1.0 + t * 0.25;
        g[2] *= 1.0 + t * 0.25;
    }
    let top = g.iter().copied().fold(0.0, f32::max);
    g.map(|v| v / top)
}

impl Console {
    fn from(cal: &Calibration, now: u64) -> Console {
        // The saved gains back to the two sliders: the closest pair, as the Manager does.
        let (mut warm, mut tint, mut best) = (0, 0, f32::MAX);
        for w in (-100..=100).step_by(WARM_STEP as usize) {
            for t in (-100..=100).step_by(WARM_STEP as usize) {
                let g = gains(w, t);
                let e: f32 = (0..3).map(|i| (g[i] - cal.gain[i]).powi(2)).sum();
                if e < best {
                    (warm, tint, best) = (w, t, e);
                }
            }
        }
        Console {
            step: 0,
            row: 0,
            warm,
            tint,
            curve: cal.gamma.map(|g| (g.log2() * 100.0).round() as i32),
            sat: SAT_STOPS[sat_stop((cal.saturation / saturation_default() * 100.0).round() as i32)],
            contrast: (cal.contrast * CONTRAST_MAX as f32).round() as i32,
            touched: now,
        }
    }

    pub fn calibration(&self) -> Calibration {
        Calibration::parse(&format!(
            "gain {}\ngamma {}\nsaturation {}\ncontrast {}\n",
            gains(self.warm, self.tint).map(|v| format!("{v:.4}")).join(" "),
            self.curve.map(|c| format!("{:.4}", 2f32.powf(c as f32 / 100.0))).join(" "),
            self.sat as f32 / 100.0 * saturation_default(),
            self.contrast as f32 / CONTRAST_MAX as f32
        ))
    }

    pub fn slider(&self) -> Option<usize> {
        sliders(self.step).get(self.row).copied()
    }

    /// A slider's knob and the neutral point it is filled from (both 0 to 1), and its value
    /// as written beside it.
    pub fn shown(&self, slider: usize) -> (f32, f32, String) {
        let part = |v: i32, lo: i32, hi: i32| (v - lo) as f32 / (hi - lo) as f32;
        match slider {
            0 | 1 => {
                let v = if slider == 0 { self.warm } else { self.tint };
                let text = if v > 0 { format!("+{v}") } else { v.to_string() };
                (part(v, -100, 100), 0.5, text)
            }
            2 => {
                let c = self.curve[self.step.clamp(1, 3) - 1];
                (part(c, -100, 100), 0.5, format!("{:.2}", 2f32.powf(c as f32 / 100.0)))
            }
            3 => {
                let v = sat_stop(self.sat) as i32 - SAT_ZERO as i32;
                let text = if v > 0 { format!("+{v}") } else { v.to_string() };
                (part(v, -(SAT_ZERO as i32), SAT_ZERO as i32), 0.5, text)
            }
            _ => {
                let v = self.contrast;
                let text = if v > 0 { format!("+{v}") } else { v.to_string() };
                (part(v, -CONTRAST_MAX, CONTRAST_MAX), 0.5, text)
            }
        }
    }

    /// Left or right on the chosen slider.
    pub fn nudge(&mut self, dir: i32) {
        match self.slider() {
            Some(0) => self.warm = (self.warm + dir * WARM_STEP).clamp(-100, 100),
            Some(1) => self.tint = (self.tint + dir * WARM_STEP).clamp(-100, 100),
            Some(2) => {
                let c = &mut self.curve[self.step - 1];
                *c = (*c + dir * CURVE_STEP).clamp(-100, 100);
            }
            Some(3) => {
                let i = (sat_stop(self.sat) as i32 + dir).clamp(0, SAT_STOPS.len() as i32 - 1);
                self.sat = SAT_STOPS[i as usize];
            }
            Some(_) => self.contrast = (self.contrast + dir * CONTRAST_STEP).clamp(-CONTRAST_MAX, CONTRAST_MAX),
            None => {}
        }
    }

    /// Y: the chosen slider back to its standard (neutral, saturation 100%).
    pub fn reset(&mut self) {
        match self.slider() {
            Some(0) => self.warm = 0,
            Some(1) => self.tint = 0,
            Some(2) => self.curve[self.step - 1] = 0,
            Some(3) => self.sat = 100,
            Some(_) => self.contrast = 0,
            None => {}
        }
    }

    pub fn step_by(&mut self, dir: i32) {
        let n = slot_ui::CAL_STEPS.len() as i32;
        self.step = (self.step as i32 + dir).rem_euclid(n) as usize;
        self.row = 0;
    }

    pub fn row_by(&mut self, dir: i32) {
        let n = sliders(self.step).len();
        if n > 0 {
            self.row = (self.row as i32 + dir).clamp(0, n as i32 - 1) as usize;
        }
    }

    pub fn picture(&self) -> Picture {
        match self.step {
            0 => Picture::White,
            1..=3 => Picture::Lines(self.step - 1),
            4 => Picture::Lines(3),
            _ => Picture::Colours,
        }
    }

    /// How much the bars show: 1 just after a press, 0 once left alone.
    pub fn bars(&self, now: u64) -> f32 {
        let idle = now.saturating_sub(self.touched);
        match idle.checked_sub(BARS_MS) {
            None => 1.0,
            Some(past) => 1.0 - (past as f32 / BARS_FADE_MS as f32).min(1.0),
        }
    }
}

impl Default for Calibrate {
    fn default() -> Self {
        Calibrate {
            booted: false,
            saturation: saturation_default(),
            shown: Calibration { saturation: 1.0, ..Default::default() },
            console: None,
            off: false,
        }
    }
}

impl Calibrate {
    /// Colour of the whole screen, for the final blit. On Screen Calibration only the Color step
    /// shows it: the white, the lines and the grey are judged raw, or a richer saturation pushes
    /// the red, green and blue patches brighter than their lines and the curves no longer match.
    pub fn saturation(&self) -> f32 {
        match &self.console {
            Some(console) if console.picture() != Picture::Colours => 1.0,
            _ => self.saturation,
        }
    }

    fn put(&mut self, cal: &Calibration) {
        let bare = Calibration { saturation: 1.0, ..Default::default() };
        let cal = if self.off && self.console.is_none() { &bare } else { cal };
        self.saturation = cal.saturation;
        self.shown = *cal;
    }

    /// White point, curves and contrast for the final blit (its saturation is `saturation()`).
    pub fn shown(&self) -> &Calibration {
        &self.shown
    }

    pub fn picture(&self) -> Option<Picture> {
        self.console.as_ref().map(Console::picture)
    }

    /// Screen Calibration opens: the saved calibration onto its sliders.
    pub fn open_console(&mut self, root: Option<&Path>, now: u64) {
        let saved = root.map(calibration::load).unwrap_or_default();
        self.console = Some(Console::from(&saved, now));
    }

    pub fn console(&self) -> Option<&Console> {
        self.console.as_ref()
    }

    /// A button on Screen Calibration: `change` works the sliders, then they go on the panel.
    pub fn console_input(&mut self, now: u64, change: impl FnOnce(&mut Console)) {
        let Some(console) = &mut self.console else { return };
        let before = console.calibration();
        change(console);
        console.touched = now;
        let after = console.calibration();
        if after != before {
            self.put(&after);
        }
    }

    /// A on Screen Calibration: the sliders kept on the card; the screen stays open.
    pub fn save_console(&mut self, root: Option<&Path>) -> std::io::Result<()> {
        let (Some(console), Some(root)) = (&self.console, root) else { return Ok(()) };
        let cal = console.calibration();
        std::fs::create_dir_all(root.join("Config"))
            .and_then(|_| slot_store::atomic_write(&root.join("Config/display.cal"), cal.text().as_bytes()))
    }

    /// Screen Calibration closes: the saved calibration back on the panel (unsaved changes go).
    pub fn close_console(&mut self, root: Option<&Path>) {
        if self.console.take().is_none() {
            return;
        }
        if let Some(root) = root {
            let saved = calibration::load(root);
            self.put(&saved);
        }
    }

    /// General's Screen Calibration row switched: off shows the panel bare, on the saved
    /// calibration again.
    pub fn set_off(&mut self, off: bool, root: Option<&Path>) {
        if self.off == off {
            return;
        }
        self.off = off;
        if self.booted && self.console.is_none() {
            let saved = root.map(calibration::load).unwrap_or_default();
            self.put(&saved);
        }
    }

    /// Once per frame: the saved calibration on the panel at start.
    pub fn tick(&mut self, root: Option<&Path>) {
        let Some(root) = root else { return };
        if !self.booted {
            self.booted = true;
            let saved = calibration::load(root);
            // Builds before 2026-11-18 put the calibration in the display engine's own table,
            // which a restart of slot alone (an update) leaves switched on: off with it, or the
            // colours would be calibrated twice.
            if !(Calibration { saturation: saturation_default(), ..saved }).is_neutral() {
                if let Err(e) = calibration::release() {
                    eprintln!("slot: display calibration: old table not switched off: {e}");
                }
            }
            self.put(&saved);
        }
    }

    pub fn draw(&self, out: &mut Vec<Draw>, now: u64) {
        let Some(picture) = self.picture() else { return };
        let (w, h) = (OUT_W as f32, OUT_H as f32);
        // On Screen Calibration the picture's own parts sit between the bars while they show,
        // and spread over the whole screen as the bars fade away.
        let (y0, rh) = match &self.console {
            Some(c) => {
                let bars = c.bars(now);
                let top = slot_ui::cal_panel_top(sliders(c.step).len());
                let y0 = slot_ui::CAL_TOP * bars;
                let bottom = top * bars + h * (1.0 - bars);
                (y0, bottom - y0)
            }
            None => (0.0, h),
        };
        let rect = |x, y, w, h, c: [f32; 3]| Draw::Rect {
            x,
            y,
            w,
            h,
            colour: [c[0], c[1], c[2], 1.0],
        };
        match picture {
            Picture::White => {
                out.push(rect(0.0, 0.0, w, h, [1.0; 3]));
                // Greys under the white: a colour cast shows best in grey.
                let cw = (w / 3.0).ceil();
                for (i, v) in [0.25, 0.5, 0.75].iter().enumerate() {
                    out.push(rect(i as f32 * cw, (y0 + rh * 0.7).round(), cw, (rh * 0.3).round(), [*v; 3]));
                }
            }
            Picture::Colours => {
                out.push(rect(0.0, 0.0, w, h, [0.0; 3]));
                let bars = [
                    [0.9, 0.1, 0.1],
                    [0.95, 0.55, 0.1],
                    [0.95, 0.85, 0.15],
                    [0.15, 0.7, 0.25],
                    [0.1, 0.7, 0.8],
                    [0.15, 0.25, 0.85],
                    [0.7, 0.2, 0.75],
                ];
                let bw = (w / bars.len() as f32).ceil();
                for (i, c) in bars.iter().enumerate() {
                    out.push(rect(i as f32 * bw, y0, bw, rh * 0.55, *c));
                }
                let ramp = [[0.96, 0.8, 0.68], [0.8, 0.6, 0.48], [0.6, 0.6, 0.6], [0.4, 0.4, 0.4], [0.2, 0.2, 0.2], [0.05, 0.05, 0.05]];
                let rw = (w / ramp.len() as f32).ceil();
                for (i, c) in ramp.iter().enumerate() {
                    out.push(rect(i as f32 * rw, y0 + rh * 0.55, rw, rh * 0.15, *c));
                }
                // Steps just above black and just below white: each should stand apart from
                // its neighbour. Run together, the curve or the panel loses the shadows or
                // the highlights.
                for (row, steps) in [NEAR_BLACK, NEAR_WHITE].iter().enumerate() {
                    let sw = (w / steps.len() as f32).ceil();
                    let y = y0 + rh * (0.7 + 0.15 * row as f32);
                    for (i, v) in steps.iter().enumerate() {
                        out.push(rect(i as f32 * sw, y, sw, rh * 0.15, [*v as f32 / 255.0; 3]));
                    }
                }
            }
            Picture::Lines(ch) => {
                let tone = |v: f32| {
                    let mut c = [0.0; 3];
                    for (i, x) in c.iter_mut().enumerate() {
                        if ch == 3 || i == ch {
                            *x = v;
                        }
                    }
                    c
                };
                out.push(rect(0.0, 0.0, w, h, tone(0.0)));
                // Three columns, a quarter, half and three quarters of the rows lit, each
                // round a solid patch that matches the lines in brightness when the curve
                // is right at that level.
                for (i, (lit, period)) in LEVELS.iter().enumerate() {
                    let x0 = (w * i as f32 / 3.0).round();
                    let cw = (w * (i + 1) as f32 / 3.0).round() - x0;
                    for y in (0..OUT_H).step_by(*period as usize) {
                        out.push(rect(x0, y as f32, cw, *lit as f32, tone(1.0)));
                    }
                    let level = *lit as f32 / *period as f32;
                    let (pw, ph) = ((cw * 0.6).round(), (h * 0.36).round());
                    out.push(rect(
                        (x0 + (cw - pw) / 2.0).round(),
                        (y0 + (rh - ph) / 2.0).round(),
                        pw,
                        ph,
                        tone(level.powf(1.0 / TARGET_GAMMA)),
                    ));
                }
            }
        }
    }
}

/// The curve the Lines pictures aim at: half the light reads as 186 of 255.
const TARGET_GAMMA: f32 = 2.2;
/// Lit rows out of every so many: a quarter, half, three quarters.
const LEVELS: [(u32, u32); 3] = [(1, 4), (1, 2), (3, 4)];
const NEAR_BLACK: [u8; 8] = [0, 3, 6, 10, 14, 19, 24, 30];
const NEAR_WHITE: [u8; 8] = [225, 230, 235, 240, 244, 248, 252, 255];

#[cfg(test)]
mod tests {
    use super::*;
    use slot_gfx::saturation_range;

    #[test]
    fn the_test_pictures_are_judged_without_the_saturation() {
        let mut cal = Calibrate::default();
        cal.saturation = 1.6;
        cal.console = Some(Console::from(&Calibration { saturation: 1.6, ..Default::default() }, 0));
        for step in 0..slot_ui::CAL_STEPS.len() {
            cal.console.as_mut().unwrap().step = step;
            let want = if step == 5 { 1.6 } else { 1.0 };
            assert_eq!(cal.saturation(), want, "step {step}");
        }
        cal.console = None;
        assert_eq!(cal.saturation(), 1.6);
    }

    #[test]
    fn no_calibration_on_the_card_is_the_default_saturation_and_a_neutral_table() {
        let dir = std::env::temp_dir().join("slot-cal-default");
        let cal = calibration::load(&dir);
        assert_eq!(cal.saturation, saturation_default());
        assert!(cal.is_neutral());
        let console = Console::from(&cal, 0);
        assert_eq!(console.sat, 100);
        assert_eq!(console.calibration(), cal);
    }

    #[test]
    fn a_saved_saturation_reads_against_the_standard() {
        let c = Console::from(&Calibration { saturation: 1.6, ..Default::default() }, 0);
        // 1.6 is 160% of the standard: the nearest stop, 156%.
        assert_eq!(c.sat, 156);
    }

    #[test]
    fn the_saturation_slider_has_nine_stops_in_the_cards_range() {
        let mut c = Console::from(&Calibration::default(), 0);
        c.step = 5;
        for _ in 0..100 {
            c.nudge(1);
        }
        assert_eq!(c.sat, 250);
        assert!(c.calibration().saturation <= saturation_range().1 + 1e-3);
        for _ in 0..100 {
            c.nudge(-1);
        }
        assert_eq!(c.sat, 40);
        assert!(c.calibration().saturation >= saturation_range().0 - 1e-3);
    }

    #[test]
    fn reset_puts_only_the_chosen_slider_back() {
        let mut c = Console::from(&Calibration { saturation: 1.6, ..Default::default() }, 0);
        c.warm = 20;
        c.step = 5;
        c.reset();
        assert_eq!((c.sat, c.warm), (100, 20));
        c.step = 2;
        c.curve = [10, 20, 30];
        c.reset();
        assert_eq!(c.curve, [10, 0, 30]);
    }

}
