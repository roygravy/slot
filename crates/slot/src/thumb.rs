use slot_retro::{GBA_H, GBA_W};
use slot_store::TV_CANVAS;

pub fn png(xrgb8888: &[u8]) -> Option<Vec<u8>> {
    let (tw, th) = TV_CANVAS;
    let tv = xrgb8888.len() / 4 == (tw * th) as usize;
    // Kept as the screen draws it: a TV console's whole frame, the filter's 602 columns
    // and all; the Game Boys' all share the GBA's.
    let (w, h) = if tv { (tw, th) } else { (GBA_W, GBA_H) };
    let n = (w * h) as usize;
    if xrgb8888.len() < n * 4 {
        return None;
    }
    let mut rgb = Vec::with_capacity(n * 3);
    for px in xrgb8888[..n * 4].chunks_exact(4) {
        rgb.extend_from_slice(&[px[2], px[1], px[0]]);
    }

    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().ok()?;
    writer.write_image_data(&rgb).ok()?;
    writer.finish().ok()?;
    Some(out)
}
