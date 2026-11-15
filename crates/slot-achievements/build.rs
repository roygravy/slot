//! Builds the vendored rcheevos (MIT, see rcheevos/LICENSE) and our small C shim.
use std::path::Path;

fn main() {
    let rc = Path::new("rcheevos");
    let mut build = cc::Build::new();
    build
        .include(rc.join("include"))
        .include(rc.join("src"))
        .define("RC_CLIENT_SUPPORTS_HASH", None)
        .warnings(false)
        .opt_level(2);
    for dir in ["src", "src/rapi", "src/rcheevos", "src/rhash"] {
        for entry in std::fs::read_dir(rc.join(dir)).expect("rcheevos source") {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "c") {
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                // The external-client and RAIntegration hooks are Windows-only features.
                if name.starts_with("rc_client_external") {
                    continue;
                }
                build.file(&path);
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
    build.file("src/shim.c");
    println!("cargo:rerun-if-changed=src/shim.c");
    println!("cargo:rerun-if-changed=src/shim.h");
    build.compile("rcheevos_shim");
}
