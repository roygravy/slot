use crate::lcd3x::ScreenEffect;
use crate::power::{gba_custom_game_rect, integer_game_rect, screen_brightness, screen_rect};
use crate::quad::Quad;
use crate::shaders::{GAME_FRAG, RECT_VERT};
use crate::surface::{GfxError, OUT_H, OUT_W};

pub const SRC_W: u32 = 240;
pub const SRC_H: u32 = 160;

pub const WHOLE_TEXTURE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

pub struct GamePass {
    prog: gl::types::GLuint,
    game: gl::types::GLuint,
    u_rect: gl::types::GLint,
    u_bright: gl::types::GLint,
    u_uv: gl::types::GLint,
    u_mode: gl::types::GLint,
    u_paper_size: gl::types::GLint,
    u_fbo: gl::types::GLint,
    u_bg: gl::types::GLint,
    u_picture_dim: gl::types::GLint,
    u_grid_opacity: gl::types::GLint,
    u_pixel_transparency: gl::types::GLint,
    u_lcd_overlay_size: gl::types::GLint,
    fbo_scale: [f32; 2],
    u_origin: gl::types::GLint,
    origin: [f32; 2],
    u_pic: gl::types::GLint,
    pic: [f32; 4],
    sample_background: bool,
    picture_dim: f32,
    grid_opacity: f32,
    pixel_transparency: bool,
    paper: gl::types::GLuint,
    paper_size: f32,
    lcd_overlay: gl::types::GLuint,
    lcd_overlay_size: [f32; 2],
    effect: ScreenEffect,
    power: f32,
    fills: bool,
    src: [f32; 4],
    background: [f32; 3],
    bg_sample_countdown: u8,
}

impl GamePass {
    pub fn new() -> Result<Self, GfxError> {
        let prog = crate::shaders::program(RECT_VERT, GAME_FRAG)?;
        let game = crate::gl::texture(SRC_W, SRC_H, gl::NEAREST, gl::CLAMP_TO_EDGE, gl::BGRA, None);
        let paper = crate::gl::texture(
            1,
            1,
            gl::NEAREST,
            gl::REPEAT,
            crate::gl::gray_format(),
            Some(&[255]),
        );
        let lcd_overlay = crate::gl::texture(
            1,
            1,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            gl::RGBA,
            Some(&[0, 0, 0, 0]),
        );
        let (
            u_rect,
            u_bright,
            u_uv,
            u_mode,
            u_paper_size,
            u_pic,
            u_fbo,
            u_bg,
            u_picture_dim,
            u_grid_opacity,
            u_pixel_transparency,
            u_lcd_overlay,
            u_lcd_overlay_size,
            u_origin,
        );
        unsafe {
            gl::UseProgram(prog);
            gl::Uniform1i(crate::gl::uniform_location(prog, "u_game"), 0);
            gl::Uniform2f(
                crate::gl::uniform_location(prog, "u_src"),
                SRC_W as f32,
                SRC_H as f32,
            );
            gl::Uniform2f(
                crate::gl::uniform_location(prog, "u_target"),
                OUT_W as f32,
                OUT_H as f32,
            );
            gl::Uniform2f(
                crate::gl::uniform_location(prog, "u_output_size"),
                OUT_W as f32,
                OUT_H as f32,
            );
            u_rect = crate::gl::uniform_location(prog, "u_rect");
            u_bright = crate::gl::uniform_location(prog, "u_bright");
            u_uv = crate::gl::uniform_location(prog, "u_uv");
            gl::Uniform1i(crate::gl::uniform_location(prog, "u_paper"), 1);
            u_mode = crate::gl::uniform_location(prog, "u_mode");
            u_paper_size = crate::gl::uniform_location(prog, "u_paper_size");
            u_fbo = crate::gl::uniform_location(prog, "u_fbo");
            u_pic = crate::gl::uniform_location(prog, "u_pic");
            u_origin = crate::gl::uniform_location(prog, "u_origin");
            u_bg = crate::gl::uniform_location(prog, "u_bg");
            u_picture_dim = crate::gl::uniform_location(prog, "u_picture_dim");
            u_grid_opacity = crate::gl::uniform_location(prog, "u_grid_opacity");
            u_pixel_transparency = crate::gl::uniform_location(prog, "u_pixel_transparency");
            u_lcd_overlay = crate::gl::uniform_location(prog, "u_lcd_overlay");
            u_lcd_overlay_size = crate::gl::uniform_location(prog, "u_lcd_overlay_size");
            gl::Uniform1i(u_lcd_overlay, 2);
        }
        Ok(GamePass {
            prog,
            game,
            u_rect,
            u_bright,
            u_uv,
            u_mode,
            u_paper_size,
            u_fbo,
            u_bg,
            u_picture_dim,
            u_grid_opacity,
            u_pixel_transparency,
            u_lcd_overlay_size,
            fbo_scale: [1.0, 1.0],
            u_origin,
            origin: [0.0, 0.0],
            u_pic,
            pic: WHOLE_TEXTURE,
            sample_background: false,
            picture_dim: 0.0,
            grid_opacity: 0.3,
            pixel_transparency: false,
            paper,
            paper_size: 1.0,
            lcd_overlay,
            lcd_overlay_size: [1.0, 1.0],
            effect: ScreenEffect::Lcd3x,
            power: 1.0,
            fills: false,
            src: WHOLE_TEXTURE,
            background: [0.0, 0.0, 0.0],
            bg_sample_countdown: 0,
        })
    }

    pub fn set_power(&mut self, t: f32) {
        self.power = t.clamp(0.0, 1.0);
    }

    pub fn frame(&self) -> (f32, f32, f32, f32) {
        match self.fills {
            true => (0.0, 0.0, OUT_W as f32, OUT_H as f32),
            false => screen_rect(self.power),
        }
    }

    pub fn set_fills_frame(&mut self, on: bool) {
        self.fills = on;
    }

    pub fn set_picture(&mut self, rect: [f32; 4]) {
        self.pic = rect;
    }

    pub fn set_picture_appearance(&mut self, sample_background: bool, dim: f32) {
        if self.sample_background != sample_background {
            self.background = [0.0, 0.0, 0.0];
            self.bg_sample_countdown = 0;
        }
        self.sample_background = sample_background;
        self.picture_dim = dim.clamp(0.0, 1.0);
    }

    pub fn set_grid_opacity(&mut self, opacity: f32) {
        self.grid_opacity = opacity.clamp(0.0, 1.0);
    }

    pub fn set_pixel_transparency(&mut self, enabled: bool) {
        self.pixel_transparency = enabled;
    }

    pub fn set_fbo_scale(&mut self, x: f32, y: f32) {
        self.fbo_scale = [x, y];
    }

    pub fn set_origin(&mut self, x: f32, y: f32) {
        self.origin = [x, y];
    }

    pub fn set_effect(&mut self, effect: ScreenEffect) {
        self.effect = effect;
    }

    pub fn set_paper(&mut self, size: u32, gray: &[u8]) {
        let tex = crate::gl::texture(
            size,
            size,
            gl::NEAREST,
            gl::REPEAT,
            crate::gl::gray_format(),
            Some(gray),
        );
        unsafe { gl::DeleteTextures(1, &self.paper) };
        self.paper = tex;
        self.paper_size = size as f32;
    }

    pub fn set_lcd_overlay(&mut self, w: u32, h: u32, rgba: &[u8]) {
        let stride = w as usize * 4;
        if w == 0 || h == 0 || rgba.len() < stride * h as usize {
            return;
        }
        let mut bottom_up = Vec::with_capacity(stride * h as usize);
        for row in rgba[..stride * h as usize].chunks_exact(stride).rev() {
            bottom_up.extend_from_slice(row);
        }
        let tex = crate::gl::texture(
            w,
            h,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            gl::RGBA,
            Some(&bottom_up),
        );
        unsafe { gl::DeleteTextures(1, &self.lcd_overlay) };
        self.lcd_overlay = tex;
        self.lcd_overlay_size = [w as f32, h as f32];
    }

    pub fn set_source_rect(&mut self, rect: [f32; 4]) {
        self.src = rect;
    }

    pub fn upload(&mut self, xrgb8888: &[u8]) {
        if xrgb8888.len() < (SRC_W * SRC_H * 4) as usize {
            return;
        }
        if self.sample_background && self.pic != WHOLE_TEXTURE {
            if self.bg_sample_countdown == 0 {
                self.background = palette_lightest_color(xrgb8888, self.pic);
                self.bg_sample_countdown = 7;
            } else {
                self.bg_sample_countdown -= 1;
            }
        }
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.game);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                SRC_W as i32,
                SRC_H as i32,
                gl::BGRA,
                gl::UNSIGNED_BYTE,
                xrgb8888.as_ptr() as *const std::ffi::c_void,
            );
        }
    }

    pub fn draw(&self, quad: &Quad) {
        let src = match self.effect {
            ScreenEffect::Grid => whole_scale_source(self.src, self.fbo_scale),
            _ => self.src,
        };
        self.draw_source(self.game, quad, src);
    }

    pub fn draw_still(&self, tex: gl::types::GLuint, quad: &Quad) {
        self.draw_source(tex, quad, WHOLE_TEXTURE);
    }

    fn draw_source(&self, tex: gl::types::GLuint, quad: &Quad, src: [f32; 4]) {
        let (x, y, w, h, bright) = if self.fills {
            (0.0, 0.0, OUT_W as f32, OUT_H as f32, 1.0)
        } else if src == WHOLE_TEXTURE && self.pic == WHOLE_TEXTURE {
            let (x, y, w, h) = gba_custom_game_rect(self.power);
            (x, y, w, h, screen_brightness(self.power))
        } else if src == WHOLE_TEXTURE {
            let (x, y, w, h) = integer_game_rect(
                self.power,
                [src[2] * SRC_W as f32, src[3] * SRC_H as f32],
                [
                    (self.pic[2] - self.pic[0]) * SRC_W as f32,
                    (self.pic[3] - self.pic[1]) * SRC_H as f32,
                ],
            );
            (x, y, w, h, screen_brightness(self.power))
        } else {
            let (x, y, w, h) = screen_rect(self.power);
            (x, y, w, h, screen_brightness(self.power))
        };
        unsafe {
            gl::UseProgram(self.prog);
            gl::Uniform4f(self.u_rect, x, y, w, h);
            gl::Uniform4f(self.u_uv, src[0], src[1], src[2], src[3]);
            gl::Uniform1f(self.u_bright, bright);
            gl::Uniform1f(self.u_mode, self.effect.mode());
            gl::Uniform1f(self.u_paper_size, self.paper_size);
            gl::Uniform2f(self.u_fbo, self.fbo_scale[0], self.fbo_scale[1]);
            gl::Uniform2f(self.u_origin, self.origin[0], self.origin[1]);
            gl::Uniform3f(
                self.u_bg,
                self.background[0],
                self.background[1],
                self.background[2],
            );
            gl::Uniform1f(self.u_picture_dim, self.picture_dim);
            gl::Uniform1f(self.u_grid_opacity, self.grid_opacity);
            gl::Uniform1f(
                self.u_pixel_transparency,
                if self.pixel_transparency { 1.0 } else { 0.0 },
            );
            gl::Uniform2f(
                self.u_lcd_overlay_size,
                self.lcd_overlay_size[0],
                self.lcd_overlay_size[1],
            );
            gl::Uniform4f(
                self.u_pic,
                self.pic[0],
                self.pic[1],
                self.pic[2],
                self.pic[3],
            );
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, self.paper);
            gl::ActiveTexture(gl::TEXTURE2);
            gl::BindTexture(gl::TEXTURE_2D, self.lcd_overlay);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, tex);
        }
        quad.draw();
    }
}

fn palette_lightest_color(pixels: &[u8], pic: [f32; 4]) -> [f32; 3] {
    const COLOR_BINS: usize = 1 << 15;
    let x0 = (pic[0].clamp(0.0, 1.0) * SRC_W as f32).floor() as usize;
    let y0 = (pic[1].clamp(0.0, 1.0) * SRC_H as f32).floor() as usize;
    let x1 = (pic[2].clamp(0.0, 1.0) * SRC_W as f32).ceil() as usize;
    let y1 = (pic[3].clamp(0.0, 1.0) * SRC_H as f32).ceil() as usize;
    let (x1, y1) = (x1.min(SRC_W as usize), y1.min(SRC_H as usize));
    if x0 >= x1 || y0 >= y1 {
        return [0.0, 0.0, 0.0];
    }

    let mut counts = [0u16; COLOR_BINS];
    for y in y0..y1 {
        for x in x0..x1 {
            let px = &pixels[(y * SRC_W as usize + x) * 4..][..3];
            let key = ((px[2] as usize >> 3) << 10)
                | ((px[1] as usize >> 3) << 5)
                | (px[0] as usize >> 3);
            counts[key] = counts[key].saturating_add(1);
        }
    }

    let min_count = (((x1 - x0) * (y1 - y0)) / 500).max(8) as u16;
    let brightest = counts
        .iter()
        .enumerate()
        .filter(|(_, count)| **count >= min_count)
        .max_by(|(a, count_a), (b, count_b)| {
            let luma = |key: usize| {
                let r = ((key >> 10) & 31) as f32 / 31.0;
                let g = ((key >> 5) & 31) as f32 / 31.0;
                let b = (key & 31) as f32 / 31.0;
                0.2126 * r + 0.7152 * g + 0.0722 * b
            };
            luma(*a)
                .total_cmp(&luma(*b))
                .then_with(|| count_a.cmp(count_b))
        })
        .map(|(key, _)| key)
        .or_else(|| counts.iter().position(|count| *count > 0));
    let Some(brightest) = brightest else {
        return [0.0, 0.0, 0.0];
    };

    for y in y0..y1 {
        for x in x0..x1 {
            let px = &pixels[(y * SRC_W as usize + x) * 4..][..3];
            let key = ((px[2] as usize >> 3) << 10)
                | ((px[1] as usize >> 3) << 5)
                | (px[0] as usize >> 3);
            if key == brightest {
                return [
                    px[2] as f32 / 255.0,
                    px[1] as f32 / 255.0,
                    px[0] as f32 / 255.0,
                ];
            }
        }
    }
    [0.0, 0.0, 0.0]
}

fn whole_scale_source(src: [f32; 4], fbo: [f32; 2]) -> [f32; 4] {
    let (x, w) = whole_scale(src[0], src[2], SRC_W as f32, OUT_W as f32 * fbo[0]);
    let (y, h) = whole_scale(src[1], src[3], SRC_H as f32, OUT_H as f32 * fbo[1]);
    [x, y, w, h]
}

fn whole_scale(at: f32, len: f32, texels: f32, out: f32) -> (f32, f32) {
    let shown = len * texels;
    let scale = (out / shown).ceil();
    let trim = scale * shown - out;
    if trim >= 0.1 * shown {
        return (at, len);
    }
    let top = (trim / 2.0).floor();
    (at + top / scale / texels, out / scale / texels)
}

impl Drop for GamePass {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.game);
            gl::DeleteTextures(1, &self.paper);
            gl::DeleteTextures(1, &self.lcd_overlay);
            gl::DeleteProgram(self.prog);
        }
    }
}
