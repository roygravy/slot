use std::collections::HashMap;

use unicode_normalization::UnicodeNormalization;

pub const CART_SHELL_FILE: &str = "Config/cart_shell.txt";
pub const LABELS_SHELL_FILE: &str = "Labels/cart_shell.txt";

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Outline {
    Auto,
    Notched,
    Rounded,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ShellFinish {
    Solid,
    Clear,
    Glitter,
    /// Opaque plastic with flecks in it, as Pokémon Gold and Silver are.
    SolidGlitter,
    /// Gold paint, as Nintendo's gold NES carts wear.
    Metallic,
    /// The cart's own colour and finish: only its outline was picked (an SNES cart's NTSC or
    /// PAL shape). The colour is not used.
    Own,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ShellChoice {
    pub outline: Outline,
    pub colour: [u8; 3],
    pub finish: ShellFinish,
}

impl ShellChoice {
    pub fn parse(value: &str) -> Option<ShellChoice> {
        let mut words = value.split_whitespace();
        let outline = match words.next()? {
            "auto" => Outline::Auto,
            "notched" => Outline::Notched,
            "rounded" => Outline::Rounded,
            _ => return None,
        };
        let hex = words.next()?;
        if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let colour = [0, 2, 4].map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0));
        let finish = match words.next()? {
            "solid" => ShellFinish::Solid,
            "clear" => ShellFinish::Clear,
            "glitter" => ShellFinish::Glitter,
            "solid-glitter" => ShellFinish::SolidGlitter,
            "metallic" => ShellFinish::Metallic,
            "own" => ShellFinish::Own,
            _ => return None,
        };
        words.next().is_none().then_some(ShellChoice {
            outline,
            colour,
            finish,
        })
    }

    pub fn to_value(&self) -> String {
        let outline = match self.outline {
            Outline::Auto => "auto",
            Outline::Notched => "notched",
            Outline::Rounded => "rounded",
        };
        let finish = match self.finish {
            ShellFinish::Solid => "solid",
            ShellFinish::Clear => "clear",
            ShellFinish::Glitter => "glitter",
            ShellFinish::SolidGlitter => "solid-glitter",
            ShellFinish::Metallic => "metallic",
            ShellFinish::Own => "own",
        };
        let [r, g, b] = self.colour;
        format!("{outline} {r:02x}{g:02x}{b:02x} {finish}")
    }
}

pub fn key(stem: &str) -> String {
    stem.nfc().collect()
}

pub fn choices(text: &str) -> HashMap<String, ShellChoice> {
    crate::ini::parse(text)
        .into_iter()
        .filter_map(|(stem, value)| Some((key(&stem), ShellChoice::parse(&value)?)))
        .collect()
}

/// A shell picked on the shelf, written where the studio writes its picks: the labels file,
/// which wins over Config's. None hands the cart back to its automatic shell.
pub fn write_choice(
    root: &std::path::Path,
    stem: &str,
    choice: Option<ShellChoice>,
) -> std::io::Result<()> {
    let value = choice.map_or_else(|| "auto".to_string(), |c| c.to_value());
    let path = root.join(LABELS_SHELL_FILE);
    let mut text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    // The same name spelled with other Unicode (a Mac writes é as e and an accent) is the
    // same cart: that line goes, or the two would race for it.
    let wanted = key(stem);
    for other in crate::ini::parse(&text).into_keys() {
        if other != stem && key(&other) == wanted {
            text = crate::ini::set(&text, &other, None)?;
        }
    }
    let text = crate::ini::set(&text, stem, Some(&value))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    crate::atomic::atomic_write(&path, text.as_bytes())
}

pub fn layered(system: &str, labels: &str) -> HashMap<String, ShellChoice> {
    let mut out = choices(system);
    for (stem, value) in crate::ini::parse(labels) {
        if value.trim() == "auto" {
            out.remove(&key(&stem));
        } else if let Some(choice) = ShellChoice::parse(&value) {
            out.insert(key(&stem), choice);
        }
    }
    out
}

#[cfg(test)]
mod write_tests {
    use super::*;

    #[test]
    fn a_pick_replaces_the_same_name_in_other_unicode() {
        let root = std::env::temp_dir().join(format!("slot-shell-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Labels")).unwrap();
        let nfd = "Pok\u{65}\u{301}mon";
        let nfc = "Pok\u{e9}mon";
        std::fs::write(
            root.join(LABELS_SHELL_FILE),
            format!("{nfd} = auto 112233 solid\nOther = auto 445566 clear\n"),
        )
        .unwrap();
        write_choice(&root, nfc, ShellChoice::parse("auto f2c230 glitter")).unwrap();
        let text = std::fs::read_to_string(root.join(LABELS_SHELL_FILE)).unwrap();
        let shells = layered("", &text);
        assert_eq!(shells.len(), 2, "{text}");
        assert_eq!(shells[&key(nfc)].colour, [0xf2, 0xc2, 0x30]);
        write_choice(&root, nfc, None).unwrap();
        let text = std::fs::read_to_string(root.join(LABELS_SHELL_FILE)).unwrap();
        assert!(!layered("", &text).contains_key(&key(nfc)), "{text}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
