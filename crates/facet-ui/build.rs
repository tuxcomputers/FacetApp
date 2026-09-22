//! Compiles the Settings window.
//!
//! **One place chooses the style, which is the point of this crate existing.** Requirement 4 in
//! `docs/rust-port.md` is a single shared UI: uniformity across the three platforms matters and fidelity to
//! any one of them does not. A style chosen per composition root would be three answers to that.

fn main() {
    // Chosen at build time, not at runtime, so one build looks the same everywhere.
    let config = slint_build::CompilerConfiguration::new().with_style("cupertino".into());
    slint_build::compile_with_config("ui/settings.slint", config)
        .expect("the Settings window failed to compile");
}
