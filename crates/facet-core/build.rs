//! Compiles in the bundled Google OAuth client when `resources/google-client.json` is there.
//!
//! The file is named in `rerun-if-changed`, so this runs again whenever it appears, changes or goes, and a
//! build never keeps a client that has been taken away or misses one that has been added.

use std::path::PathBuf;

fn main() {
    let file = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"))
        .join("resources/google-client.json");
    println!("cargo::rerun-if-changed={}", file.display());
    println!("cargo::rustc-check-cfg=cfg(facet_bundled_google)");
    if file.is_file() {
        println!("cargo::rustc-cfg=facet_bundled_google");
        println!("cargo::rustc-env=FACET_BUNDLED_GOOGLE_CLIENT={}", file.display());
    }
}
