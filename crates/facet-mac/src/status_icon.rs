//! The menu bar icon: the macOS half of it.
//!
//! **The drawing itself is `facet_ui::status_icon` and is shared**, because there is one icon and the
//! pixels are not a platform question. What is macOS's is the last step, which is handing those pixels to
//! tray-icon as an `Icon`. That is what this module is, and it is deliberately the whole of it.
//!
//! **`with_icon_as_template(false)` in `main.rs` is the other half of this** and has to stay. A template
//! image is alpha only: macOS throws the colours away and draws the shape in the menu bar's own ink, which
//! is why an earlier build came out black whatever it was given. The shared module's tests assert the
//! colours are in the buffer; only that flag makes them survive the trip to the menu bar.
//!
//! Everything else this file used to hold, including its six geometry tests, is in `facet-ui` unchanged.

use tray_icon::Icon;

/// Re-exported so `main.rs` and `examples/draw-status-icons.rs` keep naming one module for the icon.
///
/// The alternative is every caller reaching into `facet_ui` for the shape and this module for the wrapper,
/// which is two imports for one thing and an invitation to let them drift.
pub use facet_ui::status_icon::{Rendered, Showing, render};

/// Draws the icon for `showing`, as something the menu bar will take.
///
/// **The only line here that is about macOS**, and it is really about tray-icon: `Icon::from_rgba` wants
/// exactly the buffer [`render`] produces, so there is no conversion and nothing to get wrong. Linux is the
/// platform that has to convert, ksni wanting ARGB in network byte order.
pub fn draw(showing: Showing) -> Result<Icon, tray_icon::BadIcon> {
    let rendered = render(showing);
    Icon::from_rgba(rendered.rgba, rendered.width, rendered.height)
}
