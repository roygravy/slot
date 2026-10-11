use crate::surface::{OUT_H, OUT_W};

const LINE_PX: f32 = 2.0;

pub const GAME_H: f32 = (OUT_W as f32 * 2.0 / 3.0 / 2.0).floor() * 2.0;

const DOT_T: f32 = 0.12;

const OVERSHOOT: f32 = 0.6;

pub fn screen_scale(t: f32) -> f32 {
    let left = 1.0 - t.clamp(0.0, 1.0);
    1.0 - (1.0 - LINE_PX / GAME_H) * left * left
}

pub fn screen_width(t: f32) -> f32 {
    let left = 1.0 - (t.clamp(0.0, 1.0) / DOT_T).min(1.0);
    1.0 - (1.0 - LINE_PX / OUT_W as f32) * left
}

pub fn screen_brightness(t: f32) -> f32 {
    let left = 1.0 - t.clamp(0.0, 1.0);
    1.0 + OVERSHOOT * left * left
}

pub fn screen_rect(t: f32) -> (f32, f32, f32, f32) {
    let w = OUT_W as f32 * screen_width(t);
    let h = GAME_H * screen_scale(t);
    ((OUT_W as f32 - w) / 2.0, (OUT_H as f32 - h) / 2.0, w, h)
}
