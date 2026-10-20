//! Write the boot logo slot would install on a 640x480 panel: `bootlogo_preview CARD_LOGO OUT`.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mark = slot::bootlogo::decode(&std::fs::read(&args[0]).unwrap()).unwrap();
    let f = slot_ui::theme::MENU_FLAT_DARK;
    let ground: Vec<u8> = (0..640 * 480).flat_map(|_| [f[0], f[1], f[2], 255]).collect();
    let logo = slot::bootlogo::compose_over(&mark, (640, 480), &ground, (640, 480)).unwrap();
    std::fs::write(&args[1], slot::bootlogo::encode(&logo)).unwrap();
}
