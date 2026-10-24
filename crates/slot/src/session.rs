use std::path::PathBuf;
use std::time::Duration;

use slot_input::{Action, Btn, Gestures, Millis, RawEvent};
use slot_retro::Rumble;
use slot_store::Platform;
use slot_ui::FfState;

pub const RUMBLE_MIN_MS: Millis = 100;
pub const RUMBLE_MAX_MS: Millis = 150;

fn gambatte_boot_rom_status(root: &std::path::Path, platform: Platform) -> Option<String> {
    let (name, expected_size, expected_crc) = match platform {
        Platform::Gb => ("gb_bios.bin", 0x100usize, 0x59C8_598E),
        Platform::Gbc => ("gbc_bios.bin", 0x900usize, 0x4188_4E46),
        Platform::Gba | Platform::Nes | Platform::Snes => return None,
    };
    let path = root.join("BIOS").join(name);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => return Some(format!("missing at {}", path.display())),
    };
    if bytes.len() != expected_size {
        return Some(format!(
            "wrong size ({} bytes; expected {expected_size}) at {}",
            bytes.len(),
            path.display()
        ));
    }
    let checksum = crc32(&bytes);
    if checksum != expected_crc {
        return Some(format!(
            "checksum mismatch (CRC32 {checksum:08X}; expected {expected_crc:08X}) at {}",
            path.display()
        ));
    }
    Some(format!("verified official boot ROM at {}", path.display()))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

use crate::app::{App, Phase};
use crate::audio::{opened_sink, AudioSink, Ring, Sfx};
use crate::core::open_core;
use crate::emu::{CoreState, EmuHandle, Speed};
use crate::frames::FrameRef;
use crate::input::Pad;
use crate::persist;
use crate::prefetch::Prefetch;

pub struct Session {
    root: PathBuf,
    app: App,
    emu: Option<EmuHandle>,
    sink: Box<dyn AudioSink>,
    gestures: Gestures,
    pad: Pad,
    rewinding: bool,
    fast: bool,
    /// The buttons down right now, lid and POWER aside: holding one is not being idle.
    held: Vec<Btn>,
    motor: u16,
    pulse: Option<(Millis, u16)>,
    prefetch: Prefetch,
    reloading: bool,
    driven: bool,
    sgb_turn: u32,
    /// The Game Boy's game shows in the core's own grey: every screen effect is off.
    grey: bool,
}

impl Session {
    pub fn boot(root: PathBuf) -> Self {
        Self::boot_with(root, opened_sink())
    }

    pub fn boot_with(root: PathBuf, sink: Box<dyn AudioSink>) -> Self {
        crate::core::warm_cores(&root);
        let mut session = Session {
            app: App::boot(&root),
            root,
            emu: None,
            sink,
            gestures: Gestures::new(),
            pad: Pad::default(),
            rewinding: false,
            fast: false,
            held: Vec::new(),
            motor: 0,
            pulse: None,
            prefetch: Prefetch::default(),
            reloading: false,
            driven: false,
            sgb_turn: 0,
            grey: false,
        };
        // A game left running comes straight back. Its core loads on its own thread, so start it
        // now and let the card read overlap the rest of the boot. Waiting for the first update
        // left a cold boot (after a power off) idle on the slot for the whole load.
        if session.inserting() {
            session.sync_core();
        }
        session
    }

    pub fn play_sfx(&mut self, sfx: Sfx) {
        let ring = self.sink.ring();
        let rate = ring.sample_rate();
        if rate == 0 {
            return;
        }
        let mut samples = sfx.render(rate);
        crate::audio::volume::apply(&mut samples, self.app.output_volume());
        ring.mix(&samples);
    }

    pub fn audio_queued(&self) -> usize {
        self.sink.ring().queued_frames()
    }

    pub fn audio_ring(&self) -> std::sync::Arc<Ring> {
        self.sink.ring()
    }

    pub fn rumble(&mut self, strength: u16) {
        if strength == self.motor {
            return;
        }
        self.motor = strength;
        self.app.set_rumble(strength);
    }

    pub fn prefetching(&self) -> Option<&std::path::Path> {
        self.prefetch.reading()
    }

    pub fn core_rumble(&self) -> Option<&Rumble> {
        self.emu.as_ref().map(EmuHandle::rumble)
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    pub fn emu(&self) -> Option<&EmuHandle> {
        self.emu.as_ref()
    }

    pub fn set_driven(&mut self, driven: bool) {
        self.driven = driven;
        if let Some(emu) = &self.emu {
            emu.set_driven(driven);
        }
    }

    pub fn step_emulator(&self, present: Duration, timeout: Duration) -> bool {
        self.emu.as_ref().is_some_and(|emu| {
            emu.tick(present);
            emu.wait_frame(timeout)
        })
    }

    pub fn frame(&self) -> Option<FrameRef> {
        self.emu.as_ref().and_then(|e| e.latest_frame())
    }

    pub fn core_settling(&self) -> bool {
        self.emu
            .as_ref()
            .is_some_and(|e| e.state() == CoreState::Loading)
    }

    pub fn has_core(&self) -> bool {
        self.emu.is_some()
    }

    pub fn frame_ready(&self) -> bool {
        self.emu.as_ref().is_some_and(EmuHandle::frame_ready)
    }

    pub fn frames_published(&self) -> u64 {
        self.emu.as_ref().map_or(0, EmuHandle::published_count)
    }

    pub fn observed_speed(&self) -> Option<Speed> {
        self.emu.as_ref().map(EmuHandle::observed_speed)
    }

    pub fn frames_taken(&self) -> u64 {
        self.emu.as_ref().map_or(0, EmuHandle::frames_taken)
    }

    pub fn game_visible(&self) -> bool {
        self.app.game_visible()
    }

    pub fn feed(&mut self, events: impl IntoIterator<Item = RawEvent>, now: Millis) {
        let mut actions = Vec::new();
        let latched = self.gestures.ff_latched();
        for ev in events {
            match ev {
                RawEvent::Down(Btn::Lid | Btn::Power) | RawEvent::Up(Btn::Lid | Btn::Power) => {}
                RawEvent::Down(b) if !self.held.contains(&b) => self.held.push(b),
                RawEvent::Up(b) => self.held.retain(|h| *h != b),
                _ => {}
            }
            actions.extend(self.gestures.feed(ev, now));
        }
        actions.extend(self.gestures.tick(now));
        if !latched && self.app.in_game_menu().is_some() {
            actions.extend(self.gestures.drop_ff_latch());
        }
        for action in actions {
            self.act(action);
        }
        self.sync_pad();
    }

    pub fn set_menu_dpad_display(&mut self, enabled: bool) {
        self.gestures.set_menu_dpad_display(enabled);
    }

    fn sync_pad(&mut self) {
        for btn in self.app.taken_buttons() {
            self.pad.apply(Action::GbaUp(*btn));
        }
        if let Some(emu) = &self.emu {
            emu.set_input(self.pad.mask());
        }
    }

    fn bridge_link(&mut self, f: impl FnOnce(&mut App)) {
        let had_link = self.app.link_active();
        f(&mut self.app);
        if had_link && !self.app.link_active() {
            if let Some(emu) = &self.emu {
                emu.end_link();
            }
        }
    }

    fn act(&mut self, action: Action) {
        if trace() {
            eprintln!("slot: {action:?} in {:?}", self.app.phase());
        }
        let menu_open = self.app.in_game_menu().is_some();
        match action {
            Action::RewindStart if !menu_open => self.rewinding = true,
            Action::RewindStop => self.rewinding = false,
            Action::FfStart if !menu_open => self.fast = true,
            Action::FfStop => self.fast = false,
            _ => {}
        }
        let menu = self.overlaid();
        self.bridge_link(|app| app.apply(action));
        if matches!(
            action,
            Action::VolumeUp | Action::VolumeDown | Action::MuteToggle | Action::Screenshot
        ) {
            if let Some(emu) = &self.emu {
                emu.set_volume(self.app.output_volume());
            }
        }
        if menu || self.overlaid() {
            self.pad.clear();
        } else if self.app.takes_from_the_game(action) {
            if let Action::GbaDown(btn) | Action::GbaUp(btn) = action {
                self.pad.apply(Action::GbaUp(btn));
            }
        } else {
            self.pad.apply(action);
        }
        self.sync_rumble();
    }

    pub fn update(&mut self, dt: f32) {
        // A button held down, or fast forward or rewind running, keeps Auto Sleep away.
        if !self.held.is_empty() || self.fast || self.rewinding {
            self.app.keep_awake();
        }
        self.bridge_link(|app| app.update(dt));
        if let Some(emu) = &self.emu {
            emu.set_volume(self.app.output_volume());
        }
        if let Some((client_id, transport)) = self.app.take_link_transport() {
            match &self.emu {
                Some(emu) => match self.app.link_player() {
                    Some(player) => emu.begin_cable(player, transport),
                    None => emu.begin_link(client_id, transport),
                },
                None => eprintln!("slot: link: a transport arrived with no core to run it"),
            }
        }
        if self.app.take_reset() {
            if let Some(emu) = &self.emu {
                emu.reset();
            }
        }
        if let Some(codes) = self.app.take_cheats() {
            if let Some(emu) = &self.emu {
                emu.set_cheats(codes);
            }
        }
        let mut recoloured = false;
        if let Some(p) = self.app.take_gb_palette() {
            // A palette set over the grey: the grey goes on again below while effects are off.
            self.grey = false;
            if let Some(emu) = &self.emu {
                let options = match p {
                    Some(p) => crate::core::gb_palette_options(self.app.core(), p),
                    None => crate::core::gb_palette_off_options(self.app.core()),
                };
                for (key, value) in options {
                    emu.set_option(key, &value);
                    recoloured = true;
                }
            }
        }
        // Effects off is the bare emulator: a Game Boy game in the core's own grey, not the
        // chosen palette (Roy). Back on, the palette returns.
        let grey = self.app.screen_bare() && self.app.palette_live();
        if grey != self.grey {
            if let Some(emu) = &self.emu {
                let options = match (grey, self.app.gb_palette()) {
                    (true, _) => crate::core::gb_grey_options(self.app.core()),
                    (false, Some(p)) => crate::core::gb_palette_options(self.app.core(), p),
                    (false, None) => Vec::new(),
                };
                for (key, value) in options {
                    emu.set_option(key, &value);
                    recoloured = true;
                }
                self.grey = grey;
            }
        }
        if self.app.take_sgb_border_changed() && self.app.sgb_playing() {
            self.sgb_turn += 1;
            let (key, value) =
                crate::core::set_sgb_border(
                &self.root,
                self.app.sgb_border(),
                self.app.sgb_two(),
                self.sgb_turn,
            );
            if let Some(emu) = &self.emu {
                emu.set_option(key, &value);
                // Behind the menu too: the new border shows at once.
                recoloured = true;
            }
        }
        if self.app.take_tv_changed() {
            if let Some(emu) = &self.emu {
                let look = self.app.tv_look(self.app.playing_platform());
                for (key, value) in crate::core::tv_options(self.app.core(), look) {
                    emu.set_option(key, value);
                    recoloured = true;
                }
            }
        }
        if let Some(on) = self.app.take_lcd_ghosting() {
            let option = crate::core::lcd_ghosting_option(self.app.core(), on);
            if let (Some((key, value)), Some(emu)) = (option, &self.emu) {
                emu.set_option(key, value);
                recoloured = true;
            }
        }
        // Behind a menu the game is still: redraw its picture in the new colours. In play the
        // next frames show the change, Ghosting fading the old colours out like a real LCD.
        if recoloured && self.held() && !self.app.link_active() {
            if let Some(emu) = &self.emu {
                emu.refresh(true);
            }
        }
        if let Some((stem, serial)) = self.app.take_link_reload() {
            self.reload_for_link(&stem, serial);
        }
        if self.app.link_active() {
            if self.emu.as_ref().is_some_and(EmuHandle::bios_mismatch) {
                self.bridge_link(|app| app.bios_mismatch());
            } else if self.emu.as_ref().is_some_and(EmuHandle::peer_ended) {
                self.bridge_link(|app| app.peer_ended());
            } else if self.emu.as_ref().is_some_and(EmuHandle::link_lost) {
                self.app.peer_lost();
            }
        }
        if let Some(sfx) = self.app.take_sfx() {
            self.play_sfx(sfx);
        }
        self.sync_core();
        self.sync_reload();
        if !self.has_core() {
            for action in self.gestures.drop_ff_latch() {
                self.act(action);
            }
        }
        let ready = self.emu.as_ref().is_some_and(EmuHandle::has_published);
        if ready && !self.app.game_ready() {
            launch("first frame shown");
        }
        self.app.set_game_ready(ready);
        self.sync_speed();
        self.sync_rewind_hud();
        self.sync_ff_hud();
        self.sync_rumble();
        self.sync_pad();
        let now = self.app.now();
        self.prefetch.browse(self.app.browsed_rom(), now);
    }

    fn sync_rumble(&mut self) {
        let asked = match &self.emu {
            Some(emu) if self.playing() && self.app.rumble_enabled() => emu.rumble().strength(),
            _ => {
                self.pulse = None;
                return self.rumble(0);
            }
        };
        let now = self.app.now();
        let want = match (asked, self.pulse) {
            (0, Some((at, strength))) if now.saturating_sub(at) < RUMBLE_MIN_MS => strength,
            (0, _) => {
                self.pulse = None;
                0
            }
            (asked, Some((at, _))) => {
                self.pulse = Some((at, asked));
                if now.saturating_sub(at) < RUMBLE_MAX_MS {
                    asked
                } else {
                    0
                }
            }
            (asked, None) => {
                self.pulse = Some((now, asked));
                asked
            }
        };
        self.rumble(want);
    }

    fn sync_ff_hud(&mut self) {
        let ff = match (self.actually_fast_forwarding(), self.gestures.ff_latched()) {
            (false, _) => FfState::Off,
            (true, false) => FfState::Held,
            (true, true) => FfState::Latched,
        };
        self.app.set_ff(ff);
    }

    fn sync_rewind_hud(&mut self) {
        let fill = self
            .actually_rewinding()
            .then(|| self.emu.as_ref().map(EmuHandle::rewind_fill))
            .flatten();
        match fill {
            Some(fill) => self.app.show_rewind(fill),
            None => self.app.hide_rewind(),
        }
    }

    fn actually_rewinding(&self) -> bool {
        self.rewinding && self.playing() && self.app.may_rewind()
    }

    fn actually_fast_forwarding(&self) -> bool {
        self.fast && self.playing() && self.app.may_fast_forward()
    }

    fn inserting(&self) -> bool {
        matches!(self.app.phase(), Phase::Inserting { .. })
    }

    fn showing_polaroids(&self) -> bool {
        matches!(self.app.phase(), Phase::Polaroids { .. })
    }

    fn overlaid(&self) -> bool {
        self.showing_polaroids() || self.held()
    }

    fn playing(&self) -> bool {
        matches!(self.app.phase(), Phase::Playing { .. }) && !self.held()
    }

    fn held(&self) -> bool {
        self.app.game_menu_open()
            || self.app.in_game_menu().is_some()
            || self.app.shutting_down()
    }

    fn dozing(&self) -> bool {
        matches!(self.app.phase(), Phase::Doze { .. })
    }

    fn ejecting(&self) -> bool {
        matches!(self.app.phase(), Phase::Ejecting { .. })
    }

    fn sync_speed(&self) {
        if let Some(emu) = &self.emu {
            emu.set_fast_steps(u32::from(self.app.ff_speed()));
            emu.set_ff_sound(self.app.ff_sound());
            emu.set_speed(
                if self.inserting()
                    || self.ejecting()
                    || self.showing_polaroids()
                    || self.dozing()
                    || (self.held() && !self.app.link_active())
                {
                    Speed::Paused
                } else if self.actually_fast_forwarding() {
                    Speed::Fast
                } else {
                    Speed::Normal
                },
            );
            emu.set_rewinding(self.actually_rewinding());
            emu.set_turbo(self.app.turbo());
            emu.set_native_xy(self.app.playing_platform() == Platform::Snes);
            emu.set_rewind_recording(self.app.records_rewind());
        }
    }

    fn no_core() -> bool {
        std::env::var_os("SLOT_NO_CORE").is_some_and(|v| v != "0")
    }

    fn sync_core(&mut self) {
        if Self::no_core() {
            return;
        }
        let stem = match self.app.phase() {
            Phase::Shelf => {
                self.emu = None;
                return;
            }
            Phase::Inserting { cart, .. } => cart.clone(),
            _ => return,
        };
        // The Super Game Boy has no game of its own to run.
        if self.app.sgb_seated() {
            self.emu = None;
            return;
        }
        if self.emu.is_none() {
            let (_, serial) = self.app.link_mode(&stem);
            self.spawn_core(&stem, serial);
        }
        match self.emu.as_ref().map(EmuHandle::state) {
            Some(CoreState::Loading) => {}
            Some(CoreState::Ready) => {
                if let (true, Some(p), Some(emu)) =
                    (self.app.palette_live(), self.app.gb_palette(), &self.emu)
                {
                    emu.set_option("mgba_gb_colors", p.core_name());
                }
                let cheats = self.app.start_cheats();
                if let (false, Some(emu)) = (cheats.is_empty(), &self.emu) {
                    emu.set_cheats(cheats);
                }
                self.app.on_core_ready()
            }
            Some(CoreState::Failed) | None => {
                self.emu = None;
                self.app.on_core_failed();
            }
        }
    }

    fn spawn_core(&mut self, stem: &str, serial: &'static str) {
        let Some((rom, platform)) = self
            .app
            .seated_cart()
            .filter(|c| c.stem == stem)
            .map(|c| (c.rom.clone(), c.platform))
        else {
            return;
        };
        // A Game Boy game in the Super Game Boy plays on mGBA, the core that does the SGB.
        let sgb = self.app.sgb_playing();
        let core = match sgb {
            true => slot_store::Core::Mgba,
            false => slot_store::core_for_platform(&self.root, stem, platform),
        };
        launch(&format!("opening {stem} on {}", core.as_str()));
        // The core opens in the chosen palette: the grey, if wanted, follows once it runs.
        self.grey = false;
        self.app.set_core(core);
        self.app.set_platform(platform);
        self.app
            .set_video_mode(crate::video_mode::video_mode_for(&self.root, stem));
        self.app.set_link_loaded(serial);
        let resume = match self.app.starting_clean() {
            true => Ok(None),
            false => persist::read_resume(&self.root, platform, core, stem),
        };
        // A read the card failed is not an empty file: the game runs, but nothing of it is
        // written back over the save or resume state that could not be read.
        let resume = self.app.card_read("resume", resume);
        let resume_unread = resume.is_none();
        let resume = resume.flatten();
        if core == slot_store::Core::Gambatte {
            if let Some(status) = gambatte_boot_rom_status(&self.root, platform) {
                eprintln!("slot: Gambatte boot ROM: {status}");
                if resume.is_some() {
                    eprintln!("slot: Gambatte is resuming a saved state; boot logo is skipped");
                }
            }
        }
        launch("resume state read");
        let player = self.app.link_player();
        let palette = crate::core::palette_for(self.app.gb_palette(), platform, &rom, player);
        self.app.set_palette_live(palette.is_some());
        self.app.set_palette_capable(
            crate::core::palette_for(Some(slot_store::GbPalette::DEFAULT), platform, &rom, player).is_some(),
        );
        // The core opens on the emulator's thread, so the cart's insert keeps moving meanwhile.
        let root = self.root.clone();
        let ghosting = crate::core::lcd_ghosting_option(core, self.app.lcd_ghosting_for(platform));
        // The Super Game Boy's picture takes the SNES's look.
        let tv = crate::core::tv_options(core, self.app.tv_look(self.app.playing_platform()));
        let sgb_setup = sgb.then(|| (self.app.sgb_border(), self.app.sgb_two(), platform == Platform::Gbc));
        let open = Box::new(move || {
            let mut opened = open_core(
                &root,
                core,
                platform,
                serial,
                // slot.'s own screen looks take the place of the cores' colour correction.
                false,
                palette,
                player,
            );
            if let Some((key, value)) = ghosting {
                opened.core.set_option(key, value);
            }
            for (key, value) in tv {
                opened.core.set_option(key, value);
            }
            if let Some((border, two, gbc)) = sgb_setup {
                crate::core::sgb_options(&root, opened.core.as_mut(), border, two, gbc);
            }
            opened.core
        });
        self.app.set_named_core(crate::core::core_found(&self.root, core));
        launch("core opened");
        let sav = self.app.card_read("save", persist::read_sav(&self.root, platform, stem));
        let sav_unread = sav.is_none();
        let sav = sav.flatten();
        launch("save read");
        let ring = self.sink.ring();
        let linked = player.filter(|_| platform == Platform::Gba);
        let emu = EmuHandle::spawn_opening(open, rom, ring, sav, resume, linked);
        if sav_unread {
            emu.refuse_save_ram();
        }
        if resume_unread {
            emu.refuse_resume();
        }
        self.app.set_resume_unread(resume_unread);
        emu.set_volume(self.app.output_volume());
        emu.set_driven(self.driven);
        self.app.set_snapshot(Box::new(emu.snapshot()));
        self.emu = Some(emu);
    }

    fn reload_for_link(&mut self, stem: &str, serial: &'static str) {
        eprintln!("slot: link: loading {stem} again with gpsp_serial={serial}");
        if self.emu.is_some() {
            self.app.flush_resume();
        }
        self.emu = None;
        self.spawn_core(stem, serial);
        self.reloading = true;
    }

    fn sync_reload(&mut self) {
        if !self.reloading {
            return;
        }
        match self.emu.as_ref().map(EmuHandle::state) {
            Some(CoreState::Loading) => {}
            Some(CoreState::Ready) => {
                self.reloading = false;
                self.app.link_reload_done();
            }
            Some(CoreState::Failed) | None => {
                self.reloading = false;
                self.emu = None;
                self.app.link_reload_failed();
                if let Some((stem, serial)) = self.app.take_link_reload() {
                    self.reload_for_link(&stem, serial);
                }
            }
        }
    }
}

pub(crate) fn trace() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("SLOT_TRACE").is_some())
}

fn launch(step: &str) {
    eprintln!("slot: launch: {step} at {:.1} ms", crate::boot_time::ms());
}
