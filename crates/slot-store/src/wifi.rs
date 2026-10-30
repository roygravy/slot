//! The card's own `Config/wifi.toml`: the networks Home Wi-Fi joins, first one first. The
//! menu adds and forgets networks here; slot-services reads it and checks every value.
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    pub ssid: String,
    /// None for an open network.
    pub password: Option<String>,
}

pub fn path(root: &Path) -> std::path::PathBuf {
    root.join("Config/wifi.toml")
}

/// What the file holds; entries it cannot make sense of are left out.
pub fn read(root: &Path) -> Vec<Saved> {
    let Ok(text) = std::fs::read_to_string(path(root)) else {
        return Vec::new();
    };
    parse(&text)
}

pub fn parse(text: &str) -> Vec<Saved> {
    let Ok(doc) = text.parse::<toml::Table>() else {
        return Vec::new();
    };
    let Some(list) = doc.get("networks").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|e| {
            let t = e.as_table()?;
            let ssid = t.get("ssid")?.as_str()?.to_owned();
            let open = t.get("security").and_then(|v| v.as_str()) == Some("open");
            let password = t
                .get("password")
                .and_then(|v| v.as_str())
                .map(str::to_owned);
            Some(Saved {
                ssid,
                password: if open { None } else { password },
            })
        })
        .collect()
}

/// A TOML literal-free basic string: quotes, backslashes and control bytes escaped.
fn quoted(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{:04X}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn render(nets: &[Saved]) -> String {
    let mut s = String::from("# Home Wi-Fi joins these, the first one first.\n");
    for n in nets {
        s.push_str("\n[[networks]]\n");
        s.push_str(&format!("ssid = {}\n", quoted(&n.ssid)));
        match &n.password {
            Some(p) => s.push_str(&format!("password = {}\n", quoted(p))),
            None => s.push_str("security = \"open\"\n"),
        }
    }
    s
}

pub fn write(root: &Path, nets: &[Saved]) -> std::io::Result<()> {
    crate::atomic_write(&path(root), render(nets).as_bytes())
}

/// This network first, so Home Wi-Fi tries it before the others; an older entry for the same
/// name goes.
pub fn add(root: &Path, net: Saved) -> std::io::Result<()> {
    let mut nets = read(root);
    nets.retain(|n| n.ssid != net.ssid);
    nets.insert(0, net);
    nets.truncate(32);
    write(root, &nets)
}

pub fn forget(root: &Path, ssid: &str) -> std::io::Result<()> {
    let mut nets = read(root);
    nets.retain(|n| n.ssid != ssid);
    write(root, &nets)
}

/// A WPA password the service accepts: 8 to 63 printable ASCII letters, or 64 hex digits.
pub fn password_ok(p: &str) -> bool {
    ((8..=63).contains(&p.len()) && p.bytes().all(|b| (32..=126).contains(&b)))
        || (p.len() == 64 && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_written_reads_back_quotes_and_all() {
        let nets = vec![
            Saved {
                ssid: "Roy's \"Net\" \\ 5G".into(),
                password: Some("p\"ass\\word ok".into()),
            },
            Saved {
                ssid: "Café".into(),
                password: None,
            },
        ];
        assert_eq!(parse(&render(&nets)), nets);
    }

    #[test]
    fn a_new_network_goes_first_and_replaces_its_old_entry() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("Config")).unwrap();
        let net = |s: &str, p: &str| Saved {
            ssid: s.into(),
            password: Some(p.into()),
        };
        add(d.path(), net("A", "aaaaaaaa")).unwrap();
        add(d.path(), net("B", "bbbbbbbb")).unwrap();
        add(d.path(), net("A", "cccccccc")).unwrap();
        assert_eq!(read(d.path()), [net("A", "cccccccc"), net("B", "bbbbbbbb")]);
        forget(d.path(), "A").unwrap();
        assert_eq!(read(d.path()), [net("B", "bbbbbbbb")]);
    }

    #[test]
    fn passwords_are_held_to_what_wpa_takes() {
        assert!(!password_ok("short"));
        assert!(password_ok("eight888"));
        assert!(password_ok(&"a".repeat(64)));
        assert!(!password_ok(&"g".repeat(64)));
    }
}
