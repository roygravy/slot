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

/// Dark theme: bone white on charcoal, as slot has always looked.
pub const DARK: Ink = Ink {
    text: [0xf6 as f32 / 255.0, 0xf4 as f32 / 255.0, 0xef as f32 / 255.0, 1.0],
    dim: [0x9a as f32 / 255.0, 0x9a as f32 / 255.0, 0xa4 as f32 / 255.0, 1.0],
    bar: [0x4d as f32 / 255.0, 0x4d as f32 / 255.0, 0x57 as f32 / 255.0, 1.0],
    on_bar: [0xf6 as f32 / 255.0, 0xf4 as f32 / 255.0, 0xef as f32 / 255.0, 1.0],
    wash: [0.0, 0.0, 0.0, 0.65],
};

/// Over a paused game: the game dimmed a little more, the selection a band of see-through
/// black, and light text throughout.
pub const OVER_GAME: Ink = Ink {
    text: [0xf6 as f32 / 255.0, 0xf4 as f32 / 255.0, 0xef as f32 / 255.0, 1.0],
    dim: [0xa8 as f32 / 255.0, 0xa8 as f32 / 255.0, 0xb0 as f32 / 255.0, 1.0],
    bar: [0xf6 as f32 / 255.0, 0xf4 as f32 / 255.0, 0xef as f32 / 255.0, 1.0],
    on_bar: [0x1a as f32 / 255.0, 0x19 as f32 / 255.0, 0x17 as f32 / 255.0, 1.0],
    wash: [0.0, 0.0, 0.0, 0.55],
};

/// Light theme: the Game Boy's charcoal print on its warm grey plastic, with the
/// bar in the darker grey of its screen surround.
pub fn light() -> Ink {
    Ink {
        text: rgb([0x2b, 0x2a, 0x2e], 1.0),
        dim: rgb([0x6c, 0x6a, 0x66], 1.0),
        bar: rgb([0x58, 0x5a, 0x63], 1.0),
        on_bar: rgb([0xee, 0xec, 0xe6], 1.0),
        wash: rgb([0xd6, 0xd4, 0xce], 0.35),
    }
}

pub fn ink(light_theme: bool) -> Ink {
    if light_theme {
        light()
    } else {
        DARK
    }
}
