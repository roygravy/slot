use std::path::Path;

use slot_gfx::{Draw, OUT_H, OUT_W};
use slot_store::{StateEntry, RING_MAX};
use slot_ui::{
    hint_face, hint_width, Polaroids, Printed, DOT, HINT_H, LEGEND, PHOTO_H, PHOTO_W, PLATE_H,
};

fn entry(stamp: &str) -> StateEntry {
    StateEntry {
        stamp: stamp.to_string(),
        state: Path::new("/nonexistent").join(format!("{stamp}.state")),
        thumb: Path::new("/nonexistent").join(format!("{stamp}.png")),
    }
}

fn switcher_with(n: usize) -> Polaroids {
    Polaroids::new(
        (0..n)
            .map(|i| entry(&format!("2026-08-09_{:02}-00-00", 14 - i)))
            .collect(),
    )
}

fn draw(p: &Polaroids) -> Vec<Draw> {
    let mut out = Vec::new();
    p.draw(
        slot_gfx::Canvas::FRAMED,
        None,
        Printed::default(),
        None,
        Printed::default(),
        None,
        &mut out,
    );
    out
}

#[derive(Copy, Clone, Debug)]
struct Quad {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    alpha: f32,
}

fn quads(out: &[Draw]) -> Vec<Quad> {
    out.iter()
        .map(|d| match *d {
            Draw::Rect { x, y, w, h, colour } => Quad {
                x,
                y,
                w,
                h,
                alpha: colour[3],
            },
            Draw::Tex {
                x, y, w, h, alpha, ..
            } => Quad { x, y, w, h, alpha },
            Draw::Part {
                x, y, w, h, alpha, ..
            } => Quad { x, y, w, h, alpha },
            Draw::Turned {
                x, y, w, h, alpha, ..
            }
            | Draw::Light {
                x, y, w, h, alpha, ..
            } => Quad { x, y, w, h, alpha },
            Draw::Tint {
                x, y, w, h, colour, ..
            } => Quad {
                x,
                y,
                w,
                h,
                alpha: colour[3],
            },
            Draw::Game | Draw::Shot { .. } => Quad {
                x: 0.0,
                y: 0.0,
                w: OUT_W as f32,
                h: OUT_H as f32,
                alpha: 1.0,
            },
        })
        .collect()
}

fn dot_rects(out: &[Draw]) -> Vec<Quad> {
    quads(out)
        .into_iter()
        .filter(|q| q.w == DOT && q.h == DOT)
        .collect()
}

fn highlighted_dots(out: &[Draw]) -> Vec<Quad> {
    dot_rects(out)
        .into_iter()
        .filter(|q| q.alpha > 0.9)
        .collect()
}

fn hint_quads(out: &[Draw]) -> Vec<Quad> {
    quads(out)
        .into_iter()
        .filter(|q| q.h == HINT_H as f32)
        .collect()
}

fn has_hint(p: &Polaroids, out: &[Draw], key: &str) -> bool {
    p.hints().iter().any(|h| h.key == key) && hint_quads(out).len() == p.hints().len()
}

#[test]
fn the_screenshot_fills_the_canvas_and_is_letterboxed_by_the_game_pass() {
    let p = switcher_with(3);
    let out = draw(&p);
    let photo = *quads(&out).first().expect("no screenshot drawn");
    assert_eq!((photo.w, photo.h), (OUT_W as f32, OUT_H as f32));
    assert_eq!((photo.x, photo.y), (0.0, 0.0));
    // The shot is drawn by the game pass, letterboxed 3:2 inside the 4:3 canvas.
    let want = OUT_W as f32 * PHOTO_H as f32 / PHOTO_W as f32;
    assert!(
        (slot_gfx::GAME_H - want).abs() <= 1.0,
        "game area is {}",
        slot_gfx::GAME_H
    );
}

#[test]
fn nothing_of_the_paused_game_shows_through() {
    let p = switcher_with(1);
    let out = draw(&p);
    assert_eq!(quads(&out)[0].alpha, 1.0, "the screenshot is see through");
}

#[test]
fn there_is_one_dot_per_entry_and_exactly_one_is_selected() {
    let mut p = switcher_with(5);
    p.right();
    let (count, selected) = p.dots();
    assert_eq!(count, 5);
    assert_eq!(selected, 1);
    let out = draw(&p);
    assert_eq!(dot_rects(&out).len(), 5, "one dot per saved state");
    assert_eq!(
        highlighted_dots(&out).len(),
        1,
        "exactly one dot reads as current"
    );
}

#[test]
fn the_title_follows_the_selection() {
    let mut p = switcher_with(3);
    let first = p.title("2026-08-09_14-36-05");
    p.right();
    let second = p.title("2026-08-09_14-36-05");
    assert_ne!(first, second, "the title did not move with the selection");
}

#[test]
fn the_back_hint_names_its_button() {
    let p = switcher_with(2);
    let out = draw(&p);
    assert!(has_hint(&p, &out, "B"), "no back hint");
}

fn hint_rect(out: &[Draw], key: &str) -> Option<Quad> {
    let i = match key {
        "X" => LEGEND.len(),
        _ => LEGEND.iter().position(|(k, _)| *k == key)?,
    };
    hint_quads(out).get(i).copied()
}

#[test]
fn a_hint_is_tighter_than_it_was() {
    let f = hint_face("A", "Load");
    assert!(f.w < 128, "the hint is still the old fixed width: {}", f.w);
}

#[test]
fn a_hint_is_laid_out_at_the_width_it_rasterises_to() {
    for (key, label) in [("A", "Load"), ("X", "Undo an interminable action")] {
        assert_eq!(hint_width(key, label), hint_face(key, label).w);
    }
}
