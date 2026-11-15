//! RetroAchievements login for the GRAVY Manager: the password goes from the phone to the
//! console once, RetroAchievements answers with a token, and only user and token are kept.
use std::path::Path;
use std::time::Duration;

const LOGIN: &str = "https://retroachievements.org/dorequest.php";
const FILE: &str = "System/retroachievements.txt";

/// The user who is logged in, if any.
pub fn user(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join(FILE)).ok()?;
    text.lines().map(str::trim).find(|l| !l.is_empty()).map(str::to_string)
}

/// Is a web API key kept (third line)?
pub fn has_key(root: &Path) -> bool {
    std::fs::read_to_string(root.join(FILE))
        .map(|t| t.lines().map(str::trim).filter(|l| !l.is_empty()).count() >= 3)
        .unwrap_or(false)
}

/// Keep the web API key as the third line, beside user and token. An empty key removes it.
pub fn set_key(root: &Path, body: &str) -> Result<(), &'static str> {
    let key = body.trim();
    if key.chars().any(|c| !c.is_ascii_alphanumeric()) {
        return Err("That is not a Web API Key");
    }
    let text = std::fs::read_to_string(root.join(FILE)).map_err(|_| "Log in first")?;
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    let (user, token) = (lines.next().ok_or("Log in first")?, lines.next().ok_or("Log in first")?);
    let out = if key.is_empty() { format!("{user}\n{token}\n") } else { format!("{user}\n{token}\n{key}\n") };
    slot_store::atomic_write(&root.join(FILE), out.as_bytes()).map_err(|_| "The card can't be written")
}

pub fn logout(root: &Path) -> Result<(), &'static str> {
    match std::fs::remove_file(root.join(FILE)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("The card can't be written"),
    }
}

fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The string value of `"key":"..."` in a flat JSON answer.
fn field(json: &str, key: &str) -> Option<String> {
    let at = json.find(&format!("\"{key}\""))? + key.len() + 2;
    let rest = json[at..].trim_start().strip_prefix(':')?.trim_start().strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => out.push(chars.next()?),
            c => out.push(c),
        }
    }
    None
}

/// Log in with the password and keep the token. `body` is the user, the password and the web
/// API key (may be empty), one per line.
pub fn login(root: &Path, body: &str) -> Result<(), &'static str> {
    let mut parts = body.splitn(3, '\n');
    let user = parts.next().unwrap_or("").trim();
    let password = parts.next().ok_or("Bad login")?.trim_end_matches('\r');
    let key = parts.next().unwrap_or("").trim();
    if user.is_empty() || password.is_empty() {
        return Err("Fill in your user name and password");
    }
    if key.chars().any(|c| !c.is_ascii_alphanumeric()) {
        return Err("That is not a Web API Key");
    }
    let form = format!("r=login2&u={}&p={}", encode(user), encode(password));
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_global(Some(Duration::from_secs(20)))
        .http_status_as_error(false)
        .user_agent(concat!("slot/", env!("CARGO_PKG_VERSION"), " retroachievements"))
        .build()
        .into();
    let mut answer = agent
        .post(LOGIN)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send(form.as_bytes())
        .map_err(|_| "RetroAchievements can't be reached")?;
    let json = answer
        .body_mut()
        .with_config()
        .limit(1 << 16)
        .read_to_string()
        .map_err(|_| "RetroAchievements can't be reached")?;
    if !json.contains("\"Success\":true") && !json.contains("\"Success\": true") {
        return Err("Wrong user name or password");
    }
    let token = field(&json, "Token").ok_or("RetroAchievements gave no token")?;
    let name = field(&json, "User").unwrap_or_else(|| user.to_string());
    let dir = root.join("System");
    let _ = std::fs::create_dir_all(&dir);
    let text = if key.is_empty() { format!("{name}\n{token}\n") } else { format!("{name}\n{token}\n{key}\n") };
    slot_store::atomic_write(&root.join(FILE), text.as_bytes())
        .map_err(|_| "The card can't be written")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_fields_and_encodes() {
        let j = r#"{"Success":true,"User":"Roy","Token":"a\"b"}"#;
        assert_eq!(field(j, "User").unwrap(), "Roy");
        assert_eq!(field(j, "Token").unwrap(), "a\"b");
        assert_eq!(encode("a b&é"), "a%20b%26%C3%A9");
    }

    #[test]
    fn user_and_logout() {
        let d = tempfile::tempdir().unwrap();
        assert!(user(d.path()).is_none());
        std::fs::create_dir(d.path().join("System")).unwrap();
        std::fs::write(d.path().join(FILE), "Roy\ntok\n").unwrap();
        assert_eq!(user(d.path()).unwrap(), "Roy");
        logout(d.path()).unwrap();
        assert!(user(d.path()).is_none());
    }
}
