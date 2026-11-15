mod common;

use common::{app_playing_in, boot, tmp_root_with_carts};
use slot::app::{App, Phase};
use slot_input::Action;
use slot_ui::{Draw, CART_H, CART_W};

#[test]
fn double_tap_menu_with_no_states_shakes_instead_of_doing_nothing() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::Polaroids);
    assert!(
        matches!(a.phase(), Phase::Playing { .. }),
        "it must not open"
    );
    assert!(a.refusal_active(a.now()), "nothing told the player why");
}

#[test]
fn loading_with_no_states_shakes_as_well() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::LoadState);
    assert!(a.refusal_active(a.now()), "nothing told the player why");
}

#[test]
fn no_plate_is_drawn_for_a_refusal() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::Polaroids);
    let mut out = Vec::new();
    a.draw(&mut out);
    assert!(a.refusal_active(a.now()), "nothing was refused");
    assert_eq!(
        out.len(),
        draws_without_refusal(&d),
        "a refusal added something to the screen"
    );
}

#[test]
fn the_screen_shake_decays_and_stops() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::Polaroids);
    let early = peak(&a, 0, 100);
    let late = peak(&a, 200, 300);
    assert!(early > 0.0, "nothing moved");
    assert!(
        late < early,
        "the shake is not decaying: {early} then {late}"
    );
    assert_eq!(a.shake_at(a.now() + 400), 0.0, "the shake never ends");
}

#[test]
fn an_empty_ring_shakes_the_screen_and_not_a_cart() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::Polaroids);
    assert!(
        matches!(a.phase(), Phase::Playing { .. }),
        "the switcher opened with nothing in it"
    );
    assert_ne!(a.screen_shake(), 0.0, "the screen did not flinch");
}

#[test]
fn a_refused_cart_shows_an_alert_and_does_not_jitter() {
    let d = tmp_root_with_carts(&["Broken", "Fusion", "Zzz"]);
    let mut a = boot(d.path());
    a.apply(Action::Insert);
    for _ in 0..30 {
        a.update(1.0 / 60.0);
    }
    let straight = cart_x(&a);
    a.on_core_failed();
    a.update(1.0 / 60.0);
    assert_eq!(cart_x(&a), straight, "the cart is still jittering");
    assert!(
        a.alert_visible(),
        "nothing tells the player the cart was refused"
    );
    assert_eq!(
        a.screen_shake(),
        0.0,
        "the screen shook as well as the cart"
    );
}

#[test]
fn the_message_outlasts_the_cart() {
    let d = tmp_root_with_carts(&["Broken"]);
    let mut a = boot(d.path());
    a.on_core_failed();
    assert!(a.alert_visible(), "the message never appeared");
    while matches!(a.phase(), Phase::Ejecting { .. }) {
        a.update(1.0 / 60.0);
    }
    assert!(a.alert_visible(), "the message was gone before it could be read");
}

#[test]
fn a_shelf_refusal_shakes_the_carts_not_the_screen() {
    let d = tmp_root_with_carts(&["Emerald", "Zzz"]);
    let mut a = boot(d.path());
    a.refuse();
    assert_eq!(a.screen_shake(), 0.0, "the shelf shook the whole screen");
    assert_ne!(a.shelf_shake(), 0.0, "nothing moved at all");
}

fn peak(a: &App, from: u64, to: u64) -> f32 {
    (from..to)
        .map(|t| a.shake_at(a.now() + t).abs())
        .fold(0.0, f32::max)
}

fn draws_without_refusal(d: &tempfile::TempDir) -> usize {
    let a = app_playing_in(d.path(), "Emerald");
    let mut out = Vec::new();
    a.draw(&mut out);
    out.len()
}

fn drawn(a: &App) -> Vec<Draw> {
    let mut out = Vec::new();
    a.draw(&mut out);
    out
}

fn cart_x(a: &App) -> f32 {
    let shaped = |w: f32, h: f32| w > 200.0 && (w / h - CART_W as f32 / CART_H as f32).abs() < 0.05;
    drawn(a)
        .iter()
        .find_map(|d| match *d {
            Draw::Rect { x, w, h, .. } | Draw::Tex { x, w, h, .. } if shaped(w, h) => {
                Some(x + w / 2.0)
            }
            _ => None,
        })
        .expect("no cart in the list")
}
