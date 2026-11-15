use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use libloading::Library;

use super::pacer::Pacer;
use super::ring::Ring;
use super::silence::Silence;
use super::sink::{AudioError, AudioSink};

const SND_PCM_STREAM_PLAYBACK: c_int = 0;
const SND_PCM_FORMAT_S16_LE: c_int = 2;
const SND_PCM_ACCESS_RW_INTERLEAVED: c_int = 3;
const CHANNELS: c_uint = 2;

const LATENCY_US: c_uint = 40_000;

const PERIOD_FRAMES: usize = 512;

const RELEASE_AFTER: Duration = Duration::from_secs(3);

const DEVICES: [&str; 4] = ["plug:default", "default", "plughw:0,0", "hw:0,0"];

/// Bluetooth headphones add their own delay on top; this is what is held here on top of it.
const BT_LATENCY_US: c_uint = 80_000;

/// How often the headphones the service has connected are looked at, in periods (~1 s).
const ROUTE_EVERY: u32 = 90;

/// The Bluetooth headphones slot-services has connected, from its run directory.
pub fn bt_route() -> Option<String> {
    let run = std::env::var_os("SLOT_SERVICES_RUN")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/run/slot-services".into());
    let text = std::fs::read_to_string(run.join("bt-audio")).ok()?;
    let mac = text.lines().next()?.trim();
    (mac.len() == 17 && mac.chars().all(|c| c.is_ascii_hexdigit() || c == ':'))
        .then(|| mac.to_owned())
}

type PcmOpen = unsafe extern "C" fn(*mut *mut c_void, *const c_char, c_int, c_int) -> c_int;
type PcmSetParams =
    unsafe extern "C" fn(*mut c_void, c_int, c_int, c_uint, c_uint, c_int, c_uint) -> c_int;
type PcmWritei = unsafe extern "C" fn(*mut c_void, *const c_void, u64) -> i64;
type PcmRecover = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;
type PcmDrop = unsafe extern "C" fn(*mut c_void) -> c_int;
type PcmClose = unsafe extern "C" fn(*mut c_void) -> c_int;
type StrError = unsafe extern "C" fn(c_int) -> *const c_char;

struct Alsa {
    open: PcmOpen,
    set_params: PcmSetParams,
    writei: PcmWritei,
    recover: PcmRecover,
    drop: PcmDrop,
    close: PcmClose,
    strerror: StrError,
    _lib: Library,
}

impl Alsa {
    fn load() -> Result<Self, AudioError> {
        let lib = unsafe { Library::new("libasound.so.2") }
            .or_else(|_| unsafe { Library::new("libasound.so") })
            .map_err(|e| AudioError::Device(format!("libasound: {e}")))?;
        macro_rules! get {
            ($name:literal) => {
                *lib.get(concat!($name, "\0").as_bytes())
                    .map_err(|e| AudioError::Device(format!("{}: {e}", $name)))?
            };
        }
        unsafe {
            Ok(Alsa {
                open: get!("snd_pcm_open"),
                set_params: get!("snd_pcm_set_params"),
                writei: get!("snd_pcm_writei"),
                recover: get!("snd_pcm_recover"),
                drop: get!("snd_pcm_drop"),
                close: get!("snd_pcm_close"),
                strerror: get!("snd_strerror"),
                _lib: lib,
            })
        }
    }

    fn message(&self, err: c_int) -> String {
        let text = unsafe { (self.strerror)(err) };
        match text.is_null() {
            true => format!("error {err}"),
            false => unsafe { CStr::from_ptr(text) }
                .to_string_lossy()
                .into_owned(),
        }
    }

    fn open_pcm(&self, rate: u32, bt: Option<&str>) -> Result<*mut c_void, AudioError> {
        let mut last = AudioError::NoDevice;
        let headphones = bt.map(|mac| format!("bluealsa:DEV={mac},PROFILE=a2dp"));
        for name in headphones.iter().map(String::as_str).chain(DEVICES) {
            let latency = if headphones.as_deref() == Some(name) {
                BT_LATENCY_US
            } else {
                LATENCY_US
            };
            let Ok(cname) = CString::new(name) else {
                continue;
            };
            let mut pcm: *mut c_void = std::ptr::null_mut();
            let err = unsafe { (self.open)(&mut pcm, cname.as_ptr(), SND_PCM_STREAM_PLAYBACK, 0) };
            if err < 0 || pcm.is_null() {
                last = AudioError::Device(format!("{name}: {}", self.message(err)));
                continue;
            }
            let err = unsafe {
                (self.set_params)(
                    pcm,
                    SND_PCM_FORMAT_S16_LE,
                    SND_PCM_ACCESS_RW_INTERLEAVED,
                    CHANNELS,
                    rate,
                    1,
                    latency,
                )
            };
            if err < 0 {
                unsafe { (self.close)(pcm) };
                last = AudioError::Config(format!("{name}: {}", self.message(err)));
                continue;
            }
            eprintln!("slot: audio {name} at {rate} Hz");
            return Ok(pcm);
        }
        Err(last)
    }
}

pub struct AlsaSink {
    ring: Arc<Ring>,
    device: Option<Device>,
}

struct Device {
    stop: Arc<AtomicBool>,
    join: JoinHandle<()>,
}

impl AlsaSink {
    pub fn new() -> Self {
        AlsaSink {
            ring: Arc::new(Ring::new(0)),
            device: None,
        }
    }
}

impl Default for AlsaSink {
    fn default() -> Self {
        AlsaSink::new()
    }
}

impl Drop for AlsaSink {
    fn drop(&mut self) {
        self.close();
    }
}

impl AudioSink for AlsaSink {
    fn open(&mut self, sample_rate: u32) -> Result<(), AudioError> {
        self.close();
        let ring = self.ring.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let (ready_tx, ready_rx) = mpsc::channel();
        let join = std::thread::Builder::new()
            .name("slot-audio".into())
            .spawn(move || match play(&ring, sample_rate) {
                Ok(mut device) => {
                    let _ = ready_tx.send(Ok(()));
                    device.run(&ring, &flag);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| AudioError::Device(e.to_string()))?;
        match ready_rx.recv() {
            Ok(Ok(())) => {
                self.device = Some(Device { stop, join });
                Ok(())
            }
            Ok(Err(e)) => {
                let _ = join.join();
                Err(e)
            }
            Err(_) => {
                let _ = join.join();
                Err(AudioError::Device("output thread stopped".into()))
            }
        }
    }

    fn ring(&self) -> Arc<Ring> {
        self.ring.clone()
    }

    fn close(&mut self) {
        if let Some(d) = self.device.take() {
            d.stop.store(true, Ordering::Relaxed);
            let _ = d.join.join();
        }
    }
}

struct Playback {
    alsa: Alsa,
    pcm: Option<*mut c_void>,
    rate: u32,
    failing: bool,
    /// The headphones the open device plays on; None is the console's own output.
    route: Option<String>,
    route_check: u32,
}

fn play(ring: &Arc<Ring>, sample_rate: u32) -> Result<Playback, AudioError> {
    let alsa = Alsa::load()?;
    let route = bt_route();
    let pcm = match alsa.open_pcm(sample_rate, route.as_deref()) {
        Ok(pcm) => Some(pcm),
        Err(e) => {
            eprintln!("slot: audio: {e}, retrying");
            None
        }
    };
    ring.reopen(sample_rate);
    Ok(Playback {
        alsa,
        pcm,
        rate: sample_rate,
        failing: pcm.is_none(),
        route,
        route_check: 0,
    })
}

impl Playback {
    fn run(&mut self, ring: &Ring, stop: &AtomicBool) {
        let mut buf = vec![0i16; PERIOD_FRAMES * CHANNELS as usize];
        let span = Duration::from_secs_f64(PERIOD_FRAMES as f64 / self.rate as f64);
        let mut silence = Silence::new(RELEASE_AFTER);
        let mut pace = Pacer::new(span);
        while !stop.load(Ordering::Relaxed) {
            ring.fill(&mut buf);
            self.route_check += 1;
            if self.route_check >= ROUTE_EVERY {
                self.route_check = 0;
                let route = bt_route();
                if route != self.route {
                    // Headphones came or went: let go, the next open picks the new way out.
                    eprintln!("slot: audio route {:?}", route);
                    self.release();
                    self.route = route;
                }
            }
            let hold = silence.hear(&buf, span) || ring.playing();
            match (self.pcm, hold) {
                // Headphones stay open: opening them again takes a second and cuts the
                // first sound off.
                (Some(_), false) if self.route.is_none() => {
                    self.release();
                    eprintln!("slot: audio released after silence");
                }
                (None, true) => match self.alsa.open_pcm(self.rate, self.route.as_deref()) {
                    Ok(pcm) => {
                        eprintln!("slot: audio reopened at {:.1} ms", crate::boot_time::ms());
                        self.pcm = Some(pcm);
                        self.failing = false;
                    }
                    Err(e) => {
                        if !self.failing {
                            eprintln!("slot: audio: {e}, retrying");
                            self.failing = true;
                        }
                    }
                },
                _ => {}
            }
            match self.pcm {
                Some(pcm) => {
                    pace.reset();
                    if !self.write(pcm, &buf) {
                        if self.route.is_none() {
                            return;
                        }
                        // The headphones dropped away mid-write: back to the speaker.
                        self.release();
                        self.route = None;
                    }
                }
                None => std::thread::sleep(pace.wait(Instant::now())),
            }
        }
    }

    fn write(&self, pcm: *mut c_void, buf: &[i16]) -> bool {
        let mut written = 0;
        while written < PERIOD_FRAMES {
            let at = written * CHANNELS as usize;
            let frames = unsafe {
                (self.alsa.writei)(
                    pcm,
                    buf[at..].as_ptr() as *const c_void,
                    (PERIOD_FRAMES - written) as u64,
                )
            };
            if frames < 0 {
                let err = unsafe { (self.alsa.recover)(pcm, frames as c_int, 1) };
                if err < 0 {
                    eprintln!("slot: audio: {}", self.alsa.message(err));
                    return false;
                }
                continue;
            }
            written += frames as usize;
        }
        true
    }

    fn release(&mut self) {
        if let Some(pcm) = self.pcm.take() {
            unsafe {
                (self.alsa.drop)(pcm);
                (self.alsa.close)(pcm);
            }
        }
    }
}

impl Drop for Playback {
    fn drop(&mut self) {
        self.release();
    }
}
