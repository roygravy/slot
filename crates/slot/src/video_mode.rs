use slot_gfx::{SRC_H, SRC_W, WHOLE_TEXTURE};
use slot_store::Platform;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum VideoMode {
    #[default]
    Actual,
    /// As tall as the screen, as wide as its own shape lets it be.
    FillHeight,
    Stretch,
}

impl VideoMode {
    /// L: one size up, R: one size down; the ends stay put.
    pub fn bigger(self) -> VideoMode {
        match self {
            VideoMode::Actual => VideoMode::FillHeight,
            _ => VideoMode::Stretch,
        }
    }

    pub fn smaller(self) -> VideoMode {
        match self {
            VideoMode::Stretch => VideoMode::FillHeight,
            _ => VideoMode::Actual,
        }
    }
}

impl VideoMode {
    pub fn as_str(self) -> &'static str {
        match self {
            VideoMode::Actual => "actual",
            VideoMode::FillHeight => "height",
            VideoMode::Stretch => "stretch",
        }
    }

    pub fn parse(s: &str) -> Option<VideoMode> {
        match s.trim().to_ascii_lowercase().as_str() {
            "actual" => Some(VideoMode::Actual),
            "height" => Some(VideoMode::FillHeight),
            "stretch" => Some(VideoMode::Stretch),
            _ => None,
        }
    }
}

pub fn source_rect(platform: Platform, mode: VideoMode) -> [f32; 4] {
    let (w, h) = platform.picture();
    if mode != VideoMode::Stretch || (w, h) == (SRC_W, SRC_H) {
        return WHOLE_TEXTURE;
    }
    let x = SRC_W.saturating_sub(w) / 2;
    let y = SRC_H.saturating_sub(h) / 2;
    [
        x as f32 / SRC_W as f32,
        y as f32 / SRC_H as f32,
        w as f32 / SRC_W as f32,
        h as f32 / SRC_H as f32,
    ]
}
