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
/// **Pause is a press, not a state.** Whether the clock is paused is the database's answer, and this
/// thread has no connection to ask it with, so the tray posts that Pause was pressed and the UI thread
/// does the toggling through `Faces`. What the icon and the item show afterwards is pushed back in with
/// [`FacetTray::follow_clock`], from a read made after the toggle.
///
/// **Lock still carries its state**, because Lock is still the tray's own: it is the cube's, there is no
/// radio yet, and the tray thread is where it is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FromTray {
    /// Left click, which is Pause's accelerator.
    Activated,
    /// Middle click. Reported and otherwise unused: it is a spare gesture, not a mechanism.
    SecondaryActivated,
    /// The Pause or Resume menu item.
    PausePressed,
    /// The Lock menu item changed the state. Carries what it became.
    LockChanged(Showing),
    OpenSettings,
    OpenAbout,
    Quit,
}

/// The status item, and the state it is drawing.
pub struct FacetTray {
    /// What the icon draws.
    ///
    /// **`paused` is not this thread's to decide.** It is written only by [`FacetTray::follow_clock`], with
    /// what the UI thread read from the database after the clock last changed, and it is held here because
    /// `ksni` asks for the icon on this thread, synchronously, and there is no connection here to read it
    /// with. It goes stale only if a change to the clock skips `Faces`'s re-read, which nothing does.
    ///
    /// **`locked` is in-memory, and it is the exception being flagged rather than the rule being broken.**
    /// It is the cube's, and there is no radio yet to read it from. It is held so the icon can be shown to
    /// follow it; the Mac's composition root carries the same note.
    showing: Showing,
    /// The Pause item's label, `Pause` or `Resume`, pushed in alongside `paused` and for the same reason.
    pause_title: &'static str,
    /// Whether Pause and the left click do anything, pushed in alongside `paused`. False while the clock is
    /// idle or a resume would pass a spent daily limit.
    is_pause_clickable: bool,
    to_ui: Sender<FromTray>,
}

impl FacetTray {
    /// **Starts showing an idle clock with Pause disabled**, which is what an unread clock should look like:
    /// nothing offered until the first read says otherwise. That read is pushed in straight after spawning.
    pub fn new(to_ui: Sender<FromTray>) -> Self {
        Self {
            showing: Showing { paused: true, locked: false },
            pause_title: "Pause",
            is_pause_clickable: false,
            to_ui,
        }
    }

    /// Shows what the clock is doing, as the UI thread just read it.
    ///
    /// **Only the Faces tab's clock reaches this**, via `Handle::update`; nothing on the tray thread calls
    /// it. Returns whether anything changed, so the caller can say so in the trace once rather than once a
    /// tick.
    pub fn follow_clock(&mut self, is_paused: bool, pause_title: &'static str, is_clickable: bool) -> bool {
        let changed = self.showing.paused != is_paused
            || self.pause_title != pause_title
            || self.is_pause_clickable != is_clickable;
        self.showing.paused = is_paused;
        self.pause_title = pause_title;
        self.is_pause_clickable = is_clickable;
        changed
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
    ///
    /// **Posted whether or not Pause is enabled**, and the UI thread's toggle refuses it when the clock says
    /// no, exactly as the Faces tab's glyph would. The item's `enabled` is this thread's picture of that;
    /// the clock's own rule is the one that decides.
    fn activate(&mut self, _x: i32, _y: i32) {
        self.post(FromTray::Activated);
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
                label: self.pause_title.into(),
                enabled: self.is_pause_clickable,
                activate: Box::new(|tray: &mut Self| tray.post(FromTray::PausePressed)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: if self.showing.locked { "Unlock" } else { "Lock" }.into(),
                activate: Box::new(|tray: &mut Self| {
                    tray.showing.locked = !tray.showing.locked;
                    let showing = tray.showing;
                    tray.post(FromTray::LockChanged(showing));
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

    /// The Pause item is whatever the clock last said, and a repeat of it is not a change.
    ///
    /// **Worth a test because the trace depends on it**: `follow_clock` returning true on every tick would
    /// write a row a second while the figure moves, and returning false on a real change would leave the
    /// trace silent about the one thing a check wants to see.
    #[test]
    fn the_pause_item_follows_the_clock() {
        let (to_ui, _from_tray) = std::sync::mpsc::channel();
        let mut tray = FacetTray::new(to_ui);
        let pause = |tray: &FacetTray| match tray.menu().into_iter().next() {
            Some(MenuItem::Standard(item)) => (item.label, item.enabled),
            _ => panic!("the first menu item is not Pause"),
        };
        assert_eq!(pause(&tray), ("Pause".to_string(), false), "an unread clock must offer nothing");

        assert!(tray.follow_clock(true, "Resume", true));
        assert_eq!(pause(&tray), ("Resume".to_string(), true));
        assert!(tray.showing.paused);
        assert!(!tray.follow_clock(true, "Resume", true), "the same reading again is not a change");

        assert!(tray.follow_clock(false, "Pause", true));
        assert_eq!(pause(&tray), ("Pause".to_string(), true));
        assert!(!tray.showing.paused);
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
