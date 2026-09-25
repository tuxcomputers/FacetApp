//! Compiles the Settings window, and embeds the activity icons.
//!
//! **One place chooses the style, which is the point of this crate existing.** Requirement 4 in
//! `docs/rust-port.md` is a single shared UI: uniformity across the three platforms matters and fidelity to
//! any one of them does not. A style chosen per composition root would be three answers to that.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    // Chosen at build time, not at runtime, so one build looks the same everywhere.
    let config = slint_build::CompilerConfiguration::new().with_style("cupertino".into());
    slint_build::compile_with_config("ui/settings.slint", config)
        .expect("the Settings window failed to compile");

    embed_icons();
}

/// Writes `$OUT_DIR/icons.rs`: `ICONS`, every `ui/icons/*.svg` as `(name without extension, bytes)`, sorted
/// by name.
fn embed_icons() {
    let directory = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"))
        .join("ui/icons");
    println!("cargo:rerun-if-changed={}", directory.display());
    let mut names: Vec<String> = std::fs::read_dir(&directory)
        .expect("ui/icons should be readable")
        .map(|entry| entry.expect("ui/icons entries should be readable").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "svg"))
        .map(|path| path.file_stem().expect("an svg has a stem").to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert!(!names.is_empty(), "ui/icons holds no svg files");

    let mut source = String::from("pub static ICONS: &[(&str, &[u8])] = &[\n");
    for name in &names {
        let path = directory.join(format!("{name}.svg"));
        writeln!(source, "    ({name:?}, include_bytes!({:?})),", path.display().to_string())
            .expect("writing to a String cannot fail");
    }
    source.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("icons.rs");
    std::fs::write(&out, source).expect("OUT_DIR should be writable");
}
