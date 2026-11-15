const WARMEST: [f32; 3] = [1.0, 0.82, 0.62];

pub const BLUE_LIGHT_MAX: u8 = 9;

/// Software dimming below the lowest backlight step: 0 is none, 6 the darkest.
const DIM_GAINS: [f32; 7] = [1.0, 0.8, 0.65, 0.52, 0.42, 0.33, 0.25];

pub fn dim_gain(step: u8) -> f32 {
    DIM_GAINS[usize::from(step).min(DIM_GAINS.len() - 1)]
}

pub fn blue_light_gain(step: u8) -> [f32; 3] {
    let t = step.min(BLUE_LIGHT_MAX) as f32 / BLUE_LIGHT_MAX as f32;
    let mut gain = [1.0f32; 3];
    for (g, warm) in gain.iter_mut().zip(WARMEST) {
        *g = 1.0 + (warm - 1.0) * t;
    }
    gain
}
