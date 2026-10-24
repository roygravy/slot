//! The About page: the back of an original Game Boy with the credits moulded into the plastic
//! (drawn by tools/about_art.py) and this build's serial printed on its paper label.

use crate::art::decode_png_bytes;
use crate::draw::{Canvas, Draw, TexId, OUT_H, OUT_W};
use crate::plate::UndoFace;
use crate::text;

const DARK_PNG: &[u8] = include_bytes!("../../../assets/about_dark.png");
const LIGHT_PNG: &[u8] = include_bytes!("../../../assets/about_light.png");

/// The paper label, as tools/about_art.py draws it.
const LABEL: (u32, u32, u32, u32) = (200, 34, 440, 92);
const SERIAL_PX: f32 = 34.0;
const INK: [u8; 3] = [0x1c, 0x1c, 0x1e];
/// The sticker's tilt, as tools/about_art.py sticks it on (degrees, PIL's sense:
/// positive turns anticlockwise).
const STICKER_DEG: f32 = -0.9;

/// The whole About screen for a theme, with `serial` printed on the label.
pub fn about_face(light: bool, serial: &str) -> UndoFace {
    let png = if light { LIGHT_PNG } else { DARK_PNG };
    let (mut rgba, w, h) = decode_png_bytes(png)
        .unwrap_or_else(|| (vec![0; (OUT_W * OUT_H * 4) as usize], OUT_W, OUT_H));
    if let Some(font) = text::serial_font() {
        let (lx, ly, lx1, ly1) = LABEL;
        let (lw, lh) = (lx1 - lx, ly1 - ly);
        let layout = text::fit(font, serial, lw as f32 - 32.0, 1, SERIAL_PX, 14.0);
        let mut label = vec![0u8; (lw * lh * 4) as usize];
        text::draw_centred_in(font, &mut label, lw, lh, &layout, INK);
        // Print it turned with the sticker: map each screen pixel back into the flat label.
        let (sin, cos) = STICKER_DEG.to_radians().sin_cos();
        let (cx, cy) = (lw as f32 / 2.0, lh as f32 / 2.0);
        let alpha_at = |x: i32, y: i32| -> f32 {
            if x < 0 || y < 0 || x >= lw as i32 || y >= lh as i32 {
                return 0.0;
            }
            label[((y as u32 * lw + x as u32) * 4 + 3) as usize] as f32
        };
        for y in 0..lh {
            for x in 0..lw {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let sx = cos * dx - sin * dy + cx - 0.5;
                let sy = sin * dx + cos * dy + cy - 0.5;
                let (fx, fy) = (sx.floor(), sy.floor());
                let (tx, ty) = (sx - fx, sy - fy);
                let (ix, iy) = (fx as i32, fy as i32);
                let a = alpha_at(ix, iy) * (1.0 - tx) * (1.0 - ty)
                    + alpha_at(ix + 1, iy) * tx * (1.0 - ty)
                    + alpha_at(ix, iy + 1) * (1.0 - tx) * ty
                    + alpha_at(ix + 1, iy + 1) * tx * ty;
                let a = (a * 0.92).round() as u32;
                if a == 0 || x + lx >= w || y + ly >= h {
                    continue;
                }
                let d = (((y + ly) * w + x + lx) * 4) as usize;
                for c in 0..3 {
                    rgba[d + c] =
                        ((INK[c] as u32 * a + rgba[d + c] as u32 * (255 - a)) / 255) as u8;
                }
            }
        }
    }
    UndoFace { rgba, w, h }
}

pub fn draw_about(canvas: Canvas, face: Option<TexId>, out: &mut Vec<Draw>) {
    let Some(tex) = face else {
        return;
    };
    out.push(Draw::Part {
        x: 0.0,
        y: 0.0,
        w: canvas.w,
        h: OUT_H as f32,
        tex,
        alpha: 1.0,
        uv: canvas.window(OUT_W as f32),
    });
}
