mod common;

use common::{boot, tmp_root_with_gbc_carts};
use slot::app::Phase;

#[test]
fn a_card_without_gb_games_starts_on_a_shelf_with_carts() {
    let d = tmp_root_with_gbc_carts(&["Mud Warriors", "Hauntsfield"]);
    let a = boot(d.path());
    assert!(a.selected_stem().is_some(), "booted onto an empty shelf");
}

#[test]
fn a_single_gbc_cart_goes_straight_in() {
    let d = tmp_root_with_gbc_carts(&["Mud Warriors"]);
    let a = boot(d.path());
    assert!(
        matches!(a.phase(), Phase::Inserting { .. } | Phase::Playing { .. }),
        "{:?}",
        a.phase()
    );
}
