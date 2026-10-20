//! The colours that change with the theme. Text faces are rendered white and drawn with
//! `Draw::Tint`, so one face serves every theme.

fn rgb(c: [u8; 3], a: f32) -> [f32; 4] {
    [
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
        a,
    ]
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Ink {
    /// Selected text, values and switch fills.
    pub text: [f32; 4],
    /// Text on the rows that are not selected.
    pub dim: [f32; 4],
    /// The selection bar behind the current row.
    pub bar: [f32; 4],
    /// Text sitting on the selection bar.
    pub on_bar: [f32; 4],
    /// The wash laid over a wallpaper in menus.
    pub wash: [f32; 4],
}

const fn grey(v: u8, a: f32) -> [f32; 4] {
    [v as f32 / 255.0, v as f32 / 255.0, v as f32 / 255.0, a]
}

/// Dark theme, monochrome: white text, grey for the rows not chosen, and the chosen row a
/// solid white bar with black text.
pub const DARK: Ink = Ink {
    text: grey(0xf4, 1.0),
    dim: grey(0x9c, 1.0),
    bar: grey(0xf4, 1.0),
    on_bar: grey(0x16, 1.0),
    wash: [0.0, 0.0, 0.0, 0.65],
};

/// Over a paused game: the dark theme's ink, the game dimmed behind it.
pub const OVER_GAME: Ink = Ink {
    text: grey(0xf4, 1.0),
    dim: grey(0xa6, 1.0),
    bar: grey(0xf4, 1.0),
    on_bar: grey(0x16, 1.0),
    wash: [0.0, 0.0, 0.0, 0.65],
};

/// Light theme, monochrome: near-black text on the light ground, grey for the rows not
/// chosen, and the chosen row a solid black bar with white text.
pub fn light() -> Ink {
    Ink {
        text: grey(0x1c, 1.0),
        dim: grey(0x66, 1.0),
        bar: grey(0x1c, 1.0),
        on_bar: grey(0xf4, 1.0),
        wash: rgb([0xd6, 0xd4, 0xce], 0.35),
    }
}

/// The flat ground of the menus on the theme's own wallpaper: the wallpaper's middle tone, so
/// menus read as the shelf's ground without its gradient. The boot logo uses the dark one.
pub const MENU_FLAT_DARK: [u8; 3] = [0x16, 0x17, 0x1a];
pub const MENU_FLAT_LIGHT: [u8; 3] = [0xea, 0xe8, 0xe3];

pub fn menu_flat(light_ground: bool) -> [f32; 4] {
    rgb(if light_ground { MENU_FLAT_LIGHT } else { MENU_FLAT_DARK }, 1.0)
}

pub fn ink(light_theme: bool) -> Ink {
    if light_theme {
        light()
    } else {
        DARK
    }
}
