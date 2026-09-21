//! Writes every menu bar icon state to a PNG, so the icon can be looked at without running the app.
//!
//! **Because the alternative is somebody's menu bar.** Checking an icon used to mean launching the app,
//! taking over the screen and screenshotting a 36x24 point strip of it. This draws the same pixels the
//! status item gets and writes them somewhere they can be opened.
//!
//!     cargo run -p facet-mac --example draw-status-icons [output-directory]
//!
//! Scaled up, because the point is to see the shape: the real icon is 32px and unreadable at that size
//! on a page.

#[path = "../src/status_icon.rs"]
mod status_icon;

use status_icon::Showing;

const SCALE: usize = 6;
const SIZE: usize = 32;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let dir = std::path::Path::new(&out);
    std::fs::create_dir_all(dir)?;

    for (name, showing) in [
        ("running-unlocked", Showing { paused: false, locked: false }),
        ("paused-unlocked", Showing { paused: true, locked: false }),
        ("running-locked", Showing { paused: false, locked: true }),
        ("paused-locked", Showing { paused: true, locked: true }),
    ] {
        let path = dir.join(format!("{name}.png"));
        write_png(&path, &status_icon::alpha(showing))?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

/// Grey on white rather than the template's bare alpha, because alpha alone is invisible in a viewer.
/// What the menu bar actually draws is this shape in whichever colour it needs.
fn write_png(path: &std::path::Path, alpha: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let side = SIZE * SCALE;
    let mut rgb = vec![255u8; side * side * 3];
    for y in 0..side {
        for x in 0..side {
            let on = alpha[(y / SCALE) * SIZE + (x / SCALE)] != 0;
            if on {
                let i = (y * side + x) * 3;
                rgb[i] = 0x22;
                rgb[i + 1] = 0x22;
                rgb[i + 2] = 0x22;
            }
        }
    }

    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), side as u32, side as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&rgb)?;
    Ok(())
}
