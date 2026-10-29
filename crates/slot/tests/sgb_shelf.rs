mod common;

use common::{boot, tmp_root_with_gb_carts};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};

/// R1 along the shelves until the SNES's, where the Super Game Boy stands, and onto it.
fn to_the_sgb(a: &mut App) {
    for _ in 0..8 {
        if a.shelf_platform_name() == Some("Super Nintendo") {
            break;
        }
        a.apply(Action::GbaDown(Btn::R1));
    }
    assert_eq!(a.shelf_platform_name(), Some("Super Nintendo"));
    assert_eq!(a.selected_stem(), Some(slot_store::SGB_STEM));
}

/// The Super Game Boy in its slot and the camera on the Game Boy shelf it plays.
fn sgb_in(a: &mut App) {
    to_the_sgb(a);
    a.apply(Action::Insert);
    let mut now = 0;
    while now < 10_000
        && !(matches!(a.phase(), Phase::Shelf) && a.shelf_platform_name() == Some("Super Game Boy"))
    {
        now += 16;
        a.update(0.016);
    }
    assert_eq!(
        a.shelf_platform_name(),
        Some("Super Game Boy"),
        "{:?}",
        a.phase()
    );
}

#[test]
fn a_lone_game_boy_game_still_plays_at_once_beside_the_super_game_boy() {
    let d = tmp_root_with_gb_carts(&["Tetris"]);
    let a = boot(d.path());
    assert!(
        a.single_cart(),
        "the Super Game Boy counted as a second game"
    );
    assert!(matches!(a.phase(), Phase::Inserting { .. }));
}

#[test]
fn menu_held_on_the_super_game_boy_shelf_takes_it_out() {
    let d = tmp_root_with_gb_carts(&["Tetris", "Zelda"]);
    let mut a = boot(d.path());
    sgb_in(&mut a);
    a.apply(Action::Eject);
    assert!(
        !matches!(a.phase(), Phase::Buttons { .. }),
        "MENU held opened the buttons' pages over the Super Game Boy"
    );
    assert!(
        matches!(a.phase(), Phase::Ejecting { cart, .. } if cart == slot_store::SGB_STEM),
        "{:?}",
        a.phase()
    );
}
