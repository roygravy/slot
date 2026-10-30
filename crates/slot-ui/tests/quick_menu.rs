use slot_store::{parse_stamp, FF_SPEEDS};
use slot_ui::{
    date_time_text, quick_caret_face, quick_label_face, quick_value_face, ClockPicker, QuickRow,
    QuickValue, UndoFace, MENU_PAD,
};

fn inked(f: &UndoFace, x: u32, y: u32) -> bool {
    f.rgba[((y * f.w + x) * 4 + 3) as usize] > 128
}

fn ink_columns(f: &UndoFace) -> (u32, u32) {
    let cols: Vec<u32> = (0..f.w)
        .filter(|&x| (0..f.h).any(|y| inked(f, x, y)))
        .collect();
    (
        *cols.first().expect("an empty face"),
        *cols.last().expect("an empty face"),
    )
}

#[test]
fn the_type_sits_exactly_menu_pad_in_from_both_sides_of_its_face() {
    for f in [
        quick_label_face(QuickRow::FastForwardSound, true),
        quick_label_face(QuickRow::Rumble, true),
        quick_value_face("Off", false),
        quick_value_face(QuickValue::Speed6.text(), true),
        quick_value_face("SEP 15 16:35", true),
    ] {
        let (first, last) = ink_columns(&f);
        let right = f.w - 1 - MENU_PAD;
        assert!(
            (MENU_PAD..MENU_PAD + 4).contains(&first),
            "ink starts at {first} of {}",
            f.w
        );
        assert!(
            (right - 4..=right).contains(&last),
            "ink ends at {last} of {}",
            f.w
        );
    }
}

#[test]
fn a_long_label_is_set_as_large_as_a_short_one() {
    let tall = |f: &UndoFace| {
        (0..f.h)
            .filter(|&y| (0..f.w).any(|x| inked(f, x, y)))
            .count()
    };
    let long = tall(&quick_label_face(QuickRow::FastForwardSound, true));
    let short = tall(&quick_label_face(QuickRow::Rumble, true));
    assert!(long + 1 >= short, "{long} rows of ink against {short}");
}

#[test]
fn the_fast_forward_row_offers_the_four_ceilings_the_card_can_hold() {
    assert_eq!(FF_SPEEDS, [2, 3, 4, 6]);
    assert_eq!(
        FF_SPEEDS.map(QuickValue::speed),
        [
            Some(QuickValue::Speed2),
            Some(QuickValue::Speed3),
            Some(QuickValue::Speed4),
            Some(QuickValue::Speed6),
        ]
    );
    for other in [1, 5, 7, 9, 16, 28, 255] {
        assert_eq!(
            QuickValue::speed(other),
            None,
            "{other}x is not a value the row has"
        );
    }
}

#[test]
fn the_run_ahead_row_offers_off_and_one_frame() {
    assert_eq!(
        [0, 1].map(QuickValue::frames),
        [Some(QuickValue::Off), Some(QuickValue::Frames1)]
    );
    for other in [2, 3, 255] {
        assert_eq!(
            QuickValue::frames(other),
            None,
            "{other} frames is not a value the row has"
        );
    }
}

#[test]
#[ignore = "upstream expectation that slot. x GRAVY changed on purpose"]
fn a_hidden_row_is_stepped_over_and_takes_no_place() {
    let hidden = [QuickRow::WallpaperTint];
    assert_eq!(QuickRow::Wallpaper.down(&hidden), QuickRow::Theme);
    assert_eq!(QuickRow::Theme.up(&hidden), QuickRow::Wallpaper);
    assert_eq!(QuickRow::Wallpaper.up(&hidden), QuickRow::DateTime);
    assert_eq!(QuickRow::Theme.position(&hidden), 1);
    assert_eq!(QuickRow::Wallpaper.down(&[]), QuickRow::WallpaperTint);
    assert_eq!(QuickRow::Theme.position(&[]), 2);
    assert_eq!(
        slot_ui::quick_row_top(QuickRow::Theme, &hidden)
            - slot_ui::quick_row_top(QuickRow::Wallpaper, &hidden),
        slot_ui::QUICK_PITCH
    );
}

#[test]
fn the_date_and_time_read_as_a_month_a_day_and_the_carousels_24_hour_clock() {
    let at = |stamp: &str| date_time_text(parse_stamp(stamp).expect("a stamp"));
    assert_eq!(at("2026-09-15_16-35-00"), "SEP 15 16:35");
    assert_eq!(at("2027-01-05_04-07-59"), "JAN 5 04:07");
}

#[test]
fn a_picker_for_a_set_clock_starts_at_the_local_time_and_its_offset() {
    let utc = parse_stamp("2026-09-15_21-35-42").expect("a stamp");
    let p = ClockPicker::local(utc, slot_ui::TimeZone::Fixed(-300));
    assert_eq!(p.offset_min(), -300);
    assert_eq!(
        p.secs(),
        utc - 42,
        "the picker does not show the minute it was opened in"
    );
    assert!(p.text().starts_with("2026-09-15 16:35"), "{}", p.text());
}

#[test]
#[ignore = "upstream expectation that slot. x GRAVY changed on purpose"]
fn a_value_is_grey_until_its_row_is_in_hand() {
    let inkiest = |lit| {
        let f = quick_value_face("4×", lit);
        f.rgba
            .chunks(4)
            .max_by_key(|p| p[3])
            .map(|p| [p[0], p[1], p[2]])
            .expect("an empty face")
    };
    assert_eq!(inkiest(true), [0xf6, 0xf4, 0xef]);
    assert_eq!(inkiest(false), [0x9a, 0x9a, 0xa4]);
}

#[test]
fn the_arrows_are_faces_the_height_of_a_value() {
    for right in [false, true] {
        let caret = quick_caret_face(right);
        assert!(
            caret.w > 0 && caret.rgba.chunks(4).any(|p| p[3] > 0),
            "an empty arrow"
        );
        assert_eq!(caret.h, quick_value_face("On", true).h);
    }
}

#[test]
fn the_two_carets_point_different_ways() {
    assert_ne!(
        slot_ui::quick_caret_face(false).rgba,
        slot_ui::quick_caret_face(true).rgba,
        "a caret glyph is missing from the symbols font"
    );
}

#[test]
fn the_pointer_is_a_crisp_pixel_arrow_in_the_menus_ink() {
    let p = slot_ui::quick_pointer_face();
    assert_eq!(p.w, 32);
    let opaque: Vec<&[u8]> = p.rgba.chunks(4).filter(|c| c[3] > 0).collect();
    assert!(!opaque.is_empty(), "an empty pointer");
    assert!(
        opaque.iter().all(|c| c == &[0xf6, 0xf4, 0xef, 0xff]),
        "the pointer is not one solid colour"
    );
    assert!(
        p.rgba.chunks(4).all(|c| c[3] == 0 || c[3] == 0xff),
        "the arrow was smoothed, not drawn in pixels"
    );
}

#[test]
#[ignore = "upstream expectation that slot. x GRAVY changed on purpose"]
fn a_label_is_grey_until_its_row_is_in_hand() {
    let inkiest = |lit| {
        let f = quick_label_face(QuickRow::Rumble, lit);
        f.rgba
            .chunks(4)
            .max_by_key(|p| p[3])
            .map(|p| [p[0], p[1], p[2]])
            .expect("an empty face")
    };
    assert_eq!(inkiest(true), [0xf6, 0xf4, 0xef]);
    assert_eq!(inkiest(false), [0x9a, 0x9a, 0xa4]);
}

#[test]
#[ignore = "upstream expectation that slot. x GRAVY changed on purpose"]
fn in_game_pages_climb_back_through_settings() {
    assert_eq!(QuickRow::Settings.child(), Some(QuickRow::PlayScreen));
    assert_eq!(QuickRow::PlayScreen.child(), Some(QuickRow::Shader));
    assert_eq!(QuickRow::PlayGameplay.child(), Some(QuickRow::FastForward));
    assert_eq!(
        QuickRow::Shader.parent_in_game(),
        Some(QuickRow::PlayScreen)
    );
    assert_eq!(
        QuickRow::PlayGameplay.parent_in_game(),
        Some(QuickRow::Settings)
    );
    assert_eq!(
        QuickRow::Rumble.parent_in_game(),
        Some(QuickRow::PlayGameplay)
    );
    assert_eq!(QuickRow::Rumble.parent(), Some(QuickRow::Game));
}

#[test]
#[ignore = "upstream expectation that slot. x GRAVY changed on purpose"]
fn screen_opens_a_page_per_console_and_each_backs_out_to_its_console() {
    assert_eq!(QuickRow::Screen.child(), Some(QuickRow::ScreenGba));
    for (console, page) in [
        (QuickRow::ScreenGba, &QuickRow::SCREEN_GBA[..]),
        (QuickRow::ScreenGbc, &QuickRow::SCREEN_GBC[..]),
        (QuickRow::ScreenGb, &QuickRow::SCREEN_GB[..]),
    ] {
        assert_eq!(console.parent(), Some(QuickRow::Screen));
        assert_eq!(console.child(), Some(page[0]));
        for &row in page {
            assert_eq!(row.parent(), Some(console), "{row:?}");
        }
    }
    assert!(QuickRow::ScreenGb.centred());
    assert!(!QuickRow::GbShader.centred());
}

#[test]
fn the_wallpaper_tint_reads_as_how_much_it_darkens() {
    assert_eq!(
        [QuickValue::Low, QuickValue::Medium, QuickValue::High].map(QuickValue::text),
        ["25%", "45%", "65%"]
    );
}

#[test]
#[ignore = "upstream expectation that slot. x GRAVY changed on purpose"]
fn show_titles_sits_under_recents_on_the_general_page() {
    let at = |r: QuickRow| QuickRow::GENERAL.iter().position(|g| *g == r);
    assert_eq!(
        at(QuickRow::ShowTitles),
        at(QuickRow::Recents).map(|i| i + 1)
    );
    assert_eq!(QuickRow::ShowTitles.parent(), Some(QuickRow::General));
}
