fn main() {
    println!("cargo:rerun-if-changed=ntsc");
    cc::Build::new()
        .file("ntsc/snes_ntsc.c")
        .file("ntsc/slot_ntsc.c")
        .include("ntsc")
        .opt_level(2)
        .warnings(false)
        .compile("slot_ntsc");
}
