//! Writes slot's own text faces (white on transparent) as PNGs, for design mockups:
//! `mockfaces OUTDIR` reads lines from stdin: `label NAME TEXT`, `help NAME TEXT`,
//! `hint NAME KEY LABEL`, `arrows NAME LABEL`, plus the switch and caret faces.
use std::io::BufRead;

use slot_ui::{hint_face, quick_caret_face, quick_switch_face, quick_value_face, text};

fn save(dir: &str, name: &str, rgba: &[u8], w: u32, h: u32) {
    let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(rgba).unwrap();
}

fn main() {
    let dir = std::env::args().nth(1).expect("outdir");
    for (name, f) in [
        ("switch_on", quick_switch_face(true)),
        ("switch_off", quick_switch_face(false)),
        ("caret_l", quick_caret_face(false)),
        ("caret_r", quick_caret_face(true)),
    ] {
        save(&dir, name, &f.rgba, f.w, f.h);
    }
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        match parts.as_slice() {
            ["label", name, t] => {
                let f = quick_value_face(t, false);
                save(&dir, name, &f.rgba, f.w, f.h);
            }
            ["help", name, t] => {
                let font = text::label_font().unwrap();
                let layout = text::fit(font, t, 600.0, 1, 17.0, 12.0);
                let set = layout
                    .lines
                    .iter()
                    .map(|l| text::line_width(font, l, layout.px, layout.tracking))
                    .fold(0.0, f32::max);
                let (w, h) = (set.ceil() as u32 + 4, 24);
                let mut rgba = vec![0u8; (w * h * 4) as usize];
                text::draw_centred(&mut rgba, w, h, &layout, [0xff, 0xff, 0xff]);
                save(&dir, name, &rgba, w, h);
            }
            ["hint", name, key, label] => {
                let f = hint_face(key, label);
                save(&dir, name, &f.rgba, f.w, f.h);
            }
            ["arrows", name, label] => {
                let f = slot_ui::arrows_hint_face(label);
                save(&dir, name, &f.rgba, f.w, f.h);
            }
            _ => eprintln!("skip {line}"),
        }
    }
}
