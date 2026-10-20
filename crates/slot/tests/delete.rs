mod common;

use common::{app_in_switcher, tmp_root_with_carts};
use slot::app::Phase;
use slot_input::{Action, Btn};
use slot_store::{Core, Platform, StateRing};

#[test]
fn y_deletes_the_selected_state_and_nothing_else() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    for i in 0..3 {
        r.push(&[i as u8; 64], b"png", &format!("2026-08-09_00-00-{i:02}"))
            .unwrap();
    }
    let mut a = app_in_switcher(d.path(), "Emerald");
    a.apply(Action::ShelfRight);
    a.apply(Action::GbaDown(Btn::Y));
    let left = r.list().unwrap();
    assert_eq!(left.len(), 2, "delete removed the wrong number of states");
    assert!(
        !left.iter().any(|e| e.stamp == "2026-08-09_00-00-01"),
        "the wrong one went"
    );
}


#[test]
fn deleting_the_last_state_closes_the_switcher() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    r.push(&[0u8; 64], b"png", "2026-08-09_00-00-00").unwrap();
    let mut a = app_in_switcher(d.path(), "Emerald");
    a.apply(Action::GbaDown(Btn::Y));
    assert!(
        matches!(a.phase(), Phase::Playing { .. }),
        "an empty switcher stayed open"
    );
}

#[test]
fn the_deleted_state_leaves_the_switcher_with_it() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    for i in 0..3 {
        r.push(&[i as u8; 64], b"png", &format!("2026-08-09_00-00-{i:02}"))
            .unwrap();
    }
    let mut a = app_in_switcher(d.path(), "Emerald");
    a.apply(Action::GbaDown(Btn::Y));
    let stamps: Vec<&str> = a
        .polaroid_entries()
        .iter()
        .map(|e| e.stamp.as_str())
        .collect();
    assert_eq!(stamps, ["2026-08-09_00-00-01", "2026-08-09_00-00-00"]);
    assert!(
        matches!(a.phase(), Phase::Polaroids { .. }),
        "the switcher closed with states still in it"
    );
}

