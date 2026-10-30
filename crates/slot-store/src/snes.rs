//! The SNES cart's internal header: its title, and the region that says which cart it came in.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Where the header can sit: LoROM, HiROM and ExHiROM, before any copier header.
const HEADERS: [u64; 3] = [0x7fc0, 0xffc0, 0x40_ffc0];
const TITLE_LEN: usize = 21;
const REGION: usize = 0x19;
const COMPLEMENT: usize = 0x1c;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Region {
    /// The American cart: North America, Canada and Brazil.
    Ntsc,
    /// The Super Famicom's and Europe's cart, which share a shape.
    Pal,
}

/// The header whose checksum and its complement add up, as a real cart's do.
fn header(rom: &Path) -> Option<[u8; 32]> {
    let mut f = std::fs::File::open(rom).ok()?;
    let len = f.metadata().ok()?.len();
    // Copiers put a 512 byte header of their own in front of the ROM.
    let skip = if len % 1024 == 512 { 512 } else { 0 };
    HEADERS.iter().find_map(|at| {
        let mut buf = [0u8; 32];
        f.seek(SeekFrom::Start(skip + at)).ok()?;
        f.read_exact(&mut buf).ok()?;
        let complement = u16::from_le_bytes([buf[COMPLEMENT], buf[COMPLEMENT + 1]]);
        let sum = u16::from_le_bytes([buf[COMPLEMENT + 2], buf[COMPLEMENT + 3]]);
        (complement ^ sum == 0xffff).then_some(buf)
    })
}

pub fn title(rom: &Path) -> Option<String> {
    let buf = header(rom)?;
    let text: String = buf[..TITLE_LEN]
        .iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                ' '
            }
        })
        .collect();
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// The cart's shape, as `region` finds it, read once per file: the shelf and the slot ask
/// every frame.
pub fn cached_region(rom: &Path) -> Region {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    static SEEN: OnceLock<Mutex<HashMap<PathBuf, Region>>> = OnceLock::new();
    let seen = SEEN.get_or_init(Default::default);
    if let Some(r) = seen.lock().ok().and_then(|m| m.get(rom).copied()) {
        return r;
    }
    let r = region(rom);
    if let Ok(mut m) = seen.lock() {
        m.insert(rom.to_path_buf(), r);
    }
    r
}

/// The cart's shape, from the region in the file name, else from the header.
pub fn region(rom: &Path) -> Region {
    let stem = rom.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    region_of_name(stem).unwrap_or_else(|| match header(rom).map(|h| h[REGION]) {
        Some(0x01 | 0x0f | 0x10) => Region::Ntsc,
        Some(_) => Region::Pal,
        None => Region::Ntsc,
    })
}

/// The region a No-Intro or GoodTools name gives, the first one named winning.
pub fn region_of_name(stem: &str) -> Option<Region> {
    let tags = stem
        .split(['(', ')', '[', ']'])
        .skip(1)
        .step_by(2)
        .flat_map(|tag| tag.split(',').map(str::trim));
    for tag in tags {
        match tag {
            "USA" | "U" | "Canada" | "Brazil" | "UE" => return Some(Region::Ntsc),
            "Japan" | "J" | "Europe" | "E" | "Germany" | "France" | "Spain" | "Italy"
            | "Australia" | "Sweden" | "Netherlands" | "Korea" | "Asia" | "World" => {
                return Some(Region::Pal)
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_says_which_cart() {
        assert_eq!(
            region_of_name("Super Mario World (USA)"),
            Some(Region::Ntsc)
        );
        assert_eq!(
            region_of_name("Super Mario World (U) [!]"),
            Some(Region::Ntsc)
        );
        assert_eq!(
            region_of_name("Super Mario Kart (Europe)"),
            Some(Region::Pal)
        );
        assert_eq!(
            region_of_name("Super Mario Kart (Japan)"),
            Some(Region::Pal)
        );
        assert_eq!(
            region_of_name("Tetris Attack (USA, Europe)"),
            Some(Region::Ntsc)
        );
        assert_eq!(region_of_name("Homebrew"), None);
    }

    #[test]
    fn a_header_with_a_matching_checksum_names_the_region() {
        let d = std::env::temp_dir().join(format!("slot-snes-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let mut rom = vec![0u8; 0x8000 + 512];
        let h = 512 + 0x7fc0;
        rom[h..h + 9].copy_from_slice(b"EXAMPLE  ");
        rom[h + REGION] = 0x02;
        rom[h + COMPLEMENT..h + COMPLEMENT + 4].copy_from_slice(&[0x34, 0x12, 0xcb, 0xed]);
        let path = d.join("Example.sfc");
        std::fs::write(&path, &rom).unwrap();
        assert_eq!(region(&path), Region::Pal);
        assert_eq!(title(&path).as_deref(), Some("EXAMPLE"));
        std::fs::remove_dir_all(&d).unwrap();
    }
}
