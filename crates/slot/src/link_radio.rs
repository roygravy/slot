#[cfg(feature = "device")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "device")]
use std::sync::mpsc::{channel, Sender};
#[cfg(feature = "device")]
use std::sync::OnceLock;

use crate::link_net::Cancel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkRole {
    Host,
    Join,
}

impl LinkRole {
    pub fn arg(self) -> &'static str {
        match self {
            LinkRole::Host => "host",
            LinkRole::Join => "join",
        }
    }
}

/// Which network the link runs over, decided by the service when the link starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkNet {
    /// The private network: a soft-AP the host brings up and the joiner associates to, at fixed
    /// addresses. What a device with no Wi-Fi connection gets.
    Direct,
    /// The home network the device is already on, at `local`. No radio to bring up: the other
    /// handheld is found by name rather than by address.
    Lan { local: std::net::Ipv4Addr },
}

/// Three, not one, because the screen says a different sentence for each and only two of them
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadioFail {
    NoHost,
    Cancelled,
    Radio(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioJob {
    /// Desired home connectivity, independent of the Link lease.
    Home(bool),
    /// The clock follows the network (true) or is kept by hand.
    Time(bool),
    Warm,
    /// Release Link preparation, retaining the home connection.
    Cool,
    Down,
}

/// The Wi-Fi pages' questions for the service, each answered later through `wifi_answer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiJob {
    Scan,
    /// Config/wifi.toml changed: join from it afresh.
    Reload,
    WpsStart,
    WpsPoll,
    WpsStop,
    RomManagerOn,
    RomManagerOff,
}

/// A network heard nearby.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub ssid: String,
    pub signal: i32,
    pub secure: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WifiAnswer {
    /// None when the scan could not be made.
    Scanned(Option<Vec<Seen>>),
    WpsWaiting(u64),
    /// WPS joined this network and saved it.
    WpsDone(String),
    WpsFailed,
    /// The ROM Manager's address and PIN; None when it could not start.
    RomManager(Option<(String, String)>),
}

pub trait RadioJobs: Send {
    fn ask(&mut self, job: RadioJob);

    fn wifi(&mut self, _job: WifiJob) {}

    fn wifi_answer(&mut self) -> Option<WifiAnswer> {
        None
    }

    fn warmed(&self) -> bool;

    /// Observed home association plus a LAN address, never the desired menu flag.
    fn home_connected(&self) -> bool {
        false
    }

    /// The name of the home network while connected to it.
    fn home_ssid(&self) -> Option<String> {
        None
    }

    /// Around the deep sleep, waiting for the answer: the network is let go before the
    /// suspend and joined afresh after it.
    fn deep_sleep(&mut self, _asleep: bool) {}
}

pub struct RadioQueue;

pub fn radio_jobs() -> Box<dyn RadioJobs> {
    Box::new(RadioQueue)
}

#[cfg(feature = "device")]
impl RadioJobs for RadioQueue {
    fn ask(&mut self, job: RadioJob) {
        if let RadioJob::Home(enabled) = job {
            HOME_WANTED.store(enabled, Ordering::SeqCst);
            if !enabled {
                HOME_CONNECTED.store(false, Ordering::SeqCst);
            }
        }
        let _ = queue().send(job);
    }

    fn wifi(&mut self, job: WifiJob) {
        std::thread::spawn(move || {
            let word = match job {
                WifiJob::Scan => "scan",
                WifiJob::Reload => {
                    let _ = helper().args(["home", "reload"]).status();
                    return;
                }
                WifiJob::WpsStart => "wps",
                WifiJob::WpsPoll => "wpsstatus",
                WifiJob::WpsStop => "wpsstop",
                WifiJob::RomManagerOn | WifiJob::RomManagerOff => "",
            };
            let out = match job {
                WifiJob::RomManagerOn => helper().args(["web", "on"]).output(),
                WifiJob::RomManagerOff => helper().args(["web", "off"]).output(),
                _ => helper().args(["wifi", word]).output(),
            };
            let text = out
                .as_ref()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).into_owned());
            let answer = match job {
                WifiJob::Scan => Some(WifiAnswer::Scanned(text.as_deref().map(parse_seen))),
                WifiJob::WpsStart if text.is_none() => Some(WifiAnswer::WpsFailed),
                WifiJob::WpsPoll => text.as_deref().and_then(parse_wps),
                WifiJob::RomManagerOn => Some(WifiAnswer::RomManager(
                    text.as_deref().and_then(parse_rom_manager),
                )),
                _ => None,
            };
            if let (Some(a), Ok(mut q)) = (answer, WIFI_ANSWERS.lock()) {
                q.push(a);
            }
        });
    }

    fn wifi_answer(&mut self) -> Option<WifiAnswer> {
        let mut q = WIFI_ANSWERS.lock().ok()?;
        (!q.is_empty()).then(|| q.remove(0))
    }

    fn warmed(&self) -> bool {
        WARM.load(Ordering::SeqCst)
    }
    fn home_connected(&self) -> bool {
        HOME_CONNECTED.load(Ordering::SeqCst)
    }

    fn home_ssid(&self) -> Option<String> {
        HOME_SSID
            .lock()
            .ok()
            .and_then(|s| s.clone())
            .filter(|_| self.home_connected())
    }

    fn deep_sleep(&mut self, asleep: bool) {
        if !HOME_WANTED.load(Ordering::SeqCst) {
            return;
        }
        HOME_CONNECTED.store(false, Ordering::SeqCst);
        let _ = helper()
            .args(["home", if asleep { "sleep" } else { "wake" }])
            .status();
    }
}

#[cfg(feature = "device")]
static WIFI_ANSWERS: std::sync::Mutex<Vec<WifiAnswer>> = std::sync::Mutex::new(Vec::new());
#[cfg(feature = "device")]
static WARM: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "device")]
static HOME_CONNECTED: AtomicBool = AtomicBool::new(false);
/// Home Wi-Fi switched on: only then is the service asked, every few seconds, how it is.
#[cfg(feature = "device")]
static HOME_WANTED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "device")]
static HOME_SSID: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

#[cfg(feature = "device")]
fn queue() -> &'static Sender<RadioJob> {
    static Q: OnceLock<Sender<RadioJob>> = OnceLock::new();
    Q.get_or_init(|| {
        let (tx, rx) = channel::<RadioJob>();
        std::thread::spawn(move || loop {
            let job = match rx.recv_timeout(std::time::Duration::from_secs(3)) {
                Ok(job) => job,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // With Home Wi-Fi off there is nothing to ask after; a process started every
                    // few seconds through a game would only cost battery.
                    if !HOME_WANTED.load(Ordering::SeqCst) {
                        continue;
                    }
                    let status = helper()
                        .args(["service", "status"])
                        .output()
                        .ok()
                        .filter(|o| o.status.success())
                        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                        .unwrap_or_default();
                    let connected = status
                        .split_whitespace()
                        .any(|v| v == "home_connected=true");
                    if let Ok(mut ssid) = HOME_SSID.lock() {
                        *ssid = connected.then(|| status_ssid(&status)).flatten();
                    }
                    HOME_CONNECTED.store(connected, Ordering::SeqCst);
                    continue;
                }
            };
            match job {
                RadioJob::Home(enabled) => {
                    let _ = helper()
                        .args(["home", if enabled { "on" } else { "off" }])
                        .status();
                }
                RadioJob::Time(auto) => {
                    let _ = helper()
                        .args(["time", if auto { "auto" } else { "manual" }])
                        .status();
                }
                RadioJob::Warm => WARM.store(run("warm"), Ordering::SeqCst),
                RadioJob::Cool => {
                    WARM.store(false, Ordering::SeqCst);
                    run("cool");
                }
                RadioJob::Down => {
                    WARM.store(false, Ordering::SeqCst);
                    down();
                }
            }
        });
        tx
    })
}

/// The card's own `slot-services`, through the loader: exFAT carries no exec bit.
#[cfg(feature = "device")]
fn helper() -> std::process::Command {
    let root = std::env::var_os("SLOT_ROOT").unwrap_or_else(|| "/mnt/sdcard".into());
    let service = std::path::Path::new(&root).join("System/slot-services");
    let loader = std::path::Path::new("/lib/ld-linux-aarch64.so.1");
    let mut c = if loader.exists() {
        let mut c = std::process::Command::new(loader);
        c.arg(service);
        c
    } else {
        std::process::Command::new(service)
    };
    c.env("SLOT_ROOT", root)
        .env("SLOT_OWNER_PID", std::process::id().to_string());
    c
}

/// A best-effort preparation/cleanup command served by the bundled daemon.
#[cfg(feature = "device")]
fn run(sub: &str) -> bool {
    helper()
        .arg("link")
        .arg(sub)
        .status()
        .is_ok_and(|status| status.success())
}

/// The home network's address, if the service says the device is on one. `link lan`
/// answers at once and starts nothing, so this is the whole cost of asking.
#[cfg(feature = "device")]
fn home_address() -> Option<std::net::Ipv4Addr> {
    let out = helper()
        .arg("link")
        .arg("lan")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_lan_reply(&String::from_utf8_lossy(&out.stdout))
}

fn unhex(hex: &str) -> Option<String> {
    let bytes: Option<Vec<u8>> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect();
    String::from_utf8(bytes?).ok()
}

/// `0 SEEN hexname,signal,secure ...` from `wifi scan`.
pub fn parse_seen(reply: &str) -> Vec<Seen> {
    reply
        .split_whitespace()
        .skip_while(|w| *w != "SEEN")
        .skip(1)
        .filter_map(|w| {
            let mut f = w.split(',');
            Some(Seen {
                ssid: unhex(f.next()?)?,
                signal: f.next()?.parse().ok()?,
                secure: f.next()? == "1",
            })
        })
        .collect()
}

/// `0 WAITING:secs`, `0 DONE:hexname` or `0 FAILED` from `wifi wpsstatus`.
pub fn parse_wps(reply: &str) -> Option<WifiAnswer> {
    let word = reply.split_whitespace().nth(1)?;
    if let Some(secs) = word.strip_prefix("WAITING:") {
        return Some(WifiAnswer::WpsWaiting(secs.parse().ok()?));
    }
    if let Some(hex) = word.strip_prefix("DONE:") {
        return Some(WifiAnswer::WpsDone(unhex(hex)?));
    }
    (word == "FAILED").then_some(WifiAnswer::WpsFailed)
}

/// `0 WEB http://address pin` from `web on`.
pub fn parse_rom_manager(reply: &str) -> Option<(String, String)> {
    match reply.split_whitespace().collect::<Vec<_>>()[..] {
        ["0", "WEB", address, pin] => Some((address.into(), pin.into())),
        _ => None,
    }
}

/// The network name from `service status`, which sends it hex-encoded as `home_ssid=` so a
/// name with spaces stays one word.
pub fn status_ssid(status: &str) -> Option<String> {
    let hex = status
        .split_whitespace()
        .find_map(|w| w.strip_prefix("home_ssid="))?;
    let bytes: Option<Vec<u8>> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect();
    let name = String::from_utf8_lossy(&bytes?).into_owned();
    (!name.is_empty()).then_some(name)
}

/// `0 192.168.1.24`, the service's answer to `link lan`.
pub fn parse_lan_reply(reply: &str) -> Option<std::net::Ipv4Addr> {
    let mut words = reply.split_whitespace();
    (words.next()? == "0").then_some(())?;
    words.next()?.parse().ok()
}

///
/// A device that is already on the home network links over it and this returns at once: there
/// is no radio to bring up, and nothing to share a channel with. Only a device with no
/// connection gets the private network, and only that path takes any time.
#[cfg(feature = "device")]
pub fn up(role: LinkRole, cancel: &Cancel) -> Result<LinkNet, RadioFail> {
    if let Some(local) = home_address() {
        return Ok(LinkNet::Lan { local });
    }
    let mut child = helper()
        .arg("link")
        .arg(role.arg())
        .spawn()
        .map_err(|e| RadioFail::Radio(format!("link {} would not start: {e}", role.arg())))?;
    loop {
        if cancel.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(RadioFail::Cancelled);
        }
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(LinkNet::Direct),
            Ok(Some(status)) if status.code() == Some(3) => return Err(RadioFail::NoHost),
            Ok(Some(status)) => {
                return Err(RadioFail::Radio(format!(
                    "link {} failed: {status}",
                    role.arg()
                )))
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
            Err(e) => return Err(RadioFail::Radio(format!("link {}: {e}", role.arg()))),
        }
    }
}

#[cfg(feature = "device")]
pub fn down() {
    let _ = helper().arg("link").arg("down").status();
}

#[cfg(not(feature = "device"))]
pub fn up(_role: LinkRole, _cancel: &Cancel) -> Result<LinkNet, RadioFail> {
    Ok(LinkNet::Direct)
}

#[cfg(not(feature = "device"))]
pub fn down() {}

#[cfg(not(feature = "device"))]
static FAKE_WIFI: std::sync::Mutex<Vec<WifiAnswer>> = std::sync::Mutex::new(Vec::new());

#[cfg(not(feature = "device"))]
impl RadioJobs for RadioQueue {
    fn ask(&mut self, _job: RadioJob) {}

    /// Off-device: with SLOT_FAKE_HOME, a street's worth of networks and a router that never
    /// answers WPS, for previews.
    fn wifi(&mut self, job: WifiJob) {
        if std::env::var_os("SLOT_FAKE_HOME").is_none() {
            return;
        }
        let seen = |s: &str, signal, secure| Seen {
            ssid: s.into(),
            signal,
            secure,
        };
        let answer = match job {
            WifiJob::Scan => WifiAnswer::Scanned(Some(vec![
                seen("Ziggo-5G", -42, true),
                seen("KPN-Thuis", -55, true),
                seen("Ziggo-2G", -60, true),
                seen("Buren Wifi", -71, true),
                seen("Ziggo Guest", -74, false),
                seen("iPhone van Roy", -82, true),
            ])),
            WifiJob::WpsStart | WifiJob::WpsPoll => WifiAnswer::WpsWaiting(107),
            WifiJob::RomManagerOn => {
                WifiAnswer::RomManager(Some(("http://192.168.1.23".into(), "4821".into())))
            }
            _ => return,
        };
        if let Ok(mut q) = FAKE_WIFI.lock() {
            q.push(answer);
        }
    }

    fn wifi_answer(&mut self) -> Option<WifiAnswer> {
        let mut q = FAKE_WIFI.lock().ok()?;
        (!q.is_empty()).then(|| q.remove(0))
    }

    fn warmed(&self) -> bool {
        true
    }

    /// Off-device there is no home network; SLOT_FAKE_HOME pretends there is, for previews.
    fn home_connected(&self) -> bool {
        std::env::var_os("SLOT_FAKE_HOME").is_some()
    }

    fn home_ssid(&self) -> Option<String> {
        std::env::var("SLOT_FAKE_HOME")
            .ok()
            .filter(|s| !s.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_role_asks_for_its_own_subcommand() {
        assert_eq!(LinkRole::Host.arg(), "host");
        assert_eq!(LinkRole::Join.arg(), "join");
    }

    #[test]
    fn asking_for_a_job_is_never_an_error() {
        let mut jobs = radio_jobs();
        jobs.ask(RadioJob::Warm);
        jobs.ask(RadioJob::Cool);
        jobs.ask(RadioJob::Down);
    }

    #[test]
    fn the_status_names_the_network_even_with_spaces_in_it() {
        let status = "home_enabled=true home_connected=true home_error= link= freq=None home_ssid=4b5054204e6574";
        assert_eq!(status_ssid(status).as_deref(), Some("KPT Net"));
        assert_eq!(status_ssid("home_connected=false home_ssid="), None);
        assert_eq!(status_ssid("home_connected=false"), None);
    }

    #[cfg(not(feature = "device"))]
    #[test]
    fn a_host_build_has_no_driver_left_to_load() {
        assert!(radio_jobs().warmed());
    }
}
