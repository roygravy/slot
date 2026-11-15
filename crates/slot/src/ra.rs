//! RetroAchievements on the emulator thread (test build, feature `retroachievements`).
//!
//! The session starts a game by leaving a [`Setup`]; the emulator thread takes it, runs a
//! [`Runner`] beside the core and publishes what the screens show in [`STATUS`].
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use slot_achievements::{Achievement, Console, Event, Info, Session};
use slot_retro::RetroCore;

/// What the session knows when it opens a game.
pub struct Setup {
    pub console: Console,
    pub user: String,
    pub token: String,
    pub root: PathBuf,
}

static SETUP: Mutex<Option<Setup>> = Mutex::new(None);
/// Cheats are on: achievements wait.
pub static CHEATS: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
pub struct Status {
    pub info: Option<Info>,
    pub list: Vec<Achievement>,
    /// Unlock events not yet shown as a toast.
    pub toasts: Vec<Event>,
    pub paused: bool,
    /// Seconds played in this game, over all sessions.
    pub played: u64,
}

pub static STATUS: Mutex<Status> = Mutex::new(Status {
    info: None,
    list: Vec::new(),
    toasts: Vec::new(),
    paused: false,
    played: 0,
});

/// Leave (or clear) the setup for the next game that starts.
pub fn prepare(setup: Option<Setup>) {
    *SETUP.lock().unwrap() = setup;
    *STATUS.lock().unwrap() = Status::default();
}

/// The login the Manager left on the card: user on the first line, token on the second.
pub fn read_login(root: &Path) -> Option<(String, String)> {
    let text = std::fs::read_to_string(root.join("System/retroachievements.txt")).ok()?;
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    Some((lines.next()?.to_string(), lines.next()?.to_string()))
}

/// The web API key (third line of the login file), if the user gave one.
pub fn read_key(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("System/retroachievements.txt")).ok()?;
    text.lines().map(str::trim).filter(|l| !l.is_empty()).nth(2).map(str::to_string)
}

/// The number after `"key":` at the start of `s`.
fn number_after(s: &str, key: &str) -> Option<u32> {
    let at = s.find(key)? + key.len();
    let rest = s[at..].trim_start().strip_prefix(':')?.trim_start().trim_start_matches('"');
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Achievement id -> players who earned it, from a GetGameExtended answer.
pub fn parse_counts(json: &str) -> std::collections::HashMap<u32, u32> {
    let mut out = std::collections::HashMap::new();
    let mut from = 0;
    while let Some(i) = json[from..].find("\"NumAwarded\"") {
        let at = from + i;
        let before = &json[at.saturating_sub(300)..at];
        let n = number_after(&json[at..], "\"NumAwarded\"");
        let id = before.rfind("\"ID\"").and_then(|k| number_after(&before[k..], "\"ID\""));
        if let (Some(id), Some(n)) = (id, n) {
            out.insert(id, n);
        }
        from = at + 12;
    }
    out
}

/// Fetch how many players earned each achievement of `game` (needs the user's web API key).
fn fetch_counts(root: &Path, game: u32) {
    let Some(key) = read_key(root) else { return };
    std::thread::spawn(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(20)))
            .build()
            .into();
        let url = format!("https://retroachievements.org/API/API_GetGameExtended.php?y={key}&i={game}");
        let Ok(mut resp) = agent.get(&url).call() else { return };
        let Ok(json) = resp.body_mut().with_config().limit(1 << 22).read_to_string() else { return };
        let counts = parse_counts(&json);
        if let Ok(mut c) = crate::ra_page::COUNTS.lock() {
            *c = Some(counts);
        }
    });
}

/// The play time RetroAchievements itself has for a game (game id, seconds), when known.
static SITE_PLAYED: Mutex<Option<(u32, u64)>> = Mutex::new(None);

/// Ask RetroAchievements how long `user` has played `game` (needs the web API key), so the
/// time shown is the site's, with this session added (Roy 2026-11-15: the times differed).
fn fetch_played(root: &Path, user: String, game: u32) {
    let Some(key) = read_key(root) else { return };
    std::thread::spawn(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(20)))
            .build()
            .into();
        let url = format!(
            "https://retroachievements.org/API/API_GetGameInfoAndUserProgress.php?y={key}&u={user}&g={game}"
        );
        let Ok(mut resp) = agent.get(&url).call() else { return };
        let Ok(json) = resp.body_mut().with_config().limit(1 << 22).read_to_string() else { return };
        if let Some(secs) = parse_played(&json) {
            eprintln!("slot: ra: site play time {secs} s for game {game}");
            *SITE_PLAYED.lock().unwrap() = Some((game, secs));
        }
    });
}

/// `UserTotalPlaytime` (seconds) from a GetGameInfoAndUserProgress answer.
pub fn parse_played(json: &str) -> Option<u64> {
    number_after(json, "\"UserTotalPlaytime\"").map(u64::from)
}

pub struct Runner {
    session: Session,
    // The callback's `ud`: a pointer to the core, which outlives the runner.
    _core: Box<*const (dyn RetroCore + 'static)>,
    unlocked: u32,
    root: PathBuf,
    carry: f32,
    game_id: u32,
    played: u64,
    /// Seconds played since this game started, added to the site's time once it is in.
    this_run: u64,
    site_taken: bool,
    tick: Instant,
    saved: Instant,
    fetched: std::collections::HashSet<String>,
    badge_tx: std::sync::mpsc::Sender<String>,
    /// Frames since the counts, the list and the badges were last looked at.
    beat: u32,
}

/// The achievements themselves are checked every frame; the counts, the list for the
/// overview and the badges only every quarter second or on an unlock. Doing all of that
/// every frame made Pokémon's big set slow the GBA down (Roy).
const LOOK_EVERY: u32 = 15;

/// A game's achievements play time, in Config: System is swapped whole by an update, which
/// threw the times away (Roy 2026-11-14: "0m played"). One still in System is read once.
fn time_file(root: &Path, game: u32) -> PathBuf {
    let at = root.join("Config").join(format!("ra_time_{game}"));
    let old = root.join("System").join(format!("ra_time_{game}"));
    if !at.exists() && old.exists() {
        let _ = std::fs::copy(&old, &at);
    }
    at
}

/// Fetches badge pictures one by one, off the emulator thread.
fn badge_thread() -> std::sync::mpsc::Sender<String> {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(15)))
            .user_agent("slot-x-gravy")
            .build()
            .into();
        for name in rx {
            let avatar = name.strip_prefix("@avatar:").map(str::to_string);
            let url = match &avatar {
                Some(user) => format!("https://media.retroachievements.org/UserPic/{user}.png"),
                None => format!("https://media.retroachievements.org/Badge/{name}.png"),
            };
            let Ok(mut resp) = agent.get(&url).call() else { continue };
            let Ok(bytes) = resp.body_mut().with_config().limit(1 << 20).read_to_vec() else { continue };
            let Ok(mut reader) = png::Decoder::new(std::io::Cursor::new(bytes)).read_info() else { continue };
            let mut buf = vec![0; reader.output_buffer_size()];
            let Ok(frame) = reader.next_frame(&mut buf) else { continue };
            let (w, h) = (frame.width as usize, frame.height as usize);
            if w != h || (avatar.is_none() && w != 64) {
                continue;
            }
            let rgba: Vec<u8> = match frame.color_type {
                png::ColorType::Rgba => buf[..w * h * 4].to_vec(),
                png::ColorType::Rgb => buf[..w * h * 3].chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
                _ => continue,
            };
            if avatar.is_some() {
                if let Ok(mut a) = crate::ra_page::AVATAR.lock() {
                    *a = Some((w, rgba));
                }
                continue;
            }
            if let Ok(mut b) = crate::ra_page::BADGES.lock() {
                b.get_or_insert_with(Default::default).insert(name, rgba);
            }
        }
    });
    tx
}

unsafe extern "C" fn core_memory(ud: *mut c_void, id: u32, data: *mut *mut c_void, size: *mut usize) {
    let core = &**(ud as *const *const (dyn RetroCore + 'static));
    let (p, n) = core.memory_raw(id);
    *data = p;
    *size = n;
}

impl Runner {
    /// Start for the game the core just loaded, if a setup was left.
    pub fn start(core: &dyn RetroCore, rom: &Path) -> Option<Runner> {
        // A new game starts clean: nothing of the last game may show, also when this one
        // gets no achievements (no login, gpSP, no set).
        *STATUS.lock().unwrap() = Status::default();
        if let Ok(mut c) = crate::ra_page::COUNTS.lock() {
            *c = None;
        }
        let setup = SETUP.lock().unwrap().take()?;
        let bytes = std::fs::read(rom).ok()?;
        let version = std::fs::read_to_string(setup.root.join("System/version.txt")).unwrap_or_default();
        let version = version.trim().trim_start_matches('v');
        let mut session = Session::new(&setup.user, &setup.token, if version.is_empty() { "1.3" } else { version })?;
        // The core is boxed for the whole run and outlives this runner.
        let raw: *const (dyn RetroCore + 'static) = unsafe { std::mem::transmute(core as *const dyn RetroCore) };
        let holder = Box::new(raw);
        unsafe {
            session.set_memory(
                setup.console,
                core.memory_map_ptr(),
                core_memory,
                &*holder as *const *const (dyn RetroCore + 'static) as *mut c_void,
            );
        }
        session.load_game(setup.console, bytes);
        Some(Runner {
            session,
            _core: holder,
            unlocked: 0,
            game_id: 0,
            played: 0,
            this_run: 0,
            site_taken: false,
            root: setup.root,
            carry: 0.0,
            tick: Instant::now(),
            saved: Instant::now(),
            fetched: Default::default(),
            badge_tx: badge_thread(),
            beat: LOOK_EVERY,
        })
    }

    /// After each emulated frame (`run` false while the game is not playing for points).
    pub fn frame(&mut self, run: bool) {
        self.session.frame(run);
        let mut changed = false;
        let mut events = Vec::new();
        while let Some(e) = self.session.take_event() {
            changed |= matches!(e, Event::Unlocked { .. });
            events.push(e);
        }
        self.beat += 1;
        if events.is_empty() && self.beat < LOOK_EVERY {
            return;
        }
        self.beat = 0;
        let info = self.session.info();
        if changed || info.unlocked != self.unlocked {
            self.unlocked = info.unlocked;
            self.session.client().refresh_achievements();
        }
        let mut st = STATUS.lock().unwrap();
        if changed || st.list.is_empty() {
            st.list = self.session.client().achievements();
        }
        st.toasts.extend(events.into_iter().filter(|e| matches!(e, Event::Unlocked { .. })));
        if self.game_id != info.game_id && info.game_id != 0 {
            self.game_id = info.game_id;
            fetch_counts(&self.root, info.game_id);
            *SITE_PLAYED.lock().unwrap() = None;
            fetch_played(&self.root, info.user.clone(), info.game_id);
            self.this_run = 0;
            self.site_taken = false;
            self.played = std::fs::read_to_string(time_file(&self.root, info.game_id))
                .ok()
                .and_then(|t| t.trim().parse().ok())
                .unwrap_or(0);
        }
        // Play time: wall seconds while frames run.
        let dt = self.tick.elapsed();
        self.tick = Instant::now();
        if dt.as_millis() < 500 {
            self.carry += dt.as_secs_f32();
            while self.carry >= 1.0 {
                self.carry -= 1.0;
                self.played += 1;
                self.this_run += 1;
            }
        }
        // Once RetroAchievements answers, its time leads: the site's own count plus this
        // session. Without a web API key or a connection the card's own count stays.
        if !self.site_taken {
            if let Some((game, secs)) = *SITE_PLAYED.lock().unwrap() {
                if game == self.game_id {
                    self.site_taken = true;
                    self.played = secs + self.this_run;
                }
            }
        }
        if self.game_id != 0 && self.saved.elapsed().as_secs() >= 30 {
            self.saved = Instant::now();
            let _ = std::fs::write(time_file(&self.root, self.game_id), self.played.to_string());
        }
        st.played = self.played;
        if !info.user.is_empty() && self.fetched.insert(format!("@avatar:{}", info.user)) {
            let _ = self.badge_tx.send(format!("@avatar:{}", info.user));
        }
        for a in &st.list {
            for name in [a.badge.clone(), format!("{}_lock", a.badge)] {
                if !a.badge.is_empty() && self.fetched.insert(name.clone()) {
                    let _ = self.badge_tx.send(name);
                }
            }
        }
        st.info = Some(info);
        st.paused = !run;
    }
}

/// This game has achievements and they are counting.
pub fn active() -> bool {
    STATUS
        .lock()
        .map(|s| s.info.as_ref().is_some_and(|i| i.state == slot_achievements::State::GameReady && i.total > 0))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    #[test]
    fn site_play_time_is_read() {
        let json = r#"{"ID":1,"UserCompletion":"12.50%","UserTotalPlaytime":13260,"HighestAwardKind":null}"#;
        assert_eq!(super::parse_played(json), Some(13260));
        assert_eq!(super::parse_played(r#"{"ID":1}"#), None);
    }
}
