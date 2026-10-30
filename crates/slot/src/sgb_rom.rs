//! The Super Game Boy's own borders, read straight from the player's dump of the Super Game
//! Boy (or Super Game Boy 2) SNES program in BIOS: no Nintendo art ships with slot., only
//! where each border sits in that program.
//!
//! Each border is three pieces: its tiles and its tile map (on the SGB packed with its own
//! compression, on the SGB 2 stored as they are) and its colours. They are turned into the layout mGBA takes
//! for a border: 0x2000 bytes of tiles, then the map, with palettes 4 to 7 at 0x800 of it.

use std::path::Path;

/// The size of one border as mGBA takes it: tiles, then map RAM.
pub const BORDER_LEN: usize = 0x3000;

const TILES_LEN: usize = 0x2000;
const MAP_LEN: usize = 0x700;

/// One border in the SGB program: its map, its tiles (with their unpacked sizes, which show
/// the dump is the one the table was made from; no size: stored as they are) and its colours,
/// as file offsets.
struct Piece {
    map: usize,
    map_len: usize,
    tiles: usize,
    tiles_len: usize,
    colours: usize,
}

const fn piece(
    map: usize,
    map_len: usize,
    tiles: usize,
    tiles_len: usize,
    colours: usize,
) -> Piece {
    Piece {
        map,
        map_len,
        tiles,
        tiles_len,
        colours,
    }
}

/// The Super Game Boy (title "Super GAMEBOY"): Classic, Windows, Cork, Forest, Theatre, Cats,
/// Desk, Shapes. Each one's colours are the eight background palettes it loads.
const SGB1: [Piece; 8] = [
    piece(0x1e261, 0x800, 0x1d868, 0xda0, 0x26ff1),
    piece(0x1aea5, 0x1000, 0x18000, 0x1000, 0x1b474),
    piece(0x25f12, 0x800, 0x25335, 0xca0, 0x2609c),
    piece(0x1c660, 0x800, 0x1b674, 0x1800, 0x1d568),
    piece(0x2dc3c, 0x800, 0x2c6c8, 0x1000, 0x2d83c),
    piece(0x20b27, 0x1000, 0x1f716, 0x800, 0x20d73),
    piece(0x2997a, 0x800, 0x28000, 0x1200, 0x29ba1),
    piece(0x1e3cd, 0x800, 0x1e90a, 0xe00, 0x1e70a),
];

/// The Super Game Boy 2 (title "Super GAMEBOY2"): Classic, Circuit, Beach, Stone, Gears,
/// Jungle, Dolphins, Temple.
const SGB2: [Piece; 8] = [
    piece(0x51800, 0, 0x5c2c0, 0, 0x56d80),
    piece(0x52000, 0, 0x59100, 0, 0x57000),
    piece(0x67000, 0, 0x63000, 0, 0x67800),
    piece(0x52800, 0, 0x4e000, 0, 0x57400),
    piece(0x6d000, 0, 0x68000, 0, 0x67a00),
    piece(0x53000, 0, 0x48000, 0, 0x57a00),
    piece(0x53800, 0, 0x5ccc0, 0, 0x57c00),
    piece(0x54800, 0, 0x50000, 0, 0x57800),
];

/// Unpack one piece of the SGB program. A header of four words: the escape word, the packed
/// length, one unused, the first word out. Then words, each copied out, but for the escape
/// word: a word offset into what is out already and a count of words to repeat from there (no
/// count: the escape word itself).
pub fn unpack(rom: &[u8], at: usize) -> Option<Vec<u8>> {
    let word =
        |o: usize| -> Option<u16> { Some(u16::from_le_bytes([*rom.get(o)?, *rom.get(o + 1)?])) };
    let escape = word(at)?;
    let len = usize::from(word(at + 2)?);
    let mut out = vec![*rom.get(at + 6)?, *rom.get(at + 7)?];
    let mut s = 8;
    while s < 8 + len {
        let w = word(at + s)?;
        if w != escape {
            out.extend_from_slice(&w.to_le_bytes());
            s += 2;
            continue;
        }
        let from = usize::from(word(at + s + 2)?);
        let count = usize::from(*rom.get(at + s + 4)?);
        s += 5;
        if count == 0 {
            out.extend_from_slice(&escape.to_le_bytes());
        }
        for i in 0..count {
            let w = [*out.get(from + 2 * i)?, *out.get(from + 2 * i + 1)?];
            out.extend_from_slice(&w);
        }
        if out.len() > 0x10000 {
            return None;
        }
    }
    Some(out)
}

/// Which Super Game Boy a SNES program is, from the title in its header: 1, 2 or none. A
/// copier's 512-byte header in front is skipped.
fn which(rom: &[u8]) -> Option<(u8, &[u8])> {
    let rom = if rom.len() % 0x8000 == 512 {
        &rom[512..]
    } else {
        rom
    };
    match rom.get(0x7fc0..0x7fd5)? {
        b"Super GAMEBOY        " => Some((1, rom)),
        b"Super GAMEBOY2       " => Some((2, rom)),
        _ => None,
    }
}

/// Border `n` (1 to 8) out of a Super Game Boy program, in mGBA's layout. mGBA has room for
/// 256 tiles, so the tiles the map uses are renumbered from 0 in the order it uses them.
pub fn border(rom: &[u8], n: u8) -> Option<Vec<u8>> {
    let (which, rom) = which(rom)?;
    let table: &[Piece] = match which {
        1 => &SGB1,
        _ => &SGB2,
    };
    let p = table.get(usize::from(n).checked_sub(1)?)?;
    let (tiles, mut map) = if p.map_len == 0 {
        (
            rom.get(p.tiles..)?.to_vec(),
            rom.get(p.map..p.map + MAP_LEN)?.to_vec(),
        )
    } else {
        let tiles = unpack(rom, p.tiles).filter(|t| t.len() == p.tiles_len)?;
        (tiles, unpack(rom, p.map).filter(|m| m.len() == p.map_len)?)
    };
    let colours = rom.get(p.colours + 4 * 32..p.colours + 8 * 32)?;
    let mut out = vec![0; BORDER_LEN];
    let mut used: Vec<usize> = Vec::new();
    for e in map[..MAP_LEN].chunks_exact_mut(2) {
        let entry = u16::from_le_bytes([e[0], e[1]]);
        let tile = usize::from(entry & 0x3ff);
        let new = match used.iter().position(|&t| t == tile) {
            Some(i) => i,
            None => {
                used.push(tile);
                used.len() - 1
            }
        };
        if new >= TILES_LEN / 32 {
            return None;
        }
        out[new * 32..new * 32 + 32].copy_from_slice(tiles.get(tile * 32..tile * 32 + 32)?);
        let entry = (entry & !0x3ff) | new as u16;
        e.copy_from_slice(&entry.to_le_bytes());
    }
    out[TILES_LEN..TILES_LEN + MAP_LEN].copy_from_slice(&map[..MAP_LEN]);
    out[TILES_LEN + 0x800..TILES_LEN + 0x880].copy_from_slice(colours);
    Some(out)
}

/// The Super Game Boy program for the SGB (`two` false) or the SGB 2 in BIOS: any .sfc or .smc
/// there whose header names it, whatever the file is called.
pub fn find(bios: &Path, two: bool) -> Option<Vec<u8>> {
    let want = if two { 2 } else { 1 };
    std::fs::read_dir(bios)
        .ok()?
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            name.ends_with(".sfc") || name.ends_with(".smc")
        })
        .filter(|e| e.metadata().is_ok_and(|m| m.len() <= 0x100000))
        .filter_map(|e| std::fs::read(e.path()).ok())
        .find(|rom| which(rom).is_some_and(|(w, _)| w == want))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_words_and_repeats() {
        // Escape 0xBEEF, 12 packed bytes after the header, first word 0x1111.
        let mut rom = vec![0xef, 0xbe, 12, 0, 0, 0, 0x11, 0x11];
        rom.extend([0x22, 0x22]); // a word
        rom.extend([0xef, 0xbe, 0, 0, 2]); // repeat two words from the start
        rom.extend([0xef, 0xbe, 0, 0, 0]); // the escape word itself
        let out = unpack(&rom, 0).unwrap();
        assert_eq!(
            out,
            [0x11, 0x11, 0x22, 0x22, 0x11, 0x11, 0x22, 0x22, 0xef, 0xbe]
        );
    }

    #[test]
    fn a_program_that_is_not_the_sgb_has_no_borders() {
        assert!(border(&vec![0; 0x40000], 1).is_none());
    }
}
