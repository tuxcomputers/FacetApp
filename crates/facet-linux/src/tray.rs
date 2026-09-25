//! The tray icon on Linux, through `ksni`.
//!
//! **`tray-icon` cannot do this here and that is not a preference.** Its Linux backend is
//! libappindicator, which emits no click events at all, and it offers no `ksni` option. See
//! [`docs/port-findings.md`](../../../docs/port-findings.md). So the Mac and Windows share one crate and
//! this platform uses another, which is the shape `docs/architecture.md` expects: the tray is a port, and
//! a port is allowed a different adapter per platform.
//!
//! **The click split is the host's, not the app's**, measured on MATE 2026-09-18 and recorded in
//! [`docs/rust-port.md`](../../../docs/rust-port.md). Left click reaches the app. Right click never does:
//! the host shows the menu that was registered and the app is not told. That is why **nothing may live
//! behind a left click that has no menu equivalent**, and why left click here does exactly what the first
//! menu item does rather than anything of its own.
//!
//! **Everything in this file runs on a thread that is not the UI thread**, because `ksni::Tray` is
//! `Send + 'static` and a Slint handle is neither. Nothing here touches the window: a handler mutates the
//! tray's own state and posts a message, and the composition root's pump picks it up on the UI thread. The
//! Mac reaches the same arrangement from the other side, its crate delivering events on a channel it has
//! to drain.

use std::sync::mpsc::Sender;

use facet_ui::status_icon::{Rendered, Showing, render};
use ksni::menu::StandardItem;
use ksni::{Category, Icon, MenuItem, Status, ToolTip, Tray};

/// What the tray tells the UI thread happened.
///
/// **State travels with the message rather than being asked for afterwards.** The tray thread owns
/// [`Showing`] and the UI thread cannot read it without a lock, so a message that said only "pause was
/// clicked" would leave the reader to work out what it had become, and the two answers could differ. The
/// message carries what it is now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FromTray {
    /// Left click, which is Pause's accelerator. Carries what the state became.
    Activated(Showing),
    /// Middle click. Reported and otherwise unused: it is a spare gesture, not a mechanism.
    SecondaryActivated,
    /// A menu item changed the state. Carries what it became.
    Changed(Showing),
    OpenSettings,
    OpenAbout,
    Quit,
}

/// The status item, and the state it is drawing.
pub struct FacetTray {
    /// **In-memory, and it is the exception being flagged rather than the rule being broken.** Whether the
    /// cube is paused or locked belongs in the database and will be read from it at the point of use, per
    /// CLAUDE.md. There is nothing to read yet: no radio, and no rows that mean either thing. This holds
    /// them so the icon can be shown to follow the state, and it is the holding that is temporary rather
    /// than the icon. The Mac's composition root carries the same note for the same reason.
    showing: Showing,
    to_ui: Sender<FromTray>,
}

impl FacetTray {
    pub fn new(to_ui: Sender<FromTray>) -> Self {
        Self { showing: Showing { paused: false, locked: false }, to_ui }
    }

    /// Posts to the UI thread, and says so if the channel has gone.
    ///
    /// **A closed channel means the event loop has ended**, which happens on the way out and is not a
    /// fault. It is reported on stderr rather than swallowed because the other way it could happen is the
    /// pump having died while the app is still running, and that is a tray whose clicks go nowhere with
    /// nothing saying so. There is no logger on this thread to record it into: the trace holds one
    /// connection and it belongs to the UI thread.
    fn post(&self, message: FromTray) {
        if let Err(error) = self.to_ui.send(message) {
            eprintln!("[tray    ] The UI thread is not listening, so {message:?} went nowhere: {error}");
        }
    }

    /// Flips pause, which is what both the menu item and a left click do.
    ///
    /// **One function for the two routes deliberately.** Left click is an accelerator for the first menu
    /// item, so if it ever did something the item did not, the accelerator would have become a mechanism.
    fn toggle_pause(&mut self) -> Showing {
        self.showing.paused = !self.showing.paused;
        self.showing
    }
}

impl Tray for FacetTray {
    /// The bus name this item registers under. Not a label and not shown to anybody.
    fn id(&self) -> String {
        "facet".into()
    }

    /// **Shown beside the icon on desktops that do that**, which macOS and MATE can and Windows never can.
    fn title(&self) -> String {
        "Facet".into()
    }

    fn category(&self) -> Category {
        Category::ApplicationStatus
    }

    /// `Active` rather than `Passive`, or a host is entitled to hide the item.
    fn status(&self) -> Status {
        Status::Active
    }

    /// The icon, as raw pixels rather than a theme name.
    ///
    /// **A theme name would mean shipping and installing an icon theme**, and the state this icon shows
    /// changes at runtime, so it would mean one installed name per combination. `icon_pixmap` carries its
    /// own width and height, which is also what lets the icon be wider than it is tall.
    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![argb_icon(&render(self.showing))]
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: "Facet".into(),
            description: String::new(),
            icon_name: String::new(),
            icon_pixmap: Vec::new(),
        }
    }

    /// Left click.
    ///
    /// **It reaches the app here, and that is measured rather than assumed**: 2026-09-18 on MATE, left and
    /// middle arrived and right did not. See `docs/rust-port.md`.
    fn activate(&mut self, _x: i32, _y: i32) {
        let showing = self.toggle_pause();
        self.post(FromTray::Activated(showing));
    }

    /// Middle click. Reported so the trace shows it arriving, and wired to nothing.
    fn secondary_activate(&mut self, _x: i32, _y: i32) {
        self.post(FromTray::SecondaryActivated);
    }

    /// The menu, which is the primary route to everything.
    ///
    /// **Rebuilt from `self` every time the host asks**, so the labels follow the state rather than being
    /// set once and mutated: `ksni` re-reads this after every handler and diffs it. Pause and Lock are
    /// first because left click accelerates the first item. About is on it because the Slint Royalty-free
    /// licence wants it reachable from the top level menu and this menu is the app's top level menu; see
    /// NOTICE.
    ///
    /// **The labels are the addressing.** `Tests/Scripted` presses tray items by label on every platform,
    /// no identifier surviving the trip on Linux, so these words are interface and not decoration.
    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: if self.showing.paused { "Resume" } else { "Pause" }.into(),
                activate: Box::new(|tray: &mut Self| {
                    let showing = tray.toggle_pause();
                    tray.post(FromTray::Changed(showing));
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: if self.showing.locked { "Unlock" } else { "Lock" }.into(),
                activate: Box::new(|tray: &mut Self| {
                    tray.showing.locked = !tray.showing.locked;
                    let showing = tray.showing;
                    tray.post(FromTray::Changed(showing));
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Settings...".into(),
                activate: Box::new(|tray: &mut Self| tray.post(FromTray::OpenSettings)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "About Facet".into(),
                activate: Box::new(|tray: &mut Self| tray.post(FromTray::OpenAbout)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit Facet".into(),
                activate: Box::new(|tray: &mut Self| tray.post(FromTray::Quit)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// The shared drawing, in the byte order a StatusNotifierItem wants.
///
/// **This is the one conversion on this platform, and it is easy to get silently wrong.** `render` produces
/// RGBA, which is what macOS and Windows take unchanged. The StatusNotifier specification asks for ARGB32
/// in *network* byte order, so the alpha moves to the front of each pixel. Get it wrong and the icon still
/// appears, in the wrong colours with the wrong transparency, which reads as a drawing fault rather than as
/// a byte order one.
fn argb_icon(rendered: &Rendered) -> Icon {
    let data = rendered
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|pixel| [pixel[3], pixel[0], pixel[1], pixel[2]])
        .collect();
    Icon { width: rendered.width as i32, height: rendered.height as i32, data }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The conversion moves alpha to the front and leaves the colours in order.
    ///
    /// **Worth a test because nothing downstream can fail on it.** A wrong byte order produces an icon that
    /// draws, so neither the host nor the app has anything to complain about, and the only symptom is that
    /// it looks wrong on somebody's panel.
    #[test]
    fn rgba_becomes_argb_in_network_order() {
        let rendered = Rendered { width: 1, height: 1, rgba: vec![0x11, 0x22, 0x33, 0x44] };
        let icon = argb_icon(&rendered);
        assert_eq!(icon.data, vec![0x44, 0x11, 0x22, 0x33], "alpha must lead, then R, G, B");
        assert_eq!((icon.width, icon.height), (1, 1));
    }

    /// Every state converts to a full buffer of the size the dimensions claim.
    ///
    /// A pixmap shorter than `width * height * 4` is a buffer overrun on the other side of the bus.
    #[test]
    fn every_state_fills_the_buffer_it_declares() {
        for paused in [false, true] {
            for locked in [false, true] {
                let showing = Showing { paused, locked };
                let icon = argb_icon(&render(showing));
                assert_eq!(
                    icon.data.len(),
                    (icon.width * icon.height * 4) as usize,
                    "{showing:?} declares a size its buffer does not fill"
                );
            }
        }
    }

    /// Locking widens the icon, here as much as in the shared renderer.
    ///
    /// The conversion must not quietly drop the second glyph, which a chunk-size mistake would do.
    #[test]
    fn locking_widens_the_pixmap() {
        let plain = argb_icon(&render(Showing { paused: false, locked: false }));
        let locked = argb_icon(&render(Showing { paused: false, locked: true }));
        assert!(locked.width > plain.width, "the locked icon is not wider");
        assert_eq!(locked.height, plain.height, "locking must not change the height");
    }
}
