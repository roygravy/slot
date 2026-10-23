macro_rules! eprintln {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stderr(), $($arg)*);
    }};
}

mod atomic;
mod cart_marker;
pub mod cart_shell;
mod config;
mod core;
pub mod gb;
mod gb_palette;
mod gba;
pub mod ini;
mod platform;
mod ring;
mod scan;
mod slot_state;
pub mod snes;
mod stamp;
mod zone;

pub use atomic::{atomic_write, sweep_temps};
pub use cart_marker::{
    forget_no_label, has_no_label, markers, note_no_label, write_marker, CART_MARKER_FILE, NO_LABEL_FILE,
};
pub use cart_shell::{
    write_choice as write_shell_choice, Outline, ShellChoice, ShellFinish, CART_SHELL_FILE,
    LABELS_SHELL_FILE,
};
pub use config::{move_config, CONFIG_DIR};
pub use core::{
    core_for, core_for_platform, read_selected_cores, write_selected_core, Core, SELECTED_CORE_FILE,
};
pub use gb_palette::GbPalette;
pub use gba::{header_clean, header_code, header_title, header_title_code};
pub use platform::{Platform, TV_CANVAS};
pub use ring::{StateEntry, StateRing, RING_MAX};
pub use scan::{initial, is_hidden, listing, scan, sort_key, Cart, StoreError};
pub use slot_state::{
    read_slot_state, write_slot_state, Appearance, BezelPick, FavoriteMark, Recent, Shader, SlotState,
    WallpaperPick, WallpaperTint, AUTO_SLEEP_DEFAULT_SECS, AUTO_SLEEP_OPTIONS_SECS, BLUE_LIGHT_MAX,
    BRIGHTNESS_MAX, FF_SPEEDS, FF_SPEED_DEFAULT, RECENTS_MAX, UTC_OFFSET_MAX,
    UTC_OFFSET_MIN, VOLUME_MAX,
};
pub use zone::{zone_named, Summer, Zone, ZONES};
pub use stamp::{
    civil_from_days, days_from_civil, days_in_month, format_stamp, parse_stamp, stamp_now,
};
