use std::time::{Duration, Instant};

use slot_gfx::{Compositor, Draw, ScreenEffect, TexId, OUT_H, OUT_W};

/// Labels cached quicker than this are done before the banner would show them.
/// The studio's Model row words: the SNES's, the Game Boy's, then the Super Game Boy's.
const MODEL_WORDS: [&str; 7] = ["PAL", "NTSC", "GB", "GBC", "Europe", "USA", "SGB 2"];
const REBUILD_BANNER_AFTER: Duration = Duration::from_millis(600);
const PAPER: &[u8] = include_bytes!("../assets/paper.png");
const GBA_LCD_OVERLAY: &[u8] = include_bytes!("../assets/gba_lcd_overlay.png");
use slot_input::{InputSource, Millis};
use slot_power::{Platform, Power};
use slot_store::{format_stamp, BezelPick, Platform as Console};
use slot_ui::{
    arrows_hint_coloured, arrows_hint_face, badge_face, bezel_face, cart_shadow, chip_face,
    chip_shadow_face, clock_text, date_time_text_in, default_game_bezel_face, footer_ink,
    game_bezel_face, hint_coloured, hint_face, icon_face, menu_face, photo_face, quick_caret_face,
    quick_label_face, quick_legend_faces, quick_pointer_face, quick_value_face,
    rebuild_banner_face, set_clock_hint_face, slot_back, slot_back_light, slot_front,
    slot_front_light, socket_face, title_face, toast_face, wallpaper_face, word_ink, CartFace,
    Icon, LinkBadge, Mould, QuickMenuFaces, QuickRow, QuickValue, RebuildBanner, Toast, UndoFace,
    ALERT_PX, BOLT_PX, HUD_ICON_PX, HUD_INK, LEGEND,
};

use crate::app::{App, LinkRow, Phase};
use crate::build_info::Build;
use crate::cart_faces::CartFaces;
use crate::face_builder::FaceBuilder;
use crate::label_cache::{self, Rebuild};
use crate::link_art_builder::LinkArtBuilder;
use crate::link_screen::{LinkSprites, Sprite};
use crate::link_start::{LinkFail, LinkStep};
use crate::session::Session;
use slot_ui::SgbModel;

const DOZE_TIMEOUT: Duration = Duration::from_secs(300);

const ALERT_INK: [u8; 3] = [0xf0, 0xb4, 0x3c];

pub struct Frontend {
    labels: crate::labels::Labels,
    // Started once Home Wi-Fi is connected, not at boot: the network is not up yet then.
    cheat_fetch: Option<crate::cheat_fetch::CheatFetch>,
    /// The label worker has run with Home Wi-Fi connected.
    was_online: bool,
    session: Session,
    start: Instant,
    last: Instant,
    draws: Vec<Draw>,
    polaroid_texes: Vec<TexId>,
    title_tex: Option<TexId>,
    faces: FaceBuilder,
    cart_faces: CartFaces,
    rebuild: Option<Rebuild>,
    stale_check: Option<std::sync::mpsc::Receiver<Vec<slot_store::Cart>>>,
    label_fetch: Option<std::sync::mpsc::Receiver<(slot_store::Cart, LabelFetched)>>,
    versions_fetch:
        Option<std::sync::mpsc::Receiver<(std::path::PathBuf, Option<Vec<VersionFile>>)>>,
    search_fetch: Option<std::sync::mpsc::Receiver<(u64, Option<Vec<slot_labels::Game>>)>>,
    search_tex: SearchTextures,
    /// Each version's thumbnail and caption, reused from one cart to the next.
    version_tex: Vec<(Option<TexId>, Option<TexId>)>,
    ui_faces: Option<std::sync::mpsc::Receiver<UiFaces>>,
    rebuild_faces: RebuildFaces,
    link_art: LinkArtBuilder,
    link_art_done: bool,
    core_asked: Option<String>,
    core_tex: CoreTextures,
    core_built: Option<String>,
    switcher: Switcher,
    clocks: Clocks,
    about: AboutFace,
    cheats: CheatTextures,
    shot_tex: ShotTextures,
    /// A game-only frame was drawn last frame and waits to be read back.
    capture_pending: bool,
    shot_pending: bool,
    quick_clock: QuickClock,
    quick_network: QuickClock,
    quick_zone: QuickClock,
    quick_palette: QuickPalette,
    wallpaper: WallpaperSync,
    game_bezel: BezelSync,
    look: Option<ShelfLook>,
    link_shot: Option<TexId>,
    link_shot_size: (u32, u32),
    link_shot_shown: bool,
}

/// A version of a cart's label, kept on the card while the studio shows it.
struct VersionFile {
    path: std::path::PathBuf,
    caption: String,
    thumb: Vec<u8>,
    w: u32,
    h: u32,
}

/// Where the label tab keeps the versions it shows.
const VERSIONS_DIR: &str = "System/Cache/Labels/Versions";
/// A version's thumbnail is drawn this tall; it is made at twice that.
const THUMB_H: u32 = 84;

enum LabelFetched {
    Found(std::path::PathBuf),
    Missing,
    Failed,
}

#[derive(Default)]
struct RebuildFaces {
    line: Option<TexId>,
    shown: Option<usize>,
}

#[derive(Default)]
struct QuickClock {
    dim: Option<TexId>,
    lit: Option<TexId>,
    shown: String,
}

#[derive(Default)]
struct QuickPalette {
    dim: Option<TexId>,
    lit: Option<TexId>,
    shown: &'static str,
}

#[derive(Default)]
struct BezelSync {
    tex: Option<TexId>,
    image: Option<(Console, BezelPick)>,
    labels: [String; 3],
    faces: [(Option<TexId>, Option<TexId>); 3],
}

#[derive(Default)]
struct WallpaperSync {
    tex: Option<TexId>,
    image: Option<String>,
    label: String,
    dim: Option<TexId>,
    lit: Option<TexId>,
}

/// The Cheats page's textures, reused from one game to the next.
#[derive(Default)]
struct CheatTextures {
    rev: Option<u64>,
    counts: Option<(usize, usize)>,
    names: Vec<Option<TexId>>,
    help: Option<TexId>,
    empty: Option<TexId>,
    empty_help: Option<TexId>,
    legend: [Option<TexId>; 3],
    switches: [Option<TexId>; 2],
    more: [Option<TexId>; 2],
    badges: [Option<TexId>; 2],
}

/// The Screenshots page's faces and a few pictures, reused texture by texture.
#[derive(Default)]
struct ShotTextures {
    built: bool,
    key: Option<(u64, usize)>,
    title: Option<TexId>,
    date: Option<TexId>,
    fixed: [Option<TexId>; 3],
    hints: [Option<TexId>; 7],
    state_line: Option<TexId>,
    /// Up to five pictures: (path, texture), least recently used first.
    pics: Vec<(std::path::PathBuf, TexId)>,
}

#[derive(Default)]
struct AboutFace {
    tex: Option<TexId>,
    light: Option<bool>,
}

#[derive(Default)]
struct Clocks {
    line: Option<TexId>,
    hint: Option<TexId>,
    shelf: Option<TexId>,
    picked: Option<String>,
    shown: String,
    battery: String,
    battery_tex: Option<TexId>,
    platform: String,
    platform_tex: Option<TexId>,
    title: String,
    title_tex: Option<TexId>,
    ink: [u8; 3],
}

struct ShelfLook {
    slot: [(TexId, TexId); 17],
    nes_lids: [TexId; 3],
    bolt: [TexId; 2],
    wifi: [TexId; 2],
    port: [Option<TexId>; 2],
    legend: [Vec<(TexId, u32)>; 2],
    applied: Option<(bool, bool, bool)>,
}

#[derive(Default)]
struct Switcher {
    open: bool,
    titled: Option<String>,
}

impl Frontend {
    pub fn boot(platform: Box<dyn Platform>) -> Self {
        Self::boot_with(platform, crate::audio::opened_sink())
    }

    pub fn boot_with(platform: Box<dyn Platform>, sink: Box<dyn crate::audio::AudioSink>) -> Self {
        let now = Instant::now();
        let cart_faces = CartFaces::spawn(platform.root().to_path_buf());
        let mut session = Session::boot_with(platform.root().to_path_buf(), sink);
        crate::boot_time::mark("session");
        // Missing labels come from the artwork pack or, with Home Wi-Fi on, from LaunchBox.
        // With neither there is nothing to fetch them from, so no ROM is read for it.
        let fetch = session.app().home_wifi_on()
            || platform
                .root()
                .join("Labels")
                .join(slot_labels::build::PACK)
                .is_file();
        let labels = crate::labels::Labels::spawn(
            platform.root().to_path_buf(),
            if fetch {
                session.app().carts().cloned().collect()
            } else {
                Vec::new()
            },
        );
        let stale_check =
            Self::check_labels(platform.root(), session.app().carts().cloned().collect());
        session
            .app_mut()
            .set_power(Power::new(platform, DOZE_TIMEOUT));
        Frontend {
            labels,
            session,
            start: now,
            last: now,
            draws: Vec::new(),
            polaroid_texes: Vec::new(),
            title_tex: None,
            faces: FaceBuilder::spawn(),
            cart_faces,
            rebuild: None,
            stale_check,
            label_fetch: None,
            versions_fetch: None,
            search_fetch: None,
            search_tex: SearchTextures::default(),
            version_tex: Vec::new(),
            cheat_fetch: None,
            was_online: false,
            ui_faces: None,
            rebuild_faces: RebuildFaces::default(),
            link_art: LinkArtBuilder::spawn(),
            link_art_done: false,
            core_asked: None,
            core_tex: CoreTextures::default(),
            core_built: None,
            switcher: Switcher::default(),
            clocks: Clocks::default(),
            about: AboutFace::default(),
            cheats: CheatTextures::default(),
            shot_tex: ShotTextures::default(),
            capture_pending: false,
            shot_pending: false,
            quick_clock: QuickClock::default(),
            quick_network: QuickClock::default(),
            quick_zone: QuickClock::default(),
            quick_palette: QuickPalette::default(),
            wallpaper: WallpaperSync::default(),
            game_bezel: BezelSync::default(),
            look: None,
            link_shot: None,
            link_shot_size: (0, 0),
            link_shot_shown: false,
        }
    }

    fn upload_paper(compositor: &mut Compositor) {
        let mut dec = png::Decoder::new(std::io::Cursor::new(PAPER));
        dec.set_transformations(png::Transformations::normalize_to_color8());
        let Ok(mut reader) = dec.read_info() else {
            return;
        };
        let mut buf = vec![0u8; reader.output_buffer_size()];
        let Ok(info) = reader.next_frame(&mut buf) else {
            return;
        };
        if info.color_type != png::ColorType::Grayscale || info.width != info.height {
            return;
        }
        compositor.set_paper(info.width, &buf[..info.buffer_size()]);
    }

    fn upload_gba_lcd_overlay(compositor: &mut Compositor) {
        let mut dec = png::Decoder::new(std::io::Cursor::new(GBA_LCD_OVERLAY));
        dec.set_transformations(png::Transformations::normalize_to_color8());
        let Ok(mut reader) = dec.read_info() else {
            return;
        };
        let mut buf = vec![0u8; reader.output_buffer_size()];
        let Ok(info) = reader.next_frame(&mut buf) else {
            return;
        };
        if info.color_type != png::ColorType::Rgba || (info.width, info.height) != (OUT_W, OUT_H) {
            return;
        }
        compositor.set_lcd_overlay(info.width, info.height, &buf[..info.buffer_size()]);
    }

    pub fn upload_faces(&mut self, compositor: &mut Compositor) {
        self.upload_boot_faces(compositor);
        if let Some(faces) = self.ui_faces.take().and_then(|rx| rx.recv().ok()) {
            self.place_ui_faces(compositor, faces);
        }
    }

    pub fn upload_boot_faces(&mut self, compositor: &mut Compositor) {
        Self::upload_paper(compositor);
        Self::upload_gba_lcd_overlay(compositor);
        crate::boot_time::mark("paper");
        if self.rebuild.is_none() {
            self.cart_faces.sync(self.session.app_mut(), compositor);
        }
        crate::boot_time::mark("cart faces");
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("slot-ui-faces".into())
            .spawn(move || {
                let _ = tx.send(UiFaces::build());
            });
        match spawned {
            Ok(_) => self.ui_faces = Some(rx),
            Err(_) => self.place_ui_faces(compositor, UiFaces::build()),
        }
        let icons = Icon::ALL
            .iter()
            .map(|i| {
                let f = icon_face(*i, HUD_ICON_PX, HUD_INK);
                compositor.create_texture(f.w, f.h, &f.rgba)
            })
            .collect();
        self.session.app_mut().set_icon_faces(icons);
        let alert = icon_face(Icon::Alert, ALERT_PX, ALERT_INK);
        let alert = compositor.create_texture(alert.w, alert.h, &alert.rgba);
        self.session.app_mut().set_alert_face(alert);
        let legend = legend_faces(compositor, &LEGEND);
        self.session.app_mut().set_legend_faces(legend);
        let hint = set_clock_hint_face();
        self.clocks.hint = Some(compositor.create_texture(hint.w, hint.h, &hint.rgba));
        let drops = Mould::ALL.map(|m| {
            let (cw, ch) = slot_ui::cart_box(m);
            let f = slot_ui::drop_shadow_face(cw, ch, slot_ui::DROP_MARGIN);
            compositor.create_texture(f.w, f.h, &f.rgba)
        });
        self.session.app_mut().set_drop_shadows(drops);
        let mut halos = |face: fn(u32, u32, u32, [u8; 3], f32) -> slot_ui::CartFace, peak: f32| {
            Mould::ALL.map(|m| {
                let (cw, ch) = slot_ui::cart_box(m);
                let halo = face(cw, ch, slot_ui::GLOW_MARGIN, slot_ui::GLOW_GOLD, peak);
                compositor.create_texture(halo.w, halo.h, &halo.rgba)
            })
        };
        let gold = halos(slot_ui::glow_face, 0.38);
        let white = halos(slot_ui::halo_face, 0.45);
        let star = slot_ui::star_mark_face(slot_ui::STAR_PX);
        let star_id = compositor.create_texture(star.w, star.h, &star.rgba);
        self.session
            .app_mut()
            .set_mark_faces(gold, white, (star_id, star.w, star.h));
        let shadows = Mould::ALL.map(|m| {
            let shadow = cart_shadow(m);
            compositor.create_texture(shadow.w, shadow.h, &shadow.rgba)
        });
        self.session.app_mut().set_cart_shadows(shadows);
        let boards = Mould::ALL.map(|m| {
            let board = slot_ui::cart_backing(m);
            compositor.create_texture(board.w, board.h, &board.rgba)
        });
        self.session.app_mut().set_cart_backings(boards);
        let slot = {
            let mut upload = |bytes: &[u8]| {
                // The Super Game Boy's band stands taller than the console's.
                let h = bytes.len() as u32 / (slot_ui::SLOT_W * 4);
                debug_assert!(h == slot_ui::SLOT_H || h == slot_ui::SGB_BAND_H);
                compositor.create_texture(slot_ui::SLOT_W, h, bytes)
            };
            [
                (upload(slot_back()), upload(slot_front())),
                (upload(slot_back_light()), upload(slot_front_light())),
                (
                    upload(slot_ui::slot_back_gb(false)),
                    upload(slot_ui::slot_front_gb(false)),
                ),
                (
                    upload(slot_ui::slot_back_gb(true)),
                    upload(slot_ui::slot_front_gb(true)),
                ),
                (
                    upload(slot_ui::slot_tv(true, false, false)),
                    upload(slot_ui::slot_tv(true, true, false)),
                ),
                (
                    upload(slot_ui::slot_tv(true, false, true)),
                    upload(slot_ui::slot_tv(true, true, true)),
                ),
                (
                    upload(slot_ui::slot_tv(false, false, false)),
                    upload(slot_ui::slot_tv(false, true, false)),
                ),
                (
                    upload(slot_ui::slot_tv(false, false, true)),
                    upload(slot_ui::slot_tv(false, true, true)),
                ),
                (
                    upload(slot_ui::sgb_band(SgbModel::Ntsc, false)),
                    upload(slot_ui::sgb_band(SgbModel::Ntsc, true)),
                ),
                (
                    upload(slot_ui::sgb_band(SgbModel::Pal, false)),
                    upload(slot_ui::sgb_band(SgbModel::Pal, true)),
                ),
                (
                    upload(slot_ui::sgb_band(SgbModel::Sfc, false)),
                    upload(slot_ui::sgb_band(SgbModel::Sfc, true)),
                ),
                (
                    upload(slot_ui::sgb_band(SgbModel::Sgb2, false)),
                    upload(slot_ui::sgb_band(SgbModel::Sgb2, true)),
                ),
                (upload(slot_ui::slot_orig(0, false)), upload(slot_ui::slot_orig(0, true))),
                (upload(slot_ui::slot_orig(1, false)), upload(slot_ui::slot_orig(1, true))),
                (upload(slot_ui::slot_orig(2, false)), upload(slot_ui::slot_orig(2, true))),
                (upload(slot_ui::slot_orig(3, false)), upload(slot_ui::slot_orig(3, true))),
                (upload(slot_ui::slot_orig(4, false)), upload(slot_ui::slot_orig(4, true))),
            ]
        };
        let nes_lids = [
            slot_ui::nes_lid(false),
            slot_ui::nes_lid(true),
            slot_ui::nes_lid_orig(),
        ]
        .map(|lid| compositor.create_texture(slot_ui::NES_LID_W, slot_ui::SLOT_H, lid));
        let dark_bolt = icon_face(Icon::Charging, BOLT_PX, footer_ink(false));
        let light_bolt = icon_face(Icon::Charging, BOLT_PX, footer_ink(true));
        let bolt = [
            compositor.create_texture(dark_bolt.w, dark_bolt.h, &dark_bolt.rgba),
            compositor.create_texture(light_bolt.w, light_bolt.h, &light_bolt.rgba),
        ];
        let wifi = [false, true].map(|light| {
            let f = slot_ui::wifi_face(footer_ink(light));
            compositor.create_texture(f.w, f.h, &f.rgba)
        });
        self.look = Some(ShelfLook {
            slot,
            nes_lids,
            bolt,
            wifi,
            port: [None, None],
            legend: [Vec::new(), Vec::new()],
            applied: None,
        });
        self.apply_look();
        crate::boot_time::mark("shelf faces");
        self.upload_ground(compositor);
    }

    fn upload_ground(&mut self, compositor: &mut Compositor) {
        let Some(rgba) = slot_ui::default_background() else {
            return;
        };
        let id = compositor.create_texture(OUT_W, OUT_H, rgba);
        self.session.app_mut().set_ground(id);
        if let Some(rgba) = slot_ui::light_background() {
            let id = compositor.create_texture(OUT_W, OUT_H, rgba);
            self.session.app_mut().set_ground_light(id);
        }
    }

    fn apply_look(&mut self) {
        // The footer's icons sit on the slot: their ink follows its housing.
        let light = self.session.app().slot_light();
        let ready = self.look.as_ref().map(|look| {
            let i = usize::from(light);
            let legend_ready = !look.legend[i].is_empty();
            let port_ready = look.port[i].is_some();
            (
                look.applied == Some((light, legend_ready, port_ready)),
                (look.slot, look.nes_lids),
                look.bolt[i],
                (look.wifi[i], look.wifi[0]),
                look.bolt[0],
                look.port[i],
                look.legend[i].clone(),
                legend_ready,
                port_ready,
            )
        });
        let Some((done, slot, bolt, wifi, hud_bolt, port, legend, legend_ready, port_ready)) =
            ready
        else {
            return;
        };
        if done {
            return;
        }
        self.session.app_mut().set_slot_sets(slot.0, slot.1);
        self.session.app_mut().set_bolt_face(bolt);
        self.session.app_mut().set_wifi_face(wifi.0, wifi.1);
        self.session.app_mut().set_hud_bolt_face(hud_bolt);
        if let Some(tex) = port {
            self.session.app_mut().set_link_port(tex);
        }
        if legend_ready {
            self.session.app_mut().set_link_legend_faces(legend);
        }
        if let Some(look) = &mut self.look {
            look.applied = Some((light, legend_ready, port_ready));
        }
    }

    fn poll_ui_faces(&mut self, compositor: &mut Compositor) {
        let Some(rx) = &self.ui_faces else {
            return;
        };
        match rx.try_recv() {
            Ok(faces) => {
                self.ui_faces = None;
                self.place_ui_faces(compositor, faces);
                crate::boot_time::mark("ui faces");
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.ui_faces = None,
        }
    }

    fn place_ui_faces(&mut self, compositor: &mut Compositor, faces: UiFaces) {
        let mut cart = |f: CartFace| compositor.create_texture(f.w, f.h, &f.rgba);
        let link_badges = faces.link_badges.into_iter().map(&mut cart).collect();
        let toasts = faces.toasts.into_iter().map(&mut cart).collect();
        let sockets = faces.sockets.into_iter().map(&mut cart).collect();
        let chips = faces.chips.into_iter().map(&mut cart).collect();
        let blank = cart(faces.blank_chip);
        let shadow = cart(faces.chip_shadow);
        let mut up = |f: UndoFace| (compositor.create_texture(f.w, f.h, &f.rgba), f.w, f.h);
        let labels = faces
            .quick_labels
            .into_iter()
            .map(|l| l.map(&mut up))
            .collect();
        let pointer = up(faces.quick_pointer);
        let values = faces
            .quick_values
            .into_iter()
            .map(|v| v.map(&mut up))
            .collect();
        let carets = faces.quick_carets.map(&mut up);
        let more = faces.quick_more.map(&mut up);
        let switches = faces.quick_switches.map(&mut up);
        let button_tabs = faces.button_tabs.into_iter().map(&mut up).collect();
        let button_bodies = faces.button_bodies.into_iter().map(&mut up).collect();
        let [left, right] = faces.button_arrows;
        let button_arrows = [up(left), up(right)];
        let button_legend = faces.button_legend.map(|f| {
            let (tex, w, _) = up(f);
            (tex, w)
        });
        let help = faces
            .quick_help
            .into_iter()
            .map(|f| f.map(&mut up))
            .collect();
        let legend = faces.quick_legend.map(|f| {
            let (tex, w, _) = up(f);
            (tex, w)
        });
        let core_legend = faces
            .core_legend
            .into_iter()
            .map(|f| {
                let (tex, w, _) = up(f);
                (tex, w)
            })
            .collect();
        let [disc, ring, original] = faces.shell_shapes.map(|f| up(f).0);
        let mut words: Vec<(TexId, u32)> = faces
            .shell_words
            .into_iter()
            .map(|f| {
                let (tex, w, _) = up(f);
                (tex, w)
            })
            .collect();
        let inks = words.split_off(words.len() - crate::shell_picker::MARKERS.len());
        let found = words.split_off(words.len() - 2);
        let tabs = words.split_off(words.len() - 3);
        let models = words.split_off(words.len() - MODEL_WORDS.len());
        let finishes = words.split_off(words.len() - crate::shell_picker::FINISH_NAMES.len());
        let mut shell_legend: Vec<(TexId, u32)> = faces
            .shell_legend
            .into_iter()
            .map(|f| {
                let (tex, w, _) = up(f);
                (tex, w)
            })
            .collect();
        let search_hint = shell_legend.pop().expect("the search hint");
        let remove_hint = shell_legend.pop().expect("the remove hint");
        let confirm = shell_legend.split_off(shell_legend.len() - 2);
        let shell_faces = slot_ui::ShellPickerFaces {
            disc,
            ring,
            original,
            names: words,
            finishes,
            models,
            tabs: [tabs[0], tabs[1], tabs[2]],
            looking: found[0],
            none_found: found[1],
            inks,
            legend: shell_legend,
            remove: remove_hint,
            confirm,
            search: search_hint,
        };
        let roles = faces.link_roles.into_iter().map(&mut up).collect();
        let linked = up(faces.linked);
        let mut pack = |faces: Vec<UndoFace>| -> Vec<(TexId, u32)> {
            faces
                .into_iter()
                .map(|f| {
                    let (tex, w, _) = up(f);
                    (tex, w)
                })
                .collect()
        };
        let link_legend = pack(faces.link_legend);
        let link_legend_light = pack(faces.link_legend_light);
        let steps = faces.link_steps.into_iter().map(&mut up).collect();
        let fails = faces.link_fails.into_iter().map(&mut up).collect();
        if let Some(look) = &mut self.look {
            look.legend = [link_legend.clone(), link_legend_light];
            look.applied = None;
        }
        let app = self.session.app_mut();
        app.set_link_badge_faces(link_badges);
        app.set_quick_menu_faces(QuickMenuFaces {
            labels,
            pointer,
            values,
            carets,
            legend,
            switches,
            help,
            more,
        });
        app.set_buttons_faces(slot_ui::ButtonsFaces {
            tabs: button_tabs,
            bodies: button_bodies,
            legend: button_legend,
            arrows: button_arrows,
        });
        app.set_core_part_faces(sockets, chips, blank, shadow);
        app.set_core_legend_faces(core_legend);
        app.set_shell_picker_faces(shell_faces);
        app.set_link_menu_faces(roles);
        app.set_link_linked_face(linked);
        app.set_link_legend_faces(link_legend);
        app.set_link_step_faces(steps);
        app.set_link_fail_faces(fails);
        app.set_toast_faces(toasts);
    }

    pub fn upload_bezel(&mut self, compositor: &mut Compositor, panel: (u32, u32)) {
        self.session.app_mut().set_panel(panel);
        if !slot_gfx::framed(panel) {
            return;
        }
        let Some(path) = self
            .session
            .app()
            .root()
            .and_then(|root| crate::bezel::pick(root, panel))
        else {
            return;
        };
        if let Some(rgba) = bezel_face(&path, panel.0, panel.1) {
            compositor.set_bezel(panel.0, panel.1, &rgba);
            eprintln!("slot: bezel {}", path.display());
        }
    }

    pub fn render(&mut self, compositor: &mut Compositor, window: (u32, u32)) {
        self.session.app_mut().set_panel(window);
        compositor.fit(window);
        self.compose(compositor);
        compositor.end_frame(window);
    }

    fn grab_link_shot(&mut self, compositor: &mut Compositor) {
        let open = self.session.app().game_menu_open();
        if !open {
            if self.link_shot_shown {
                self.link_shot_shown = false;
                self.session.app_mut().set_link_shot(None);
            }
            return;
        }
        if self.link_shot_shown {
            return;
        }
        let (w, h) = compositor.frame_size();
        if w == 0 || h == 0 {
            return;
        }
        let rgba = compositor.read_frame();
        if rgba.len() < (w * h * 4) as usize {
            return;
        }
        let tex = match self.link_shot {
            Some(id) if self.link_shot_size == (w, h) => {
                compositor.update_texture(id, w, h, &rgba);
                id
            }
            _ => {
                let id = compositor.create_texture_nearest(w, h, &rgba);
                self.link_shot = Some(id);
                self.link_shot_size = (w, h);
                id
            }
        };
        self.session.app_mut().set_link_shot(Some(tex));
        self.link_shot_shown = true;
    }

    /// Read back the game-only frame drawn for the boot screen, the frame after it was drawn.
    fn grab_boot_picture(&mut self, compositor: &mut Compositor) {
        if std::mem::take(&mut self.capture_pending) {
            let (w, h) = compositor.frame_size();
            let rgba = compositor.read_frame();
            if w > 0 && h > 0 && rgba.len() >= (w * h * 4) as usize {
                self.session.app_mut().set_boot_picture(rgba, w, h);
            }
        }
        if self.session.app().capturing() {
            self.capture_pending = true;
        }
    }

    /// Read back the frame drawn for a screenshot, the frame after it was asked for.
    fn grab_screenshot(&mut self, compositor: &mut Compositor) {
        if std::mem::take(&mut self.shot_pending) {
            let (w, h) = compositor.frame_size();
            let rgba = compositor.read_frame();
            if w > 0 && h > 0 && rgba.len() >= (w * h * 4) as usize {
                self.session.app_mut().save_screenshot(rgba, w, h);
            }
        }
        if self.session.app().shot_pending() {
            self.shot_pending = true;
        }
    }

    /// Booting back into a game: the picture it left as the boot screen, until it runs again.
    fn load_resume_picture(&mut self, compositor: &mut Compositor) {
        let app = self.session.app_mut();
        if app.resume_picture_done() {
            if let Some(tex) = app.clear_resume_picture() {
                // Shrink the picture's texture away; it shows once a boot.
                compositor.update_texture(tex, 1, 1, &[0, 0, 0, 0]);
            }
            return;
        }
        if !app.picture_wanted() {
            return;
        }
        let tex = app
            .root()
            .map(|r| r.to_path_buf())
            .and_then(|root| crate::bootlogo::take_picture(&root))
            .map(|bmp| {
                let rgba: Vec<u8> = bmp
                    .rgb
                    .iter()
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect();
                compositor.create_texture(bmp.width as u32, bmp.height as u32, &rgba)
            });
        app.set_resume_picture(tex);
    }

    pub fn compose(&mut self, compositor: &mut Compositor) {
        compositor.set_blue_light(self.session.app().blue_light());
        compositor.set_picture(self.session.app().picture_rect());
        let (cw, ch) = self.session.app().platform().canvas();
        compositor.set_game_canvas(cw, ch);
        let platform = self.session.app().playing_platform();
        // A Game Boy game in its own Game Boy Color colors (Color Palettes off) is drawn as
        // the GBC draws it: the GBC's panel, LCD lines and saturation.
        let colour = !self.session.app().gb_palette_live();
        let gb = platform == slot_store::Platform::Gb && !colour;
        let gbc = platform == slot_store::Platform::Gbc
            || (platform == slot_store::Platform::Gb && colour);
        // Effects off is the emulator's own picture: no dimming, saturation or panel tone.
        let bare = self.session.app().screen_bare();
        compositor.set_picture_appearance(gb, if gb && !bare { 0.03 } else { 0.0 });
        let gba = platform == slot_store::Platform::Gba;
        // The colour screens, GBC and GBA, cast lighter shadows than the Game Boy's.
        compositor.set_pixel_shadow(match (gba || gbc, gb) {
            (true, _) => COLOUR_PIXEL_SHADOW,
            (_, true) => GB_PIXEL_SHADOW,
            _ => slot_gfx::PIXEL_SHADOW,
        });
        compositor.set_pale_opacity(match gba || gbc {
            true => COLOUR_PALE_OPACITY,
            false => GB_PALE_OPACITY,
        });
        compositor.set_lcd_mix(GBC_LCD[0], GBC_LCD[1], gba);
        // Effects off: black around the picture too, nothing of slot.'s screen left.
        compositor.set_black_edge(self.session.app().black_border() || bare);
        compositor.set_saturation(match (gba, gbc) {
            _ if bare => 1.0,
            (true, _) => GBA_SATURATION,
            (_, true) => GBC_SATURATION,
            _ => 1.0,
        });
        match gbc || gba {
            true if bare => compositor.set_panel(Some(GBC_PANEL), [1.0; 3]),
            true => compositor.set_panel(Some(GBC_PANEL), GBC_PANEL),
            false => compositor.set_panel(None, [1.0; 3]),
        }
        compositor.set_grid_opacity(if gb {
            0.50
        } else if gbc {
            0.20
        } else {
            0.30
        });
        // A TV console's picture is a tube's, not an LCD's: no see-through pixels.
        let tv = platform.tv();
        // A TV console's screen effects go on and off together: the filter and colours in
        // its core, the scanlines and the phosphor's afterglow here.
        let tv_look = self.session.app().tv_look(platform);
        let afterglow = tv_look.is_some_and(|l| l.afterglow);
        compositor.set_afterglow(if afterglow { TV_AFTERGLOW } else { 0 });
        let lines = tv_look.map_or((false, false), |l| (l.scanlines, l.mask));
        compositor.set_tv_lines(lines.0, lines.1);
        compositor.set_pixel_transparency(self.session.app().pixel_transparency() && !tv);
        compositor.set_screen_effect(match self.session.app().screen_shader() {
            _ if tv && (lines.0 || lines.1) => ScreenEffect::Crt,
            _ if tv => ScreenEffect::None,
            slot_store::Shader::Off => ScreenEffect::None,
            slot_store::Shader::Grid if gbc => ScreenEffect::GbaLcd,
            slot_store::Shader::Lcd3x => ScreenEffect::Lcd3x,
            slot_store::Shader::Grid => ScreenEffect::Grid,
            slot_store::Shader::Dot => ScreenEffect::Dot,
            slot_store::Shader::Simpletex => ScreenEffect::Simpletex,
            // The GBC's LCD is the GBA's mode, drawn on the GBC's own pixels over its panel.
            slot_store::Shader::GbaLcd | slot_store::Shader::GbcLcd => ScreenEffect::GbaLcd,
        });
        compositor.set_shake(self.session.app().screen_shake());
        compositor.set_screen_power(self.session.app().screen_power());
        compositor.set_overlay_slide(self.session.app().overlay_slide());
        compositor.set_game_source_rect(self.session.app().source_rect());
        self.grab_link_shot(compositor);
        self.grab_boot_picture(compositor);
        self.grab_screenshot(compositor);
        self.load_resume_picture(compositor);
        compositor.begin_frame(self.session.app().layout());
        self.poll_ui_faces(compositor);
        self.poll_rebuild();
        if let Some(frame) = self.session.frame() {
            compositor.upload_game(&frame);
            crate::latency::taken();
        }
        self.sync_labels();
        self.fetch_asked_label();
        self.fetch_versions(compositor);
        self.search_labels();
        sync_label_search(self.session.app_mut(), compositor, &mut self.search_tex);
        self.cart_faces.sync(self.session.app_mut(), compositor);
        sync_clock(self.session.app_mut(), compositor, &mut self.clocks);
        sync_about(self.session.app_mut(), compositor, &mut self.about);
        sync_cheats(self.session.app_mut(), compositor, &mut self.cheats);
        sync_shots(self.session.app_mut(), compositor, &mut self.shot_tex);
        sync_quick_clock(self.session.app_mut(), compositor, &mut self.quick_clock);
        sync_quick_network(self.session.app_mut(), compositor, &mut self.quick_network);
        sync_quick_zone(self.session.app_mut(), compositor, &mut self.quick_zone);
        sync_quick_palette(self.session.app_mut(), compositor, &mut self.quick_palette);
        sync_wallpaper(self.session.app_mut(), compositor, &mut self.wallpaper);
        sync_game_bezel(self.session.app_mut(), compositor, &mut self.game_bezel);
        compositor.set_game_bezel(self.session.app().shown_game_bezel());
        compositor.set_sp_frame(self.session.app().sp_frame());
        sync_core_picker(
            self.session.app_mut(),
            compositor,
            &self.faces,
            &mut self.core_asked,
            &mut self.core_tex,
            &mut self.core_built,
        );
        if !self.link_art_done {
            if let Some(art) = self.link_art.take() {
                let mut up = |f: &slot_ui::CartFace| Sprite {
                    tex: compositor.create_texture(f.w, f.h, &f.rgba),
                    w: f.w,
                    h: f.h,
                };
                let port_dark = up(&art.port);
                let port_light = up(&art.port_light);
                if let Some(look) = &mut self.look {
                    look.port = [Some(port_dark.tex), Some(port_light.tex)];
                    look.applied = None;
                }
                let sprites = LinkSprites {
                    port: port_dark,
                    plug_host: up(&art.plug_host),
                    plug_join: up(&art.plug_join),
                    adapter: up(&art.adapter),
                    arcs_right: [
                        up(&art.arcs_right[0]),
                        up(&art.arcs_right[1]),
                        up(&art.arcs_right[2]),
                    ],
                    arcs_left: [
                        up(&art.arcs_left[0]),
                        up(&art.arcs_left[1]),
                        up(&art.arcs_left[2]),
                    ],
                    clicks: up(&art.clicks),
                    arrow_left: up(&art.arrow_left),
                    arrow_right: up(&art.arrow_right),
                    net_home: up(&art.net_home),
                    net_direct: up(&art.net_direct),
                };
                self.session.app_mut().set_link_sprites(sprites);
                self.link_art_done = true;
            }
        }
        self.apply_look();
        sync_switcher(
            self.session.app_mut(),
            compositor,
            Faces {
                pool: &mut self.polaroid_texes,
                title: &mut self.title_tex,
            },
            &mut self.switcher,
        );
        self.draws.clear();
        self.session.app().draw(&mut self.draws);
        compositor.draw_list(&self.draws);
        self.draw_rebuild(compositor);
    }

    pub fn drive_emulator(&mut self) {
        self.session.set_driven(true);
    }

    pub fn step_emulator(&self, present: Duration, timeout: Duration) -> bool {
        self.session.step_emulator(present, timeout)
    }

    /// A label the background worker fetched: the cart takes it and its face is drawn again.
    fn sync_labels(&mut self) {
        let Some(ready) = self.labels.take() else {
            return;
        };
        let Some(path) = ready.cart.label else {
            return;
        };
        self.session.app_mut().attach_label(&ready.cart.rom, path);
        if self.core_asked.as_deref() == Some(ready.cart.stem.as_str())
            || self.core_built.as_deref() == Some(ready.cart.stem.as_str())
        {
            self.core_asked = None;
            self.core_built = None;
        }
    }

    /// The label search asked for the games some words find: looked for on a thread of its own.
    fn search_labels(&mut self) {
        if let Some(rx) = &self.search_fetch {
            match rx.try_recv() {
                Ok((asked, games)) => {
                    self.search_fetch = None;
                    self.session.app_mut().search_found(asked, games);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.search_fetch = None,
            }
        }
        let Some((asked, words, platform)) = self.session.app_mut().take_search_ask() else {
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("slot-label-search".into())
            .spawn(move || {
                let games = match slot_labels::Downloader::default().search(&words, platform) {
                    Ok(games) => Some(games),
                    Err(slot_labels::Error::Unavailable(_)) => Some(Vec::new()),
                    Err(e) => {
                        eprintln!("slot: labels: search {words:?}: {e}");
                        None
                    }
                };
                let _ = tx.send((asked, games));
            });
        match spawned {
            Ok(_) => self.search_fetch = Some(rx),
            Err(e) => eprintln!("slot: labels: search thread failed to start: {e}"),
        }
    }

    /// The studio's label tab asked for the versions of a cart's label: fetched and cut on a
    /// thread of its own, then each one's thumbnail and caption uploaded.
    fn fetch_versions(&mut self, compositor: &mut Compositor) {
        if let Some(rx) = &self.versions_fetch {
            let (rom, got) = match rx.try_recv() {
                Ok(got) => got,
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.versions_fetch = None;
                    return;
                }
            };
            self.versions_fetch = None;
            let found = got.map(|versions| {
                versions
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| {
                        if self.version_tex.len() <= i {
                            self.version_tex.resize(i + 1, (None, None));
                        }
                        let thumb =
                            upload_rgba(compositor, &mut self.version_tex[i].0, v.w, v.h, &v.thumb);
                        let word = slot_ui::shell_picker_word(&v.caption);
                        let caption = upload_rgba(
                            compositor,
                            &mut self.version_tex[i].1,
                            word.w,
                            word.h,
                            &word.rgba,
                        );
                        (v.path, (thumb, v.w, v.h), (caption, word.w))
                    })
                    .collect()
            });
            self.session.app_mut().versions_found(&rom, found);
            return;
        }
        let Some((cart, picked)) = self.session.app_mut().take_versions_ask() else {
            return;
        };
        let Some(root) = self.session.app().root().map(|r| r.to_path_buf()) else {
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("slot-label-versions".into())
            .spawn(move || {
                let mut downloader = slot_labels::Downloader::default();
                let versions = match picked {
                    Some(game) => downloader.versions_of(&cart, game),
                    None => downloader.versions(&cart),
                };
                let got = match versions {
                    Ok(versions) => Some(version_files(&root, &cart, versions)),
                    Err(slot_labels::Error::Unavailable(_) | slot_labels::Error::Image(_)) => {
                        Some(Vec::new())
                    }
                    Err(e) => {
                        eprintln!("slot: labels: {}: {e}", cart.stem);
                        None
                    }
                };
                let _ = tx.send((cart.rom, got));
            });
        match spawned {
            Ok(_) => self.versions_fetch = Some(rx),
            Err(e) => eprintln!("slot: labels: versions thread failed to start: {e}"),
        }
    }

    /// The shell picker asked for one cart's label: fetched on a thread of its own, as the
    /// background worker fetches them, and attached when it comes.
    fn fetch_asked_label(&mut self) {
        if let Some(rx) = &self.label_fetch {
            let (cart, got) = match rx.try_recv() {
                Ok(got) => got,
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.label_fetch = None;
                    return;
                }
            };
            self.label_fetch = None;
            let app = self.session.app_mut();
            let found = match got {
                LabelFetched::Found(path) => {
                    app.attach_label(&cart.rom, path);
                    Some(true)
                }
                LabelFetched::Missing => Some(false),
                LabelFetched::Failed => None,
            };
            app.label_fetched(&cart.rom, found);
            return;
        }
        let Some(cart) = self.session.app_mut().take_label_ask() else {
            return;
        };
        let Some(root) = self.session.app().root().map(|r| r.to_path_buf()) else {
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("slot-label-ask".into())
            .spawn(move || {
                let got = match slot_labels::Downloader::default().prepare(&root, &cart) {
                    Ok(path) => {
                        slot_store::forget_no_label(&root, cart.platform, &cart.stem);
                        LabelFetched::Found(path)
                    }
                    Err(slot_labels::Error::Unavailable(_) | slot_labels::Error::Image(_)) => {
                        slot_store::note_no_label(&root, cart.platform, &cart.stem);
                        LabelFetched::Missing
                    }
                    Err(e) => {
                        eprintln!("slot: labels: {}: {e}", cart.stem);
                        LabelFetched::Failed
                    }
                };
                let _ = tx.send((cart, got));
            });
        match spawned {
            Ok(_) => self.label_fetch = Some(rx),
            Err(e) => eprintln!("slot: labels: fetch thread failed to start: {e}"),
        }
    }

    fn check_labels(
        root: &std::path::Path,
        carts: Vec<slot_store::Cart>,
    ) -> Option<std::sync::mpsc::Receiver<Vec<slot_store::Cart>>> {
        let (tx, rx) = std::sync::mpsc::channel();
        let root = root.to_path_buf();
        std::thread::Builder::new()
            .name("slot-label-check".into())
            .spawn(move || {
                let _ = tx.send(label_cache::stale(&root, carts.iter()));
            })
            .ok()
            .map(|_| rx)
    }

    fn poll_label_check(&mut self) {
        let Some(rx) = &self.stale_check else {
            return;
        };
        let stale = match rx.try_recv() {
            Ok(stale) => stale,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Vec::new(),
        };
        self.stale_check = None;
        let Some(root) = self.session.app().root().map(|r| r.to_path_buf()) else {
            return;
        };
        if !stale.is_empty() {
            eprintln!("slot: label cache: {} new labels", stale.len());
            self.rebuild = Some(Rebuild::start(&root, stale));
        }
    }

    /// New labels are cached on worker threads while the shelf is in use; they wait
    /// while a game runs. On the shelf a banner along the top counts them.
    fn poll_rebuild(&mut self) {
        self.poll_label_check();
        let Some(rebuild) = &self.rebuild else {
            return;
        };
        if rebuild.finished() {
            // A handful done before the banner ever showed needs no word that they are.
            if self.rebuild_faces.shown.is_some() {
                self.session.app_mut().say_labels_ready();
            }
            self.rebuild = None;
            self.rebuild_faces.shown = None;
            return;
        }
        rebuild.set_paused(!self.session.app().labels_may_cache());
    }

    fn draw_rebuild(&mut self, compositor: &mut Compositor) {
        let Some(rebuild) = &self.rebuild else {
            return;
        };
        if !self.session.app().labels_banner_shown()
            || rebuild.started().elapsed() < REBUILD_BANNER_AFTER
        {
            return;
        }
        let done = rebuild.done();
        let faces = &mut self.rebuild_faces;
        if faces.shown != Some(done) {
            let f = rebuild_banner_face(&format!("Caching Labels  {done} / {}", rebuild.total()));
            upload_rgba(compositor, &mut faces.line, f.w, f.h, &f.rgba);
            faces.shown = Some(done);
        }
        let banner = RebuildBanner {
            line: faces.line,
            fraction: done as f32 / rebuild.total().max(1) as f32,
        };
        self.draws.clear();
        banner.draw(self.session.app().canvas(), &mut self.draws);
        compositor.draw_list(&self.draws);
    }

    pub fn advance(&mut self, input: &mut dyn InputSource) {
        let now = self.now();
        let events = input.poll(now);
        self.session
            .set_menu_dpad_display(input.menu_dpad_display());
        self.session.feed(events, now);
        let dt = self.last.elapsed().as_secs_f32();
        self.last = Instant::now();
        self.session.update(dt);
        self.fetch_once_online();
    }

    /// Cheat files and labels come only from online: their workers start the first time Home
    /// Wi-Fi is connected, whether that is shortly after boot or after it is switched on in
    /// the menu. They give up on a network that keeps failing, so every time the connection
    /// comes back (after the deep sleep, or Wi-Fi off and on) fresh ones start for the carts
    /// still without a label or a cheat look-up.
    fn fetch_once_online(&mut self) {
        let app = self.session.app();
        let online = app.home_wifi_on() && app.home_connected();
        let back = online && !self.was_online;
        self.was_online = online;
        if !back {
            return;
        }
        let Some(root) = app.root().map(|r| r.to_path_buf()) else {
            return;
        };
        let carts: Vec<_> = app.carts().cloned().collect();
        self.labels = crate::labels::Labels::spawn(root.clone(), carts.clone());
        self.cheat_fetch = Some(crate::cheat_fetch::CheatFetch::spawn(root, carts));
    }

    pub fn app(&self) -> &crate::app::App {
        self.session.app()
    }

    pub fn core_settling(&self) -> bool {
        self.session.core_settling()
    }

    pub fn advance_at(&mut self, input: &mut dyn InputSource, now: Millis, dt: f32) {
        let events = input.poll(now);
        self.session
            .set_menu_dpad_display(input.menu_dpad_display());
        self.session.feed(events, now);
        self.session.update(dt);
        self.fetch_once_online();
    }

    fn now(&self) -> Millis {
        self.start.elapsed().as_millis() as Millis
    }

    pub fn powering_off(&self) -> bool {
        self.session.app().ready_to_power_off()
    }

    /// The light sleep: the screen is off and nothing needs drawing.
    pub fn dozing(&self) -> bool {
        matches!(self.session.app().phase(), Phase::Doze { .. })
    }

    pub fn restarting(&self) -> bool {
        self.session.app().ready_to_restart()
    }

    pub fn restart(&mut self) {
        self.leave_boot_picture();
        self.session.app_mut().restart();
    }

    pub fn poweroff(&mut self) {
        self.leave_boot_picture();
        self.session.app_mut().poweroff();
    }

    /// Shutting down in a game: its picture becomes the boot screen.
    fn leave_boot_picture(&mut self) {
        let app = self.session.app_mut();
        let Some((rgba, w, h)) = app.take_boot_picture() else {
            return;
        };
        if let Some(root) = app.root().map(|r| r.to_path_buf()) {
            crate::bootlogo::install_picture(&root, &rgba, (w as usize, h as usize));
        }
    }

    /// For previews: the picture a shutdown would leave as the boot screen.
    pub fn boot_picture(&mut self) -> Option<(Vec<u8>, u32, u32)> {
        self.session.app_mut().take_boot_picture()
    }

    pub fn stop(&mut self) {
        self.session.app_mut().flush_resume();
        self.session.app_mut().settle_saves();
        self.session.app_mut().clear_loading_mark();
    }
}

fn link_legend_faces(ink: [u8; 3]) -> Vec<UndoFace> {
    vec![
        hint_coloured("B", "Cancel", ink),
        hint_coloured("SELECT", "Mode", ink),
        arrows_hint_coloured("Swap", ink),
        hint_coloured("A", "Link", ink),
        hint_coloured("A", "OK", ink),
        hint_coloured("B", "Back", ink),
        hint_coloured("A", "End Link", ink),
    ]
}

fn legend_faces(compositor: &mut Compositor, legend: &[(&str, &str)]) -> Vec<TexId> {
    legend
        .iter()
        .map(|(key, label)| {
            let f = hint_face(key, label);
            compositor.create_texture(f.w, f.h, &f.rgba)
        })
        .collect()
}

struct UiFaces {
    link_badges: Vec<CartFace>,
    quick_labels: Vec<[UndoFace; 2]>,
    quick_pointer: UndoFace,
    quick_values: Vec<[UndoFace; 2]>,
    quick_carets: [UndoFace; 2],
    quick_more: [UndoFace; 2],
    quick_legend: [UndoFace; 5],
    quick_switches: [UndoFace; 2],
    quick_help: Vec<Option<UndoFace>>,
    button_tabs: Vec<UndoFace>,
    button_bodies: Vec<UndoFace>,
    button_arrows: [UndoFace; 2],
    button_legend: [UndoFace; 2],
    sockets: Vec<CartFace>,
    chips: Vec<CartFace>,
    blank_chip: CartFace,
    chip_shadow: CartFace,
    core_legend: Vec<UndoFace>,
    shell_shapes: [UndoFace; 3],
    shell_words: Vec<UndoFace>,
    shell_legend: Vec<UndoFace>,
    link_roles: Vec<UndoFace>,
    linked: UndoFace,
    link_legend: Vec<UndoFace>,
    link_legend_light: Vec<UndoFace>,
    link_steps: Vec<UndoFace>,
    link_fails: Vec<UndoFace>,
    toasts: Vec<CartFace>,
}

impl UiFaces {
    fn build() -> Self {
        UiFaces {
            link_badges: LinkBadge::FACES
                .iter()
                .map(|b| {
                    let (badge, ink) = (
                        b.badge().expect("a face has a glyph"),
                        b.colour().expect("and a colour"),
                    );
                    badge_face(badge, HUD_ICON_PX, ink)
                })
                .collect(),
            quick_labels: QuickRow::ALL
                .iter()
                .map(|r| [false, true].map(|lit| quick_label_face(*r, lit)))
                .collect(),
            quick_pointer: quick_pointer_face(),
            quick_values: QuickValue::ALL
                .iter()
                .map(|v| [false, true].map(|lit| quick_value_face(v.text(), lit)))
                .collect(),
            quick_carets: [false, true].map(quick_caret_face),
            quick_more: [
                quarter_turn(quick_caret_face(true), false),
                quarter_turn(quick_caret_face(true), true),
            ],
            quick_legend: quick_legend_faces(),
            quick_switches: [false, true].map(slot_ui::quick_switch_face),
            quick_help: QuickRow::ALL
                .iter()
                .map(|r| slot_ui::quick_help_face(*r))
                .collect(),
            button_tabs: (0..slot_ui::BUTTON_PAGES.len())
                .map(slot_ui::buttons_tab_face)
                .collect(),
            // The menus' own carets, so the pages turn with the same arrows.
            button_arrows: [false, true].map(quick_caret_face),
            button_bodies: (0..slot_ui::BUTTON_PAGES.len())
                .map(slot_ui::buttons_body_face)
                .collect(),
            button_legend: [
                slot_ui::tint_mask(arrows_hint_face("Page")),
                slot_ui::tint_mask(hint_face("B", "Back")),
            ],
            sockets: slot_store::Core::ALL
                .iter()
                .map(|c| socket_face(*c))
                .collect(),
            chips: slot_store::Core::ALL
                .iter()
                .map(|c| chip_face(Some(*c)))
                .collect(),
            blank_chip: chip_face(None),
            chip_shadow: chip_shadow_face(),
            core_legend: vec![
                hint_face("B", "Cancel"),
                arrows_hint_face("Swap"),
                hint_face("A", "Choose"),
            ],
            shell_shapes: slot_ui::shell_picker_shapes(),
            // The swatches' names, the finishes, the shapes, the tabs, the label tab's states, then
            // the markers' names.
            shell_words: crate::shell_picker::swatch_names()
                .into_iter()
                .chain(crate::shell_picker::FINISH_NAMES.map(|(_, name)| name))
                .chain(MODEL_WORDS)
                .chain(["Color", "Label", "Marker", "Looking", "None Found"])
                .chain(crate::shell_picker::MARKERS.map(|(name, _)| name))
                .map(slot_ui::shell_picker_word)
                .collect(),
            shell_legend: vec![
                slot_ui::tint_mask(hint_face("B", "Cancel")),
                slot_ui::tint_mask(arrows_hint_face("Change")),
                slot_ui::tint_mask(hint_face("A", "Save")),
                // Taken off as the confirm legend, then the remove hint, shown only for a cart
                // wearing a label.
                slot_ui::tint_mask(hint_face("B", "Keep Label")),
                slot_ui::tint_mask(hint_face("A", "Remove Label")),
                slot_ui::tint_mask(hint_face("Y", "Remove Label")),
                slot_ui::tint_mask(hint_face("Y", "Search")),
            ],
            link_roles: LinkRow::ALL.iter().map(|r| menu_face(r.text())).collect(),
            linked: menu_face("Linked"),
            link_legend: link_legend_faces(footer_ink(false)),
            link_legend_light: link_legend_faces(footer_ink(true)),
            link_steps: LinkStep::ALL.iter().map(|s| menu_face(s.line())).collect(),
            link_fails: LinkFail::SHOWN
                .iter()
                .map(|f| menu_face(f.line()))
                .collect(),
            toasts: Toast::all().into_iter().map(toast_face).collect(),
        }
    }
}

struct Faces<'a> {
    pool: &'a mut Vec<TexId>,
    title: &'a mut Option<TexId>,
}

fn sync_switcher(app: &mut App, compositor: &mut Compositor, texes: Faces, state: &mut Switcher) {
    if !matches!(app.phase(), Phase::Polaroids { .. }) {
        state.open = false;
        return;
    }
    if !state.open {
        state.open = true;
        state.titled = None;
        let faces: Vec<_> = app.polaroid_entries().iter().map(photo_face).collect();
        let ids = faces
            .iter()
            .enumerate()
            .map(|(i, f)| match texes.pool.get(i) {
                Some(id) => {
                    compositor.update_texture(*id, f.w, f.h, &f.rgba);
                    *id
                }
                None => {
                    let id = compositor.create_texture_nearest(f.w, f.h, &f.rgba);
                    texes.pool.push(id);
                    id
                }
            })
            .collect();
        app.set_polaroid_faces(ids);
    }
    if state.titled.as_deref() != app.polaroid_stamp() {
        state.titled = app.polaroid_stamp().map(str::to_string);
        let face = title_face(&app.polaroid_title(&format_stamp(app.wall_secs())));
        let id = upload(compositor, texes.title, face);
        app.set_polaroid_title_face(id);
    }
}

fn sync_clock(app: &mut App, compositor: &mut Compositor, clocks: &mut Clocks) {
    let ink = app.status_ink();
    if clocks.ink != ink {
        clocks.ink = ink;
        clocks.shown.clear();
        clocks.battery.clear();
    }
    let picked = app.picker().map(|p| p.text());
    if picked != clocks.picked {
        clocks.picked = picked;
        if let (Some(face), Some(hint)) = (app.picker().map(|p| p.face()), clocks.hint) {
            let line = upload(compositor, &mut clocks.line, face);
            app.set_clock_faces(line, hint);
        }
    }
    let shown = clock_text(app.wall_secs(), app.clock_24h());
    if shown != clocks.shown {
        let face = slot_ui::status_clock_face(&shown, clocks.ink);
        clocks.shown = shown;
        let w = face.w;
        let id = upload(compositor, &mut clocks.shelf, face);
        app.set_shelf_clock_face(id, w);
    }
    let platform_shown = app.slot_text().unwrap_or_default();
    if platform_shown != clocks.platform {
        clocks.platform = platform_shown.clone();
        if platform_shown.is_empty() {
            app.clear_shelf_platform();
        } else {
            let face = word_ink(&platform_shown, [0xff, 0xff, 0xff]);
            let w = face.w;
            let id = upload(compositor, &mut clocks.platform_tex, face);
            app.set_shelf_platform_face(id, w);
        }
    }
    let title_shown = app.title_text().unwrap_or_default();
    if title_shown != clocks.title {
        clocks.title = title_shown.clone();
        if title_shown.is_empty() {
            app.clear_shelf_title();
        } else {
            let face = slot_ui::title_line_face(&title_shown);
            let w = face.w;
            let id = upload(compositor, &mut clocks.title_tex, face);
            app.set_shelf_title_face(id, w);
        }
    }
    let battery = app.battery();
    let battery_shown = battery
        .map(|b| format!("{}{:?}", b.percent, b.charge))
        .unwrap_or_default();
    if battery_shown != clocks.battery {
        clocks.battery = battery_shown.clone();
        if let Some(b) = battery {
            let charging = b.charge == slot_power::Charge::Charging;
            let face = slot_ui::battery_face(b.percent, charging, clocks.ink);
            let w = face.w;
            let id = upload(compositor, &mut clocks.battery_tex, face);
            app.set_battery_percent_face(id, w);
        }
    }
}

fn sync_game_bezel(app: &mut App, compositor: &mut Compositor, state: &mut BezelSync) {
    let platform = app.playing_platform();
    let pick = app.resolved_bezel(platform);
    let shown = (platform, pick);
    if state.image.as_ref() != Some(&shown) {
        let rgba = match &shown.1 {
            BezelPick::None | BezelPick::Ags102 => None,
            BezelPick::Default => default_game_bezel_face(platform == Console::Gbc),
            BezelPick::File(name) => app
                .root()
                .map(|root| {
                    root.join(crate::bezel::GAME_BEZELS)
                        .join(platform.dir_name())
                        .join(name)
                })
                .and_then(|path| game_bezel_face(&path)),
        };
        let tex = rgba.map(|rgba| match state.tex {
            Some(id) => {
                compositor.update_texture(id, OUT_W, OUT_H, &rgba);
                id
            }
            None => {
                let id = compositor.create_texture(OUT_W, OUT_H, &rgba);
                state.tex = Some(id);
                id
            }
        });
        app.set_game_bezel(tex);
        state.image = Some(shown);
    }
    if app.quick_menu().is_none() && app.in_game_menu().is_none() {
        return;
    }
    for (at, platform) in [Console::Gba, Console::Gb, Console::Gbc]
        .into_iter()
        .enumerate()
    {
        let label = app.resolved_bezel(platform).label();
        if state.labels[at] == label {
            continue;
        }
        let (dim, lit) = (
            quick_value_face(&label, false),
            quick_value_face(&label, true),
        );
        let (dim_size, lit_size) = ((dim.w, dim.h), (lit.w, lit.h));
        let dim = upload(compositor, &mut state.faces[at].0, dim);
        let lit = upload(compositor, &mut state.faces[at].1, lit);
        app.set_quick_bezel_faces(
            platform,
            (dim, dim_size.0, dim_size.1),
            (lit, lit_size.0, lit_size.1),
        );
        state.labels[at] = label;
    }
}

fn sync_wallpaper(app: &mut App, compositor: &mut Compositor, state: &mut WallpaperSync) {
    let shown = app.shown_wallpaper();
    if state.image != shown {
        state.image = shown.clone();
        if let Some(name) = &shown {
            if let Some(rgba) = app
                .root()
                .map(|root| root.join("Wallpapers").join(name))
                .and_then(|path| wallpaper_face(&path))
            {
                let id = match state.tex {
                    Some(id) => {
                        compositor.update_texture(id, OUT_W, OUT_H, &rgba);
                        id
                    }
                    None => {
                        let id = compositor.create_texture(OUT_W, OUT_H, &rgba);
                        state.tex = Some(id);
                        id
                    }
                };
                app.set_wallpaper(id);
            }
        }
    }
    if app.quick_menu().is_none() {
        return;
    }
    let label = app.resolved_wallpaper().label();
    if state.label == label {
        return;
    }
    let (dim, lit) = (
        quick_value_face(&label, false),
        quick_value_face(&label, true),
    );
    let (dim_size, lit_size) = ((dim.w, dim.h), (lit.w, lit.h));
    let dim = upload(compositor, &mut state.dim, dim);
    let lit = upload(compositor, &mut state.lit, lit);
    app.set_quick_wallpaper_faces((dim, dim_size.0, dim_size.1), (lit, lit_size.0, lit_size.1));
    state.label = label;
}

/// Connected To on the Network page: the home network's name, or that there is none.
fn sync_quick_network(app: &mut App, compositor: &mut Compositor, state: &mut QuickClock) {
    if app.quick_menu().is_none() {
        return;
    }
    let text = app
        .home_network()
        .unwrap_or_else(|| "Not connected".to_string());
    if text == state.shown {
        return;
    }
    let (dim, lit) = (
        quick_value_face(&text, false),
        quick_value_face(&text, true),
    );
    let (dim_size, lit_size) = ((dim.w, dim.h), (lit.w, lit.h));
    let dim = upload(compositor, &mut state.dim, dim);
    let lit = upload(compositor, &mut state.lit, lit);
    app.set_quick_network_faces((dim, dim_size.0, dim_size.1), (lit, lit_size.0, lit_size.1));
    state.shown = text;
}

/// Time Zone on the General page: the clock's city.
fn sync_quick_zone(app: &mut App, compositor: &mut Compositor, state: &mut QuickClock) {
    if app.quick_menu().is_none() {
        return;
    }
    let text = app.zone_name();
    if text == state.shown {
        return;
    }
    let (dim, lit) = (
        quick_value_face(&text, false),
        quick_value_face(&text, true),
    );
    let (dim_size, lit_size) = ((dim.w, dim.h), (lit.w, lit.h));
    let dim = upload(compositor, &mut state.dim, dim);
    let lit = upload(compositor, &mut state.lit, lit);
    app.set_quick_zone_faces((dim, dim_size.0, dim_size.1), (lit, lit_size.0, lit_size.1));
    state.shown = text;
}

fn sync_quick_clock(app: &mut App, compositor: &mut Compositor, state: &mut QuickClock) {
    if app.quick_menu().is_none() {
        return;
    }
    let text = date_time_text_in(app.wall_secs(), app.clock_24h());
    if text == state.shown {
        return;
    }
    let (dim, lit) = (
        quick_value_face(&text, false),
        quick_value_face(&text, true),
    );
    let (dim_size, lit_size) = ((dim.w, dim.h), (lit.w, lit.h));
    let dim = upload(compositor, &mut state.dim, dim);
    let lit = upload(compositor, &mut state.lit, lit);
    app.set_quick_clock_faces((dim, dim_size.0, dim_size.1), (lit, lit_size.0, lit_size.1));
    state.shown = text;
}

fn sync_quick_palette(app: &mut App, compositor: &mut Compositor, state: &mut QuickPalette) {
    let text = app.gb_palette_label();
    if text == state.shown {
        return;
    }
    let (dim, lit) = (quick_value_face(text, false), quick_value_face(text, true));
    let (dim_size, lit_size) = ((dim.w, dim.h), (lit.w, lit.h));
    let dim = upload(compositor, &mut state.dim, dim);
    let lit = upload(compositor, &mut state.lit, lit);
    app.set_quick_palette_faces((dim, dim_size.0, dim_size.1), (lit, lit_size.0, lit_size.1));
    state.shown = text;
}

/// A help-line face: the settings pages' small centred line, in white.
fn help_line_face(line: &str) -> slot_ui::UndoFace {
    use slot_ui::text;
    let Some(font) = text::label_font() else {
        return slot_ui::UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let layout = text::fit(font, line, 600.0, 1, 17.0, 12.0);
    let set = layout
        .lines
        .iter()
        .map(|l| text::line_width(font, l, layout.px, layout.tracking))
        .fold(0.0, f32::max);
    let (w, h) = ((set.ceil() as u32 + 4).min(600), 24);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    text::draw_centred(&mut rgba, w, h, &layout, [0xff, 0xff, 0xff]);
    slot_ui::UndoFace { rgba, w, h }
}

/// A cheat's name in the menu rows' type, shrinking to leave room for its switch.
fn cheat_name_face(name: &str) -> slot_ui::UndoFace {
    use slot_ui::text;
    let Some(font) = text::label_font() else {
        return slot_ui::UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    // Shrink to fit; past the smallest size, drop words from the end behind an ellipsis.
    let words: Vec<&str> = name.split_whitespace().collect();
    let mut layout = text::fit(font, name, slot_ui::NAME_MAX_W, 1, 26.0, 15.0);
    for n in (1..=words.len()).rev() {
        let candidate = match n == words.len() {
            true => words.join(" "),
            false => format!("{}…", words[..n].join(" ")),
        };
        let l = text::fit(font, &candidate, slot_ui::NAME_MAX_W, 1, 26.0, 15.0);
        let whole = l.lines.len() == 1 && l.lines[0] == candidate.to_uppercase();
        layout = l;
        if whole {
            break;
        }
    }
    let set = layout
        .lines
        .iter()
        .map(|l| text::line_width(font, l, layout.px, layout.tracking))
        .fold(0.0, f32::max);
    let reference = quick_value_face("Hg", false);
    let (w, h) = (
        (set.ceil() as u32 + 4)
            .min(slot_ui::NAME_MAX_W as u32 + 4)
            .max(1),
        reference.h,
    );
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    text::draw_centred(&mut rgba, w, h, &layout, [0xff, 0xff, 0xff]);
    slot_ui::UndoFace { rgba, w, h }
}

/// Turn a face a quarter: clockwise when `cw`.
fn quarter_turn(f: slot_ui::UndoFace, cw: bool) -> slot_ui::UndoFace {
    let (w, h) = (f.w, f.h);
    let mut rgba = vec![0u8; f.rgba.len()];
    for y in 0..h {
        for x in 0..w {
            let (nx, ny) = if cw { (h - 1 - y, x) } else { (y, w - 1 - x) };
            let from = ((y * w + x) * 4) as usize;
            let to = ((ny * h + nx) * 4) as usize;
            rgba[to..to + 4].copy_from_slice(&f.rgba[from..from + 4]);
        }
    }
    slot_ui::UndoFace { rgba, w: h, h: w }
}

fn sync_cheats(app: &mut App, compositor: &mut Compositor, state: &mut CheatTextures) {
    if state.badges[0].is_none() {
        let menu = slot_ui::cheat_badge_face(slot_ui::BADGE_MENU_H);
        let game = slot_ui::cheat_badge_face(slot_ui::BADGE_GAME_H);
        let (mw, mh, gw, gh) = (menu.w, menu.h, game.w, game.h);
        let m = upload(compositor, &mut state.badges[0], menu);
        let g = upload(compositor, &mut state.badges[1], game);
        app.set_cheat_badges([(m, mw, mh), (g, gw, gh)]);
    }
    let rev = app.cheats_rev();
    let counts = app.cheat_counts();
    if state.rev != Some(rev) {
        state.rev = Some(rev);
        state.counts = None;
        let names = app.cheat_names();
        if state.names.len() < names.len() {
            state.names.resize(names.len(), None);
        }
        let mut faces = Vec::with_capacity(names.len());
        for (i, name) in names.iter().enumerate() {
            let f = cheat_name_face(name);
            let (w, h) = (f.w, f.h);
            faces.push((upload(compositor, &mut state.names[i], f), w, h));
        }
        let mut up = |slot: &mut Option<TexId>, f: slot_ui::UndoFace| {
            let (w, h) = (f.w, f.h);
            (upload(compositor, slot, f), w, h)
        };
        let empty = up(
            &mut state.empty,
            quick_value_face("No cheats for this game", false),
        );
        let line = format!(
            "Put a .cht file named like this game in Cheats/{}.",
            app.cheat_platform().dir_name()
        );
        let empty_help = up(&mut state.empty_help, help_line_face(&line));
        let off = up(&mut state.switches[0], slot_ui::quick_switch_face(false));
        let on = up(&mut state.switches[1], slot_ui::quick_switch_face(true));
        let more_up = up(
            &mut state.more[0],
            quarter_turn(quick_caret_face(true), false),
        );
        let more_down = up(
            &mut state.more[1],
            quarter_turn(quick_caret_face(true), true),
        );
        let mut hint = |slot: &mut Option<TexId>, key: &str, label: &str| {
            let f = slot_ui::tint_mask(hint_face(key, label));
            let w = f.w;
            (upload(compositor, slot, f), w)
        };
        let [l0, l1, l2] = &mut state.legend;
        let back = hint(l0, "B", "Back");
        let toggle = hint(l1, "A", "Toggle");
        let all_off = hint(l2, "Y", "All Off");
        app.set_cheat_faces(slot_ui::CheatFaces {
            names: faces,
            help: None,
            empty,
            empty_help,
            legend: vec![back, toggle, all_off],
            legend_empty: vec![back],
            switches: [off, on],
            more: [more_up, more_down],
        });
    }
    if state.counts != Some(counts) {
        state.counts = Some(counts);
        let (on, total) = counts;
        let line = match on {
            0 => format!("{total} cheats. A turns one on, right away."),
            _ => format!("{on} of {total} cheats on. Changes apply right away."),
        };
        let f = help_line_face(&line);
        let (w, h) = (f.w, f.h);
        let id = upload(compositor, &mut state.help, f);
        if let Some(faces) = app.cheat_faces_mut() {
            faces.help = Some((id, w, h));
        }
    }
}

/// A line of white text at `px`, fitted into `max_w`.
fn text_face(line: &str, px: f32, max_w: f32) -> slot_ui::UndoFace {
    use slot_ui::text;
    let Some(font) = text::label_font() else {
        return slot_ui::UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let layout = text::fit(font, line, max_w, 1, px, 12.0);
    let set = layout
        .lines
        .iter()
        .map(|l| text::line_width(font, l, layout.px, layout.tracking))
        .fold(0.0, f32::max);
    let (w, h) = (
        (set.ceil() as u32 + 4).min(max_w as u32 + 4),
        (px * 1.5).ceil() as u32,
    );
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    text::draw_centred(&mut rgba, w, h, &layout, [0xff, 0xff, 0xff]);
    slot_ui::UndoFace { rgba, w, h }
}

fn read_png(path: &std::path::Path) -> Option<(Vec<u8>, u32, u32)> {
    let file = std::fs::File::open(path).ok()?;
    let mut dec = png::Decoder::new(std::io::BufReader::new(file));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let px = (w * h) as usize;
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..px * 4].to_vec(),
        png::ColorType::Rgb => buf[..px * 3]
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::Grayscale => buf[..px].iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::GrayscaleAlpha => buf[..px * 2]
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        _ => return None,
    };
    Some((rgba, w, h))
}

fn build_shot_faces(app: &mut App, compositor: &mut Compositor, st: &mut ShotTextures) {
    st.built = true;
    {
        let mut up = |slot: &mut Option<TexId>, f: slot_ui::UndoFace| {
            let (w, h) = (f.w, f.h);
            (upload(compositor, slot, f), w, h)
        };
        let [f0, f1, f2] = &mut st.fixed;
        let confirm = up(f0, help_line_face("Delete this screenshot?"));
        let empty = up(f1, quick_value_face("No screenshots yet", false));
        let empty_help = up(
            f2,
            help_line_face("Press VOL+ and POWER together to take one."),
        );
        let mut hint = |slot: &mut Option<TexId>, f: slot_ui::UndoFace| {
            let f = slot_ui::tint_mask(f);
            let w = f.w;
            (upload(compositor, slot, f), w)
        };
        let [h0, h1, h2, h3, h4, h5, h6] = &mut st.hints;
        let back = hint(h0, hint_face("B", "Back"));
        let view_ = hint(h1, hint_face("A", "View"));
        let delete = hint(h2, hint_face("Y", "Delete"));
        let cancel = hint(h3, hint_face("B", "Cancel"));
        let sure = hint(h4, hint_face("A", "Delete"));
        let close = hint(h5, hint_face("B", "Close"));
        let flick = hint(h6, slot_ui::arrows_hint_face("Flick"));
        let f = slot_ui::tint_mask(help_line_face("Delete this save state?"));
        let (lw, lh) = (f.w, f.h);
        let line = upload(compositor, &mut st.state_line, f);
        app.set_state_confirm_faces(slot_ui::ConfirmFaces {
            cancel,
            delete: sure,
            line: (line, lw, lh),
        });
        app.set_shot_faces(slot_ui::ShotFaces {
            title: None,
            date: None,
            confirm,
            empty,
            empty_help,
            legend: vec![back, view_, delete],
            legend_empty: vec![back],
            legend_confirm: vec![cancel, sure],
            full_left: vec![close, delete],
            full_right: vec![flick],
        });
    }
}

/// The Game Boy's pixels cast a stronger shadow on its pale panel than the GBA's or GBC's.
/// The Game Boy Color's unlit, reflective panel: what white becomes, and the tone light colours
/// take on. A warm grey, like the glass of a GBC held in daylight.
const GBC_PANEL: [f32; 3] = [0.79, 0.80, 0.74];
/// How much of the last picture a TV's phosphor still glows with, out of 256: brief, a
/// trace behind what moves fast.
const TV_AFTERGLOW: u32 = 108;

/// Grid lines a little darker than the panel, as the gaps between a GBC's pixels look.
/// The GBC's LCD on its own pixels: a dark line under each row at 10%, and a red, green or blue
/// line down each third of a pixel at 20%.
const GBC_LCD: [f32; 2] = [0.10, 0.20];

/// A GBA's colours shown a quarter less vivid than the raw ones: between the raw colours and
/// the old colour correction, as the screen showed them.
const GBA_SATURATION: f32 = 0.75;
/// A GBC's colours, a touch softer than the raw ones.
const GBC_SATURATION: f32 = 0.85;

/// How dark, how wide, and soft-edged.
/// The GBC's and GBA's pixels let less light through than the Game Boy's and cast lighter
/// shadows, so white lettering on dark ground stays white.
const COLOUR_PIXEL_SHADOW: [f32; 3] = [0.28, 1.1, 1.0];
const COLOUR_PALE_OPACITY: f32 = 0.80;
const GB_PALE_OPACITY: f32 = slot_gfx::PALE_OPACITY;

const GB_PIXEL_SHADOW: [f32; 3] = [0.56, 1.1, 1.0];

fn sync_shots(app: &mut App, compositor: &mut Compositor, st: &mut ShotTextures) {
    if !st.built {
        build_shot_faces(app, compositor, st);
    }
    let Some(view) = app.shot_view() else {
        st.key = None;
        return;
    };
    let key = (view.rev, view.sel);
    let title = view.title();
    let date = view.selected().map(|s| crate::shots::when(s.secs));
    // The pictures to have: the chosen one first, then its neighbours either side.
    let mut wanted: Vec<usize> = match view.shots.is_empty() {
        true => Vec::new(),
        false => vec![view.sel],
    };
    for i in [
        view.sel.wrapping_sub(1),
        view.sel + 1,
        view.pos.floor() as usize,
        view.pos.ceil() as usize,
    ] {
        if i < view.shots.len() && !wanted.contains(&i) {
            wanted.push(i);
        }
    }
    let paths: Vec<(usize, std::path::PathBuf)> = wanted
        .iter()
        .map(|&i| (i, view.shots[i].path.clone()))
        .collect();
    if st.key != Some(key) {
        st.key = Some(key);
        let t = (!title.is_empty()).then(|| {
            let f = text_face(&title, 20.0, 380.0);
            let (w, h) = (f.w, f.h);
            (upload(compositor, &mut st.title, f), w, h)
        });
        let d = date.map(|line| {
            let f = help_line_face(&line);
            let (w, h) = (f.w, f.h);
            (upload(compositor, &mut st.date, f), w, h)
        });
        if let Some(faces) = app.shot_faces_mut() {
            faces.title = t;
            faces.date = d;
        }
    }
    // Load at most one new picture a frame, so flicking never stalls.
    let mut loaded = false;
    let mut pics = Vec::new();
    for (i, path) in &paths {
        if let Some(at) = st.pics.iter().position(|(p, _)| p == path) {
            let entry = st.pics.remove(at);
            pics.push((*i, entry.1));
            st.pics.push(entry);
            continue;
        }
        if loaded {
            continue;
        }
        loaded = true;
        let Some((rgba, w, h)) = read_png(path) else {
            continue;
        };
        let keep: Vec<&std::path::PathBuf> = paths.iter().map(|(_, p)| p).collect();
        let tex = if st.pics.len() >= 5 {
            let at = st
                .pics
                .iter()
                .position(|(p, _)| !keep.contains(&p))
                .unwrap_or(0);
            let (_, tex) = st.pics.remove(at);
            compositor.update_texture(tex, w, h, &rgba);
            tex
        } else {
            compositor.create_texture(w, h, &rgba)
        };
        st.pics.push((path.clone(), tex));
        pics.push((*i, tex));
    }
    app.set_shot_pics(pics);
}

fn sync_about(app: &mut App, compositor: &mut Compositor, state: &mut AboutFace) {
    if !matches!(app.phase(), Phase::About) {
        return;
    }
    let light = app.about_light();
    if state.tex.is_some() && state.light == Some(light) {
        return;
    }
    state.light = Some(light);
    let build = Build::current();
    let serial = format!("{} {}", build.serial(), build.dirty_digit());
    let face = slot_ui::about_face(light, &serial);
    let id = upload(compositor, &mut state.tex, face);
    app.set_sticker_face(id);
}

/// The textures the core picker reuses from one cart to the next.
#[derive(Default)]
struct CoreTextures {
    interior: Option<TexId>,
    board: Option<TexId>,
    lid: Option<TexId>,
}

fn sync_core_picker(
    app: &mut App,
    compositor: &mut Compositor,
    builder: &FaceBuilder,
    asked: &mut Option<String>,
    tex: &mut CoreTextures,
    built: &mut Option<String>,
) {
    let highlighted = app.selected_stem().map(str::to_string);
    if highlighted.is_some() && *asked != highlighted {
        if let Some(cart) = app
            .carts()
            .find(|c| highlighted.as_deref() == Some(c.stem.as_str()))
        {
            builder.request(cart.clone());
        }
        *asked = highlighted.clone();
    }
    let Some(faces) = builder.take() else {
        return;
    };
    if highlighted.as_deref() != Some(faces.stem.as_str()) || *built == highlighted {
        return;
    }
    let interior_id = upload_rgba(
        compositor,
        &mut tex.interior,
        faces.interior.w,
        faces.interior.h,
        &faces.interior.rgba,
    );
    app.set_core_interior_face(interior_id);
    let board_id = upload_rgba(
        compositor,
        &mut tex.board,
        faces.board.w,
        faces.board.h,
        &faces.board.rgba,
    );
    let lid_id = upload_rgba(
        compositor,
        &mut tex.lid,
        faces.lid.w,
        faces.lid.h,
        &faces.lid.rgba,
    );
    app.set_core_board_faces(board_id, lid_id);
    *built = Some(faces.stem);
}

/// The label search's textures: its keyboard's, made once, and its words', made again as they
/// change.
#[derive(Default)]
struct SearchTextures {
    heading: Option<TexId>,
    typed: Option<TexId>,
    status: Option<TexId>,
    rows: Vec<(Option<TexId>, Option<TexId>)>,
}

fn sync_label_search(app: &mut App, compositor: &mut Compositor, state: &mut SearchTextures) {
    if !app.label_search_open() {
        return;
    }
    if !app.has_search_faces() {
        let faces = slot_ui::KeyboardFaces::build()
            .map(|f| (compositor.create_texture(f.w, f.h, &f.rgba), f.w, f.h));
        app.set_search_faces(faces);
    }
    let Some((rev, words)) = app.search_words() else {
        return;
    };
    let mut up = |slot: &mut Option<TexId>, f: slot_ui::UndoFace| {
        let (w, h) = (f.w, f.h);
        (upload(compositor, slot, f), w, h)
    };
    let heading = up(
        &mut state.heading,
        slot_ui::search_heading_face(&words.heading),
    );
    let typed = words
        .typed
        .map(|t| up(&mut state.typed, slot_ui::search_typed_face(&t)));
    let status = words
        .status
        .map(|t| up(&mut state.status, slot_ui::search_status_face(&t)));
    if state.rows.len() < words.rows.len() {
        state.rows.resize(words.rows.len(), (None, None));
    }
    let rows = words
        .rows
        .iter()
        .zip(state.rows.iter_mut())
        .map(|((name, platform), (a, b))| {
            let (n, p) = slot_ui::search_row_faces(name, platform);
            (up(a, n), up(b, p))
        })
        .collect();
    app.set_search_texts(
        rev,
        slot_ui::SearchTexts {
            heading,
            typed,
            status,
            rows,
        },
    );
}

fn upload(compositor: &mut Compositor, slot: &mut Option<TexId>, face: slot_ui::UndoFace) -> TexId {
    upload_rgba(compositor, slot, face.w, face.h, &face.rgba)
}

/// The versions written where the label tab keeps them, each with a thumbnail and a caption:
/// where it comes from, and which of how many it is.
fn version_files(
    root: &std::path::Path,
    cart: &slot_store::Cart,
    versions: Vec<slot_labels::Version>,
) -> Vec<VersionFile> {
    let dir = root.join(VERSIONS_DIR);
    let _ = std::fs::create_dir_all(&dir);
    let (lw, lh) = slot_ui::label_size(cart);
    let (w, h) = (lw * THUMB_H / lh.max(1), THUMB_H);
    // An SNES cart's label tab offers only the labels made for its shape, so each is counted
    // among those of its own shape: the wide PAL and Super Famicom ones, or the taller
    // American ones.
    let shape = |png: &[u8]| {
        let size = |at: usize| {
            png.get(at..at + 4)
                .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
        };
        match (cart.platform, size(16), size(20)) {
            (slot_store::Platform::Snes, Some(w), Some(h)) => {
                Some(w as f32 / h.max(1) as f32 > 2.45)
            }
            _ => None,
        }
    };
    let shapes: Vec<_> = versions.iter().map(|v| shape(&v.png)).collect();
    versions
        .into_iter()
        .enumerate()
        .filter_map(|(i, v)| {
            let n = shapes.iter().filter(|s| **s == shapes[i]).count();
            let at = shapes[..i].iter().filter(|s| **s == shapes[i]).count();
            let path = dir.join(format!("{i}.png"));
            if let Err(e) = slot_store::atomic_write(&path, &v.png) {
                eprintln!("slot: labels: {}: {e}", path.display());
                return None;
            }
            let thumb = slot_ui::cover(&path, w, h)?;
            let region = match v.region.as_str() {
                "" => "Version",
                "North America" | "United States" => "USA",
                region => region,
            };
            let kind = if v.render { "Render" } else { "Photo" };
            Some(VersionFile {
                path,
                caption: format!("{region} · {kind} · {} of {n}", at + 1),
                thumb,
                w,
                h,
            })
        })
        .collect()
}

fn upload_rgba(
    compositor: &mut Compositor,
    slot: &mut Option<TexId>,
    w: u32,
    h: u32,
    rgba: &[u8],
) -> TexId {
    match *slot {
        Some(id) => {
            compositor.update_texture(id, w, h, rgba);
            id
        }
        None => {
            let id = compositor.create_texture(w, h, rgba);
            *slot = Some(id);
            id
        }
    }
}
