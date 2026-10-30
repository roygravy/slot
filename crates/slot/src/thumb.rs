use slot_retro::{GBA_H, GBA_W};
use slot_store::TV_CANVAS;

/// A TV console's picture is kept at the console's own 256 columns: the filter's 602 are
/// one signal smeared wide.
const TV_W: u32 = 256;

pub fn png(xrgb8888: &[u8]) -> Option<Vec<u8>> {
    let (tw, th) = TV_CANVAS;
    let tv = xrgb8888.len() / 4 == (tw * th) as usize;
    // A TV console's frame is its own size; the Game Boys' all share the GBA's.
    let (w, h) = if tv { (TV_W, th) } else { (GBA_W, GBA_H) };
    let n = (w * h) as usize;
    if !tv && xrgb8888.len() < n * 4 {
        return None;
    }
    let mut rgb = Vec::with_capacity(n * 3);
    if tv {
        for row in xrgb8888.chunks_exact(tw as usize * 4) {
            for x in 0..TV_W as usize {
                // Each column averages the stretch of the filter's columns it covers.
                let a = x * tw as usize / TV_W as usize;
                let b = ((x + 1) * tw as usize / TV_W as usize).max(a + 1);
                let mut sum = [0u32; 3];
                for px in row[a * 4..b * 4].chunks_exact(4) {
                    for (s, c) in sum.iter_mut().zip([px[2], px[1], px[0]]) {
                        *s += c as u32;
                    }
                }
                rgb.extend(sum.map(|s| (s / (b - a) as u32) as u8));
            }
        }
    } else {
        for px in xrgb8888[..n * 4].chunks_exact(4) {
            rgb.extend_from_slice(&[px[2], px[1], px[0]]);
        }
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
