use slot_store::{Cart, Platform};
use slot_ui::Shelf;

fn shelf_with(n: usize) -> Shelf {
    Shelf::new(
        (0..n)
            .map(|i| Cart {
                platform: Platform::Gba,
                stem: format!("Game {i}"),
                rom: format!("Games/GBA/Game {i}.gba").into(),
                label: None,
                code: String::new(),
                shell: None,
                marker: None,
                title: format!("GAME {i}"),
            })
            .collect(),
    )
}

#[test]
fn a_long_frame_does_not_throw_the_shelf_off_screen() {
    let mut s = shelf_with(20);
    for _ in 0..5 {
        s.right();
    }
    let (from, to) = (s.scroll, s.scroll_target());
    // One frame of half a second, as when a game is loading.
    s.update(0.5);
    let span = (to - from).abs();
    assert!(
        (s.scroll - to).abs() <= span,
        "jumped to {} going from {from} to {to}",
        s.scroll
    );
    assert!(s.scroll.is_finite());
}
