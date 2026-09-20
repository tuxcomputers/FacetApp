fn main() {
    // The style is chosen at build time, not at runtime, so one build looks the same everywhere.
    // docs/rust-port.md explains why that serves the uniformity requirement rather than fighting it.
    let config = slint_build::CompilerConfiguration::new().with_style("cupertino".into());
    slint_build::compile_with_config("ui/settings.slint", config).expect("the Settings window failed to compile");
}
