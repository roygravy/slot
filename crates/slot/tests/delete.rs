mod common;

use common::{app_in_switcher, tmp_root_with_carts};
use slot::app::Phase;
use slot_input::{Action, Btn};
use slot_store::{Core, Platform, StateRing};
use slot_ui::{ConfirmFaces, TexId};

/// Y only asks; A on the question deletes. The question needs its faces to show.
fn confirm_faces(a: &mut slot::app::App) {
    let id = TexId::from_raw;
    a.set_state_confirm_faces(ConfirmFaces {
        cancel: (id(1), 40),
        delete: (id(2), 40),
        line: (id(3), 200, 30),
    });
}

#[test]
fn y_deletes_the_selected_state_and_nothing_else() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    for i in 0..3 {
        r.push(&[i as u8; 64], b"png", &format!("2026-08-09_00-00-{i:02}"))
            .unwrap();
    }
    let mut a = app_in_switcher(d.path(), "Emerald");
    confirm_faces(&mut a);
    a.apply(Action::ShelfRight);
    a.apply(Action::GbaDown(Btn::Y));
    a.apply(Action::GbaDown(Btn::A));
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
    confirm_faces(&mut a);
    a.apply(Action::GbaDown(Btn::Y));
    a.apply(Action::GbaDown(Btn::A));
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
    confirm_faces(&mut a);
    a.apply(Action::GbaDown(Btn::Y));
    a.apply(Action::GbaDown(Btn::A));
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

