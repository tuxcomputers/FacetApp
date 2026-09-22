//! Compiles the Settings window, and draws the Dock icon.

use std::path::Path;

/// The side of the Dock icon, in pixels. macOS draws the Dock at up to 128 points and asks for twice that
/// on a Retina display; 512 is the usual size an app bundle ships and leaves room above both.
const ICON_SIDE: u32 = 512;

/// How much of that square the drawing fills, leaving the rest as margin.
///
/// **Every icon in the Dock is inset, and one that is not looks bigger than its neighbours rather than
/// closer.** Facet.svg fills its own viewBox to the edges, so drawn at full size the ring sat flush against
/// the tile and the arrowhead was clipped by it. Apple's own grid gives a round mark about 88% of the
/// canvas; this is a shade under that, the ring being the outermost thing in the drawing.
const ICON_FILL: f32 = 0.86;

fn main() {
    // The style is chosen at build time, not at runtime, so one build looks the same everywhere.
    // docs/rust-port.md explains why that serves the uniformity requirement rather than fighting it.
    let config = slint_build::CompilerConfiguration::new().with_style("cupertino".into());
    slint_build::compile_with_config("ui/settings.slint", config).expect("the Settings window failed to compile");

    draw_dock_icon();
}

/// Rasterises `Facet.svg` into `OUT_DIR/dock-icon.png`, which `main.rs` includes.
///
/// **Drawn here rather than committed.** A PNG in the repository is a second copy of the logo that can be
/// left behind when the drawing changes, and nothing would fail when it was: the app would simply go on
/// wearing the old one. Doing it at build time means the icon cannot be older than the SVG.
fn draw_dock_icon() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = repository.join("Facet.svg");
    println!("cargo:rerun-if-changed={}", source.display());

    let svg = std::fs::read(&source)
        .unwrap_or_else(|error| panic!("{} could not be read: {error}", source.display()));
    let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default())
        .unwrap_or_else(|error| panic!("{} is not an SVG this can draw: {error}", source.display()));

    let mut pixmap = resvg::tiny_skia::Pixmap::new(ICON_SIDE, ICON_SIDE)
        .expect("a square pixmap of a fixed size is always allocatable");
    // The drawing is square, so one scale serves both axes. Taken from the tree rather than assumed, so a
    // redrawn logo at another size still fills the icon the same way.
    let scale = (ICON_SIDE as f32 * ICON_FILL) / tree.size().width();
    // Centred in what is left, so the margin is even on all four sides.
    let margin = (ICON_SIDE as f32 * (1.0 - ICON_FILL)) / 2.0;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_translate(margin, margin).pre_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    let out = Path::new(&std::env::var("OUT_DIR").expect("cargo always sets OUT_DIR")).join("dock-icon.png");
    let png = pixmap.encode_png().expect("a pixmap this size always encodes");
    std::fs::write(&out, png)
        .unwrap_or_else(|error| panic!("{} could not be written: {error}", out.display()));
}
