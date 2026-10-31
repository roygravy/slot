use crate::lcd3x::ScreenEffect;
use crate::power::{
    fullscreen_game_rect, gba_custom_game_rect, integer_game_rect, screen_brightness, screen_rect,
};
use crate::quad::Quad;
use crate::shaders::{GAME_FRAG, RECT_VERT};
use crate::surface::{GfxError, OUT_H, OUT_W};

pub const SRC_W: u32 = 240;
pub const SRC_H: u32 = 160;

pub const WHOLE_TEXTURE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

/// The drop shadow pixels cast with Pixel Transparency on, as it has always been: how dark,
/// and how soft.
pub const PALE_OPACITY: f32 = 0.62;
pub const PIXEL_SHADOW: [f32; 3] = [0.16, 0.55, 0.0];

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
    u_shadow: gl::types::GLint,
    u_tint: gl::types::GLint,
    u_saturation: gl::types::GLint,
    u_pale_opacity: gl::types::GLint,
    u_panel_fixed: gl::types::GLint,
    u_black_edge: gl::types::GLint,
    u_lcd_mix: gl::types::GLint,
    u_grid_opacity: gl::types::GLint,
    u_pixel_transparency: gl::types::GLint,
    u_tv: gl::types::GLint,
    u_edge: gl::types::GLint,
    u_lcd_overlay_size: gl::types::GLint,
    fbo_scale: [f32; 2],
    u_origin: gl::types::GLint,
    origin: [f32; 2],
    u_pic: gl::types::GLint,
    pic: [f32; 4],
    u_src: gl::types::GLint,
    /// The game's frame size: the GBA's 240x160, or a TV console's own.
    canvas: (u32, u32),
    sample_background: bool,
    picture_dim: f32,
    shadow: [f32; 3],
    panel: Option<[f32; 3]>,
    tint: [f32; 3],
    saturation: f32,
    pale_opacity: f32,
    black_edge: bool,
    fills_height: bool,
    lcd_mix: [f32; 3],
    grid_opacity: f32,
    pixel_transparency: bool,
    edge: f32,
    paper: gl::types::GLuint,
    paper_size: f32,
    lcd_overlay: gl::types::GLuint,
    lcd_overlay_size: [f32; 2],
    effect: ScreenEffect,
    power: f32,
    fills: bool,
    src: [f32; 4],
    background: [f32; 3],
    shades: Shades,
    /// A tube's phosphor still glowing from the frames before: how much of the last picture
    /// stays (out of 256), and the picture as it glows. 0 for none.
    afterglow: u32,
    glow: Vec<u8>,
    /// A TV picture's scanlines and mask, each on or off.
    tv_lines: (bool, bool),
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
            u_shadow,
            u_tint,
            u_saturation,
            u_pale_opacity,
            u_panel_fixed,
            u_black_edge,
            u_lcd_mix,
            u_grid_opacity,
            u_pixel_transparency,
            u_tv,
            u_edge,
            u_lcd_overlay,
            u_lcd_overlay_size,
            u_origin,
            u_src,
        );
        unsafe {
            gl::UseProgram(prog);
            gl::Uniform1i(crate::gl::uniform_location(prog, "u_game"), 0);
            u_src = crate::gl::uniform_location(prog, "u_src");
            gl::Uniform2f(u_src, SRC_W as f32, SRC_H as f32);
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
            u_shadow = crate::gl::uniform_location(prog, "u_shadow");
            u_tint = crate::gl::uniform_location(prog, "u_tint");
            u_saturation = crate::gl::uniform_location(prog, "u_saturation");
            u_pale_opacity = crate::gl::uniform_location(prog, "u_pale_opacity");
            u_panel_fixed = crate::gl::uniform_location(prog, "u_panel_fixed");
            u_black_edge = crate::gl::uniform_location(prog, "u_black_edge");
            u_lcd_mix = crate::gl::uniform_location(prog, "u_lcd_mix");
            u_grid_opacity = crate::gl::uniform_location(prog, "u_grid_opacity");
            u_pixel_transparency = crate::gl::uniform_location(prog, "u_pixel_transparency");
            u_tv = crate::gl::uniform_location(prog, "u_tv");
            u_edge = crate::gl::uniform_location(prog, "u_edge");
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
            u_shadow,
            u_tint,
            u_saturation,
            u_pale_opacity,
            u_panel_fixed,
            u_black_edge,
            u_lcd_mix,
            u_grid_opacity,
            u_pixel_transparency,
            u_tv,
            u_edge,
            u_lcd_overlay_size,
            fbo_scale: [1.0, 1.0],
            u_origin,
            origin: [0.0, 0.0],
            u_pic,
            pic: WHOLE_TEXTURE,
            u_src,
            canvas: (SRC_W, SRC_H),
            sample_background: false,
            picture_dim: 0.0,
            shadow: PIXEL_SHADOW,
            panel: None,
            tint: [1.0, 1.0, 1.0],
            saturation: 1.0,
            pale_opacity: PALE_OPACITY,
            black_edge: false,
            fills_height: false,
            lcd_mix: [1.0, 1.0, 0.0],
            grid_opacity: 0.3,
            pixel_transparency: false,
            edge: 0.0,
            paper,
            paper_size: 1.0,
            lcd_overlay,
            lcd_overlay_size: [1.0, 1.0],
            effect: ScreenEffect::Lcd3x,
            power: 1.0,
            fills: false,
            src: WHOLE_TEXTURE,
            background: [0.0, 0.0, 0.0],
            shades: Shades::default(),
            afterglow: 0,
            glow: Vec::new(),
            tv_lines: (false, false),
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

    pub fn set_fills_height(&mut self, on: bool) {
        self.fills_height = on;
    }

    pub fn set_fills_frame(&mut self, on: bool) {
        self.fills = on;
    }

    pub fn set_picture(&mut self, rect: [f32; 4]) {
        self.pic = rect;
    }

    /// How dark the drop shadow each pixel casts on the panel behind it, and how far (in
    /// source pixels) it softens; a third value over 0.5 blurs its edges smooth.
    pub fn set_pixel_shadow(&mut self, [strength, spread, soft]: [f32; 3]) {
        self.shadow = [strength.clamp(0.0, 1.0), spread.clamp(0.0, 2.0), soft];
    }

    /// A fixed panel behind the picture (instead of black, or the GB's sampled lightest
    /// colour), and the tone light colours take from it.
    /// For the GBC's LCD: how dark the line under each row, and how strong each column's
    /// red, green or blue. On the GBC's own pixels, how grey its black: an unlit panel in a
    /// dim room never shows a true black.
    pub fn set_lcd_mix(&mut self, scanlines: f32, tint: f32, on_screen_pixels: bool, ink: f32) {
        self.lcd_mix = [
            scanlines.max(0.0),
            tint.max(0.0),
            if on_screen_pixels {
                1.0
            } else {
                ink.clamp(0.0, 0.4)
            },
        ];
    }

    /// How vivid colours show, 1.0 being the game's own.
    /// Black around the picture rather than the panel's own colour.
    pub fn set_black_edge(&mut self, black: bool) {
        self.black_edge = black;
    }

    /// How opaque the palest pixels stay with Pixel Transparency on: the lower, the more of
    /// the screen behind them shows.
    pub fn set_pale_opacity(&mut self, opacity: f32) {
        self.pale_opacity = opacity.clamp(0.0, 1.0);
    }

    pub fn set_saturation(&mut self, saturation: f32) {
        self.saturation = saturation.clamp(0.0, 2.0);
    }

    pub fn set_panel(&mut self, panel: Option<[f32; 3]>, tint: [f32; 3]) {
        self.panel = panel;
        self.tint = tint;
    }

    pub fn set_picture_appearance(&mut self, sample_background: bool, dim: f32) {
        if self.sample_background != sample_background {
            self.background = [0.0, 0.0, 0.0];
            self.shades.clear();
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

    pub fn set_tv_lines(&mut self, scanlines: bool, mask: bool) {
        self.tv_lines = (scanlines, mask);
    }

    /// The Game Boy's bare LCD round its picture: 0 the picture's ground, 1 ochre, 2 brown.
    pub fn set_screen_edge(&mut self, tone: u8) {
        self.edge = f32::from(tone.min(2));
    }

    /// The share of the last picture a TV's phosphor keeps glowing with, out of 256; 0 for none.
    pub fn set_afterglow(&mut self, keep: u32) {
        if keep == 0 {
            self.glow.clear();
        }
        self.afterglow = keep;
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

    /// The size of the frames `upload` takes. A canvas other than the GBA's is a TV
    /// console's: its picture fills the screen.
    pub fn set_canvas(&mut self, w: u32, h: u32) {
        if self.canvas == (w, h) || w == 0 || h == 0 {
            return;
        }
        let game = crate::gl::texture(w, h, gl::NEAREST, gl::CLAMP_TO_EDGE, gl::BGRA, None);
        unsafe {
            gl::DeleteTextures(1, &self.game);
            gl::UseProgram(self.prog);
            gl::Uniform2f(self.u_src, w as f32, h as f32);
        }
        self.game = game;
        self.canvas = (w, h);
    }

    fn tv(&self) -> bool {
        self.canvas != (SRC_W, SRC_H)
    }

    pub fn set_source_rect(&mut self, rect: [f32; 4]) {
        self.src = rect;
    }

    pub fn upload(&mut self, xrgb8888: &[u8]) {
        let (cw, ch) = self.canvas;
        if xrgb8888.len() < (cw * ch * 4) as usize {
            return;
        }
        if self.sample_background && self.pic != WHOLE_TEXTURE && !self.tv() {
            // Every frame: a Game Boy picture is small, and the screen behind it must change
            // colour with the palette on the same frame, not some frames later.
            self.background = self.shades.lightest(xrgb8888, self.pic);
        }
        let mut xrgb8888 = xrgb8888;
        if self.afterglow > 0 && self.tv() {
            // Each channel fades from what it was, and the new picture lights over that.
            let n = (cw * ch * 4) as usize;
            if self.glow.len() != n {
                self.glow = xrgb8888[..n].to_vec();
            }
            let keep = self.afterglow;
            for (g, &c) in self.glow.iter_mut().zip(&xrgb8888[..n]) {
                *g = c.max(((*g as u32 * keep) >> 8) as u8);
            }
            xrgb8888 = &self.glow;
        }
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.game);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                cw as i32,
                ch as i32,
                gl::BGRA,
                gl::UNSIGNED_BYTE,
                xrgb8888.as_ptr() as *const std::ffi::c_void,
            );
        }
    }

    pub fn draw(&self, quad: &Quad) {
        let src = match self.effect {
            ScreenEffect::Grid if !self.tv() => whole_scale_source(self.src, self.fbo_scale),
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
        } else if self.tv() {
            let (x, y, w, h) = fullscreen_game_rect(self.power);
            (x, y, w, h, screen_brightness(self.power))
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
                self.fills_height,
            );
            // Drawn across a panel wider than 4:3 (the frame's 640 columns stretched to it):
            // the picture keeps its own pixels in the middle and its ground runs to the edges.
            let k = (self.fbo_scale[0] / self.fbo_scale[1]).max(1.0);
            let x = (x + OUT_W as f32 * (k - 1.0) / 2.0) / k;
            (x, y, w / k, h, screen_brightness(self.power))
        } else {
            let (x, y, w, h) = fullscreen_game_rect(self.power);
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
            let bg = self.panel.unwrap_or(self.background);
            gl::Uniform3f(self.u_bg, bg[0], bg[1], bg[2]);
            gl::Uniform1f(self.u_picture_dim, self.picture_dim);
            gl::Uniform3f(
                self.u_shadow,
                self.shadow[0],
                self.shadow[1],
                self.shadow[2],
            );
            gl::Uniform3f(self.u_tint, self.tint[0], self.tint[1], self.tint[2]);
            gl::Uniform1f(self.u_saturation, self.saturation);
            gl::Uniform1f(self.u_pale_opacity, self.pale_opacity);
            gl::Uniform1f(
                self.u_panel_fixed,
                if self.panel.is_some() { 1.0 } else { 0.0 },
            );
            gl::Uniform1f(self.u_black_edge, if self.black_edge { 1.0 } else { 0.0 });
            gl::Uniform3f(
                self.u_lcd_mix,
                self.lcd_mix[0],
                self.lcd_mix[1],
                self.lcd_mix[2],
            );
            gl::Uniform1f(self.u_grid_opacity, self.grid_opacity);
            gl::Uniform1f(
                self.u_pixel_transparency,
                if self.pixel_transparency { 1.0 } else { 0.0 },
            );
            gl::Uniform2f(
                self.u_tv,
                self.tv_lines.0 as u8 as f32,
                self.tv_lines.1 as u8 as f32,
            );
            gl::Uniform1f(self.u_edge, self.edge);
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

/// How long a shade not drawn is still taken as one of the palette's: half a minute.
const SHADE_FRAMES: u64 = 1800;

/// The Game Boy palette's shades seen lately, so the screen behind the picture stays the
/// palette's lightest while the game fades to its darkest or shows nothing but one shade.
#[derive(Default)]
struct Shades {
    /// Each shade's 15-bit key, its colour, and the frame it was last drawn on.
    seen: Vec<(usize, [f32; 3], u64)>,
    frame: u64,
}

impl Shades {
    fn clear(&mut self) {
        self.seen.clear();
    }

    /// The lightest shade of the palette, from this frame and the ones before it.
    fn lightest(&mut self, pixels: &[u8], pic: [f32; 4]) -> [f32; 3] {
        self.frame += 1;
        let drawn = frame_shades(pixels, pic);
        // Two shades never seen at once: another palette, so the old one's are let go.
        let new = drawn
            .iter()
            .filter(|(k, _)| !self.seen.iter().any(|s| s.0 == *k))
            .count();
        if new >= 2 {
            self.seen.retain(|s| drawn.iter().any(|(k, _)| *k == s.0));
        }
        for (key, rgb) in drawn {
            match self.seen.iter_mut().find(|s| s.0 == key) {
                Some(s) => s.2 = self.frame,
                None => self.seen.push((key, rgb, self.frame)),
            }
        }
        let frame = self.frame;
        self.seen.retain(|s| frame - s.2 <= SHADE_FRAMES);
        self.seen
            .iter()
            .max_by(|a, b| luma(a.0).total_cmp(&luma(b.0)))
            .map_or([0.0; 3], |s| s.1)
    }
}

fn luma(key: usize) -> f32 {
    let r = ((key >> 10) & 31) as f32 / 31.0;
    let g = ((key >> 5) & 31) as f32 / 31.0;
    let b = (key & 31) as f32 / 31.0;
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// The shades a frame draws with, enough of each to count, as 15-bit keys and colours.
fn frame_shades(pixels: &[u8], pic: [f32; 4]) -> Vec<(usize, [f32; 3])> {
    const COLOR_BINS: usize = 1 << 15;
    let x0 = (pic[0].clamp(0.0, 1.0) * SRC_W as f32).floor() as usize;
    let y0 = (pic[1].clamp(0.0, 1.0) * SRC_H as f32).floor() as usize;
    let x1 = (pic[2].clamp(0.0, 1.0) * SRC_W as f32).ceil() as usize;
    let y1 = (pic[3].clamp(0.0, 1.0) * SRC_H as f32).ceil() as usize;
    let (x1, y1) = (x1.min(SRC_W as usize), y1.min(SRC_H as usize));
    if x0 >= x1 || y0 >= y1 {
        return Vec::new();
    }
    let mut counts = [0u16; COLOR_BINS];
    let mut first = vec![[0u8; 3]; 0];
    let mut keys = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let px = &pixels[(y * SRC_W as usize + x) * 4..][..3];
            let key = ((px[2] as usize >> 3) << 10)
                | ((px[1] as usize >> 3) << 5)
                | (px[0] as usize >> 3);
            if counts[key] == 0 {
                keys.push(key);
                first.push([px[2], px[1], px[0]]);
            }
            counts[key] = counts[key].saturating_add(1);
        }
    }
    let min_count = (((x1 - x0) * (y1 - y0)) / 500).max(8) as u16;
    let mut shades: Vec<_> = keys
        .iter()
        .zip(&first)
        .filter(|(k, _)| counts[**k] >= min_count)
        .map(|(k, c)| (*k, c.map(|v| v as f32 / 255.0)))
        .collect();
    if shades.is_empty() {
        // A picture of specks: its most drawn colour.
        if let Some(i) = (0..keys.len()).max_by_key(|i| counts[keys[*i]]) {
            shades.push((keys[i], first[i].map(|v| v as f32 / 255.0)));
        }
    }
    shades
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

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(shades: &[[u8; 3]]) -> Vec<u8> {
        let n = (SRC_W * SRC_H) as usize;
        (0..n)
            .flat_map(|i| {
                let [r, g, b] = shades[i % shades.len()];
                [b, g, r, 255]
            })
            .collect()
    }

    #[test]
    fn the_screen_behind_stays_the_palettes_lightest_while_the_game_goes_dark() {
        let pic = [0.0, 0.0, 160.0 / SRC_W as f32, 144.0 / SRC_H as f32];
        let green = [[142, 149, 38], [95, 124, 72], [54, 85, 79], [33, 58, 59]];
        let mut shades = Shades::default();
        let light = shades.lightest(&frame(&green), pic);
        assert_eq!(light, [142.0 / 255.0, 149.0 / 255.0, 38.0 / 255.0]);
        // A fade to the darkest shade, and a screen of nothing else.
        assert_eq!(shades.lightest(&frame(&green[2..]), pic), light);
        assert_eq!(shades.lightest(&frame(&green[3..]), pic), light);
        // Another palette chosen: its own lightest at once.
        let grey = [[200, 200, 200], [120, 120, 120], [60, 60, 60], [10, 10, 10]];
        assert_eq!(shades.lightest(&frame(&grey), pic), [200.0 / 255.0; 3]);
    }
}
