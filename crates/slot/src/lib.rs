pub mod app;
pub mod audio;
pub mod bezel;
pub mod boot_time;
pub mod bootlogo;
pub mod build_info;
pub mod cable;
pub mod card_log;
pub mod cart_faces;
pub mod cheats;
pub mod core;
pub mod core_picker;
#[cfg(unix)]
pub mod crash;
#[cfg(not(unix))]
pub mod crash {
    pub fn install() {}
}
pub mod drc;
pub mod emu;
pub mod face_builder;
pub mod frames;
pub mod frontend;
pub mod input;
pub mod label_cache;
mod labels;
pub mod latency;
pub mod link_art_builder;
pub mod link_kind;
pub mod link_lan;
pub mod link_mdns;
pub mod link_net;
pub mod link_radio;
pub mod link_screen;
pub mod link_start;
pub mod link_state;
pub mod panel_turn;
pub mod persist;
pub mod prefetch;
pub mod resample;
pub mod rewind;
pub mod screenshot;
pub mod shell_picker;
pub mod shots;
pub mod root;
pub mod session;
pub mod state_writer;
pub mod thumb;
pub mod video_mode;
pub mod wallpaper;
