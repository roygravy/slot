//! RetroAchievements for slot: rcheevos (C, MIT) behind a flat shim, wrapped safely.
//!
//! One thread (the game thread) owns the [`Client`]. A network worker only moves bytes:
//! it takes [`Request`]s, performs them, and sends the answers back to the game thread,
//! which calls [`Client::complete`]. Nothing here ever blocks on the network.
use std::ffi::{c_char, c_int, c_void, CStr, CString};

#[repr(C)]
struct RawRequest {
    id: u32,
    url: *mut c_char,
    post: *mut c_char,
    content_type: *mut c_char,
}

#[repr(C)]
struct RawEvent {
    kind: c_int,
    id: u32,
    points: u32,
    title: [c_char; 128],
    description: [c_char; 256],
    badge: [c_char; 16],
    message: [c_char; 256],
}

#[repr(C)]
struct RawInfo {
    state: c_int,
    game_id: u32,
    game_title: [c_char; 96],
    game_badge: [c_char; 16],
    user: [c_char; 64],
    error: [c_char; 256],
    total: u32,
    unlocked: u32,
    points_total: u32,
    points_unlocked: u32,
    beaten_time: i64,
}

#[repr(C)]
struct RawAch {
    id: u32,
    points: u32,
    unlocked: c_int,
    unlock_time: i64,
    rarity: f32,
    title: [c_char; 96],
    description: [c_char; 256],
    badge: [c_char; 16],
}

#[repr(C)]
struct Ra {
    _private: [u8; 0],
}

/// The core's memory for one libretro memory id: pointer and size, null/0 if it has none.
pub type CoreMemory = unsafe extern "C" fn(ud: *mut c_void, id: u32, data: *mut *mut c_void, size: *mut usize);

extern "C" {
    fn ra_new() -> *mut Ra;
    fn ra_free(ra: *mut Ra);
    fn ra_set_memory(ra: *mut Ra, console: u32, mmap: *const c_void, info: Option<CoreMemory>, ud: *mut c_void);
    fn ra_login_token(ra: *mut Ra, user: *const c_char, token: *const c_char);
    fn ra_logout(ra: *mut Ra);
    fn ra_load_game(ra: *mut Ra, console: u32, rom: *const u8, len: usize);
    fn ra_unload_game(ra: *mut Ra);
    fn ra_forget_memory(ra: *mut Ra);
    fn ra_do_frame(ra: *mut Ra);
    fn ra_idle(ra: *mut Ra);
    fn ra_take_request(ra: *mut Ra, out: *mut RawRequest) -> c_int;
    fn ra_free_request(r: *mut RawRequest);
    fn ra_complete_request(ra: *mut Ra, id: u32, status: c_int, body: *const u8, len: usize);
    fn ra_take_event(ra: *mut Ra, out: *mut RawEvent) -> c_int;
    fn ra_get_info(ra: *mut Ra, out: *mut RawInfo);
    fn ra_ach_count(ra: *mut Ra) -> c_int;
    fn ra_ach_get(ra: *mut Ra, index: c_int, out: *mut RawAch) -> c_int;
    fn ra_refresh_list(ra: *mut Ra);
    fn ra_user_agent_clause(ra: *mut Ra, buf: *mut c_char, n: usize) -> usize;
}

/// RetroAchievements console ids for the systems slot runs.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Console {
    GameBoy = 4,
    GameBoyColor = 6,
    GameBoyAdvance = 5,
    Nes = 7,
    Snes = 3,
}

fn text(s: &[c_char]) -> String {
    // The shim always terminates; stop at the first NUL anyway.
    let bytes: Vec<u8> = s.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// One network request the worker must perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: u32,
    pub url: String,
    /// POST body, or `None` for a GET.
    pub post: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Unlocked { id: u32, points: u32, title: String, description: String, badge: String },
    GameCompleted,
    ServerError(String),
    Disconnected,
    Reconnected,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum State {
    Idle,
    LoggingIn,
    LoggedIn,
    Loading,
    GameReady,
    UnknownGame,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Info {
    pub state: State,
    pub game_id: u32,
    pub game_title: String,
    pub game_badge: String,
    pub user: String,
    pub error: String,
    pub total: u32,
    pub unlocked: u32,
    pub points_total: u32,
    pub points_unlocked: u32,
    /// Unix seconds the game was beaten (softcore), 0 if not yet.
    pub beaten_time: i64,
}

#[derive(Debug, Clone)]
pub struct Achievement {
    pub id: u32,
    pub points: u32,
    pub unlocked: bool,
    /// Unix seconds, 0 if not unlocked.
    pub unlock_time: i64,
    /// Share of players (0..100) that unlocked it, 0 if unknown.
    pub rarity: f32,
    pub title: String,
    pub description: String,
    pub badge: String,
}

pub struct Client {
    ra: *mut Ra,
}

/// Hands answers to rcheevos from the network thread; see [`Client::completer`]. rcheevos and
/// the shim lock their own state, so this may run while the game thread drives the client.
pub struct Completer {
    ra: *mut Ra,
}

unsafe impl Send for Completer {}

impl Completer {
    pub fn complete(&self, id: u32, status: i32, body: &[u8]) {
        unsafe { ra_complete_request(self.ra, id, status, body.as_ptr(), body.len()) };
    }
}

// The shim keeps no thread-local state; the caller keeps to one thread at a time.
unsafe impl Send for Client {}

impl Client {
    pub fn new() -> Option<Client> {
        let ra = unsafe { ra_new() };
        (!ra.is_null()).then_some(Client { ra })
    }

    /// Point the client at the running core's memory. `mmap` is a copy of the core's
    /// `retro_memory_map` (or null when the core never sent one). `core` answers for a libretro
    /// memory id; `ud` is passed back to it.
    ///
    /// # Safety
    /// `mmap` must be null or point at a valid `retro_memory_map`; its memory, and what `core`
    /// returns, must stay valid while the game runs.
    pub unsafe fn set_memory(&mut self, console: Console, mmap: *const c_void, core: CoreMemory, ud: *mut c_void) {
        ra_set_memory(self.ra, console as u32, mmap, Some(core), ud);
    }

    pub fn login_token(&mut self, user: &str, token: &str) {
        if let (Ok(u), Ok(t)) = (CString::new(user), CString::new(token)) {
            unsafe { ra_login_token(self.ra, u.as_ptr(), t.as_ptr()) };
        }
    }

    pub fn logout(&mut self) {
        unsafe { ra_logout(self.ra) };
    }

    /// Identify the game from its ROM bytes (hashed here, per console rules) and load it.
    pub fn load_game(&mut self, console: Console, rom: &[u8]) {
        unsafe { ra_load_game(self.ra, console as u32, rom.as_ptr(), rom.len()) };
    }

    pub fn unload_game(&mut self) {
        unsafe { ra_unload_game(self.ra) };
    }

    /// The core goes away: rcheevos reads its memory no more.
    pub fn forget_memory(&mut self) {
        unsafe { ra_forget_memory(self.ra) };
    }

    /// Call once per emulated frame while achievements are active.
    pub fn do_frame(&mut self) {
        unsafe { ra_do_frame(self.ra) };
    }

    /// Call instead of `do_frame` while the game is paused or achievements are paused.
    pub fn idle(&mut self) {
        unsafe { ra_idle(self.ra) };
    }

    /// The next request the worker has not taken yet.
    pub fn take_request(&mut self) -> Option<Request> {
        let mut raw = RawRequest {
            id: 0,
            url: std::ptr::null_mut(),
            post: std::ptr::null_mut(),
            content_type: std::ptr::null_mut(),
        };
        if unsafe { ra_take_request(self.ra, &mut raw) } == 0 {
            return None;
        }
        let get = |p: *mut c_char| {
            (!p.is_null()).then(|| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
        };
        let req = Request {
            id: raw.id,
            url: get(raw.url).unwrap_or_default(),
            post: get(raw.post),
            content_type: get(raw.content_type),
        };
        unsafe { ra_free_request(&mut raw) };
        Some(req)
    }

    /// Hand back the answer to a request. `status` 0 means the request never got an answer.
    pub fn complete(&mut self, id: u32, status: i32, body: &[u8]) {
        unsafe { ra_complete_request(self.ra, id, status, body.as_ptr(), body.len()) };
    }

    /// A handle the network thread answers requests with, while the game thread goes on.
    ///
    /// # Safety
    /// The handle must not be used after this client is dropped.
    pub unsafe fn completer(&self) -> Completer {
        Completer { ra: self.ra }
    }

    pub fn take_event(&mut self) -> Option<Event> {
        let mut raw: RawEvent = unsafe { std::mem::zeroed() };
        if unsafe { ra_take_event(self.ra, &mut raw) } == 0 {
            return None;
        }
        Some(match raw.kind {
            1 => Event::Unlocked {
                id: raw.id,
                points: raw.points,
                title: text(&raw.title),
                description: text(&raw.description),
                badge: text(&raw.badge),
            },
            2 => Event::GameCompleted,
            3 => Event::ServerError(text(&raw.message)),
            4 => Event::Disconnected,
            _ => Event::Reconnected,
        })
    }

    pub fn info(&mut self) -> Info {
        let mut raw: RawInfo = unsafe { std::mem::zeroed() };
        unsafe { ra_get_info(self.ra, &mut raw) };
        Info {
            state: match raw.state {
                1 => State::LoggingIn,
                2 => State::LoggedIn,
                3 => State::Loading,
                4 => State::GameReady,
                5 => State::UnknownGame,
                6 => State::Failed,
                _ => State::Idle,
            },
            game_id: raw.game_id,
            game_title: text(&raw.game_title),
            game_badge: text(&raw.game_badge),
            user: text(&raw.user),
            error: text(&raw.error),
            total: raw.total,
            unlocked: raw.unlocked,
            points_total: raw.points_total,
            points_unlocked: raw.points_unlocked,
            beaten_time: raw.beaten_time,
        }
    }

    /// The rcheevos part of the User-Agent, e.g. `rcheevos/12.1`.
    pub fn user_agent_clause(&mut self) -> String {
        let mut buf = [0 as c_char; 64];
        unsafe { ra_user_agent_clause(self.ra, buf.as_mut_ptr(), buf.len()) };
        text(&buf)
    }

    /// Rebuild the achievement list (after an unlock).
    pub fn refresh_achievements(&mut self) {
        unsafe { ra_refresh_list(self.ra) };
    }

    pub fn achievements(&mut self) -> Vec<Achievement> {
        let n = unsafe { ra_ach_count(self.ra) };
        (0..n)
            .filter_map(|i| {
                let mut raw: RawAch = unsafe { std::mem::zeroed() };
                (unsafe { ra_ach_get(self.ra, i, &mut raw) } != 0).then(|| Achievement {
                    id: raw.id,
                    points: raw.points,
                    unlocked: raw.unlocked != 0,
                    unlock_time: raw.unlock_time,
                    rarity: raw.rarity,
                    title: text(&raw.title),
                    description: text(&raw.description),
                    badge: text(&raw.badge),
                })
            })
            .collect()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        unsafe { ra_free(self.ra) };
    }
}

mod session;
pub use session::Session;

#[cfg(test)]
mod tests;
