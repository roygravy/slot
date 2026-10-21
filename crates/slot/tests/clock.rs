mod common;

use common::{app_booting_at, app_booting_with_clock, tmp_root_with_carts, Clock};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{read_slot_state, write_slot_state, SlotState};
use slot_ui::{clock_text, date_time_text_in, ClockPicker, Field, QuickRow, TimeZone};
use tempfile::TempDir;

#[test]
fn the_clock_is_asked_for_once_and_only_once() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let a = App::boot(d.path());
    assert!(
        matches!(a.phase(), Phase::SetClock { .. }),
        "not asked on first launch"
    );

    let mut a = App::boot(d.path());
    a.confirm_clock();
    assert!(read_slot_state(d.path()).clock_set);
    let again = App::boot(d.path());
    assert!(
        !matches!(again.phase(), Phase::SetClock { .. }),
        "asked twice"
    );
}

#[test]
fn the_clock_comes_before_a_seated_cart() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            cart: Some("Emerald".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let a = App::boot(d.path());
    assert!(matches!(a.phase(), Phase::SetClock { .. }));
}

#[test]
fn confirming_writes_the_picked_time_to_the_platform() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let (mut a, clock) = app_booting_with_clock(d.path());
    a.apply(Action::GbaDown(Btn::Up));
    a.confirm_clock();
    assert_ne!(clock.get(), 0, "the platform clock was never set");
}

#[test]
fn the_picker_starts_from_the_clock_the_platform_already_has() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let (mut a, clock) = app_booting_at(d.path(), 1_700_000_000);
    a.confirm_clock();
    assert_eq!(
        clock.get(),
        1_700_000_000,
        "confirming the clock as it stands moved it"
    );
}

#[test]
fn a_field_wraps_rather_than_running_off_the_end() {
    let mut c = ClockPicker::from_secs(0);
    c.field(Field::Month);
    for _ in 0..13 {
        c.up();
    }
    assert!(
        (1..=12).contains(&c.month()),
        "month {} left the calendar",
        c.month()
    );
}

#[test]
fn february_29_is_reachable_in_a_leap_year_and_not_otherwise() {
    let mut c = ClockPicker::from_ymd(2028, 2, 28);
    c.field(Field::Day);
    c.up();
    assert_eq!(c.day(), 29, "2028 is a leap year");
    let mut c = ClockPicker::from_ymd(2027, 2, 28);
    c.field(Field::Day);
    c.up();
    assert_eq!(c.day(), 1, "2027 has no 29th of February");
}

#[test]
fn a_day_the_new_month_does_not_have_is_pulled_back() {
    let mut c = ClockPicker::from_ymd(2026, 1, 31);
    c.field(Field::Month);
    c.up();
    assert_eq!((c.month(), c.day()), (2, 28));
}

#[test]
fn the_wall_clock_follows_summer_time_in_the_city() {
    let d = tmp_root_with_carts(&["Emerald"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            time_zone: Some("Amsterdam"),
            ..SlotState::default()
        },
    )
    .unwrap();
    let summer = slot_store::parse_stamp("2026-10-21_18-22-00").unwrap();
    let (a, _clock) = app_booting_at(d.path(), summer);
    assert_eq!(a.wall_secs(), summer + 120 * 60);
    let winter = slot_store::parse_stamp("2026-12-01_18-22-00").unwrap();
    let (a, _clock) = app_booting_at(d.path(), winter);
    assert_eq!(a.wall_secs(), winter + 60 * 60);
}

#[test]
fn the_wall_clock_is_local_rather_than_the_utc_the_card_keeps() {
    let d = tmp_root_with_carts(&["Emerald"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            utc_offset_min: -300,
            ..SlotState::default()
        },
    )
    .unwrap();
    let (a, _clock) = app_booting_at(d.path(), 1_700_000_000);
    assert_eq!(a.wall_secs(), 1_700_000_000 - 300 * 60);
}

#[test]
fn a_clock_that_never_got_set_is_asked_for_again() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            ..SlotState::default()
        },
    )
    .unwrap();
    let (a, _clock) = app_booting_at(d.path(), 0);
    assert!(
        matches!(a.phase(), Phase::SetClock { .. }),
        "a 1970 clock was taken at face value: {:?}",
        a.phase()
    );
}

#[test]
fn a_clock_that_looks_like_a_real_date_is_not_asked_for_again() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            ..SlotState::default()
        },
    )
    .unwrap();
    let (a, _clock) = app_booting_at(d.path(), 1_786_568_000);
    assert!(
        !matches!(a.phase(), Phase::SetClock { .. }),
        "asked again for a clock that was already right"
    );
}

const AT: i64 = 1_786_568_022;
const OPEN_FOR: i64 = 100;

#[derive(Copy, Clone, Debug)]
enum Way {
    FirstBoot,
    QuickMenu,
}

fn clock_screen(way: Way) -> (TempDir, App, Clock) {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    if let Way::QuickMenu = way {
        write_slot_state(
            d.path(),
            &SlotState {
                clock_set: true,
                utc_offset_min: -300,
                auto_time: false,
                ..SlotState::default()
            },
        )
        .unwrap();
    }
    let (mut a, clock) = app_booting_at(d.path(), AT);
    if let Way::QuickMenu = way {
        a.apply(Action::QuickMenu);
        for _ in 0..QuickRow::General.position(&[]) {
            a.apply(Action::GbaDown(Btn::Down));
        }
        a.apply(Action::GbaDown(Btn::A));
        for _ in 0..QuickRow::DateTime.position(a.hidden_rows()) {
            a.apply(Action::GbaDown(Btn::Down));
        }
        a.apply(Action::GbaDown(Btn::A));
    }
    assert!(
        matches!(a.phase(), Phase::SetClock { .. }),
        "{way:?} never reached the clock: {:?}",
        a.phase()
    );
    (d, a, clock)
}

#[test]
fn confirming_the_clock_untouched_leaves_it_where_it_is() {
    for way in [Way::FirstBoot, Way::QuickMenu] {
        let (_d, mut a, clock) = clock_screen(way);
        clock.advance(OPEN_FOR);
        a.apply(Action::GbaDown(Btn::A));
        assert_eq!(
            clock.get(),
            AT + OPEN_FOR,
            "{way:?}: an untouched confirm moved the clock by {} s",
            clock.get() - (AT + OPEN_FOR)
        );
    }
}

#[test]
fn changing_the_minute_moves_the_clock_by_exactly_that_minute() {
    for way in [Way::FirstBoot, Way::QuickMenu] {
        let (_d, mut a, clock) = clock_screen(way);
        for _ in 0..4 {
            a.apply(Action::GbaDown(Btn::Right));
        }
        assert_eq!(a.picker().expect("on the clock").cursor(), Field::Minute);
        a.apply(Action::GbaDown(Btn::Up));
        clock.advance(OPEN_FOR);
        a.apply(Action::GbaDown(Btn::A));
        assert_eq!(
            clock.get(),
            AT + OPEN_FOR + 60,
            "{way:?}: a minute on moved the clock by {} s",
            clock.get() - (AT + OPEN_FOR)
        );
    }
}

#[test]
fn the_picker_in_a_city_hands_back_utc_rather_than_what_was_typed() {
    let ny = slot_store::zone_named("New York").unwrap();
    let utc = slot_store::parse_stamp("2026-08-12_13-00-00").unwrap();
    let mut c = ClockPicker::local(utc, TimeZone::City(ny));
    assert!(c.text().contains("09:00"), "{}", c.text());
    c.field(Field::Hour);
    c.up();
    assert_eq!(c.secs(), utc + 3600);
}

#[test]
fn the_time_zone_walks_the_cities_and_wraps_around_the_globe() {
    let first = TimeZone::Fixed(0).next(true);
    assert!(first.city().is_some(), "a bare offset stayed a bare offset");
    let mut z = first;
    for _ in 0..slot_store::ZONES.len() {
        z = z.next(true);
    }
    assert_eq!(z, first);
    assert_eq!(first.next(true).next(false), first);
}

#[test]
fn a_12_hour_clock_says_am_and_pm() {
    let evening = slot_store::parse_stamp("2026-10-21_18-05-00").unwrap();
    assert_eq!(clock_text(evening, true), "18:05");
    assert_eq!(clock_text(evening, false), "6:05 PM");
    let midnight = slot_store::parse_stamp("2026-10-21_00-30-00").unwrap();
    assert_eq!(clock_text(midnight, false), "12:30 AM");
    let noon = slot_store::parse_stamp("2026-10-21_12-00-00").unwrap();
    assert_eq!(clock_text(noon, false), "12:00 PM");
    assert_eq!(date_time_text_in(evening, false), "OCT 21 6:05 PM");
}

#[test]
fn the_12_hour_picker_shows_am_and_pm_and_keeps_the_moment() {
    let utc = slot_store::parse_stamp("2026-10-21_18-05-00").unwrap();
    let c = ClockPicker::local(utc, TimeZone::Fixed(0)).hours24(false);
    assert!(c.text().ends_with("6:05 PM"), "{}", c.text());
    assert_eq!(c.secs(), utc);
}

fn general_row(state: SlotState, row: QuickRow) -> (TempDir, App) {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(d.path(), &SlotState { clock_set: true, ..state }).unwrap();
    let (mut a, _clock) = app_booting_at(d.path(), AT);
    a.apply(Action::QuickMenu);
    for _ in 0..QuickRow::General.position(&[]) {
        a.apply(Action::GbaDown(Btn::Down));
    }
    a.apply(Action::GbaDown(Btn::A));
    for _ in 0..row.position(a.hidden_rows()) {
        a.apply(Action::GbaDown(Btn::Down));
    }
    assert_eq!(a.quick_menu(), Some(row));
    (d, a)
}

#[test]
fn time_zone_steps_through_the_cities_and_is_saved() {
    let (d, mut a) = general_row(
        SlotState {
            time_zone: Some("Amsterdam"),
            ..SlotState::default()
        },
        QuickRow::TimeZone,
    );
    a.apply(Action::GbaDown(Btn::Right));
    assert_eq!(a.zone_name(), "Berlin");
    a.apply(Action::GbaDown(Btn::Left));
    a.apply(Action::GbaDown(Btn::Left));
    assert_eq!(a.zone_name(), "Lisbon");
    a.apply(Action::QuickMenu);
    assert_eq!(read_slot_state(d.path()).time_zone, Some("Lisbon"));
}

#[test]
fn date_and_time_stays_shut_while_auto_time_is_on() {
    let (_d, mut a) = general_row(SlotState::default(), QuickRow::DateTime);
    a.apply(Action::GbaDown(Btn::A));
    assert!(!matches!(a.phase(), Phase::SetClock { .. }), "opened while locked");
    assert_eq!(a.quick_menu(), Some(QuickRow::DateTime));
}

#[test]
fn auto_time_and_the_24_hour_clock_are_on_until_turned_off() {
    let (d, mut a) = general_row(SlotState::default(), QuickRow::AutoTime);
    assert!(read_slot_state(d.path()).auto_time);
    a.apply(Action::GbaDown(Btn::Right));
    a.apply(Action::GbaDown(Btn::Down));
    a.apply(Action::GbaDown(Btn::Down));
    a.apply(Action::GbaDown(Btn::Right));
    assert!(!a.clock_24h());
    a.apply(Action::QuickMenu);
    let saved = read_slot_state(d.path());
    assert!(!saved.auto_time);
    assert!(!saved.clock_24h);
}

