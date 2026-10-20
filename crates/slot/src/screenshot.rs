//! Screenshots: the frame as it shows, written to the card as a PNG off the frame loop.

use std::path::PathBuf;

/// Write `rgba` (top-down, `w` by `h`) to `path` as an opaque PNG, on a thread of its own so
/// the frame never waits on the card.
pub fn save(path: PathBuf, rgba: Vec<u8>, w: u32, h: u32) {
    std::thread::spawn(move || {
        if let Err(e) = write(&path, &rgba, w, h) {
            eprintln!("slot: screenshot: {}: {e}", path.display());
        }
    });
}

fn write(path: &std::path::Path, rgba: &[u8], w: u32, h: u32) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let rgb: Vec<u8> = rgba
        .chunks_exact(4)
        .take((w * h) as usize)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    let mut bytes = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut bytes, w, h);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(std::io::Error::other)?;
        writer.write_image_data(&rgb).map_err(std::io::Error::other)?;
    }
    slot_store::atomic_write(path, &bytes)
}
