//! Renders each Settings tab to a PNG, with no window and no menu bar.
//!
//! **This is what lets the tabs be worked on without taking over the screen.** Launching the app puts a real
//! window on the owner's display and needs asking first (CLAUDE.md), so a layout question that only needs
//! looking at is answered here instead: Slint's software renderer draws the same widget tree into a buffer.
//!
//! It is not a substitute for looking at the running app. The software renderer draws the same layout, but
//! font rasterisation and the native window chrome are not what the Mac backend produces, so a pixel here is
//! evidence about arrangement rather than about appearance.
//!
//!     cargo run --example draw-settings-tabs
//!
//! Writes target/settings-tabs/<tab>.png, one per tab.

use std::cell::RefCell;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::rc::Rc;

use slint::platform::software_renderer::{MinimalSoftwareWindow, PremultipliedRgbaColor, TargetPixel};
use slint::platform::{Platform, WindowAdapter};
use slint::{LogicalSize, PhysicalSize, PlatformError};

slint::include_modules!();

/// How tall each tab is rendered. Taller than the window's own default so a tab that overflows shows what it
/// would scroll to rather than being cut at the point the scroll view would cut it.
const HEIGHT: u32 = 900;

/// The tabs, by the index `active-tab` takes and the name the file gets.
const TABS: [(i32, &str); 6] = [
    (0, "faces"),
    (1, "categories"),
    (2, "report"),
    (3, "app"),
    (4, "device"),
    (5, "about"),
];

struct SoftwareBackend {
    window: Rc<MinimalSoftwareWindow>,
}

impl Platform for SoftwareBackend {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.window.clone())
    }
}

/// A pixel the renderer can write and this file can turn into PNG bytes.
#[derive(Clone, Copy, Default)]
struct Rgba(u8, u8, u8, u8);

impl TargetPixel for Rgba {
    fn blend(&mut self, colour: PremultipliedRgbaColor) {
        let keep = |under: u8| ((under as u32 * (255 - colour.alpha) as u32) / 255) as u8;
        self.0 = colour.red + keep(self.0);
        self.1 = colour.green + keep(self.1);
        self.2 = colour.blue + keep(self.2);
        self.3 = colour.alpha + keep(self.3);
    }

    fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Rgba(red, green, blue, 255)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window = MinimalSoftwareWindow::new(Default::default());
    slint::platform::set_platform(Box::new(SoftwareBackend { window: window.clone() }))?;

    let width = 640u32;
    let ui = SettingsWindow::new()?;
    ui.window().set_size(LogicalSize::new(width as f32, HEIGHT as f32));
    window.set_size(PhysicalSize::new(width, HEIGHT));
    ui.show()?;

    let directory = Path::new("target/settings-tabs");
    std::fs::create_dir_all(directory)?;

    let buffer = RefCell::new(vec![Rgba::default(); (width * HEIGHT) as usize]);
    for (index, name) in TABS {
        ui.set_active_tab(index);

        // Twice, and the second one is not belt and braces. Switching tabs replaces the pane, and the first
        // pass is what lays the new one out; asking for the pixels in the same pass renders the tab that was
        // there before.
        for _ in 0..2 {
            slint::platform::update_timers_and_animations();
            window.request_redraw();
            window.draw_if_needed(|renderer| {
                buffer.borrow_mut().fill(Rgba::default());
                renderer.render(&mut buffer.borrow_mut(), width as usize);
            });
        }

        let path = directory.join(format!("{name}.png"));
        write_png(&path, width, HEIGHT, &buffer.borrow())?;
        println!("wrote {}", path.display());
    }

    Ok(())
}

fn write_png(path: &Path, width: u32, height: u32, pixels: &[Rgba]) -> Result<(), Box<dyn std::error::Error>> {
    let file = BufWriter::new(File::create(path)?);
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;

    let mut bytes = Vec::with_capacity(pixels.len() * 4);
    for pixel in pixels {
        bytes.extend_from_slice(&[pixel.0, pixel.1, pixel.2, pixel.3]);
    }
    writer.write_image_data(&bytes)?;
    Ok(())
}
