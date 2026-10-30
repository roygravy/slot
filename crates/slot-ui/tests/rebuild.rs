use slot_gfx::{Canvas, Draw};
use slot_ui::{RebuildBanner, TexId, PLATE_H};

#[test]
fn the_banner_sits_in_the_top_bar_with_its_words_in_the_middle() {
    let banner = RebuildBanner {
        line: Some(TexId::from_raw(1)),
        fraction: 0.25,
    };
    for canvas in [Canvas::FRAMED, Canvas { w: 640.0, h: 480.0 }] {
        let mut out = Vec::new();
        banner.draw(canvas, &mut out);
        for d in &out {
            if let Draw::Tex { x, y, w, h, .. } | Draw::Rect { x, y, w, h, .. } = d {
                assert!(*y >= 0.0 && y + h <= PLATE_H, "{d:?} leaves the bar");
                if let Draw::Tex { .. } = d {
                    assert_eq!(x + w / 2.0, canvas.w / 2.0, "off centre across {canvas:?}");
                    assert_eq!(y + h / 2.0, PLATE_H / 2.0, "off centre down the bar");
                }
            }
        }
        let Some(Draw::Rect { w, .. }) = out.last() else {
            panic!("no progress line");
        };
        assert_eq!(*w, canvas.w * 0.25);
    }
}
