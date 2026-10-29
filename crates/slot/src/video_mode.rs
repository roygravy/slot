use slot_gfx::{SRC_H, SRC_W, WHOLE_TEXTURE};
use slot_store::Platform;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum VideoMode {
    #[default]
    Actual,
    Stretch,
}

impl VideoMode {
    pub fn as_str(self) -> &'static str {
        match self {
            VideoMode::Actual => "actual",
            VideoMode::Stretch => "stretch",
        }
    }

    pub fn parse(s: &str) -> Option<VideoMode> {
        match s.trim().to_ascii_lowercase().as_str() {
            "actual" => Some(VideoMode::Actual),
            "stretch" => Some(VideoMode::Stretch),
            _ => None,
        }
    }
}

pub fn source_rect(platform: Platform, mode: VideoMode) -> [f32; 4] {
    let (w, h) = platform.picture();
    if mode == VideoMode::Actual || (w, h) == (SRC_W, SRC_H) {
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
