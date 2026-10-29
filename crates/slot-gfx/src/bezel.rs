use crate::surface::{OUT_H, OUT_W};

/// A panel wider than 4:3, as the RG34XX's 720x480: the shelf is drawn across all of it, a
/// game in the 4:3 picture in its middle.
pub fn framed(panel: (u32, u32)) -> bool {
    panel.1 > 0 && panel.0 * OUT_H > panel.1 * OUT_W
}

pub fn screen_rect(panel: (u32, u32)) -> (i32, i32, i32, i32) {
    let h = panel.1;
    let w = (h * OUT_W + OUT_H / 2) / OUT_H;
    ((panel.0 - w) as i32 / 2, 0, w as i32, h as i32)
}

pub fn lifted_rect(panel: (u32, u32), lift: f32) -> (i32, i32, i32, i32) {
    let (x, _, w, h) = screen_rect(panel);
    let t = lift.clamp(0.0, 1.0);
    let eased = t * t * (3.0 - 2.0 * t);
    let drop = ((panel.1 as i32 - h) as f32 * (1.0 - eased)).round() as i32;
    (x, drop, w, h)
}
