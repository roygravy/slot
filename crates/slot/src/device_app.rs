use std::path::PathBuf;
use std::time::{Duration, Instant};

use slot::frontend::Frontend;
use slot::input::DeviceInput;
use slot::panel_turn::{panel_turn, PanelTurn};
use slot_gfx::{Compositor, FbdevSurface, Surface};
use slot_power::{trace_first_frame, DevicePlatform};

const CARD: &str = "/mnt/sdcard";

const MIN_FRAME: Duration = Duration::from_millis(12);

const STEP_TIMEOUT: Duration = Duration::from_millis(12);
/// How often the light sleep looks at the buttons and its timers, the screen off.
const DOZE_POLL: Duration = Duration::from_millis(50);

const MARGIN: Duration = Duration::from_micros(1500);

const LOG_SYNC: Duration = Duration::from_secs(1);

struct Pacer {
    last_return: Option<Instant>,
    period: Duration,
    work: Duration,
    works: [Duration; WORK_WINDOW],
    next: usize,
}

const WORK_WINDOW: usize = 32;

impl Pacer {
    fn new() -> Self {
        Pacer {
            last_return: None,
            period: Duration::from_micros(16_760),
            work: Duration::from_millis(4),
            works: [Duration::from_millis(4); WORK_WINDOW],
            next: 0,
        }
    }

    fn wait(&self) {
        let Some(last) = self.last_return else {
            return;
        };
        let delay = self.period.saturating_sub(self.work + MARGIN);
        if let Some(left) = (last + delay).checked_duration_since(Instant::now()) {
            std::thread::sleep(left);
        }
    }

    fn swapped(&mut self, work: Duration, blocked: bool) -> bool {
        let now = Instant::now();
        let mut missed = false;
        if let Some(last) = self.last_return {
            let seen = now - last;
            missed = seen > self.period.mul_f64(1.5);
            if blocked && seen > Duration::from_millis(12) && seen < Duration::from_millis(22) {
                self.period = self.period.mul_f64(0.95) + seen.mul_f64(0.05);
            }
        }
        self.works[self.next] = work;
        self.next = (self.next + 1) % WORK_WINDOW;
        self.work = self.works.iter().copied().max().unwrap_or(work);
        self.last_return = Some(now);
        missed
    }
}

pub fn run() {
    slot::boot_time::mark("start");
    let _ = std::thread::Builder::new()
        .name("slot-fonts".into())
        .spawn(slot_ui::warm_fonts);
    let audio = std::thread::Builder::new()
        .name("slot-audio-open".into())
        .spawn(slot::audio::opened_sink);
    let root = std::env::var_os("SLOT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(CARD));
    let mut surface = match FbdevSurface::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("slot: {e}");
            return;
        }
    };
    slot::boot_time::mark("surface");
    let mut compositor = match Compositor::new(&surface) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("slot: {e}");
            return;
        }
    };
    slot::boot_time::mark("compositor");
    let panel = surface.window_size();
    let env = |key| std::env::var(key).ok();
    let turn = panel_turn(
        panel,
        env("BASEOS_PANEL_ROTATION_CCW").as_deref(),
        env("SLOT_ROTATE").as_deref(),
    );
    let turned = turn != PanelTurn::None;
    let window = if turned { (panel.1, panel.0) } else { panel };
    eprintln!(
        "slot: baseos target {}, model {}, rotation ccw {}",
        env("BASEOS_TARGET").unwrap_or_default(),
        env("BASEOS_MODEL").unwrap_or_default(),
        env("BASEOS_PANEL_ROTATION_CCW").unwrap_or_default()
    );
    eprintln!(
        "slot: panel {}x{}, canvas {}x{}, turn {turn:?}",
        panel.0, panel.1, window.0, window.1
    );
    if turned {
        if let Err(e) = compositor.set_panel_turn(panel, turn == PanelTurn::Ccw) {
            eprintln!("slot: {e}");
            return;
        }
    }
    let platform = DevicePlatform::new(root.clone());
    eprintln!("slot: {}", platform.report());
    eprintln!("slot: {}", bluetooth_report());
    platform.trace_boot();
    slot::boot_time::mark("platform");
    let sink = match audio.map(|h| h.join()) {
        Ok(Ok(sink)) => sink,
        _ => slot::audio::opened_sink(),
    };
    slot::boot_time::mark("audio");
    let mut frontend = Frontend::boot_with(Box::new(platform), sink);
    slot::boot_time::mark("frontend");
    frontend.upload_boot_faces(&mut compositor);
    slot::boot_time::mark("faces");
    frontend.upload_bezel(&mut compositor, window);
    slot::boot_time::mark("bezel");
    let mut input = DeviceInput::open(&root);
    slot::boot_time::mark("input");
    let card = root.clone();
    let _ = std::thread::Builder::new()
        .name("slot-bootlogo".into())
        .spawn(move || {
            slot::bootlogo::refresh(&card);
            slot::root::sweep_temps(&card);
        });
    let _ = std::thread::Builder::new()
        .name("slot-logsync".into())
        .spawn(sync_log_writes);
    let mut drawn = false;
    let mut cpu = CpuClock::new();
    frontend.drive_emulator();
    let mut pacer = Pacer::new();
    let (mut frames, mut missed, mut early) = (0u32, 0u32, 0u32);
    loop {
        pacer.wait();
        let began = Instant::now();
        frontend.advance(&mut input);
        if frontend.restarting() {
            frontend.restart();
        }
        if frontend.powering_off() {
            frontend.poweroff();
            return;
        }
        if slot::crash::stopping() {
            eprintln!("slot: told to stop, saving first");
            frontend.stop();
            return;
        }
        // The light sleep draws nothing: the screen is dark, and the downloads go on.
        if frontend.dozing() {
            cpu.game(false);
            std::thread::sleep(DOZE_POLL);
            continue;
        }
        cpu.game(frontend.game_open());
        frontend.step_emulator(pacer.period, STEP_TIMEOUT);
        frontend.render(&mut compositor, window);
        let swap = Instant::now();
        let work = swap - began;
        if let Err(e) = surface.swap() {
            eprintln!("slot: {e}");
            frontend.stop();
            return;
        }
        let swap_took = swap.elapsed();
        slot::latency::swapped(swap_took.as_secs_f64() * 1000.0);
        let dropped = pacer.swapped(work, swap_took > Duration::from_millis(1));
        if slot::latency::tracing() {
            frames += 1;
            missed += u32::from(dropped);
            early += u32::from(swap_took > pacer.period / 2);
            if frames == 600 {
                eprintln!(
                    "slot: pace: 600 frames, {missed} missed a latch, {early} drawn early, work {:.1} ms, period {:.2} ms",
                    pacer.work.as_secs_f64() * 1e3,
                    pacer.period.as_secs_f64() * 1e3
                );
                (frames, missed, early) = (0, 0, 0);
            }
        }
        if !drawn {
            drawn = true;
            slot::boot_time::mark("first frame");
            trace_first_frame();
        }
        if let Some(left) = MIN_FRAME.checked_sub(began.elapsed()) {
            std::thread::sleep(left);
        }
    }
}

/// The CPU at full speed while a game is open: the stock governor clocks it down between
/// frames, and a GBA frame in mGBA then took 12-15 ms of the 16.7, too slow with the screen
/// effects on top (Roy). Back to the governor it had once the game closes or the screen sleeps.
struct CpuClock {
    dir: Option<PathBuf>,
    stock: String,
    fast: bool,
}

impl CpuClock {
    fn new() -> Self {
        let dir = [
            "/sys/devices/system/cpu/cpufreq/policy0",
            "/sys/devices/system/cpu/cpu0/cpufreq",
        ]
        .iter()
        .map(PathBuf::from)
        .find(|d| d.join("scaling_governor").is_file());
        let read = |name: &str| {
            dir.as_ref()
                .and_then(|d| std::fs::read_to_string(d.join(name)).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_default()
        };
        let stock = read("scaling_governor");
        eprintln!(
            "slot: cpu: governor {stock} of [{}], {}-{} kHz",
            read("scaling_available_governors"),
            read("scaling_min_freq"),
            read("scaling_max_freq"),
        );
        CpuClock {
            dir,
            stock,
            fast: false,
        }
    }

    fn game(&mut self, open: bool) {
        if open == self.fast {
            return;
        }
        self.fast = open;
        let Some(dir) = &self.dir else { return };
        if self.stock.is_empty() || self.stock == "performance" {
            return;
        }
        let to = if open { "performance" } else { self.stock.as_str() };
        match std::fs::write(dir.join("scaling_governor"), to) {
            Ok(()) => eprintln!("slot: cpu: {to}"),
            Err(e) => eprintln!("slot: cpu: {to}: {e}"),
        }
    }
}

impl Drop for CpuClock {
    fn drop(&mut self) {
        self.game(false);
    }
}

fn sync_log_writes() {
    let mut synced = 0;
    loop {
        std::thread::sleep(LOG_SYNC);
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstat(2, &mut st) } != 0 {
            return;
        }
        if st.st_size != synced {
            #[cfg(target_os = "linux")]
            unsafe {
                libc::syncfs(2);
            }
            synced = st.st_size;
        }
    }
}

/// One line in slot.log on what this device offers for Bluetooth, so
/// Bluetooth audio can be planned from real devices. Only reads.
fn bluetooth_report() -> String {
    let names = |dir: &str| -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .map(|d| {
                d.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    };
    let rfkill: Vec<String> = names("/sys/class/rfkill")
        .into_iter()
        .filter_map(|r| {
            let t = std::fs::read_to_string(format!("/sys/class/rfkill/{r}/type")).ok()?;
            let soft =
                std::fs::read_to_string(format!("/sys/class/rfkill/{r}/soft")).unwrap_or_default();
            Some(format!(
                "{}{}",
                t.trim(),
                if soft.trim() == "1" { "(off)" } else { "" }
            ))
        })
        .collect();
    let found = |paths: &[&str]| paths.iter().any(|p| std::path::Path::new(p).exists());
    let tools = [
        (
            "bluetoothd",
            found(&[
                "/usr/libexec/bluetooth/bluetoothd",
                "/usr/lib/bluetooth/bluetoothd",
                "/usr/sbin/bluetoothd",
                "/usr/bin/bluetoothd",
            ]),
        ),
        ("bluetoothctl", found(&["/usr/bin/bluetoothctl"])),
        (
            "bluealsa",
            found(&["/usr/bin/bluealsa", "/usr/bin/bluealsad"]),
        ),
        (
            "rtk_hciattach",
            found(&["/usr/bin/rtk_hciattach", "/usr/sbin/rtk_hciattach"]),
        ),
    ];
    let tools: Vec<&str> = tools.iter().filter(|t| t.1).map(|t| t.0).collect();
    let fw: Vec<String> = names("/lib/firmware/rtlbt")
        .into_iter()
        .chain(names("/lib/firmware/rtl_bt"))
        .collect();
    format!(
        "bluetooth hci [{}], rfkill [{}], tools [{}], firmware [{}]",
        names("/sys/class/bluetooth").join(" "),
        rfkill.join(" "),
        tools.join(" "),
        fw.join(" ")
    )
}
