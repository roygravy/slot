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
    screen_width_for(t, OUT_W as f32)
}

fn screen_width_for(t: f32, width: f32) -> f32 {
    let left = 1.0 - (t.clamp(0.0, 1.0) / DOT_T).min(1.0);
    1.0 - (1.0 - LINE_PX / width) * left
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

pub fn fullscreen_game_rect(t: f32) -> (f32, f32, f32, f32) {
    let w = OUT_W as f32 * screen_width_for(t, OUT_W as f32);
    let h = OUT_H as f32 * screen_scale_for(t, OUT_H as f32);
    ((OUT_W as f32 - w) / 2.0, (OUT_H as f32 - h) / 2.0, w, h)
}

pub fn integer_game_rect(
    t: f32,
    source_size: [f32; 2],
    picture_size: [f32; 2],
    fill_height: bool,
) -> (f32, f32, f32, f32) {
    if source_size
        .iter()
        .chain(picture_size.iter())
        .any(|n| *n <= 0.0)
    {
        return screen_rect(t);
    }

    // As tall as the screen: the ground round the picture runs past the edges.
    let scale = if fill_height {
        OUT_H as f32 / picture_size[1]
    } else {
        (OUT_W as f32 / picture_size[0])
            .floor()
            .min((OUT_H as f32 / picture_size[1]).floor())
            .max(1.0)
    };
    let source_w = source_size[0] * scale;
    let source_h = source_size[1] * scale;
    let w = source_w * screen_width_for(t, source_w);
    let h = source_h * screen_scale_for(t, source_h);
    ((OUT_W as f32 - w) / 2.0, (OUT_H as f32 - h) / 2.0, w, h)
}

pub fn gba_custom_game_rect(t: f32) -> (f32, f32, f32, f32) {
    // Match the user's RetroArch viewport: 640 × 478, centered in 640 × 480.
    const WIDTH: f32 = OUT_W as f32;
    const HEIGHT: f32 = (OUT_H - 2) as f32;
    let w = WIDTH * screen_width_for(t, WIDTH);
    let h = HEIGHT * screen_scale_for(t, HEIGHT);
    ((OUT_W as f32 - w) / 2.0, (OUT_H as f32 - h) / 2.0, w, h)
}

fn screen_scale_for(t: f32, height: f32) -> f32 {
    let left = 1.0 - t.clamp(0.0, 1.0);
    1.0 - (1.0 - LINE_PX / height) * left * left
}
