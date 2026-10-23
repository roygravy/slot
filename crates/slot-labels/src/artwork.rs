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

/// NES and SNES scans come in every size and framing, so the sticker is found rather than
/// measured: it is the block of columns, then rows, in the top of the cart that is not the shell's
/// own plastic, sampled low on the cart where no sticker sits.
fn tv_label(image: &DynamicImage, platform: Platform) -> Result<RgbImage, Error> {
    let nes = platform == Platform::Nes;
    let rgba = image.to_rgba8();
    let opaque = |x: u32, y: u32| rgba.get_pixel(x, y)[3] > 128;
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, px) in rgba.enumerate_pixels() {
        if px[3] > 128 {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
        }
    }
    let unsupported = || Error::Image("no sticker on this cartridge scan".into());
    if x1 < x0 + 200 || y1 < y0 + 120 {
        return Err(unsupported());
    }
    let (w, h) = (x1 - x0, y1 - y0);
    let mut sample: [Vec<u8>; 3] = Default::default();
    for y in y0 + h * 80 / 100..y0 + h * 92 / 100 {
        for x in x0 + w * 30 / 100..x0 + w * 70 / 100 {
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
        opaque(x, y) && (0..3).any(|c| (px[c] as i32 - shell[c]).abs() > 48)
    };
    let bottom = y0 + h * if nes { 75 } else { 55 } / 100;
    let cols: Vec<f32> = (x0..x1)
        .map(|x| (y0..bottom).filter(|&y| sticker(x, y)).count() as f32 / (bottom - y0) as f32)
        .collect();
    let (l, r) = longest_run(&cols, 0.45);
    let (l, r) = (x0 + l, x0 + r);
    let rows: Vec<f32> = (y0..bottom)
        .map(|y| (l..r).filter(|&x| sticker(x, y)).count() as f32 / (r - l).max(1) as f32)
        .collect();
    let (t, b) = longest_run(&rows, 0.5);
    let (t, b) = (y0 + t, y0 + b);
    let (cw, ch) = (r - l, b - t);
    if cw < 60 || ch < 40 {
        return Err(unsupported());
    }
    // A Famicom cart, or a sticker that is not one, does not fit the mould.
    let aspect = cw as f32 / ch as f32;
    let fits = if nes {
        (0.5..=0.75).contains(&aspect)
    } else {
        (1.8..=3.2).contains(&aspect)
    };
    if !fits {
        return Err(Error::Image(format!("sticker {cw}x{ch} does not fit the cart")));
    }
    // Step just inside the sticker's edge, so no shell or outline rims the label well.
    let (ix, iy) = (cw * 15 / 1000 + 1, ch * 15 / 1000 + 1);
    // The SNES sticker keeps its own shape: a wide PAL/SFC one or a taller NTSC one. The shelf
    // fits it to whichever mould the cart takes.
    let (lw, lh) = match platform {
        Platform::Snes if aspect > 2.45 => (277, 97),
        _ => crate::label_size(platform),
    };
    Ok(image
        .crop_imm(l + ix, t + iy, cw - 2 * ix, ch - 2 * iy)
        .resize_exact(lw, lh, FilterType::Lanczos3)
        .to_rgb8())
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
