mod about;
mod art;
pub use art::{cover, cover_label, is_wide_snes_label};
mod bitmaps;
mod buttons_help;
mod cheat_page;
pub use bitmaps::{
    link_port, link_port_light, nes_lid, nes_lid_orig, sgb_band, slot_back, slot_back_gb,
    slot_back_light, slot_front, slot_front_gb, slot_front_light, slot_orig, slot_tv, Mould,
    SgbModel, NES_LID_W, SGB_BAND_H, SLOT_H, SLOT_W,
};
mod backdrop;
mod barcode;
mod battery;
mod board;
mod cart;
mod clock;
mod draw;
mod footer;
mod hud;
mod icon;
mod keyboard;
mod link_art;
mod plate;
mod polaroids;
mod power_menu;
mod quick_menu;
mod rebuild;
mod refusal;
mod shelf;
mod shell;
mod shell_picker;
mod shot_page;
mod silhouette;
mod slot_chrome;
pub mod text;
pub mod theme;
mod toast;

pub use about::{about_face, draw_about};
pub use backdrop::{
    bezel_face, default_background, default_game_bezel_face, draw_backdrop, draw_ground,
    game_bezel_face, light_background, wallpaper_face,
};
pub use barcode::{code39, CODE39_NARROW, CODE39_WIDE};
pub use battery::{battery_face, draw_gauge, BOLT_PX, GAUGE_H, GAUGE_W, WALL};
pub use board::{
    board_at, board_face, board_from, board_in, board_x, board_zoom, chip_face, chip_shadow_face,
    grown, lid_at, lid_from, lift_of, on_board, padded, rom_marking, rom_marking_face, shelf_cart,
    shelf_cart_at, slide_of, socket_face, Placed, BOARD_H, BOARD_W, BOARD_Y, CHIP_H, CHIP_TIP,
    CHIP_U, CHIP_V, CHIP_W, HOP_LIFT, LID_TURN, ROM_H, ROM_W, ROM_X, ROM_Y, SHADOW_H, SHADOW_W,
    SLIDE_SHARE, SLIDE_UP, SOCKET_H, SOCKET_U, SOCKET_V, SOCKET_W, TURN_PAD,
};
pub use buttons_help::{
    buttons_body_face, buttons_tab_face, ButtonsFaces, ButtonsHelp, BUTTON_PAGES,
    LEGEND_Y as MENU_LEGEND_Y,
};
pub use cart::{
    cart_backing, cart_box, cart_face, cart_face_with, cart_interior, cart_shadow, clean_label,
    gb_label_panel, gb_shell_of, is_clear, label_colour, label_panel, label_size, label_tags,
    label_text, mould_of, neutral_shell, seated_box, sgb_model_of, shows_interior, CartFace,
    CART_H, CART_W, GBC_LABEL_H, GBC_LABEL_W, GBC_LABEL_X, GBC_LABEL_Y, GB_CART_H, GB_CART_W,
    GB_LABEL_H, GB_LABEL_W, GB_LABEL_X, GB_LABEL_Y, LABEL_H, LABEL_W, LABEL_WELL, LABEL_X, LABEL_Y,
};
pub use cheat_page::{
    cheat_badge_face, draw_cheat_badge_in_game, draw_cheat_badge_in_menu, first_shown, CheatFaces,
    CheatPage, BADGE_GAME_H, BADGE_MENU_H, CHEAT_ROWS, NAME_MAX_W,
};
pub use clock::{
    clock_label, clock_text, date_time_text, date_time_text_in, hhmm, set_clock_hint_face,
    ClockPicker, Field, TimeZone,
};
pub use draw::{Canvas, Draw, Layout, TexId, OUT_H, OUT_W};
pub use footer::{corner_clock_end, draw_corners, draw_footer, wifi_face, Printed, FOOTER_Y};
pub use hud::{
    badge_at, ff_badge, footer_ink, FfState, Hud, HudKind, LinkBadge, Millis, ALARM_BAND_H,
    HUD_ICON_PX, HUD_INK, HUD_MS, LINK_HOST_INK, LINK_JOIN_INK, PLATE_H,
};
pub use icon::{
    badge_face, drop_shadow_face, glow_face, halo_face, icon_box, icon_face, icon_glyph_box,
    icon_glyph_face, star_mark_face, warm_fonts, Badge, Icon,
};
pub use keyboard::{
    accents, draw_label_search, key_rows, search_heading_face, search_row_faces,
    search_status_face, search_typed_face, Cap, Key, KeyboardFaces, SearchScreen, SearchTexts,
    SearchView, Shape, Up,
};
pub use link_art::{
    link_art, LinkArt, ADAPTER_BASE_X, ADAPTER_BASE_Y, ADAPTER_H, ADAPTER_W, ARCS, ARROW_H,
    ARROW_LEFT_X, ARROW_RIGHT_X, ARROW_W, ARROW_Y, CLICKS_H, CLICKS_W, CLICKS_X, CLICKS_Y, NET_H,
    NET_W, NET_X, NET_Y, PLUG_H, PLUG_TIP_X, PLUG_W, PORT_H, PORT_W, PORT_Y,
};
pub use plate::{
    arrows_hint_coloured, arrows_hint_face, arrows_hint_width, cap_width, centred_hints,
    hint_coloured, hint_face, hint_quad, hint_row, hint_width, status_clock_face, tint_mask,
    title_face, title_line_face, word_face, word_ink, word_width, Hint, UndoFace, ARROW_GAP, CAP,
    CAP_GAP, HINT_EDGE, HINT_GAP, HINT_H, LEGEND_GAP, TITLE_H, TITLE_LINE_H, TITLE_W,
};
pub use polaroids::{
    photo_face, ConfirmFaces, PhotoFace, Polaroids, DOT, LEGEND, PHOTO_H, PHOTO_W,
};
pub use power_menu::{menu_face, PowerChoice, MENU_PAD};
pub use quick_menu::{
    draw_menu_ground, quick_caret_face, quick_first, quick_help_face, quick_label_face,
    quick_legend_faces, quick_pitch, quick_pointer_face, quick_row_top, quick_switch_face,
    quick_value_face, window_first, QuickMenu, QuickMenuFaces, QuickRow, QuickValue, QUICK_EDGE,
    QUICK_LABEL_X, QUICK_PITCH, SUB_ROWS,
};
pub use rebuild::{rebuild_banner_box, rebuild_banner_face, RebuildBanner};
pub use refusal::Refusal;
pub use shelf::{
    foot_y, repeat_gap, rest_y, Shelf, DROP_MARGIN, GLOW_GOLD, GLOW_MARGIN, GLOW_WHITE,
    REPEAT_DELAY_MS, STAR_PX,
};
pub use shell::{
    gb_plastics, gb_table_shells, gba_plastics, gba_shell_for,
    lookup_order_is_exact_then_family_then_default, shell_for, shell_presets, table_keys, Finish,
    Shell, DEFAULT_SHELL, DMG_SHELL, DUAL_MODE_SHELL, GBC_CLEAR_SHELL, GB_CLEAR_SHELL,
};
pub use shell_picker::{
    draw_shell_picker, shell_picker_hop, shell_picker_shapes, shell_picker_word, LabelTab,
    PickerRow, ShellPickerFaces, ShellPickerView,
};
pub use shot_page::{ShotFaces, ShotPage, SHOT_H, SHOT_W};
pub use silhouette::{gb_silhouette, silhouette, GbShell};
pub use slot_chrome::{
    click_at, draw_empty_slot, draw_slot_name, draw_slot_part, draw_slot_title, ease, edge,
    housing, insert_seconds, opening, recess, set_light_ground, set_sgb_band, sgb_band_rect,
    sgb_seated_rect, slot_name_y, SlotChrome, ALERT_PX, GB_SEAT_LIFT, LIP_H, MOUTH_H, MOUTH_W,
};
pub use toast::{toast_box, toast_face, toast_rect, Toast};
