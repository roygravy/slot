//! The menu's list pages over the search screen: the Wi-Fi networks nearby and their
//! password, the networks saved, WPS, and the time zones.
use slot_input::Millis;
use slot_ui::{SearchScreen, WifiLegend};

use crate::label_search::{self, LabelSearch, Press};
use crate::link_radio::Seen;

const OPEN_MS: f32 = 160.0;
const CLOSE_MS: f32 = 120.0;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Kind {
    Nearby,
    Saved,
    Wps,
    Zones,
    RomManager,
}

pub struct Row {
    pub title: String,
    pub sub: String,
    /// What the row stands for: a network's name, a city.
    pub key: String,
    /// A network that wants a password.
    pub secure: bool,
}

enum Stage {
    Waiting,
    List(usize),
    Typing(Box<LabelSearch>, String),
    Confirm(usize),
    /// A sentence and a line under it: nothing found, or WPS waiting.
    Message(String, String),
}

pub enum Step {
    None,
    Close,
    Scan,
    Join {
        ssid: String,
        password: Option<String>,
    },
    Forget(String),
    WpsStop,
    Zone(String),
    RomManagerStop,
}

pub struct ListPage {
    pub kind: Kind,
    stage: Stage,
    rows: Vec<Row>,
    rev: u64,
    /// The ROM Manager's PIN, shown large.
    pin: Option<String>,
    address: Option<String>,
    opened: Millis,
    closed: Option<(Millis, f32)>,
}

/// What the page shows, made into faces by the frontend.
pub struct Words {
    pub heading: String,
    pub typed: Option<String>,
    pub status: Option<String>,
    pub note: Option<String>,
    /// Large over the status: the ROM Manager's PIN.
    pub big: Option<String>,
    /// The ROM Manager's address with its PIN, as a QR code to scan.
    pub qr: Option<String>,
    pub rows: Vec<(String, String)>,
    /// Network names keep their own capitals.
    pub upper: bool,
}

/// A signal in dBm, as a word.
fn strength(dbm: i32) -> &'static str {
    match dbm {
        d if d >= -55 => "Strong",
        d if d >= -70 => "Good",
        _ => "Weak",
    }
}

impl ListPage {
    fn new(kind: Kind, stage: Stage, rows: Vec<Row>, now: Millis) -> Self {
        ListPage {
            kind,
            stage,
            rows,
            rev: 0,
            pin: None,
            address: None,
            opened: now,
            closed: None,
        }
    }

    /// Scanning, until `scanned` brings what was heard.
    pub fn nearby(now: Millis) -> Self {
        Self::new(Kind::Nearby, Stage::Waiting, Vec::new(), now)
    }

    pub fn saved(names: Vec<String>, current: Option<&str>, now: Millis) -> Self {
        let rows: Vec<Row> = names
            .into_iter()
            .map(|n| Row {
                sub: if current == Some(n.as_str()) {
                    "Connected"
                } else {
                    "Saved"
                }
                .into(),
                title: n.clone(),
                key: n,
                secure: false,
            })
            .collect();
        let stage = if rows.is_empty() {
            Stage::Message(
                "No networks saved".into(),
                "Add one from Add Network".into(),
            )
        } else {
            Stage::List(0)
        };
        Self::new(Kind::Saved, stage, rows, now)
    }

    pub fn wps(now: Millis) -> Self {
        Self::new(Kind::Wps, Self::wps_message(120), Vec::new(), now)
    }

    /// The ROM Manager's address and PIN, once the console has them.
    pub fn rom_manager(now: Millis) -> Self {
        Self::new(
            Kind::RomManager,
            Stage::Message("Starting...".into(), String::new()),
            Vec::new(),
            now,
        )
    }

    /// The page is up at `address`, or could not be started.
    pub fn rom_manager_ready(&mut self, page: Option<(String, String)>) {
        if self.kind != Kind::RomManager {
            return;
        }
        self.stage = match page {
            Some((address, pin)) => {
                self.pin = Some(pin);
                self.address = Some(address.clone());
                Stage::Message(
                    address,
                    "Scan the code, or open the address and enter the PIN".into(),
                )
            }
            None => Stage::Message(
                "Not connected".into(),
                "Join a network first, then try again".into(),
            ),
        };
        self.rev += 1;
    }

    /// An update came down while the page is open: the note says so until it is installed.
    pub fn rom_manager_update(&mut self, version: &str) {
        let Stage::Message(address, note) = &self.stage else {
            return;
        };
        if self.kind != Kind::RomManager || self.address.is_none() {
            return;
        }
        let ready = format!("Update {version} ready · install it from the web page");
        if *note != ready {
            self.stage = Stage::Message(address.clone(), ready);
            self.rev += 1;
        }
    }

    fn wps_message(secs: u64) -> Stage {
        Stage::Message(
            "Press the WPS button on your router".into(),
            format!("Waiting · {}:{:02}", secs / 60, secs % 60),
        )
    }

    /// The cities, each with its offset now, the current one chosen.
    pub fn zones(rows: Vec<(String, String)>, at: usize, now: Millis) -> Self {
        let rows = rows
            .into_iter()
            .map(|(city, offset)| Row {
                title: city.clone(),
                sub: offset,
                key: city,
                secure: false,
            })
            .collect();
        Self::new(Kind::Zones, Stage::List(at), rows, now)
    }

    pub fn scanned(&mut self, seen: Option<Vec<Seen>>, current: Option<&str>) {
        if self.kind != Kind::Nearby || !matches!(self.stage, Stage::Waiting) {
            return;
        }
        self.rev += 1;
        let Some(seen) = seen.filter(|s| !s.is_empty()) else {
            self.stage = Stage::Message("No networks found".into(), String::new());
            return;
        };
        self.rows = seen
            .into_iter()
            .map(|s| {
                let sub = if current == Some(s.ssid.as_str()) {
                    "Connected".to_owned()
                } else {
                    format!(
                        "{} · {}",
                        if s.secure { "Secured" } else { "Open" },
                        strength(s.signal)
                    )
                };
                Row {
                    title: s.ssid.clone(),
                    sub,
                    key: s.ssid,
                    secure: s.secure,
                }
            })
            .collect();
        self.stage = Stage::List(0);
    }

    pub fn wps_waiting(&mut self, secs: u64) {
        if self.kind == Kind::Wps {
            self.stage = Self::wps_message(secs);
            self.rev += 1;
        }
    }

    pub fn rev(&self) -> u64 {
        match &self.stage {
            Stage::Typing(k, _) => self.rev.wrapping_add(k.rev() << 20),
            _ => self.rev,
        }
    }

    pub fn screen(&self, now: Millis) -> SearchScreen {
        match &self.stage {
            Stage::Waiting => SearchScreen::Searching,
            Stage::List(at) => SearchScreen::Results { at: *at },
            Stage::Typing(k, _) => k.screen(now),
            Stage::Confirm(_) | Stage::Message(..) => SearchScreen::Nothing,
        }
    }

    pub fn legend(&self) -> Option<WifiLegend> {
        Some(match (&self.stage, self.kind) {
            (Stage::Typing(..), _) => return None,
            (Stage::Confirm(_), _) => WifiLegend::Confirm,
            (_, Kind::Wps) => WifiLegend::Wps,
            (_, Kind::RomManager) => WifiLegend::Back,
            (Stage::Message(..), Kind::Nearby) => WifiLegend::Nearby,
            (Stage::Message(..), _) => WifiLegend::Back,
            (_, Kind::Nearby) => WifiLegend::Nearby,
            (_, Kind::Saved) => WifiLegend::Saved,
            (_, Kind::Zones) => WifiLegend::Zones,
        })
    }

    pub fn words(&self) -> Words {
        let mut w = Words {
            heading: String::new(),
            typed: None,
            status: None,
            note: None,
            big: self.pin.clone(),
            qr: self
                .pin
                .as_ref()
                .zip(self.address.as_ref())
                .map(|(pin, address)| {
                    // The PIN rides along after the #, so scanning signs in at once; it never
                    // leaves the phone in a request.
                    format!("{address}/#pin={pin}")
                }),
            rows: self
                .rows
                .iter()
                .map(|r| (r.title.clone(), r.sub.clone()))
                .collect(),
            upper: self.kind == Kind::Zones,
        };
        match &self.stage {
            Stage::Waiting => w.heading = "Networks Nearby".into(),
            Stage::List(_) => {
                w.status = Some(
                    match self.kind {
                        Kind::Nearby => "Networks Nearby",
                        Kind::Saved => "My Networks",
                        Kind::Wps => "WPS",
                        Kind::Zones => "Time Zone",
                        Kind::RomManager => "GRAVY Manager",
                    }
                    .into(),
                )
            }
            Stage::Typing(k, ssid) => {
                w.heading = format!("Password · {ssid}");
                w.typed = Some(k.text().to_owned()).filter(|t| !t.is_empty());
            }
            Stage::Confirm(at) => {
                w.heading = "My Networks".into();
                w.status = Some(format!("Forget {}?", self.rows[*at].title));
                w.note = Some("Its password is removed from the card".into());
            }
            Stage::Message(line, note) => {
                w.heading = match self.kind {
                    Kind::Wps => "WPS",
                    Kind::RomManager => "GRAVY Manager",
                    Kind::Saved => "My Networks",
                    _ => "Networks Nearby",
                }
                .into();
                w.status = Some(line.clone());
                w.note = Some(note.clone()).filter(|n| !n.is_empty());
            }
        }
        w
    }

    pub fn openness(&self, now: Millis) -> f32 {
        match self.closed {
            Some((at, from)) => (from - now.saturating_sub(at) as f32 / CLOSE_MS).max(0.0),
            None => (now.saturating_sub(self.opened) as f32 / OPEN_MS).min(1.0),
        }
    }

    pub fn closing(&self) -> bool {
        self.closed.is_some()
    }

    pub fn finished(&self, now: Millis) -> bool {
        self.closed.is_some() && self.openness(now) <= 0.0
    }

    pub fn close(&mut self, now: Millis) -> Step {
        if self.closed.is_none() {
            self.closed = Some((now, self.openness(now)));
        }
        match self.kind {
            Kind::Wps => Step::WpsStop,
            Kind::RomManager => Step::RomManagerStop,
            _ => Step::Close,
        }
    }

    pub fn press(&mut self, press: Press, now: Millis) -> Step {
        if self.closed.is_some() {
            return Step::None;
        }
        if press == Press::Exit {
            return self.close(now);
        }
        self.rev += 1;
        let n = self.rows.len();
        match &mut self.stage {
            Stage::Typing(k, ssid) => {
                let ssid = ssid.clone();
                match k.press(press, now) {
                    label_search::Step::Search(..) => {
                        let password = k.text().to_owned();
                        self.close(now);
                        Step::Join {
                            ssid,
                            password: Some(password),
                        }
                    }
                    label_search::Step::Close => {
                        let at = self.rows.iter().position(|r| r.key == ssid).unwrap_or(0);
                        self.stage = Stage::List(at);
                        Step::None
                    }
                    _ => Step::None,
                }
            }
            Stage::Confirm(at) => {
                let at = *at;
                match press {
                    Press::Choose => {
                        let ssid = self.rows.remove(at).key;
                        self.stage = if self.rows.is_empty() {
                            Stage::Message("No networks saved".into(), String::new())
                        } else {
                            Stage::List(at.min(self.rows.len() - 1))
                        };
                        Step::Forget(ssid)
                    }
                    Press::Back => {
                        self.stage = Stage::List(at);
                        Step::None
                    }
                    _ => Step::None,
                }
            }
            Stage::List(at) => match press {
                Press::Up => {
                    *at = if *at == 0 { n - 1 } else { *at - 1 };
                    Step::None
                }
                Press::Down => {
                    *at = (*at + 1) % n.max(1);
                    Step::None
                }
                Press::Back => self.close(now),
                Press::Space if self.kind == Kind::Nearby => self.rescan(),
                Press::Space if self.kind == Kind::Saved => {
                    self.stage = Stage::Confirm(*at);
                    Step::None
                }
                Press::Choose => {
                    let row = &self.rows[*at];
                    match self.kind {
                        Kind::Nearby if row.secure => {
                            let ssid = row.key.clone();
                            self.stage = Stage::Typing(Box::new(LabelSearch::password(now)), ssid);
                            Step::None
                        }
                        Kind::Nearby => {
                            let ssid = row.key.clone();
                            self.close(now);
                            Step::Join {
                                ssid,
                                password: None,
                            }
                        }
                        Kind::Zones => {
                            let city = row.key.clone();
                            self.close(now);
                            Step::Zone(city)
                        }
                        _ => Step::None,
                    }
                }
                _ => Step::None,
            },
            Stage::Waiting | Stage::Message(..) => match press {
                Press::Back => self.close(now),
                Press::Space if self.kind == Kind::Nearby => self.rescan(),
                _ => Step::None,
            },
        }
    }

    fn rescan(&mut self) -> Step {
        self.stage = Stage::Waiting;
        self.rows.clear();
        Step::Scan
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(s: &str, secure: bool) -> Seen {
        Seen {
            ssid: s.into(),
            signal: -50,
            secure,
        }
    }

    #[test]
    fn a_secured_network_asks_its_password_and_start_joins_with_it() {
        let mut p = ListPage::nearby(0);
        p.scanned(
            Some(vec![seen("Open Cafe", false), seen("KPN", true)]),
            None,
        );
        p.press(Press::Down, 1);
        p.press(Press::Choose, 2);
        assert!(matches!(p.screen(3), SearchScreen::Keyboard { .. }));
        assert_eq!(p.words().heading, "Password · KPN");
        for _ in 0..8 {
            p.press(Press::Choose, 4);
        }
        match p.press(Press::Search, 5) {
            Step::Join { ssid, password } => {
                assert_eq!(ssid, "KPN");
                assert_eq!(password.as_deref(), Some("qqqqqqqq"));
            }
            _ => panic!("START did not join"),
        }
    }

    #[test]
    fn an_open_network_joins_at_once_and_b_on_the_keyboard_goes_back_to_the_list() {
        let mut p = ListPage::nearby(0);
        p.scanned(Some(vec![seen("KPN", true), seen("Open", false)]), None);
        p.press(Press::Choose, 1);
        p.press(Press::Back, 2);
        assert!(matches!(p.screen(3), SearchScreen::Results { at: 0 }));
        p.press(Press::Down, 4);
        assert!(matches!(
            p.press(Press::Choose, 5),
            Step::Join { password: None, .. }
        ));
    }

    #[test]
    fn y_asks_before_a_saved_network_is_forgotten() {
        let mut p = ListPage::saved(vec!["A".into(), "B".into()], Some("A"), 0);
        p.press(Press::Down, 1);
        p.press(Press::Space, 2);
        assert_eq!(p.words().status.as_deref(), Some("Forget B?"));
        assert!(matches!(p.press(Press::Choose, 3), Step::Forget(s) if s == "B"));
        assert_eq!(p.words().rows.len(), 1);
    }

    #[test]
    fn nothing_heard_says_so_and_y_scans_again() {
        let mut p = ListPage::nearby(0);
        p.scanned(Some(Vec::new()), None);
        assert_eq!(p.words().status.as_deref(), Some("No networks found"));
        assert!(matches!(p.press(Press::Space, 1), Step::Scan));
        assert!(matches!(p.screen(2), SearchScreen::Searching));
    }
}
