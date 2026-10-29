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

pub trait RadioJobs: Send {
    fn ask(&mut self, job: RadioJob);

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
impl RadioJobs for RadioQueue {
    fn ask(&mut self, _job: RadioJob) {}

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
