//! The menu's Wi-Fi pages: the networks nearby, and WPS. Joining and forgetting are the
//! frontend's own edits to Config/wifi.toml, followed by `home reload`.
use crate::config;
use crate::radio::{connected, Home};
use crate::system::output;
use std::path::Path;
use std::time::{Duration, Instant};

/// A network heard nearby: its name, signal in dBm, and whether it wants a password.
#[derive(Debug, PartialEq)]
pub struct Seen {
    pub ssid: String,
    pub signal: i32,
    pub secure: bool,
}

/// One word per network, `hexname,signal,secure`, strongest first; empty names left out
/// and each name once, at its strongest.
pub fn reply(mut seen: Vec<Seen>) -> String {
    seen.retain(|s| !s.ssid.is_empty() && !s.ssid.contains('\0'));
    seen.sort_by_key(|s| std::cmp::Reverse(s.signal));
    let mut out: Vec<Seen> = Vec::new();
    for s in seen {
        if !out.iter().any(|o| o.ssid == s.ssid) {
            out.push(s);
        }
    }
    out.truncate(24);
    out.iter()
        .map(|s| format!("{},{},{}", config::hex(&s.ssid), s.signal, s.secure as u8))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The supplicant's `scan_results`: bssid, frequency, signal, flags and name, tab apart.
pub fn parse_supplicant(text: &str) -> Vec<Seen> {
    text.lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.splitn(5, '\t').collect();
            if f.len() < 5 {
                return None;
            }
            Some(Seen {
                ssid: unescape(f[4]),
                signal: f[2].trim().parse().ok()?,
                secure: f[3].contains("WPA") || f[3].contains("RSN") || f[3].contains("WEP"),
            })
        })
        .collect()
}

/// `iw dev wlan0 scan`: one `BSS` block per access point.
pub fn parse_iw(text: &str) -> Vec<Seen> {
    let mut out = Vec::new();
    let mut cur: Option<Seen> = None;
    for line in text.lines() {
        let t = line.trim();
        if line.starts_with("BSS ") {
            out.extend(cur.take());
            cur = Some(Seen {
                ssid: String::new(),
                signal: -100,
                secure: false,
            });
        } else if let Some(c) = cur.as_mut() {
            if let Some(v) = t.strip_prefix("signal:") {
                c.signal = v
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse::<f32>().ok())
                    .map_or(-100, |n| n as i32);
            } else if let Some(v) = t.strip_prefix("SSID:") {
                c.ssid = unescape(v.trim_start());
            } else if t.starts_with("RSN:") || t.starts_with("WPA:") || t.contains("Privacy") {
                c.secure = true;
            }
        }
    }
    out.extend(cur);
    out
}

/// Both tools write a byte outside printable ASCII as `\xNN`.
fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 3 < b.len() && b[i + 1] == b'x' {
            if let Ok(v) =
                u8::from_str_radix(std::str::from_utf8(&b[i + 2..i + 4]).unwrap_or(""), 16)
            {
                out.push(v);
                i += 4;
                continue;
            }
        }
        if b[i] == b'\\' && i + 1 < b.len() && b[i + 1] == b'\\' {
            out.push(b'\\');
            i += 2;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A scan, through the supplicant when one runs, else straight from the radio.
pub fn scan(home: &Home) -> Result<String, &'static str> {
    if home.interface.wpa.is_some() {
        home.interface.cli(&["scan"])?;
        std::thread::sleep(Duration::from_millis(3500));
        return Ok(reply(parse_supplicant(
            &home.interface.cli(&["scan_results"])?,
        )));
    }
    let _ = output("rfkill", &["unblock", "wifi"]);
    output("ip", &["link", "set", "wlan0", "up"])?;
    Ok(reply(parse_iw(&output("iw", &["dev", "wlan0", "scan"])?)))
}

pub enum WpsState {
    Waiting(Instant),
    /// Joined: the network's name, saved to the card.
    Done(String),
    Failed,
}

pub struct Wps {
    pub state: WpsState,
}

impl Wps {
    pub fn start(home: &mut Home) -> Result<Wps, &'static str> {
        home.held = true;
        home.interface.start_wps()?;
        std::thread::sleep(Duration::from_millis(500));
        home.interface.cli(&["wps_pbc"])?;
        Ok(Wps {
            state: WpsState::Waiting(Instant::now() + Duration::from_secs(120)),
        })
    }

    pub fn tick(&mut self, home: &mut Home, root: &Path) {
        let WpsState::Waiting(until) = self.state else {
            return;
        };
        let status = home.interface.status();
        if connected(&status) {
            let _ = home.interface.cli(&["save_config"]);
            let conf = std::fs::read_to_string(home.interface.conf()).unwrap_or_default();
            self.state = match saved_network(&conf) {
                Some(net) => {
                    let ssid = net.ssid.clone();
                    let _ = slot_store::wifi::add(root, net);
                    WpsState::Done(ssid)
                }
                None => WpsState::Failed,
            };
            stop(home, root);
        } else if Instant::now() >= until {
            self.state = WpsState::Failed;
            stop(home, root);
        }
    }

    pub fn word(&self) -> String {
        match &self.state {
            WpsState::Waiting(until) => format!(
                "WAITING:{}",
                until.saturating_duration_since(Instant::now()).as_secs()
            ),
            WpsState::Done(ssid) => format!("DONE:{}", config::hex(ssid)),
            WpsState::Failed => "FAILED".into(),
        }
    }
}

pub fn stop(home: &mut Home, root: &Path) {
    home.interface.stop();
    home.held = false;
    if home.enabled {
        home.reload(root);
    }
}

/// The network block the supplicant saved after WPS: its name and passphrase or key.
pub fn saved_network(conf: &str) -> Option<slot_store::wifi::Saved> {
    let block = &conf[conf.find("network={")?..];
    let value = |key: &str| {
        block
            .lines()
            .map(str::trim)
            .find_map(|l| l.strip_prefix(key).and_then(|v| v.strip_prefix('=')))
    };
    let ssid = value("ssid")?;
    let ssid = match ssid.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        Some(q) => q.to_owned(),
        None => {
            let bytes: Option<Vec<u8>> = (0..ssid.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(ssid.get(i..i + 2)?, 16).ok())
                .collect();
            String::from_utf8(bytes?).ok()?
        }
    };
    let password = value("psk").map(|p| {
        p.strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(p)
            .to_owned()
    });
    Some(slot_store::wifi::Saved { ssid, password })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supplicant_results_list_each_name_once_strongest_first() {
        let text = "bssid / frequency / signal level / flags / ssid\n\
            aa\t2412\t-70\t[WPA2-PSK-CCMP][ESS]\tHome\n\
            bb\t5180\t-40\t[WPA2-PSK-CCMP][ESS]\tHome\n\
            cc\t2437\t-60\t[ESS]\tCafe\\x20Free\n\
            dd\t2437\t-50\t[ESS]\t\n";
        let r = reply(parse_supplicant(text));
        assert_eq!(
            r,
            format!(
                "{},-40,1 {},-60,0",
                config::hex("Home"),
                config::hex("Cafe Free")
            )
        );
    }

    #[test]
    fn iw_blocks_read_name_signal_and_lock() {
        let text = "BSS 11:22(on wlan0)\n\tsignal: -48.00 dBm\n\tSSID: Ziggo\n\tRSN:\t * Version: 1\nBSS 33:44(on wlan0)\n\tsignal: -80.00 dBm\n\tSSID: Open\n";
        let seen = parse_iw(text);
        assert_eq!(
            seen[0],
            Seen {
                ssid: "Ziggo".into(),
                signal: -48,
                secure: true
            }
        );
        assert_eq!(
            seen[1],
            Seen {
                ssid: "Open".into(),
                signal: -80,
                secure: false
            }
        );
    }

    #[test]
    fn wps_saves_the_name_and_passphrase_it_was_given() {
        let conf = "ctrl_interface=/run/x\nupdate_config=1\n\nnetwork={\n\tssid=\"KPN\"\n\tpsk=\"geheim123\"\n\tkey_mgmt=WPA-PSK\n}\n";
        let n = saved_network(conf).unwrap();
        assert_eq!(
            (n.ssid.as_str(), n.password.as_deref()),
            ("KPN", Some("geheim123"))
        );
        let hexed = "network={\n\tssid=4b504e\n\tpsk=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n}\n";
        assert_eq!(saved_network(hexed).unwrap().ssid, "KPN");
    }
}
