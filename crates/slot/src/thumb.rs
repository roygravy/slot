use slot_retro::{GBA_H, GBA_W};

const TV_W: u32 = 256;
const TV_H: u32 = 224;

pub fn png(xrgb8888: &[u8]) -> Option<Vec<u8>> {
    // A TV console's frame is its own size; the Game Boys' all share the GBA's.
    let (w, h) = match xrgb8888.len() / 4 {
        n if n == (TV_W * TV_H) as usize => (TV_W, TV_H),
        _ => (GBA_W, GBA_H),
    };
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
