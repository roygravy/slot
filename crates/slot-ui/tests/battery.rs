use slot_power::{Battery, Charge};
use slot_ui::{battery_face, draw_gauge, Draw, Printed, TexId, HUD_INK};

fn percent_face() -> Printed {
    Printed { face: None, w: 30 }
}

fn quads(out: &[Draw]) -> Vec<(f32, f32, f32, f32)> {
    out.iter()
        .filter_map(|d| match *d {
            Draw::Rect { x, y, w, h, .. } | Draw::Tex { x, y, w, h, .. } => Some((x, y, w, h)),
            _ => None,
        })
        .collect()
}

fn at(percent: u8, charge: Charge) -> Option<Battery> {
    Some(Battery { percent, charge })
}

#[test]
fn nothing_moves_when_the_charge_state_changes() {
    let mut idle = Vec::new();
    let mut charging = Vec::new();
    draw_gauge(
        24.0,
        400.0,
        at(68, Charge::Discharging),
        percent_face(),
        None,
        HUD_INK,
        &mut idle,
    );
    draw_gauge(
        24.0,
        400.0,
        at(68, Charge::Charging),
        percent_face(),
        Some(TexId::from_raw(7)),
        HUD_INK,
        &mut charging,
    );
    let idle = quads(&idle);
    for q in idle.iter() {
        assert!(
            quads(&charging).contains(q),
            "{q:?} moved or vanished when charging started"
        );
    }
}

#[test]
fn the_face_shows_a_bolt_only_while_charging() {
    let ink = [0xff, 0xff, 0xff];
    assert_ne!(
        battery_face(68, true, ink).rgba,
        battery_face(68, false, ink).rgba,
        "charging looked the same as not charging"
    );
}

#[test]
fn the_fill_tracks_the_percent_in_one_ink() {
    let ink = [0xf6, 0xf4, 0xef];
    // Row 2 runs above the number, so only the fill changes along it.
    let lit = |percent| {
        let f = battery_face(percent, false, ink);
        let row = 2 * f.w as usize * 4;
        (0..f.w as usize)
            .map(|x| u32::from(f.rgba[row + x * 4 + 3]))
            .sum::<u32>()
    };
    let widths: Vec<u32> = [10u8, 50, 90].into_iter().map(lit).collect();
    for pair in widths.windows(2) {
        assert!(
            pair[1] > pair[0],
            "the fill must grow with the percent, got {widths:?}"
        );
    }
    let low = battery_face(5, false, ink);
    for px in low.rgba.chunks(4).filter(|p| p[3] > 0) {
        assert_eq!(&px[..3], &ink[..], "a low battery took on a colour");
    }
}

#[test]
fn no_reading_draws_nothing() {
    let mut out = Vec::new();
    draw_gauge(24.0, 400.0, None, percent_face(), None, HUD_INK, &mut out);
    assert!(out.is_empty());
}
