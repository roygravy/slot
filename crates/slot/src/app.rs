use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use slot_gfx::{Canvas, Layout, OUT_H, OUT_W};
use slot_input::{Action, Btn, MENU_DOUBLE_TAP_MS, MUTE_CHORD_MS};
use slot_power::{Battery, Charge, LedState, LidPolicy, Power};
use slot_retro::LinkChannel;
use slot_store::{
    FavoriteMark,
    format_stamp, read_slot_state, scan, write_slot_state, BezelPick, Cart, Core, GbPalette,
    NesColors, Platform, Recent, Shader, SlotState, StateEntry, StateRing, TvCable, WallpaperPick, WallpaperTint,
    AUTO_SLEEP_OPTIONS_SECS, BLUE_LIGHT_MAX, BRIGHTNESS_MAX, FF_SPEEDS,
    VOLUME_MAX,
};
use slot_ui::{
    board_from, board_x, board_zoom, cart_box, draw_empty_slot, draw_footer, draw_ground,
    draw_slot_name, ease, foot_y, footer_ink, grown, lid_at, lid_from, lift_of,
    on_board, repeat_gap, shelf_cart_at, ClockPicker, Draw, TimeZone, FfState, Mould, Hud, HudKind, Icon,
    LinkBadge, Millis, Placed, Polaroids, QuickMenu, QuickMenuFaces, QuickRow,
    QuickValue, Refusal, Shelf, SlotChrome, TexId, Toast, BOARD_W, CART_W, CHIP_H, CHIP_U, CHIP_V,
    CHIP_W, FOOTER_Y, HINT_EDGE, HINT_H, HOP_LIFT, REPEAT_DELAY_MS, SHADOW_H, SHADOW_W, SOCKET_H,
    SOCKET_U, SOCKET_V, SOCKET_W, TURN_PAD,
};

use crate::audio::Sfx;
use crate::core_picker::{Chip, CorePicker, Outcome, Press};
use crate::label_search::{self, LabelSearch};
use crate::shell_picker::{self, ShellPicker};
use crate::link_kind::{link_carried, link_kind, serial_option, LinkKind};
use crate::link_radio::{radio_jobs, LinkRole, RadioJob, RadioJobs};
use crate::link_screen::LinkSprites;
use crate::link_start::{link_port, LinkFail, LinkProgress, LinkStarter, LinkStep};
use crate::persist::{self, Snapshot};
use crate::video_mode::{self, VideoMode};

pub const INSERT_S: f32 = 0.73;
/// How long the screenshot flash takes to fade, and how close a volume step must come
/// before POWER to be undone.
const SHOT_FLASH_MS: Millis = 280;
const SHOT_CHORD_MS: Millis = 400;
/// How long the full-screen picture keeps its hints before they fade.
const SHOT_HINTS_MS: Millis = 2000;
const INSERT_HOLD_S: f32 = 0.28;

pub const SEATED_AT: f32 = INSERT_S - INSERT_HOLD_S;

pub const EJECT_S: f32 = SEATED_AT;

const EJECT_HOLD_S: f32 = 0.35;

pub const IN_GAME_MENU: Layout = Layout::Native;
/// Room between the battery's percentage and the cheats robber in the menu corner.
const MENU_BADGE_GAP: f32 = 4.0;

const POWER_ON_S: f32 = 0.22;

const POWER_OFF_S: f32 = 0.16;

const VOLUME_STEP: u8 = 5;

const AUTOSAVE_MS: Millis = 60_000;
const CARD_CHECK_MS: Millis = 2_000;

const BATTERY_CRITICAL: u8 = 5;

const BATTERY_POLL_MS: Millis = 10_000;
const MENU_REVEAL_MS: Millis = 180;

const CHARGE_POLL_MS: Millis = 1_000;
const HEADPHONES_POLL_MS: Millis = 500;

const BATTERY_LOW: u8 = 20;


pub const LINK_LOST_MS: Millis = 2000;

const PLAY_HOLD_MS: Millis = 500;

const SHUTDOWN_SHOW_MS: Millis = 250;

const CORE_PICKER_RECEDE: f32 = 0.26;

const SLOT_NAME_IN_MS: Millis = 200;
const SLOT_NAME_HOLD_MS: Millis = 1200;
const SLOT_NAME_OUT_MS: Millis = 800;
const SLOT_NAME_ALPHA: f32 = 1.0;
const CORE_PICKER_DIM: f32 = 0.25 / (1.0 - CORE_PICKER_RECEDE);
const CORE_LEGEND_Y: f32 = 386.0;
const LINK_TEXT_Y: f32 = 44.0;
const LINK_LEGEND_Y: f32 = FOOTER_Y;
pub const LINK_SCRIM: [f32; 4] = [0.0, 0.0, 0.0, 0.5];
const LINK_LEGEND_GAP: f32 = 40.0;
const LINK_LEGEND_EDGE: f32 = 24.0;
const LID_SHADOW_W: f32 = 168.0;
const LID_SHADOW_H: f32 = 18.0;
const LID_SHADOW_DROP: f32 = 27.1;
const LID_SHADOW_ALPHA: f32 = 0.8;
const FACES_WAIT_MS: Millis = 1500;

const ALERT_HOLD: f32 = 0.45;
const ALERT_GONE: f32 = 0.9;

const CLOCK_FLOOR: i64 = 1_577_836_800;

struct LinkSession {
    client_id: u16,
    lost_at: Option<Millis>,
}

struct LinkStarting {
    starter: LinkStarter,
    client_id: u16,
}

struct Reload {
    stem: String,
    role: LinkRow,
    cancelled: bool,
    from: LinkKind,
    from_serial: &'static str,
    fallback: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GameMenu {
    Pick(LinkRow),
    Working {
        role: LinkRow,
        step: LinkStep,
        since: Millis,
    },
    Linked {
        role: LinkRow,
        worked: Millis,
        since: Millis,
        opened: bool,
    },
    Failed {
        role: LinkRow,
        fail: LinkFail,
        worked: Millis,
        since: Millis,
    },
    Unplug {
        role: LinkRow,
        since: Millis,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LinkRow {
    Host,
    Join,
}

impl LinkRow {
    pub const ALL: [LinkRow; 2] = [LinkRow::Host, LinkRow::Join];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn text(self) -> &'static str {
        match self {
            LinkRow::Host => "Host",
            LinkRow::Join => "Join",
        }
    }

    pub fn role(self) -> LinkRole {
        match self {
            LinkRow::Host => LinkRole::Host,
            LinkRow::Join => LinkRole::Join,
        }
    }

    pub fn client_id(self) -> u16 {
        match self {
            LinkRow::Host => 0,
            LinkRow::Join => 1,
        }
    }

    pub fn other(self) -> LinkRow {
        match self {
            LinkRow::Host => LinkRow::Join,
            LinkRow::Join => LinkRow::Host,
        }
    }

    pub fn from_client_id(id: u16) -> LinkRow {
        if id == 0 {
            LinkRow::Host
        } else {
            LinkRow::Join
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LinkLegend {
    Cancel,
    Mode,
    Swap,
    Link,
    Ok,
    Back,
    EndLink,
}

impl LinkLegend {
    pub const ALL: [LinkLegend; 7] = [
        LinkLegend::Cancel,
        LinkLegend::Mode,
        LinkLegend::Swap,
        LinkLegend::Link,
        LinkLegend::Ok,
        LinkLegend::Back,
        LinkLegend::EndLink,
    ];

    pub fn index(self) -> usize {
        self as usize
    }
}

pub const LINKED_HOLD_MS: Millis = 1000;

pub const UNPLUG_HOLD_MS: Millis = 420;

#[derive(Debug)]
pub enum Phase {
    SetClock {
        picker: ClockPicker,
        seed: i64,
        from_menu: bool,
    },
    Shelf,
    QuickMenu {
        row: QuickRow,
    },
    Inserting {
        cart: String,
        t: f32,
        core_ready: bool,
        resumed: bool,
        clean: bool,
    },
    Playing {
        cart: String,
    },
    Ejecting {
        cart: String,
        t: f32,
    },
    Polaroids {
        cart: String,
    },
    About,
    Buttons {
        page: usize,
    },
    Doze {
        cart: Option<String>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Rack {
    Recents,
    Favorites,
    Of(Platform),
}

impl Rack {
    fn platform(self) -> Option<Platform> {
        match self {
            Rack::Of(p) => Some(p),
            Rack::Recents | Rack::Favorites => None,
        }
    }

    fn key(self) -> String {
        match self {
            Rack::Recents => "recents".into(),
            Rack::Favorites => "favorites".into(),
            Rack::Of(p) => p.dir_name().to_ascii_lowercase(),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Rack::Recents => "Recents",
            Rack::Favorites => "Favorites",
            Rack::Of(p) => p.name(),
        }
    }
}

const RECENTS_AT: usize = 0;
const FAVORITES_AT: usize = 1;

pub struct App {
    phase: Phase,
    shelves: Vec<(Rack, Shelf)>,
    shelf_at: usize,
    orphaned: Vec<TexId>,
    play_held: Option<Millis>,
    held_dir: Option<(Btn, Millis, usize)>,
    refusal: Option<Refusal>,
    refused_from: Option<f32>,
    alert_face: Option<TexId>,
    /// When POWER went down, while it is held towards powering off.
    power_held_since: Option<Millis>,
    core_picker: Option<CorePicker>,
    shell_picker: Option<ShellPicker>,
    shell_picker_faces: Option<slot_ui::ShellPickerFaces>,
    /// A cart whose label the shell picker asked for, for the frontend to fetch.
    label_ask: Option<Cart>,
    /// The studio's label tab asked for this cart's versions of its label: its own, or those of
    /// the game a search picked.
    versions_ask: Option<(Cart, Option<u64>)>,
    /// The label tab's search by name, over the studio while it is open.
    label_search: Option<LabelSearch>,
    /// A search asked for: its number, the words and the shelf, for the frontend to make.
    search_ask: Option<(u64, String, Platform)>,
    search_faces: Option<slot_ui::KeyboardFaces<slot_ui::Up>>,
    /// The search's words as faces, and the search's rev they were made for.
    search_texts: Option<(u64, slot_ui::SearchTexts<slot_ui::Up>)>,
    /// The versions found: where each is kept while it is looked at, its thumbnail and its
    /// caption.
    version_paths: Vec<std::path::PathBuf>,
    version_thumbs: Vec<(TexId, u32, u32)>,
    version_captions: Vec<(TexId, u32)>,
    /// The label the studio's cart wore when it opened, put back if a version is not kept.
    label_was: Option<std::path::PathBuf>,
    core_board_face: Option<TexId>,
    core_interior_face: Option<TexId>,
    core_lid_face: Option<TexId>,
    core_faces_stem: Option<String>,
    core_socket_faces: Vec<TexId>,
    core_chip_faces: Vec<TexId>,
    core_blank_chip_face: Option<TexId>,
    core_chip_shadow_face: Option<TexId>,
    core_legend_faces: Vec<(TexId, u32)>,
    game_menu: Option<GameMenu>,
    link_sprites: Option<LinkSprites>,
    link_shot: Option<TexId>,
    link_hardware: LinkKind,
    last_role: LinkRow,
    link_choices: HashMap<String, LinkKind>,
    link_loaded: Option<&'static str>,
    link_reload: Option<(String, &'static str)>,
    reload: Option<Reload>,
    link_menu_faces: Vec<(TexId, u32, u32)>,
    link_linked_face: Option<(TexId, u32, u32)>,
    link_step_faces: Vec<(TexId, u32, u32)>,
    link_fail_faces: Vec<(TexId, u32, u32)>,
    link_legend_faces: Vec<(TexId, u32)>,
    starting: Option<LinkStarting>,
    link_transport: Option<(u16, Box<dyn LinkChannel>)>,
    restarting: bool,
    act_at: Millis,
    root: Option<PathBuf>,
    state: SlotState,
    vol_before: Vec<(u8, bool, Millis)>,
    snapshot: Option<Box<dyn Snapshot>>,
    core: Core,
    lcd_ghosting_pending: Option<bool>,
    /// A TV console's look changed: its core is told again.
    tv_changed: bool,
    sgb_border_changed: bool,
    palette_live: bool,
    /// A palette change for the playing game: Some(None) turns the palettes off.
    palette_pending: Option<Option<GbPalette>>,
    /// Whether the playing game can take a palette at all (a Game Boy game, not linked).
    palette_capable: bool,
    link_player: Option<u8>,
    platform: Platform,
    /// The Super Game Boy is in the SNES slot: the Game Boy shelf's games play through it.
    sgb: bool,
    named_core: bool,
    video_mode: VideoMode,
    link: Option<LinkSession>,
    sfx: Option<Sfx>,
    polaroids: Option<Polaroids>,
    legend_faces: Vec<TexId>,
    clock_faces: Option<(TexId, TexId)>,
    quick_menu_faces: Option<QuickMenuFaces>,
    buttons_faces: Option<slot_ui::ButtonsFaces>,
    quick_clock_faces: Option<[(TexId, u32, u32); 2]>,
    quick_network_faces: Option<[(TexId, u32, u32); 2]>,
    quick_zone_faces: Option<[(TexId, u32, u32); 2]>,
    quick_wallpaper_faces: Option<[(TexId, u32, u32); 2]>,
    quick_palette_faces: Option<[(TexId, u32, u32); 2]>,
    quick_bezel_faces: [Option<[(TexId, u32, u32); 2]>; 3],
    game_bezel: Option<TexId>,
    in_game_menu: Option<QuickRow>,
    states_from_menu: bool,
    menu_tap: Option<(Millis, Option<QuickRow>)>,
    sticker_face: Option<TexId>,
    /// The playing game's cheats, which game they belong to, and the Cheats page's cursor
    /// (selected row, first row shown) while it is open.
    cheats: Vec<crate::cheats::Cheat>,
    cheats_for: Option<(Platform, String)>,
    cheat_page: Option<(usize, usize)>,
    cheats_pending: Option<Vec<String>>,
    /// Bumped whenever the list of names changes, so the frontend rebuilds their faces.
    cheats_rev: u64,
    cheat_faces: Option<slot_ui::CheatFaces>,
    /// The robber: in the menu corner, and in play.
    cheat_badges: Option<[(TexId, u32, u32); 2]>,
    /// Which page scrolled last, and the first row it showed, so a list stays still while
    /// the selection moves within it.
    menu_scroll: std::cell::Cell<(usize, usize)>,
    /// Shutting down in a game: draw the game alone for a frame so its picture can be kept.
    capturing: bool,
    /// A screenshot waits for the next frame, drawn without the HUD; then the flash.
    shot_pending: bool,
    shot_flash: Option<Millis>,
    last_shot: Option<PathBuf>,
    /// START+SELECT+L+R asked for the cart to be reset.
    reset_pending: bool,
    /// The Screenshots page, its fixed faces, and the pictures loaded for it.
    shots: Option<crate::shots::ShotView>,
    shot_faces: Option<slot_ui::ShotFaces>,
    shot_pics: Vec<(usize, TexId)>,
    /// Asking before a save state goes.
    state_confirm: Option<slot_ui::ConfirmFaces>,
    boot_picture: Option<(Vec<u8>, u32, u32)>,
    /// Booted back into a game whose picture was the boot screen: show that picture until
    /// the game runs again (`picture_wanted` until the frontend has loaded it).
    picture_wanted: bool,
    resume_picture: Option<TexId>,
    ground: Option<TexId>,
    ground_light: Option<TexId>,
    wallpaper: Option<TexId>,
    random_wallpaper: Option<Option<String>>,
    battery_percent: slot_ui::Printed,
    bolt: Option<TexId>,
    wifi: Option<TexId>,
    hud_wifi: Option<TexId>,
    hud_bolt: Option<TexId>,
    /// GBA dark, GBA light, Game Boy dark, Game Boy light.
    slot_sets: Option<[(TexId, TexId); 8]>,
    nes_lids: Option<[TexId; 2]>,
    shelf_platform: slot_ui::Printed,
    shelf_title: slot_ui::Printed,
    name_pending: bool,
    slot_letter: Option<char>,
    shelf_named: Option<Millis>,
    shelf_clock: slot_ui::Printed,
    hud: Hud,
    screen: f32,
    lift: f32,
    eject_overlay: f32,
    panel: (u32, u32),
    game_ready: bool,
    clock: f64,
    last_activity: Millis,
    power: Option<Power>,
    dozed_at: Millis,
    woke_on_press: bool,
    autosave_at: Millis,
    pending_save: Option<std::thread::JoinHandle<()>>,
    card_writes: Arc<crate::card_log::CardWrites>,
    resume_unread: bool,
    card_read_only: bool,
    card_check_at: Millis,
    battery_at: Millis,
    charge_at: Millis,
    headphones: bool,
    headphones_at: Millis,
    battery: Option<Battery>,
    last_led: Option<LedState>,
    powering_off: bool,
    radio: Box<dyn RadioJobs>,
    state_writer: crate::state_writer::StateWriter,
}

const FACE_AHEAD: i32 = 8;
const FACE_KEEP: i32 = 12;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FaceWant {
    pub shelf: usize,
    pub index: usize,
    pub on_screen: bool,
}

fn shelves_of(carts: Vec<Cart>) -> Vec<(Rack, Shelf)> {
    let mut rows: Vec<(Rack, Vec<Cart>)> = [Rack::Recents, Rack::Favorites]
        .into_iter()
        .chain(Platform::ALL.iter().map(|p| Rack::Of(*p)))
        .map(|r| (r, Vec::new()))
        .collect();
    for cart in carts {
        if let Some((_, row)) = rows.iter_mut().find(|(r, _)| *r == Rack::Of(cart.platform)) {
            row.push(cart);
        }
    }
    rows.into_iter()
        .map(|(rack, carts)| (rack, Shelf::new(carts)))
        .collect()
}

impl App {
    pub fn new(carts: Vec<Cart>) -> Self {
        let shelves = shelves_of(carts);
        // The first shelf with carts on it: GB, then GBC, then GBA. An empty GB shelf would
        // leave a card without GB games on a bare shelf, and a single cart out of reach.
        let shelf_at = shelves
            .iter()
            .position(|(_, s)| !s.carts.is_empty())
            .unwrap_or(0);
        let state_writer = crate::state_writer::StateWriter::spawn();
        let card_writes = state_writer.card_writes();
        App {
            radio: radio_jobs(),
            state_writer,
            phase: Phase::Shelf,
            shelves,
            shelf_at,
            orphaned: Vec::new(),
            play_held: None,
            held_dir: None,
            refusal: None,
            refused_from: None,
            alert_face: None,
            power_held_since: None,
            core_picker: None,
            shell_picker: None,
            shell_picker_faces: None,
            label_ask: None,
            versions_ask: None,
            label_search: None,
            search_ask: None,
            search_faces: None,
            search_texts: None,
            version_paths: Vec::new(),
            version_thumbs: Vec::new(),
            version_captions: Vec::new(),
            label_was: None,
            core_board_face: None,
            core_interior_face: None,
            core_lid_face: None,
            core_faces_stem: None,
            core_socket_faces: Vec::new(),
            core_chip_faces: Vec::new(),
            core_blank_chip_face: None,
            core_chip_shadow_face: None,
            core_legend_faces: Vec::new(),
            game_menu: None,
            link_sprites: None,
            link_shot: None,
            link_hardware: LinkKind::Cable,
            last_role: LinkRow::Host,
            link_choices: HashMap::new(),
            link_loaded: None,
            link_reload: None,
            reload: None,
            link_menu_faces: Vec::new(),
            link_linked_face: None,
            link_step_faces: Vec::new(),
            link_fail_faces: Vec::new(),
            link_legend_faces: Vec::new(),
            starting: None,
            link_transport: None,
            restarting: false,
            act_at: 0,
            root: None,
            state: SlotState::default(),
            vol_before: Vec::new(),
            snapshot: None,
            core: Core::default(),
            lcd_ghosting_pending: None,
            tv_changed: false,
            sgb_border_changed: false,
            palette_live: false,
            palette_pending: None,
            palette_capable: false,
            link_player: None,
            platform: Platform::default(),
            sgb: false,
            named_core: false,
            video_mode: VideoMode::default(),
            link: None,
            sfx: None,
            polaroids: None,
            legend_faces: Vec::new(),
            clock_faces: None,
            quick_menu_faces: None,
            quick_clock_faces: None,
            quick_network_faces: None,
            quick_zone_faces: None,
            quick_wallpaper_faces: None,
            buttons_faces: None,
            quick_palette_faces: None,
            quick_bezel_faces: [None, None, None],
            game_bezel: None,
            in_game_menu: None,
            states_from_menu: false,
            menu_tap: None,
            sticker_face: None,
            cheats: Vec::new(),
            cheats_for: None,
            cheat_page: None,
            cheats_pending: None,
            cheats_rev: 0,
            cheat_faces: None,
            cheat_badges: None,
            menu_scroll: std::cell::Cell::new((usize::MAX, 0)),
            capturing: false,
            shot_pending: false,
            shot_flash: None,
            last_shot: None,
            reset_pending: false,
            shots: None,
            shot_faces: None,
            shot_pics: Vec::new(),
            state_confirm: None,
            boot_picture: None,
            picture_wanted: false,
            resume_picture: None,
            ground: None,
            ground_light: None,
            wallpaper: None,
            random_wallpaper: None,
            battery_percent: slot_ui::Printed::default(),
            bolt: None,
            wifi: None,
            hud_wifi: None,
            hud_bolt: None,
            slot_sets: None,
            nes_lids: None,
            shelf_platform: slot_ui::Printed::default(),
            shelf_title: slot_ui::Printed::default(),
            name_pending: false,
            slot_letter: None,
            shelf_named: None,
            shelf_clock: slot_ui::Printed::default(),
            hud: Hud::new(),
            screen: 0.0,
            lift: 1.0,
            eject_overlay: 0.0,
            panel: (0, 0),
            game_ready: false,
            clock: 0.0,
            last_activity: 0,
            power: None,
            dozed_at: 0,
            woke_on_press: false,
            autosave_at: AUTOSAVE_MS,
            pending_save: None,
            card_writes,
            resume_unread: false,
            card_read_only: false,
            card_check_at: 0,
            battery_at: BATTERY_POLL_MS,
            charge_at: CHARGE_POLL_MS,
            headphones: false,
            headphones_at: 0,
            battery: None,
            last_led: None,
            powering_off: false,
        }
    }

    pub fn boot(root: &Path) -> Self {
        crate::root::ensure(root);
        crate::boot_time::mark("root");
        let carts = scan(root).unwrap_or_default();
        crate::boot_time::mark("scan");
        let mut app = App::new(carts);
        app.root = Some(root.to_path_buf());
        app.state = read_slot_state(root);
        app.radio.ask(RadioJob::Home(app.state.home_wifi_enabled));
        app.radio.ask(RadioJob::Time(app.state.auto_time));
        app.restock_recents();
        app.restock_favorites();
        if app.state.remember_position {
            if let Some((at, i)) = app.remembered_seat() {
                app.shelf_at = at;
                app.shelves[at].1.select(i);
            }
        } else if !app.shelves[FAVORITES_AT].1.carts.is_empty() {
            app.shelf_at = FAVORITES_AT;
        } else if app.state.show_recents && !app.shelves[RECENTS_AT].1.carts.is_empty() {
            app.shelf_at = RECENTS_AT;
        }
        if app.state.clock_set {
            app.start();
        } else {
            app.phase = clock_screen(system_secs(), app.time_zone(), app.state.clock_24h, false);
        }
        app
    }

    fn start(&mut self) {
        let seated = if self.single_cart() {
            Some((self.shelf_at, 0))
        } else {
            let stem = self.state.cart.clone();
            let platform = self.state.cart_platform;
            stem.and_then(|stem| {
                let shelf = &self.shelf().carts;
                let here = shelf
                    .iter()
                    .position(|c| c.stem == stem && platform.is_none_or(|p| c.platform == p));
                here.map(|i| (self.shelf_at, i))
                    .or_else(|| self.seat_of(&stem, platform))
            })
        };
        self.phase = Phase::Shelf;
        if !self.state.resume_game && !self.single_cart() {
            // Start on the shelf, the last game picked out but left out of the slot.
            if let Some((at, i)) = seated {
                self.shelf_at = at;
                self.shelf_mut().select(i);
            }
            self.state.cart = None;
            self.state.cart_platform = None;
            self.name_pending = true;
            return;
        }
        match seated {
            Some((at, i)) => {
                self.shelf_at = at;
                self.shelf_mut().select(i);
                // A Game Boy game left in the Super Game Boy goes back into it.
                self.sgb = self.state.cart_sgb
                    && self.shelf().carts.get(i).is_some_and(|c| c.platform == Platform::Gb)
                    && self.carts().any(Cart::is_sgb);
                self.insert(false);
                if let Phase::Inserting { resumed, t, .. } = &mut self.phase {
                    *resumed = true;
                    *t = INSERT_S;
                }
                // Resuming into a game: its picture is the boot screen.
                self.picture_wanted = true;
            }
            None => {
                self.state.cart = None;
                self.state.cart_platform = None;
                self.name_pending = true;
            }
        }
    }

    fn shelf(&self) -> &Shelf {
        &self.shelves[self.shelf_at].1
    }

    fn shelf_mut(&mut self) -> &mut Shelf {
        &mut self.shelves[self.shelf_at].1
    }

    fn switch_shelf(&mut self, by: i32) {
        // With the Super Game Boy in, the Game Boy shelf is the only one; B takes it out.
        if self.sgb {
            return self.refuse();
        }
        let Some(to) = self.next_shelf(by) else {
            return;
        };
        self.shelf_mut().release_hold();
        self.play_held = None;
        self.shelf_at = to;
        self.shelf_platform = slot_ui::Printed::default();
        self.slot_letter = None;
        self.shelf_named = None;
        self.name_pending = true;
    }

    fn track_held_dir(&mut self, action: Action) {
        match action {
            Action::GbaDown(b @ (Btn::Up | Btn::Down | Btn::Left | Btn::Right)) => {
                if self.held_dir.map(|(held, ..)| held) != Some(b) {
                    self.held_dir = Some((b, self.now() + REPEAT_DELAY_MS, 0));
                }
            }
            Action::GbaUp(b) if self.held_dir.is_some_and(|(held, ..)| held == b) => {
                self.held_dir = None;
            }
            _ => {}
        }
    }

    fn repeat_held_dir(&mut self) {
        let Some((btn, due, fired)) = self.held_dir else {
            return;
        };
        if self.now() < due {
            return;
        }
        if !self.repeats(btn) {
            self.held_dir = None;
            return;
        }
        self.held_dir = Some((btn, self.now() + repeat_gap(fired), fired + 1));
        self.apply(Action::GbaDown(btn));
    }

    fn repeats(&self, btn: Btn) -> bool {
        let vertical = matches!(btn, Btn::Up | Btn::Down);
        if self.game_menu.is_some() {
            return false;
        }
        match self.phase {
            Phase::SetClock { .. } => true,
            Phase::QuickMenu { .. } => vertical,
            Phase::Shelf => vertical && self.core_picker.is_none() && self.shell_picker.is_none(),
            Phase::Polaroids { .. } => !vertical,
            Phase::Playing { .. } => vertical && self.in_game_menu.is_some(),
            _ => false,
        }
    }

    fn jump_letter(&mut self, dir: i32) {
        let before = self.shelf().index;
        match dir > 0 {
            true => self.shelf_mut().jump_next_letter(),
            false => self.shelf_mut().jump_prev_letter(),
        }
        if self.shelf().index == before {
            return;
        }
        let Some(letter) = self.selected_stem().map(slot_store::initial) else {
            return;
        };
        if self.slot_letter != Some(letter) {
            self.shelf_platform = slot_ui::Printed::default();
        }
        self.slot_letter = Some(letter);
        self.shelf_named = None;
        self.name_pending = true;
    }

    fn slot_name_alpha(&self) -> f32 {
        let Some(at) = self.shelf_named else {
            return 0.0;
        };
        let t = self.now().saturating_sub(at);
        let level = if t < SLOT_NAME_IN_MS {
            t as f32 / SLOT_NAME_IN_MS as f32
        } else if t < SLOT_NAME_IN_MS + SLOT_NAME_HOLD_MS {
            1.0
        } else {
            let out = t - SLOT_NAME_IN_MS - SLOT_NAME_HOLD_MS;
            1.0 - (out as f32 / SLOT_NAME_OUT_MS as f32).min(1.0)
        };
        SLOT_NAME_ALPHA * ease(level)
    }

    fn next_shelf(&self, by: i32) -> Option<usize> {
        let n = self.shelves.len() as i32;
        (1..n)
            .map(|step| (self.shelf_at as i32 + by * step).rem_euclid(n) as usize)
            .find(|at| !self.shelves[*at].1.carts.is_empty())
    }

    fn remembered_seat(&self) -> Option<(usize, usize)> {
        let key = self.state.shelf.as_deref()?;
        let cart = self.state.shelf_cart.as_ref()?;
        let at = self.shelves.iter().position(|(r, _)| r.key() == key)?;
        let i = self.shelves[at]
            .1
            .carts
            .iter()
            .position(|c| c.platform == cart.platform && c.stem == cart.stem)?;
        Some((at, i))
    }

    fn remember_shelf(&mut self) {
        if !self.state.remember_position {
            return;
        }
        let Some(cart) = self.shelf().carts.get(self.shelf().index) else {
            return;
        };
        self.state.shelf_cart = Some(Recent {
            platform: cart.platform,
            stem: cart.stem.clone(),
        });
        self.state.shelf = Some(self.shelves[self.shelf_at].0.key());
        self.persist();
    }

    fn seat_of(&self, stem: &str, platform: Option<Platform>) -> Option<(usize, usize)> {
        self.shelves
            .iter()
            .enumerate()
            .filter(|(_, (r, _))| {
                platform.is_none_or(|want| r.platform().is_none_or(|p| p == want))
            })
            .find_map(|(at, (_, shelf))| {
                shelf
                    .carts
                    .iter()
                    .position(|c| c.stem == stem && platform.is_none_or(|p| c.platform == p))
                    .map(|i| (at, i))
            })
    }

    pub fn confirm_clock(&mut self) {
        let Phase::SetClock {
            picker,
            seed,
            from_menu,
        } = &self.phase
        else {
            return;
        };
        let (moved, zone, from_menu) = (picker.secs() - *seed, picker.zone(), *from_menu);
        let utc = self.utc_secs() + moved;
        if let Some(power) = &mut self.power {
            power.set_clock(utc);
        }
        if let TimeZone::Fixed(offset) = zone {
            self.state.utc_offset_min = offset as i16;
        }
        self.state.time_zone = zone.city();
        self.state.clock_set = true;
        self.persist();
        if from_menu {
            self.phase = Phase::QuickMenu {
                row: QuickRow::DateTime,
            };
        } else {
            self.start();
        }
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub fn wall_secs(&self) -> i64 {
        let utc = self.utc_secs();
        utc + self.time_zone().offset_at(utc) * 60
    }

    fn time_zone(&self) -> TimeZone {
        TimeZone::from_state(self.state.time_zone, self.state.utc_offset_min)
    }

    fn utc_secs(&self) -> i64 {
        self.power.as_ref().map_or_else(system_secs, |p| p.now())
    }

    pub fn picker(&self) -> Option<&ClockPicker> {
        match &self.phase {
            Phase::SetClock { picker, .. } => Some(picker),
            _ => None,
        }
    }

    pub fn quick_menu(&self) -> Option<QuickRow> {
        match self.phase {
            Phase::QuickMenu { row } => Some(row),
            _ => None,
        }
    }

    pub fn in_game_menu(&self) -> Option<QuickRow> {
        self.in_game_menu
    }

    /// The in-game rows this game has no use for.
    pub fn in_game_hidden(&self) -> Vec<QuickRow> {
        let platform = self.playing_platform();
        let mut hidden = Vec::new();
        if !self.palette_capable {
            hidden.push(QuickRow::Palette);
        }
        if platform == Platform::Gba && !self.framed_panel() {
            hidden.push(QuickRow::Bezel);
        }
        if platform == Platform::Gba {
            hidden.push(QuickRow::BlackBorder);
        }
        // A TV console has no LCD to dress up: none of the handhelds' screen rows apply, so
        // there is no Screen page to open either.
        if platform.tv() {
            hidden.extend([
                QuickRow::Shader,
                QuickRow::Palette,
                QuickRow::Bezel,
                QuickRow::PixelTransparency,
                QuickRow::LcdGhosting,
                QuickRow::BlackBorder,
            ]);
        }
        if !platform.tv() {
            hidden.extend(TV_ROWS);
        }
        if !self.sgb_playing() {
            hidden.push(QuickRow::SgbBorder);
        }
        // Only the NES's picture has colours a TV made up.
        if platform != Platform::Nes {
            hidden.push(QuickRow::Colors);
        }
        if self.single_cart() {
            hidden.push(QuickRow::ExitGame);
        }
        hidden
    }

    pub fn gb_palette_label(&self) -> &'static str {
        match self.state.gb_palettes {
            true => self.state.gb_palette.label(),
            false => "Off",
        }
    }

    pub fn set_quick_palette_faces(&mut self, dim: (TexId, u32, u32), lit: (TexId, u32, u32)) {
        self.quick_palette_faces = Some([dim, lit]);
    }

    fn playing_row(&self, row: QuickRow) -> QuickRow {
        match (row, self.playing_platform()) {
            (QuickRow::Shader, Platform::Gba) => QuickRow::GbaShader,
            (QuickRow::Shader, Platform::Gb) => QuickRow::GbShader,
            (QuickRow::Shader, Platform::Gbc) => QuickRow::GbcShader,
            (QuickRow::Bezel, Platform::Gba) => QuickRow::GbaBezel,
            (QuickRow::Bezel, Platform::Gb) => QuickRow::GbBezel,
            (QuickRow::Bezel, Platform::Gbc) => QuickRow::GbcBezel,
            (QuickRow::PixelTransparency, Platform::Gb) => QuickRow::GbTransparency,
            (QuickRow::PixelTransparency, Platform::Gbc) => QuickRow::GbcTransparency,
            (QuickRow::PixelTransparency, Platform::Gba) => QuickRow::GbaTransparency,
            (QuickRow::LcdGhosting, Platform::Gb) => QuickRow::GbGhosting,
            (QuickRow::LcdGhosting, Platform::Gbc) => QuickRow::GbcGhosting,
            (QuickRow::LcdGhosting, Platform::Gba) => QuickRow::GbaGhosting,
            (QuickRow::BlackBorder, Platform::Gb) => QuickRow::GbBlackBorder,
            (QuickRow::BlackBorder, Platform::Gbc) => QuickRow::GbcBlackBorder,
            (QuickRow::Cable, Platform::Nes) => QuickRow::NesCable,
            (QuickRow::Cable, Platform::Snes) => QuickRow::SnesCable,
            (QuickRow::Colors, Platform::Nes) => QuickRow::NesColors,
            (QuickRow::Scanlines, Platform::Nes) => QuickRow::NesScanlines,
            (QuickRow::Scanlines, Platform::Snes) => QuickRow::SnesScanlines,
            (QuickRow::Mask, Platform::Nes) => QuickRow::NesMask,
            (QuickRow::Mask, Platform::Snes) => QuickRow::SnesMask,
            (QuickRow::Afterglow, Platform::Nes) => QuickRow::NesAfterglow,
            (QuickRow::Afterglow, Platform::Snes) => QuickRow::SnesAfterglow,
            (row, _) => row,
        }
    }

    pub fn quick_value(&self, row: QuickRow) -> Option<QuickValue> {
        match row {
            QuickRow::Shader
            | QuickRow::PixelTransparency
            | QuickRow::LcdGhosting
            | QuickRow::BlackBorder
            | QuickRow::Cable
            | QuickRow::Colors
            | QuickRow::Scanlines
            | QuickRow::Mask
            | QuickRow::Afterglow => match self.playing_row(row) {
                stood_in if stood_in != row => self.quick_value(stood_in),
                _ => None,
            },
            QuickRow::SgbBorder => Some(match self.state.sgb_border {
                1 => QuickValue::BorderGravy,
                2 => QuickValue::Off,
                n @ 11..=19 => QuickValue::ALL[QuickValue::Border1.index() + usize::from(n - 11)],
                _ => QuickValue::BorderAuto,
            }),
            QuickRow::NesCable => Some(cable_value(self.state.tv_cable_nes)),
            QuickRow::SnesCable => Some(cable_value(self.state.tv_cable_snes)),
            QuickRow::NesColors => Some(match self.state.nes_colors {
                NesColors::Standard => QuickValue::Standard,
                NesColors::Sony => QuickValue::Sony,
                NesColors::Nescap => QuickValue::Nescap,
                NesColors::Pvm => QuickValue::Pvm,
            }),
            QuickRow::NesScanlines => Some(QuickValue::flag(self.state.tv_scanlines_nes)),
            QuickRow::SnesScanlines => Some(QuickValue::flag(self.state.tv_scanlines_snes)),
            QuickRow::NesMask => Some(QuickValue::flag(self.state.tv_mask_nes)),
            QuickRow::SnesMask => Some(QuickValue::flag(self.state.tv_mask_snes)),
            QuickRow::NesAfterglow => Some(QuickValue::flag(self.state.tv_afterglow_nes)),
            QuickRow::SnesAfterglow => Some(QuickValue::flag(self.state.tv_afterglow_snes)),
            QuickRow::ScreenNes | QuickRow::ScreenSnes => None,
            QuickRow::FastForward => QuickValue::speed(self.state.ff_speed),
            QuickRow::FastForwardSound => Some(QuickValue::flag(self.state.ff_sound)),
            QuickRow::GbTransparency => Some(QuickValue::flag(self.state.pixel_transparency_gb)),
            QuickRow::GbcTransparency => Some(QuickValue::flag(self.state.pixel_transparency_gbc)),
            QuickRow::GbaTransparency => Some(QuickValue::flag(self.state.pixel_transparency_gba)),
            QuickRow::GbGhosting => Some(QuickValue::flag(self.state.lcd_ghosting_gb)),
            QuickRow::GbcGhosting => Some(QuickValue::flag(self.state.lcd_ghosting_gbc)),
            QuickRow::GbaGhosting => Some(QuickValue::flag(self.state.lcd_ghosting_gba)),
            QuickRow::GbaShader => Some(QuickValue::flag(self.state.shader_gba != Shader::Off)),
            QuickRow::GbShader => Some(QuickValue::flag(self.state.shader_gb != Shader::Off)),
            QuickRow::GbcShader => Some(QuickValue::flag(self.state.shader_gbc != Shader::Off)),
            QuickRow::Rumble => Some(QuickValue::flag(self.state.rumble)),
            QuickRow::HomeWifi => Some(QuickValue::flag(self.state.home_wifi_enabled)),
            QuickRow::AutoTime => Some(QuickValue::flag(self.state.auto_time)),
            QuickRow::Clock24 => Some(QuickValue::flag(self.state.clock_24h)),
            QuickRow::CheatIcon => Some(QuickValue::flag(self.state.cheat_icon)),
            QuickRow::GbcBlackBorder => Some(QuickValue::flag(self.state.gbc_black_border)),
            QuickRow::GbBlackBorder => Some(QuickValue::flag(self.state.gb_black_border)),
            QuickRow::EjectSave => Some(QuickValue::flag(self.state.eject_save)),
            QuickRow::Turbo => Some(QuickValue::flag(self.state.turbo)),
            QuickRow::Rewind => Some(QuickValue::flag(self.state.rewind)),

            QuickRow::Recents => Some(if self.state.show_recents {
                QuickValue::Show
            } else {
                QuickValue::Hide
            }),
            QuickRow::ShowTitles => Some(if self.state.show_titles {
                QuickValue::Show
            } else {
                QuickValue::Hide
            }),
            QuickRow::WallpaperTint => Some(match self.state.wallpaper_tint {
                WallpaperTint::Off => QuickValue::Off,
                WallpaperTint::Low => QuickValue::Low,
                WallpaperTint::Medium => QuickValue::Medium,
                WallpaperTint::High => QuickValue::High,
            }),
            QuickRow::FavoriteMark => Some(match self.state.favorite_mark {
                FavoriteMark::GoldGlow | FavoriteMark::WhiteGlow => QuickValue::GoldGlow,
                FavoriteMark::Star => QuickValue::StarMark,
            }),
            QuickRow::Theme => Some(match self.state.appearance {
                slot_store::Appearance::Dark => QuickValue::Dark,
                slot_store::Appearance::Light => QuickValue::Light,
            }),
            QuickRow::DateTime
            | QuickRow::TimeZone
            | QuickRow::About
            | QuickRow::Screenshots
            | QuickRow::Screen
            | QuickRow::Game
            | QuickRow::General
            | QuickRow::Network
            | QuickRow::ConnectedTo
            | QuickRow::Wallpaper
            | QuickRow::Resume
            | QuickRow::States
            | QuickRow::Cheats
            | QuickRow::GameScreenshots
            | QuickRow::Settings
            | QuickRow::ExitGame
            | QuickRow::PlayScreen
            | QuickRow::PlayGameplay
            | QuickRow::Palette
            | QuickRow::GbPalettes
            | QuickRow::ScreenGba
            | QuickRow::ScreenGbc
            | QuickRow::ScreenGb
            | QuickRow::GbBezel
            | QuickRow::GbcBezel
            | QuickRow::GbaBezel
            | QuickRow::Bezel
            | QuickRow::Buttons
            | QuickRow::Sleep
            | QuickRow::Restart
            | QuickRow::PowerOff => None,
            QuickRow::AutoSleep => Some(auto_sleep_value(self.state.auto_sleep_secs)),
            QuickRow::InGameMenu => Some(if self.state.in_game_menu {
                QuickValue::Show
            } else {
                QuickValue::Hide
            }),
            QuickRow::ResumeGame => Some(QuickValue::flag(self.state.resume_game)),
            QuickRow::BootPicture => Some(QuickValue::flag(self.state.boot_picture)),
            QuickRow::RememberPosition => Some(if self.state.remember_position {
                QuickValue::Show
            } else {
                QuickValue::Hide
            }),
        }
    }

    pub fn set_buttons_faces(&mut self, faces: slot_ui::ButtonsFaces) {
        self.buttons_faces = Some(faces);
    }

    pub fn set_quick_menu_faces(&mut self, faces: QuickMenuFaces) {
        self.quick_menu_faces = Some(faces);
    }

    pub fn set_quick_zone_faces(&mut self, dim: (TexId, u32, u32), lit: (TexId, u32, u32)) {
        self.quick_zone_faces = Some([dim, lit]);
    }

    /// The clock's city for Time Zone, or its bare offset from before cities.
    pub fn zone_name(&self) -> String {
        match self.time_zone() {
            TimeZone::City(zone) => zone.name.to_string(),
            TimeZone::Fixed(min) => {
                let sign = if min < 0 { '-' } else { '+' };
                format!("UTC{sign}{:02}:{:02}", min.abs() / 60, min.abs() % 60)
            }
        }
    }

    pub fn clock_24h(&self) -> bool {
        self.state.clock_24h
    }

    pub fn set_quick_network_faces(&mut self, dim: (TexId, u32, u32), lit: (TexId, u32, u32)) {
        self.quick_network_faces = Some([dim, lit]);
    }

    /// The home network's name while Home Wi-Fi is on and connected.
    pub fn home_network(&self) -> Option<String> {
        self.state
            .home_wifi_enabled
            .then(|| self.radio.home_ssid())
            .flatten()
    }

    pub fn set_quick_clock_faces(&mut self, dim: (TexId, u32, u32), lit: (TexId, u32, u32)) {
        self.quick_clock_faces = Some([dim, lit]);
    }

    pub fn set_quick_wallpaper_faces(&mut self, dim: (TexId, u32, u32), lit: (TexId, u32, u32)) {
        self.quick_wallpaper_faces = Some([dim, lit]);
    }

    pub fn set_quick_bezel_faces(
        &mut self,
        platform: Platform,
        dim: (TexId, u32, u32),
        lit: (TexId, u32, u32),
    ) {
        self.quick_bezel_faces[bezel_slot(platform)] = Some([dim, lit]);
    }

    pub fn bezel_files(&self, platform: Platform) -> Vec<String> {
        match platform {
            Platform::Gba => Vec::new(),
            _ => self
                .root()
                .map(|root| crate::bezel::list(root, platform))
                .unwrap_or_default(),
        }
    }

    fn bezel_choices(&self, _platform: Platform) -> Vec<BezelPick> {
        vec![BezelPick::None]
    }

    fn bezel_pick(&mut self, platform: Platform) -> &mut BezelPick {
        match platform {
            Platform::Gba | Platform::Nes | Platform::Snes => &mut self.state.bezel_gba,
            Platform::Gb => &mut self.state.bezel_gb,
            Platform::Gbc => &mut self.state.bezel_gbc,
        }
    }

    pub fn resolved_bezel(&self, platform: Platform) -> BezelPick {
        let pick = match platform {
            Platform::Gba | Platform::Nes | Platform::Snes => &self.state.bezel_gba,
            Platform::Gb => &self.state.bezel_gb,
            Platform::Gbc => &self.state.bezel_gbc,
        };
        match self.bezel_choices(platform).contains(pick) {
            true => pick.clone(),
            false => BezelPick::None,
        }
    }

    fn cycle_bezel(&mut self, platform: Platform, right: bool) {
        let choices = self.bezel_choices(platform);
        let key = self.resolved_bezel(platform).key();
        let at = choices.iter().position(|c| c.key() == key).unwrap_or(0);
        let n = choices.len();
        *self.bezel_pick(platform) = choices[if right {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        }]
        .clone();
    }

    pub fn sp_frame(&self) -> bool {
        self.resolved_bezel(self.playing_platform()) == BezelPick::Ags102
    }

    pub fn set_game_bezel(&mut self, tex: Option<TexId>) {
        self.game_bezel = tex;
    }

    pub fn shown_game_bezel(&self) -> Option<TexId> {
        if self.video_mode == VideoMode::Stretch {
            return None;
        }
        self.game_bezel
    }

    pub fn wallpaper_files(&self) -> Vec<String> {
        self.root().map(crate::wallpaper::list).unwrap_or_default()
    }

    pub fn wallpaper(&self) -> WallpaperPick {
        self.state.wallpaper.clone()
    }

    /// What Wallpaper shows: Dark, Light, one of your own, or Off. A card still on Default
    /// (or the old blur) shows the theme's own wallpaper, and so does your own file while it
    /// cannot be loaded.
    pub fn resolved_wallpaper(&self) -> WallpaperPick {
        let theme = if self.light() { WallpaperPick::Light } else { WallpaperPick::Dark };
        match &self.state.wallpaper {
            WallpaperPick::None => WallpaperPick::None,
            WallpaperPick::Dark => WallpaperPick::Dark,
            WallpaperPick::Light => WallpaperPick::Light,
            WallpaperPick::File(name) => WallpaperPick::File(name.clone()),
            WallpaperPick::Random => WallpaperPick::Random,
            _ => theme,
        }
    }

    pub fn shown_wallpaper(&mut self) -> Option<String> {
        match self.resolved_wallpaper() {
            WallpaperPick::File(name) => Some(name),
            WallpaperPick::Random => self.random_wallpaper(),
            _ => None,
        }
    }

    fn random_wallpaper(&mut self) -> Option<String> {
        if self.random_wallpaper.is_none() {
            let seed = self.wall_secs().unsigned_abs();
            let pick = self
                .root()
                .and_then(|root| crate::wallpaper::pick(root, seed))
                .and_then(|path| Some(path.file_name()?.to_str()?.to_owned()));
            self.random_wallpaper = Some(pick);
        }
        self.random_wallpaper.clone().flatten()
    }

    fn cycle_wallpaper(&mut self, right: bool) {
        let mut choices = vec![WallpaperPick::Dark, WallpaperPick::Light];
        choices.extend(self.wallpaper_files().into_iter().map(WallpaperPick::File));
        choices.push(WallpaperPick::None);
        let key = self.resolved_wallpaper().key();
        let at = choices.iter().position(|c| c.key() == key).unwrap_or(0);
        let n = choices.len();
        let next = if right {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        };
        self.state.wallpaper = choices[next].clone();
        self.mark_stars();
    }

    /// The wallpaper behind the shelf and menus: the theme's own default, or none (black) when
    /// Wallpaper is Off.
    fn ground_face(&self) -> Option<TexId> {
        match self.resolved_wallpaper() {
            WallpaperPick::None => None,
            WallpaperPick::Light => self.ground_light.or(self.ground),
            WallpaperPick::File(_) | WallpaperPick::Random if self.wallpaper.is_some() => {
                self.wallpaper
            }
            WallpaperPick::File(_) | WallpaperPick::Random if self.light() => {
                self.ground_light.or(self.ground)
            }
            _ => self.ground,
        }
    }

    /// Which About back to show: the one matching the wallpaper, Dark or Light; with your own
    /// wallpaper or none, the theme's.
    pub fn about_light(&self) -> bool {
        match self.resolved_wallpaper() {
            WallpaperPick::Dark => false,
            WallpaperPick::Light => true,
            _ => self.light(),
        }
    }

    /// Whether the wallpaper behind the shelf and menus is a light one, so text and shadows on
    /// it go dark. Your own wallpaper is taken to suit the theme you picked.
    fn light_ground(&self) -> bool {
        match self.resolved_wallpaper() {
            WallpaperPick::None | WallpaperPick::Dark => false,
            WallpaperPick::Light => true,
            _ => self.light(),
        }
    }

    fn menu_ground(&self) -> Option<TexId> {
        self.ground_face()
    }

    /// Dark ink on a light wallpaper, light ink on a dark one or on black (Wallpaper Off).
    pub fn menu_ink(&self) -> slot_ui::theme::Ink {
        let mut ink = slot_ui::theme::ink(self.light_ground());
        // On the Dark or Light wallpaper the menus sit on a flat ground of its own tone.
        if matches!(self.resolved_wallpaper(), WallpaperPick::Dark | WallpaperPick::Light) {
            ink.wash = slot_ui::theme::menu_flat(self.light_ground());
        }
        ink
    }

    fn draw_background(&self, out: &mut Vec<Draw>) {
        let face = self.ground_face();
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: self.canvas().w,
            h: OUT_H as f32,
            colour: [0.0, 0.0, 0.0, 1.0],
        });
        draw_ground(self.canvas(), face, out);
        let alpha = tint_alpha(self.state.wallpaper_tint);
        if alpha > 0.0 && self.own_wallpaper_shown() {
            out.push(Draw::Rect {
                x: 0.0,
                y: 0.0,
                w: self.canvas().w,
                h: OUT_H as f32,
                colour: [0.0, 0.0, 0.0, alpha],
            });
        }
    }

    fn own_wallpaper_shown(&self) -> bool {
        match self.state.wallpaper {
            WallpaperPick::File(_) => self.wallpaper.is_some(),
            WallpaperPick::Random => self.random_drawn() && self.wallpaper.is_some(),
            _ => false,
        }
    }

    pub fn hidden_rows(&self) -> &'static [QuickRow] {
        let own = matches!(
            self.state.wallpaper,
            WallpaperPick::File(_) | WallpaperPick::Random
        );
        match (own, self.framed_panel()) {
            (true, true) => &[],
            (true, false) => &[QuickRow::GbaBezel],
            (false, true) => &[QuickRow::WallpaperTint],
            (false, false) => &[QuickRow::WallpaperTint, QuickRow::GbaBezel],
        }
    }

    fn random_drawn(&self) -> bool {
        matches!(self.random_wallpaper, Some(Some(_)))
    }

    pub fn set_sticker_face(&mut self, face: TexId) {
        self.sticker_face = Some(face);
    }

    pub fn set_clock_faces(&mut self, line: TexId, hint: TexId) {
        self.clock_faces = Some((line, hint));
    }

    /// Per mould, in `Mould::ALL` order.
    pub fn set_mark_faces(
        &mut self,
        gold: [TexId; Mould::ALL.len()],
        white: [TexId; Mould::ALL.len()],
        star: (TexId, u32, u32),
    ) {
        for (_, shelf) in &mut self.shelves {
            for (style, faces) in [(FavoriteMark::GoldGlow, gold), (FavoriteMark::WhiteGlow, white)] {
                for (mould, face) in Mould::ALL.into_iter().zip(faces) {
                    shelf.set_glow(style, mould, face);
                }
            }
            shelf.set_star(star.0, star.1, star.2);
        }
    }

    fn mark_stars(&mut self) {
        let light_ground = self.light_ground();
        slot_ui::set_light_ground(light_ground);
        let state = &self.state;
        for (rack, shelf) in &mut self.shelves {
            shelf.set_mark(state.favorite_mark);
            // The Favorites shelf is all favourites already: marking each one only adds noise.
            let on_favorites = *rack == Rack::Favorites;
            shelf.mark_starred(|c| !on_favorites && state.is_favorite(c.platform, &c.stem));
            shelf.set_spotlight(on_favorites);
            shelf.set_light_ground(light_ground);
        }
    }

    /// Per mould, in `Mould::ALL` order.
    pub fn set_drop_shadows(&mut self, faces: [TexId; Mould::ALL.len()]) {
        for (_, shelf) in &mut self.shelves {
            for (mould, face) in Mould::ALL.into_iter().zip(faces) {
                shelf.set_drop(mould, face);
            }
        }
    }

    /// Per mould, in `Mould::ALL` order.
    pub fn set_cart_shadows(&mut self, faces: [TexId; Mould::ALL.len()]) {
        for (_, shelf) in &mut self.shelves {
            for (mould, face) in Mould::ALL.into_iter().zip(faces) {
                shelf.set_shadow(mould, face);
            }
        }
    }

    /// The board behind a clear cart, per mould, in `Mould::ALL` order.
    pub fn set_cart_backings(&mut self, faces: [TexId; Mould::ALL.len()]) {
        for (_, shelf) in &mut self.shelves {
            for (mould, face) in Mould::ALL.into_iter().zip(faces) {
                shelf.set_backing(mould, face);
            }
        }
    }

    pub fn set_ground(&mut self, face: TexId) {
        self.ground = Some(face);
    }

    pub fn set_ground_light(&mut self, face: TexId) {
        self.ground_light = Some(face);
    }

    pub fn set_wallpaper(&mut self, face: TexId) {
        self.wallpaper = Some(face);
    }

    pub fn set_wifi_face(&mut self, face: TexId, hud: TexId) {
        self.hud_wifi = Some(hud);
        self.wifi = Some(face);
    }

    /// The menu's Home Wi-Fi setting, whether or not a network is up yet.
    pub fn home_wifi_on(&self) -> bool {
        self.state.home_wifi_enabled
    }

    pub fn home_connected(&self) -> bool {
        self.radio.home_connected()
    }

    pub fn set_bolt_face(&mut self, bolt: TexId) {
        self.bolt = Some(bolt);
    }

    /// The Wi-Fi mark, only while home Wi-Fi is on and connected.
    fn wifi_shown(&self, face: Option<TexId>) -> Option<TexId> {
        (self.state.home_wifi_enabled && self.radio.home_connected()).then_some(face).flatten()
    }

    pub fn set_hud_bolt_face(&mut self, bolt: TexId) {
        self.hud_bolt = Some(bolt);
    }

    pub fn set_slot_sets(&mut self, sets: [(TexId, TexId); 8], nes_lids: [TexId; 2]) {
        self.slot_sets = Some(sets);
        self.nes_lids = Some(nes_lids);
    }

    pub fn set_shelf_platform_face(&mut self, face: TexId, w: u32) {
        self.shelf_platform = slot_ui::Printed::new(face, w);
    }

    pub fn clear_shelf_platform(&mut self) {
        self.shelf_platform = slot_ui::Printed::default();
    }

    pub fn slot_text(&self) -> Option<String> {
        match self.slot_letter {
            Some(letter) => Some(letter.to_string()),
            None => self.shelf_platform_name().map(str::to_string),
        }
    }

    pub fn title_text(&self) -> Option<String> {
        if !self.state.show_titles {
            return None;
        }
        let shelf = self.shelf();
        shelf.carts.get(shelf.index).map(slot_ui::label_text)
    }

    pub fn set_shelf_title_face(&mut self, face: TexId, w: u32) {
        self.shelf_title = slot_ui::Printed::new(face, w);
    }

    pub fn clear_shelf_title(&mut self) {
        self.shelf_title = slot_ui::Printed::default();
    }

    fn title_alpha(&self) -> f32 {
        if !self.state.show_titles || (self.name_pending && self.slot_text().is_some()) {
            return 0.0;
        }
        let named = self.shelf_named.map(|at| self.now().saturating_sub(at));
        if named.is_some_and(|t| t < SLOT_NAME_IN_MS + SLOT_NAME_HOLD_MS) {
            return 0.0;
        }
        1.0 - self.slot_name_alpha() / SLOT_NAME_ALPHA
    }

    pub fn shelf_platform_name(&self) -> Option<&'static str> {
        self.next_shelf(1)?;
        Some(self.shelves[self.shelf_at].0.name())
    }

    pub fn set_battery_percent_face(&mut self, face: TexId, w: u32) {
        self.battery_percent = slot_ui::Printed::new(face, w);
    }

    pub fn set_shelf_clock_face(&mut self, face: TexId, w: u32) {
        self.shelf_clock = slot_ui::Printed::new(face, w);
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    pub fn carts(&self) -> impl Iterator<Item = &Cart> {
        self.shelves
            .iter()
            .filter(|(r, _)| r.platform().is_some())
            .flat_map(|(_, shelf)| shelf.carts.iter())
    }

    pub fn seated_cart(&self) -> Option<&Cart> {
        let stem = match &self.phase {
            Phase::Inserting { cart, .. }
            | Phase::Playing { cart }
            | Phase::Ejecting { cart, .. }
            | Phase::Polaroids { cart } => cart,
            Phase::Doze { cart: Some(cart) } => cart,
            _ => return None,
        };
        self.shelf().carts.iter().find(|c| c.stem == *stem)
    }

    pub fn single_cart(&self) -> bool {
        self.carts().count() == 1
    }

    pub fn set_faces(&mut self, faces: Vec<TexId>) {
        let mut faces = faces.into_iter();
        for (_, shelf) in &mut self.shelves {
            let n = shelf.carts.len();
            shelf.set_faces(faces.by_ref().take(n).collect());
        }
    }

    /// Attach a label fetched in the background to every shelf's copy of its cart, and let
    /// those faces go so they are drawn again with it. The shelf, selection and phase stay put.
    /// A cart that already has a label, its own or an earlier download, keeps it.
    pub fn attach_label(&mut self, rom: &Path, label: std::path::PathBuf) -> Vec<TexId> {
        let mut faces = Vec::new();
        for (_, shelf) in &mut self.shelves {
            for index in 0..shelf.carts.len() {
                if shelf.carts[index].rom != rom || shelf.carts[index].label.is_some() {
                    continue;
                }
                shelf.carts[index].label = Some(label.clone());
                faces.extend(shelf.take_face(index));
                self.orphaned.extend(shelf.take_interior(index));
            }
        }
        self.orphaned.extend(faces.iter().copied());
        faces
    }

    pub fn face_wants(&self) -> Vec<FaceWant> {
        let here = self.shelf();
        let screen = here.on_screen();
        let mut wants: Vec<(usize, usize)> = screen.iter().map(|i| (self.shelf_at, *i)).collect();
        for at in self.neighbour_shelves() {
            wants.extend(self.shelves[at].1.on_screen().into_iter().map(|i| (at, i)));
        }
        wants.extend(
            here.around(FACE_AHEAD)
                .into_iter()
                .filter(|i| !screen.contains(i))
                .map(|i| (self.shelf_at, i)),
        );
        wants
            .into_iter()
            .filter(|(at, i)| self.shelves[*at].1.face(*i).is_none())
            .map(|(shelf, index)| FaceWant {
                shelf,
                index,
                on_screen: shelf == self.shelf_at && screen.contains(&index),
            })
            .collect()
    }

    pub fn face_cart(&self, shelf: usize, index: usize) -> Option<&Cart> {
        self.shelves.get(shelf)?.1.carts.get(index)
    }

    pub fn set_cart_face(&mut self, shelf: usize, index: usize, tex: TexId) -> Option<TexId> {
        self.shelves.get_mut(shelf)?.1.set_face(index, tex)
    }

    pub fn set_cart_interior(&mut self, shelf: usize, index: usize, tex: TexId) -> Option<TexId> {
        self.shelves.get_mut(shelf)?.1.set_interior(index, tex)
    }

    pub fn shed_faces(&mut self) -> Vec<TexId> {
        let neighbours = self.neighbour_shelves();
        let mut shed = std::mem::take(&mut self.orphaned);
        for (at, (_, shelf)) in self.shelves.iter_mut().enumerate() {
            let keep = if at == self.shelf_at {
                shelf.around(FACE_KEEP)
            } else if neighbours.contains(&at) {
                shelf.on_screen()
            } else {
                Vec::new()
            };
            for i in 0..shelf.carts.len() {
                if !keep.contains(&i) {
                    shed.extend(shelf.take_face(i));
                    shed.extend(shelf.take_interior(i));
                }
            }
        }
        shed
    }

    fn neighbour_shelves(&self) -> Vec<usize> {
        let mut out = Vec::new();
        for by in [1, -1] {
            if let Some(at) = self.next_shelf(by) {
                if !out.contains(&at) {
                    out.push(at);
                }
            }
        }
        out
    }

    pub fn set_snapshot(&mut self, snapshot: Box<dyn Snapshot>) {
        self.snapshot = Some(snapshot);
    }

    pub fn set_core(&mut self, core: Core) {
        self.core = core;
    }

    pub fn set_platform(&mut self, platform: Platform) {
        self.platform = platform;
    }

    pub fn set_named_core(&mut self, named: bool) {
        self.named_core = named;
    }

    pub fn set_video_mode(&mut self, mode: VideoMode) {
        self.video_mode = mode;
    }

    pub fn source_rect(&self) -> [f32; 4] {
        video_mode::source_rect(self.screen_platform(), self.video_mode)
    }

    fn slot_owns_the_shoulders(&self) -> bool {
        matches!(self.phase, Phase::Playing { .. })
            && matches!(self.screen_platform(), Platform::Gb | Platform::Gbc)
    }

    pub fn taken_buttons(&self) -> &'static [Btn] {
        if self.slot_owns_the_shoulders() {
            &[Btn::L1, Btn::R1]
        } else {
            &[]
        }
    }

    pub fn takes_from_the_game(&self, action: Action) -> bool {
        match action {
            Action::GbaDown(btn) | Action::GbaUp(btn) => self.taken_buttons().contains(&btn),
            _ => false,
        }
    }

    fn set_picture(&mut self, mode: VideoMode) {
        if self.video_mode == mode {
            return;
        }
        self.video_mode = mode;
        let (Some(root), Phase::Playing { cart }) = (self.root.clone(), &self.phase) else {
            return;
        };
        let written = video_mode::write_video_mode(&root, cart, mode);
        if let Err(e) = &written {
            eprintln!("slot: video: could not write video_mode.txt: {e}");
        }
        self.card_writes.wrote("video_mode.txt", &written);
    }

    pub fn set_link_loaded(&mut self, serial: &'static str) {
        self.link_loaded = Some(serial);
    }

    pub fn link_mode(&self, stem: &str) -> (LinkKind, &'static str) {
        let Some((cart, auto)) = self.auto_link(stem) else {
            return (LinkKind::Cable, "auto");
        };
        let chosen = self.link_choices.get(stem).copied().unwrap_or(auto);
        (chosen, serial_option(chosen, auto, &cart.code, &cart.title))
    }

    pub fn link_active(&self) -> bool {
        self.link.is_some()
    }

    pub fn begin_link(&mut self, client_id: u16) {
        self.link = Some(LinkSession {
            client_id,
            lost_at: None,
        });
        self.sync_link_badge();
    }

    pub fn link_client_id(&self) -> Option<u16> {
        self.link.as_ref().map(|s| s.client_id)
    }

    pub fn may_rewind(&self) -> bool {
        self.state.rewind && !self.link_active()
    }

    pub fn turbo(&self) -> bool {
        self.state.turbo
    }

    pub fn records_rewind(&self) -> bool {
        self.state.rewind
    }

    pub fn picture_rect(&self) -> [f32; 4] {
        let (w, h) = self.screen_platform().picture();
        let (sw, sh) = self.screen_platform().canvas();
        let (sw, sh) = (sw as f32, sh as f32);
        let (x, y) = ((sw - w as f32) / 2.0, (sh - h as f32) / 2.0);
        [x / sw, y / sh, (x + w as f32) / sw, (y + h as f32) / sh]
    }

    /// The console whose screen the game is on: the SNES's for a Game Boy game played through
    /// the Super Game Boy, else the game's own.
    pub fn platform(&self) -> Platform {
        self.screen_platform()
    }

    fn screen_platform(&self) -> Platform {
        if self.sgb_playing() {
            Platform::Snes
        } else {
            self.platform
        }
    }

    /// Whether a Game Boy game sits in the Super Game Boy.
    pub fn sgb_playing(&self) -> bool {
        self.sgb && self.seated_cart().is_some_and(|c| c.platform == Platform::Gb)
    }

    /// Whether the cart going in or seated is the Super Game Boy itself.
    pub fn sgb_seated(&self) -> bool {
        self.seated_cart().is_some_and(Cart::is_sgb)
    }

    /// Take the Super Game Boy out: back to the SNES shelf, the SGB coming out of its slot.
    fn eject_sgb(&mut self) {
        let Some(at) = self
            .shelves
            .iter()
            .position(|(r, _)| *r == Rack::Of(Platform::Snes))
        else {
            self.sgb = false;
            return;
        };
        let Some(i) = self.shelves[at].1.carts.iter().position(Cart::is_sgb) else {
            self.sgb = false;
            return;
        };
        self.shelf_mut().release_hold();
        self.sgb = false;
        self.shelf_at = at;
        self.shelf_mut().select(i);
        self.shelf_platform = slot_ui::Printed::default();
        self.slot_letter = None;
        self.shelf_named = None;
        self.name_pending = true;
        self.phase = Phase::Ejecting {
            cart: slot_store::SGB_STEM.to_string(),
            t: -EJECT_HOLD_S,
        };
    }

    /// The Super Game Boy is in: on to the Game Boy shelf, to pick the game that goes in it.
    fn seat_sgb(&mut self) {
        let Some(at) = self
            .shelves
            .iter()
            .position(|(r, _)| *r == Rack::Of(Platform::Gb))
        else {
            return;
        };
        self.sgb = true;
        self.shelf_at = at;
        self.shelf_platform = slot_ui::Printed::default();
        self.slot_letter = None;
        self.shelf_named = None;
        self.name_pending = true;
    }

    pub fn may_load_state(&self) -> bool {
        !self.link_active()
    }

    pub fn may_fast_forward(&self) -> bool {
        !self.link_active()
    }

    pub fn end_link(&mut self) {
        self.link = None;
        self.link_player = None;
        self.sync_link_badge();
        self.radio.ask(RadioJob::Down);
        self.radio.ask(RadioJob::Cool);
    }

    pub fn peer_lost(&mut self) {
        if matches!(self.game_menu, Some(GameMenu::Linked { .. })) {
            self.game_menu = None;
        }
        let now = self.now();
        if let Some(session) = &mut self.link {
            session.lost_at.get_or_insert(now);
        }
        self.sync_link_badge();
    }

    pub fn bios_mismatch(&mut self) {
        if !self.link_active() {
            return;
        }
        let role = self
            .link_client_id()
            .map_or(self.last_role, LinkRow::from_client_id);
        self.end_link();
        self.hud.toast(Toast::BiosMismatch, self.now());
        self.unplug(role);
    }

    pub fn peer_ended(&mut self) {
        if !self.link_active() {
            return;
        }
        let role = self
            .link_client_id()
            .map_or(self.last_role, LinkRow::from_client_id);
        self.end_link();
        self.hud.toast(Toast::PeerEnded, self.now());
        self.unplug(role);
    }

    pub fn link_badge(&self) -> LinkBadge {
        self.hud.link()
    }

    pub fn set_link_badge_faces(&mut self, faces: Vec<TexId>) {
        self.hud.set_link_faces(faces);
    }

    fn sync_link_badge(&mut self) {
        let badge = match &self.link {
            None => LinkBadge::Off,
            Some(s) => match (s.client_id == 0, s.lost_at.is_some()) {
                (true, false) => LinkBadge::Hosting,
                (false, false) => LinkBadge::Joined,
                (true, true) => LinkBadge::HostingLost,
                (false, true) => LinkBadge::JoinedLost,
            },
        };
        self.hud.set_link(badge);
    }

    pub fn take_sfx(&mut self) -> Option<Sfx> {
        self.sfx.take()
    }

    pub fn set_power(&mut self, mut power: Power) {
        power.set_backlight(self.state.brightness);
        self.headphones = power.headphones();
        self.hud.set_headphones(self.headphones);
        let secs = power.now();
        if matches!(self.phase, Phase::SetClock { .. }) || secs < CLOCK_FLOOR {
            self.phase = clock_screen(secs, self.time_zone(), self.state.clock_24h, false);
        }
        self.copy_card_log(&power);
        self.power = Some(power);
        self.battery_at = self.now();
        self.charge_at = self.now();
        self.last_led = None;
    }

    fn copy_card_log(&self, power: &Power) {
        let (Some(log), Some(root)) = (power.card_log(), &self.root) else {
            return;
        };
        if let Err(e) = crate::card_log::copy_to_card(&log, root) {
            eprintln!("slot: card log: {e}");
        }
    }

    fn log_card(&self, alarm: bool) {
        let Some(power) = &self.power else {
            return;
        };
        let Some(log) = power.card_log() else {
            return;
        };
        let kernel = if alarm {
            power.card_kernel_lines()
        } else {
            Vec::new()
        };
        let write_error = self.card_writes.last_error();
        let entry = crate::card_log::entry(&crate::card_log::CardEvent {
            secs: power.now(),
            alarm,
            read_only: self.card_read_only,
            write_error: write_error.as_deref(),
            cart: self.seated(),
            kernel: &kernel,
        });
        if let Err(e) = crate::card_log::append(&log, &entry) {
            eprintln!("slot: card log: {e}");
        }
    }

    pub fn set_rumble(&mut self, strength: u16) {
        if let Some(power) = &mut self.power {
            power.set_rumble(strength);
        }
    }

    pub fn rumble_enabled(&self) -> bool {
        self.state.rumble
    }

    pub fn ff_speed(&self) -> u8 {
        self.state.ff_speed
    }

    pub fn ff_sound(&self) -> bool {
        self.state.ff_sound
    }

    pub fn screen_shader(&self) -> Shader {
        match self.playing_platform() {
            Platform::Gba => self.state.shader_gba,
            Platform::Gb => self.state.shader_gb,
            Platform::Gbc => self.state.shader_gbc,
            Platform::Nes | Platform::Snes => Shader::Off,
        }
    }

    /// The console the game plays on, for its screen and menus: the SNES for a Game Boy game in
    /// the Super Game Boy.
    pub fn playing_platform(&self) -> Platform {
        if self.sgb_playing() {
            return Platform::Snes;
        }
        self.seated_cart().map_or(self.platform, |c| c.platform)
    }

    /// Whether the playing GB or GBC picture is framed in black rather than the screen's glass.
    pub fn black_border(&self) -> bool {
        match self.playing_platform() {
            Platform::Gb => self.state.gb_black_border,
            Platform::Gbc => self.state.gbc_black_border,
            Platform::Gba | Platform::Nes | Platform::Snes => false,
        }
    }

    pub fn light(&self) -> bool {
        self.state.appearance.light()
    }

    /// The platform whose slot is showing: the cart in it, else the one picked on the shelf.
    fn slot_platform(&self) -> Platform {
        if let Some(cart) = self.seated_cart() {
            return cart.platform;
        }
        let shelf = self.shelf();
        match shelf.carts.get(shelf.index) {
            Some(cart) => cart.platform,
            None => self.shelves[self.shelf_at].0.platform().unwrap_or(self.platform),
        }
    }

    /// The slot art for the platform showing: the GBA slot, the Game Boy's for GB and GBC
    /// carts, the NES band with its lid or the SNES band, in the theme's housing.
    fn slot_faces(&self) -> Option<(TexId, TexId)> {
        let sets = self.slot_sets?;
        let kind = match self.slot_platform() {
            Platform::Gba => 0,
            Platform::Gb | Platform::Gbc => 1,
            Platform::Nes => 2,
            Platform::Snes => 3,
        };
        Some(sets[kind * 2 + usize::from(self.light())])
    }

    /// How many seconds an NES or SNES insert takes for each second of the slot's own: they go
    /// in at their own pace. 1 for the handhelds.
    fn insert_pace(&self, stem: &str) -> f32 {
        self.shelf()
            .find(stem)
            .and_then(|(cart, _)| slot_ui::insert_seconds(slot_ui::mould_of(cart)))
            .map_or(1.0, |s| s / SEATED_AT)
    }

    /// How far the carts beside the one going in have faded off the shelf: as quickly as they
    /// do for a Game Boy pak, however long an NES or SNES cart takes, so they stay put.
    fn recede(&self, stem: &str) -> f32 {
        (self.seat() * self.insert_pace(stem)).min(1.0)
    }

    fn click_at(&self, stem: &str) -> f32 {
        self.shelf()
            .find(stem)
            .map_or(1.0, |(cart, _)| slot_ui::click_at(slot_ui::mould_of(cart)))
    }

    pub fn status_ink(&self) -> [u8; 3] {
        match self.phase {
            Phase::Playing { .. } | Phase::Polaroids { .. } => footer_ink(false),
            _ => footer_ink(self.light()),
        }
    }

    /// The playing system's Pixel Transparency.
    pub fn pixel_transparency(&self) -> bool {
        match self.playing_platform() {
            Platform::Gb => self.state.pixel_transparency_gb,
            Platform::Gbc => self.state.pixel_transparency_gbc,
            Platform::Gba => self.state.pixel_transparency_gba,
            Platform::Nes | Platform::Snes => false,
        }
    }

    /// The playing system's Ghosting.
    pub fn lcd_ghosting(&self) -> bool {
        self.lcd_ghosting_for(self.playing_platform())
    }

    pub fn lcd_ghosting_for(&self, platform: Platform) -> bool {
        match platform {
            Platform::Gb => self.state.lcd_ghosting_gb,
            Platform::Gbc => self.state.lcd_ghosting_gbc,
            Platform::Gba => self.state.lcd_ghosting_gba,
            Platform::Nes | Platform::Snes => false,
        }
    }

    /// A TV console's screen effects as chosen, or none while SELECT + B has them off; the
    /// Game Boys have none of them.
    pub fn tv_look(&self, platform: Platform) -> Option<TvLook> {
        let s = &self.state;
        let look = match platform {
            Platform::Nes if s.tv_filter_nes => TvLook {
                cable: s.tv_cable_nes,
                colors: s.nes_colors,
                scanlines: s.tv_scanlines_nes,
                mask: s.tv_mask_nes,
                afterglow: s.tv_afterglow_nes,
            },
            Platform::Snes if s.tv_filter_snes => TvLook {
                cable: s.tv_cable_snes,
                colors: NesColors::Standard,
                scanlines: s.tv_scanlines_snes,
                mask: s.tv_mask_snes,
                afterglow: s.tv_afterglow_snes,
            },
            _ => return None,
        };
        Some(look)
    }

    /// Left / Right on one of a TV console's screen rows; false for any other row.
    fn change_tv_setting(&mut self, row: QuickRow, right: bool) -> bool {
        let s = &mut self.state;
        match row {
            QuickRow::NesCable => s.tv_cable_nes = step_cable(s.tv_cable_nes, right, false),
            QuickRow::SnesCable => s.tv_cable_snes = step_cable(s.tv_cable_snes, right, true),
            QuickRow::NesColors => {
                let all = NesColors::ALL;
                let at = all.iter().position(|c| *c == s.nes_colors).unwrap_or(0);
                let n = all.len();
                s.nes_colors = all[if right { (at + 1) % n } else { (at + n - 1) % n }];
            }
            QuickRow::NesScanlines => s.tv_scanlines_nes = !s.tv_scanlines_nes,
            QuickRow::SnesScanlines => s.tv_scanlines_snes = !s.tv_scanlines_snes,
            QuickRow::NesMask => s.tv_mask_nes = !s.tv_mask_nes,
            QuickRow::SnesMask => s.tv_mask_snes = !s.tv_mask_snes,
            QuickRow::NesAfterglow => s.tv_afterglow_nes = !s.tv_afterglow_nes,
            QuickRow::SnesAfterglow => s.tv_afterglow_snes = !s.tv_afterglow_snes,
            _ => return false,
        }
        // A choice made in the menu shows: the effects come back on with it.
        match row {
            QuickRow::NesCable
            | QuickRow::NesColors
            | QuickRow::NesScanlines
            | QuickRow::NesMask
            | QuickRow::NesAfterglow => s.tv_filter_nes = true,
            _ => s.tv_filter_snes = true,
        }
        true
    }

    /// Whether this game shows in one of the Game Boy palettes, rather than in its own GBC
    /// colours (Color Palettes off), which take the GBC's screen.
    pub fn gb_palette_live(&self) -> bool {
        self.palette_live
    }

    pub fn powering_off(&self) -> bool {
        self.powering_off
    }

    pub fn ready_to_power_off(&self) -> bool {
        self.powering_off && self.now() >= self.act_at
    }

    pub fn ready_to_restart(&self) -> bool {
        self.restarting && self.now() >= self.act_at
    }

    pub fn shutting_down(&self) -> bool {
        self.powering_off || self.restarting
    }

    pub fn restarting(&self) -> bool {
        self.restarting
    }

    pub fn restart(&mut self) {
        self.settle_saves();
        if let Some(power) = &mut self.power {
            power.restart();
        }
    }

    pub fn core_picker(&self) -> Option<Core> {
        self.core_picker.map(|p| p.seat())
    }

    pub fn core_picker_chip(&self) -> Option<Chip> {
        self.core_picker.map(|p| p.chip(self.now()))
    }

    pub fn game_menu_open(&self) -> bool {
        self.game_menu.is_some()
    }

    pub fn game_menu(&self) -> Option<GameMenu> {
        self.game_menu
    }

    pub fn take_link_transport(&mut self) -> Option<(u16, Box<dyn LinkChannel>)> {
        self.link_transport.take()
    }

    pub fn take_link_reload(&mut self) -> Option<(String, &'static str)> {
        self.link_reload.take()
    }

    pub fn link_player(&self) -> Option<u8> {
        self.link_player
    }

    pub fn take_lcd_ghosting(&mut self) -> Option<bool> {
        self.lcd_ghosting_pending.take()
    }

    pub fn take_sgb_border_changed(&mut self) -> bool {
        std::mem::take(&mut self.sgb_border_changed)
    }

    pub fn sgb_border(&self) -> u8 {
        self.state.sgb_border
    }

    pub fn take_tv_changed(&mut self) -> bool {
        std::mem::take(&mut self.tv_changed)
    }

    pub fn gb_palette(&self) -> Option<GbPalette> {
        self.state.gb_palettes.then_some(self.state.gb_palette)
    }

    /// A card read as `Some` file contents (or none on the card), or `None` when the card failed it.
    pub fn card_read(
        &self,
        what: &str,
        read: std::io::Result<Option<Vec<u8>>>,
    ) -> Option<Option<Vec<u8>>> {
        match read {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                eprintln!("slot: {what} read: {e}");
                self.card_writes.failed(what, &e);
                None
            }
        }
    }

    pub fn set_resume_unread(&mut self, unread: bool) {
        self.resume_unread = unread;
    }

    pub fn set_palette_live(&mut self, live: bool) {
        self.palette_live = live;
    }

    pub fn palette_live(&self) -> bool {
        self.palette_live
    }

    pub fn set_palette_capable(&mut self, capable: bool) {
        self.palette_capable = capable;
    }

    pub fn take_gb_palette(&mut self) -> Option<Option<GbPalette>> {
        self.palette_pending.take()
    }

    pub fn core(&self) -> Core {
        self.core
    }

    pub fn link_reload_done(&mut self) {
        let Some(reload) = self.reload.take() else {
            return;
        };
        if reload.fallback {
            self.link_choices.insert(reload.stem, reload.from);
            self.close_game_menu();
            return self.refuse();
        }
        if reload.cancelled {
            return self.close_game_menu();
        }
        let since = match self.game_menu {
            Some(GameMenu::Working { since, .. }) => since,
            _ => self.now(),
        };
        self.start_link_from(
            self.spawn_link(reload.role),
            reload.role.client_id(),
            since,
        );
    }

    pub fn link_reload_failed(&mut self) {
        let Some(mut reload) = self.reload.take() else {
            return;
        };
        if !reload.fallback {
            self.link_reload = Some((reload.stem.clone(), reload.from_serial));
            reload.fallback = true;
            self.reload = Some(reload);
            return;
        }
        self.link_choices.insert(reload.stem, reload.from);
        self.refuse_seated();
    }

    pub fn selected_stem(&self) -> Option<&str> {
        self.shelf()
            .carts
            .get(self.shelf().index)
            .map(|c| c.stem.as_str())
    }

    pub fn battery(&self) -> Option<Battery> {
        self.battery
    }

    pub fn poweroff(&mut self) {
        self.settle_saves();
        if let Some(power) = &mut self.power {
            power.poweroff();
        }
    }

    pub fn apply(&mut self, action: Action) {
        self.track_held_dir(action);
        self.last_activity = self.now();
        match action {
            Action::Screenshot => {
                self.end_power_hold();
                return self.screenshot();
            }
            Action::ResetGame => return self.reset_game(),
            _ => {}
        }
        match action {
            Action::LidClose => return self.doze(),
            Action::LidOpen => return self.wake(),
            Action::PowerPress => {
                if self.link_active() {
                    self.end_link();
                    return;
                }
                self.woke_on_press = matches!(self.phase, Phase::Doze { .. });
                if self.woke_on_press {
                    self.wake();
                } else if !self.shutting_down() {
                    self.power_held_since = Some(self.now());
                }
                return self.flush_resume();
            }
            Action::PowerTap => {
                self.end_power_hold();
                return self.power_press();
            }
            Action::PowerCancel => return self.end_power_hold(),
            Action::PowerHold => {
                if self.power_held_since.is_some() {
                    self.end_power_hold();
                    self.close_game_menu();
                    self.begin_power_off();
                }
                return;
            }
            Action::PowerOff => return,
            _ => {}
        }
        if self.shots.is_some() {
            return self.shots_input(action);
        }
        if let Phase::SetClock {
            picker, from_menu, ..
        } = &mut self.phase
        {
            match action {
                Action::GbaDown(Btn::Left) | Action::ShelfLeft => picker.left(),
                Action::GbaDown(Btn::Right) | Action::ShelfRight => picker.right(),
                Action::GbaDown(Btn::Up) => picker.up(),
                Action::GbaDown(Btn::Down) => picker.down(),
                Action::GbaDown(Btn::A) | Action::Insert => self.confirm_clock(),
                Action::GbaDown(Btn::B) if *from_menu => {
                    self.phase = Phase::QuickMenu {
                        row: QuickRow::DateTime,
                    }
                }
                _ => {}
            }
            return;
        }
        if self.adjust(action) {
            return;
        }
        match action {
            Action::GbaUp(Btn::Left) => self.shelf_mut().release_left(),
            Action::GbaUp(Btn::Right) => self.shelf_mut().release_right(),
            _ => {}
        }
        if self.game_menu.is_some() {
            return self.game_menu_input(action);
        }
        if self.in_game_menu.is_none() {
            self.cheat_page = None;
        }
        if let (Some(_), Some(_), Phase::Playing { .. }) =
            (self.in_game_menu, self.cheat_page, &self.phase)
        {
            if action != Action::Eject {
                return self.cheat_page_input(action);
            }
        }
        if let (Some(row), Phase::Playing { .. }) = (self.in_game_menu, &self.phase) {
            if action != Action::Eject {
                return self.in_game_menu_input(row, action);
            }
        }
        let now = self.now();
        match self.phase {
            Phase::Shelf => match action {
                _ if self.label_search.as_ref().is_some_and(|s| !s.closing()) => {
                    self.label_search_input(action)
                }
                _ if self.shell_picker.is_some() => self.shell_picker_input(action),
                Action::GbaDown(Btn::Start) if self.core_picker.is_none() => {
                    self.open_core_picker()
                }
                Action::GbaDown(Btn::Select) if self.core_picker.is_none() => {
                    self.open_shell_picker()
                }
                _ if self.core_picker.is_some() => self.core_picker_input(action),
                Action::GbaDown(Btn::Up) => self.jump_letter(-1),
                Action::GbaDown(Btn::Down) => self.jump_letter(1),
                Action::ShelfLeft | Action::GbaDown(Btn::Left) => self.shelf_mut().hold_left(now),
                Action::ShelfRight | Action::GbaDown(Btn::Right) => {
                    self.shelf_mut().hold_right(now)
                }
                Action::QuickMenu => self.open_quick_menu(),
                Action::GbaDown(Btn::A) => self.play_held = Some(now),
                Action::GbaUp(Btn::A) => {
                    if self.play_held.take().is_some() {
                        self.insert(!self.state.eject_save);
                    }
                }
                Action::Insert => self.insert(!self.state.eject_save),
                Action::GbaDown(Btn::L1) => self.switch_shelf(-1),
                Action::GbaDown(Btn::R1) => self.switch_shelf(1),
                Action::GbaDown(Btn::Y) => self.toggle_favorite(),
                Action::GbaDown(Btn::B) | Action::Eject if self.sgb => self.eject_sgb(),
                _ => {}
            },
            Phase::Inserting { .. } if action == Action::Eject => self.eject(),
            Phase::Playing { .. } => match action {
                Action::GbaDown(Btn::L1) if self.slot_owns_the_shoulders() => {
                    self.set_picture(VideoMode::Stretch)
                }
                Action::GbaDown(Btn::R1) if self.slot_owns_the_shoulders() => {
                    self.set_picture(VideoMode::Actual)
                }
                Action::Eject => self.eject(),
                Action::GameMenu => self.game_menu_shortcut(),
                Action::QuickMenu if self.state.in_game_menu => {
                    self.tap_in_game_menu(Some(QuickRow::PLAY[0]))
                }
                Action::Polaroids => self.double_tap_states(),
                Action::SaveState => self.save_state(),
                Action::LoadState => self.load_newest(),
                Action::RewindStart if !self.may_rewind() => self.refuse(),
                Action::FfStart if !self.may_fast_forward() => self.refuse(),
                _ => {}
            },
            Phase::Polaroids { .. } if self.polaroids.as_ref().is_some_and(|p| p.confirm.is_some()) => {
                match action {
                    Action::GbaDown(Btn::A) | Action::Insert => {
                        self.end_state_confirm();
                        self.delete_selected();
                    }
                    Action::GbaDown(Btn::B) | Action::Polaroids => self.end_state_confirm(),
                    _ => {}
                }
            }
            Phase::Polaroids { .. } => match action {
                Action::ShelfLeft | Action::GbaDown(Btn::Left) => self.flick(Polaroids::left),
                Action::ShelfRight | Action::GbaDown(Btn::Right) => self.flick(Polaroids::right),
                Action::GbaDown(Btn::A) => self.load_selected(),
                Action::GbaDown(Btn::B) | Action::Polaroids => self.close_polaroids(),
                Action::GbaDown(Btn::Y) => self.ask_state_delete(),
                _ => {}
            },
            Phase::About if action == Action::GbaDown(Btn::B) || action == Action::QuickMenu => {
                self.phase = Phase::QuickMenu {
                    row: QuickRow::About,
                }
            }
            Phase::Buttons { page } => {
                let n = slot_ui::BUTTON_PAGES.len();
                match action {
                    Action::ShelfLeft | Action::GbaDown(Btn::Left) | Action::GbaDown(Btn::L1) => {
                        self.phase = Phase::Buttons {
                            page: (page + n - 1) % n,
                        }
                    }
                    Action::ShelfRight
                    | Action::GbaDown(Btn::Right)
                    | Action::GbaDown(Btn::R1) => {
                        self.phase = Phase::Buttons {
                            page: (page + 1) % n,
                        }
                    }
                    Action::GbaDown(Btn::B) | Action::QuickMenu => {
                        self.phase = Phase::QuickMenu {
                            row: QuickRow::Buttons,
                        }
                    }
                    _ => {}
                }
            }
            Phase::QuickMenu { row } => self.quick_menu_input(row, action),
            _ => {}
        }
    }

    fn open_quick_menu(&mut self) {
        self.phase = Phase::QuickMenu {
            row: QuickRow::MAIN[0],
        };
    }

    fn quick_menu_input(&mut self, row: QuickRow, action: Action) {
        let row = match action {
            Action::GbaDown(Btn::Up) => row.up(self.hidden_rows()),
            Action::GbaDown(Btn::Down) => row.down(self.hidden_rows()),
            Action::GbaDown(Btn::Left) => return self.change_setting(row, false),
            Action::GbaDown(Btn::Right) => return self.change_setting(row, true),
            Action::GbaDown(Btn::A) => return self.open_quick_row(row),
            Action::GbaDown(Btn::B) | Action::QuickMenu => {
                self.phase = match row.parent() {
                    Some(row) => Phase::QuickMenu { row },
                    None => Phase::Shelf,
                };
                return;
            }
            _ => return,
        };
        self.phase = Phase::QuickMenu { row };
    }

    fn in_game_menu_input(&mut self, row: QuickRow, action: Action) {
        let hidden = self.in_game_hidden();
        self.in_game_menu = match action {
            Action::GbaDown(Btn::Up) => Some(row.up(&hidden)),
            Action::GbaDown(Btn::Down) => Some(row.down(&hidden)),
            Action::GbaDown(Btn::Left) => return self.change_setting(row, false),
            Action::GbaDown(Btn::Right) => return self.change_setting(row, true),
            Action::GbaDown(Btn::A) => return self.open_in_game_row(row),
            Action::GbaDown(Btn::B) => row.parent_in_game(),
            Action::QuickMenu => return self.tap_in_game_menu(None),
            Action::Polaroids => return self.double_tap_states(),
            _ => return,
        };
    }

    fn open_in_game_row(&mut self, row: QuickRow) {
        match row {
            QuickRow::Resume => self.in_game_menu = None,
            QuickRow::States => self.open_states(),
            QuickRow::Cheats => self.open_cheats(),
            QuickRow::GameScreenshots => self.open_shots(true),
            QuickRow::ExitGame => self.eject(),
            _ => {
                // The page opens on its first row this game shows; a page it shows none of
                // does not open.
                let hidden = self.in_game_hidden();
                if let Some(child) = row.child().and_then(|c| c.shown(&hidden).first().copied()) {
                    self.in_game_menu = Some(child);
                }
            }
        }
    }

    /// The seated or playing cart, and its platform.
    fn cart_in_slot(&self) -> Option<(Platform, String)> {
        let stem = match &self.phase {
            Phase::Playing { cart } | Phase::Inserting { cart, .. } => cart.clone(),
            Phase::Polaroids { cart } => cart.clone(),
            _ => return None,
        };
        let platform = self.seated_cart().map_or(self.platform, |c| c.platform);
        Some((platform, stem))
    }

    /// Read the cheats of the game in the slot, unless they are read already. A game that had
    /// none is read again, as its files may have been fetched while it ran.
    fn load_cheats(&mut self) {
        let Some(key) = self.cart_in_slot() else {
            return;
        };
        if self.cheats_for.as_ref() == Some(&key) && !self.cheats.is_empty() {
            return;
        }
        self.cheats = match self.root() {
            Some(root) => crate::cheats::load(root, key.0, &key.1),
            None => Vec::new(),
        };
        self.cheats_for = Some(key);
        self.cheats_rev += 1;
    }

    /// The codes to hand a freshly loaded core: the cheats this game left on.
    pub fn start_cheats(&mut self) -> Vec<String> {
        self.load_cheats();
        crate::cheats::codes(&self.cheats)
    }

    pub fn take_cheats(&mut self) -> Option<Vec<String>> {
        self.cheats_pending.take()
    }

    /// Whether the robber shows: a cheat is on and the Cheat Icon setting allows it.
    fn cheat_icon_shown(&self) -> bool {
        self.state.cheat_icon && self.cheats_on()
    }

    pub fn cheats_on(&self) -> bool {
        self.cheats_for.is_some()
            && self.cart_in_slot() == self.cheats_for
            && self.cheats.iter().any(|c| c.on)
    }

    pub fn cheats_rev(&self) -> u64 {
        self.cheats_rev
    }

    pub fn cheat_names(&self) -> Vec<String> {
        self.cheats.iter().map(|c| c.name.clone()).collect()
    }

    pub fn cheat_counts(&self) -> (usize, usize) {
        (self.cheats.iter().filter(|c| c.on).count(), self.cheats.len())
    }

    pub fn cheat_platform(&self) -> Platform {
        self.cheats_for.as_ref().map_or(self.playing_platform(), |k| k.0)
    }

    pub fn set_cheat_faces(&mut self, faces: slot_ui::CheatFaces) {
        self.cheat_faces = Some(faces);
    }

    pub fn cheat_faces_mut(&mut self) -> Option<&mut slot_ui::CheatFaces> {
        self.cheat_faces.as_mut()
    }

    pub fn set_cheat_badges(&mut self, badges: [(TexId, u32, u32); 2]) {
        self.cheat_badges = Some(badges);
    }

    pub fn cheat_page_open(&self) -> bool {
        self.cheat_page.is_some()
    }

    fn open_cheats(&mut self) {
        self.load_cheats();
        self.cheat_page = Some((0, 0));
    }

    fn cheats_changed(&mut self) {
        if let Some((platform, stem)) = self.cheats_for.clone() {
            if let Some(root) = self.root() {
                crate::cheats::save(root, platform, &stem, &self.cheats);
            }
        }
        self.cheats_pending = Some(crate::cheats::codes(&self.cheats));
    }

    fn cheat_page_input(&mut self, action: Action) {
        let Some((sel, first)) = self.cheat_page else {
            return;
        };
        let n = self.cheats.len();
        match action {
            Action::GbaDown(Btn::Up) if n > 0 => {
                let sel = if sel == 0 { n - 1 } else { sel - 1 };
                self.cheat_page = Some((sel, slot_ui::first_shown(sel, n, first)));
            }
            Action::GbaDown(Btn::Down) if n > 0 => {
                let sel = if sel + 1 >= n { 0 } else { sel + 1 };
                self.cheat_page = Some((sel, slot_ui::first_shown(sel, n, first)));
            }
            Action::GbaDown(Btn::A) if sel < n => {
                self.cheats[sel].on = !self.cheats[sel].on;
                self.cheats_changed();
            }
            Action::GbaDown(Btn::Y) if self.cheats.iter().any(|c| c.on) => {
                for c in &mut self.cheats {
                    c.on = false;
                }
                self.cheats_changed();
            }
            Action::GbaDown(Btn::B) => self.cheat_page = None,
            Action::QuickMenu => {
                self.cheat_page = None;
                self.tap_in_game_menu(None);
            }
            _ => {}
        }
    }

    fn tap_in_game_menu(&mut self, to: Option<QuickRow>) {
        self.menu_tap = Some((self.now(), self.in_game_menu));
        self.in_game_menu = to;
    }

    fn in_game_menu_shown(&self) -> Option<QuickRow> {
        match self.menu_tap {
            Some((at, None)) if self.now().saturating_sub(at) < MENU_REVEAL_MS => None,
            _ => self.in_game_menu,
        }
    }

    fn double_tap_states(&mut self) {
        if let Some((at, before)) = self.menu_tap.take() {
            if self.now().saturating_sub(at) <= MENU_DOUBLE_TAP_MS {
                self.in_game_menu = before;
            }
        }
        match self.in_game_menu {
            Some(_) => self.open_states(),
            None => self.open_polaroids(),
        }
    }

    fn open_states(&mut self) {
        self.in_game_menu = None;
        self.open_polaroids();
        match self.phase {
            Phase::Polaroids { .. } => self.states_from_menu = true,
            _ => self.in_game_menu = Some(QuickRow::States),
        }
    }

    fn open_quick_row(&mut self, row: QuickRow) {
        match row {
            // Auto Time keeps the clock; Date & Time only opens when it is off.
            QuickRow::DateTime if self.state.auto_time => {}
            QuickRow::DateTime => {
                self.phase = clock_screen(self.utc_secs(), self.time_zone(), self.state.clock_24h, true);
            }
            QuickRow::About => self.phase = Phase::About,
            QuickRow::Screenshots => self.open_shots(false),
            QuickRow::Buttons => self.phase = Phase::Buttons { page: 0 },
            QuickRow::Screen
            | QuickRow::Game
            | QuickRow::General
            | QuickRow::Network
            | QuickRow::ScreenGba
            | QuickRow::ScreenGbc
            | QuickRow::ScreenGb
            | QuickRow::ScreenNes
            | QuickRow::ScreenSnes => {
                if let Some(row) = row.child() {
                    let row = row.shown(self.hidden_rows())[0];
                    self.phase = Phase::QuickMenu { row };
                }
            }
            QuickRow::EjectSave
            | QuickRow::SgbBorder
            | QuickRow::Turbo
            | QuickRow::Rewind
            | QuickRow::FastForward
            | QuickRow::FastForwardSound
            | QuickRow::GbaShader
            | QuickRow::GbShader
            | QuickRow::GbcShader
            | QuickRow::GbPalettes
            | QuickRow::Rumble
            | QuickRow::HomeWifi
            | QuickRow::ConnectedTo
            | QuickRow::AutoTime
            | QuickRow::TimeZone
            | QuickRow::Clock24
            | QuickRow::CheatIcon
            | QuickRow::GbcBlackBorder
            | QuickRow::GbBlackBorder
            | QuickRow::Theme
            | QuickRow::Recents
            | QuickRow::FavoriteMark
            | QuickRow::ShowTitles
            | QuickRow::Wallpaper
            | QuickRow::WallpaperTint
            | QuickRow::Resume
            | QuickRow::States
            | QuickRow::Cheats
            | QuickRow::GameScreenshots
            | QuickRow::Settings
            | QuickRow::ExitGame
            | QuickRow::PlayScreen
            | QuickRow::PlayGameplay
            | QuickRow::Shader
            | QuickRow::Palette
            | QuickRow::GbBezel
            | QuickRow::GbcBezel
            | QuickRow::GbaBezel
            | QuickRow::Bezel
            | QuickRow::PixelTransparency
            | QuickRow::LcdGhosting
            | QuickRow::BlackBorder
            | QuickRow::GbTransparency
            | QuickRow::GbcTransparency
            | QuickRow::GbaTransparency
            | QuickRow::GbGhosting
            | QuickRow::GbcGhosting
            | QuickRow::GbaGhosting
            | QuickRow::AutoSleep
            | QuickRow::InGameMenu
            | QuickRow::RememberPosition
            | QuickRow::ResumeGame
            | QuickRow::BootPicture
            | QuickRow::Sleep
            | QuickRow::Restart
            | QuickRow::PowerOff
            | QuickRow::NesCable
            | QuickRow::SnesCable
            | QuickRow::NesColors
            | QuickRow::NesScanlines
            | QuickRow::SnesScanlines
            | QuickRow::NesMask
            | QuickRow::SnesMask
            | QuickRow::NesAfterglow
            | QuickRow::SnesAfterglow
            | QuickRow::Cable
            | QuickRow::Colors
            | QuickRow::Scanlines
            | QuickRow::Mask
            | QuickRow::Afterglow => {
                // A on a switch flips it, as Left / Right do.
                if self.quick_value(row).and_then(QuickValue::switch).is_some() {
                    self.change_setting(row, true);
                }
            }
        }
    }

    fn change_setting(&mut self, row: QuickRow, right: bool) {
        let row = self.playing_row(row);
        if row == QuickRow::SgbBorder {
            // Auto, slot.'s, the SGB's own that are in BIOS, none, and round again.
            let mut all = vec![0u8, 1];
            if let Some(root) = &self.root {
                all.extend(
                    crate::core::sgb_borders_on_card(root)
                        .into_iter()
                        .map(|n| 10 + n),
                );
            }
            all.push(2);
            let at = all
                .iter()
                .position(|b| *b == self.state.sgb_border)
                .unwrap_or(0);
            let n = all.len();
            self.state.sgb_border = all[if right {
                (at + 1) % n
            } else {
                (at + n - 1) % n
            }];
            self.sgb_border_changed = true;
            self.persist();
            return;
        }
        if self.change_tv_setting(row, right) {
            self.tv_changed = true;
            self.persist();
            return;
        }
        if let QuickRow::Palette | QuickRow::GbPalettes = row {
            // Off, then each palette in turn, and round again.
            let s = &mut self.state;
            let at = if s.gb_palettes { s.gb_palette.index() + 1 } else { 0 };
            let n = GbPalette::COUNT + 1;
            let to = if right { (at + 1) % n } else { (at + n - 1) % n };
            s.gb_palettes = to > 0;
            if to > 0 {
                s.gb_palette = GbPalette::all().nth(to - 1).unwrap_or(GbPalette::DEFAULT);
            }
            if self.palette_capable && row == QuickRow::Palette {
                self.palette_live = self.state.gb_palettes;
                self.palette_pending = Some(self.gb_palette());
            }
            self.persist();
            return;
        }
        if let QuickRow::GbaBezel | QuickRow::GbBezel | QuickRow::GbcBezel = row {
            let platform = match row {
                QuickRow::GbaBezel => Platform::Gba,
                QuickRow::GbBezel => Platform::Gb,
                _ => Platform::Gbc,
            };
            self.cycle_bezel(platform, right);
            self.persist();
            return;
        }
        if row == QuickRow::Wallpaper {
            self.cycle_wallpaper(right);
            self.persist();
            return;
        }
        if row == QuickRow::Recents {
            self.state.show_recents = !self.state.show_recents;
            self.restock_recents();
            self.persist();
            return;
        }
        if row == QuickRow::FavoriteMark {
            self.state.favorite_mark = self.state.favorite_mark.step(right);
            self.mark_stars();
            self.persist();
            return;
        }
        if row == QuickRow::ShowTitles {
            self.state.show_titles = !self.state.show_titles;
            self.persist();
            return;
        }
        if row == QuickRow::TimeZone {
            self.state.time_zone = self.time_zone().next(right).city();
            self.persist();
            return;
        }
        let s = &mut self.state;
        match row {
            QuickRow::FastForward => {
                let to = ff_next(s.ff_speed, right);
                if to == s.ff_speed {
                    return;
                }
                s.ff_speed = to;
            }
            QuickRow::FastForwardSound => s.ff_sound = !s.ff_sound,
            QuickRow::GbaShader => {
                let to = s.shader_gba.step(&Shader::GBA, right);
                if to == s.shader_gba {
                    return;
                }
                s.shader_gba = to;
            }
            QuickRow::GbShader => {
                let to = s.shader_gb.step(&Shader::GB, right);
                if to == s.shader_gb {
                    return;
                }
                s.shader_gb = to;
            }
            QuickRow::GbcShader => {
                let to = s.shader_gbc.step(&Shader::GBC, right);
                if to == s.shader_gbc {
                    return;
                }
                s.shader_gbc = to;
            }
            QuickRow::Rumble => s.rumble = !s.rumble,
            QuickRow::HomeWifi => {
                s.home_wifi_enabled = !s.home_wifi_enabled;
                self.radio.ask(RadioJob::Home(s.home_wifi_enabled));
            }
            QuickRow::AutoTime => {
                s.auto_time = !s.auto_time;
                self.radio.ask(RadioJob::Time(s.auto_time));
            }
            QuickRow::Clock24 => s.clock_24h = !s.clock_24h,
            QuickRow::CheatIcon => s.cheat_icon = !s.cheat_icon,
            QuickRow::GbcBlackBorder => s.gbc_black_border = !s.gbc_black_border,
            QuickRow::GbBlackBorder => s.gb_black_border = !s.gb_black_border,
            QuickRow::EjectSave => s.eject_save = !s.eject_save,
            QuickRow::Turbo => s.turbo = !s.turbo,
            QuickRow::Rewind => s.rewind = !s.rewind,

            QuickRow::Theme => s.appearance = s.appearance.toggle(),
            QuickRow::WallpaperTint => s.wallpaper_tint = s.wallpaper_tint.step(right),
            QuickRow::Wallpaper
            | QuickRow::Recents
            | QuickRow::FavoriteMark
            | QuickRow::ShowTitles
            | QuickRow::DateTime
            | QuickRow::TimeZone
            | QuickRow::About
            | QuickRow::SgbBorder
            | QuickRow::Screenshots
            | QuickRow::Screen
            | QuickRow::Game
            | QuickRow::General
            | QuickRow::Network
            | QuickRow::ConnectedTo
            | QuickRow::Resume
            | QuickRow::States
            | QuickRow::Cheats
            | QuickRow::GameScreenshots
            | QuickRow::Settings
            | QuickRow::ExitGame
            | QuickRow::PlayScreen
            | QuickRow::PlayGameplay
            | QuickRow::Shader
            | QuickRow::Palette
            | QuickRow::ScreenGba
            | QuickRow::ScreenGbc
            | QuickRow::ScreenGb
            | QuickRow::GbBezel
            | QuickRow::GbcBezel
            | QuickRow::GbaBezel
            | QuickRow::Bezel
            | QuickRow::Buttons
            | QuickRow::Sleep
            | QuickRow::Restart
            | QuickRow::PowerOff
            | QuickRow::GbPalettes
            // Mapped to this system's own row above; on the GBA there is no border.
            | QuickRow::PixelTransparency
            | QuickRow::LcdGhosting
            | QuickRow::BlackBorder
            | QuickRow::ScreenNes
            | QuickRow::ScreenSnes
            | QuickRow::NesCable
            | QuickRow::SnesCable
            | QuickRow::NesColors
            | QuickRow::NesScanlines
            | QuickRow::SnesScanlines
            | QuickRow::NesMask
            | QuickRow::SnesMask
            | QuickRow::NesAfterglow
            | QuickRow::SnesAfterglow
            | QuickRow::Cable
            | QuickRow::Colors
            | QuickRow::Scanlines
            | QuickRow::Mask
            | QuickRow::Afterglow => return,

            QuickRow::GbTransparency => s.pixel_transparency_gb = !s.pixel_transparency_gb,
            QuickRow::GbcTransparency => s.pixel_transparency_gbc = !s.pixel_transparency_gbc,
            QuickRow::GbaTransparency => s.pixel_transparency_gba = !s.pixel_transparency_gba,
            QuickRow::GbGhosting | QuickRow::GbcGhosting | QuickRow::GbaGhosting => {
                let on = match row {
                    QuickRow::GbGhosting => &mut s.lcd_ghosting_gb,
                    QuickRow::GbcGhosting => &mut s.lcd_ghosting_gbc,
                    _ => &mut s.lcd_ghosting_gba,
                };
                *on = !*on;
                self.lcd_ghosting_pending = Some(self.lcd_ghosting());
            }
            QuickRow::AutoSleep => {
                let at = AUTO_SLEEP_OPTIONS_SECS
                    .iter()
                    .position(|secs| *secs == s.auto_sleep_secs)
                    .unwrap_or(0);
                let next = if right {
                    (at + 1) % AUTO_SLEEP_OPTIONS_SECS.len()
                } else {
                    (at + AUTO_SLEEP_OPTIONS_SECS.len() - 1) % AUTO_SLEEP_OPTIONS_SECS.len()
                };
                s.auto_sleep_secs = AUTO_SLEEP_OPTIONS_SECS[next];
            }
            QuickRow::InGameMenu => s.in_game_menu = !s.in_game_menu,
            QuickRow::RememberPosition => s.remember_position = !s.remember_position,
            QuickRow::ResumeGame => s.resume_game = !s.resume_game,
            QuickRow::BootPicture => s.boot_picture = !s.boot_picture,
        }
        if matches!(row, QuickRow::Theme) {
            self.mark_stars();
        }
        self.persist();
    }

    pub fn apply_at(&mut self, action: Action, now: Millis) {
        self.clock = now as f64;
        self.apply(action);
    }

    /// SELECT + B: the screen effects chosen for this system off, or those same ones back on.
    /// With none chosen, on brings the standard look: Dot Matrix in the DMG palette for the GB,
    /// the LCD filter for the GBC and GBA. The NES and SNES switch their TV look.
    fn toggle_effects(&mut self) {
        if !matches!(self.phase, Phase::Inserting { .. } | Phase::Playing { .. }) {
            return;
        }
        const SHADER: u8 = 1;
        const TRANSPARENCY: u8 = 2;
        const GHOSTING: u8 = 4;
        const BORDER: u8 = 8;
        let platform = self.playing_platform();
        if platform.tv() {
            // A TV console's chosen screen effects go off and back on together; with none
            // chosen, on brings the standard TV look.
            let s = &mut self.state;
            let nes = platform == Platform::Nes;
            let on = match nes {
                true => &mut s.tv_filter_nes,
                false => &mut s.tv_filter_snes,
            };
            *on = !*on;
            let on = *on;
            let chosen = match nes {
                true => {
                    s.tv_cable_nes != TvCable::Off
                        || s.nes_colors != NesColors::Standard
                        || s.tv_scanlines_nes
                        || s.tv_mask_nes
                        || s.tv_afterglow_nes
                }
                false => {
                    s.tv_cable_snes != TvCable::Off
                        || s.tv_scanlines_snes
                        || s.tv_mask_snes
                        || s.tv_afterglow_snes
                }
            };
            let on = if !on && !chosen {
                // Nothing was chosen to switch off: switch the standard look on instead.
                let d = SlotState::default();
                match nes {
                    true => {
                        s.tv_filter_nes = true;
                        s.tv_cable_nes = d.tv_cable_nes;
                        s.nes_colors = d.nes_colors;
                        s.tv_scanlines_nes = d.tv_scanlines_nes;
                        s.tv_mask_nes = d.tv_mask_nes;
                        s.tv_afterglow_nes = d.tv_afterglow_nes;
                    }
                    false => {
                        s.tv_filter_snes = true;
                        s.tv_cable_snes = d.tv_cable_snes;
                        s.tv_scanlines_snes = d.tv_scanlines_snes;
                        s.tv_mask_snes = d.tv_mask_snes;
                        s.tv_afterglow_snes = d.tv_afterglow_snes;
                    }
                }
                true
            } else {
                on
            };
            self.tv_changed = true;
            self.persist();
            let said = if on {
                Toast::EffectsOn
            } else {
                Toast::EffectsOff
            };
            self.hud.toast(said, self.now());
            return;
        }
        let s = &mut self.state;
        let mut no_border = false;
        let (shader, transparency, ghosting, border, saved, filter) = match platform {
            Platform::Gb => (
                &mut s.shader_gb,
                &mut s.pixel_transparency_gb,
                &mut s.lcd_ghosting_gb,
                &mut s.gb_black_border,
                &mut s.effects_saved_gb,
                Shader::Grid,
            ),
            Platform::Gbc => (
                &mut s.shader_gbc,
                &mut s.pixel_transparency_gbc,
                &mut s.lcd_ghosting_gbc,
                &mut s.gbc_black_border,
                &mut s.effects_saved_gbc,
                Shader::GbcLcd,
            ),
            Platform::Gba | Platform::Nes | Platform::Snes => (
                &mut s.shader_gba,
                &mut s.pixel_transparency_gba,
                &mut s.lcd_ghosting_gba,
                &mut no_border,
                &mut s.effects_saved_gba,
                Shader::GbaLcd,
            ),
        };
        let shown = (*shader != Shader::Off) as u8 * SHADER
            | *transparency as u8 * TRANSPARENCY
            | *ghosting as u8 * GHOSTING
            | *border as u8 * BORDER;
        let mut standard = false;
        let (to, on) = match shown {
            0 => match *saved {
                0 => {
                    standard = true;
                    (SHADER, true)
                }
                kept => (kept, true),
            },
            chosen => {
                *saved = chosen;
                (0, false)
            }
        };
        *shader = if to & SHADER != 0 { filter } else { Shader::Off };
        *transparency = to & TRANSPARENCY != 0;
        *ghosting = to & GHOSTING != 0;
        *border = to & BORDER != 0 && platform != Platform::Gba;
        self.lcd_ghosting_pending = Some(*ghosting);
        if standard && platform == Platform::Gb && !self.state.gb_palettes {
            self.state.gb_palettes = true;
            self.state.gb_palette = GbPalette::DEFAULT;
            if self.palette_capable {
                self.palette_live = true;
                self.palette_pending = Some(Some(GbPalette::DEFAULT));
            }
        }
        self.persist();
        let said = if on { Toast::EffectsOn } else { Toast::EffectsOff };
        self.hud.toast(said, self.now());
    }

    fn adjust(&mut self, action: Action) -> bool {
        if action == Action::MuteToggle {
            self.mute_toggle();
            return true;
        }
        if self.in_game_menu.is_some()
            && matches!(
                action,
                Action::ShaderNext
                    | Action::ShaderPrev
                    | Action::PaletteNext
                    | Action::PalettePrev
            )
        {
            return true;
        }
        if action == Action::ShaderPrev {
            self.toggle_effects();
            return true;
        }
        if let Action::PaletteNext | Action::PalettePrev = action {
            if self.palette_live && matches!(self.phase, Phase::Playing { .. }) {
                let to = match action {
                    Action::PalettePrev => self.state.gb_palette.prev(),
                    _ => self.state.gb_palette.next(),
                };
                self.state.gb_palette = to;
                self.palette_pending = Some(Some(to));
                self.persist();
                self.hud.toast(Toast::Palette(to), self.now());
            }
            return true;
        }
        let s = &self.state;
        let (kind, value) = match action {
            Action::BrightnessUp => (HudKind::Brightness, up(s.brightness, 1, BRIGHTNESS_MAX)),
            Action::BrightnessDown => (HudKind::Brightness, s.brightness.saturating_sub(1)),
            Action::BlueLightUp => (HudKind::BlueLight, up(s.blue_light, 1, BLUE_LIGHT_MAX)),
            Action::BlueLightDown => (HudKind::BlueLight, s.blue_light.saturating_sub(1)),
            Action::VolumeUp => (HudKind::Volume, up(self.level(), VOLUME_STEP, VOLUME_MAX)),
            Action::VolumeDown => (HudKind::Volume, self.level().saturating_sub(VOLUME_STEP)),
            _ => return false,
        };
        if kind == HudKind::Volume {
            self.remember_volume();
        }
        let level = match kind {
            HudKind::Brightness => &mut self.state.brightness,
            HudKind::BlueLight => &mut self.state.blue_light,
            HudKind::Volume if self.headphones => &mut self.state.volume_hp,
            HudKind::Volume => &mut self.state.volume,
            HudKind::Rewind | HudKind::Power => return false,
        };
        let moved = *level != value;
        *level = value;
        let unmuted = kind == HudKind::Volume && std::mem::take(self.muted_mut());
        let (shown, now) = (self.hud_value(kind, value), self.now());
        self.hud.show(kind, shown, self.muted(), now);
        if let (HudKind::Brightness, Some(power)) = (kind, &mut self.power) {
            power.set_backlight(value);
        }
        if moved || unmuted {
            self.persist_soon();
        }
        true
    }

    fn remember_volume(&mut self) {
        if self.vol_before.len() == 2 {
            self.vol_before.remove(0);
        }
        self.vol_before
            .push((self.level(), self.muted(), self.now()));
    }

    fn mute_toggle(&mut self) {
        let now = self.now();
        if let Some((volume, muted, _)) = self
            .vol_before
            .iter()
            .find(|(_, _, at)| now.saturating_sub(*at) <= MUTE_CHORD_MS)
            .copied()
        {
            *self.level_mut() = volume;
            *self.muted_mut() = muted;
        }
        self.vol_before.clear();
        let muted = !self.muted();
        *self.muted_mut() = muted;
        self.hud
            .show(HudKind::Volume, self.output_volume(), muted, now);
        self.persist_soon();
    }

    fn hud_value(&self, kind: HudKind, value: u8) -> u8 {
        match kind {
            HudKind::Volume => self.output_volume(),
            _ => value,
        }
    }

    pub fn show_rewind(&mut self, fill: u8) {
        let now = self.now();
        self.hud.show(HudKind::Rewind, fill, false, now);
    }

    pub fn hide_rewind(&mut self) {
        self.hud.release_rewind();
    }

    pub fn set_ff(&mut self, ff: FfState) {
        self.hud.set_ff(ff);
    }

    pub fn ff_badge(&self) -> Option<Icon> {
        self.hud.badge()
    }

    pub fn blue_light(&self) -> u8 {
        self.state.blue_light
    }

    pub fn brightness(&self) -> u8 {
        self.state.brightness
    }

    pub fn volume(&self) -> u8 {
        self.level()
    }

    fn level(&self) -> u8 {
        if self.headphones {
            self.state.volume_hp
        } else {
            self.state.volume
        }
    }

    fn level_mut(&mut self) -> &mut u8 {
        if self.headphones {
            &mut self.state.volume_hp
        } else {
            &mut self.state.volume
        }
    }

    pub fn muted(&self) -> bool {
        if self.headphones {
            self.state.muted_hp
        } else {
            self.state.muted
        }
    }

    fn muted_mut(&mut self) -> &mut bool {
        if self.headphones {
            &mut self.state.muted_hp
        } else {
            &mut self.state.muted
        }
    }

    pub fn output_volume(&self) -> u8 {
        if self.muted() {
            0
        } else {
            self.level()
        }
    }

    pub fn hud_icon(&self) -> Icon {
        self.hud.glyph()
    }

    pub fn now(&self) -> Millis {
        self.clock as Millis
    }

    fn flick(&mut self, step: fn(&mut Polaroids)) {
        if let Some(p) = &mut self.polaroids {
            step(p);
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.clock += dt as f64 * 1000.0;
        self.keep_shots(dt);
        if self.name_pending && self.shelf_platform.face.is_some() {
            self.name_pending = false;
            self.shelf_named = Some(self.now());
        }
        self.timers();
        self.poll_link();
        let now = self.now();
        let ready = self.core_faces_ready();
        if let Some(picker) = &mut self.core_picker {
            if picker.waiting() && (ready || picker.waited(now) >= FACES_WAIT_MS) {
                picker.start(now);
            }
        }
        if self.core_picker.is_some_and(|p| p.finished(now)) {
            self.core_picker = None;
        }
        if self.shell_picker.as_ref().is_some_and(|p| p.finished(now)) {
            self.shell_picker = None;
        }
        if self.label_search.as_ref().is_some_and(|s| s.finished(now)) {
            self.label_search = None;
        }
        if !self.on_shelf() {
            self.shelf_mut().release_hold();
        }
        let mut touched = false;
        let at = self.shelf_at;
        let (pace, click) = match &self.phase {
            Phase::Inserting { cart, .. } | Phase::Ejecting { cart, .. } => {
                (self.insert_pace(cart), self.click_at(cart))
            }
            _ => (1.0, 1.0),
        };
        // The Super Game Boy runs nothing of its own: once it is in, the Game Boy shelf.
        let sgb_going_in = self.sgb_seated() && matches!(self.phase, Phase::Inserting { .. });
        let mut sgb_in = false;
        let next = match &mut self.phase {
            Phase::Shelf => {
                let shelf = &mut self.shelves[at].1;
                shelf.tick(now);
                shelf.update(dt);
                None
            }
            Phase::Inserting {
                cart,
                t,
                core_ready,
                resumed,
                ..
            } => {
                let was = *t;
                // Up to seated the cart moves at its own console's pace; the hold after is the same.
                *t += if was < SEATED_AT { dt / pace } else { dt };
                let at = SEATED_AT * click - Sfx::Insert.lead() / pace;
                touched = !*resumed && was < at && *t >= at;
                if sgb_going_in {
                    sgb_in = *t >= INSERT_S;
                    sgb_in.then_some(Phase::Shelf)
                } else {
                    (*t >= INSERT_S && *core_ready).then(|| Phase::Playing {
                        cart: std::mem::take(cart),
                    })
                }
            }
            Phase::Ejecting { t, .. } => {
                if self.screen <= 0.0 {
                    let was = *t;
                    // An NES or SNES cart comes out the way it went in, twice as fast.
                    *t += if was < 0.0 { dt } else { dt * 2.0 / pace };
                    touched = was < 0.0 && *t >= 0.0;
                }
                (*t >= EJECT_S).then_some(Phase::Shelf)
            }
            _ => None,
        };
        if touched {
            self.sfx = Some(match self.phase {
                Phase::Ejecting { .. } => Sfx::Eject,
                _ => Sfx::Insert,
            });
        }
        if sgb_in {
            self.phase = Phase::Shelf;
            self.seat_sgb();
        } else if let Some(phase) = next {
            self.phase = phase;
            self.refused_from = None;
            let seated = match &self.phase {
                Phase::Playing { cart } => Some(cart.clone()),
                _ => None,
            };
            if seated.is_none() {
                self.link_loaded = None;
            }
            self.record_cart(seated);
        }
        self.step_screen(dt);
    }

    fn step_screen(&mut self, dt: f32) {
        let lit = matches!(self.phase, Phase::Playing { .. } | Phase::Polaroids { .. });
        // Coming back from the boot screen's picture, the game is simply there: no fade.
        if self.resume_picture.is_some() || self.picture_wanted {
            self.screen = if lit { 1.0 } else { 0.0 };
        }
        let step = if lit {
            dt / POWER_ON_S
        } else {
            -dt / POWER_OFF_S
        };
        self.screen = (self.screen + step).clamp(0.0, 1.0);
        let docked = matches!(
                self.phase,
                Phase::Shelf | Phase::Inserting { .. } | Phase::Ejecting { .. }
            );
        let sliding = matches!(
                self.phase,
                Phase::Inserting { .. }
                    | Phase::Playing { .. }
                    | Phase::Polaroids { .. }
                    | Phase::Ejecting { .. }
            );
        let reach = if sliding { dt / POWER_ON_S } else { 1.0 };
        self.lift = match docked {
            true => (self.lift - reach).max(0.0),
            false => (self.lift + reach).min(1.0),
        };
    }

    pub fn set_panel(&mut self, panel: (u32, u32)) {
        self.panel = panel;
    }

    pub fn layout(&self) -> Layout {
        if matches!(self.phase, Phase::Playing { .. }) && self.in_game_menu_shown().is_some() {
            return IN_GAME_MENU;
        }
        match &self.phase {
            Phase::Playing { .. } | Phase::Polaroids { .. } => Layout::Framed,
            Phase::Ejecting { .. } if self.overlay_slide() > 0.0 => Layout::Framed,
            Phase::Doze { cart: Some(_) } => Layout::Framed,
            _ => Layout::Native,
        }
    }

    pub fn canvas(&self) -> Canvas {
        Canvas::of(self.layout(), self.panel)
    }

    pub fn overlay_slide(&self) -> f32 {
        match &self.phase {
            Phase::Ejecting { t, .. } => ((-t - (EJECT_HOLD_S - POWER_ON_S)) / POWER_ON_S)
                .clamp(0.0, 1.0)
                .min(self.eject_overlay),
            Phase::Playing { .. } | Phase::Polaroids { .. } | Phase::Doze { cart: Some(_) } => {
                self.lift
            }
            _ => 0.0,
        }
    }

    fn framed_panel(&self) -> bool {
        slot_gfx::framed(self.panel)
    }

    fn draw_powering(&self, out: &mut Vec<Draw>) {
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: OUT_W as f32,
            h: OUT_H as f32,
            colour: [0.0, 0.0, 0.0, 1.0],
        });
        if self.game_ready && self.screen > 0.0 {
            out.push(Draw::Game);
        }
    }

    pub fn screen_power(&self) -> f32 {
        self.screen
    }

    pub fn set_game_ready(&mut self, ready: bool) {
        self.game_ready = ready;
    }

    pub fn game_ready(&self) -> bool {
        self.game_ready
    }

    pub fn game_visible(&self) -> bool {
        self.game_ready && self.screen > 0.0
    }

    pub fn tick_ms(&mut self, now: Millis) {
        self.clock = self.clock.max(now as f64);
        self.timers();
    }

    fn timers(&mut self) {
        if let Some(since) = self.power_held_since {
            // A tap sleeps; held past it, the bar counts down to powering off.
            let held = self.now().saturating_sub(since);
            if held >= slot_input::POWER_TAP_MS {
                let span = slot_input::POWER_HOLD_MS - slot_input::POWER_TAP_MS;
                let done = (held - slot_input::POWER_TAP_MS).min(span) * 100 / span;
                self.hud.show(HudKind::Power, done as u8, false, self.now());
            }
        }
        self.play_hold();
        self.repeat_held_dir();
        if self.auto_sleep_expired() {
            self.doze();
        }
        if self.doze_expired() {
            self.on_doze_timeout();
        }
        if self.now() >= self.autosave_at {
            self.autosave();
        }
        if self.now() >= self.card_check_at {
            self.card_check_at = self.now() + CARD_CHECK_MS;
            self.card_read_only = self.power.as_ref().is_some_and(|p| p.card_read_only());
        }
        let failing = self.card_writes.failing() || self.card_read_only;
        if failing != self.hud.alarmed() {
            eprintln!(
                "slot: card: alarm {} (read-only {})",
                if failing { "on" } else { "off" },
                self.card_read_only
            );
            self.log_card(failing);
        }
        self.hud.set_alarm(failing, self.now());
        if self.now() >= self.battery_at {
            self.battery_at = self.now() + BATTERY_POLL_MS;
            self.battery = self.power.as_ref().and_then(|p| p.battery());
            if let Some(b) = self.battery {
                self.on_battery(b);
            }
        }
        if self.now() >= self.headphones_at {
            self.headphones_at = self.now() + HEADPHONES_POLL_MS;
            let on = self.power.as_ref().is_some_and(|p| p.headphones());
            if on != self.headphones {
                self.headphones = on;
                self.hud.set_headphones(on);
                let now = self.now();
                self.hud
                    .show(HudKind::Volume, self.output_volume(), self.muted(), now);
            }
        }
        if self.now() >= self.charge_at {
            self.charge_at = self.now() + CHARGE_POLL_MS;
            if let (Some(power), Some(b)) = (self.power.as_ref(), self.battery.as_mut()) {
                b.charge = power.charge();
            }
            let state = self.led_state();
            self.set_led(state);
        }
        if let Some(GameMenu::Linked {
            since,
            opened: false,
            ..
        }) = self.game_menu
        {
            if self.now().saturating_sub(since) >= LINKED_HOLD_MS {
                self.game_menu = None;
            }
        }
        if let Some(GameMenu::Unplug { since, .. }) = self.game_menu {
            if self.now().saturating_sub(since) >= UNPLUG_HOLD_MS {
                self.game_menu = None;
            }
        }
        if let Some(at) = self.link.as_ref().and_then(|s| s.lost_at) {
            if self.now().saturating_sub(at) >= LINK_LOST_MS {
                self.end_link();
            }
        }
    }

    pub fn led_state(&self) -> LedState {
        let Some(b) = self.battery else {
            return LedState::Running;
        };
        match b.charge {
            Charge::Charging => LedState::Charging,
            Charge::Full => LedState::Charged,
            _ if b.percent <= BATTERY_LOW => LedState::Low,
            _ => LedState::Running,
        }
    }

    fn set_led(&mut self, state: LedState) {
        if self.shutting_down() && state != LedState::Off {
            return;
        }
        if self.last_led == Some(state) {
            return;
        }
        self.last_led = Some(state);
        if let Some(power) = self.power.as_mut() {
            power.set_led(state);
        }
    }

    fn restock_recents(&mut self) {
        let carts: Vec<Cart> = match self.state.show_recents {
            true => self
                .state
                .recents
                .iter()
                .filter_map(|r| {
                    self.carts()
                        .find(|c| c.platform == r.platform && c.stem == r.stem)
                })
                .cloned()
                .collect(),
            false => Vec::new(),
        };
        self.restock(RECENTS_AT, carts);
        self.mark_stars();
    }

    fn restock_favorites(&mut self) {
        let mut carts: Vec<Cart> = self
            .carts()
            .filter(|c| self.state.is_favorite(c.platform, &c.stem))
            .cloned()
            .collect();
        carts.sort_by(|a, b| {
            (slot_store::sort_key(&a.stem), a.platform as u8)
                .cmp(&(slot_store::sort_key(&b.stem), b.platform as u8))
        });
        let index = self.shelves[FAVORITES_AT].1.index;
        self.restock(FAVORITES_AT, carts);
        let shelf = &mut self.shelves[FAVORITES_AT].1;
        if self.shelf_at == FAVORITES_AT && !shelf.carts.is_empty() {
            shelf.select(index.min(shelf.carts.len() - 1));
        }
        self.mark_stars();
    }

    fn restock(&mut self, at: usize, carts: Vec<Cart>) {
        let stocked = &self.shelves[at].1.carts;
        if stocked.len() == carts.len()
            && stocked
                .iter()
                .zip(&carts)
                .all(|(a, b)| a.platform == b.platform && a.stem == b.stem)
        {
            return;
        }
        let old = self.shelves[at].1.restock(carts);
        self.orphaned.extend(old);
        let empty = self.shelves[at].1.carts.is_empty();
        if self.shelf_at == at && empty {
            self.shelf_at = self.next_shelf(1).unwrap_or(0);
        }
    }

    fn toggle_favorite(&mut self) {
        let Some(cart) = self.shelf().carts.get(self.shelf().index) else {
            return;
        };
        let (platform, stem) = (cart.platform, cart.stem.clone());
        self.state.toggle_favorite(platform, &stem);
        let shelf_at = self.shelf_at;
        self.restock_favorites();
        if self.shelf_at != shelf_at {
            self.shelf_platform = slot_ui::Printed::default();
            self.slot_letter = None;
            self.shelf_named = None;
            self.name_pending = true;
        }
        self.persist();
    }

    fn record_cart(&mut self, cart: Option<String>) {
        let platform = self.seated_cart().map(|c| c.platform);
        if let (Some(stem), Some(platform)) = (&cart, platform) {
            let first = self.state.recents.first();
            if first.is_none_or(|r| r.platform != platform || r.stem != *stem) {
                let stem = stem.clone();
                self.state.played(platform, &stem);
                self.restock_recents();
                self.persist();
            }
        }
        let sgb = cart.is_some() && self.sgb;
        if self.state.cart == cart && self.state.cart_platform == platform && self.state.cart_sgb == sgb {
            return;
        }
        self.state.cart = cart;
        self.state.cart_platform = platform;
        self.state.cart_sgb = sgb;
        self.persist();
    }

    fn persist(&self) {
        let Some(root) = &self.root else {
            return;
        };
        self.state_writer.flush();
        let written = write_slot_state(root, &self.state);
        if let Err(e) = &written {
            eprintln!("slot: slot.state: {e}");
        }
        self.card_writes.wrote("slot.state", &written);
    }

    fn persist_soon(&self) {
        if let Some(root) = &self.root {
            self.state_writer.save(root, &self.state);
        }
    }

    pub fn flush_state(&self) {
        self.state_writer.flush();
    }

    pub fn on_core_ready(&mut self) {
        let Phase::Inserting {
            cart, core_ready, ..
        } = &mut self.phase
        else {
            return;
        };
        *core_ready = true;
        let cart = cart.clone();
        self.retire_refused_resume(&cart);
    }

    fn retire_refused_resume(&mut self, stem: &str) {
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        if snapshot.resume_trusted() || self.resume_unread {
            return;
        }
        if !self.named_core {
            return;
        }
        let Some(root) = &self.root else {
            return;
        };
        if let Some(h) = self.pending_save.take() {
            let _ = h.join();
        }
        let ring = StateRing::new(root, self.platform, self.core, stem);
        match ring.retire_resume(&format_stamp(self.wall_secs())) {
            Ok(Some(to)) => eprintln!(
                "slot: resume: {} refused this state, moved it to {}",
                self.core.as_str(),
                to.display()
            ),
            Ok(None) => {}
            Err(e) => eprintln!("slot: resume: could not move the refused state aside: {e}"),
        }
    }

    pub fn on_core_failed(&mut self) {
        let caught = self.seat();
        let Phase::Inserting { cart, .. } = &mut self.phase else {
            return;
        };
        let cart = std::mem::take(cart);
        self.refuse_out(cart, caught);
    }

    fn refuse_out(&mut self, cart: String, caught: f32) {
        let t = (1.0 - caught) * EJECT_S;
        self.phase = Phase::Ejecting { cart, t };
        self.refused_from = Some(t);
    }

    pub fn refuse(&mut self) {
        self.refusal = Some(Refusal::started(self.now()));
    }

    pub fn refusal_active(&self, now: Millis) -> bool {
        self.refusal.is_some_and(|r| r.active(now))
    }

    pub fn seat(&self) -> f32 {
        match &self.phase {
            Phase::Shelf => 0.0,
            Phase::Inserting { t, resumed, .. } => {
                if *resumed {
                    1.0
                } else {
                    (t / SEATED_AT).clamp(0.0, 1.0)
                }
            }
            Phase::Ejecting { t, .. } => 1.0 - (t / EJECT_S).clamp(0.0, 1.0),
            _ => 1.0,
        }
    }

    pub fn draw(&self, out: &mut Vec<Draw>) {
        let start = out.len();
        self.draw_every(out);
        // A refusal in a menu (States or Screenshots with nothing in them): its words shake no.
        let in_menu = matches!(self.phase, Phase::QuickMenu { .. }) || self.in_game_menu.is_some();
        let dx = self.refusal.map_or(0.0, |r| r.offset(self.now()));
        if in_menu && dx != 0.0 {
            for d in &mut out[start..] {
                if let Draw::Tint { x, .. } = d {
                    *x += dx;
                }
            }
        }
    }

    fn draw_every(&self, out: &mut Vec<Draw>) {
        match &self.shots {
            Some(view)
                if !self.capturing && !self.shutting_down() =>
            {
                self.draw_shots(view, out);
                if !self.shot_pending {
                    self.hud.draw(self.canvas(), self.now(), out);
                }
            }
            _ => self.draw_screen(out),
        }
        if let (false, Some(at)) = (self.capturing, self.shot_flash) {
            let t = self.now().saturating_sub(at) as f32 / SHOT_FLASH_MS as f32;
            if t < 1.0 {
                let alpha = 0.9 * (1.0 - t) * (1.0 - t);
                out.push(Draw::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: self.canvas().w,
                    h: OUT_H as f32,
                    colour: [1.0, 1.0, 1.0, alpha],
                });
            }
        }
    }

    fn draw_screen(&self, out: &mut Vec<Draw>) {
        if self.capturing {
            out.push(Draw::Game);
            return;
        }
        if let Some(tex) = self.resume_picture {
            if !(matches!(self.phase, Phase::Playing { .. }) && self.game_ready) {
                out.push(Draw::Tex {
                    x: 0.0,
                    y: 0.0,
                    w: OUT_W as f32,
                    h: OUT_H as f32,
                    tex,
                    alpha: 1.0,
                });
                return;
            }
        }
        if self.shutting_down() {
            out.push(Draw::Rect {
                x: 0.0,
                y: 0.0,
                w: self.canvas().w,
                h: OUT_H as f32,
                colour: [0.0, 0.0, 0.0, 1.0],
            });
            // Straight to black: no words while it powers down.
            return;
        }
        match &self.phase {
            Phase::SetClock {
                picker, from_menu, ..
            } => {
                let (line, hint) = match self.clock_faces {
                    Some((line, hint)) => (Some(line), Some(hint)),
                    None => (None, None),
                };
                let back = self
                    .quick_menu_faces
                    .as_ref()
                    .filter(|_| *from_menu)
                    .map(|f| f.legend[0]);
                picker.draw(self.canvas(), self.menu_ground(), self.menu_ink(), line, hint, back, out);
                return;
            }
            Phase::QuickMenu { row } => QuickMenu {
                row: *row,
                hidden: self.hidden_rows(),
                values: QuickRow::ALL.map(|r| self.quick_value(r)),
                clock: self.quick_clock_faces,
                network: self.quick_network_faces,
                zone: self.quick_zone_faces,
                locked: self.state.auto_time.then_some(QuickRow::DateTime),
                wallpaper: self.quick_wallpaper_faces,
                faces: self.quick_menu_faces.as_ref(),
                ground: self.menu_ground(),
                palette: self.quick_palette_faces,
                gba_bezel: self.quick_bezel_faces[bezel_slot(Platform::Gba)],
                gb_bezel: self.quick_bezel_faces[bezel_slot(Platform::Gb)],
                gbc_bezel: self.quick_bezel_faces[bezel_slot(Platform::Gbc)],
                bezel: None,
                over_game: false,
                canvas: self.canvas(),
                ink: self.menu_ink(),
                first: self.menu_first(*row, self.hidden_rows()),
                named_as: if self.state.gb_palettes { &[] } else { &[(QuickRow::GbShader, QuickRow::GbcShader)] },
            }
            .draw(out),
            Phase::Shelf => {
                self.draw_background(out);
                match (self.core_picker_shown(), self.selected_stem()) {
                    _ if self.shell_picker.is_some() => {
                        // The studio: the shelf parts around the cart and fades as it lifts.
                        let picker = self.shell_picker.as_ref().unwrap();
                        let open = ease(picker.openness(self.now()));
                        self.shelf().draw_row(
                            self.canvas(),
                            Some(&picker.stem),
                            0.0,
                            open,
                            1.0 - open,
                            out,
                        );
                        draw_empty_slot(self.canvas(), self.slot_faces(), out);
                        draw_slot_name(
                            self.canvas(),
                            self.shelf_platform,
                            self.slot_name_alpha() * (1.0 - open),
                            self.menu_ink().text,
                            out,
                        );
                        slot_ui::draw_slot_title(
                            self.canvas(),
                            self.shelf_title,
                            self.title_alpha() * (1.0 - open),
                            self.menu_ink().text,
                            out,
                        );
                    }
                    (Some(picker), Some(stem)) => {
                        let open = ease(picker.openness(self.now()));
                        let dim = 1.0 + (CORE_PICKER_DIM - 1.0) * open;
                        self.shelf().draw_row(
                            self.canvas(),
                            Some(stem),
                            0.0,
                            CORE_PICKER_RECEDE * open,
                            dim,
                            out,
                        );
                        draw_empty_slot(self.canvas(), self.slot_faces(), out);
                    }
                    _ => {
                        self.shelf()
                            .draw(self.canvas(), self.shelf_shake(), self.slot_faces(), out);
                        draw_slot_name(
                            self.canvas(),
                            self.shelf_platform,
                            self.slot_name_alpha(),
                            self.menu_ink().text,
                            out,
                        );
                        slot_ui::draw_slot_title(
                            self.canvas(),
                            self.shelf_title,
                            self.title_alpha(),
                            self.menu_ink().text,
                            out,
                        );
                    }
                }
                draw_footer(
                    self.canvas(),
                    self.battery,
                    self.battery_percent,
                    self.bolt,
                    self.shelf_clock,
                    self.wifi_shown(self.wifi),
                    footer_ink(self.light()),
                    out,
                );
            }
            Phase::Buttons { page } => {
                slot_ui::ButtonsHelp {
                    page: *page,
                    faces: self.buttons_faces.as_ref(),
                    ground: self.menu_ground(),
                    canvas: self.canvas(),
                    ink: self.menu_ink(),
                }
                .draw(out);
                return;
            }
            Phase::About => {
                slot_ui::draw_about(self.canvas(), self.sticker_face, out);
                if let Some(back) = self.buttons_faces.as_ref().map(|f| [f.legend[1]]) {
                    let ink = self.menu_ink();
                    for (tex, w, x) in
                        slot_ui::centred_hints(self.canvas(), &back, slot_ui::LEGEND_GAP)
                    {
                        out.push(Draw::Tint {
                            x,
                            y: slot_ui::MENU_LEGEND_Y,
                            w: w as f32,
                            h: slot_ui::HINT_H as f32,
                            tex,
                            colour: ink.text,
                        });
                    }
                }
                return;
            }
            Phase::Inserting { cart, resumed, .. } => {
                if !resumed {
                    self.draw_background(out);
                    self.shelf()
                        .draw_row(self.canvas(), Some(cart), 0.0, self.recede(cart), 1.0, out);
                }
                self.chrome(cart, 0.0, out);
            }
            Phase::Ejecting { .. }
                if self.framed_panel()
                    && (self.layout() == Layout::Framed || self.screen > 0.0) =>
            {
                self.draw_powering(out)
            }
            Phase::Playing { .. } if self.framed_panel() && self.screen < 1.0 => {
                self.draw_powering(out)
            }
            Phase::Ejecting { cart, .. } => {
                self.draw_background(out);
                self.shelf()
                    .draw_row(self.canvas(), Some(cart), 0.0, self.recede(cart), 1.0, out);
                self.chrome(cart, 0.0, out);
            }
            Phase::Playing { cart } if self.screen < 1.0 => {
                self.draw_background(out);
                self.chrome(cart, 0.0, out);
            }
            Phase::Playing { .. } => self.push_game(out),
            Phase::Polaroids { .. } => {
                self.push_game(out);
                if let Some(p) = &self.polaroids {
                    p.draw(
                        self.battery,
                        self.battery_percent,
                        self.hud_bolt,
                        self.shelf_clock,
                        self.wifi_shown(self.hud_wifi),
                        out,
                    );
                }
            }
            Phase::Doze { .. } => {
                out.push(Draw::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: self.canvas().w,
                    h: OUT_H as f32,
                    colour: [0.0, 0.0, 0.0, 1.0],
                });
                return;
            }
        }
        let page = self.cheat_page.filter(|_| self.in_game_menu_shown().is_some());
        if let (Some((sel, first)), Phase::Playing { .. }, true) =
            (page, &self.phase, self.screen >= 1.0)
        {
            let on: Vec<bool> = self.cheats.iter().map(|c| c.on).collect();
            slot_ui::CheatPage {
                sel,
                first,
                on: &on,
                faces: self.cheat_faces.as_ref(),
                canvas: self.canvas(),
            }
            .draw(out);
        } else if let (Some(row), Phase::Playing { .. }, true) =
            (self.in_game_menu_shown(), &self.phase, self.screen >= 1.0)
        {
            let hidden = self.in_game_hidden();
            // A Game Boy game in its own colors has the GBC's LCD Filter, not the Dot Matrix.
            let shader = match self.playing_row(QuickRow::Shader) {
                QuickRow::GbShader if !self.palette_live => QuickRow::GbcShader,
                row => row,
            };
            let named_as = [(QuickRow::Shader, shader)];
            QuickMenu {
                row,
                hidden: &hidden,
                named_as: &named_as,
                values: QuickRow::ALL.map(|r| self.quick_value(r)),
                clock: None,
                network: None,
                zone: None,
                locked: None,
                wallpaper: None,
                faces: self.quick_menu_faces.as_ref(),
                ground: None,
                palette: self.quick_palette_faces,
                gba_bezel: None,
                gb_bezel: None,
                gbc_bezel: None,
                bezel: self.quick_bezel_faces[bezel_slot(self.playing_platform())],
                over_game: true,
                canvas: self.canvas(),
                ink: slot_ui::theme::DARK,
                first: self.menu_first(row, &hidden),
            }
            .draw(out);
            // The settings pages fill the screen; only the centred menu leaves room for the
            // battery and clock.
            if row.centred() {
                slot_ui::draw_corners(
                    self.canvas(),
                    self.battery,
                    self.battery_percent,
                    self.hud_bolt,
                    self.shelf_clock,
                    self.wifi_shown(self.hud_wifi),
                    out,
                );
                if let (true, Some([menu, _])) = (self.cheat_icon_shown(), self.cheat_badges) {
                    let x = slot_ui::corner_clock_end(self.shelf_clock) + MENU_BADGE_GAP;
                    slot_ui::draw_cheat_badge_in_menu(x, menu, out);
                }
            }
        }
        if let (Phase::Playing { .. }, None, true, Some([_, game])) = (
            &self.phase,
            self.in_game_menu_shown(),
            self.cheat_icon_shown(),
            self.cheat_badges,
        ) {
            if self.screen >= 1.0 && self.game_menu.is_none() {
                slot_ui::draw_cheat_badge_in_game(self.canvas(), game, out);
            }
        }
        if let Some(picker) = self.core_picker_shown() {
            self.draw_core_picker(&picker, out);
        }
        if let Some(picker) = &self.shell_picker {
            self.draw_shell_picker(picker, out);
        }
        if let Some(search) = &self.label_search {
            self.draw_label_search(search, out);
        }
        if let Some(menu) = self.game_menu {
            self.draw_game_menu(menu, out);
        }
        if !self.shot_pending {
            self.hud.draw(self.canvas(), self.now(), out);
        }
    }

    pub fn screen_shake(&self) -> f32 {
        self.shake_at(self.now())
    }

    pub fn shake_at(&self, now: Millis) -> f32 {
        self.shake_when(matches!(self.phase, Phase::Playing { .. }), now)
    }

    pub fn shelf_shake(&self) -> f32 {
        self.shake_when(
            self.on_shelf() && self.core_picker.is_none() && self.shell_picker.is_none(),
            self.now(),
        )
    }

    fn shake_when(&self, mine: bool, now: Millis) -> f32 {
        if !mine {
            return 0.0;
        }
        self.refusal.map_or(0.0, |r| r.offset(now))
    }

    fn push_game(&self, out: &mut Vec<Draw>) {
        if self.game_visible() {
            out.push(Draw::Game);
        }
    }

    fn chrome(&self, stem: &str, dim: f32, out: &mut Vec<Draw>) {
        let Some((cart, face)) = self.shelf().find(stem) else {
            return;
        };
        let alpha = self.alert_alpha();
        let (rest, scale) = self.shelf().selected_at(self.canvas());
        SlotChrome {
            cart,
            face,
            rest,
            scale,
            seat: self.seat(),
            alert: self.alert_face.filter(|_| alpha > 0.0).map(|t| (t, alpha)),
            dim,
            screen: self.screen,
            game: self.game_ready,
            over: !matches!(
                &self.phase,
                Phase::Inserting { resumed: true, .. }
                    | Phase::Playing { .. }
                    | Phase::Ejecting { .. }
            ),
            behind: self.shelf().behind(stem),
            slot: self.slot_faces(),
            canvas: self.canvas(),
            // Every cart seats with its top level with the slot's top edge.
            lift: slot_ui::GB_SEAT_LIFT,
            lid: self
                .nes_lids
                .filter(|_| cart.platform == Platform::Nes)
                .map(|l| l[usize::from(self.light())]),
            light: self.light(),
        }
        .draw(out);
    }

    pub fn alert_visible(&self) -> bool {
        self.alert_alpha() > 0.0
    }

    pub fn alert_alpha(&self) -> f32 {
        let (Some(from), Phase::Ejecting { t, .. }) = (self.refused_from, &self.phase) else {
            return 0.0;
        };
        let span = EJECT_S - from;
        if span <= 0.0 {
            return 0.0;
        }
        let u = ((t - from) / span).clamp(0.0, 1.0);
        ((ALERT_GONE - u) / (ALERT_GONE - ALERT_HOLD)).clamp(0.0, 1.0)
    }

    pub fn set_alert_face(&mut self, face: TexId) {
        self.alert_face = Some(face);
    }

    pub fn set_core_interior_face(&mut self, interior: TexId) {
        self.core_interior_face = Some(interior);
    }

    pub fn set_core_board_faces(&mut self, board: TexId, lid: TexId) {
        self.core_board_face = Some(board);
        self.core_lid_face = Some(lid);
        self.core_faces_stem = self.selected_stem().map(str::to_string);
    }

    pub fn set_core_part_faces(
        &mut self,
        sockets: Vec<TexId>,
        chips: Vec<TexId>,
        blank: TexId,
        shadow: TexId,
    ) {
        self.core_socket_faces = sockets;
        self.core_chip_faces = chips;
        self.core_blank_chip_face = Some(blank);
        self.core_chip_shadow_face = Some(shadow);
    }

    pub fn set_core_legend_faces(&mut self, faces: Vec<(TexId, u32)>) {
        self.core_legend_faces = faces;
    }

    pub fn set_link_menu_faces(&mut self, faces: Vec<(TexId, u32, u32)>) {
        self.link_menu_faces = faces;
    }

    pub fn set_link_linked_face(&mut self, face: (TexId, u32, u32)) {
        self.link_linked_face = Some(face);
    }

    pub fn set_link_step_faces(&mut self, faces: Vec<(TexId, u32, u32)>) {
        self.link_step_faces = faces;
    }

    pub fn set_link_fail_faces(&mut self, faces: Vec<(TexId, u32, u32)>) {
        self.link_fail_faces = faces;
    }

    pub fn set_link_legend_faces(&mut self, faces: Vec<(TexId, u32)>) {
        self.link_legend_faces = faces;
    }

    pub fn set_link_sprites(&mut self, sprites: LinkSprites) {
        self.link_sprites = Some(sprites);
    }

    pub fn set_link_shot(&mut self, tex: Option<TexId>) {
        self.link_shot = tex;
    }

    pub fn set_link_port(&mut self, tex: TexId) {
        if let Some(sprites) = &mut self.link_sprites {
            sprites.port.tex = tex;
        }
    }

    pub fn link_sprites_ready(&self) -> bool {
        self.link_sprites.is_some()
    }

    /// The first row to show for `row`'s page, remembering it for the next frame.
    fn menu_first(&self, row: QuickRow, hidden: &[QuickRow]) -> usize {
        let page = row.page()[0].index();
        let (last_page, prev) = self.menu_scroll.get();
        let prev = if last_page == page { prev } else { 0 };
        let first = slot_ui::quick_first(row, hidden, prev);
        self.menu_scroll.set((page, first));
        first
    }

    fn core_picker_shown(&self) -> Option<CorePicker> {
        self.core_picker.filter(|p| !p.waiting())
    }

    fn draw_core_picker(&self, picker: &CorePicker, out: &mut Vec<Draw>) {
        let now = self.now();
        if let Some(stem) = self.selected_stem() {
            if let Some((cart, face)) = self.shelf().find(stem) {
                if matches!(cart.platform, Platform::Gb | Platform::Gbc) {
                    self.draw_handheld_core_picker(picker, slot_ui::mould_of(cart), face, now, out);
                    return;
                }
            }
        }
        let progress = picker.openness(now);
        let lift = lift_of(progress);
        let (rest, scale) = self.shelf().selected_at(self.canvas());
        let shelf = shelf_cart_at(rest, scale);
        let board = board_from(self.canvas(), shelf, progress);
        let zoom = board_zoom(board);
        let ready = self.core_faces_ready();

        if ready {
            if let Some(tex) = self.core_interior_face {
                out.push(Draw::Tex {
                    x: board.x,
                    y: board.y,
                    w: board.w,
                    h: board.h,
                    tex,
                    alpha: 1.0,
                });
            }
            if let Some(tex) = self.core_board_face {
                out.push(Draw::Tex {
                    x: board.x,
                    y: board.y,
                    w: board.w,
                    h: board.h,
                    tex,
                    alpha: 1.0,
                });
            }
            for (side, core) in picker.choices().iter().copied().enumerate() {
                let Some(tex) = self.core_socket_faces.get(core.index()).copied() else {
                    continue;
                };
                let (x, y) = on_board(board, SOCKET_U[side], SOCKET_V);
                out.push(Draw::Tex {
                    x: x.round(),
                    y: y.round(),
                    w: SOCKET_W as f32 * zoom,
                    h: SOCKET_H as f32 * zoom,
                    tex,
                    alpha: 1.0,
                });
            }

            let chip = picker.chip(now);
            let u = CHIP_U[0] + (CHIP_U[1] - CHIP_U[0]) * chip.across;
            if chip.lift > 0.0 {
                if let Some(tex) = self.core_chip_shadow_face {
                    let (cx, cy) = on_board(board, u + 19.0, CHIP_V + 29.4);
                    let (w, h) = (SHADOW_W as f32 * zoom, SHADOW_H as f32 * zoom);
                    out.push(Draw::Tex {
                        x: cx - w / 2.0,
                        y: cy - h / 2.0,
                        w,
                        h,
                        tex,
                        alpha: 0.6 * chip.lift * lift,
                    });
                }
            }
            let face = match chip.seated {
                Some(core) => self.core_chip_faces.get(core.index()).copied(),
                None => self.core_blank_chip_face,
            };
            if let Some(tex) = face {
                let (x, y) = on_board(board, u, CHIP_V - HOP_LIFT * chip.lift);
                let body = Placed {
                    x: x + chip.shake,
                    y,
                    w: CHIP_W as f32 * zoom,
                    h: CHIP_H as f32 * zoom,
                };
                let at = grown(body, TURN_PAD as f32 * zoom);
                out.push(Draw::Turned {
                    x: at.x.round(),
                    y: at.y.round(),
                    w: at.w,
                    h: at.h,
                    tex,
                    alpha: 1.0,
                    turn: chip.tip,
                });
            }
        }

        if let Some(tex) = self.core_chip_shadow_face {
            let (lid, _) = lid_from(self.canvas(), shelf, progress);
            let k = lid.w / lid_at(self.canvas(), 1.0).0.w;
            let (w, h) = (LID_SHADOW_W * k, LID_SHADOW_H * k);
            out.push(Draw::Tex {
                x: lid.x + (lid.w - w) / 2.0,
                y: lid.y + lid.h + LID_SHADOW_DROP * k - h / 2.0,
                w,
                h,
                tex,
                alpha: LID_SHADOW_ALPHA * lift,
            });
        }

        if ready {
            if let Some(tex) = self.core_lid_face {
                let (lid, turn) = lid_from(self.canvas(), shelf, progress);
                let at = grown(lid, TURN_PAD as f32 * lid.w / CART_W as f32);
                out.push(Draw::Turned {
                    x: at.x,
                    y: at.y,
                    w: at.w,
                    h: at.h,
                    tex,
                    alpha: 1.0,
                    turn,
                });
            }
        } else if let Some((_, Some(tex))) = self
            .selected_stem()
            .and_then(|stem| self.shelf().find(stem))
        {
            let (lid, turn) = lid_from(self.canvas(), shelf, progress);
            out.push(Draw::Turned {
                x: lid.x,
                y: lid.y,
                w: lid.w,
                h: lid.h,
                tex,
                alpha: 1.0,
                turn,
            });
        }

        if let [cancel, swap, choose] = self.core_legend_faces.as_slice() {
            let right = board_x(self.canvas()) + BOARD_W as f32;
            let seen = |w: u32| w.saturating_sub(HINT_EDGE) as f32;
            for (tex, w, x) in [
                (cancel.0, cancel.1, board_x(self.canvas())),
                (swap.0, swap.1, self.canvas().centred(seen(swap.1))),
                (choose.0, choose.1, right - seen(choose.1)),
            ] {
                out.push(Draw::Tex {
                    x: x.round(),
                    y: CORE_LEGEND_Y,
                    w: w as f32,
                    h: HINT_H as f32,
                    tex,
                    alpha: lift,
                });
            }
        }
    }

    fn draw_handheld_core_picker(
        &self,
        picker: &CorePicker,
        mould: Mould,
        face: Option<TexId>,
        now: Millis,
        out: &mut Vec<Draw>,
    ) {
        let progress = picker.openness(now);
        let movement = ease(progress);
        let reveal = lift_of(progress);
        let (rest_x, rest_scale) = self.shelf().selected_at(self.canvas());
        let (cart_w, cart_h) = cart_box(mould);
        let source = Placed {
            x: rest_x,
            y: foot_y(cart_h as f32) - cart_h as f32 * rest_scale,
            w: cart_w as f32 * rest_scale,
            h: cart_h as f32 * rest_scale,
        };
        let scale = 0.56;
        let target = Placed {
            x: (OUT_W as f32 - cart_w as f32 * scale) / 2.0,
            y: 30.0,
            w: cart_w as f32 * scale,
            h: cart_h as f32 * scale,
        };
        let cart = Placed {
            x: source.x + (target.x - source.x) * movement,
            y: source.y + (target.y - source.y) * movement,
            w: source.w + (target.w - source.w) * movement,
            h: source.h + (target.h - source.h) * movement,
        };

        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: OUT_W as f32,
            h: OUT_H as f32,
            colour: [0.0, 0.0, 0.0, 0.32 * movement],
        });
        if let Some(tex) = face {
            out.push(Draw::Tex {
                x: cart.x,
                y: cart.y,
                w: cart.w,
                h: cart.h,
                tex,
                alpha: 1.0,
            });
        }

        if reveal > 0.0 {
            let scale = 2.1;
            let centres = [215.0_f32, 425.0_f32];
            let socket_w = SOCKET_W as f32 * scale;
            let socket_h = SOCKET_H as f32 * scale;
            let socket_y = 267.0;
            for (side, core) in picker.choices().iter().copied().enumerate() {
                let Some(tex) = self.core_socket_faces.get(core.index()).copied() else {
                    continue;
                };
                out.push(Draw::Tex {
                    x: centres[side] - socket_w / 2.0,
                    y: socket_y,
                    w: socket_w,
                    h: socket_h,
                    tex,
                    alpha: reveal,
                });
            }

            let chip = picker.chip(now);
            let centre_x = centres[0] + (centres[1] - centres[0]) * chip.across;
            let body = Placed {
                x: centre_x - CHIP_W as f32 * scale / 2.0 + chip.shake,
                y: socket_y + (socket_h - CHIP_H as f32 * scale) / 2.0
                    - HOP_LIFT * scale * chip.lift,
                w: CHIP_W as f32 * scale,
                h: CHIP_H as f32 * scale,
            };
            if chip.lift > 0.0 {
                if let Some(tex) = self.core_chip_shadow_face {
                    let w = SHADOW_W as f32 * scale;
                    let h = SHADOW_H as f32 * scale;
                    out.push(Draw::Tex {
                        x: centre_x - w / 2.0,
                        y: socket_y + socket_h + 8.0 - h / 2.0,
                        w,
                        h,
                        tex,
                        alpha: 0.6 * chip.lift * reveal,
                    });
                }
            }
            let face = match chip.seated {
                Some(core) => self.core_chip_faces.get(core.index()).copied(),
                None => self.core_blank_chip_face,
            };
            if let Some(tex) = face {
                let at = grown(body, TURN_PAD as f32 * scale);
                out.push(Draw::Turned {
                    x: at.x,
                    y: at.y,
                    w: at.w,
                    h: at.h,
                    tex,
                    alpha: reveal,
                    turn: chip.tip,
                });
            }
        }

        if let [cancel, swap, choose] = self.core_legend_faces.as_slice() {
            let board_x = board_x(self.canvas());
            let right = board_x + BOARD_W as f32;
            let seen = |w: u32| w.saturating_sub(HINT_EDGE) as f32;
            for (tex, w, x) in [
                (cancel.0, cancel.1, board_x),
                (swap.0, swap.1, (OUT_W as f32 - seen(swap.1)) / 2.0),
                (choose.0, choose.1, right - seen(choose.1)),
            ] {
                out.push(Draw::Tex {
                    x: x.round(),
                    y: CORE_LEGEND_Y,
                    w: w as f32,
                    h: HINT_H as f32,
                    tex,
                    alpha: reveal,
                });
            }
        }
    }

    fn draw_game_menu(&self, menu: GameMenu, out: &mut Vec<Draw>) {
        if let Some(tex) = self.link_shot {
            out.push(Draw::Tex {
                x: 0.0,
                y: 0.0,
                w: OUT_W as f32,
                h: OUT_H as f32,
                tex,
                alpha: 1.0,
            });
        }
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: OUT_W as f32,
            h: OUT_H as f32,
            colour: LINK_SCRIM,
        });
        if let Some(sprites) = &self.link_sprites {
            crate::link_screen::draw_link_art(menu, self.link_hardware, self.now(), sprites, out);
            // Which network a link started now would use, read the way the service will decide
            // it: a device on the home network links over it.
            crate::link_screen::draw_network_label(menu, self.radio.home_connected(), sprites, out);
        }
        let line = match menu {
            GameMenu::Pick(role) => self.link_menu_faces.get(role.index()).copied(),
            GameMenu::Working { step, .. } => self
                .link_step_faces
                .get(step.shown(self.radio.warmed()).index())
                .copied(),
            GameMenu::Linked { .. } => self.link_linked_face,
            GameMenu::Failed { fail, .. } => fail
                .shown()
                .and_then(|i| self.link_fail_faces.get(i).copied()),
            GameMenu::Unplug { .. } => None,
        };
        if let Some((tex, w, h)) = line {
            out.push(Draw::Tex {
                x: ((OUT_W as f32 - w as f32) / 2.0).round(),
                y: LINK_TEXT_Y,
                w: w as f32,
                h: h as f32,
                tex,
                alpha: 1.0,
            });
        }
        let switchable = self.seated().is_some_and(|stem| self.link_switchable(stem));
        // Cancel and Mode stay on the left of the footer, Swap and Link on the right, so the
        // socket in the middle of the strip stays clear.
        let (left, right): (&[LinkLegend], &[LinkLegend]) = match menu {
            GameMenu::Pick(_) if switchable => (
                &[LinkLegend::Cancel, LinkLegend::Mode],
                &[LinkLegend::Swap, LinkLegend::Link],
            ),
            GameMenu::Pick(_) => (&[LinkLegend::Cancel], &[LinkLegend::Swap, LinkLegend::Link]),
            GameMenu::Working { .. } => (&[LinkLegend::Cancel], &[]),
            GameMenu::Linked { opened: true, .. } => (&[LinkLegend::Back], &[LinkLegend::EndLink]),
            GameMenu::Linked { .. } => (&[], &[]),
            GameMenu::Failed { .. } => (&[], &[LinkLegend::Ok]),
            GameMenu::Unplug { .. } => (&[], &[]),
        };
        self.draw_link_keys(left, false, out);
        self.draw_link_keys(right, true, out);
    }

    fn draw_link_keys(&self, keys: &[LinkLegend], right: bool, out: &mut Vec<Draw>) {
        let faces: Vec<(TexId, u32)> = keys
            .iter()
            .filter_map(|k| self.link_legend_faces.get(k.index()).copied())
            .collect();
        if faces.is_empty() {
            return;
        }
        let seen = |w: u32| w.saturating_sub(HINT_EDGE) as f32;
        let total = faces.iter().map(|(_, w)| seen(*w)).sum::<f32>()
            + LINK_LEGEND_GAP * faces.len().saturating_sub(1) as f32;
        let mut x = if right {
            (OUT_W as f32 - LINK_LEGEND_EDGE - total).round()
        } else {
            LINK_LEGEND_EDGE
        };
        for (tex, w) in faces {
            out.push(Draw::Tex {
                x,
                y: LINK_LEGEND_Y,
                w: w as f32,
                h: HINT_H as f32,
                tex,
                alpha: 1.0,
            });
            x += seen(w) + LINK_LEGEND_GAP;
        }
    }

    fn auto_link(&self, stem: &str) -> Option<(&Cart, LinkKind)> {
        let cart = self.shelf().carts.iter().find(|c| c.stem == stem)?;
        let auto = link_kind(&cart.code, &cart.title, slot_store::header_clean(&cart.rom));
        Some((cart, auto))
    }

    fn on_shelf(&self) -> bool {
        matches!(self.phase, Phase::Shelf)
    }

    pub fn browsed_rom(&self) -> Option<&std::path::Path> {
        if !self.on_shelf() {
            return None;
        }
        let shelf = self.shelf();
        shelf.carts.get(shelf.index).map(|c| c.rom.as_path())
    }

    fn insert(&mut self, clean: bool) {
        if !self.on_shelf() {
            return;
        }
        let at = self.shelf().index;
        let Some(cart) = self.shelf_mut().carts.get_mut(at).map(|c| {
            c.read_header();
            c.stem.clone()
        }) else {
            return;
        };
        eprintln!(
            "slot: launch: insert {cart} at {:.1} ms",
            crate::boot_time::ms()
        );
        self.play_held = None;
        self.refusal = None;
        self.refused_from = None;
        self.phase = Phase::Inserting {
            cart,
            t: 0.0,
            core_ready: false,
            resumed: false,
            clean,
        };
    }

    pub fn starting_clean(&self) -> bool {
        matches!(self.phase, Phase::Inserting { clean: true, .. })
    }

    fn play_hold(&mut self) {
        let Some(at) = self.play_held else {
            return;
        };
        if !self.on_shelf() {
            self.play_held = None;
            return;
        }
        if self.now().saturating_sub(at) >= PLAY_HOLD_MS {
            self.insert(true);
        }
    }

    fn eject(&mut self) {
        if self.single_cart() {
            return self.refuse();
        }
        self.eject_overlay = match self.layout() {
            Layout::Framed => self.overlay_slide(),
            Layout::Native => 0.0,
        };
        self.in_game_menu = None;
        let cart = match &mut self.phase {
            Phase::Playing { cart } | Phase::Inserting { cart, .. } => std::mem::take(cart),
            _ => return,
        };
        self.end_link();
        self.close_game_menu();
        self.flush_eject(&cart);
        self.refusal = None;
        self.refused_from = None;
        self.phase = Phase::Ejecting {
            cart,
            t: -EJECT_HOLD_S,
        };
    }

    fn flush_eject(&mut self, stem: &str) {
        self.settle_saves();
        let (Some(root), Some(snapshot)) = (&self.root, &self.snapshot) else {
            return;
        };
        let Some(state) = snapshot.state() else {
            eprintln!("slot: eject: the core gave up no state");
            return;
        };
        let (state, sav) = trusted_write(snapshot.as_ref(), state, "eject");
        let state = state.filter(|_| self.state.eject_save);
        self.state_writer.flush();
        match persist::eject(
            root,
            self.platform,
            self.core,
            stem,
            state.as_deref(),
            sav.as_deref(),
            &self.state,
        ) {
            Ok(()) => {
                self.state.cart = None;
                self.state.cart_platform = None;
                self.card_writes.ok("eject");
            }
            Err(e) => {
                eprintln!("slot: eject: {e}");
                self.card_writes.failed("eject", &e);
            }
        }
    }

    fn doze(&mut self) {
        if self.link_active() {
            self.end_link();
        }
        self.close_game_menu();
        self.end_power_hold();
        self.core_picker = None;
        if matches!(self.phase, Phase::Doze { .. }) {
            return;
        }
        self.state_writer.flush();
        self.flush_resume();
        let cart = match &mut self.phase {
            Phase::Playing { cart } | Phase::Polaroids { cart } => Some(std::mem::take(cart)),
            _ => None,
        };
        if std::mem::take(&mut self.states_from_menu) {
            self.in_game_menu = Some(QuickRow::States);
        }
        self.polaroids = None;
        self.phase = Phase::Doze { cart };
        self.dozed_at = self.now();
        self.radio.ask(RadioJob::Cool);
        if let Some(power) = &mut self.power {
            power.on_close();
        }
    }

    fn wake(&mut self) {
        let Phase::Doze { cart } = &mut self.phase else {
            return;
        };
        self.phase = match cart.take() {
            Some(cart) => {
                // Waking straight back into the game: no power-on animation, the screen is
                // simply on again.
                self.screen = 1.0;
                Phase::Playing { cart }
            }
            None => Phase::Shelf,
        };
        if let Some(power) = &mut self.power {
            power.on_open();
        }
    }

    pub fn on_doze_timeout(&mut self) {
        if !matches!(self.phase, Phase::Doze { .. }) {
            return;
        }
        self.begin_power_off();
    }

    /// POWER let go, or done with: the countdown bar goes.
    fn end_power_hold(&mut self) {
        if self.power_held_since.take().is_some() {
            self.hud.release_power();
        }
    }

    fn power_press(&mut self) {
        if std::mem::take(&mut self.woke_on_press) {
            return;
        }
        match self.phase {
            Phase::Doze { .. } => self.wake(),
            _ => self.doze(),
        }
    }

    fn open_core_picker(&mut self) {
        let Some(root) = self.root.clone() else {
            return;
        };
        let Some(cart) = self.shelf().carts.get(self.shelf().index) else {
            return;
        };
        if cart.platform != Platform::Gba {
            return;
        }
        let choices = [Core::Mgba, Core::Gpsp];
        let seat = slot_store::core_for_platform(&root, &cart.stem, cart.platform);
        let now = self.now();
        let mut picker = CorePicker::open_for(seat, choices, now);
        if self.core_faces_ready() {
            picker.start(now);
        }
        self.core_picker = Some(picker);
        self.shelf_mut().release_hold();
        self.play_held = None;
    }

    pub fn set_shell_picker_faces(&mut self, faces: slot_ui::ShellPickerFaces) {
        self.shell_picker_faces = Some(faces);
    }

    fn open_shell_picker(&mut self) {
        if self.shell_picker_faces.is_none() {
            return;
        }
        let Some(cart) = self.shelf().carts.get(self.shelf().index) else {
            return;
        };
        let no_label = cart.label.is_none()
            && self
                .root
                .as_ref()
                .is_some_and(|root| slot_store::has_no_label(root, cart.platform, &cart.stem));
        let picker = ShellPicker::open(
            cart.rom.clone(),
            cart.stem.clone(),
            cart.shell,
            cart.label.is_some(),
            no_label,
            cart.marker,
            cart.platform == Platform::Gba,
            self.now(),
        );
        let label_was = cart.label.clone();
        let picker = match cart.platform {
            Platform::Snes => {
                let auto = match slot_store::snes::cached_region(&cart.rom) {
                    slot_store::snes::Region::Ntsc => 0,
                    slot_store::snes::Region::Pal => 1,
                };
                picker.with_model(auto)
            }
            Platform::Gb => picker.with_model(0),
            Platform::Gbc => picker.with_model(1),
            Platform::Nes => picker.with_metallic(),
            _ => picker,
        };
        self.label_was = label_was;
        self.version_paths.clear();
        self.version_thumbs.clear();
        self.version_captions.clear();
        self.shell_picker = Some(picker);
        self.shelf_mut().release_hold();
        self.play_held = None;
    }

    fn shell_picker_input(&mut self, action: Action) {
        use shell_picker::{Outcome, Press};
        let press = match action {
            Action::GbaDown(Btn::Left) | Action::ShelfLeft => Press::Left,
            Action::GbaDown(Btn::Right) | Action::ShelfRight => Press::Right,
            Action::GbaDown(Btn::Up) => Press::Up,
            Action::GbaDown(Btn::Down) => Press::Down,
            Action::GbaDown(Btn::A) => Press::Keep,
            Action::GbaDown(Btn::Y) => Press::Remove,
            Action::GbaDown(Btn::B) | Action::GbaDown(Btn::Select) => Press::Back,
            _ => return,
        };
        let now = self.now();
        let Some(picker) = &mut self.shell_picker else {
            return;
        };
        let (rom, stem) = (picker.rom.clone(), picker.stem.clone());
        let was_marker = picker.was_marker();
        let marker = picker.marker_choice();
        let outcome = picker.press(press, now);
        let version = picker.version();
        match outcome {
            Outcome::None => {}
            Outcome::Preview(choice) => self.restyle(&rom, choice, marker),
            Outcome::Marker(ink) => {
                let choice = self.shell_picker.as_ref().and_then(|p| p.choice());
                self.restyle(&rom, choice, ink);
            }
            Outcome::Revert(choice, ink) => {
                self.restyle(&rom, choice, ink);
                if version.is_some() {
                    self.relabel(&rom, self.label_was.clone());
                }
            }
            Outcome::Save(choice, ink) => {
                self.restyle(&rom, choice, ink);
                if let Some(i) = version {
                    self.keep_version(&rom, i);
                }
                if let Some(root) = &self.root {
                    if let Err(e) = slot_store::write_shell_choice(root, &stem, choice) {
                        eprintln!("slot: shell: could not write {}: {e}", slot_store::LABELS_SHELL_FILE);
                    }
                    if ink != was_marker {
                        if let Err(e) = slot_store::write_marker(root, &stem, ink) {
                            eprintln!("slot: shell: could not write {}: {e}", slot_store::CART_MARKER_FILE);
                        }
                    }
                }
            }
            Outcome::RemoveLabel => self.remove_label(&rom),
            Outcome::Versions => self.ask_versions(&rom),
            Outcome::Search => self.open_label_search(&rom, &stem),
            Outcome::Version(i) => {
                if let Some(path) = self.version_paths.get(i).cloned() {
                    self.relabel(&rom, Some(path));
                }
            }
        }
    }

    /// Show every shelf's copy of a cart in another label, or none.
    fn relabel(&mut self, rom: &Path, label: Option<std::path::PathBuf>) {
        for (_, shelf) in &mut self.shelves {
            for index in 0..shelf.carts.len() {
                if shelf.carts[index].rom != rom || shelf.carts[index].label == label {
                    continue;
                }
                shelf.carts[index].label = label.clone();
                self.orphaned.extend(shelf.take_face(index));
                self.orphaned.extend(shelf.take_interior(index));
            }
        }
    }

    /// The version picked becomes the cart's label, in the file the old one was kept in.
    fn keep_version(&mut self, rom: &Path, i: usize) {
        let (Some(root), Some(path)) = (self.root.clone(), self.version_paths.get(i).cloned())
        else {
            return;
        };
        let Some(mut cart) = self.shelf().carts.iter().find(|c| c.rom == rom).cloned() else {
            return;
        };
        cart.label = self.label_was.clone();
        let saved = std::fs::read(&path)
            .map_err(slot_labels::Error::from)
            .and_then(|png| slot_labels::save_label(&root, &cart, &png));
        match saved {
            Ok(saved) => {
                slot_store::forget_no_label(&root, cart.platform, &cart.stem);
                slot_store::forget_label_removed(&root, cart.platform, &cart.stem);
                self.relabel(rom, Some(saved));
            }
            Err(e) => {
                eprintln!("slot: labels: {}: {e}", cart.stem);
                self.relabel(rom, self.label_was.clone());
            }
        }
    }

    /// The label tab's versions are looked up online, off the frame loop.
    fn ask_versions(&mut self, rom: &Path) {
        let cart = self.shelf().carts.iter().find(|c| c.rom == rom).cloned();
        match cart {
            Some(mut cart) if self.home_wifi_on() => {
                cart.label = self.label_was.clone();
                self.versions_ask = Some((cart, None));
            }
            _ => {
                self.hud.toast(Toast::TurnOnWifi, self.now());
                if let Some(picker) = &mut self.shell_picker {
                    picker.found(None);
                }
            }
        }
    }

    pub fn take_versions_ask(&mut self) -> Option<(Cart, Option<u64>)> {
        self.versions_ask.take()
    }

    /// Y on the studio's label tab: the keyboard opens with the cart's name typed.
    fn open_label_search(&mut self, rom: &Path, stem: &str) {
        let Some(platform) = self.shelf().carts.iter().find(|c| c.rom == rom).map(|c| c.platform)
        else {
            return;
        };
        if !self.home_wifi_on() {
            self.hud.toast(Toast::TurnOnWifi, self.now());
            return;
        }
        self.label_search = Some(LabelSearch::open(rom.to_path_buf(), platform, stem, self.now()));
    }

    fn label_search_input(&mut self, action: Action) {
        use label_search::{Press, Step};
        let press = match action {
            Action::GbaDown(Btn::Left) | Action::ShelfLeft => Press::Left,
            Action::GbaDown(Btn::Right) | Action::ShelfRight => Press::Right,
            Action::GbaDown(Btn::Up) => Press::Up,
            Action::GbaDown(Btn::Down) => Press::Down,
            Action::GbaDown(Btn::A) => Press::Choose,
            Action::GbaDown(Btn::B) => Press::Back,
            Action::GbaDown(Btn::X) => Press::Clear,
            Action::GbaDown(Btn::Y) => Press::Space,
            Action::GbaDown(Btn::Select) => Press::Shift,
            Action::GbaDown(Btn::Start) => Press::Search,
            _ => return,
        };
        let now = self.now();
        let Some(search) = &mut self.label_search else {
            return;
        };
        let (rom, platform) = (search.rom.clone(), search.platform);
        match search.press(press, now) {
            Step::None | Step::Close => {}
            Step::Search(asked, words) => self.search_ask = Some((asked, words, platform)),
            Step::Pick(game) => {
                let cart = self.shelf().carts.iter().find(|c| c.rom == rom).cloned();
                if let (Some(mut cart), Some(picker)) = (cart, &mut self.shell_picker) {
                    picker.searched();
                    cart.label = self.label_was.clone();
                    self.versions_ask = Some((cart, Some(game.id)));
                    // The versions are kept in the files the last ones were, so the cart
                    // wears its own label until they come, to be drawn again with them.
                    self.relabel(&rom, self.label_was.clone());
                }
            }
        }
    }

    pub fn take_search_ask(&mut self) -> Option<(u64, String, Platform)> {
        self.search_ask.take()
    }

    /// The games a search found, or None when it could not be made.
    pub fn search_found(&mut self, asked: u64, games: Option<Vec<slot_labels::Game>>) {
        if let Some(search) = &mut self.label_search {
            if !search.found(asked, games) {
                self.hud.toast(Toast::LabelFailed, self.now());
            }
        }
    }

    pub fn label_search_open(&self) -> bool {
        self.label_search.is_some()
    }

    pub fn has_search_faces(&self) -> bool {
        self.search_faces.is_some()
    }

    pub fn set_search_faces(&mut self, faces: slot_ui::KeyboardFaces<slot_ui::Up>) {
        self.search_faces = Some(faces);
    }

    /// The search's words, and the rev they belong to, when its faces were made for another.
    pub fn search_words(&self) -> Option<(u64, SearchWords)> {
        let search = self.label_search.as_ref()?;
        if self.search_texts.as_ref().is_some_and(|(rev, _)| *rev == search.rev()) {
            return None;
        }
        let games = search.games();
        let status = if search.found_none() {
            Some("Nothing Found".to_owned())
        } else if games.len() == 1 {
            Some("1 Game Found".to_owned())
        } else if !games.is_empty() {
            Some(format!("{} Games Found", games.len()))
        } else {
            None
        };
        Some((
            search.rev(),
            SearchWords {
                heading: format!("Search for a label · {}", search.platform.name()),
                typed: Some(search.text().to_owned()).filter(|t| !t.is_empty()),
                status,
                rows: games.iter().map(|g| (g.name.clone(), g.platform.clone())).collect(),
            },
        ))
    }

    pub fn set_search_texts(&mut self, rev: u64, texts: slot_ui::SearchTexts<slot_ui::Up>) {
        self.search_texts = Some((rev, texts));
    }

    fn draw_label_search(&self, search: &LabelSearch, out: &mut Vec<Draw>) {
        let (Some(faces), Some((_, texts))) = (&self.search_faces, &self.search_texts) else {
            return;
        };
        let light = self.light_ground();
        let now = self.now();
        let view = slot_ui::SearchView {
            faces,
            texts,
            ink: slot_ui::theme::ink(light),
            ground: slot_ui::theme::menu_flat(light),
            light,
            screen: search.screen(now),
            openness: search.openness(now),
        };
        slot_ui::draw_label_search(self.canvas(), &view, out);
    }

    /// The versions the label tab asked for came back, each with its thumbnail and caption
    /// uploaded; None when they could not be looked for. The first is shown at once.
    #[allow(clippy::type_complexity)]
    pub fn versions_found(
        &mut self,
        rom: &Path,
        found: Option<Vec<(std::path::PathBuf, (TexId, u32, u32), (TexId, u32))>>,
    ) {
        let Some(picker) = self.shell_picker.as_mut().filter(|p| p.rom == rom) else {
            return;
        };
        let count = found.as_ref().map(Vec::len);
        let shown = picker.found(count);
        match count {
            Some(0) => self.hud.toast(Toast::NoLabelFound, self.now()),
            None => self.hud.toast(Toast::LabelFailed, self.now()),
            _ => {}
        }
        let found = found.unwrap_or_default();
        self.version_paths = found.iter().map(|v| v.0.clone()).collect();
        self.version_thumbs = found.iter().map(|v| v.1).collect();
        self.version_captions = found.iter().map(|v| v.2).collect();
        if let Some(path) = shown.and_then(|i| self.version_paths.get(i).cloned()) {
            self.relabel(rom, Some(path));
        }
    }

    /// Draw every shelf's copy of a cart again in another shell or marker.
    fn restyle(
        &mut self,
        rom: &Path,
        choice: Option<slot_store::ShellChoice>,
        marker: Option<[u8; 3]>,
    ) {
        for (_, shelf) in &mut self.shelves {
            for index in 0..shelf.carts.len() {
                let cart = &mut shelf.carts[index];
                if cart.rom != rom || (cart.shell == choice && cart.marker == marker) {
                    continue;
                }
                cart.shell = choice;
                cart.marker = marker;
                shelf.restyled(index);
                self.orphaned.extend(shelf.take_face(index));
                self.orphaned.extend(shelf.take_interior(index));
            }
        }
    }

    /// Y, then A, in the studio: the cart's label file goes, its title is written in marker,
    /// and no label is fetched for it again until one is picked from the label's tab.
    fn remove_label(&mut self, rom: &Path) {
        let (Some(root), Some(path)) = (self.root.clone(), self.label_was.clone()) else {
            return;
        };
        let Some(cart) = self.shelf().carts.iter().find(|c| c.rom == rom).cloned() else {
            return;
        };
        if let Err(e) = std::fs::remove_file(&path) {
            eprintln!("slot: labels: could not remove {}: {e}", path.display());
            return;
        }
        slot_store::note_label_removed(&root, cart.platform, &cart.stem);
        self.label_was = None;
        self.relabel(rom, None);
    }

    pub fn take_label_ask(&mut self) -> Option<Cart> {
        self.label_ask.take()
    }

    /// The fetch the shell picker asked for came back: Some(found) or None when it failed
    /// for want of a connection.
    pub fn label_fetched(&mut self, rom: &Path, found: Option<bool>) {
        let toast = match found {
            Some(true) => Toast::LabelDownloaded,
            Some(false) => Toast::NoLabelFound,
            None => Toast::LabelFailed,
        };
        self.hud.toast(toast, self.now());
        if let Some(picker) = &mut self.shell_picker {
            if picker.rom == rom {
                picker.fetched(found);
            }
        }
    }

    fn draw_shell_picker(&self, picker: &ShellPicker, out: &mut Vec<Draw>) {
        let Some(faces) = &self.shell_picker_faces else {
            return;
        };
        let Some((cart, face)) = self.shelf().find(&picker.stem) else {
            return;
        };
        let (w, h) = slot_ui::cart_box(slot_ui::mould_of(cart));
        let (rest, scale) = self.shelf().selected_at(self.canvas());
        let from = slot_ui::Placed {
            x: rest,
            y: slot_ui::foot_y(h as f32) - h as f32 * scale,
            w: w as f32 * scale,
            h: h as f32 * scale,
        };
        let openness = picker.openness(self.now());
        // Its shadow stays on the shelf as it rises, and goes as it lifts away.
        self.shelf().draw_drop(
            &picker.stem,
            from,
            scale,
            slot_ui::shell_picker_hop(openness),
            1.0 - slot_ui::lift_of(openness),
            out,
        );
        let swatches: Vec<(Option<[u8; 3]>, usize)> = picker
            .swatches()
            .iter()
            .map(|s| (s.colour(), s.name_index()))
            .collect();
        let light = self.light_ground();
        let view = slot_ui::ShellPickerView {
            faces,
            ink: slot_ui::theme::ink(light),
            ground: slot_ui::theme::menu_flat(light),
            cart: face,
            behind: self.shelf().behind(&picker.stem),
            gba: cart.platform == Platform::Gba,
            cart_box: (w, h),
            from,
            openness,
            swatches: &swatches,
            swatch: picker.swatch(),
            finish: picker.finish(),
            finishes: picker.finish_faces(),
            finish_on: picker.finish_on(),
            model: picker
                .model()
                .map(|m| (if cart.platform == Platform::Snes { 0 } else { 2 }, m)),
            row: match picker.row() {
                shell_picker::Row::Tabs => slot_ui::PickerRow::Tabs,
                shell_picker::Row::Colour => slot_ui::PickerRow::Colour,
                shell_picker::Row::Finish => slot_ui::PickerRow::Finish,
                shell_picker::Row::Model => slot_ui::PickerRow::Model,
                shell_picker::Row::Label => slot_ui::PickerRow::Label,
                shell_picker::Row::Marker => slot_ui::PickerRow::Marker,
            },
            tabs: picker.tabs().into_iter().map(tab_face).collect(),
            tab: tab_face(picker.tab()),
            label: match picker.versions() {
                shell_picker::Versions::Unasked | shell_picker::Versions::Looking => {
                    slot_ui::LabelTab::Looking
                }
                shell_picker::Versions::Missing => slot_ui::LabelTab::Missing,
                shell_picker::Versions::Found(_) => slot_ui::LabelTab::Found {
                    thumbs: &self.version_thumbs,
                    captions: &self.version_captions,
                    at: picker.version(),
                },
            },
            marker: picker.marker(),
            inks: shell_picker::MARKERS.map(|(_, ink)| ink),
            remove: picker.offers_remove(),
            search: picker.offers_search(),
            confirming: picker.confirming(),
        };
        slot_ui::draw_shell_picker(self.canvas(), &view, out);
    }

    fn core_faces_ready(&self) -> bool {
        self.core_faces_stem
            .as_deref()
            .is_some_and(|stem| self.selected_stem() == Some(stem))
    }

    fn core_picker_input(&mut self, action: Action) {
        let press = match action {
            Action::GbaDown(Btn::Left) | Action::ShelfLeft => Press::Left,
            Action::GbaDown(Btn::Right) | Action::ShelfRight => Press::Right,
            Action::GbaDown(Btn::A) => Press::Keep,
            Action::GbaDown(Btn::B) => Press::Back,
            _ => return,
        };
        let now = self.now();
        let Some(picker) = &mut self.core_picker else {
            return;
        };
        let outcome = picker.press(press, now);
        if let Outcome::Write(core) = outcome {
            self.write_core(core);
        }
    }

    fn open_game_menu(&mut self) {
        if self.game_menu.is_some() {
            return;
        }
        if self.link_active() {
            return self.refuse();
        }
        if self.core == Core::Mgba {
            let wireless = self
                .seated()
                .is_some_and(|stem| self.link_mode(stem).0 == LinkKind::Wireless);
            if wireless {
                self.hud.toast(Toast::NeedsGpsp, self.now());
                return;
            }
            self.link_hardware = LinkKind::Cable;
            self.radio.ask(RadioJob::Warm);
            self.game_menu = Some(GameMenu::Pick(self.last_role));
            return;
        }
        if self.platform != Platform::Gba {
            self.hud.toast(Toast::NoLink, self.now());
            return;
        }
        let carried = self
            .seated()
            .and_then(|stem| self.auto_link(stem))
            .is_some_and(|(cart, _)| link_carried(&cart.code, &cart.title));
        if !carried {
            self.hud.toast(Toast::NoLink, self.now());
            return;
        }
        if self.core != Core::Gpsp {
            self.hud.toast(Toast::NeedsGpsp, self.now());
            return;
        }
        let hardware = self
            .seated()
            .map_or(LinkKind::Cable, |stem| self.link_mode(stem).0);
        self.link_hardware = hardware;
        self.radio.ask(RadioJob::Warm);
        self.game_menu = Some(GameMenu::Pick(self.last_role));
    }

    fn game_menu_shortcut(&mut self) {
        let Some(client_id) = self.link_client_id() else {
            return self.open_game_menu();
        };
        let now = self.now();
        self.game_menu = Some(GameMenu::Linked {
            role: LinkRow::from_client_id(client_id),
            worked: now,
            since: now,
            opened: true,
        });
    }

    fn end_link_from_menu(&mut self) {
        let role = self
            .link_client_id()
            .map_or(self.last_role, LinkRow::from_client_id);
        self.end_link();
        self.hud.toast(Toast::LinkEnded, self.now());
        self.unplug(role);
    }

    fn unplug(&mut self, role: LinkRow) {
        self.game_menu = Some(GameMenu::Unplug {
            role,
            since: self.now(),
        });
    }

    pub fn set_radio_jobs(&mut self, jobs: Box<dyn RadioJobs>) {
        self.radio = jobs;
    }

    fn game_menu_input(&mut self, action: Action) {
        let Some(menu) = self.game_menu else {
            return;
        };
        match menu {
            GameMenu::Pick(role) => match action {
                Action::GbaDown(Btn::Left) | Action::GbaDown(Btn::Right) => {
                    self.last_role = role.other();
                    self.game_menu = Some(GameMenu::Pick(role.other()));
                }
                Action::GbaDown(Btn::Select) => self.switch_hardware(),
                Action::GbaDown(Btn::A) => self.pick_link(role),
                Action::GbaDown(Btn::B) | Action::GameMenu => self.close_game_menu(),
                _ => {}
            },
            GameMenu::Working { .. } => {
                if action == Action::GbaDown(Btn::B) {
                    if let Some(starting) = &mut self.starting {
                        starting.starter.cancel();
                    }
                    if let Some(reload) = &mut self.reload {
                        reload.cancelled = true;
                    }
                }
            }
            GameMenu::Linked { opened: true, .. } => match action {
                Action::GbaDown(Btn::A) => self.end_link_from_menu(),
                Action::GbaDown(Btn::B) | Action::GameMenu => self.close_game_menu(),
                _ => {}
            },
            GameMenu::Linked { .. } => {}
            GameMenu::Failed { .. } => {
                if matches!(
                    action,
                    Action::GbaDown(Btn::A) | Action::GbaDown(Btn::B) | Action::GameMenu
                ) {
                    self.close_game_menu();
                }
            }
            GameMenu::Unplug { .. } => {}
        }
    }

    pub fn start_link(&mut self, starter: LinkStarter, client_id: u16) {
        self.start_link_from(starter, client_id, self.now());
    }

    fn start_link_from(&mut self, starter: LinkStarter, client_id: u16, since: Millis) {
        if let Some(mut old) = self.starting.replace(LinkStarting { starter, client_id }) {
            old.starter.cancel();
        }
        self.game_menu = Some(GameMenu::Working {
            role: LinkRow::from_client_id(client_id),
            step: LinkStep::Radio,
            since,
        });
    }

    fn switch_hardware(&mut self) {
        let Some(stem) = self.seated().map(str::to_string) else {
            return;
        };
        if !self.link_switchable(&stem) {
            return self.refuse();
        }
        let other = self.link_hardware.other();
        self.link_hardware = other;
        self.link_choices.insert(stem, other);
    }

    fn link_switchable(&self, stem: &str) -> bool {
        self.auto_link(stem).is_some_and(|(cart, auto)| {
            serial_option(self.link_hardware.other(), auto, &cart.code, &cart.title)
                != serial_option(self.link_hardware, auto, &cart.code, &cart.title)
        })
    }

    /// The real starter for this cart, which names its game to the other handheld by header code.
    fn spawn_link(&self, role: LinkRow) -> LinkStarter {
        let game = self.seated_cart().map_or("", |c| c.code.as_str());
        LinkStarter::spawn(role.role(), link_port(), game)
    }

    fn pick_link(&mut self, role: LinkRow) {
        let Some(stem) = self.seated().map(str::to_string) else {
            return;
        };
        let loaded = self.link_loaded.unwrap_or("auto");
        if self.core == Core::Mgba {
            let player = role.client_id() as u8;
            if self.link_player == Some(player) {
                return self.start_link(
                    self.spawn_link(role),
                    role.client_id(),
                );
            }
            if self.snapshot.as_ref().is_some_and(|s| !s.resume_trusted()) {
                return self.refuse();
            }
            self.link_player = Some(player);
            self.link_reload = Some((stem.clone(), loaded));
            self.reload = Some(Reload {
                stem,
                role,
                cancelled: false,
                from: self.link_hardware,
                from_serial: loaded,
                fallback: false,
            });
            self.game_menu = Some(GameMenu::Working {
                role,
                step: LinkStep::Radio,
                since: self.now(),
            });
            return;
        }
        let (_, serial) = self.link_mode(&stem);
        if serial == loaded {
            return self.start_link(
                self.spawn_link(role),
                role.client_id(),
            );
        }
        if self.snapshot.as_ref().is_some_and(|s| !s.resume_trusted()) {
            return self.refuse();
        }
        self.link_reload = Some((stem.clone(), serial));
        self.reload = Some(Reload {
            stem,
            role,
            cancelled: false,
            from: self.link_hardware.other(),
            from_serial: loaded,
            fallback: false,
        });
        self.game_menu = Some(GameMenu::Working {
            role,
            step: LinkStep::Radio,
            since: self.now(),
        });
    }

    fn refuse_seated(&mut self) {
        self.close_game_menu();
        let caught = self.seat();
        let cart = match &mut self.phase {
            Phase::Playing { cart } => Some(std::mem::take(cart)),
            Phase::Doze { cart } => {
                *cart = None;
                None
            }
            _ => None,
        };
        if let Some(cart) = cart {
            self.refuse_out(cart, caught);
        }
    }

    fn close_game_menu(&mut self) {
        self.game_menu = None;
        if let Some(mut starting) = self.starting.take() {
            starting.starter.cancel();
        }
        if !self.link_active() {
            self.radio.ask(RadioJob::Cool);
        }
        let uncollected =
            self.link_reload.is_some() && self.reload.as_ref().is_some_and(|r| !r.fallback);
        if uncollected {
            self.link_reload = None;
            self.reload = None;
        } else if let Some(reload) = &mut self.reload {
            reload.cancelled = true;
        }
    }

    fn working_role(&self, client_id: u16) -> (LinkRow, Millis) {
        match self.game_menu {
            Some(GameMenu::Working { role, since, .. }) => (role, since),
            _ => (LinkRow::from_client_id(client_id), self.now()),
        }
    }

    fn poll_link(&mut self) {
        let Some(mut starting) = self.starting.take() else {
            return;
        };
        match starting.starter.poll() {
            None => self.starting = Some(starting),
            Some(LinkProgress::At(step)) => {
                if let Some(GameMenu::Working { role, since, .. }) = self.game_menu {
                    self.game_menu = Some(GameMenu::Working { role, step, since });
                }
                self.starting = Some(starting);
            }
            Some(LinkProgress::Ready(link)) => {
                let (role, worked) = self.working_role(starting.client_id);
                self.game_menu = Some(GameMenu::Linked {
                    role,
                    worked,
                    since: self.now(),
                    opened: false,
                });
                self.begin_link(starting.client_id);
                self.link_transport = Some((starting.client_id, Box::new(link)));
            }
            Some(LinkProgress::Failed(LinkFail::Cancelled)) => self.game_menu = None,
            Some(LinkProgress::Failed(fail)) => {
                let (role, worked) = self.working_role(starting.client_id);
                self.game_menu = Some(GameMenu::Failed {
                    role,
                    fail,
                    worked,
                    since: self.now(),
                });
            }
        }
    }

    fn write_core(&self, core: Core) {
        let (Some(root), Some(cart)) = (
            self.root.clone(),
            self.shelf().carts.get(self.shelf().index),
        ) else {
            return;
        };
        if let Err(e) = slot_store::write_selected_core(&root, &cart.stem, core) {
            eprintln!("slot: core: could not write selected_core.txt: {e}");
        }
    }

    /// VOL+ and POWER: keep the next frame as it shows, HUD aside. A volume step the
    /// chord began with is taken back.
    fn screenshot(&mut self) {
        if matches!(self.phase, Phase::Doze { .. })
            || self.shutting_down()
            || self.capturing
            || self.root().is_none()
        {
            return;
        }
        let now = self.now();
        if let Some(&(volume, muted, at)) = self.vol_before.last() {
            if now.saturating_sub(at) <= SHOT_CHORD_MS {
                *self.level_mut() = volume;
                *self.muted_mut() = muted;
                self.vol_before.clear();
                self.persist_soon();
            }
        }
        if self.hud.kind == HudKind::Volume {
            self.hud.shown_at = None;
        }
        self.shot_pending = true;
    }

    /// Two shots in one second keep apart: "… 2.png", "… 3.png".
    fn free_shot_path(&self, path: PathBuf) -> PathBuf {
        let taken = |p: &PathBuf| p.exists() || self.last_shot.as_ref() == Some(p);
        if !taken(&path) {
            return path;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("slot").to_string();
        (2..)
            .map(|n| path.with_file_name(format!("{stem} {n}.png")))
            .find(|p| !taken(p))
            .unwrap_or(path)
    }

    pub fn shot_pending(&self) -> bool {
        self.shot_pending
    }

    /// The frame the screenshot asked for, read back: written to the card, then the flash.
    pub fn save_screenshot(&mut self, rgba: Vec<u8>, w: u32, h: u32) {
        self.shot_pending = false;
        let Some(root) = self.root().map(Path::to_path_buf) else {
            return;
        };
        let stamp = slot_store::format_stamp(self.wall_secs());
        let path = match (matches!(self.phase, Phase::Playing { .. }), self.cart_in_slot()) {
            (true, Some((platform, stem))) => root
                .join("Screenshots")
                .join(platform.dir_name())
                .join(format!("{stem} {stamp}.png")),
            _ => root.join("Screenshots").join(format!("slot {stamp}.png")),
        };
        let path = self.free_shot_path(path);
        self.last_shot = Some(path.clone());
        crate::screenshot::save(path, rgba, w, h);
        let now = self.now();
        self.shot_flash = Some(now);
        self.hud.toast(Toast::ScreenshotSaved, now);
    }

    /// The Screenshots page: this game's from the in-game menu, every one from the main menu.
    fn open_shots(&mut self, in_game: bool) {
        let Some(root) = self.root().map(Path::to_path_buf) else {
            return;
        };
        let list = match (in_game, self.cart_in_slot()) {
            (true, Some((platform, stem))) => crate::shots::for_game(&root, platform, &stem),
            (true, None) => return,
            (false, _) => crate::shots::all(&root),
        };
        // Nothing taken yet: shake no, like States with nothing saved, and stay in the menu.
        if list.is_empty() {
            return self.refuse();
        }
        self.shot_pics.clear();
        self.shots = Some(crate::shots::ShotView::new(list, in_game, self.now()));
    }

    /// Close the page when what it was opened from has gone (the game ended, the menu closed).
    fn keep_shots(&mut self, dt: f32) {
        let Some(view) = &mut self.shots else {
            return;
        };
        view.ease(dt);
        let home = match view.in_game {
            true => matches!(self.phase, Phase::Playing { .. }) && self.in_game_menu.is_some(),
            false => matches!(self.phase, Phase::QuickMenu { .. }),
        };
        if !home {
            self.shots = None;
            self.shot_pics.clear();
        }
    }

    fn shots_input(&mut self, action: Action) {
        let now = self.now();
        let Some(view) = &mut self.shots else {
            return;
        };
        if view.full {
            view.hints_at = now;
        }
        if view.confirm {
            match action {
                Action::GbaDown(Btn::A) | Action::Insert => {
                    view.confirm = false;
                    view.delete_selected();
                }
                Action::GbaDown(Btn::B) | Action::QuickMenu => view.confirm = false,
                _ => {}
            }
            return;
        }
        match action {
            Action::GbaDown(Btn::Left) | Action::ShelfLeft => {
                view.step(false);
            }
            Action::GbaDown(Btn::Right) | Action::ShelfRight => {
                view.step(true);
            }
            Action::GbaDown(Btn::A) | Action::Insert if !view.shots.is_empty() => {
                view.full = !view.full;
            }
            Action::GbaDown(Btn::Y) if !view.shots.is_empty() => view.confirm = true,
            Action::GbaDown(Btn::B) if view.full => view.full = false,
            Action::GbaDown(Btn::B) => {
                self.shots = None;
                self.shot_pics.clear();
            }
            Action::QuickMenu => {
                let in_game = view.in_game;
                self.shots = None;
                self.shot_pics.clear();
                if in_game {
                    self.in_game_menu = None;
                }
            }
            _ => {}
        }
    }

    fn draw_shots(&self, view: &crate::shots::ShotView, out: &mut Vec<Draw>) {
        let light = self.about_light();
        let full_hints = {
            let since = self.now().saturating_sub(view.hints_at);
            match since {
                s if s < SHOT_HINTS_MS => 1.0,
                s if s < SHOT_HINTS_MS + 300 => 1.0 - (s - SHOT_HINTS_MS) as f32 / 300.0,
                _ => 0.0,
            }
        };
        slot_ui::ShotPage {
            count: view.shots.len(),
            sel: view.sel,
            pos: view.pos,
            pics: &self.shot_pics,
            full: view.full,
            hints: full_hints,
            confirm: view.confirm,
            faces: self.shot_faces.as_ref(),
            ink: slot_ui::theme::ink(light),
            ground: slot_ui::theme::menu_flat(light),
            canvas: self.canvas(),
        }
        .draw(out);
        if !view.full || view.shots.is_empty() {
            slot_ui::draw_corners(
                self.canvas(),
                self.battery,
                self.battery_percent,
                self.hud_bolt,
                self.shelf_clock,
                self.wifi_shown(self.hud_wifi),
                out,
            );
        }
    }

    pub fn shot_view(&self) -> Option<&crate::shots::ShotView> {
        self.shots.as_ref()
    }

    pub fn set_shot_faces(&mut self, faces: slot_ui::ShotFaces) {
        self.shot_faces = Some(faces);
    }

    pub fn shot_faces_mut(&mut self) -> Option<&mut slot_ui::ShotFaces> {
        self.shot_faces.as_mut()
    }

    pub fn set_shot_pics(&mut self, pics: Vec<(usize, TexId)>) {
        self.shot_pics = pics;
    }

    /// START+SELECT+L+R: the cart starts over, as from power on.
    fn reset_game(&mut self) {
        if !(matches!(self.phase, Phase::Playing { .. }) && self.game_ready) || self.link_active() {
            return;
        }
        self.reset_pending = true;
        self.in_game_menu = None;
        self.cheat_page = None;
        self.hud.toast(Toast::GameReset, self.now());
    }

    pub fn take_reset(&mut self) -> bool {
        std::mem::take(&mut self.reset_pending)
    }

    /// Shutting down with a game playing and Resume Game on: keep its picture for the boot
    /// screen, as the device will start back into that game.
    fn begin_capture(&mut self) {
        let playing = matches!(self.phase, Phase::Playing { .. }) && self.game_ready;
        self.capturing = playing
            && self.state.resume_game
            && !self.link_active();
    }

    pub fn capturing(&self) -> bool {
        self.capturing
    }

    pub fn set_boot_picture(&mut self, rgba: Vec<u8>, w: u32, h: u32) {
        self.capturing = false;
        self.boot_picture = Some((rgba, w, h));
    }

    pub fn take_boot_picture(&mut self) -> Option<(Vec<u8>, u32, u32)> {
        self.boot_picture.take()
    }

    pub fn picture_wanted(&self) -> bool {
        self.picture_wanted && self.resume_picture.is_none()
    }

    pub fn set_resume_picture(&mut self, tex: Option<TexId>) {
        self.picture_wanted = false;
        self.resume_picture = tex;
    }

    pub fn resume_picture_done(&self) -> bool {
        self.resume_picture.is_some()
            && matches!(self.phase, Phase::Playing { .. })
            && self.game_ready
    }

    pub fn clear_resume_picture(&mut self) -> Option<TexId> {
        self.resume_picture.take()
    }

    fn begin_power_off(&mut self) {
        if self.powering_off {
            return;
        }
        if self.link_active() {
            self.end_link();
        }
        self.begin_capture();
        self.close_game_menu();
        self.remember_shelf();
        self.powering_off = true;
        self.act_at = self.now() + SHUTDOWN_SHOW_MS;
        self.set_led(LedState::Off);
    }

    pub fn on_battery(&mut self, b: Battery) {
        if b.percent > BATTERY_CRITICAL || self.powering_off {
            return;
        }
        if matches!(b.charge, Charge::Charging | Charge::Full) {
            return;
        }
        self.flush_resume();
        self.begin_power_off();
    }

    fn doze_expired(&self) -> bool {
        if self.link_active() {
            return false;
        }
        let (Phase::Doze { .. }, Some(power)) = (&self.phase, &self.power) else {
            return false;
        };
        self.now().saturating_sub(self.dozed_at) >= power.timeout().as_millis() as Millis
    }

    /// Something is going on that is not a press: Auto Sleep starts counting again.
    pub fn keep_awake(&mut self) {
        self.last_activity = self.now();
    }

    fn auto_sleep_expired(&self) -> bool {
        let timeout = Millis::from(self.state.auto_sleep_secs) * 1_000;
        timeout > 0
            && !self.powering_off
            && !self.link_active()
            && !matches!(self.phase, Phase::Doze { .. })
            && self.now().saturating_sub(self.last_activity) >= timeout
    }

    pub fn settle_saves(&mut self) {
        self.state_writer.flush();
        if let Some(h) = self.pending_save.take() {
            let _ = h.join();
        }
    }

    fn autosave(&mut self) {
        self.autosave_at = self.now() + AUTOSAVE_MS;
        self.settle_saves();
        let (Some(root), Some(snapshot), Some(cart)) = (&self.root, &self.snapshot, self.seated())
        else {
            return;
        };
        let Some(state) = snapshot.state() else {
            eprintln!("slot: autosave: the core gave up no state");
            return;
        };
        let (state, sav) = trusted_write(snapshot.as_ref(), state, "autosave");
        let (root, platform, core, cart) =
            (root.clone(), self.platform, self.core, cart.to_owned());
        let failing = self.card_writes.clone();
        let write = move || {
            let written = persist::flush(
                &root,
                platform,
                core,
                &cart,
                state.as_deref(),
                sav.as_deref(),
            );
            if let Err(e) = &written {
                eprintln!("slot: autosave: {e}");
            }
            failing.wrote("autosave", &written);
        };
        match std::thread::Builder::new()
            .name("slot-autosave".into())
            .spawn(write)
        {
            Ok(h) => self.pending_save = Some(h),
            Err(e) => eprintln!("slot: autosave: no writer thread: {e}"),
        }
    }

    pub fn flush_resume(&mut self) {
        self.settle_saves();
        self.autosave_at = self.now() + AUTOSAVE_MS;
        let (Some(root), Some(snapshot), Some(cart)) = (&self.root, &self.snapshot, self.seated())
        else {
            return;
        };
        let Some(state) = snapshot.state() else {
            eprintln!("slot: flush: the core gave up no state");
            return;
        };
        let (state, sav) = trusted_write(snapshot.as_ref(), state, "flush");
        let written = persist::flush(
            root,
            self.platform,
            self.core,
            cart,
            state.as_deref(),
            sav.as_deref(),
        );
        if let Err(e) = &written {
            eprintln!("slot: flush: {e}");
        }
        self.card_writes.wrote("flush", &written);
    }

    fn ring(&self) -> Option<StateRing> {
        let (Some(root), Some(cart)) = (&self.root, self.seated()) else {
            return None;
        };
        Some(StateRing::new(root, self.platform, self.core, cart))
    }

    fn seated(&self) -> Option<&str> {
        match &self.phase {
            Phase::Playing { cart } | Phase::Polaroids { cart } => Some(cart),
            _ => None,
        }
    }

    fn entries(&self) -> Vec<StateEntry> {
        self.ring().and_then(|r| r.list().ok()).unwrap_or_default()
    }

    fn open_polaroids(&mut self) {
        if self.link_active() {
            return self.refuse();
        }
        let entries = self.entries();
        if entries.is_empty() {
            return self.refuse();
        }
        let Phase::Playing { cart } = &mut self.phase else {
            return;
        };
        let cart = std::mem::take(cart);
        self.polaroids = Some(Polaroids::new(entries));
        self.phase = Phase::Polaroids { cart };
        self.push_hint_faces();
    }

    fn close_polaroids(&mut self) {
        let Phase::Polaroids { cart } = &mut self.phase else {
            return;
        };
        let cart = std::mem::take(cart);
        self.polaroids = None;
        self.phase = Phase::Playing { cart };
        if std::mem::take(&mut self.states_from_menu) {
            self.in_game_menu = Some(QuickRow::States);
        }
    }

    fn load_selected(&mut self) {
        let state = self
            .polaroids
            .as_ref()
            .and_then(|p| p.selected())
            .map(|e| e.state.clone());
        let refused = state.map(|state| self.load_file(&state)) == Some(false);
        if !refused {
            self.states_from_menu = false;
            self.close_polaroids();
        }
    }

    /// Y on a state asks first: A deletes it, B keeps it.
    fn ask_state_delete(&mut self) {
        let faces = self.state_confirm;
        if let (Some(p), Some(faces)) = (&mut self.polaroids, faces) {
            if !p.is_empty() {
                p.confirm = Some(faces);
            }
        }
    }

    fn end_state_confirm(&mut self) {
        if let Some(p) = &mut self.polaroids {
            p.confirm = None;
        }
    }

    pub fn set_state_confirm_faces(&mut self, faces: slot_ui::ConfirmFaces) {
        self.state_confirm = Some(faces);
    }

    fn delete_selected(&mut self) {
        let stamp = self
            .polaroids
            .as_ref()
            .and_then(|p| p.selected())
            .map(|e| e.stamp.clone());
        let (Some(stamp), Some(ring)) = (stamp, self.ring()) else {
            return;
        };
        if let Err(e) = ring.remove(&stamp) {
            eprintln!("slot: delete: {e}");
            return;
        }
        let Some(p) = &mut self.polaroids else {
            return;
        };
        p.remove_selected();
        if p.is_empty() {
            self.close_polaroids();
        }
    }

    fn load_newest(&mut self) {
        let Some(newest) = self.entries().first().map(|e| e.state.clone()) else {
            return self.refuse();
        };
        self.load_file(&newest);
    }

    fn load_file(&mut self, state: &Path) -> bool {
        if !self.may_load_state() {
            self.refuse();
            return false;
        }
        let Some(snapshot) = &self.snapshot else {
            return false;
        };
        let bytes = match std::fs::read(state) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("slot: load: {e}");
                return false;
            }
        };
        snapshot.load(bytes);
        self.hud.toast(Toast::StateLoaded, self.now());
        true
    }

    fn save_state(&mut self) {
        let (Some(ring), Some(snapshot)) = (self.ring(), &self.snapshot) else {
            return;
        };
        if !snapshot.resume_trusted() {
            eprintln!("slot: save: the core refused the resume it was given, not pushing a state");
            return self.refuse();
        }
        let Some(state) = snapshot.state() else {
            eprintln!("slot: save: the core gave up no state");
            return;
        };
        let thumb = snapshot.thumb().unwrap_or_default();
        let stamp = free_stamp(&ring, self.wall_secs());
        let pushed = ring.push(&state, &thumb, &stamp);
        self.card_writes.wrote("save state", &pushed);
        if let Err(e) = pushed {
            eprintln!("slot: save: {e}");
            return;
        }
        self.hud.toast(Toast::StateSaved, self.now());
    }

    pub fn set_legend_faces(&mut self, faces: Vec<TexId>) {
        self.legend_faces = faces;
        self.push_hint_faces();
    }

    fn push_hint_faces(&mut self) {
        let faces = self.legend_faces.clone();
        if let Some(p) = &mut self.polaroids {
            p.set_hint_faces(faces);
        }
    }

    pub fn set_icon_faces(&mut self, faces: Vec<TexId>) {
        self.hud.set_icons(faces);
    }

    pub fn set_toast_faces(&mut self, faces: Vec<TexId>) {
        self.hud.set_toasts(faces);
    }

    pub fn toast(&self) -> Option<Toast> {
        self.hud.said(self.now())
    }

    /// New labels are cached in the background while no game is running.
    pub fn labels_may_cache(&self) -> bool {
        matches!(
            self.phase,
            Phase::SetClock { .. }
                | Phase::Shelf
                | Phase::QuickMenu { .. }
                | Phase::About
                | Phase::Buttons { .. }
        )
    }

    /// The caching banner takes the top bar on the shelf while nothing else is in it.
    pub fn labels_banner_shown(&self) -> bool {
        let now = self.now();
        self.on_shelf()
            && self.core_picker.is_none()
            && self.shell_picker.is_none()
            && !self.hud.visible(now)
            && self.hud.said(now).is_none()
    }

    pub fn say_labels_ready(&mut self) {
        self.hud.toast(Toast::LabelsReady, self.now());
    }

    pub fn card_alarm(&self) -> bool {
        self.hud.alarmed()
    }

    pub fn polaroid_entries(&self) -> &[StateEntry] {
        match &self.polaroids {
            Some(p) => &p.entries,
            None => &[],
        }
    }

    pub fn set_polaroid_faces(&mut self, faces: Vec<TexId>) {
        if let Some(p) = &mut self.polaroids {
            p.set_faces(faces);
        }
    }

    pub fn polaroid_stamp(&self) -> Option<&str> {
        self.polaroids
            .as_ref()
            .and_then(|p| p.selected())
            .map(|e| e.stamp.as_str())
    }

    pub fn polaroid_title(&self, now: &str) -> String {
        self.polaroids
            .as_ref()
            .map_or_else(String::new, |p| p.title(now))
    }

    pub fn set_polaroid_title_face(&mut self, face: TexId) {
        if let Some(p) = &mut self.polaroids {
            p.set_title_face(Some(face));
        }
    }
}

fn trusted_write(
    snapshot: &dyn Snapshot,
    state: Vec<u8>,
    verb: &str,
) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    let state = if snapshot.resume_trusted() {
        Some(state)
    } else {
        eprintln!(
            "slot: {verb}: the core refused the resume it was given, not overwriting the saved one"
        );
        None
    };
    let sav = snapshot.save_ram();
    let sav = if snapshot.save_ram_trusted() {
        sav
    } else {
        if sav.is_some() {
            eprintln!(
                "slot: {verb}: the core refused the save ram it was given, not overwriting the saved one"
            );
        }
        None
    };
    (state, sav)
}

fn up(level: u8, step: u8, max: u8) -> u8 {
    level.saturating_add(step).min(max)
}

fn tint_alpha(tint: WallpaperTint) -> f32 {
    match tint {
        WallpaperTint::Off => 0.0,
        WallpaperTint::Low => 0.25,
        WallpaperTint::Medium => 0.45,
        WallpaperTint::High => 0.65,
    }
}

fn auto_sleep_value(secs: u16) -> QuickValue {
    match secs {
        30 => QuickValue::Sleep30s,
        60 => QuickValue::Sleep1m,
        300 => QuickValue::Sleep5m,
        _ => QuickValue::Off,
    }
}

fn ff_next(from: u8, right: bool) -> u8 {
    let n = FF_SPEEDS.len();
    let at = FF_SPEEDS.iter().position(|&v| v == from).unwrap_or(0);
    FF_SPEEDS[if right {
        (at + 1) % n
    } else {
        (at + n - 1) % n
    }]
}

fn clock_screen(utc: i64, zone: TimeZone, h24: bool, from_menu: bool) -> Phase {
    Phase::SetClock {
        picker: ClockPicker::local(utc, zone).hours24(h24),
        seed: utc - utc.rem_euclid(60),
        from_menu,
    }
}

fn system_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn free_stamp(ring: &StateRing, now: i64) -> String {
    let taken: Vec<String> = ring
        .list()
        .map(|l| l.into_iter().map(|e| e.stamp).collect())
        .unwrap_or_default();
    // Network time may move the clock backwards. Keep a new snapshot after the newest stored
    // one so eviction cannot throw away the save just made.
    let newest = taken.iter().filter_map(|s| slot_store::parse_stamp(s)).max();
    let mut secs = newest.map_or(now, |latest| now.max(latest.saturating_add(1)));
    let mut stamp = format_stamp(secs);
    while taken.contains(&stamp) {
        secs += 1;
        stamp = format_stamp(secs);
    }
    stamp
}

fn bezel_slot(platform: Platform) -> usize {
    match platform {
        Platform::Gba | Platform::Nes | Platform::Snes => 0,
        Platform::Gb => 1,
        Platform::Gbc => 2,
    }
}

#[cfg(test)]
mod network_time_tests {
    use super::*;

    #[test]
    fn backward_clock_correction_does_not_evict_the_new_snapshot() {
        let d = tempfile::tempdir().unwrap();
        let ring = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Example");
        let now = 1_800_000_000;
        for n in 0..slot_store::RING_MAX {
            ring.push(&[n as u8], &[], &format_stamp(now + n as i64))
                .unwrap();
        }
        let stamp = free_stamp(&ring, now - 3600);
        ring.push(b"latest", &[], &stamp).unwrap();
        assert_eq!(ring.list().unwrap()[0].stamp, stamp);
        assert_eq!(ring.read(&stamp).unwrap().0, b"latest");
        assert_eq!(ring.list().unwrap().len(), slot_store::RING_MAX);
    }
}

/// A studio tab's name's place among the faces: colour, label, marker.
fn tab_face(tab: shell_picker::Tab) -> usize {
    match tab {
        shell_picker::Tab::Colour => 0,
        shell_picker::Tab::Label => 1,
        shell_picker::Tab::Marker => 2,
    }
}

/// The in-game Screen rows only a TV console has.
const TV_ROWS: [QuickRow; 5] = [
    QuickRow::Cable,
    QuickRow::Colors,
    QuickRow::Scanlines,
    QuickRow::Mask,
    QuickRow::Afterglow,
];

/// A TV console's screen effects, as they are drawn.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct TvLook {
    pub cable: TvCable,
    pub colors: NesColors,
    pub scanlines: bool,
    pub mask: bool,
    pub afterglow: bool,
}

fn cable_value(cable: TvCable) -> QuickValue {
    match cable {
        TvCable::Off => QuickValue::Off,
        TvCable::Rf => QuickValue::Rf,
        TvCable::Composite => QuickValue::Composite,
        TvCable::SVideo => QuickValue::SVideo,
        TvCable::Rgb => QuickValue::Rgb,
    }
}

/// The next cable along: the SNES's core also draws an aerial's RF signal, the NES's does not.
fn step_cable(cable: TvCable, right: bool, rf: bool) -> TvCable {
    let all: Vec<TvCable> = TvCable::ALL
        .into_iter()
        .filter(|c| rf || *c != TvCable::Rf)
        .collect();
    let at = all.iter().position(|c| *c == cable).unwrap_or(0);
    let n = all.len();
    all[if right { (at + 1) % n } else { (at + n - 1) % n }]
}

/// The label search's words, for the frontend to make faces of.
pub struct SearchWords {
    pub heading: String,
    pub typed: Option<String>,
    pub status: Option<String>,
    /// Each game found: its name and its platform.
    pub rows: Vec<(String, String)>,
}
