//! Blargg's NTSC filter, as Snes9x runs it for its `snes9x_blargg` option, for a picture slot.
//! filters itself: the Super Game Boy's, which mGBA draws, goes to the TV down the same cable
//! as an SNES game's.

use std::ffi::{c_int, c_long, c_void};

#[repr(C)]
struct Raw {
    _private: [u8; 0],
}

extern "C" {
    fn slot_ntsc_new(cable: c_int) -> *mut Raw;
    fn slot_ntsc_free(ntsc: *mut Raw);
    fn slot_ntsc_blit(
        ntsc: *const Raw,
        input: *const u16,
        in_row_width: c_long,
        burst_phase: c_int,
        in_width: c_int,
        in_height: c_int,
        out: *mut c_void,
        out_pitch: c_long,
    );
}

/// The cables the filter draws, as the frontend's option names them.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Cable {
    Rf,
    Composite,
    SVideo,
    Rgb,
}

impl Cable {
    /// The value of the frontend's own `slot_ntsc` option; None for "off" (or anything else).
    pub fn parse(value: &str) -> Option<Cable> {
        match value {
            "rf" => Some(Cable::Rf),
            "composite" => Some(Cable::Composite),
            "svideo" | "s-video" => Some(Cable::SVideo),
            "rgb" => Some(Cable::Rgb),
            _ => None,
        }
    }
}

pub struct Ntsc {
    raw: *mut Raw,
    pub cable: Cable,
    burst: c_int,
    input: Vec<u16>,
    pub output: Vec<u8>,
}

// The filter's tables are read only once made; the handle moves with its core's thread.
unsafe impl Send for Ntsc {}

/// How wide the filter draws `width` columns: 602 for the SNES's 256.
pub fn out_width(width: usize) -> usize {
    (width.saturating_sub(1) / 3 + 1) * 7
}

impl Ntsc {
    pub fn new(cable: Cable) -> Option<Ntsc> {
        let code = match cable {
            Cable::Rf => 1,
            Cable::Composite => 2,
            Cable::SVideo => 3,
            Cable::Rgb => 4,
        };
        let raw = unsafe { slot_ntsc_new(code) };
        (!raw.is_null()).then(|| Ntsc {
            raw,
            cable,
            burst: 0,
            input: Vec::new(),
            output: Vec::new(),
        })
    }

    /// Filter one frame of 565 pixels into `output`, XRGB8888 rows `out_width(width)` wide.
    /// `row` writes a line of the source into the 565 line it is handed.
    pub fn run(&mut self, width: usize, height: usize, mut row: impl FnMut(usize, &mut [u16])) {
        self.input.resize(width * height, 0);
        for (y, line) in self.input.chunks_exact_mut(width).enumerate() {
            row(y, line);
        }
        let ow = out_width(width);
        self.output.resize(ow * height * 4, 0);
        // As Snes9x: the colour burst steps a third of a cycle each frame.
        self.burst = (self.burst + 1) % 3;
        unsafe {
            slot_ntsc_blit(
                self.raw,
                self.input.as_ptr(),
                width as c_long,
                self.burst,
                width as c_int,
                height as c_int,
                self.output.as_mut_ptr() as *mut c_void,
                (ow * 4) as c_long,
            );
        }
    }
}

impl Drop for Ntsc {
    fn drop(&mut self) {
        unsafe { slot_ntsc_free(self.raw) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_256_columns_602_wide() {
        assert_eq!(out_width(256), 602);
        let mut ntsc = Ntsc::new(Cable::Composite).unwrap();
        ntsc.run(256, 4, |_, line| line.fill(0xffff));
        assert_eq!(ntsc.output.len(), 602 * 4 * 4);
        // White stays near white through the middle of the line.
        let mid = &ntsc.output[(602 * 2 + 300) * 4..(602 * 2 + 301) * 4];
        assert!(mid[..3].iter().all(|&c| c > 200), "{mid:?}");
    }
}
