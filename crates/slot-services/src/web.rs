//! The ROM Manager: a small web page on the home network for adding, renaming and deleting
//! games on the card. It only listens while the console shows its screen, and every change
//! asks for the PIN shown there.
use crate::update::UPDATE;
use slot_store::Platform;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PAGE: &str = include_str!("../web/index.html");
/// The boot logo's own slot., x and GRAVY, side by side.
const LOGO: &[u8] = include_bytes!("../web/logo.png");
/// Larger than any GBA game (32 MB), so a wrong file is turned away before it fills the card.
const MAX_UPLOAD: u64 = 64 << 20;
const MAX_HEAD: usize = 16 << 10;
const MAX_CONNECTIONS: usize = 8;
/// Wrong PINs before the page stops taking any, until the screen is opened again.
const MAX_WRONG: usize = 10;
/// The screen closed and opened again within this keeps its PIN, so the phone stays signed in.
const KEEP_PIN: Duration = Duration::from_secs(5 * 60);
/// The PIN of the page last closed, and when.
static LAST_PIN: Mutex<Option<(String, Instant)>> = Mutex::new(None);
/// Where a half-sent game waits before it gets its name.
const PART: &str = ".rom-manager.part";
/// Phones ticked "Remember this phone": one secret key a line, the newest last.
const KEYS: &str = "Config/.manager-keys";
const MAX_KEYS: usize = 8;

pub struct Web {
    pub owner: String,
    pub address: String,
    pub pin: String,
    /// The same page as `http://gravy.local`, while mDNS answers for it.
    pub named: Option<String>,
    stop: Arc<AtomicBool>,
    _mdns: Option<crate::mdns::Responder>,
}

struct Shared {
    root: PathBuf,
    pin: String,
    stop: Arc<AtomicBool>,
    wrong: AtomicUsize,
    open: AtomicUsize,
    /// `http://gravy.local` when mDNS answers for it: a page opened by IP moves there.
    named: Option<String>,
}

impl Web {
    pub fn start(root: &Path, owner: &str, ip: Ipv4Addr) -> Result<Web, &'static str> {
        // Port 80 so the address needs no number; another server there leaves 8080. A page
        // just closed lets go of 80 within its 50 ms accept nap: wait that out, so the
        // address stays the same (Roy).
        let mut bound = TcpListener::bind(("0.0.0.0", 80));
        for _ in 0..4 {
            if bound.is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
            bound = TcpListener::bind(("0.0.0.0", 80));
        }
        let (listener, port) = match bound {
            Ok(l) => (l, 80),
            Err(_) => (
                TcpListener::bind(("0.0.0.0", 8080)).map_err(|_| "WEB_PORT_BUSY")?,
                8080,
            ),
        };
        listener
            .set_nonblocking(true)
            .map_err(|_| "WEB_PORT_BUSY")?;
        let at = |host: &str| match port {
            80 => format!("http://{host}"),
            p => format!("http://{host}:{p}"),
        };
        let mdns = crate::mdns::Responder::start(ip);
        let named = mdns.as_ref().map(|m| at(&m.name));
        let pin = pin();
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Shared {
            root: root.to_path_buf(),
            pin: pin.clone(),
            stop: stop.clone(),
            wrong: AtomicUsize::new(0),
            open: AtomicUsize::new(0),
            named: named.clone(),
        });
        std::thread::spawn(move || accept(listener, shared));
        Ok(Web {
            owner: owner.into(),
            address: at(&ip.to_string()),
            pin,
            named,
            stop,
            _mdns: mdns,
        })
    }

    pub fn word(&self) -> String {
        match &self.named {
            Some(n) => format!("WEB {} {} {n}", self.address, self.pin),
            None => format!("WEB {} {}", self.address, self.pin),
        }
    }
}

impl Drop for Web {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(mut last) = LAST_PIN.lock() {
            *last = Some((self.pin.clone(), Instant::now()));
        }
    }
}

fn pin() -> String {
    let kept = LAST_PIN.lock().ok().and_then(|last| {
        last.as_ref()
            .filter(|(_, at)| at.elapsed() < KEEP_PIN)
            .map(|(pin, _)| pin.clone())
    });
    if let Some(pin) = kept {
        return pin;
    }
    let mut b = [0u8; 4];
    let _ = File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b));
    format!("{:04}", u32::from_le_bytes(b) % 10_000)
}

fn keys(root: &Path) -> Vec<String> {
    fs::read_to_string(root.join(KEYS))
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|k| k.len() == 32)
        .map(str::to_owned)
        .collect()
}

fn known_key(root: &Path, key: &str) -> bool {
    key.len() == 32 && keys(root).iter().any(|k| k == key)
}

/// A new key for this phone, kept on the card; the oldest go once there are too many.
fn remember(root: &Path) -> Option<String> {
    let mut b = [0u8; 16];
    File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).ok()?;
    let key: String = b.iter().map(|x| format!("{x:02x}")).collect();
    let mut all = keys(root);
    all.push(key.clone());
    let from = all.len().saturating_sub(MAX_KEYS);
    let text: String = all[from..].iter().map(|k| format!("{k}\n")).collect();
    slot_store::atomic_write(&root.join(KEYS), text.as_bytes()).ok()?;
    Some(key)
}

fn accept(listener: TcpListener, shared: Arc<Shared>) {
    while !shared.stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                if shared.open.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
                    continue;
                }
                shared.open.fetch_add(1, Ordering::SeqCst);
                let shared = shared.clone();
                std::thread::spawn(move || {
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(20)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(20)));
                    handle(stream, &shared);
                    shared.open.fetch_sub(1, Ordering::SeqCst);
                });
            }
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

struct Request {
    method: String,
    path: Vec<String>,
    pin: String,
    key: String,
    length: u64,
}

fn read_head(reader: &mut BufReader<TcpStream>) -> Option<Request> {
    let mut line = String::new();
    let mut taken = reader.read_line(&mut line).ok()?;
    let mut words = line.split_whitespace();
    let method = words.next()?.to_owned();
    let target = words.next()?.split('?').next()?.to_owned();
    let mut pin = String::new();
    let mut key = String::new();
    let mut length = 0;
    loop {
        let mut h = String::new();
        let n = reader.read_line(&mut h).ok()?;
        taken += n;
        if n == 0 || taken > MAX_HEAD {
            return None;
        }
        let h = h.trim_end();
        if h.is_empty() {
            break;
        }
        let Some((name, value)) = h.split_once(':') else {
            continue;
        };
        match name.trim().to_ascii_lowercase().as_str() {
            "x-pin" => pin = value.trim().to_owned(),
            "x-key" => key = value.trim().to_owned(),
            "content-length" => length = value.trim().parse().ok()?,
            _ => {}
        }
    }
    let path = target
        .split('/')
        .filter(|s| !s.is_empty())
        .map(decode)
        .collect::<Option<_>>()?;
    Some(Request {
        method,
        path,
        pin,
        key,
        length,
    })
}

fn decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn reply(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8]) {
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(body);
}

fn fail(stream: &mut TcpStream, status: &str, why: &str) {
    reply(stream, status, "text/plain; charset=utf-8", why.as_bytes());
}

fn handle(stream: TcpStream, shared: &Shared) {
    let Ok(mut out) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(stream);
    serve(&mut out, &mut reader, shared);
    // Close gently: say we're done, then swallow whatever the phone still sends (a refused
    // upload's body). Closing with bytes unread resets the connection, and Safari then loses
    // the answer and the upload never finishes on the page.
    let _ = out.flush();
    let _ = out.shutdown(std::net::Shutdown::Write);
    let _ = reader.get_ref().set_read_timeout(Some(Duration::from_secs(2)));
    let _ = std::io::copy(&mut (&mut reader).take(MAX_UPLOAD + (1 << 20)), &mut std::io::sink());
}

fn serve(out: &mut TcpStream, reader: &mut BufReader<TcpStream>, shared: &Shared) {
    let Some(req) = read_head(reader) else {
        return fail(out, "400 Bad Request", "Bad request");
    };
    let path: Vec<&str> = req.path.iter().map(String::as_str).collect();
    match (req.method.as_str(), path.as_slice()) {
        ("GET", []) => {
            return reply(
                out,
                "200 OK",
                "text/html; charset=utf-8",
                PAGE.as_bytes(),
            )
        }
        ("GET", ["logo.png"]) => return reply(out, "200 OK", "image/png", LOGO),
        ("GET", ["api", "name"]) => {
            let name = shared.named.as_deref().unwrap_or("");
            return reply(out, "200 OK", "text/plain", name.as_bytes());
        }
        ("GET", ["favicon.ico"]) => return fail(out, "404 Not Found", ""),
        _ => {}
    }
    // A remembered phone needs no PIN; the page still only listens while the screen is open.
    let remembered = known_key(&shared.root, &req.key);
    if !remembered && shared.wrong.load(Ordering::SeqCst) >= MAX_WRONG {
        return fail(
            out,
            "403 Forbidden",
            "Too many wrong PINs. Open GRAVY Manager on the console again.",
        );
    }
    if !remembered && req.pin != shared.pin {
        shared.wrong.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Duration::from_secs(1));
        return fail(out, "401 Unauthorized", "Wrong PIN");
    }
    let root = &shared.root;
    let result = match (req.method.as_str(), path.as_slice()) {
        ("GET", ["api", "update"]) => {
            return reply(
                out,
                "200 OK",
                "application/json",
                UPDATE.json(root).as_bytes(),
            )
        }
        ("POST", ["api", "remember"]) => {
            return match remember(root) {
                Some(key) => reply(out, "200 OK", "text/plain", key.as_bytes()),
                None => fail(out, "409 Conflict", "The card can't be written"),
            };
        }
        ("GET", ["api", "ra"]) => {
            let json = match crate::ra::user(root) {
                Some(u) => format!(
                    "{{\"user\":\"{}\",\"key\":{}}}",
                    u.replace(['\\', '"'], ""),
                    crate::ra::has_key(root)
                ),
                None => "{}".to_string(),
            };
            return reply(out, "200 OK", "application/json", json.as_bytes());
        }
        ("POST", ["api", "ra", "login"]) => {
            let mut body = String::new();
            if req.length > 1024 || (&mut *reader).take(req.length).read_to_string(&mut body).is_err() {
                Err("Bad login")
            } else {
                crate::ra::login(root, &body)
            }
        }
        ("POST", ["api", "ra", "key"]) => {
            let mut body = String::new();
            if req.length > 256 || (&mut *reader).take(req.length).read_to_string(&mut body).is_err() {
                Err("Bad key")
            } else {
                crate::ra::set_key(root, &body)
            }
        }
        ("DELETE", ["api", "ra"]) => crate::ra::logout(root),
        ("POST", ["api", "update", "check"]) => UPDATE.check(root),
        ("POST", ["api", "update", "download"]) => UPDATE.download(root),
        ("POST", ["api", "update", "install"]) => UPDATE.install(root),
        ("POST", ["api", "update", "baseos", "check"]) => UPDATE.baseos_check(),
        ("POST", ["api", "update", "baseos", "download"]) => UPDATE.baseos_download(root),
        ("POST", ["api", "update", "baseos", "install"]) => UPDATE.baseos_install(root),
        ("PUT", ["api", "update", "zip"]) => UPDATE.upload(root, req.length, reader),
        ("GET", ["api", "list"]) => {
            return reply(
                out,
                "200 OK",
                "application/json",
                list(root).as_bytes(),
            )
        }
        // Wallpapers and screenshots are shown on the page; nothing else on the card is
        // handed out.
        ("GET", ["api", "games", system @ ("WALLPAPERS" | "SCREENSHOTS"), name]) => {
            let picture = checked(system, name)
                .ok()
                .and_then(|(p, n)| fs::read(p.dir(root).join(n)).ok());
            return match picture {
                Some(png) => reply(out, "200 OK", "image/png", &png),
                None => fail(out, "404 Not Found", "That file isn't there"),
            };
        }
        // The hidden Logs tab (tap the logo five times): the console's own logs, to read along.
        ("GET", ["api", "logs", name]) => {
            return match log_file(root, name) {
                Some(text) => reply(out, "200 OK", "text/plain; charset=utf-8", &text),
                None => fail(out, "404 Not Found", "That log isn't there"),
            };
        }
        ("PUT", ["api", "games", system, name]) => {
            upload(root, system, name, req.length, reader)
        }
        ("DELETE", ["api", "games", system, name]) => delete(root, system, name),
        ("PUT", ["api", "label", system, name]) => label(root, system, name, req.length, reader),
        ("DELETE", ["api", "label", system, name]) => drop_label(root, system, name),
        ("GET", ["api", "label", system, name]) => {
            return match label_file(root, system, name).and_then(|f| fs::read(f).ok()) {
                Some(png) => reply(out, "200 OK", "image/png", &png),
                None => fail(out, "404 Not Found", "No label"),
            };
        }
        ("POST", ["api", "rename", system, name]) => {
            let mut to = String::new();
            if req.length > 1024
                || (&mut *reader)
                    .take(req.length)
                    .read_to_string(&mut to)
                    .is_err()
            {
                Err("Bad name")
            } else {
                rename(root, system, name, to.trim())
            }
        }
        _ => return fail(out, "404 Not Found", "Not found"),
    };
    tell_console(req.method.as_str(), path.as_slice(), result.is_ok());
    match result {
        Ok(()) => reply(out, "200 OK", "text/plain", b"OK"),
        Err(why) => fail(out, "409 Conflict", why),
    }
}

/// What the page just did, one word a line, for the console to show as a toast (slot's
/// app.rs reads and empties it while the Manager is open).
pub const NEWS: &str = "/tmp/gravy-manager-news";

fn tell_console(method: &str, path: &[&str], ok: bool) {
    let word = match (method, path, ok) {
        ("PUT", ["api", "games", system, _], true) => match Place::named(system) {
            Some(Place::Games(_)) => "game",
            Some(Place::Bios) => "bios",
            Some(Place::Wallpapers) => "wallpaper",
            _ => return,
        },
        ("PUT", ["api", "label", ..], true) => "label",
        ("PUT", ["api", "games" | "label", ..] | ["api", "update", "zip"], false) => "failed",
        ("DELETE", ["api", "games", ..], true) => "deleted",
        ("POST", ["api", "rename", ..], true) => "renamed",
        _ => return,
    };
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(NEWS) {
        let _ = writeln!(f, "{word}");
    }
}

/// A folder the page looks after: a shelf's games, the BIOS files or the wallpapers.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    Games(Platform),
    Bios,
    Wallpapers,
    /// Taken on the console: looked at, downloaded and deleted, never added or renamed.
    Screenshots,
}

impl Place {
    const ALL: [Place; 8] = [
        Place::Games(Platform::Gb),
        Place::Games(Platform::Gbc),
        Place::Games(Platform::Gba),
        Place::Games(Platform::Nes),
        Place::Games(Platform::Snes),
        Place::Bios,
        Place::Wallpapers,
        Place::Screenshots,
    ];

    fn key(self) -> &'static str {
        match self {
            Place::Games(p) => p.dir_name(),
            Place::Bios => "BIOS",
            Place::Wallpapers => "WALLPAPERS",
            Place::Screenshots => "SCREENSHOTS",
        }
    }

    fn named(key: &str) -> Option<Place> {
        Place::ALL.into_iter().find(|p| p.key() == key)
    }

    fn dir(self, root: &Path) -> PathBuf {
        match self {
            Place::Games(p) => root.join("Games").join(p.dir_name()),
            Place::Bios => root.join("BIOS"),
            Place::Wallpapers => root.join("Wallpapers"),
            Place::Screenshots => root.join("Screenshots"),
        }
    }

    fn accepts(self, name: &str) -> bool {
        let ext = |list: &[&str]| {
            Path::new(name)
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| list.iter().any(|k| e.eq_ignore_ascii_case(k)))
        };
        match self {
            Place::Games(p) => p.accepts(Path::new(name)),
            // slot_ files are made by slot itself from the Super Game Boy program.
            Place::Bios => ext(&["bin", "sfc", "smc"]) && !name.starts_with("slot_"),
            Place::Wallpapers | Place::Screenshots => ext(&["png"]),
        }
    }

    fn wrong_kind(self) -> &'static str {
        match self {
            Place::Games(Platform::Gb | Platform::Gbc) => "Only .gb and .gbc files go here",
            Place::Games(Platform::Gba) => "Only .gba files go here",
            Place::Games(Platform::Nes) => "Only .nes files go here",
            Place::Games(Platform::Snes) => "Only .sfc and .smc files go here",
            Place::Bios => "Only .bin, .sfc and .smc files go here",
            Place::Wallpapers | Place::Screenshots => "Only .png pictures go here",
        }
    }
}

/// A packed file (.zip, .7z, ...): the cores only open bare games, so it never goes on.
fn packed(name: &str) -> bool {
    let packs = ["zip", "7z", "rar", "gz", "tar"];
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| packs.iter().any(|k| e.eq_ignore_ascii_case(k)))
}

/// A file name as the card may hold it: one name in its folder, never a path, a hidden file
/// or a file slot would not use.
fn checked(system: &str, name: &str) -> Result<(Place, String), &'static str> {
    let place = Place::named(system).ok_or("Unknown folder")?;
    let name = name.trim();
    // A screenshot taken in a game sits in its system's folder: "GB/Tetris 2026-10-20_09-14-02.png".
    let (sub, leaf) = match (place, name.split_once('/')) {
        (Place::Screenshots, Some((sub, leaf)))
            if Platform::ALL.iter().any(|p| p.dir_name() == sub) =>
        {
            (Some(sub), leaf)
        }
        _ => (None, name),
    };
    let name = leaf;
    let bad = name.is_empty()
        || name.len() > 200
        || name.starts_with('.')
        || name.contains(['/', '\\', '\0'])
        || name.chars().any(char::is_control);
    if bad {
        return Err("That name can't be used");
    }
    if packed(name) {
        return Err("Unzip it first: the console can't open .zip or .7z files");
    }
    if !place.accepts(name) {
        return Err(place.wrong_kind());
    }
    Ok((
        place,
        match sub {
            Some(sub) => format!("{sub}/{name}"),
            None => name.to_owned(),
        },
    ))
}

/// The pictures in a folder, as names and sizes, each with when it was written.
fn pictures(dir: &Path, prefix: &str, place: Place, out: &mut Vec<(String, u64, u64)>) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let Ok(name) = e.file_name().into_string() else {
            continue;
        };
        let Ok(meta) = e.metadata() else {
            continue;
        };
        if meta.is_file() && !name.starts_with('.') && place.accepts(&name) {
            let when = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            out.push((format!("{prefix}{name}"), meta.len(), when));
        }
    }
}

fn list(root: &Path) -> String {
    let mut json = String::from("{\"systems\":{");
    let wear = slot_store::wear::WearBook::read(root);
    for (i, p) in Place::ALL.into_iter().enumerate() {
        let mut found = Vec::new();
        pictures(&p.dir(root), "", p, &mut found);
        if p == Place::Screenshots {
            for sub in Platform::ALL.map(Platform::dir_name) {
                pictures(&p.dir(root).join(sub), &format!("{sub}/"), p, &mut found);
            }
            // Newest first, as the console's own Screenshots page shows them.
            found.sort_by(|a, b| b.2.cmp(&a.2));
        } else {
            found.sort_by_key(|(n, _, _)| slot_store::sort_key(n));
        }
        let games = found;
        if i > 0 {
            json.push(',');
        }
        json.push_str(&format!("\"{}\":[", p.key()));
        // A game says whether it has a label, downloaded or the user's own.
        let labels = match p {
            Place::Games(platform) => Some(label_names(root, platform)),
            _ => None,
        };
        for (j, (name, size, when)) in games.iter().enumerate() {
            if j > 0 {
                json.push(',');
            }
            match (&labels, p) {
                (Some(l), Place::Games(platform)) => {
                    let has = l.contains(&format!("{}.png", stem(name)).to_lowercase());
                    // For sorting on the page: when it came onto the card (the day the shelf
                    // first showed it, else the file's own date) and how long it has been played.
                    let e = wear.get(&slot_store::wear::key(platform, stem(name)));
                    let added = if e.first_seen > 0 { e.first_seen } else { *when };
                    json.push_str(&format!(
                        "[{},{size},{},{added},{}]",
                        quote(name),
                        u8::from(has),
                        e.play_secs
                    ))
                }
                _ => json.push_str(&format!("[{},{size}]", quote(name))),
            }
        }
        json.push(']');
    }
    // The console's logs, for the hidden Logs tab; newest change first.
    let mut logs = Vec::new();
    for e in fs::read_dir(root.join("Logs")).into_iter().flatten().flatten() {
        let (Ok(name), Ok(meta)) = (e.file_name().into_string(), e.metadata()) else {
            continue;
        };
        if meta.is_file() && !name.starts_with('.') {
            logs.push((name, meta.len(), meta.modified().ok()));
        }
    }
    logs.sort_by(|a, b| b.2.cmp(&a.2));
    let logs: Vec<String> = logs.iter().map(|(n, b, _)| format!("[{},{b}]", quote(n))).collect();
    json.push_str(&format!(",\"LOGS\":[{}]", logs.join(",")));
    json.push_str(&format!("}},\"free\":{}}}", free(root)));
    json
}

/// One log from Logs/, only a plain name there; a long one hands out its last 4 MB.
fn log_file(root: &Path, name: &str) -> Option<Vec<u8>> {
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\']) {
        return None;
    }
    let mut f = File::open(root.join("Logs").join(name)).ok()?;
    let len = f.metadata().ok().filter(|m| m.is_file())?.len();
    const MAX: u64 = 4 << 20;
    if len > MAX {
        use std::io::Seek;
        f.seek(std::io::SeekFrom::Start(len - MAX)).ok()?;
    }
    let mut text = Vec::new();
    f.take(MAX).read_to_end(&mut text).ok()?;
    Some(text)
}

pub(crate) fn quote(s: &str) -> String {
    let mut q = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => q.push_str("\\\""),
            '\\' => q.push_str("\\\\"),
            c if (c as u32) < 0x20 => q.push_str(&format!("\\u{:04x}", c as u32)),
            c => q.push(c),
        }
    }
    q.push('"');
    q
}

pub(crate) fn free(root: &Path) -> u64 {
    use std::os::unix::ffi::OsStrExt;
    let Ok(path) = std::ffi::CString::new(root.as_os_str().as_bytes()) else {
        return 0;
    };
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(path.as_ptr(), &mut s) } != 0 {
        return 0;
    }
    s.f_bavail as u64 * s.f_frsize as u64
}

/// Written beside its place under a hidden name, then renamed: a game cut off halfway never
/// reaches the shelf.
fn upload(
    root: &Path,
    system: &str,
    name: &str,
    length: u64,
    body: &mut impl Read,
) -> Result<(), &'static str> {
    let (p, name) = checked(system, name)?;
    if p == Place::Screenshots {
        return Err("Screenshots are taken on the console");
    }
    if length == 0 || length > MAX_UPLOAD {
        return Err("That file is too big to be a game");
    }
    if length + (16 << 20) > free(root) {
        return Err("The card is full");
    }
    let dir = p.dir(root);
    fs::create_dir_all(&dir).map_err(|_| "The card can't be written")?;
    let to = dir.join(&name);
    if exists_any_case(&dir, &name) {
        return Err("A file with that name is already there");
    }
    let part = dir.join(PART);
    let written = (|| -> std::io::Result<()> {
        let mut f = File::create(&part)?;
        let copied = std::io::copy(&mut body.take(length), &mut f)?;
        if copied != length {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        f.sync_all()?;
        fs::rename(&part, &to)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&part);
        return Err("The upload stopped halfway");
    }
    Ok(())
}

const MAX_LABEL: u64 = 8 << 20;

/// A game's own label, a PNG the page made of the picture picked on the phone: it takes the
/// place of any label there was, downloaded or not, under the game's name.
fn label(
    root: &Path,
    system: &str,
    name: &str,
    length: u64,
    body: &mut impl Read,
) -> Result<(), &'static str> {
    let (p, name) = checked(system, name)?;
    let Place::Games(platform) = p else {
        return Err("Only games have labels");
    };
    if !p.dir(root).join(&name).is_file() {
        return Err("That game isn't there");
    }
    if length == 0 || length > MAX_LABEL {
        return Err("That picture is too big");
    }
    let mut png = Vec::with_capacity(length as usize);
    if body.take(length).read_to_end(&mut png).is_err() || png.len() as u64 != length {
        return Err("The upload stopped halfway");
    }
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("That isn't a picture");
    }
    let dir = root.join("Labels").join(platform.dir_name());
    fs::create_dir_all(&dir).map_err(|_| "The card can't be written")?;
    let file = format!("{}.png", stem(&name));
    // The card is FAT and labels are found in any case: the old one goes, whatever its case.
    for e in fs::read_dir(&dir).into_iter().flatten().flatten() {
        if e.file_name().to_str().is_some_and(|n| n.eq_ignore_ascii_case(&file)) {
            let _ = fs::remove_file(e.path());
        }
    }
    slot_store::atomic_write(&dir.join(&file), &png).map_err(|_| "The card can't be written")
}

/// The label files of a platform's games, lower case.
fn label_names(root: &Path, platform: Platform) -> std::collections::HashSet<String> {
    fs::read_dir(root.join("Labels").join(platform.dir_name()))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .map(|n| n.to_lowercase())
        .collect()
}

/// A game's label on the card, in whatever case it was written.
fn label_file(root: &Path, system: &str, name: &str) -> Option<std::path::PathBuf> {
    let (Place::Games(platform), name) = checked(system, name).ok()? else {
        return None;
    };
    let want = format!("{}.png", stem(&name));
    fs::read_dir(root.join("Labels").join(platform.dir_name()))
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_str().is_some_and(|n| n.eq_ignore_ascii_case(&want)))
        .map(|e| e.path())
}

/// The label goes: the console looks one up online again next time.
fn drop_label(root: &Path, system: &str, name: &str) -> Result<(), &'static str> {
    let file = label_file(root, system, name).ok_or("No label")?;
    fs::remove_file(file).map_err(|_| "The card can't be written")
}

/// The card is FAT: two names that differ only in case are the same file.
fn exists_any_case(dir: &Path, name: &str) -> bool {
    fs::read_dir(dir).into_iter().flatten().flatten().any(|e| {
        e.file_name()
            .to_str()
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
    })
}

/// The file goes; a game's saves stay on the card, so adding it back brings them back.
fn delete(root: &Path, system: &str, name: &str) -> Result<(), &'static str> {
    let (p, name) = checked(system, name)?;
    fs::remove_file(p.dir(root).join(name)).map_err(|_| "That file isn't there")
}

fn stem(name: &str) -> &str {
    Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name)
}

/// Renames the game and everything kept under its name: battery saves, save states and label.
fn rename(root: &Path, system: &str, name: &str, to: &str) -> Result<(), &'static str> {
    let (p, from) = checked(system, name)?;
    let (_, to) = checked(system, to)?;
    if p == Place::Screenshots {
        return Err("Screenshots keep the name the console gave them");
    }
    if from == to {
        return Ok(());
    }
    let dir = p.dir(root);
    if !dir.join(&from).is_file() {
        return Err("That file isn't there");
    }
    // Only a change of case may land on its own name.
    if !from.eq_ignore_ascii_case(&to) && exists_any_case(&dir, &to) {
        return Err("A file with that name is already there");
    }
    let (old, new) = (stem(&from), stem(&to));
    if let (Place::Games(p), true) = (p, old != new) {
        slot_store::move_game(root, p, old, new)?;
    }
    fs::rename(dir.join(&from), dir.join(&to)).map_err(|_| "The card can't be written")
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_stay_in_their_folder() {
        assert!(checked("GB", "Tetris.gb").is_ok());
        assert!(checked("GB", "Zelda DX.gbc").is_ok());
        assert!(checked("GB", "../Tetris.gb").is_err());
        assert!(checked("GB", ".hidden.gb").is_err());
        assert!(checked("GB", "Tetris.exe").is_err());
        assert!(checked("GBA", "Tetris.gb").is_err());
        assert!(checked("PSX", "Tetris.gb").is_err());
        assert!(checked("BIOS", "gba_bios.bin").is_ok());
        assert!(checked("BIOS", "slot_sgb_ours.bin").is_err());
        assert!(checked("WALLPAPERS", "Sunset.PNG").is_ok());
        assert!(checked("WALLPAPERS", "Sunset.jpg").is_err());
        assert!(checked("SCREENSHOTS", "GB/Tetris 2026-10-20_09-14-02.png").is_ok());
        assert!(checked("SCREENSHOTS", "slot 2026-10-20_09-14-02.png").is_ok());
        assert!(checked("SCREENSHOTS", "../GB/x.png").is_err());
        assert!(checked("SCREENSHOTS", "Saves/x.png").is_err());
        assert!(checked("WALLPAPERS", "GB/x.png").is_err());
        assert_eq!(decode("Pok%C3%A9mon%20Red.gb").unwrap(), "Pokémon Red.gb");
    }

    #[test]
    fn logs_only_come_from_the_logs_folder() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        fs::create_dir_all(r.join("Logs")).unwrap();
        fs::create_dir_all(r.join("Config")).unwrap();
        fs::write(r.join("Logs/slot.log"), b"hello").unwrap();
        fs::write(r.join("Config/slot.state"), b"secret").unwrap();
        assert_eq!(log_file(r, "slot.log").unwrap(), b"hello");
        assert!(log_file(r, "../Config/slot.state").is_none());
        assert!(log_file(r, "..").is_none());
        assert!(log_file(r, "nope.log").is_none());
        assert!(list(r).contains("\"LOGS\":[[\"slot.log\",5]]"));
    }

    #[test]
    fn a_label_upload_replaces_the_games_label_in_any_case() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        for (dir, file) in [("Games/GB", "Tetris.gb"), ("Labels/GB", "tetris.png")] {
            fs::create_dir_all(r.join(dir)).unwrap();
            fs::write(r.join(dir).join(file), b"x").unwrap();
        }
        let png = b"\x89PNG\r\n\x1a\nrest";
        label(r, "GB", "Tetris.gb", png.len() as u64, &mut &png[..]).unwrap();
        let names: Vec<_> = fs::read_dir(r.join("Labels/GB"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, ["Tetris.png"]);
        assert_eq!(fs::read(r.join("Labels/GB/Tetris.png")).unwrap(), png);
        assert!(label(r, "GB", "Tetris.gb", 4, &mut &b"GIF8"[..]).is_err());
        assert!(label(r, "GB", "Nope.gb", png.len() as u64, &mut &png[..]).is_err());
        assert!(label(r, "WALLPAPERS", "a.png", png.len() as u64, &mut &png[..]).is_err());
        assert!(list(r).contains("[\"Tetris.gb\",1,1,"));
        drop_label(r, "GB", "Tetris.gb").unwrap();
        assert!(!r.join("Labels/GB/Tetris.png").exists());
        assert!(list(r).contains("[\"Tetris.gb\",1,0,"));
        // Sorting on the page: the day the shelf first showed it, and its play time.
        fs::create_dir_all(r.join("Config")).unwrap();
        fs::write(r.join("Config/wear.tsv"), "GB/Tetris\t600\t0\t1234\t0\n").unwrap();
        assert!(list(r).contains("[\"Tetris.gb\",1,0,1234,600]"));
    }

    #[test]
    fn rename_takes_saves_states_and_label_along() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        for (dir, file) in [
            ("Games/GB", "Tetris.gb"),
            ("Saves/GB", "Tetris.sav"),
            ("States/GB/gambatte/Tetris", "1.state"),
            ("Labels/GB", "tetris.png"),
            ("Config", "cart_shell.txt"),
        ] {
            fs::create_dir_all(r.join(dir)).unwrap();
            fs::write(r.join(dir).join(file), b"x").unwrap();
        }
        fs::write(
            r.join(slot_store::CART_SHELL_FILE),
            "Tetris = round 3a7bd5 clear\n",
        )
        .unwrap();
        rename(r, "GB", "Tetris.gb", "Tetris DX.gb").unwrap();
        assert!(r.join("Games/GB/Tetris DX.gb").is_file());
        assert!(r.join("Saves/GB/Tetris DX.sav").is_file());
        assert!(r.join("States/GB/gambatte/Tetris DX/1.state").is_file());
        assert!(r.join("Labels/GB/Tetris DX.png").is_file());
        assert_eq!(
            fs::read_to_string(r.join(slot_store::CART_SHELL_FILE)).unwrap(),
            "Tetris DX = round 3a7bd5 clear\n"
        );
        assert_eq!(
            fs::read_to_string(r.join(slot_store::rename::RENAMED)).unwrap(),
            "GB\tTetris\tTetris DX\n"
        );
        assert!(rename(r, "GB", "Tetris DX.gb", "../x.gb").is_err());
    }

    #[test]
    fn a_remembered_phone_is_known() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        fs::create_dir_all(r.join("Config")).unwrap();
        let key = remember(r).unwrap();
        assert!(known_key(r, &key));
        assert!(!known_key(r, "0123456789abcdef0123456789abcdef"));
        assert!(!known_key(r, ""));
        for _ in 0..MAX_KEYS {
            remember(r).unwrap();
        }
        assert!(!known_key(r, &key));
        assert_eq!(keys(r).len(), MAX_KEYS);
    }

    #[test]
    fn upload_lands_whole_or_not_at_all() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        upload(r, "GBA", "Game.gba", 4, &mut &b"abcd"[..]).unwrap();
        assert_eq!(fs::read(r.join("Games/GBA/Game.gba")).unwrap(), b"abcd");
        assert!(upload(r, "GBA", "game.GBA", 4, &mut &b"abcd"[..]).is_err());
        assert!(upload(r, "GBA", "Short.gba", 8, &mut &b"abcd"[..]).is_err());
        assert!(!r.join("Games/GBA/Short.gba").exists());
        assert!(!r.join("Games/GBA").join(PART).exists());
        delete(r, "GBA", "Game.gba").unwrap();
        assert!(list(r).contains("\"GBA\":[]"));
    }
}
