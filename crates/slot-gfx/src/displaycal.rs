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
}

impl Default for Calibration {
    fn default() -> Self {
        Calibration {
            gain: [1.0; 3],
            gamma: [1.0; 3],
            saturation: saturation_default(),
        }
    }
}

pub const GAIN_MIN: f32 = 0.5;
pub const GAMMA_RANGE: (f32, f32) = (0.5, 2.0);
/// Without a calibration of its own the RG35XX Plus shows a little more colour than drawn: its
/// panel looks flat at 100% (Roy 2026-11-13). Other consoles' panels are livelier (an RG SP
/// tester, 2026-11-16): they show the colours as drawn.
pub const SATURATION_RG35XX_PLUS: f32 = 1.4;

/// The standard saturation of this console's panel: the RG35XX Plus's, also off the device
/// (the simulator, the tests) and when BaseOS does not say which console it is.
pub fn saturation_default() -> f32 {
    static DEFAULT: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *DEFAULT.get_or_init(|| {
        let target = std::env::var("BASEOS_TARGET").ok().or_else(|| {
            let release = std::fs::read_to_string("/etc/baseos-release").ok()?;
            release.lines().find_map(|l| {
                l.strip_prefix("BASEOS_TARGET=").map(|v| v.trim().trim_matches('"').to_string())
            })
        });
        saturation_for(target.as_deref())
    })
}

fn saturation_for(target: Option<&str>) -> f32 {
    match target.map(str::trim) {
        None | Some("") | Some("rg35xxplus") => SATURATION_RG35XX_PLUS,
        Some(_) => 1.0,
    }
}

/// The saturation a card may keep: 60% to 180% of the standard.
pub fn saturation_range() -> (f32, f32) {
    let d = saturation_default();
    (d * 0.6, d * 1.8)
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
            "gain {r:.4} {g:.4} {b:.4}\ngamma {er:.4} {eg:.4} {eb:.4}\nsaturation {:.4}\n",
            self.saturation
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
                let v = f64::from(self.gain[ch]) * x.powf(f64::from(self.gamma[ch]));
                (v * 255.0).round().clamp(0.0, 255.0) as u32
            };
            (c(0) << 16) | (c(1) << 8) | c(2)
        })
    }
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
    fn only_the_rg35xx_plus_gets_the_extra_colour() {
        assert_eq!(saturation_for(Some("rg35xxplus")), SATURATION_RG35XX_PLUS);
        assert_eq!(saturation_for(None), SATURATION_RG35XX_PLUS);
        assert_eq!(saturation_for(Some("rg35xxsp")), 1.0);
        assert_eq!(saturation_for(Some("rg34xxsp")), 1.0);
    }

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
        };
        assert_eq!(Calibration::parse(&cal.text()), cal);
        let wild = Calibration::parse("gain 2 0.1 nan\ngamma 9 0.1 1\nsaturation 9\n");
        assert_eq!(wild.saturation, 2.52);
        assert_eq!(wild.gain, [1.0; 3]);
        assert_eq!(wild.gamma, [2.0, 0.5, 1.0]);
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
