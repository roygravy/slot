//! The ROM Manager: a small web page on the home network for adding, renaming and deleting
//! games on the card. It only listens while the console shows its screen, and every change
//! asks for the PIN shown there.
use slot_store::Platform;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

const PAGE: &str = include_str!("../web/index.html");
/// The boot logo's own slot., x and GRAVY, side by side.
const LOGO: &[u8] = include_bytes!("../web/logo.png");
/// Larger than any GBA game (32 MB), so a wrong file is turned away before it fills the card.
const MAX_UPLOAD: u64 = 64 << 20;
const MAX_HEAD: usize = 16 << 10;
const MAX_CONNECTIONS: usize = 8;
/// Wrong PINs before the page stops taking any, until the screen is opened again.
const MAX_WRONG: usize = 10;
/// Where a half-sent game waits before it gets its name.
const PART: &str = ".rom-manager.part";

pub struct Web {
    pub owner: String,
    pub address: String,
    pub pin: String,
    stop: Arc<AtomicBool>,
}

struct Shared {
    root: PathBuf,
    pin: String,
    stop: Arc<AtomicBool>,
    wrong: AtomicUsize,
    open: AtomicUsize,
}

impl Web {
    pub fn start(root: &Path, owner: &str, ip: Ipv4Addr) -> Result<Web, &'static str> {
        // Port 80 so the address needs no number; another server there leaves 8080.
        let (listener, port) = match TcpListener::bind(("0.0.0.0", 80)) {
            Ok(l) => (l, 80),
            Err(_) => (
                TcpListener::bind(("0.0.0.0", 8080)).map_err(|_| "WEB_PORT_BUSY")?,
                8080,
            ),
        };
        listener
            .set_nonblocking(true)
            .map_err(|_| "WEB_PORT_BUSY")?;
        let pin = pin();
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Shared {
            root: root.to_path_buf(),
            pin: pin.clone(),
            stop: stop.clone(),
            wrong: AtomicUsize::new(0),
            open: AtomicUsize::new(0),
        });
        std::thread::spawn(move || accept(listener, shared));
        let address = match port {
            80 => format!("http://{ip}"),
            p => format!("http://{ip}:{p}"),
        };
        Ok(Web {
            owner: owner.into(),
            address,
            pin,
            stop,
        })
    }

    pub fn word(&self) -> String {
        format!("WEB {} {}", self.address, self.pin)
    }
}

impl Drop for Web {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn pin() -> String {
    let mut b = [0u8; 4];
    let _ = File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b));
    format!("{:04}", u32::from_le_bytes(b) % 10_000)
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
    length: u64,
}

fn read_head(reader: &mut BufReader<TcpStream>) -> Option<Request> {
    let mut line = String::new();
    let mut taken = reader.read_line(&mut line).ok()?;
    let mut words = line.split_whitespace();
    let method = words.next()?.to_owned();
    let target = words.next()?.split('?').next()?.to_owned();
    let mut pin = String::new();
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
    let Some(req) = read_head(&mut reader) else {
        return fail(&mut out, "400 Bad Request", "Bad request");
    };
    let path: Vec<&str> = req.path.iter().map(String::as_str).collect();
    match (req.method.as_str(), path.as_slice()) {
        ("GET", []) => {
            return reply(
                &mut out,
                "200 OK",
                "text/html; charset=utf-8",
                PAGE.as_bytes(),
            )
        }
        ("GET", ["logo.png"]) => return reply(&mut out, "200 OK", "image/png", LOGO),
        ("GET", ["favicon.ico"]) => return fail(&mut out, "404 Not Found", ""),
        _ => {}
    }
    if shared.wrong.load(Ordering::SeqCst) >= MAX_WRONG {
        return fail(
            &mut out,
            "403 Forbidden",
            "Too many wrong PINs. Open ROM Manager on the console again.",
        );
    }
    if req.pin != shared.pin {
        shared.wrong.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(Duration::from_secs(1));
        return fail(&mut out, "401 Unauthorized", "Wrong PIN");
    }
    let root = &shared.root;
    let result = match (req.method.as_str(), path.as_slice()) {
        ("GET", ["api", "list"]) => {
            return reply(
                &mut out,
                "200 OK",
                "application/json",
                list(root).as_bytes(),
            )
        }
        // Wallpapers are shown on the page; nothing else on the card is handed out.
        ("GET", ["api", "games", "WALLPAPERS", name]) => {
            let picture = checked("WALLPAPERS", name)
                .ok()
                .and_then(|(p, n)| fs::read(p.dir(root).join(n)).ok());
            return match picture {
                Some(png) => reply(&mut out, "200 OK", "image/png", &png),
                None => fail(&mut out, "404 Not Found", "That file isn't there"),
            };
        }
        ("PUT", ["api", "games", system, name]) => {
            upload(root, system, name, req.length, &mut reader)
        }
        ("DELETE", ["api", "games", system, name]) => delete(root, system, name),
        ("POST", ["api", "rename", system, name]) => {
            let mut to = String::new();
            if req.length > 1024
                || (&mut reader)
                    .take(req.length)
                    .read_to_string(&mut to)
                    .is_err()
            {
                Err("Bad name")
            } else {
                rename(root, system, name, to.trim())
            }
        }
        _ => return fail(&mut out, "404 Not Found", "Not found"),
    };
    match result {
        Ok(()) => reply(&mut out, "200 OK", "text/plain", b"OK"),
        Err(why) => fail(&mut out, "409 Conflict", why),
    }
}

/// A folder the page looks after: a shelf's games, the BIOS files or the wallpapers.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    Games(Platform),
    Bios,
    Wallpapers,
}

impl Place {
    const ALL: [Place; 7] = [
        Place::Games(Platform::Gb),
        Place::Games(Platform::Gbc),
        Place::Games(Platform::Gba),
        Place::Games(Platform::Nes),
        Place::Games(Platform::Snes),
        Place::Bios,
        Place::Wallpapers,
    ];

    fn key(self) -> &'static str {
        match self {
            Place::Games(p) => p.dir_name(),
            Place::Bios => "BIOS",
            Place::Wallpapers => "WALLPAPERS",
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
            Place::Wallpapers => ext(&["png"]),
        }
    }

    fn wrong_kind(self) -> &'static str {
        match self {
            Place::Games(Platform::Gb | Platform::Gbc) => "Only .gb and .gbc files go here",
            Place::Games(Platform::Gba) => "Only .gba files go here",
            Place::Games(Platform::Nes) => "Only .nes files go here",
            Place::Games(Platform::Snes) => "Only .sfc and .smc files go here",
            Place::Bios => "Only .bin, .sfc and .smc files go here",
            Place::Wallpapers => "Only .png pictures go here",
        }
    }
}

/// A file name as the card may hold it: one name in its folder, never a path, a hidden file
/// or a file slot would not use.
fn checked(system: &str, name: &str) -> Result<(Place, String), &'static str> {
    let place = Place::named(system).ok_or("Unknown folder")?;
    let name = name.trim();
    let bad = name.is_empty()
        || name.len() > 200
        || name.starts_with('.')
        || name.contains(['/', '\\', '\0'])
        || name.chars().any(char::is_control);
    if bad {
        return Err("That name can't be used");
    }
    if !place.accepts(name) {
        return Err(place.wrong_kind());
    }
    Ok((place, name.to_owned()))
}

fn list(root: &Path) -> String {
    let mut json = String::from("{\"systems\":{");
    for (i, p) in Place::ALL.into_iter().enumerate() {
        let mut games: Vec<(String, u64)> = fs::read_dir(p.dir(root))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                let shown = !name.starts_with('.') && p.accepts(&name);
                shown.then(|| (name, e.metadata().map(|m| m.len()).unwrap_or(0)))
            })
            .collect();
        games.sort_by_key(|(n, _)| slot_store::sort_key(n));
        if i > 0 {
            json.push(',');
        }
        json.push_str(&format!("\"{}\":[", p.key()));
        for (j, (name, size)) in games.iter().enumerate() {
            if j > 0 {
                json.push(',');
            }
            json.push_str(&format!("[{},{size}]", quote(name)));
        }
        json.push(']');
    }
    json.push_str(&format!("}},\"free\":{}}}", free(root)));
    json
}

fn quote(s: &str) -> String {
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

fn free(root: &Path) -> u64 {
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
        let taken = |path: PathBuf| path.exists() && !old.eq_ignore_ascii_case(new);
        let saves = root.join("Saves").join(p.dir_name());
        let labels = root.join("Labels").join(p.dir_name());
        let states: Vec<PathBuf> = fs::read_dir(root.join("States").join(p.dir_name()))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .collect();
        let clash = ["sav", "srm"]
            .iter()
            .any(|e| taken(saves.join(format!("{new}.{e}"))))
            || states.iter().any(|core| taken(core.join(new)));
        if clash {
            return Err("Saves for a game with that name are already there");
        }
        for ext in ["sav", "srm"] {
            move_if_there(
                &saves.join(format!("{old}.{ext}")),
                &saves.join(format!("{new}.{ext}")),
            );
        }
        for core in &states {
            move_if_there(&core.join(old), &core.join(new));
        }
        if let Some(label) = slot_store::listing(&labels).get(&format!("{old}.png").to_lowercase())
        {
            move_if_there(&labels.join(label), &labels.join(format!("{new}.png")));
        }
    }
    fs::rename(dir.join(&from), dir.join(&to)).map_err(|_| "The card can't be written")
}

fn move_if_there(from: &Path, to: &Path) {
    if from.exists() {
        if let Err(e) = fs::rename(from, to) {
            eprintln!("slot-services: rom manager: {}: {e}", from.display());
        }
    }
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
        assert_eq!(decode("Pok%C3%A9mon%20Red.gb").unwrap(), "Pokémon Red.gb");
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
        ] {
            fs::create_dir_all(r.join(dir)).unwrap();
            fs::write(r.join(dir).join(file), b"x").unwrap();
        }
        rename(r, "GB", "Tetris.gb", "Tetris DX.gb").unwrap();
        assert!(r.join("Games/GB/Tetris DX.gb").is_file());
        assert!(r.join("Saves/GB/Tetris DX.sav").is_file());
        assert!(r.join("States/GB/gambatte/Tetris DX/1.state").is_file());
        assert!(r.join("Labels/GB/Tetris DX.png").is_file());
        assert!(rename(r, "GB", "Tetris DX.gb", "../x.gb").is_err());
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
