use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const OUT_W: usize = slot_gfx::OUT_W as usize;
const OUT_H: usize = slot_gfx::OUT_H as usize;

pub const CARD_LOGO: &str = "System/bootlogo.bmp";
pub const USER_LOGO: &str = "Config/bootlogo.bmp";
/// The game picture left as the boot screen, kept on the card so slot can show the same
/// picture while the game loads again.
pub const BOOT_PICTURE: &str = "Config/boot_picture.bmp";
const LOGO: &str = "bootlogo.bmp";
const ORIGINAL: &str = "bootlogo.baseos.bmp";
const PARTITION: &str = "boot-resource";
const MOUNT: &str = "/tmp/slot-boot-resource";

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Outcome {
    Same,
    Installed,
    Invalid,
}

pub struct Bmp {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<[u8; 3]>,
}

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

pub fn size(bmp: &[u8]) -> Option<(usize, usize)> {
    if bmp.get(0..2)? != b"BM" || u32_at(bmp, 14)? < 40 {
        return None;
    }
    let w = u32_at(bmp, 18)? as i32;
    let h = u32_at(bmp, 22)? as i32;
    let (w, h) = (w.unsigned_abs() as usize, h.unsigned_abs() as usize);
    ((1..=4096).contains(&w) && (1..=4096).contains(&h)).then_some((w, h))
}

pub fn decode(bmp: &[u8]) -> Option<Bmp> {
    let (width, height) = size(bmp)?;
    if u16_at(bmp, 28)? != 24 || u32_at(bmp, 30)? != 0 {
        return None;
    }
    let top_down = (u32_at(bmp, 22)? as i32) < 0;
    let offset = u32_at(bmp, 10)? as usize;
    let stride = (width * 3 + 3) & !3;
    let data = bmp.get(offset..offset + stride * height)?;
    let mut rgb = Vec::with_capacity(width * height);
    for y in 0..height {
        let row = if top_down { y } else { height - 1 - y };
        for px in data[row * stride..].chunks_exact(3).take(width) {
            rgb.push([px[2], px[1], px[0]]);
        }
    }
    Some(Bmp { width, height, rgb })
}

pub fn encode(img: &Bmp) -> Vec<u8> {
    let stride = (img.width * 3 + 3) & !3;
    let data = stride * img.height;
    let mut out = Vec::with_capacity(54 + data);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&((54 + data) as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(img.width as i32).to_le_bytes());
    out.extend_from_slice(&(img.height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(data as u32).to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for y in (0..img.height).rev() {
        for c in &img.rgb[y * img.width..(y + 1) * img.width] {
            out.extend_from_slice(&[c[2], c[1], c[0]]);
        }
        out.resize(out.len() + stride - img.width * 3, 0);
    }
    out
}

fn turned(img: &Bmp) -> Bmp {
    let (w, h) = (img.height, img.width);
    let mut rgb = vec![[0; 3]; w * h];
    for y in 0..img.height {
        for x in 0..img.width {
            rgb[(img.width - 1 - x) * w + y] = img.rgb[y * img.width + x];
        }
    }
    Bmp {
        width: w,
        height: h,
        rgb,
    }
}

pub fn compose(wordmark: &Bmp, (width, height): (usize, usize)) -> Option<Bmp> {
    let turned_mark;
    let mark = if height > width {
        turned_mark = turned(wordmark);
        &turned_mark
    } else {
        wordmark
    };
    if mark.width > width || mark.height > height {
        return None;
    }
    let mut rgb = vec![*mark.rgb.first()?; width * height];
    let (x0, y0) = ((width - mark.width) / 2, (height - mark.height) / 2);
    for y in 0..mark.height {
        let at = (y0 + y) * width + x0;
        rgb[at..at + mark.width].copy_from_slice(&mark.rgb[y * mark.width..(y + 1) * mark.width]);
    }
    Some(Bmp { width, height, rgb })
}

/// The wordmark laid over a ground picture (`ground` is `gw` x `gh` RGBA), stretched to the
/// panel: the wordmark's own background colour is taken as clear and its lighter pixels as
/// white ink, so the boot screen shows the same wallpaper the shelf opens on.
pub fn compose_over(
    wordmark: &Bmp,
    (width, height): (usize, usize),
    ground: &[u8],
    (gw, gh): (usize, usize),
) -> Option<Bmp> {
    if height > width || wordmark.width > width || wordmark.height > height {
        return compose(wordmark, (width, height));
    }
    if ground.len() < gw * gh * 4 || gw == 0 || gh == 0 {
        return compose(wordmark, (width, height));
    }
    let mut rgb = Vec::with_capacity(width * height);
    for y in 0..height {
        let sy = (y * gh / height).min(gh - 1);
        for x in 0..width {
            let sx = (x * gw / width).min(gw - 1);
            let o = (sy * gw + sx) * 4;
            rgb.push([ground[o], ground[o + 1], ground[o + 2]]);
        }
    }
    let bg = *wordmark.rgb.first()?;
    let bg_lum = (bg[0] as f32 + bg[1] as f32 + bg[2] as f32) / 3.0;
    let (x0, y0) = ((width - wordmark.width) / 2, (height - wordmark.height) / 2);
    for y in 0..wordmark.height {
        for x in 0..wordmark.width {
            let p = wordmark.rgb[y * wordmark.width + x];
            let lum = (p[0] as f32 + p[1] as f32 + p[2] as f32) / 3.0;
            let a = ((lum - bg_lum) / (255.0 - bg_lum)).clamp(0.0, 1.0);
            if a <= 0.0 {
                continue;
            }
            let d = &mut rgb[(y0 + y) * width + x0 + x];
            for c in 0..3 {
                d[c] = (d[c] as f32 * (1.0 - a) + 255.0 * a).round() as u8;
            }
        }
    }
    Some(Bmp { width, height, rgb })
}

struct Plan {
    logo: Vec<u8>,
    current: Option<Vec<u8>>,
    original: Option<Vec<u8>>,
}

fn plan(wordmark: &[u8], dir: &Path) -> Result<Plan, Outcome> {
    let mark = decode(wordmark).ok_or(Outcome::Invalid)?;
    let current = fs::read(dir.join(LOGO)).ok();
    let original = fs::read(dir.join(ORIGINAL)).ok();
    let panel = original
        .as_deref()
        .or(current.as_deref())
        .and_then(size)
        .ok_or(Outcome::Invalid)?;
    // The boot screen is the dark menus' flat ground, so boot, menus and shelf share a tone.
    let flat = slot_ui::theme::MENU_FLAT_DARK;
    let ground: Vec<u8> = (0..OUT_W * OUT_H)
        .flat_map(|_| [flat[0], flat[1], flat[2], 255])
        .collect();
    let logo = compose_over(&mark, panel, &ground, (OUT_W, OUT_H))
        .map(|b| encode(&b))
        .ok_or(Outcome::Invalid)?;
    if current.as_deref() == Some(&logo[..]) {
        return Err(Outcome::Same);
    }
    Ok(Plan {
        logo,
        current,
        original,
    })
}

/// What `install` would end in without writing anything, or `None` when it would write.
pub fn pending(wordmark: &[u8], dir: &Path) -> Option<Outcome> {
    plan(wordmark, dir).err()
}

pub fn install(wordmark: &[u8], dir: &Path) -> io::Result<Outcome> {
    let Plan {
        logo,
        current,
        original,
    } = match plan(wordmark, dir) {
        Ok(plan) => plan,
        Err(outcome) => return Ok(outcome),
    };
    if let Some(current) = &current {
        if original.is_none() {
            replace(dir, ORIGINAL, current)?;
        }
        if current.len() == logo.len() {
            overwrite(dir, LOGO, &logo)?;
            return Ok(Outcome::Installed);
        }
    }
    replace(dir, LOGO, &logo)?;
    Ok(Outcome::Installed)
}

fn overwrite(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    let mut f = fs::OpenOptions::new().write(true).open(dir.join(name))?;
    f.write_all(bytes)?;
    f.sync_all()
}

fn replace(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    let tmp = dir.join("bootlogo.tmp");
    let mut f = File::create(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    fs::rename(&tmp, dir.join(name))?;
    File::open(dir)?.sync_all()
}

pub fn chosen(card: &Path) -> Option<Vec<u8>> {
    for name in [USER_LOGO, CARD_LOGO] {
        let Ok(logo) = fs::read(card.join(name)) else {
            continue;
        };
        if decode(&logo).is_some() {
            return Some(logo);
        }
        eprintln!("slot: bootlogo: {name} is not a 24-bit bmp, skipping it");
    }
    None
}

/// Run `f` on the mounted boot-resource partition.
fn on_partition(rw: bool, f: impl FnOnce(&Path)) {
    let Some(device) = partition(PARTITION) else {
        eprintln!("slot: bootlogo: no {PARTITION} partition");
        return;
    };
    if fs::create_dir_all(MOUNT).is_err()
        || !run(
            "mount",
            &[
                "-t",
                "vfat",
                "-o",
                if rw { "rw,noatime" } else { "ro,noatime" },
                &device,
                MOUNT,
            ],
        )
    {
        eprintln!("slot: bootlogo: could not mount {device}");
        return;
    }
    f(Path::new(MOUNT));
    if !run("umount", &[MOUNT]) {
        eprintln!("slot: bootlogo: could not unmount {MOUNT}");
    }
}

pub fn refresh(card: &Path) {
    let Some(logo) = chosen(card) else {
        return;
    };
    // Mounted read-only unless the logo changes, so a power cut at boot cannot hurt the partition.
    on_partition(false, |dir| match pending(&logo, dir) {
        Some(Outcome::Invalid) => eprintln!("slot: bootlogo: no logo fits this panel, left alone"),
        Some(_) => {}
        None if !run("mount", &["-o", "remount,rw,noatime", MOUNT]) => {
            eprintln!("slot: bootlogo: could not remount {MOUNT} to write the logo")
        }
        None => match install(&logo, dir) {
            Ok(Outcome::Installed) => eprintln!("slot: bootlogo: installed"),
            Ok(_) => {}
            Err(e) => eprintln!("slot: bootlogo: {e}"),
        },
    });
}

/// A screen picture (`w` x `h` RGBA, top row first) stretched to a panel.
pub fn picture_bmp(rgba: &[u8], (w, h): (usize, usize), (pw, ph): (usize, usize)) -> Option<Bmp> {
    if w == 0 || h == 0 || rgba.len() < w * h * 4 || pw == 0 || ph == 0 {
        return None;
    }
    let mut rgb = Vec::with_capacity(pw * ph);
    for y in 0..ph {
        let sy = (y * h / ph).min(h - 1);
        for x in 0..pw {
            let sx = (x * w / pw).min(w - 1);
            let o = (sy * w + sx) * 4;
            rgb.push([rgba[o], rgba[o + 1], rgba[o + 2]]);
        }
    }
    Some(Bmp {
        width: pw,
        height: ph,
        rgb,
    })
}

/// Write `bmp` as the boot logo in `dir`, sized like the panel's own.
fn install_bmp(rgba: &[u8], size: (usize, usize), dir: &Path) -> io::Result<Outcome> {
    let current = fs::read(dir.join(LOGO)).ok();
    let original = fs::read(dir.join(ORIGINAL)).ok();
    let Some(panel) = original
        .as_deref()
        .or(current.as_deref())
        .and_then(self::size)
    else {
        return Ok(Outcome::Invalid);
    };
    if panel.1 > panel.0 {
        return Ok(Outcome::Invalid);
    }
    let Some(logo) = picture_bmp(rgba, size, panel).map(|b| encode(&b)) else {
        return Ok(Outcome::Invalid);
    };
    if let Some(current) = &current {
        if original.is_none() {
            replace(dir, ORIGINAL, current)?;
        }
    }
    replace(dir, LOGO, &logo)?;
    Ok(Outcome::Installed)
}

/// Leave the game's picture as the boot screen, and keep a copy on the card.
pub fn install_picture(card: &Path, rgba: &[u8], size: (usize, usize)) {
    if let Some(bmp) = picture_bmp(rgba, size, size) {
        let path = card.join(BOOT_PICTURE);
        if let Err(e) = slot_store::atomic_write(&path, &encode(&bmp)) {
            eprintln!("slot: bootlogo: {}: {e}", path.display());
        }
    }
    on_partition(true, |dir| match install_bmp(rgba, size, dir) {
        Ok(Outcome::Installed) => {
            eprintln!("slot: bootlogo: the game's picture is the boot screen")
        }
        Ok(_) => eprintln!("slot: bootlogo: the game's picture does not fit this panel"),
        Err(e) => eprintln!("slot: bootlogo: {e}"),
    });
}

/// The game picture kept from the last power off, taken off the card so it shows only once.
pub fn take_picture(card: &Path) -> Option<Bmp> {
    let path = card.join(BOOT_PICTURE);
    let bytes = fs::read(&path).ok()?;
    let _ = fs::remove_file(&path);
    decode(&bytes)
}

fn partition(name: &str) -> Option<String> {
    for entry in fs::read_dir("/sys/class/block").ok()?.flatten() {
        let uevent = fs::read_to_string(entry.path().join("uevent")).unwrap_or_default();
        if uevent.lines().any(|l| l == format!("PARTNAME={name}")) {
            return Some(
                PathBuf::from("/dev")
                    .join(entry.file_name())
                    .display()
                    .to_string(),
            );
        }
    }
    None
}

fn run(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .is_ok_and(|s| s.success())
}
