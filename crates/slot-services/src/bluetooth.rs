//! Bluetooth audio: the radio on and off, finding headphones nearby, pairing, connecting and
//! forgetting them, through BlueZ's `bluetoothctl`. Sound reaches them through bluez-alsa;
//! the frontend reads `bt-audio` in the run directory to know where to send it.
//!
//! The tools come from BaseOS when it has them, else from `System/slot-bt` on the card,
//! started through the system loader like `iw` (exFAT carries no exec bit).
use crate::config::hex;
use crate::system::{output, private_write, Process};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long one search for devices nearby listens.
const SCAN_SECS: &str = "12";
/// How often the connected headphones are looked at while the radio is on.
const WATCH: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub mac: String,
    pub name: String,
    pub paired: bool,
    pub connected: bool,
    pub audio: bool,
    /// BlueZ's icon for it, empty when it gave none.
    pub icon: String,
    /// For the log only: what a nameless device says about itself.
    pub extra: String,
}

/// `Device AA:BB:CC:DD:EE:FF Name with spaces` lines of `bluetoothctl devices`.
pub fn parse_devices(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| {
            let rest = l.trim().strip_prefix("Device ")?;
            let (mac, name) = rest.split_once(' ').unwrap_or((rest, ""));
            valid_mac(mac).then(|| (mac.to_owned(), name.trim().to_owned()))
        })
        .collect()
}

/// The icon BlueZ gives the device (audio-headphones, input-gaming, phone...), if any.
pub fn icon(text: &str) -> &str {
    text.lines()
        .find_map(|l| l.trim().strip_prefix("Icon:"))
        .map(str::trim)
        .unwrap_or("")
}

/// Something that is surely not for sound: a pad, keyboard, phone or computer. Devices found
/// over Bluetooth LE often carry no icon at all (AirPods among them), so those stay.
pub fn surely_not_audio(icon: &str) -> bool {
    icon.starts_with("input") || icon == "phone" || icon == "computer"
}

/// For the log: what a nameless device tells about itself (class, appearance, maker, signal).
pub fn extra(text: &str) -> String {
    let mut out = Vec::new();
    let mut lines = text.lines().map(str::trim).peekable();
    while let Some(l) = lines.next() {
        for key in ["Class:", "Appearance:", "AddressType:"] {
            if let Some(v) = l.strip_prefix(key) {
                out.push(format!("{}{}", &key[..3].to_lowercase(), v.trim()));
            }
        }
        if let Some(v) = l.strip_prefix("ManufacturerData Key:") {
            // The first value line holds the maker's own type byte (Apple 0x07: AirPods).
            let first = lines.peek().map(|n| n.split_whitespace().next().unwrap_or("")).unwrap_or("");
            out.push(format!("mfr{} {}", v.trim(), first));
        }
        if l.starts_with("UUID:") {
            out.push(l.trim_start_matches("UUID:").split('(').next().unwrap_or("").trim().to_owned());
        }
    }
    out.join(" ")
}

/// BlueZ names a device it has no name for after its address, with dashes.
pub fn unnamed(name: &str) -> bool {
    name.is_empty() || valid_mac(&name.replace('-', ":"))
}

/// The fields of `bluetoothctl info` that matter here: paired, connected, and whether it
/// plays sound (an audio icon or an Audio Sink profile).
pub fn parse_info(text: &str) -> (bool, bool, bool) {
    let yes = |key: &str| {
        text.lines().any(|l| {
            l.trim()
                .strip_prefix(key)
                .is_some_and(|v| v.trim() == "yes")
        })
    };
    let audio = text.lines().any(|l| {
        let l = l.trim();
        l.strip_prefix("Icon:")
            .is_some_and(|v| v.trim().starts_with("audio"))
            || (l.starts_with("UUID:") && l.contains("Audio Sink"))
    });
    (yes("Paired:"), yes("Connected:"), audio)
}

pub fn valid_mac(mac: &str) -> bool {
    mac.len() == 17
        && mac.split(':').count() == 6
        && mac
            .split(':')
            .all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// `0 BT mac,hexname,flags ...`: flags p paired, c connected, a audio. Connected first,
/// then paired, then those that say they play sound, then the rest; nameless ones and ones
/// that are surely no headphones (pads, phones, computers) stay out unless paired.
pub fn reply(mut devices: Vec<Device>) -> String {
    devices.retain(|d| {
        d.paired || d.audio || (!unnamed(&d.name) && !surely_not_audio(&d.icon))
    });
    for d in devices.iter_mut().filter(|d| unnamed(&d.name)) {
        d.name = "Headphones".into();
    }
    devices.sort_by_key(|d| (!d.connected, !d.paired, !d.audio));
    let words: Vec<String> = devices
        .iter()
        .take(16)
        .map(|d| {
            let mut flags = String::new();
            for (on, c) in [(d.paired, 'p'), (d.connected, 'c'), (d.audio, 'a')] {
                if on {
                    flags.push(c);
                }
            }
            format!("{},{},{}", d.mac, hex(&d.name), flags)
        })
        .collect();
    format!("BT {}", words.join(" "))
}

/// A tool from BaseOS, else the card's own copy through the loader.
fn tool(name: &str) -> Option<Command> {
    for dir in [
        "/usr/bin",
        "/usr/sbin",
        "/usr/libexec/bluetooth",
        "/usr/lib/bluetooth",
    ] {
        let p = Path::new(dir).join(name);
        if p.is_file() {
            return Some(Command::new(p));
        }
    }
    let dir = PathBuf::from(std::env::var_os("SLOT_ROOT")?).join("System/slot-bt");
    let p = dir.join(name);
    p.is_file().then(|| {
        let mut c = Command::new("/lib/ld-linux-aarch64.so.1");
        c.arg("--library-path").arg(&dir).arg(p);
        c
    })
}

/// Logs/bluetooth.log on the card: what the radio did, for finding out why on a device.
pub fn log(line: &str) {
    let Some(root) = std::env::var_os("SLOT_ROOT") else { return };
    let path = PathBuf::from(root).join("Logs/bluetooth.log");
    // Kept small: a fresh file once it passes 64 KB.
    if fs::metadata(&path).is_ok_and(|m| m.len() > 64 * 1024) {
        let _ = fs::remove_file(&path);
    }
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        let _ = writeln!(f, "{secs} {line}");
    }
}

fn short(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(240).collect()
}

fn ctl(args: &[&str]) -> Result<String, &'static str> {
    use std::io::Read;
    let mut c = tool("bluetoothctl").ok_or("BT_TOOL_UNAVAILABLE")?;
    // With bluetoothd gone bluetoothctl waits for it forever, and with it this whole service
    // (Wi-Fi, the Manager, the clock): every short command gets a limit.
    let mut child = c
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "BT_TOOL_UNAVAILABLE")?;
    let read = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut text = Vec::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_end(&mut text);
            }
            String::from_utf8_lossy(&text).into_owned()
        })
    };
    let out = read(child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let err = read(child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let end = Instant::now() + CTL_LIMIT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < end => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let mut text = out.join().unwrap_or_default();
    let Some(status) = status else {
        return Ok(format!("(no answer in {} s) {}", CTL_LIMIT.as_secs(), short(&text)));
    };
    if text.trim().is_empty() {
        // Nothing said: why is on stderr, for the log.
        text = format!("(exit {:?}) {}", status.code(), err.join().unwrap_or_default());
    }
    Ok(text)
}

/// The longest a short bluetoothctl command may take.
const CTL_LIMIT: Duration = Duration::from_secs(5);

/// One pair, trust, connect, disconnect or remove. bluetoothctl keeps its words to itself
/// until it quits and waits out its whole timeout even when it is done, so the device's own
/// state says when it is done; its words, when it does quit, say why not.
fn step(step: &str, mac: &str, limit: Duration) -> (bool, String) {
    use std::io::Read;
    let Some(mut c) = tool("bluetoothctl") else {
        return (false, "no bluetoothctl".into());
    };
    let secs = limit.as_secs().to_string();
    let Ok(mut child) = c
        .args(["--agent", "NoInputNoOutput", "--timeout", &secs, step, mac])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return (false, "no bluetoothctl".into());
    };
    let words = Arc::new(Mutex::new(String::new()));
    if let Some(mut out) = child.stdout.take() {
        let words = words.clone();
        std::thread::spawn(move || {
            let mut text = String::new();
            let _ = out.read_to_string(&mut text);
            *words.lock().unwrap() = text;
        });
    }
    let end = Instant::now() + limit + Duration::from_secs(2);
    let result = loop {
        std::thread::sleep(Duration::from_millis(400));
        if let Ok(Some(_)) = child.try_wait() {
            std::thread::sleep(Duration::from_millis(100));
            let text = words.lock().unwrap().clone();
            let info = ctl(&["info", mac]).unwrap_or_default();
            break (went_well(&text) || done(step, &info), text);
        }
        let info = ctl(&["info", mac]).unwrap_or_default();
        if done(step, &info) {
            break (true, "(state)".into());
        }
        if Instant::now() >= end {
            break (false, "(timed out)".into());
        }
    };
    let _ = child.kill();
    let _ = child.wait();
    result
}

/// What `bluetoothctl info` shows once the step has done its work.
pub fn done(step: &str, info: &str) -> bool {
    let has = |key: &str, v: &str| {
        info.lines()
            .any(|l| l.trim().strip_prefix(key).is_some_and(|x| x.trim() == v))
    };
    match step {
        "pair" => has("Paired:", "yes"),
        "trust" => has("Trusted:", "yes"),
        "connect" => has("Connected:", "yes") && has("Paired:", "yes"),
        "disconnect" => has("Connected:", "no"),
        "remove" => info.contains("not available"),
        _ => false,
    }
}

/// BlueZ keeps its pairings in /var/lib/bluetooth, which BaseOS forgets at every start:
/// the headphones still know us, we no longer know them, and they turn us away. A copy
/// goes on the card (as one file: the card cannot hold names with colons).
const KEYS_DIR: &str = "/var/lib/bluetooth";

fn keys_file() -> Option<PathBuf> {
    std::env::var_os("SLOT_ROOT").map(|r| PathBuf::from(r).join("System/bluetooth-pairings"))
}

/// Every file under `dir` but the cache, as `path\nlength\nbytes`.
pub fn pack(dir: &Path) -> Vec<u8> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<u8>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let p = e.path();
            if e.file_name() == "cache" {
                continue;
            }
            if p.is_dir() {
                walk(base, &p, out);
            } else if let (Ok(bytes), Ok(rel)) = (fs::read(&p), p.strip_prefix(base)) {
                out.extend_from_slice(format!("{}\n{}\n", rel.display(), bytes.len()).as_bytes());
                out.extend_from_slice(&bytes);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out
}

/// The files `pack` made, back under `dir`; never outside it.
pub fn unpack(dir: &Path, mut data: &[u8]) -> usize {
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    let mut n = 0;
    let line = |d: &mut &[u8]| -> Option<String> {
        let i = d.iter().position(|b| *b == b'\n')?;
        let s = String::from_utf8(d[..i].to_vec()).ok()?;
        *d = &d[i + 1..];
        Some(s)
    };
    while let (Some(rel), Some(len)) = (line(&mut data), line(&mut data)) {
        let Ok(len) = len.parse::<usize>() else { break };
        if len > data.len() {
            break;
        }
        let (bytes, rest) = data.split_at(len);
        data = rest;
        let rel = Path::new(&rel);
        if rel.is_absolute() || rel.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
            continue;
        }
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            let _ = fs::DirBuilder::new().recursive(true).mode(0o700).create(parent);
        }
        let written = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&path)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(bytes)
            });
        if written.is_ok() {
            n += 1;
        }
    }
    n
}

fn keep_pairings() {
    if let Some(f) = keys_file() {
        let data = pack(Path::new(KEYS_DIR));
        if fs::read(&f).ok().as_deref() != Some(&data[..]) {
            let ok = fs::write(&f, &data).is_ok();
            log(&format!("pairings kept {ok} ({} bytes)", data.len()));
        }
    }
}

fn bring_back_pairings() {
    if let Some(data) = keys_file().and_then(|f| fs::read(f).ok()) {
        let n = unpack(Path::new(KEYS_DIR), &data);
        log(&format!("pairings back: {n} files"));
    }
}

fn went_well(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("successful") || t.contains("succeeded") || t.contains("device has been removed")
}

/// The system bus BlueZ and bluez-alsa talk over.
fn bus_socket() -> Option<&'static str> {
    ["/run/dbus/system_bus_socket", "/var/run/dbus/system_bus_socket"]
        .into_iter()
        .find(|p| Path::new(p).exists())
}

fn daemon_log() -> Option<PathBuf> {
    std::env::var_os("SLOT_ROOT").map(|r| PathBuf::from(r).join("Logs/bluetooth-daemons.log"))
}

fn running(name: &str) -> bool {
    fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| {
            fs::read(e.path().join("cmdline"))
                .map(|c| {
                    let first = c.split(|b| *b == 0).next().unwrap_or(&[]);
                    String::from_utf8_lossy(first).ends_with(name)
                })
                .unwrap_or(false)
        })
}

fn rfkill(block: bool) {
    let _ = output(
        "rfkill",
        &[if block { "block" } else { "unblock" }, "bluetooth"],
    );
}

/// A UART chip only loads its firmware from cold, and an old rtk_hciattach still holds the
/// port: end that one, cut the chip's power through rfkill (a real power cut on the H700),
/// and attach it afresh.
fn reset_radio() {
    let _ = output("killall", &["rtk_hciattach"]);
    std::thread::sleep(Duration::from_millis(300));
    rfkill(true);
    std::thread::sleep(Duration::from_millis(800));
    rfkill(false);
    std::thread::sleep(Duration::from_millis(500));
    attach();
}

/// The H700's 8821cs speaks Bluetooth over a UART; BaseOS may not attach it.
fn attach() {
    match tool("rtk_hciattach") {
        Some(mut c) => {
            let started = c
                .args(["-n", "-s", "115200", "/dev/ttyS1", "rtk_h5"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .is_ok();
            log(&format!("rtk_hciattach ttyS1 started {started}"));
            // The firmware takes a few seconds to load.
            for _ in 0..16 {
                if hci_up() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }
        None => log("no rtk_hciattach"),
    }
}

fn hci_up() -> bool {
    fs::read_dir("/sys/class/bluetooth")
        .map(|mut d| d.next().is_some())
        .unwrap_or(false)
}

const INFO_FOR: Duration = Duration::from_secs(10);
const RETRY: Duration = Duration::from_secs(30);

pub struct Bluetooth {
    run: PathBuf,
    pub enabled: bool,
    pub error: &'static str,
    daemons: Vec<Process>,
    scan: Option<std::process::Child>,
    job: Arc<Mutex<&'static str>>,
    audio: Option<(String, String)>,
    watch_at: Instant,
    last_heard: Mutex<String>,
    /// On but the radio did not come up: try again now and then, the switch stays on.
    retry_at: Option<Instant>,
    /// `bluetoothctl info` per device, with when it was asked.
    info: Mutex<std::collections::HashMap<String, (Instant, String)>>,
}

impl Bluetooth {
    pub fn new(run: &Path) -> Self {
        Self {
            run: run.to_owned(),
            enabled: false,
            error: "",
            daemons: Vec::new(),
            scan: None,
            job: Arc::new(Mutex::new("NONE")),
            audio: None,
            watch_at: Instant::now(),
            last_heard: Mutex::new(String::new()),
            retry_at: None,
            info: Mutex::new(Default::default()),
        }
    }

    /// On at the service's start, a moment later from `tick`: the radio takes seconds, and
    /// the frontend's first questions (Wi-Fi, the clock) are answered first.
    pub fn enable_soon(&mut self) {
        self.enabled = true;
        self.error = "";
        self.retry_at = Some(Instant::now() + Duration::from_secs(3));
    }

    pub fn enable(&mut self, on: bool) {
        self.enabled = on;
        self.error = "";
        if on {
            log("on");
            rfkill(false);
            if !hci_up() {
                reset_radio();
            }
            log(&format!("hci up {}", hci_up()));
            if bus_socket().is_none() {
                // No system bus: BlueZ cannot run without one. Start it if BaseOS has it.
                let has = |p: &str| Path::new(p).exists();
                log(&format!(
                    "no system bus; dbus-daemon {}, system.conf {}, bluetooth.conf {}",
                    tool("dbus-daemon").is_some(),
                    has("/usr/share/dbus-1/system.conf") || has("/etc/dbus-1/system.conf"),
                    has("/etc/dbus-1/system.d/bluetooth.conf")
                        || has("/usr/share/dbus-1/system.d/bluetooth.conf")
                ));
                let _ = fs::create_dir_all("/run/dbus");
                self.start("dbus-daemon", &["--system", "--nofork", "--nopidfile"]);
                for _ in 0..10 {
                    if bus_socket().is_some() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(300));
                }
            }
            log(&format!("system bus {:?}", bus_socket()));
            if !running("bluetoothd") {
                bring_back_pairings();
            }
            self.start("bluetoothd", &["-n"]);
            if !running("bluealsad") && !running("bluealsa") {
                if tool("bluealsad").is_some() {
                    self.start("bluealsad", &["-p", "a2dp-source"]);
                } else {
                    self.start("bluealsa", &["-p", "a2dp-source"]);
                }
            }
            std::thread::sleep(Duration::from_millis(800));
            log(&format!(
                "bluetoothd running {}, bluealsa running {}",
                running("bluetoothd"),
                running("bluealsa") || running("bluealsad")
            ));
            let mut powered = self.power_on();
            if !powered && self.error.is_empty() {
                // A controller that lost its firmware (radio blocked, chip reset) keeps its hci
                // entry but never answers: attach it again from scratch and try once more.
                // A UART chip only loads its firmware from cold: power it off and on through
                // rfkill (that is a real power cut on the H700), then attach it afresh.
                log("no answer: power-cycling the radio and attaching it again");
                reset_radio();
                log(&format!("hci up {}", hci_up()));
                powered = self.power_on();
            }
            if self.error.is_empty() && !powered {
                self.error = if hci_up() { "BT_NO_POWER" } else { "BT_NO_RADIO" };
            }
            log(&format!("powered {powered} {}", self.error));
            self.retry_at = (!powered).then(|| Instant::now() + RETRY);
        } else {
            log("off");
            self.retry_at = None;
            self.stop_scan();
            let _ = ctl(&["power", "off"]);
            // The bus, bluetoothd and bluealsa keep running: stopping the bus left its socket
            // behind and every later On failed on it. No rfkill block either: on the H700 it
            // cuts the chip's power, and the controller comes back without its firmware.
            self.set_audio(None);
        }
    }

    /// bluetoothd needs a moment before it has the controller: try for a few seconds.
    fn power_on(&mut self) -> bool {
        for _ in 0..8 {
            std::thread::sleep(Duration::from_millis(500));
            match ctl(&["power", "on"]) {
                Err(e) => {
                    log(&format!("power on: {e}"));
                    self.error = "BT_TOOL_UNAVAILABLE";
                    return false;
                }
                Ok(out) => {
                    if out.contains("succeeded") {
                        return true;
                    }
                    log(&format!("power on: {}", short(&out)));
                }
            }
        }
        false
    }

    fn start(&mut self, name: &str, args: &[&str]) {
        if running(name) {
            return;
        }
        let Some(c) = tool(name) else { return };
        let program = c.get_program().to_string_lossy().into_owned();
        let mut all: Vec<String> = c
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        all.extend(args.iter().map(|a| a.to_string()));
        let all: Vec<&str> = all.iter().map(String::as_str).collect();
        let logged = daemon_log();
        if let Some(l) = &logged {
            if fs::metadata(l).is_ok_and(|m| m.len() > 64 * 1024) {
                let _ = fs::remove_file(l);
            }
            let _ = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(l)
                .and_then(|mut f| {
                    use std::io::Write;
                    writeln!(f, "--- {name} {}", all.join(" "))
                });
        }
        match Process::spawn_to(
            &program,
            &all,
            self.run.join(format!("bt-{name}.pid")),
            logged.as_deref(),
        ) {
            Ok(p) => {
                if name.starts_with("bluealsa") || name == "bluetoothd" {
                    // Sound must not wait on a busy emulator: the encoder runs ahead of it.
                    // SAFETY: plain syscall on a pid of our own child.
                    unsafe { libc::setpriority(libc::PRIO_PROCESS, p.pid(), -10) };
                }
                self.daemons.push(p)
            }
            Err(e) => log(&format!("{name} did not start: {e}")),
        }
    }

    /// Dropping the child does not end it: kill it, and end discovery, or bluetoothd stays
    /// busy and the next power on is refused.
    fn stop_scan(&mut self) {
        if let Some(mut c) = self.scan.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        let _ = ctl(&["scan", "off"]);
    }

    pub fn scan(&mut self) -> Result<(), &'static str> {
        if !self.enabled {
            return Err("BT_OFF");
        }
        if self
            .scan
            .as_mut()
            .is_some_and(|c| matches!(c.try_wait(), Ok(None)))
        {
            return Ok(());
        }
        let mut c = tool("bluetoothctl").ok_or("BT_TOOL_UNAVAILABLE")?;
        self.scan = c
            .args(["--timeout", SCAN_SECS, "scan", "on"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok();
        log(&format!("scan started {}", self.scan.is_some()));
        Ok(())
    }

    pub fn devices(&self) -> Result<Vec<Device>, &'static str> {
        if !self.enabled {
            return Err("BT_OFF");
        }
        let list = ctl(&["devices"])?;
        let paired = ctl(&["devices", "Paired"]).unwrap_or_default();
        let connected = ctl(&["devices", "Connected"]).unwrap_or_default();
        let has = |text: &str, mac: &str| parse_devices(text).iter().any(|(m, _)| m == mac);
        let mut cache = self.info.lock().unwrap();
        let now = Instant::now();
        cache.retain(|_, (at, _)| now.duration_since(*at) < INFO_FOR);
        Ok(parse_devices(&list)
            .into_iter()
            .take(24)
            .map(|(mac, name)| {
                // One `info` per device per while: a call each costs a tenth of a second,
                // and the page asks every second.
                let info = &cache
                    .entry(mac.clone())
                    .or_insert_with(|| (now, ctl(&["info", &mac]).unwrap_or_default()))
                    .1;
                let (was_paired, _, audio) = parse_info(info);
                Device {
                    // Either way: one busy bluetoothctl answer must not move a paired device
                    // to Other Devices, where A pairs afresh instead of connecting.
                    paired: has(&paired, &mac) || was_paired,
                    connected: has(&connected, &mac),
                    audio,
                    icon: icon(info).to_owned(),
                    extra: extra(info),
                    mac,
                    name,
                }
            })
            .collect::<Vec<_>>())
            .inspect(|d| {
                // Only when what was heard changes, so the log stays short.
                let seen: Vec<String> = d
                    .iter()
                    .map(|d| {
                        if unnamed(&d.name) {
                            format!("{} [{}] {}", d.name, d.icon, d.extra)
                        } else {
                            format!("{} [{}]", d.name, d.icon)
                        }
                    })
                    .collect();
                let line = format!("heard {}: {}", d.len(), seen.join(", "));
                if self.last_heard.lock().map(|mut l| std::mem::replace(&mut *l, line.clone()) != line).unwrap_or(false) {
                    log(&line);
                }
            })
    }

    /// Pair, trust and connect, or only connect, disconnect or forget, in the background so
    /// the service keeps answering; `job` says how it went.
    pub fn act(&mut self, verb: &str, mac: &str) -> Result<(), &'static str> {
        if !self.enabled {
            return Err("BT_OFF");
        }
        if !valid_mac(mac) {
            return Err("BT_BAD_DEVICE");
        }
        let steps: &[&str] = match verb {
            "pair" => &["pair", "trust", "connect"],
            "connect" => &["connect"],
            "disconnect" => &["disconnect"],
            "forget" => &["remove"],
            _ => return Err("UNKNOWN_COMMAND"),
        };
        // Searching while connecting slows the connect down a lot on this one radio.
        self.stop_scan();
        self.info.lock().unwrap().remove(mac);
        let steps: Vec<&'static str> = steps.to_vec();
        let mac = mac.to_owned();
        let job = self.job.clone();
        *job.lock().unwrap() = if verb == "pair" { "PAIRING" } else { "WORKING" };
        std::thread::spawn(move || {
            let mut ok = true;
            for one in steps {
                let started = Instant::now();
                // Pairing with AirPods can take longer than the rest: the keys go back and forth twice.
                let limit = Duration::from_secs(if one == "pair" { 40 } else { 20 });
                let (went, out) = step(one, &mac, limit);
                log(&format!(
                    "{one} {mac} ({:.1} s) {went}: {}",
                    started.elapsed().as_secs_f32(),
                    short(&out)
                ));
                if !went {
                    ok = false;
                    break;
                }
            }
            if ok {
                keep_pairings();
            }
            *job.lock().unwrap() = if ok { "DONE" } else { "FAILED" };
        });
        Ok(())
    }

    pub fn status(&self) -> String {
        let (mac, name) = self.audio.clone().unwrap_or_default();
        format!(
            "bt_enabled={} bt_error={} bt_job={} bt_audio={} bt_audio_name={}",
            self.enabled,
            self.error,
            self.job.lock().map(|j| *j).unwrap_or("NONE"),
            mac,
            hex(&name)
        )
    }

    /// Keeps `bt-audio` naming the headphones that are connected, for the frontend.
    pub fn tick(&mut self) {
        if self.enabled && self.retry_at.is_some_and(|t| Instant::now() >= t) {
            log("retry");
            self.enable(true);
        }
        if !self.enabled || Instant::now() < self.watch_at {
            return;
        }
        self.watch_at = Instant::now() + WATCH;
        let list = ctl(&["devices", "Connected"]).unwrap_or_default();
        let audio = parse_devices(&list)
            .into_iter()
            .find(|(mac, _)| parse_info(&ctl(&["info", mac]).unwrap_or_default()).2);
        self.set_audio(audio);
    }

    fn set_audio(&mut self, audio: Option<(String, String)>) {
        if audio == self.audio {
            return;
        }
        let path = self.run.join("bt-audio");
        match &audio {
            Some((mac, name)) => {
                let _ = private_write(&path, &format!("{mac}\n{name}\n"));
                // A connect can renew the link key.
                keep_pairings();
            }
            None => {
                let _ = fs::remove_file(&path);
            }
        }
        self.audio = audio;
    }
}

impl Drop for Bluetooth {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.run.join("bt-audio"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devices_lines_give_address_and_name() {
        let text = "Device 11:22:33:44:55:66 Roy's AirPods Pro\nDevice AA:BB:CC:DD:EE:FF\nnoise\n";
        assert_eq!(
            parse_devices(text),
            vec![
                ("11:22:33:44:55:66".into(), "Roy's AirPods Pro".into()),
                ("AA:BB:CC:DD:EE:FF".into(), "".into())
            ]
        );
    }

    #[test]
    fn info_reads_paired_connected_and_audio() {
        let text = "Device 11:22:33:44:55:66 (public)\n\tName: AirPods\n\tIcon: audio-headphones\n\tPaired: yes\n\tConnected: no\n";
        assert_eq!(parse_info(text), (true, false, true));
        let pad = "\tIcon: input-gaming\n\tPaired: no\n\tConnected: yes\n";
        assert_eq!(parse_info(pad), (false, true, false));
        let sink = "\tUUID: Audio Sink                (0000110b-0000-1000-8000-00805f9b34fb)\n";
        assert!(parse_info(sink).2);
    }

    #[test]
    fn reply_keeps_sound_devices_connected_first() {
        let d = |mac: &str, name: &str, p, c, a| Device {
            mac: mac.into(),
            name: name.into(),
            paired: p,
            connected: c,
            audio: a,
            icon: if name == "Pad" { "input-gaming".into() } else { String::new() },
            extra: String::new(),
        };
        let r = reply(vec![
            d("00:00:00:00:00:01", "Pad", false, false, false),
            d("00:00:00:00:00:02", "Speaker", false, false, true),
            d("00:00:00:00:00:03", "Buds", true, true, true),
        ]);
        assert_eq!(
            r,
            format!(
                "BT 00:00:00:00:00:03,{},pca 00:00:00:00:00:02,{},a",
                hex("Buds"),
                hex("Speaker")
            )
        );
    }

    #[test]
    fn nameless_sound_device_is_listed_as_headphones() {
        let r = reply(vec![Device {
            mac: "00:00:00:00:00:09".into(),
            name: "00-00-00-00-00-09".into(),
            paired: false,
            connected: false,
            audio: true,
            icon: "audio-headphones".into(),
            extra: String::new(),
        }]);
        assert_eq!(r, format!("BT 00:00:00:00:00:09,{},a", hex("Headphones")));
    }

    #[test]
    fn extra_tells_class_and_maker() {
        let info = "\tClass: 0x00240418\n\tManufacturerData Key: 0x004c\n\t  07 19 01 0e 20  .... \n\tRSSI: -60\n";
        assert_eq!(extra(info), "cla0x00240418 mfr0x004c 07");
    }

    #[test]
    fn state_ends_the_wait() {
        let info = "Device AA\n\tPaired: yes\n\tTrusted: no\n\tConnected: yes\n";
        assert!(done("pair", info));
        assert!(!done("trust", info));
        assert!(done("connect", info));
        assert!(!done("disconnect", info));
        assert!(done("remove", "Device AA:BB not available"));
        assert!(!went_well("Failed to pair"));
        assert!(went_well("Successful disconnected"));
    }

    #[test]
    fn pairings_survive_the_round_trip() {
        let base = std::env::temp_dir().join(format!("slot-bt-{}", std::process::id()));
        let from = base.join("from");
        let dev = from.join("68:8F:C9:0C:74:80/28:F0:33:A5:3B:C6");
        fs::create_dir_all(&dev).unwrap();
        fs::create_dir_all(from.join("68:8F:C9:0C:74:80/cache")).unwrap();
        fs::write(dev.join("info"), "[LinkKey]\nKey=00\n").unwrap();
        fs::write(from.join("68:8F:C9:0C:74:80/settings"), "x").unwrap();
        fs::write(from.join("68:8F:C9:0C:74:80/cache/junk"), "y").unwrap();
        let data = pack(&from);
        let to = base.join("to");
        assert_eq!(unpack(&to, &data), 2);
        assert_eq!(
            fs::read_to_string(to.join("68:8F:C9:0C:74:80/28:F0:33:A5:3B:C6/info")).unwrap(),
            "[LinkKey]\nKey=00\n"
        );
        assert_eq!(unpack(&to, b"../evil\n1\nx"), 0);
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn only_real_addresses_pass() {
        assert!(valid_mac("AA:bb:01:23:45:67"));
        assert!(!valid_mac("AA:bb:01:23:45"));
        assert!(!valid_mac("AA:bb:01:23:45:6g"));
        assert!(!valid_mac("aa;rm -rf;xx:yy"));
    }

    #[test]
    fn nameless_and_surely_silent_devices_stay_out() {
        assert!(unnamed("4C-87-5D-11-22-33"));
        assert!(unnamed(""));
        assert!(!unnamed("AirPods Pro"));
        assert!(surely_not_audio("input-gaming"));
        assert!(surely_not_audio("phone"));
        assert!(!surely_not_audio(""));
        assert!(!surely_not_audio("audio-headset"));
    }
}
