//! A running RetroAchievements session: the client, plus the one thread that talks to the
//! network. The game thread calls [`Session::pump`] (or [`Session::frame`]) every frame; it
//! never waits for the network.
use std::mem::ManuallyDrop;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::{Client, Completer, Console, CoreMemory, Event, Info, Request, State};
use std::ffi::c_void;

/// Status 0 never reaches rcheevos: a request that got no answer is reported as a retryable
/// error (-2), which rcheevos retries on its own.
const RETRYABLE: i32 = -2;

pub struct Session {
    // Freed by Drop, after the network thread is done with it.
    client: ManuallyDrop<Client>,
    to_net: Option<Sender<Request>>,
    net: Option<JoinHandle<()>>,
    waiting_to_load: Option<(Console, Vec<u8>)>,
}

impl Session {
    /// Start the network thread and log in with the saved token. `version` is slot's own
    /// version ("1.3"), sent in the User-Agent.
    pub fn new(user: &str, token: &str, version: &str) -> Option<Session> {
        let mut client = Client::new()?;
        let (to_net, requests) = channel::<Request>();
        // The network thread hands each answer to rcheevos itself: reading Pokémon's 1.7 MB
        // set took 2.4 s, and on the game thread the game stood still that long (Roy). The
        // thread is joined before the client is dropped (see Drop).
        let completer = unsafe { client.completer() };
        // RetroAchievements asks every client to name itself, its version and rcheevos;
        // an unnamed client is counted as an unknown emulator.
        let agent = format!("slot-x-gravy/{version} (Linux) {}", client.user_agent_clause());
        let net = std::thread::Builder::new()
            .name("slot-ra-net".into())
            .spawn(move || net_loop(requests, completer, agent))
            .ok()?;
        client.login_token(user, token);
        let mut session = Session {
            client: ManuallyDrop::new(client),
            to_net: Some(to_net),
            net: Some(net),
            waiting_to_load: None,
        };
        session.forward_requests();
        Some(session)
    }

    /// See [`Client::set_memory`].
    ///
    /// # Safety
    /// Same rules as [`Client::set_memory`].
    pub unsafe fn set_memory(&mut self, console: Console, mmap: *const c_void, core: CoreMemory, ud: *mut c_void) {
        self.client.set_memory(console, mmap, core, ud);
    }

    /// Identify and load this game; it waits for the login if that is not done yet.
    pub fn load_game(&mut self, console: Console, rom: Vec<u8>) {
        self.waiting_to_load = Some((console, rom));
        self.start_pending_load();
    }

    fn start_pending_load(&mut self) {
        if self.waiting_to_load.is_none() {
            return;
        }
        match self.client.info().state {
            State::LoggedIn | State::GameReady | State::UnknownGame => {
                if let Some((console, rom)) = self.waiting_to_load.take() {
                    let at = std::time::Instant::now();
                    self.client.load_game(console, &rom);
                    slow("game identified", rom.len(), at);
                }
            }
            State::Failed => self.waiting_to_load = None,
            _ => {}
        }
    }

    fn forward_requests(&mut self) {
        while let Some(req) = self.client.take_request() {
            if let Some(tx) = &self.to_net {
                if tx.send(req).is_err() {
                    break;
                }
            }
        }
    }

    /// Start a waiting game load, then send out whatever rcheevos asked for.
    pub fn pump(&mut self) {
        self.start_pending_load();
        self.forward_requests();
    }

    /// One emulated frame. `run` false (cheats on, game paused) keeps the session alive but
    /// checks no achievements.
    pub fn frame(&mut self, run: bool) {
        self.pump();
        if run && self.client.info().state == State::GameReady {
            let at = std::time::Instant::now();
            self.client.do_frame();
            slow("frame checked", 0, at);
        } else {
            self.client.idle();
        }
        self.forward_requests();
    }

    pub fn take_event(&mut self) -> Option<Event> {
        self.client.take_event()
    }

    pub fn info(&mut self) -> Info {
        self.client.info()
    }

    pub fn client(&mut self) -> &mut Client {
        &mut self.client
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // The game is gone at once: no memory reads, no game. The network thread may still
        // be in a request (up to 15 s on a weak Wi-Fi), longer than a closing game may take:
        // it finishes in the background and the client is freed after it, never before.
        self.client.unload_game();
        self.client.forget_memory();
        self.to_net = None;
        // SAFETY: taken once, here; the field is not used after.
        let client = unsafe { ManuallyDrop::take(&mut self.client) };
        let Some(net) = self.net.take() else { return };
        let held = std::sync::Arc::new(std::sync::Mutex::new(Some((net, client))));
        let mine = held.clone();
        let ended = std::thread::Builder::new().name("slot-ra-end".into()).spawn(move || {
            if let Some((net, client)) = mine.lock().ok().and_then(|mut h| h.take()) {
                let _ = net.join();
                drop(client);
            }
        });
        if ended.is_err() {
            // No thread to spare: wait here, as before.
            if let Some((net, client)) = held.lock().ok().and_then(|mut h| h.take()) {
                let _ = net.join();
                drop(client);
            }
        }
    }
}

fn net_loop(requests: Receiver<Request>, client: Completer, user_agent: String) {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .http_status_as_error(false)
        .user_agent(&user_agent)
        .build()
        .into();
    while let Ok(req) = requests.recv() {
        let result = match &req.post {
            Some(body) => agent
                .post(&req.url)
                .header(
                    "Content-Type",
                    req.content_type.as_deref().unwrap_or("application/x-www-form-urlencoded"),
                )
                .send(body.as_bytes()),
            None => agent.get(&req.url).call(),
        };
        let (status, body) = match result {
            Ok(mut resp) => {
                let status = resp.status().as_u16() as i32;
                match resp.body_mut().read_to_vec() {
                    Ok(body) => (status, body),
                    Err(_) => (RETRYABLE, Vec::new()),
                }
            }
            Err(_) => (RETRYABLE, Vec::new()),
        };
        let at = std::time::Instant::now();
        client.complete(req.id, status, &body);
        slow("answer (network thread)", body.len(), at);
    }
}

/// slot.log: which step of achievements held the game up, and on how many bytes.
fn slow(what: &str, bytes: usize, at: std::time::Instant) {
    let ms = at.elapsed().as_secs_f64() * 1000.0;
    if ms > 50.0 {
        eprintln!("slot: ra: {what} took {ms:.1} ms ({bytes} bytes)");
    }
}
