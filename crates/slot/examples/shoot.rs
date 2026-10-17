//! Drive slot headless from a script and write stills: `shoot ROOT SCRIPT OUTDIR`.
//! Script words: `wait N`, `tap BTN [frames]`, `down BTN`, `up BTN`, `snap NAME`.

use std::time::Duration;

use slot::frontend::Frontend;
use slot_gfx::{Compositor, HeadlessSurface, OUT_H, OUT_W};
use slot_input::{Btn, InputSource, Millis, RawEvent};
use slot_power::SimPlatform;

const FPS: f64 = 60.0;
const CLOCK: i64 = 1_790_784_000;

fn btn(name: &str) -> Btn {
    match name {
        "up" => Btn::Up,
        "down" => Btn::Down,
        "left" => Btn::Left,
        "right" => Btn::Right,
        "a" => Btn::A,
        "b" => Btn::B,
        "x" => Btn::X,
        "y" => Btn::Y,
        "l1" => Btn::L1,
        "r1" => Btn::R1,
        "l2" => Btn::L2,
        "r2" => Btn::R2,
        "start" => Btn::Start,
        "select" => Btn::Select,
        "menu" => Btn::Menu,
        other => panic!("unknown button {other}"),
    }
}

#[derive(Default)]
struct Scripted(Vec<RawEvent>);

impl InputSource for Scripted {
    fn poll(&mut self, _now: Millis) -> Vec<RawEvent> {
        std::mem::take(&mut self.0)
    }
}

fn write_png(path: &str, rgba: &[u8]) {
    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), OUT_W, OUT_H);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().unwrap();
    let mut px = rgba.to_vec();
    for p in px.chunks_exact_mut(4) {
        p[3] = 255;
    }
    w.write_image_data(&px).unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [root, script, out] = args.as_slice() else {
        eprintln!("usage: shoot ROOT SCRIPT OUTDIR");
        std::process::exit(2);
    };
    std::fs::create_dir_all(out).unwrap();
    let script = std::fs::read_to_string(script).expect("read the script");

    let surface = HeadlessSurface::new().expect("headless GL");
    let mut compositor = Compositor::new(&surface).expect("compositor");
    let mut frontend = Frontend::boot(Box::new(SimPlatform::at(root.into()).stopped_at(CLOCK)));
    frontend.upload_faces(&mut compositor);
    frontend.drive_emulator();

    let mut input = Scripted::default();
    let mut frame = 0u64;
    let present = Duration::from_secs_f64(1.0 / FPS);
    let mut run = |input: &mut Scripted, frame: &mut u64| -> Vec<u8> {
        let loading = std::time::Instant::now();
        while frontend.core_settling() && loading.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(5));
        }
        *frame += 1;
        let now = (*frame as f64 * 1000.0 / FPS) as Millis;
        frontend.advance_at(input, now, (1.0 / FPS) as f32);
        frontend.step_emulator(present, Duration::from_secs(1));
        frontend.compose(&mut compositor);
        compositor.read_frame()
    };
    let mut last;
    for line in script.lines() {
        let words: Vec<&str> = line.split('#').next().unwrap().split_whitespace().collect();
        let n = |i: usize, d: u32| words.get(i).map_or(d, |w| w.parse().unwrap());
        match words.as_slice() {
            [] => {}
            ["wait", ..] => {
                for _ in 0..n(1, 1) {
                    last = run(&mut input, &mut frame);
                }
            }
            ["tap", b, ..] => {
                input.0.push(RawEvent::Down(btn(b)));
                for _ in 0..n(2, 6) {
                    last = run(&mut input, &mut frame);
                }
                input.0.push(RawEvent::Up(btn(b)));
                for _ in 0..12 {
                    last = run(&mut input, &mut frame);
                }
            }
            ["down", b] => input.0.push(RawEvent::Down(btn(b))),
            ["up", b] => input.0.push(RawEvent::Up(btn(b))),
            ["snap", name] => {
                last = run(&mut input, &mut frame);
                write_png(&format!("{out}/{name}.png"), &last);
                eprintln!("snap {name}");
            }
            _ => panic!("cannot read {line:?}"),
        }
    }
}
