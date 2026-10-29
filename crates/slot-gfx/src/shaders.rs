use crate::surface::GfxError;

const VERT_PREAMBLE: &str = "#version 330 core\n#define attribute in\n#define varying out\n";

const FRAG_PREAMBLE: &str = "#version 330 core\n#define varying in\n\
                             #define texture2D texture\nout vec4 FRAG_COLOR;\n";

const VERT_PREAMBLE_ES: &str = "";
const FRAG_PREAMBLE_ES: &str = "#define FRAG_COLOR gl_FragColor\n";

pub fn program(vert: &str, frag: &str) -> Result<gl::types::GLuint, GfxError> {
    let (vp, fp) = match crate::gl::es() {
        true => (VERT_PREAMBLE_ES, FRAG_PREAMBLE_ES),
        false => (VERT_PREAMBLE, FRAG_PREAMBLE),
    };
    crate::gl::program(&format!("{vp}{vert}"), &format!("{fp}{frag}"))
}

pub const RECT_VERT: &str = r#"
attribute vec2 a_pos;
uniform vec4 u_rect;
uniform vec2 u_target;
varying vec2 v_uv;
void main() {
    v_uv = a_pos;
    vec2 p = (u_rect.xy + a_pos * u_rect.zw) / u_target;
    gl_Position = vec4(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0);
}
"#;

pub const SPRITE_VERT: &str = r#"
attribute vec2 a_pos;
uniform vec4 u_rect;
uniform vec2 u_target;
uniform vec2 u_turn;
uniform vec4 u_src;
varying vec2 v_uv;
void main() {
    v_uv = u_src.xy + a_pos * u_src.zw;
    vec2 mid = u_rect.zw * 0.5;
    vec2 local = a_pos * u_rect.zw - mid;
    vec2 turned = vec2(u_turn.x * local.x - u_turn.y * local.y,
                       u_turn.y * local.x + u_turn.x * local.y);
    vec2 p = (u_rect.xy + a_pos * u_rect.zw + (turned - local)) / u_target;
    gl_Position = vec4(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0);
}
"#;

pub const GAME_FRAG: &str = r#"
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
uniform sampler2D u_game;
uniform sampler2D u_paper;
uniform sampler2D u_lcd_overlay;
uniform vec2 u_src;
uniform vec2 u_output_size;
uniform vec2 u_lcd_overlay_size;
uniform vec4 u_uv;
uniform float u_bright;
uniform float u_mode;
uniform float u_paper_size;
uniform vec2 u_origin;
uniform vec4 u_rect;
uniform vec2 u_fbo;
uniform vec4 u_pic;
uniform vec3 u_bg;
uniform float u_picture_dim;
uniform vec3 u_shadow;
uniform vec3 u_tint;
uniform float u_saturation;
uniform float u_pale_opacity;
uniform float u_panel_fixed;
uniform float u_black_edge;
uniform vec3 u_lcd_mix;
uniform float u_grid_opacity;
uniform float u_pixel_transparency;
uniform vec2 u_tv;
varying vec2 v_uv;
const vec3 offsets = vec3(3.141592654) * vec3(1.0 / 2.0, 1.0 / 2.0 - 2.0 / 3.0, 1.0 / 2.0 - 4.0 / 3.0);
const float TV_SCANLINE = 0.72;
const float TV_MASK = 0.18;
const float TV_GLOW = 0.08;
const vec3 luma_coeff = vec3(0.2126, 0.7152, 0.0722);
vec2 grid_band(vec2 x) {
    return floor(x) / 3.0 + min(fract(x), vec2(1.0 / 3.0));
}
// How much of [x0, x1] falls on the band [a, b) of a pattern that repeats every 1.
float band_cover(float x0, float x1, float a, float b) {
    float sum = 0.0;
    for (int k = -1; k <= 1; k++) {
        float o = float(k);
        sum += max(0.0, min(x1, b + o) - max(x0, a + o));
    }
    return sum;
}
float dot_weight(vec2 f, vec2 o, float bloom) {
    return exp(-4.0 * length(f - (o + vec2(0.5))) * bloom);
}
void main() {
    vec2 uv = u_uv.xy + v_uv * u_uv.zw;
    vec2 f = fract(uv * u_src);
    vec2 px = max((u_rect.zw * u_fbo) / (u_uv.zw * u_src), vec2(1.0));
    vec2 d = fract(uv * u_src) - 0.5;
    vec2 span = 0.5 - 0.5 / px;
    vec2 sharp = (floor(uv * u_src) + (d - clamp(d, -span, span)) * px + 0.5) / u_src;
    vec3 c = texture2D(u_game, sharp).rgb;
    vec4 lcd = vec4(0.0);
    bool inside = all(greaterThanEqual(uv, u_pic.xy)) && all(lessThan(uv, u_pic.zw));
    if (!inside) {
        c = u_black_edge > 0.5 ? vec3(0.0) : u_bg;
    } else if (u_mode > 5.5) {
        // A TV's picture: the console's pixels as they are, every other screen line a little
        // darker, a faint slot mask over them, and a soft glow around the bright parts. Worked
        // in linear light; the darker lines give way under bright colours, as a beam widens.
        vec3 lin = pow(c, vec3(2.2));
        vec2 screen_px = floor(gl_FragCoord.xy / u_fbo);
        float luma = dot(lin, luma_coeff);
        if (u_tv.x > 0.5 && mod(screen_px.y, 2.0) > 0.5) {
            lin *= mix(TV_SCANLINE, 1.0, luma * 0.5);
        }
        float cell = mod(screen_px.x + floor(screen_px.y / 2.0) * 1.5, 3.0);
        vec3 mask = vec3(cell < 1.0 ? 1.0 : 0.0, cell >= 1.0 && cell < 2.0 ? 1.0 : 0.0, cell >= 2.0 ? 1.0 : 0.0);
        float m = TV_MASK * u_tv.y;
        lin *= (1.0 - m) + m * 3.0 * mask;
        vec2 at = uv * u_src;
        vec3 glow = vec3(0.0);
        for (int j = -1; j <= 1; j++) {
            for (int i = -1; i <= 1; i++) {
                vec2 o = vec2(float(i) * 2.0 * u_src.x / 256.0, float(j) * 2.0);
                float w = (i == 0 ? 2.0 : 1.0) * (j == 0 ? 2.0 : 1.0) / 16.0;
                glow += texture2D(u_game, (floor(at) + o + 0.5) / u_src).rgb * w;
            }
        }
        lin += TV_GLOW * pow(glow, vec3(2.2));
        c = pow(clamp(lin, 0.0, 1.0), vec3(1.0 / 2.2));
    } else if (u_mode > 4.5) {
        vec2 screen_px = floor(gl_FragCoord.xy / u_fbo);
        vec2 overlay_px = floor(screen_px * u_lcd_overlay_size / u_output_size);

        vec2 overlay_uv = (overlay_px + 0.5) / u_lcd_overlay_size;
        lcd = texture2D(u_lcd_overlay, overlay_uv);
        if (u_panel_fixed > 0.5) {
            // The GBC's LCD, drawn on its own pixels (three screen pixels each way): a solid
            // red, green and blue line down each pixel, and a faint dark line under every row.
            // On the GBA, which is not scaled a whole number of times, the cells follow the
            // screen's own pixels instead, three to a cell.
            // Each screen pixel takes the stripes it covers, so a picture scaled by no whole
            // number, full screen, keeps the colours it has at three times.
            vec3 stripe;
            float dark;
            if (u_lcd_mix.z > 0.5) {
                vec2 cell = mod(screen_px, 3.0);
                stripe = cell.x < 0.5 ? vec3(1.0, 0.0, 0.0) : (cell.x < 1.5 ? vec3(0.0, 1.0, 0.0) : vec3(0.0, 0.0, 1.0));
                dark = cell.y > 1.5 ? 1.0 : 0.0;
            } else {
                vec2 w = 0.5 / px;
                vec2 lo = f - w;
                vec2 hi = f + w;
                stripe = vec3(
                    band_cover(lo.x, hi.x, 0.0, 1.0 / 3.0),
                    band_cover(lo.x, hi.x, 1.0 / 3.0, 2.0 / 3.0),
                    band_cover(lo.x, hi.x, 2.0 / 3.0, 1.0)) / (2.0 * w.x);
                dark = band_cover(lo.y, hi.y, 2.0 / 3.0, 1.0) / (2.0 * w.y);
            }
            c = mix(c, stripe, u_lcd_mix.y);
            c = mix(c, vec3(0.0), u_lcd_mix.x * dark);
        } else {
            c = mix(c, lcd.rgb, lcd.a * 0.45);
        }
    } else if (u_mode > 3.5) {
        c = pow(c, vec3(1.05));
        vec2 d = abs(f - 0.5);
        float x2 = max(d.x, d.y);
        x2 = x2 * x2;
        float x4 = x2 * x2;
        float w = 48.0 * (x4 - (8.0 / 3.0) * x4 * x2);
        w = w * (w + (1.0 - w) * 0.65) * 0.65;
        float luma = dot(c, luma_coeff);
        w = w * luma;
        c = mix(c, vec3(1.0), w);
        vec3 bg = vec3(texture2D(u_paper, (floor(gl_FragCoord.xy - u_origin) + 0.5) / u_paper_size).r) * c;
        c = mix(c, bg, dot(c, luma_coeff));
    } else if (u_mode > 2.5) {
        float bloom = mix(1.05, 0.95, dot(c, vec3(0.30, 0.59, 0.11)));
        float mid = dot_weight(f, vec2(0.0), bloom);
        float sum = 0.0;
        for (int j = -1; j <= 1; j++) {
            for (int i = -1; i <= 1; i++) {
                sum += dot_weight(f, vec2(float(i), float(j)), bloom);
            }
        }
        c = mix(c, mix(1.2 * c * mid, c * sum, 0.85), 0.85);
    } else if (u_mode > 1.5) {
        vec2 hp = 0.5 * u_uv.zw * u_src / (u_rect.zw * u_fbo);
        vec2 edge = (grid_band(f + hp) - grid_band(f - hp)) / (2.0 * hp);
        float line = clamp(max(edge.x, edge.y), 0.0, 1.0);
        c = mix(c, u_bg, line * u_grid_opacity);
    } else if (u_mode > 0.5) {
        vec2 angle = f * 6.283185307;
        float yfactor = (16.0 + sin(angle.y)) / 17.0;
        vec3 xfactors = (4.0 + sin(angle.x + offsets)) / 5.0;
        c *= yfactor * xfactors;
    }
    if (inside) {
        c *= 1.0 - u_picture_dim;
        // How vivid the screen shows colour: a little less than the raw colours on a GBA.
        c = mix(vec3(dot(c, luma_coeff)), c, u_saturation);
        // A reflective panel: the brighter each channel, the more it takes the panel's tone.
        c *= mix(vec3(1.0), u_tint, clamp(c, 0.0, 1.0));
    }
    if (inside && u_pixel_transparency > 0.5) {
        vec3 panel = (u_mode > 4.5 && u_panel_fixed < 0.5) ? lcd.rgb : u_bg;
        float luma = clamp(dot(c, luma_coeff), 0.0, 1.0);
        float ink = 1.0 - smoothstep(0.05, 0.88, luma);
        float opacity = mix(u_pale_opacity, 0.97, ink);
        c = mix(panel, c, opacity);

        vec2 texel = vec2(1.0 / u_src.x, 1.0 / u_src.y);
        vec2 edge_distance = min((uv - u_pic.xy) / texel, (u_pic.zw - uv) / texel);
        bool away_from_edge = all(greaterThan(edge_distance, vec2(1.0)));
        vec2 shadow_uv = sharp - texel;
        float spread = u_shadow.y;
        float shadow_luma;
        vec3 shadow_rgb = vec3(0.0);
        if (u_shadow.z > 0.5) {
            // Soft: a small blur over the pixel up and to the left, edges fading out.
            vec2 soft_uv = uv - texel * 0.85;
            shadow_rgb = vec3(0.0);
            for (int j = -1; j <= 1; j++) {
                for (int i = -1; i <= 1; i++) {
                    float w = (i == 0 ? 2.0 : 1.0) * (j == 0 ? 2.0 : 1.0) / 16.0;
                    vec2 o = vec2(float(i), float(j)) * texel * spread;
                    // Only the game's own pixels cast shadows: never sample past its picture.
                    vec2 at = clamp(soft_uv + o, u_pic.xy + texel * 0.5, u_pic.zw - texel * 0.5);
                    shadow_rgb += texture2D(u_game, at).rgb * w;
                }
            }
            shadow_luma = dot(shadow_rgb, luma_coeff);
        } else {
            shadow_luma = dot(texture2D(u_game, shadow_uv).rgb, luma_coeff) * 0.60;
            shadow_luma += dot(texture2D(u_game, shadow_uv - vec2(texel.x * spread, 0.0)).rgb, luma_coeff) * 0.10;
            shadow_luma += dot(texture2D(u_game, shadow_uv + vec2(texel.x * spread, 0.0)).rgb, luma_coeff) * 0.10;
            shadow_luma += dot(texture2D(u_game, shadow_uv - vec2(0.0, texel.y * spread)).rgb, luma_coeff) * 0.10;
            shadow_luma += dot(texture2D(u_game, shadow_uv + vec2(0.0, texel.y * spread)).rgb, luma_coeff) * 0.10;
        }
        float shadow_ink = 1.0 - smoothstep(0.05, 0.45, shadow_luma);
        float exposed_panel = smoothstep(0.08, 0.70, luma);
        if (u_shadow.z > 0.5) {
            // The whole LCD lets light through: every pixel darker than the spot its shadow
            // falls on casts one, a coloured pixel as well as a black one, the darker the more.
            float here = dot(texture2D(u_game, sharp).rgb, luma_coeff);
            shadow_ink = smoothstep(0.0, 0.6, here - shadow_luma);
            exposed_panel = 1.0;
        }
        float shadow = away_from_edge ? shadow_ink * exposed_panel * u_shadow.x : 0.0;
        // Light through a coloured pixel is coloured: its shadow is a darker shade of it.
        vec3 shade = u_shadow.z > 0.5 ? shadow_rgb * 0.5 : vec3(0.0);
        c = mix(c, c * shade, shadow);
    }
    FRAG_COLOR = vec4(c * u_bright, 1.0);
}
"#;

pub const SPRITE_FRAG: &str = r#"
precision mediump float;
uniform sampler2D u_tex;
uniform vec4 u_colour;
varying vec2 v_uv;
void main() {
    FRAG_COLOR = texture2D(u_tex, v_uv) * u_colour;
}
"#;

pub const BLIT_VERT: &str = r#"
attribute vec2 a_pos;
uniform vec4 u_uv;
varying vec2 v_uv;
void main() {
    v_uv = u_uv.xy + a_pos * u_uv.zw;
    gl_Position = vec4(a_pos * 2.0 - 1.0, 0.0, 1.0);
}
"#;

pub const TURN_VERT: &str = r#"
attribute vec2 a_pos;
uniform float u_ccw;
varying vec2 v_uv;
void main() {
    vec2 cw = vec2(1.0 - a_pos.y, a_pos.x);
    vec2 ccw = vec2(a_pos.y, 1.0 - a_pos.x);
    v_uv = mix(cw, ccw, u_ccw);
    gl_Position = vec4(a_pos * 2.0 - 1.0, 0.0, 1.0);
}
"#;

pub const BLIT_FRAG: &str = r#"
precision mediump float;
uniform sampler2D u_tex;
uniform vec3 u_gain;
varying vec2 v_uv;
void main() {
    FRAG_COLOR = vec4(texture2D(u_tex, v_uv).rgb * u_gain, 1.0);
}
"#;

pub const BEZEL_FRAG: &str = r#"
precision mediump float;
uniform sampler2D u_tex;
uniform vec3 u_gain;
uniform float u_alpha;
varying vec2 v_uv;
void main() {
    vec4 c = texture2D(u_tex, v_uv);
    FRAG_COLOR = vec4(c.rgb * u_gain, c.a * u_alpha);
}
"#;
