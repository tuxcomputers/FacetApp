//! Renders each Settings tab to a PNG, with no window and no menu bar.
//!
//! **This is what lets the tabs be worked on without taking over the screen.** Launching the app puts a real
//! window on the owner's display and needs asking first (CLAUDE.md), so a layout question that only needs
//! looking at is answered here instead: Slint's software renderer draws the same widget tree into a buffer.
//!
//! It is not a substitute for looking at the running app. The software renderer draws the same layout, but
//! font rasterisation and the native window chrome are not what either backend produces, so a pixel here is
//! evidence about arrangement rather than about appearance.
//!
//! **It lives in `facet-ui` because nothing in it is about a platform**, which is also why it can answer
//! whether the shared window is really shared: both machines run the same command against the same sources
//! and the images are comparable. Taking the window server out of the comparison is the point rather than a
//! limitation, so a difference in the output is a difference in the layout. It was in `facet-mac` until
//! 2026-09-25 for the same reason the status icon was: that is where the UI used to live.
//!
//!     cargo run -p facet-ui --example draw-settings-tabs
//!     FACET_TAB_HEIGHT=1200 cargo run -p facet-ui --example draw-settings-tabs
//!     FACET_DATABASE=path/to/appdata.sqlite cargo run -p facet-ui --example draw-settings-tabs
//!
//! FACET_DATABASE fills the Faces, Categories and Report tabs from that database, which must already exist.
//! Rendering it may finalise segments an earlier launch left open on an app face, as launching the app does.
//! Without it those tabs draw empty. FACET_NOW, whole unix seconds, fixes the day the Report opens on, so two
//! machines render the same calendars; without it the Report opens on today.
//!
//! Writes target/settings-tabs/<n>-<tab>.png, one per tab, numbered from 1 in tab order so a listing sorts the
//! way the window reads. The default height is the one the window opens at, so
//! what it draws is what somebody opening Settings sees; FACET_TAB_HEIGHT draws a taller one, which is how to
//! see the whole of a tab that scrolls.

use std::cell::RefCell;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::rc::Rc;

use facet_ui::{ComponentHandle, SettingsWindow};
use slint::platform::software_renderer::{MinimalSoftwareWindow, PremultipliedRgbaColor, TargetPixel};
use slint::platform::{Platform, WindowAdapter};
use slint::{LogicalSize, PhysicalSize, PlatformError};

/// How tall each tab is rendered. **The height the window opens at**, so a tab that does not fit is cut here
/// exactly where the scroll view cuts it in the app. FACET_TAB_HEIGHT overrides it.
const DEFAULT_HEIGHT: u32 = 680;

/// The tabs, by the index `active-tab` takes and the name the file gets.
///
/// **The file is prefixed with the index plus one**, so `1-faces.png` to `6-about.png` sort in tab order
/// rather than alphabetically. The number comes from the index, so reordering the tabs renumbers the files.
const TABS: [(i32, &str); 6] =
    [(0, "faces"), (1, "categories"), (2, "report"), (3, "app"), (4, "device"), (5, "about")];

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

    let height: u32 = std::env::var("FACET_TAB_HEIGHT")
        .ok()
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(DEFAULT_HEIGHT);

    let width = 640u32;
    let ui = SettingsWindow::new()?;
    let fixed_now: Option<i64> = std::env::var("FACET_NOW").ok().map(|value| value.parse()).transpose()?;
    let tabs = std::env::var_os("FACET_DATABASE").map(|path| {
        let path: std::path::PathBuf = path.into();
        let notice = facet_ui::notice::Notice::attach(&ui);
        let faces = facet_ui::faces::Faces::attach(
            &ui,
            path.clone(),
            Rc::new(facet_core::debug_log::Trace::none()),
            true,
            Rc::clone(&notice),
        );
        let categories = facet_ui::categories::Categories::attach(
            &ui,
            path.clone(),
            Rc::new(facet_core::debug_log::Trace::none()),
            notice,
        );
        let report = match fixed_now {
            Some(seconds) => facet_ui::report::Report::attach_with_clock(
                &ui,
                path,
                Rc::new(facet_core::debug_log::Trace::none()),
                move || seconds,
            ),
            None => {
                facet_ui::report::Report::attach(&ui, path, Rc::new(facet_core::debug_log::Trace::none()))
            }
        };
        (faces, categories, report)
    });
    ui.window().set_size(LogicalSize::new(width as f32, height as f32));
    window.set_size(PhysicalSize::new(width, height));
    ui.show()?;
    if let Some((faces, categories, report)) = &tabs {
        faces.refresh();
        categories.refresh();
        report.open();
    }

    // **Emptied first**, so a tab that is renamed or renumbered does not leave its old file beside the new one
    // for `cp target/settings-tabs/*.png` to carry into docs/ as a seventh tab.
    let directory = Path::new("target/settings-tabs");
    if directory.exists() {
        std::fs::remove_dir_all(directory)
            .map_err(|error| format!("{} could not be emptied: {error}", directory.display()))?;
    }
    std::fs::create_dir_all(directory)?;

    let buffer = RefCell::new(vec![Rgba::default(); (width * height) as usize]);
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

        let path = directory.join(format!("{}-{name}.png", index + 1));
        write_png(&path, width, height, &buffer.borrow())?;
        println!("wrote {}", path.display());
    }

    Ok(())
}

fn write_png(
    path: &Path,
    width: u32,
    height: u32,
    pixels: &[Rgba],
) -> Result<(), Box<dyn std::error::Error>> {
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
