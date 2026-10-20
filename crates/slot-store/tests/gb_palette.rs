use slot_store::GbPalette;

#[test]
fn the_list_is_the_game_boys_own_three_screens() {
    let names: Vec<&str> = GbPalette::all().map(GbPalette::core_name).collect();
    assert_eq!(names, ["DMG Green", "GB Pocket", "GB Light"]);
    assert_eq!(GbPalette::COUNT, 3);
}

#[test]
fn labels_read_like_the_game_boys_and_stay_unique() {
    let labels: Vec<&str> = GbPalette::all().map(GbPalette::label).collect();
    assert_eq!(labels, ["GB DMG", "GB Pocket", "GB Light"]);
}

#[test]
fn a_palette_from_the_old_list_reads_as_none() {
    assert_eq!(GbPalette::parse("GBC Dark Green →A"), None);
    assert_eq!(GbPalette::parse("SGB 2-C"), None);
    assert_eq!(GbPalette::parse("not a palette"), None);
}

#[test]
fn next_and_prev_walk_the_list_and_wrap() {
    assert_eq!(GbPalette::DEFAULT.core_name(), "DMG Green");
    assert_eq!(GbPalette::DEFAULT.next().core_name(), "GB Pocket");
    assert_eq!(GbPalette::DEFAULT.prev().core_name(), "GB Light");
    for p in GbPalette::all() {
        assert_eq!(p.next().prev(), p);
    }
}
