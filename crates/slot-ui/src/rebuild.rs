use slot_gfx::{Canvas, Draw, TexId};

use crate::hud::{FILL, HUD_INK, PLATE, PLATE_H};
use crate::icon::{haloed, HALO_PX};
use crate::text;
use crate::CartFace;

const LINE_W: u32 = 400;
const LINE_H: u32 = 22;
const LINE_PX: f32 = 16.0;
const LINE_MIN_PX: f32 = 12.0;
const PROGRESS_H: f32 = 2.0;

/// The words on the banner, set as the toasts are: the same size, ink and halo.
pub fn rebuild_banner_face(text: &str) -> CartFace {
    let Some(font) = text::label_font() else {
        return CartFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let layout = text::fit(font, text, LINE_W as f32, 1, LINE_PX, LINE_MIN_PX);
    let cov = text::coverage(LINE_W, LINE_H, &layout);
    haloed(&cov, LINE_W, LINE_H, HUD_INK)
}

pub fn rebuild_banner_box() -> (u32, u32) {
    (LINE_W + 2 * HALO_PX, LINE_H + 2 * HALO_PX)
}

/// New labels being cached while the shelf is in use: the toasts' black bar along the
/// top, with the count in it and a thin line along its foot that fills as they are done.
pub struct RebuildBanner {
    pub line: Option<TexId>,
    pub fraction: f32,
}

impl RebuildBanner {
    pub fn draw(&self, canvas: Canvas, out: &mut Vec<Draw>) {
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: canvas.w,
            h: PLATE_H,
            colour: PLATE,
        });
        if let Some(tex) = self.line {
            let (w, h) = rebuild_banner_box();
            let (w, h) = (w as f32, h as f32);
            out.push(Draw::Tex {
                x: canvas.centred(w),
                y: (PLATE_H - h) / 2.0,
                w,
                h,
                tex,
                alpha: 1.0,
            });
        }
        out.push(Draw::Rect {
            x: 0.0,
            y: PLATE_H - PROGRESS_H,
            w: canvas.w * self.fraction.clamp(0.0, 1.0),
            h: PROGRESS_H,
            colour: FILL,
        });
    }
}
