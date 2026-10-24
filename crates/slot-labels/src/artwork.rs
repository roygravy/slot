use crate::Error;
use image::codecs::jpeg::JpegEncoder;
use image::{imageops::FilterType, DynamicImage, ImageFormat, ImageReader, Limits, RgbImage};
use slot_store::Platform;
use std::io::Cursor;

fn bad(e: image::ImageError) -> Error {
    Error::Image(e.to_string())
}

fn decode(bytes: &[u8]) -> Result<DynamicImage, Error> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| Error::Image(e.to_string()))?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().map_err(bad)
}

fn png(label: RgbImage) -> Result<Vec<u8>, Error> {
    let mut out = Cursor::new(Vec::new());
    label.write_to(&mut out, ImageFormat::Png).map_err(bad)?;
    Ok(out.into_inner())
}

/// A label from an artwork pack, which holds it at its own platform's size: a Game Boy cart filed
/// on the Game Boy Color shelf is resized to that shelf's label.
pub(crate) fn fit(bytes: &[u8], platform: Platform) -> Result<Vec<u8>, Error> {
    let (lw, lh) = crate::label_size(platform);
    let image = decode(bytes)?;
    if image.width() < 32 || image.height() < 32 {
        return Err(Error::Image("pack label too small".into()));
    }
    if (image.width(), image.height()) == (lw, lh) {
        return png(image.to_rgb8());
    }
    png(image.resize_exact(lw, lh, FilterType::Lanczos3).to_rgb8())
}

/// A straight-on photo of the whole cart, cropped to the label on it, as the device writes it.
pub(crate) fn prepare(bytes: &[u8], platform: Platform) -> Result<Vec<u8>, Error> {
    png(cart_label(bytes, platform)?)
}

/// The same label, as the JPEG an artwork pack holds.
pub fn cart_label_jpeg(bytes: &[u8], platform: Platform) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, 88)
        .encode_image(&cart_label(bytes, platform)?)
        .map_err(bad)?;
    Ok(out)
}

fn cart_label(bytes: &[u8], platform: Platform) -> Result<RgbImage, Error> {
    let image = decode(bytes)?;
    if matches!(platform, Platform::Nes | Platform::Snes) {
        return tv_label(&image, platform);
    }
    if matches!(platform, Platform::Gb | Platform::Gbc) {
        if let Some(found) = find_sticker(&image, &GB_SPEC) {
            return Ok(cut(&image, found, crate::label_size(platform)));
        }
        if let Some((l, t, r, b)) = game_boy_sticker(&image) {
            // Step just inside the sticker's edge, so no shell rims the label well.
            let (cw, ch) = (r - l, b - t);
            let (ix, iy) = (cw * 12 / 1000 + 1, ch * 12 / 1000 + 1);
            let (lw, lh) = crate::label_size(platform);
            return Ok(image
                .crop_imm(l + ix, t + iy, cw - 2 * ix, ch - 2 * iy)
                .resize_exact(lw, lh, FilterType::Lanczos3)
                .to_rgb8());
        }
    }
    // Only a straight-on scan of a GBA cart, which is about 1.7 times as wide as it is tall.
    let wide = image.width() as f32 / image.height() as f32;
    if platform == Platform::Gba && (1.5..=1.9).contains(&wide) {
        if let Some(found) = find_sticker(&image, &GBA_SPEC) {
            return Ok(cut(&image, found, crate::label_size(platform)));
        }
    }
    let (w, h) = (image.width(), image.height());
    let aspect = w as f64 / h as f64;
    // Only a straight-on scan of the whole cart. Reject box art, angled photos, and tiny
    // thumbnails rather than crop arbitrary imagery.
    let (left, top, right, bottom) = match (platform, w, h) {
        (Platform::Gba, 1000, 574) => (137, 130, 865, 497),
        (Platform::Gba, 600, 355) => (82, 82, 520, 305),
        (Platform::Gba, 473, 283) => (69, 68, 402, 246),
        (Platform::Gba, ..) if w >= 390 && (1.65..=1.80).contains(&aspect) => (
            w * 137 / 1000,
            h * 226 / 1000,
            w * 865 / 1000,
            h * 866 / 1000,
        ),
        (Platform::Gb | Platform::Gbc, ..) if w >= 300 && (0.84..=0.93).contains(&aspect) => (
            w * 135 / 1000,
            h * 285 / 1000,
            w * 865 / 1000,
            h * 858 / 1000,
        ),
        _ => {
            return Err(Error::Image(format!(
                "unsupported cartridge layout {w}x{h}"
            )))
        }
    };
    let (lw, lh) = crate::label_size(platform);
    let cw = right - left;
    let ch = bottom - top;
    // Fit without distortion. Anchor to the top, as in the approved standalone labels.
    let (cw, ch) = if cw * lh > ch * lw {
        (ch * lw / lh, ch)
    } else {
        (cw, cw * lh / lw)
    };
    let left = left + (right - left - cw) / 2;
    let label = image
        .crop_imm(left, top, cw, ch)
        .resize_exact(lw, lh, FilterType::Lanczos3)
        .to_rgb8();
    Ok(label)
}

/// Where to look for a cart's sticker in a scan, as fractions of the cart: the rims beside the
/// sticker that show the shell's own plastic, the band of rows the sticker surely crosses, the
/// rows it may span, and the shapes a sticker of this cart takes.
struct Spec {
    rims: &'static [(f32, f32)],
    rim_rows: (f32, f32),
    /// More bare plastic, under the sticker: columns, then rows. A cart's top and its rims can
    /// be moulded or lit apart.
    below: &'static [((f32, f32), (f32, f32))],
    band: (f32, f32),
    rows: (f32, f32),
    aspect: (f32, f32),
    /// The sticker sits centred between the rims: a pale part of it lost against the plastic
    /// is found again by mirroring the other edge.
    centred: bool,
    /// The sticker's usual width, as a fraction of the cart, for when both its edges are lost
    /// to pale strips that match the plastic; 0 when stickers vary too much to guess.
    width: f32,
}

const SNES_SPEC: Spec = Spec {
    rims: &[(0.03, 0.10), (0.90, 0.97)],
    rim_rows: (0.10, 0.45),
    below: &[((0.40, 0.60), (0.52, 0.58))],
    band: (0.10, 0.40),
    rows: (0.0, 0.60),
    aspect: (1.8, 3.2),
    centred: true,
    width: 0.0,
};

// The NES cart's left is its grip; only its right rim is bare plastic. A Famicom cart, or a
// sticker that is not one, does not fit the aspect.
const NES_SPEC: Spec = Spec {
    rims: &[(0.90, 0.97)],
    rim_rows: (0.10, 0.60),
    below: &[],
    band: (0.15, 0.60),
    rows: (0.0, 0.80),
    aspect: (0.5, 0.75),
    centred: false,
    width: 0.0,
};

const GB_SPEC: Spec = Spec {
    rims: &[(0.02, 0.08), (0.92, 0.98)],
    rim_rows: (0.45, 0.75),
    below: &[((0.30, 0.70), (0.93, 0.97)), ((0.30, 0.70), (0.12, 0.24))],
    band: (0.45, 0.80),
    rows: (0.20, 0.97),
    aspect: (0.95, 1.3),
    centred: true,
    width: 0.73,
};

const GBA_SPEC: Spec = Spec {
    rims: &[(0.02, 0.08), (0.92, 0.98)],
    rim_rows: (0.35, 0.75),
    below: &[],
    band: (0.35, 0.75),
    rows: (0.10, 0.95),
    aspect: (1.6, 2.2),
    centred: true,
    width: 0.0,
};

/// A cart's sticker, found rather than measured, as every console's scans come in their own
/// size and framing: the shell's plastic is sampled on the rims beside the sticker, and the
/// sticker is the block of columns, then rows, that is not that plastic. Gaps narrower than a
/// few hundredths of the cart (a pale strip of the sticker) are bridged.
fn find_sticker(image: &DynamicImage, spec: &Spec) -> Option<(u32, u32, u32, u32)> {
    let rgba = image.to_rgba8();
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, px) in rgba.enumerate_pixels() {
        if px[3] > 128 {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
        }
    }
    if x1 < x0 + 200 || y1 < y0 + 120 {
        return None;
    }
    let (w, h) = (x1 - x0, y1 - y0);
    let at_x = |f: f32| x0 + (w as f32 * f) as u32;
    let at_y = |f: f32| y0 + (h as f32 * f) as u32;
    let median = |areas: &[((u32, u32), (u32, u32))]| {
        let mut sample: [Vec<u8>; 3] = Default::default();
        for &((xa, xb), (ya, yb)) in areas {
            for y in ya..yb {
                for x in xa..xb {
                    for (c, v) in sample.iter_mut().zip(rgba.get_pixel(x, y).0) {
                        c.push(v);
                    }
                }
            }
        }
        (!sample[0].is_empty()).then(|| {
            sample.map(|mut c| {
                c.sort_unstable();
                c[c.len() / 2] as i32
            })
        })
    };
    let rim_rows = (at_y(spec.rim_rows.0), at_y(spec.rim_rows.1));
    let rims: Vec<_> = spec.rims.iter().map(|&(a, b)| ((at_x(a), at_x(b)), rim_rows)).collect();
    let mut shells = vec![median(&rims)?];
    for &((xa, xb), (ya, yb)) in spec.below {
        shells.extend(median(&[((at_x(xa), at_x(xb)), (at_y(ya), at_y(yb)))]));
    }
    let sticker = |x: u32, y: u32| {
        let px = rgba.get_pixel(x, y);
        px[3] > 128
            && shells
                .iter()
                .all(|shell| (0..3).any(|c| (px[c] as i32 - shell[c]).abs() > 30))
    };
    let (band_top, band_bottom) = (at_y(spec.band.0), at_y(spec.band.1));
    let cols: Vec<f32> = (x0..x1)
        .map(|x| {
            (band_top..band_bottom).filter(|&y| sticker(x, y)).count() as f32
                / (band_bottom - band_top).max(1) as f32
        })
        .collect();
    // A recess's shadow is a narrow run of its own, too narrow to count.
    let (l, r) = bridged_run(&cols, 0.3, w * 3 / 100, w * 4 / 100);
    let (mut l, mut r) = (x0 + l, x0 + r);
    if spec.centred && (r - l) * 10 < w * 7 {
        // One edge was lost to a pale stretch of the sticker: the other one, mirrored.
        let (from_left, from_right) = (l - x0, x1 - r);
        let edge = from_left.min(from_right);
        (l, r) = (x0 + edge, x1 - edge);
    }
    let off = ((r - l) as f32 / w as f32 - spec.width).abs();
    if spec.width > 0.0 && off > spec.width * 0.06 {
        // Both edges lost to strips that match the plastic, or the plastic is as dark as the
        // sticker: a sticker of the usual width, centred, as the cart's recess is.
        let edge = (w as f32 * (1.0 - spec.width) / 2.0) as u32;
        (l, r) = (x0 + edge, x1 - edge);
    }
    let (top, bottom) = (at_y(spec.rows.0), at_y(spec.rows.1));
    let rows: Vec<f32> = (top..bottom)
        .map(|y| (l..r).filter(|&x| sticker(x, y)).count() as f32 / (r - l).max(1) as f32)
        .collect();
    let (t, b) = bridged_run(&rows, 0.5, h * 3 / 100, h * 3 / 100);
    let (t, b) = (top + t, top + b);
    let (t, b) = if spec.width > 0.0 {
        // Clear plastic shows the board behind it, so the rows found are rough. A Game Boy
        // sticker has a plain strip down each side: where a strip ends, the sticker does.
        let cw = r - l;
        let strips = [(l + cw * 2 / 100, l + cw * 8 / 100), (r - cw * 8 / 100, r - cw * 2 / 100)];
        // A strip that ran on into plastic of its own grey is not one that ended.
        let shape = |(t, b): (u32, u32)| {
            (1.08..=1.21).contains(&(cw as f32 / (b - t).max(1) as f32))
        };
        let found: Vec<(u32, u32)> = strips
            .iter()
            .filter_map(|&(xa, xb)| {
                let mid = (t + b) / 2;
                let strip = median(&[((xa, xb), (mid - (b - t) / 4, mid + (b - t) / 4))])?;
                let rows: Vec<f32> = (y0..y1)
                    .map(|y| {
                        (xa..xb)
                            .filter(|&x| {
                                let px = rgba.get_pixel(x, y);
                                (0..3).all(|c| (px[c] as i32 - strip[c]).abs() <= 30)
                            })
                            .count() as f32
                            / (xb - xa).max(1) as f32
                    })
                    .collect();
                let (st, sb) = bridged_run(&rows, 0.5, h * 6 / 100, h * 3 / 100);
                Some((y0 + st, y0 + sb)).filter(|&found| shape(found))
            })
            // A strip as grey as the shell runs on into it, and ends where the recess does:
            // far from the rows the sticker's art spans, it is not the sticker's edge.
            .filter(|&(ft, fb)| ft.abs_diff(t) + fb.abs_diff(b) <= h / 8)
            .collect();
        match found[..] {
            [(ta, ba), (tb, bb)] if ta.abs_diff(tb) + ba.abs_diff(bb) <= h / 50 => {
                ((ta + tb) / 2, (ba + bb) / 2)
            }
            [one] => one,
            [a, b2] => {
                let off = |(ft, fb): (u32, u32)| ft.abs_diff(t) + fb.abs_diff(b);
                if off(a) <= off(b2) { a } else { b2 }
            }
            _ => (t, b),
        }
    } else {
        (t, b)
    };
    let (cw, ch) = (r - l, b - t);
    if cw < 60 || ch < 40 {
        return None;
    }
    let aspect = cw as f32 / ch as f32;
    (spec.aspect.0..=spec.aspect.1)
        .contains(&aspect)
        .then_some((l, t, r, b))
}

/// The sticker cut out, just inside its edge so no shell or outline rims the label well.
fn cut(image: &DynamicImage, (l, t, r, b): (u32, u32, u32, u32), size: (u32, u32)) -> RgbImage {
    let (cw, ch) = (r - l, b - t);
    let (ix, iy) = (cw * 15 / 1000 + 1, ch * 15 / 1000 + 1);
    image
        .crop_imm(l + ix, t + iy, cw - 2 * ix, ch - 2 * iy)
        .resize_exact(size.0, size.1, FilterType::Lanczos3)
        .to_rgb8()
}

fn tv_label(image: &DynamicImage, platform: Platform) -> Result<RgbImage, Error> {
    let spec = if platform == Platform::Nes { &NES_SPEC } else { &SNES_SPEC };
    let found = find_sticker(image, spec)
        .ok_or_else(|| Error::Image("no sticker on this cartridge scan".into()))?;
    let (l, t, r, b) = found;
    // The SNES sticker keeps its own shape: a wide PAL/SFC one or a taller NTSC one. The shelf
    // fits it to whichever mould the cart takes.
    let size = match platform {
        Platform::Snes if (r - l) as f32 / (b - t) as f32 > 2.65 => (277, 97),
        _ => crate::label_size(platform),
    };
    Ok(cut(image, found, size))
}

/// A Game Boy pak photo's sticker, found rather than measured: carts are shot with different
/// margins, and a Color pak's sticker sits lower than a Game Boy one's. The shell's plastic is
/// sampled on the rims either side of the sticker; the sticker is the block of columns, then
/// rows, that is not that plastic. Even a grey sticker edge on a grey pak differs enough.
fn game_boy_sticker(image: &DynamicImage) -> Option<(u32, u32, u32, u32)> {
    let rgba = image.to_rgba8();
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, px) in rgba.enumerate_pixels() {
        if px[3] > 128 {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
        }
    }
    if x1 < x0 + 200 || y1 < y0 + 200 {
        return None;
    }
    let (w, h) = (x1 - x0, y1 - y0);
    let mut sample: [Vec<u8>; 3] = Default::default();
    for y in y0 + h * 45 / 100..y0 + h * 75 / 100 {
        for x in (x0 + w * 3 / 100..x0 + w * 9 / 100).chain(x0 + w * 91 / 100..x0 + w * 97 / 100) {
            for (c, v) in sample.iter_mut().zip(rgba.get_pixel(x, y).0) {
                c.push(v);
            }
        }
    }
    let shell = sample.map(|mut c| {
        c.sort_unstable();
        c[c.len() / 2] as i32
    });
    let sticker = |x: u32, y: u32| {
        let px = rgba.get_pixel(x, y);
        px[3] > 128 && (0..3).any(|c| (px[c] as i32 - shell[c]).abs() > 22)
    };
    let (band_top, band_bottom) = (y0 + h * 40 / 100, y0 + h * 80 / 100);
    let cols: Vec<f32> = (x0..x1)
        .map(|x| {
            (band_top..band_bottom).filter(|&y| sticker(x, y)).count() as f32
                / (band_bottom - band_top) as f32
        })
        .collect();
    let (l, r) = longest_run(&cols, 0.6);
    let (l, r) = (x0 + l, x0 + r);
    let (top, bottom) = (y0 + h * 20 / 100, y0 + h * 95 / 100);
    let rows: Vec<f32> = (top..bottom)
        .map(|y| (l..r).filter(|&x| sticker(x, y)).count() as f32 / (r - l).max(1) as f32)
        .collect();
    let (t, b) = longest_run(&rows, 0.6);
    let (t, b) = (top + t, top + b);
    let (cw, ch) = (r - l, b - t);
    // Anything but a sticker-shaped block across most of the pak is left to the fixed crop.
    let fits = cw * 100 >= w * 60 && (1.0..=1.3).contains(&(cw as f32 / ch.max(1) as f32));
    fits.then_some((l, t, r, b))
}

/// The longest run over the threshold, gaps up to `gap` long bridged; runs shorter than
/// `least` are specks and left out.
fn bridged_run(fill: &[f32], threshold: f32, gap: u32, least: u32) -> (u32, u32) {
    let (gap, least) = (gap as usize, least as usize);
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut start = None;
    for (i, &v) in fill.iter().chain([&0.0]).enumerate() {
        match (v > threshold, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                if i - s >= least.max(1) {
                    match runs.last_mut() {
                        Some(last) if s - last.1 <= gap => last.1 = i,
                        _ => runs.push((s, i)),
                    }
                }
                start = None;
            }
            _ => {}
        }
    }
    runs.into_iter()
        .max_by_key(|(s, e)| e - s)
        .map_or((0, 0), |(s, e)| (s as u32, e as u32))
}

fn longest_run(fill: &[f32], threshold: f32) -> (u32, u32) {
    let (mut best, mut start) = ((0, 0), None);
    for (i, &v) in fill.iter().chain([&0.0]).enumerate() {
        match (v > threshold, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                if i - s > best.1 - best.0 {
                    best = (s, i);
                }
                start = None;
            }
            _ => {}
        }
    }
    (best.0 as u32, best.1 as u32)
}
