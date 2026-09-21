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
        let rendered = status_icon::render(showing);
        write_png(&path, &rendered)?;
        println!("wrote {} ({}x{})", path.display(), rendered.width, rendered.height);
    }
    Ok(())
}

/// Each state on both a light and a dark background, stacked.
///
/// **Both, because the colours no longer adapt.** The icon is not a template any more, so what you see
/// here is what the menu bar draws whichever mode it is in. White pause on the light half is the case
/// worth looking at.
fn write_png(
    path: &std::path::Path,
    rendered: &status_icon::Rendered,
) -> Result<(), Box<dyn std::error::Error>> {
    let (w, h) = (rendered.width as usize, rendered.height as usize);
    let (out_w, band) = (w * SCALE, h * SCALE);
    let out_h = band * 2;
    let mut rgb = vec![0u8; out_w * out_h * 3];

    // Top band a light menu bar, bottom band a dark one.
    for y in 0..out_h {
        let bg: u8 = if y < band { 0xEC } else { 0x2B };
        for x in 0..out_w {
            let i = (y * out_w + x) * 3;
            rgb[i] = bg;
            rgb[i + 1] = bg;
            rgb[i + 2] = bg;

            let sy = (y % band) / SCALE;
            let j = (sy * w + x / SCALE) * 4;
            if rendered.rgba[j + 3] != 0 {
                rgb[i] = rendered.rgba[j];
                rgb[i + 1] = rendered.rgba[j + 1];
                rgb[i + 2] = rendered.rgba[j + 2];
            }
        }
    }

    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), out_w as u32, out_h as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&rgb)?;
    Ok(())
}
