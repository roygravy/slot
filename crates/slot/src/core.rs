use std::path::{Path, PathBuf};

use slot_retro::{LibretroCore, MockCore, RetroCore};
use slot_store::{Core, GbPalette, NesColors, Platform, TvCable};

use crate::root;

const CORE_ENV: &str = "SLOT_CORE";

pub fn dylib_name(core: Core) -> String {
    format!(
        "{}_libretro.{}",
        core.as_str(),
        std::env::consts::DLL_EXTENSION
    )
}

fn candidates(root: &Path, core: Core) -> Vec<PathBuf> {
    if let Some(named) = std::env::var_os(CORE_ENV) {
        return vec![PathBuf::from(named)];
    }
    let name = dylib_name(core);
    let mut paths = vec![root.join("System").join(&name)];
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        paths.push(dir.join(&name));
        paths.push(dir.join("vendor").join(&name));
    }
    paths.push(Path::new("vendor").join(&name));
    paths
}

/// Read every core's library once, in the background, so the first game of a console does not
/// wait on the card for its core: on the device the first SNES insert stuttered while Snes9x
/// was read in (Roy). Later opens find it in memory.
pub fn warm_cores(root: &Path) {
    let files: Vec<PathBuf> = Core::ALL
        .into_iter()
        .filter_map(|c| candidates(root, c).into_iter().find(|p| p.exists()))
        .collect();
    let _ = std::thread::Builder::new()
        .name("warm-cores".into())
        .spawn(move || {
            for file in files {
                let _ = std::fs::read(file);
            }
        });
}

/// Whether a library for this core is on the card (or named): what `open_core` will find.
pub fn core_found(root: &Path, core: Core) -> bool {
    candidates(root, core).iter().any(|p| p.exists())
}

pub struct Opened {
    pub core: Box<dyn RetroCore>,
    pub named: bool,
}

pub fn open_core(
    root: &Path,
    core: Core,
    platform: Platform,
    serial: &str,
    colour: bool,
    palette: Option<GbPalette>,
    link: Option<u8>,
) -> Opened {
    let paths = candidates(root, core);
    match open_named(root, core, platform, serial, colour, palette, link, &paths) {
        Some(core) => Opened { core, named: true },
        None => {
            report_missing(core, &paths);
            Opened {
                core: Box::new(MockCore::new()),
                named: false,
            }
        }
    }
}

pub fn open_core_for(
    root: &Path,
    core: Core,
    serial: &str,
    colour: bool,
    paths: &[PathBuf],
) -> Box<dyn RetroCore> {
    open_core_for_palette(root, core, Platform::Gba, serial, colour, None, paths)
}

pub fn open_core_for_palette(
    root: &Path,
    core: Core,
    platform: Platform,
    serial: &str,
    colour: bool,
    palette: Option<GbPalette>,
    paths: &[PathBuf],
) -> Box<dyn RetroCore> {
    open_named(root, core, platform, serial, colour, palette, None, paths).unwrap_or_else(|| {
        report_missing(core, paths);
        Box::new(MockCore::new())
    })
}

#[allow(clippy::too_many_arguments)]
fn open_named(
    root: &Path,
    core: Core,
    platform: Platform,
    serial: &str,
    colour: bool,
    palette: Option<GbPalette>,
    link: Option<u8>,
    paths: &[PathBuf],
) -> Option<Box<dyn RetroCore>> {
    let bios = root::bios_dir(root);
    let saves = root::saves_dir(root);
    for path in paths {
        if !path.exists() {
            continue;
        }
        if core == Core::Gambatte {
            ensure_dmg_palette(&bios);
        }
        // Gambatte reads this option during retro_init, so it has to exist
        // before the core is initialized (setting it afterward is too late).
        let opened = if core == Core::Gambatte {
            LibretroCore::open_with_options(
                path,
                &bios,
                &saves,
                &[("gambatte_gb_bootloader", "enabled")],
            )
        } else {
            LibretroCore::open_with(path, &bios, &saves)
        };
        match opened {
            Ok(mut opened) => {
                let (w, h) = platform.canvas();
                opened.set_canvas(w, h);
                apply_core_options(
                    &mut opened,
                    core,
                    platform,
                    serial,
                    root::has_real_bios(root),
                    colour,
                    palette,
                );
                if let Some(player) = link {
                    apply_link_options(&mut opened, core, player);
                }
                eprintln!("slot: core {}", path.display());
                return Some(Box::new(opened));
            }
            Err(e) => eprintln!("slot: {}: {e}", path.display()),
        }
    }
    None
}

pub fn palette_for(
    chosen: Option<GbPalette>,
    platform: Platform,
    rom: &Path,
    link: Option<u8>,
) -> Option<GbPalette> {
    // A cart on the GB shelf is played as a Game Boy, whatever its header says: some Game Boy
    // games mark themselves as Game Boy Color only and run fine on the DMG. On the GBC shelf
    // only a cart that cannot do colour takes a palette.
    let fits = match platform {
        Platform::Gb => true,
        Platform::Gbc => dmg_only(rom),
        Platform::Gba | Platform::Nes | Platform::Snes => false,
    };
    chosen.filter(|_| link.is_none() && fits)
}

pub fn dmg_only(rom: &Path) -> bool {
    use std::io::Read;
    let mut header = [0u8; 0x150];
    std::fs::File::open(rom)
        .and_then(|mut f| f.read_exact(&mut header))
        .is_ok_and(|_| header[0x143] & 0x80 == 0)
}

/// A cart that speaks to the Super Game Boy (header 0x146 = 0x03), Game Boy Color or not.
pub fn sgb_aware(rom: &Path) -> bool {
    use std::io::Read;
    let mut header = [0u8; 0x150];
    std::fs::File::open(rom)
        .and_then(|mut f| f.read_exact(&mut header))
        .is_ok_and(|_| header[0x146] == 0x03)
}

fn report_missing(core: Core, paths: &[PathBuf]) {
    eprintln!(
        "slot: no {} core found, running the mock test pattern instead. Looked in: {}",
        core.as_str(),
        paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

pub fn colour_option(
    which: Core,
    platform: Platform,
    on: bool,
) -> Option<(&'static str, &'static str)> {
    match which {
        Core::Mgba => Some((
            "mgba_color_correction",
            if on && platform == Platform::Gb {
                "GBC"
            } else if on {
                "Auto"
            } else {
                "OFF"
            },
        )),
        Core::Gpsp => Some((
            "gpsp_color_correction",
            if on { "enabled" } else { "disabled" },
        )),
        Core::Gambatte => Some((
            "gambatte_gbc_color_correction",
            if on { "GBC only" } else { "disabled" },
        )),
        Core::Fceumm | Core::Snes9x => None,
    }
}

/// The Game Boy Color's screen answers quicker than the Game Boy's, so its games trail less.
pub fn lcd_ghosting_option(
    which: Core,
    platform: Platform,
    on: bool,
) -> Option<(&'static str, &'static str)> {
    match which {
        Core::Mgba => Some((
            "mgba_interframe_blending",
            if on { "lcd_ghosting" } else { "OFF" },
        )),
        Core::Gpsp => Some(("gpsp_frame_mixing", if on { "enabled" } else { "disabled" })),
        Core::Gambatte => Some((
            "gambatte_mix_frames",
            match (on, platform) {
                (false, _) => "disabled",
                (true, Platform::Gbc) => "lcd_ghosting_gbc",
                (true, _) => "lcd_ghosting",
            },
        )),
        Core::Fceumm | Core::Snes9x => None,
    }
}

/// A TV console's look in its core: the picture as its cable carried it to the TV, colour
/// fringes and all, and for the NES the colours a TV's decoder made of its signal; with the
/// effects off, the console's own sharp pixels in the core's own colours.
pub fn tv_options(
    which: Core,
    look: Option<crate::app::TvLook>,
) -> Vec<(&'static str, &'static str)> {
    let cable = look.map_or(TvCable::Off, |l| l.cable);
    let colors = look.map_or(NesColors::Standard, |l| l.colors);
    match which {
        Core::Fceumm => vec![
            (
                "fceumm_ntsc_filter",
                match cable {
                    TvCable::Off => "disabled",
                    // The NES's core draws no aerial: its RF is the nearest, composite.
                    TvCable::Rf | TvCable::Composite => "composite",
                    TvCable::SVideo => "svideo",
                    TvCable::Rgb => "rgb",
                },
            ),
            (
                "fceumm_palette",
                match colors {
                    NesColors::Standard => "default",
                    NesColors::Sony => "sony-cxa2025as-us",
                    NesColors::Authentic => "composite-direct-fbx",
                    NesColors::Nescap => "nescap",
                    NesColors::Pvm => "pvm-style-d93-fbx",
                },
            ),
        ],
        Core::Snes9x => vec![(
            "snes9x_blargg",
            match cable {
                TvCable::Off => "disabled",
                TvCable::Rf => "rf",
                TvCable::Composite => "composite",
                TvCable::SVideo => "s-video",
                TvCable::Rgb => "rgb",
            },
        )],
        // mGBA has no filter of its own: slot. runs Snes9x's on the Super Game Boy's picture.
        Core::Mgba => vec![(slot_retro::SLOT_NTSC, cable.name())],
        Core::Gpsp | Core::Gambatte => Vec::new(),
    }
}

fn gambatte_palette_name(palette: GbPalette) -> String {
    let name = palette.core_name();
    let base = name
        .split(['↑', '↓', '←', '→'])
        .next()
        .unwrap_or(name)
        .trim();
    if name.starts_with("SGB ") {
        let digits: String = name[4..]
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect();
        return format!("SGB - {digits}");
    }
    match name {
        "DMG Green" => "GB - DMG".to_string(),
        "GB Pocket" => "GB - Pocket".to_string(),
        "GB Light" => "GB - Light".to_string(),
        _ => match base {
            "Grayscale" | "GBC Gray" | "GBC Grayscale" => "GBC - Grayscale".to_string(),
            "GBC Reverse" => "GBC - Inverted".to_string(),
            "GBC Pale Yellow" => "GBC - Yellow".to_string(),
            value if value.starts_with("GBC ") => {
                format!("GBC - {}", value.trim_start_matches("GBC "))
            }
            _ => "GB - DMG".to_string(),
        },
    }
}

/// slot.'s own DMG Green, lightest to darkest: a yellow-green glass with dark, slightly blue
/// pixels, nearer a real DMG-01 than Gambatte's built-in "GB - DMG".
pub const DMG_GREEN: [u32; 4] = [0x8E9526, 0x5F7C48, 0x36554F, 0x213A3B];

/// Marks the palette file as slot.'s own, so one a player wrote themselves is left alone.
const DMG_MARK: &str = "[slot. DMG Green]";

/// The custom palette Gambatte reads from `BIOS/palettes/default.pal` when DMG Green is on.
pub fn dmg_palette_text() -> String {
    let mut text = format!("{DMG_MARK}\n");
    for (i, c) in DMG_GREEN.iter().enumerate() {
        text.push_str(&format!(
            "Background{i}={c}\nSprite%201{i}={c}\nSprite%202{i}={c}\n"
        ));
    }
    text
}

/// Put slot.'s DMG Green where Gambatte looks for a custom palette, unless the player has
/// a default.pal of their own there.
pub fn ensure_dmg_palette(bios: &Path) {
    let path = bios.join("palettes").join("default.pal");
    let want = dmg_palette_text();
    match std::fs::read_to_string(&path) {
        Ok(have) if have == want => return,
        Ok(have) if !have.starts_with(DMG_MARK) => return,
        _ => {}
    }
    if let Err(e) = std::fs::create_dir_all(bios.join("palettes"))
        .and_then(|()| slot_store::atomic_write(&path, want.as_bytes()))
    {
        eprintln!("slot: core: could not write {}: {e}", path.display());
    }
}

pub fn gb_palette_options(which: Core, palette: GbPalette) -> Vec<(&'static str, String)> {
    match which {
        Core::Mgba => vec![("mgba_gb_colors", palette.core_name().to_string())],
        Core::Gambatte if palette.core_name() == "DMG Green" => {
            vec![("gambatte_gb_colorization", "custom".to_string())]
        }
        Core::Gambatte => vec![
            ("gambatte_gb_colorization", "internal".to_string()),
            (
                "gambatte_gb_internal_palette",
                gambatte_palette_name(palette),
            ),
        ],
        Core::Gpsp | Core::Fceumm | Core::Snes9x => Vec::new(),
    }
}

/// Every screen effect off: the Game Boy's picture in the core's own plain grey.
pub fn gb_grey_options(which: Core) -> Vec<(&'static str, String)> {
    match which {
        Core::Mgba => vec![
            ("mgba_gb_colors_preset", "0".to_string()),
            ("mgba_gb_colors", "Grayscale".to_string()),
        ],
        Core::Gambatte => vec![("gambatte_gb_colorization", "disabled".to_string())],
        Core::Gpsp | Core::Fceumm | Core::Snes9x => Vec::new(),
    }
}

/// Palettes off: the game's own Game Boy Color colors, as when it starts that way.
pub fn gb_palette_off_options(which: Core) -> Vec<(&'static str, String)> {
    match which {
        Core::Mgba => vec![
            ("mgba_gb_colors_preset", "1".to_string()),
            ("mgba_gb_colors", "GBC Dark Green →A".to_string()),
        ],
        Core::Gambatte => vec![
            ("gambatte_gb_colorization", "auto".to_string()),
            (
                "gambatte_gb_internal_palette",
                "GBC - Dark Green".to_string(),
            ),
        ],
        Core::Gpsp | Core::Fceumm | Core::Snes9x => Vec::new(),
    }
}

pub fn apply_link_options(core: &mut LibretroCore, which: Core, player: u8) {
    if which != Core::Mgba {
        return;
    }
    core.set_option("mgba_link", "on");
    core.set_option("mgba_link_player", &player.to_string());
    eprintln!("slot: core: link mode on, player {player}");
}

/// slot.'s own Super Game Boy border, for the games that bring none.
const SGB_BORDER_GRAVY: &[u8] = include_bytes!("../../../assets/sgb_border_gravy.bin");
/// The same for the Super Game Boy 2, in its own layout.
const SGB2_BORDER_GRAVY: &[u8] = include_bytes!("../../../assets/sgb_border_gravy2.bin");

/// The Super Game Boy's own eight borders (the SGB 2's with `two`), each there or not: from
/// its program in BIOS (an .sfc or .smc of the SGB), else from files lifted from it, sgb1_1.bin
/// to sgb1_8.bin (sgb2_ for the SGB 2) in BIOS/sgb_borders or loose in BIOS.
fn sgb_own_borders(root: &Path, two: bool) -> Vec<Option<Vec<u8>>> {
    let bios = root::bios_dir(root);
    let rom = crate::sgb_rom::find(&bios, two);
    (1..=8)
        .map(|n| {
            if let Some(b) = rom.as_deref().and_then(|r| crate::sgb_rom::border(r, n)) {
                return Some(b);
            }
            let name = format!("sgb{}_{n}.bin", if two { 2 } else { 1 });
            [bios.join("sgb_borders").join(&name), bios.join(&name)]
                .iter()
                .find_map(|at| std::fs::read(at).ok())
                .filter(|b| b.len() == SGB_BORDER_GRAVY.len())
        })
        .collect()
}

/// The numbers of the Super Game Boy's own borders on the card (see `sgb_own_borders`).
pub fn sgb_borders_on_card(root: &Path, two: bool) -> Vec<u8> {
    (1..=8)
        .zip(sgb_own_borders(root, two))
        .filter_map(|(n, b)| b.map(|_| n))
        .collect()
}

/// The border for the choice made (see `SlotState::sgb_border`), and whether it stays over a
/// game's own: Auto takes the SGB's first border, or slot.'s, and gives way to the game's.
fn sgb_border_for(root: &Path, choice: u8, two: bool) -> (Vec<u8>, bool) {
    let mut borders = sgb_own_borders(root, two);
    let mut own = |n: u8| borders.get_mut(usize::from(n) - 1).and_then(Option::take);
    let gravy = if two {
        SGB2_BORDER_GRAVY
    } else {
        SGB_BORDER_GRAVY
    };
    match choice {
        1 => (gravy.to_vec(), true),
        // No border: every tile blank, every colour black.
        2 => (vec![0; gravy.len()], true),
        n @ 11..=18 => (own(n - 10).unwrap_or_else(|| gravy.to_vec()), true),
        _ => (own(1).unwrap_or_else(|| gravy.to_vec()), false),
    }
}

/// Put the chosen border where mGBA reads it, and the option that makes it take it: the
/// option changes each time (`turn`) so a choice made while playing shows at once.
pub fn set_sgb_border(root: &Path, choice: u8, two: bool, turn: u32) -> (&'static str, String) {
    let (border, lock) = sgb_border_for(root, choice, two);
    let at = root::bios_dir(root).join("slot_sgb_border.bin");
    if std::fs::read(&at).ok().as_deref() != Some(&border[..]) {
        if let Err(e) = std::fs::write(&at, &border) {
            eprintln!("slot: sgb: could not write {}: {e}", at.display());
        }
    }
    // And every border slot. could have put in before, which a saved state may bring back:
    // mGBA swaps one of those for this one, and keeps a game's own.
    let mut ours = [SGB_BORDER_GRAVY, SGB2_BORDER_GRAVY].concat();
    ours.extend(vec![0; SGB_BORDER_GRAVY.len()]);
    for two in [false, true] {
        for b in sgb_own_borders(root, two).into_iter().flatten() {
            ours.extend(b);
        }
    }
    let at = root::bios_dir(root).join("slot_sgb_ours.bin");
    if std::fs::read(&at).ok().as_deref() != Some(&ours[..]) {
        if let Err(e) = std::fs::write(&at, &ours) {
            eprintln!("slot: sgb: could not write {}: {e}", at.display());
        }
    }
    let mode = if lock { "lock" } else { "auto" };
    ("mgba_slot_sgb_border", format!("{mode}:{turn}"))
}

/// mGBA as a Super Game Boy, drawing on the SNES's screen: its borders, and the SGB's own
/// colours for the games that set none. The Super Game Boy 2 has a secret: a Game Boy Color
/// game plays in its own colours inside the border (`gbc`), as mGBA's Super Game Boy Color.
/// Test only: with bsnes on the card the Super Game Boy plays as the real thing does, a SNES
/// running the SGB cart (BIOS/SGB1.sfc or SGB2.sfc), to see whether the H700 keeps up.
pub fn bsnes_sgb_test(root: &Path) -> Option<PathBuf> {
    let path = root
        .join("System")
        .join(format!("bsnes_libretro.{}", std::env::consts::DLL_EXTENSION));
    path.exists().then_some(path)
}

pub fn open_bsnes_sgb(root: &Path, path: &Path, two: bool) -> Box<dyn RetroCore> {
    let bios = root::bios_dir(root);
    let saves = root::saves_dir(root);
    let cart = if two { "SGB2.sfc" } else { "SGB1.sfc" };
    if !bios.join(cart).exists() {
        eprintln!("slot: bsnes-sgb: {} is missing; bsnes cannot start", bios.join(cart).display());
    }
    match LibretroCore::open_with_options(path, &bios, &saves, &[("bsnes_sgb_bios", cart)]) {
        Ok(mut core) => {
            let (w, h) = Platform::Snes.canvas();
            core.set_canvas(w, h);
            eprintln!("slot: bsnes-sgb: core {} with {cart}", path.display());
            Box::new(core)
        }
        Err(e) => {
            eprintln!("slot: bsnes-sgb: {}: {e}", path.display());
            Box::new(MockCore::new())
        }
    }
}

pub fn sgb_options(root: &Path, core: &mut dyn RetroCore, choice: u8, two: bool, gbc: bool) {
    let (w, h) = Platform::Snes.canvas();
    core.set_canvas(w, h);
    let model = if gbc {
        "Super Game Boy Color"
    } else {
        "Super Game Boy"
    };
    core.set_option("mgba_gb_model", model);
    core.set_option("mgba_sgb_borders", "ON");
    core.set_option("mgba_gb_colors_preset", "0");
    core.set_option("mgba_gb_colors", "SGB 1-A");
    let (key, value) = set_sgb_border(root, choice, two, 0);
    core.set_option(key, &value);
}

pub fn apply_core_options(
    core: &mut LibretroCore,
    which: Core,
    platform: Platform,
    serial: &str,
    bios: bool,
    colour: bool,
    palette: Option<GbPalette>,
) {
    core.set_option(&format!("{}_frameskip", which.as_str()), "auto");
    if which == Core::Mgba {
        if let Some((key, value)) = lcd_ghosting_option(which, platform, true) {
            core.set_option(key, value);
        }
        core.set_option("mgba_sgb_borders", "OFF");
        core.set_option("mgba_force_gbp", "ON");
        match palette {
            Some(p) => {
                core.set_option("mgba_gb_model", "Game Boy");
                core.set_option("mgba_gb_colors_preset", "0");
                core.set_option("mgba_gb_colors", p.core_name());
            }
            None => {
                core.set_option("mgba_gb_model", "Autodetect");
                core.set_option("mgba_gb_colors_preset", "1");
                core.set_option("mgba_gb_colors", "GBC Dark Green →A");
            }
        }
        if let Some((key, value)) = colour_option(which, platform, colour) {
            core.set_option(key, value);
        }
    }
    if which == Core::Gpsp {
        if let Some((key, value)) = lcd_ghosting_option(which, platform, true) {
            core.set_option(key, value);
        }
        core.set_option("gpsp_serial", serial);
        core.set_option("gpsp_rtc_time_source", "system");
        if bios {
            core.set_option("gpsp_boot_mode", "bios");
        }
        if let Some((key, value)) = colour_option(which, platform, colour) {
            core.set_option(key, value);
        }
    }
    if which == Core::Gambatte {
        if let Some((key, value)) = lcd_ghosting_option(which, platform, true) {
            core.set_option(key, value);
        }
        // A palette means a Game Boy: run the cart as one, so the palette always takes.
        let model = match (palette, platform) {
            (Some(_), Platform::Gb) => "GB",
            _ => "Auto",
        };
        core.set_option("gambatte_gb_hwmode", model);
        core.set_option("gambatte_gb_bootloader", "enabled");
        core.set_option("gambatte_gbc_color_correction_mode", "accurate");
        core.set_option("gambatte_gbc_frontlight_position", "central");
        if let Some(palette) = palette {
            for (key, value) in gb_palette_options(which, palette) {
                core.set_option(key, &value);
            }
        } else {
            core.set_option("gambatte_gb_colorization", "auto");
            // A game with no colours of its own falls back on this, as mGBA's does, rather
            // than on Gambatte's own GB - DMG, which slot. no longer uses.
            core.set_option("gambatte_gb_internal_palette", "GBC - Dark Green");
        }
        if let Some((key, value)) = colour_option(which, platform, colour) {
            core.set_option(key, value);
        }
    }
    if which == Core::Fceumm {
        // A TV's tube hid the picture's edges, where games leave rubbish (Super Mario Bros.
        // 3's scroll seams). Cut as much off the sides as off the top and bottom, so the
        // NES's own pixel shape holds when the rest fills the 4:3 screen.
        core.set_option("fceumm_overscan_v_top", "16");
        core.set_option("fceumm_overscan_v_bottom", "16");
        core.set_option("fceumm_overscan_h_left", "8");
        core.set_option("fceumm_overscan_h_right", "8");
    }
    if which == Core::Snes9x {
        core.set_option("snes9x_overscan", "disabled");
        // High-resolution screens blend to 256 wide, as a TV's blur showed them.
        core.set_option("snes9x_hires_blend", "merge");
        core.set_option("snes9x_audio_interpolation", "gaussian");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn each_core_spells_colour_correction_its_own_way() {
        assert_eq!(
            colour_option(Core::Mgba, Platform::Gba, true),
            Some(("mgba_color_correction", "Auto"))
        );
        assert_eq!(
            colour_option(Core::Mgba, Platform::Gba, false),
            Some(("mgba_color_correction", "OFF"))
        );
        assert_eq!(
            colour_option(Core::Gpsp, Platform::Gba, true),
            Some(("gpsp_color_correction", "enabled"))
        );
        assert_eq!(
            colour_option(Core::Gpsp, Platform::Gba, false),
            Some(("gpsp_color_correction", "disabled"))
        );
    }

    #[test]
    fn mgba_uses_gbc_correction_for_game_boy_games() {
        assert_eq!(
            colour_option(Core::Mgba, Platform::Gb, true),
            Some(("mgba_color_correction", "GBC"))
        );
    }

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn candidates_search_the_named_cores_own_filename_only() {
        let _g = lock();
        std::env::remove_var(CORE_ENV);

        let root = Path::new("/root");
        let mgba = candidates(root, Core::Mgba);
        let gpsp = candidates(root, Core::Gpsp);
        assert_ne!(mgba, gpsp, "the two cores searched the same paths");
        assert!(!mgba.is_empty());
        assert!(!gpsp.is_empty());
        for path in &mgba {
            let name = path.file_name().unwrap().to_str().unwrap();
            assert!(name.starts_with("mgba_libretro"), "{name} is not mGBA's");
        }
        for path in &gpsp {
            let name = path.file_name().unwrap().to_str().unwrap();
            assert!(name.starts_with("gpsp_libretro"), "{name} is not gpSP's");
        }
    }

    #[test]
    fn candidates_search_the_roots_own_system_directory() {
        let _g = lock();
        std::env::remove_var(CORE_ENV);
        let root = Path::new("/some/content/root");
        assert_eq!(
            candidates(root, Core::Gpsp)[0],
            root.join("System").join(dylib_name(Core::Gpsp)),
        );
    }

    #[test]
    fn the_env_override_ignores_which_core_was_asked_for() {
        let _g = lock();
        std::env::set_var(CORE_ENV, "/dev/null/named-core");
        let root = Path::new("/root");
        let mgba = candidates(root, Core::Mgba);
        let gpsp = candidates(root, Core::Gpsp);
        std::env::remove_var(CORE_ENV);
        assert_eq!(mgba, vec![PathBuf::from("/dev/null/named-core")]);
        assert_eq!(
            mgba, gpsp,
            "the override stopped winning for one of the cores"
        );
    }
}
