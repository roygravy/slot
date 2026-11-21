//! The panel's own colour calibration, set by eye in General > Screen Calibration: a gain per
//! channel for the white point and an exponent per channel for the gamma curve. The display engine
//! applies it as a 3x1D table after composition, to games and menus alike, with no work per
//! frame. Kept in `Config/display.cal` as two lines: `gain r g b` and `gamma r g b`.

/// Gains darken a channel to move the white point; exponents bend its curve.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Calibration {
    pub gain: [f32; 3],
    pub gamma: [f32; 3],
    /// Colour of the whole screen, 1.0 as drawn, `saturation_default()` when not set. Done in the final blit, not in the table.
    pub saturation: f32,
    /// Contrast, 0 as the curve leaves it, -1 to 1: up lifts the middle and light tones, down
    /// lowers them. The dark tones, black and white stay where they are, so no shadow detail
    /// is lost.
    pub contrast: f32,
}

impl Default for Calibration {
    fn default() -> Self {
        Calibration {
            gain: [1.0; 3],
            gamma: [1.0; 3],
            saturation: saturation_default(),
            contrast: 0.0,
        }
    }
}

pub const GAIN_MIN: f32 = 0.5;
pub const CONTRAST_RANGE: f32 = 1.0;
pub const GAMMA_RANGE: (f32, f32) = (0.5, 2.0);
/// The standard saturation: the colours as drawn, on every console. The RG35XX Plus had 1.4
/// here until 2026-11-17; Roy chose one start for all, more colour is a stop on the slider.
pub fn saturation_default() -> f32 {
    1.0
}

/// The saturation a card may keep: 40% to 250% of the standard.
pub fn saturation_range() -> (f32, f32) {
    let d = saturation_default();
    (d * 0.4, d * 2.5)
}

impl Calibration {
    /// Reads `gain r g b` and `gamma r g b`; a missing line keeps the neutral value,
    /// anything out of range is pulled back in.
    pub fn parse(text: &str) -> Calibration {
        let mut cal = Calibration::default();
        for line in text.lines() {
            let mut words = line.split_whitespace();
            let (Some(key), values) = (words.next(), words) else {
                continue;
            };
            let v: Vec<f32> = values.filter_map(|w| w.parse().ok()).collect();
            if key == "saturation" && v.len() == 1 && v[0].is_finite() {
                let (lo, hi) = saturation_range();
                cal.saturation = v[0].clamp(lo, hi);
                continue;
            }
            if key == "contrast" && v.len() == 1 && v[0].is_finite() {
                cal.contrast = v[0].clamp(-CONTRAST_RANGE, CONTRAST_RANGE);
                continue;
            }
            if v.len() != 3 || !v.iter().all(|x| x.is_finite()) {
                continue;
            }
            match key {
                "gain" => cal.gain = [0, 1, 2].map(|i| v[i].clamp(GAIN_MIN, 1.0)),
                "gamma" => cal.gamma = [0, 1, 2].map(|i| v[i].clamp(GAMMA_RANGE.0, GAMMA_RANGE.1)),
                _ => {}
            }
        }
        cal
    }

    pub fn text(&self) -> String {
        let [r, g, b] = self.gain;
        let [er, eg, eb] = self.gamma;
        format!(
            "gain {r:.4} {g:.4} {b:.4}\ngamma {er:.4} {eg:.4} {eb:.4}\nsaturation {:.4}\ncontrast {:.4}\n",
            self.saturation, self.contrast
        )
    }

    pub fn is_neutral(&self) -> bool {
        *self == Calibration::default()
    }

    /// 256 entries of 0x00RRGGBB, as the display engine takes them.
    pub fn table(&self) -> [u32; 256] {
        std::array::from_fn(|i| {
            let x = i as f64 / 255.0;
            let c = |ch: usize| {
                let v = f64::from(self.gain[ch]) * s_curve(x.powf(f64::from(self.gamma[ch])), self.contrast);
                (v * 255.0).round().clamp(0.0, 255.0) as u32
            };
            (c(0) << 16) | (c(1) << 8) | c(2)
        })
    }
}

/// Contrast on a tone from 0 to 1: nothing below `SHADOWS`, most in the light middle, nothing
/// at white. Even at the ends no two neighbouring tones run together.
fn s_curve(v: f64, contrast: f32) -> f64 {
    const SHADOWS: f64 = 0.2;
    const STRENGTH: f64 = 5.0;
    let u = ((v - SHADOWS) / (1.0 - SHADOWS)).max(0.0);
    v + f64::from(contrast) * STRENGTH * u * u * (1.0 - v) * (1.0 - v)
}

/// Loads the card's calibration, neutral when there is none.
pub fn load(root: &std::path::Path) -> Calibration {
    std::fs::read_to_string(root.join("Config/display.cal"))
        .map(|t| Calibration::parse(&t))
        .unwrap_or_default()
}

/// Hands the table to the display engine. Off the device there is nothing to do.
#[cfg(feature = "device")]
pub fn apply(cal: &Calibration) -> std::io::Result<()> {
    use std::ffi::{c_int, c_ulong};
    use std::os::fd::AsRawFd;

    // Allwinner's sunxi display ioctls (H700): four unsigned longs, screen 0, a userspace
    // pointer and a byte count.
    const SET_GAMMA_TABLE: c_ulong = 0x10b;
    const ENABLE_GAMMA: c_ulong = 0x10c;
    extern "C" {
        fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    }
    let disp = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/disp")?;
    let table = cal.table();
    let send = |request: c_ulong, args: [c_ulong; 4]| {
        // The arguments and the table outlive the synchronous call.
        if unsafe { ioctl(disp.as_raw_fd(), request, args.as_ptr()) } < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    };
    // Neutral is sent as the identity table rather than switched off: only these two
    // requests are known to behave on BaseOS.
    send(
        SET_GAMMA_TABLE,
        [
            0,
            table.as_ptr() as c_ulong,
            std::mem::size_of_val(&table) as c_ulong,
            0,
        ],
    )?;
    send(ENABLE_GAMMA, [0; 4])
}

/// Switches the engine's gamma off before the deep sleep: with it on, the display engine
/// froze on waking (Roy, 2026-11-07). The table is sent again after waking.
#[cfg(feature = "device")]
pub fn release() -> std::io::Result<()> {
    use std::ffi::{c_int, c_ulong};
    use std::os::fd::AsRawFd;

    const DISABLE_GAMMA: c_ulong = 0x10d;
    extern "C" {
        fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    }
    let disp = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/disp")?;
    let args: [c_ulong; 4] = [0; 4];
    if unsafe { ioctl(disp.as_raw_fd(), DISABLE_GAMMA, args.as_ptr()) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(feature = "device"))]
pub fn release() -> std::io::Result<()> {
    Ok(())
}

#[cfg(not(feature = "device"))]
pub fn apply(_cal: &Calibration) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_is_the_identity() {
        let t = Calibration::default().table();
        for (i, v) in t.iter().enumerate() {
            let i = i as u32;
            assert_eq!(*v, (i << 16) | (i << 8) | i);
        }
    }

    #[test]
    fn text_round_trips_and_clamps() {
        let cal = Calibration {
            gain: [1.0, 0.95, 0.9],
            gamma: [1.1, 1.0, 0.9],
            saturation: 1.3,
            contrast: 0.2,
        };
        assert_eq!(Calibration::parse(&cal.text()), cal);
        let wild = Calibration::parse("gain 2 0.1 nan\ngamma 9 0.1 1\nsaturation 9\n");
        assert_eq!(wild.saturation, 2.5);
        assert_eq!(wild.gain, [1.0; 3]);
        assert_eq!(wild.gamma, [2.0, 0.5, 1.0]);
    }

    #[test]
    fn contrast_keeps_black_and_white_and_order() {
        for c in [-1.0, -0.5, 0.5, 1.0] {
            let t = Calibration { contrast: c, ..Default::default() }.table();
            assert_eq!(t[0], 0);
            assert_eq!(t[255], 0xffffff);
            assert!(t.windows(2).all(|w| (w[0] & 255) <= (w[1] & 255)));
            // The shadows untouched, the middle moved the way asked.
            assert!((0..=51).all(|i| t[i] & 255 == i as u32), "contrast {c}: shadows moved");
            let mid = (t[128] & 255) as i32;
            assert!(if c > 0.0 { mid > 140 } else { mid < 116 }, "contrast {c}: {mid}");
        }
        assert_eq!(Calibration::parse("gain 1 1 1\n").contrast, 0.0);
        assert_eq!(Calibration::parse("contrast 9\n").contrast, CONTRAST_RANGE);
    }

    #[test]
    fn curves_stay_monotonic_and_white_follows_the_gain() {
        let t = Calibration::parse("gain 1 0.9 0.8\ngamma 1.2 1 0.8").table();
        assert_eq!(t[255], 0xffe5cc);
        for ch in [0, 8, 16] {
            assert!(t.windows(2).all(|w| (w[0] >> ch) & 255 <= (w[1] >> ch) & 255));
        }
    }
}
