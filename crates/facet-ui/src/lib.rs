//! The Settings window, and nothing that knows which machine it is drawn on.
//!
//! **This crate exists because the UI is shared and the composition roots are not.** Requirement 4 in
//! [`docs/rust-port.md`](../../../docs/rust-port.md) is a single shared UI, and it is the requirement the
//! choice of Rust rests on: uniformity across macOS, Linux and Windows matters, and fidelity to any one of
//! them does not. The window lived inside `facet-mac` while there was only one composition root, which made
//! it a macOS window by accident of where the file sat.
//!
//! **It is not the core, and the core does not depend on it.** `facet-core` states what the app *is*;
//! this states what it *looks like*, and [`faces`] fills the Faces tab from the core. The dependency runs one
//! way: this crate uses the core, and the core knows nothing of this crate.
//!
//! **It is platform-blind in the same sense the core is**: no `cfg`, no platform crate, and nothing here
//! may learn what it is running on. What differs per platform is the menu bar and the tray, and those are
//! the composition root's.
//!
//! Everything the `.slint` files export arrives through `include_modules!` and is re-exported, so a caller
//! writes `use facet_ui::SettingsWindow;` and never names this crate's internals.
//!
//! **The tray icon is here for the same reason the window is**, in [`status_icon`]: there is one of it, and
//! the pixels and the geometry are not a platform question. What differs is the tray API each composition
//! root hands the result to, and that stays with the root. The window is Slint and the icon is a byte
//! buffer, which is a difference in how they are drawn rather than in who they belong to.

slint::include_modules!();

pub mod categories;
pub mod create;
pub mod faces;
pub mod icons;
pub mod notice;
pub mod status_icon;

/// Re-exported because every generated component needs it and nothing else about it is interesting.
///
/// `show`, `hide`, `window` and `as_weak` are all `ComponentHandle`'s rather than the component's own, so
/// a caller that has [`SettingsWindow`] and not this trait has a handle it cannot do anything with. Taking
/// it from here means the window and the one trait that operates it arrive together.
pub use slint::ComponentHandle;
