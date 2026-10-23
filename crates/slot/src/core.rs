use std::path::{Path, PathBuf};

use slot_retro::{LibretroCore, MockCore, RetroCore};
use slot_store::{Core, GbPalette, Platform};

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

pub fn lcd_ghosting_option(which: Core, on: bool) -> Option<(&'static str, &'static str)> {
    match which {
        Core::Mgba => Some((
            "mgba_interframe_blending",
            if on { "lcd_ghosting" } else { "OFF" },
        )),
        Core::Gpsp => Some(("gpsp_frame_mixing", if on { "enabled" } else { "disabled" })),
        Core::Gambatte => Some((
            "gambatte_mix_frames",
            if on { "lcd_ghosting" } else { "disabled" },
        )),
        Core::Fceumm | Core::Snes9x => None,
    }
}

/// A TV console's NTSC filter: the picture as a composite cable carried it to the TV, colour
/// fringes and all, or the console's own sharp pixels.
pub fn tv_filter_option(which: Core, on: bool) -> Option<(&'static str, &'static str)> {
    let value = if on { "composite" } else { "disabled" };
    match which {
        Core::Fceumm => Some(("fceumm_ntsc_filter", value)),
        Core::Snes9x => Some(("snes9x_blargg", value)),
        Core::Mgba | Core::Gpsp | Core::Gambatte => None,
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
        text.push_str(&format!("Background{i}={c}\nSprite%201{i}={c}\nSprite%202{i}={c}\n"));
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

/// Palettes off: the game's own Game Boy Color colors, as when it starts that way.
pub fn gb_palette_off_options(which: Core) -> Vec<(&'static str, String)> {
    match which {
        Core::Mgba => vec![
            ("mgba_gb_colors_preset", "1".to_string()),
            ("mgba_gb_colors", "GBC Dark Green →A".to_string()),
        ],
        Core::Gambatte => vec![
            ("gambatte_gb_colorization", "auto".to_string()),
            ("gambatte_gb_internal_palette", "GBC - Dark Green".to_string()),
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
        if let Some((key, value)) = lcd_ghosting_option(which, true) {
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
        if let Some((key, value)) = lcd_ghosting_option(which, true) {
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
        if let Some((key, value)) = lcd_ghosting_option(which, true) {
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
        // A TV hid the top and bottom eight lines; games leave rubbish there.
        core.set_option("fceumm_overscan_v_top", "8");
        core.set_option("fceumm_overscan_v_bottom", "8");
        core.set_option("fceumm_overscan_h_left", "0");
        core.set_option("fceumm_overscan_h_right", "0");
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
