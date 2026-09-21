//! Composition root for macOS. The only place that knows both a port and the thing that performs it.
//!
//! Today that is barely anything: a status item, a menu, and the Settings window. There is no radio,
//! no database and no core behind it yet. What it does establish is the shape the rest hangs off, and
//! the two rules that are easy to get wrong later: the menu is the primary route to everything, and
//! left click is an accelerator rather than a mechanism.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use tray_icon::{
    TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

mod status_icon;

slint::include_modules!();

/// How often the tray's event channels are drained, on the UI thread.
///
/// tray-icon delivers events on crossbeam channels rather than through the Slint event loop, so
/// something has to pump them. Polling on the main thread is what keeps every handler free to touch
/// the window directly; the alternative, a `Send + Sync` callback, cannot hold a Slint handle at all.
const TRAY_POLL: Duration = Duration::from_millis(100);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = SettingsWindow::new()?;

    // A menu bar app owns no dock icon. This has to happen after Slint has built its backend, because
    // that is what creates the application object, and again from inside the event loop below, because
    // the windowing layer sets its own policy on the way up.
    show_in_dock(false);

    ui.on_tab_selected(|tab| {
        // Stands in for the debug log until there is one. The scripted suite reads a line of exactly
        // this shape to prove that selecting a tab did something, so the wording is interface.
        println!("[settings] Settings tab selected: {tab}");
    });

    // Closing Settings puts the app back in the menu bar and nowhere else.
    //
    // **Hidden rather than destroyed**, which is what makes the next open cheap and is why this callback
    // has to say so: the default for a Slint window is to close it, and a closed window cannot be shown
    // again. The values it holds are not carried over, whatever it keeps in memory: the window reads them
    // again on every open, per the source-of-truth rule in CLAUDE.md.
    ui.window().on_close_requested(|| {
        show_in_dock(false);
        println!("[settings] Settings closed");
        slint::CloseRequestResponse::HideWindow
    });

    // About sits on the top level menu, and that placement is a licence condition. Slint's
    // Royalty-free licence wants the AboutSlint widget in an About screen "accessible from the top
    // level menu of the Application"; this app has no application menu bar, being an accessory, so
    // the status item's menu is its top level menu. Reaching About only by opening Settings and
    // then finding a tab would rest on reading "accessible from" loosely. See NOTICE.
    //
    // Where About sits in the menu does not matter to the licence, only that it is on it, so it goes
    // below the separator beside Quit where the things that are not about tracking time belong.
    //
    // When Pause and Resume arrive they go first, per the rule in docs/rust-port.md that the menu is
    // the primary route to everything and left click is only an accelerator for its first item.
    // Pause and Lock are first, per the rule in docs/rust-port.md that the menu is the primary route
    // to everything and left click is only an accelerator for its first item.
    //
    // **In-memory state, and it is the exception being flagged rather than the rule being broken.**
    // These two facts belong in the database and will be read from it at the point of use like
    // everything else. There is no database yet, so this holds them to demonstrate that the icon
    // follows the state; it is the demonstration that is temporary, not the icon.
    let menu = Menu::new();
    let pause_item = MenuItem::with_id("pause", "Pause", true, None);
    let lock_item = MenuItem::with_id("lock", "Lock", true, None);
    let settings_item = MenuItem::with_id("settings", "Settings...", true, None);
    let about_item = MenuItem::with_id("about", "About Facet", true, None);
    let quit_item = MenuItem::with_id("quit", "Quit Facet", true, None);
    menu.append(&pause_item)?;
    menu.append(&lock_item)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&settings_item)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&about_item)?;
    menu.append(&quit_item)?;

    let showing = Rc::new(Cell::new(status_icon::Showing { paused: false, locked: false }));

    // Right click makes the host show the menu; left click reaches the app instead. That split is the
    // shape every platform can manage, and it is why nothing may live behind a left click that has no
    // menu equivalent: on some Linux desktops the app never receives one. See docs/rust-port.md.
    let tray = TrayIconBuilder::new()
        .with_id("facet-status-item")
        .with_menu(Box::new(menu))
        .with_icon(status_icon::draw(showing.get())?)
        // **Not a template.** A template image is alpha only: macOS throws the colours away and draws
        // the shape in the menu bar's own ink, which is why the icon came out black whatever it was
        // given. False is what lets the glyph colours through, at the cost of them no longer adapting
        // to a light or dark menu bar. See status_icon.
        .with_icon_as_template(false)
        .with_tooltip("Facet")
        .with_menu_on_left_click(false)
        .with_menu_on_right_click(true)
        .build()?;

    // tray-icon sets no accessibility identifier of its own, which is what docs/rust-port.md called
    // out as the one real line item against a suite whose front door is the status item. It does hand
    // over the NSStatusItem, so the identifier can be set directly and scripts/status-item-click.py
    // keeps working unchanged.
    name_the_status_item(&tray);

    // The status item is removed from the menu bar the moment it is dropped, so it has to outlive
    // the event loop rather than the function that built it.
    let _tray: TrayIcon = tray;


    let ui_weak = ui.as_weak();
    let tray_handle = Rc::new(_tray);
    let pump_tray = Rc::clone(&tray_handle);
    let pump_showing = Rc::clone(&showing);
    let pump = slint::Timer::default();
    pump.start(slint::TimerMode::Repeated, TRAY_POLL, move || {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.as_ref() {
                "pause" => {
                    let mut next = pump_showing.get();
                    next.paused = !next.paused;
                    pump_showing.set(next);
                    pause_item.set_text(if next.paused { "Resume" } else { "Pause" });
                    redraw_status_item(&pump_tray, next);
                }
                "lock" => {
                    let mut next = pump_showing.get();
                    next.locked = !next.locked;
                    pump_showing.set(next);
                    lock_item.set_text(if next.locked { "Unlock" } else { "Lock" });
                    redraw_status_item(&pump_tray, next);
                }
                "about" => {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.invoke_open_on_about();
                        show_settings(&ui, "About");
                    }
                }
                "settings" => {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.invoke_open_on_faces();
                        show_settings(&ui, "Faces");
                    }
                }
                "quit" => {
                    println!("[quit    ] Quitting on the menu item");
                    let _ = slint::quit_event_loop();
                }
                other => {
                    // Nothing fails silently: an id with no arm is a menu item somebody added and
                    // did not wire up, and it should say so rather than doing nothing.
                    eprintln!("[menu    ] No handler for menu item id {other}");
                }
            }
        }

        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click { button, button_state, .. } = event {
                println!("[tray    ] status item {button:?} {button_state:?}");
            }
        }
    });

    // Once more, now that the windowing layer has finished starting up.
    let settle = slint::Timer::default();
    settle.start(slint::TimerMode::SingleShot, Duration::from_millis(0), || {
        show_in_dock(false);
    });

    println!("[launch  ] Facet is in the menu bar. Right click the icon for the menu.");

    // Not `ui.run()`: the window is not shown at launch, and the loop must outlive it being closed.
    slint::run_event_loop_until_quit()?;
    Ok(())
}

/// Brings the Settings window up on `tab` and puts the app in front of whatever had focus.
///
/// An accessory app is not activated by showing a window, so without the activation the window
/// appears behind the frontmost application and looks as though the menu item did nothing.
fn show_settings(ui: &SettingsWindow, tab: &str) {
    // **Before the window, not after.** The Dock icon and the window are the same act to macOS: the policy
    // is what decides whether the app has a place in the Dock at all, and changing it out from under a
    // window already on screen leaves that window belonging to an app the Dock has only just heard of.
    show_in_dock(true);

    if let Err(error) = ui.show() {
        eprintln!("[settings] The Settings window could not be shown: {error}");
        // Back out of the Dock, or the app sits there advertising a window that never appeared.
        show_in_dock(false);
        return;
    }
    ui.window().set_maximized(false);
    activate_app();
    // Reported here rather than left to the tab callback, which does not fire for a tab that is
    // already selected, and since every ordinary open lands on Faces that is most opens.
    println!("[settings] Settings opened on {tab}");
}

/// Puts the app in the Dock, or takes it out again.
///
/// **There is no separate switch for a Dock icon on macOS: it is the activation policy.** `Accessory` is what
/// a menu bar app is, and giving up the Dock is part of what it buys. So the policy is flipped rather than set
/// once, `Regular` for as long as a window is on screen and `Accessory` again the moment the last one closes,
/// which is how a Dock icon can appear beside the Settings window and be gone with it.
///
/// **`Regular` also gives the app a menu bar of its own**, which does not change where About lives. The Slint
/// Royalty-free licence wants it reachable from the top level menu, and the status item's menu is that menu
/// whether or not a window happens to be open. See NOTICE.
#[cfg(target_os = "macos")]
fn show_in_dock(wanted: bool) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("[launch  ] Not on the main thread, so the activation policy was left alone");
        return;
    };
    let policy = if wanted {
        NSApplicationActivationPolicy::Regular
    } else {
        NSApplicationActivationPolicy::Accessory
    };
    if !NSApplication::sharedApplication(mtm).setActivationPolicy(policy) {
        // A refusal here is the app being in the Dock when it should not be, or out of it when it should be.
        // Neither loses anything, and both look like a fault nobody caused, so it says so.
        eprintln!("[launch  ] macOS refused the activation policy, so the Dock icon is not what it should be");
    }
}

#[cfg(not(target_os = "macos"))]
fn show_in_dock(_wanted: bool) {}

/// Gives the status item's button an accessibility identifier, so a script can find it by name.
///
/// The identifier is `status-item` because that is what `scripts/status-item-click.py` already looks
/// for: the locator model converts rather than being reinvented.
#[cfg(target_os = "macos")]
fn name_the_status_item(tray: &TrayIcon) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSAccessibility;
    use objc2_foundation::NSString;

    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("[launch  ] Not on the main thread, so the status item was not named");
        return;
    };
    let Some(item) = tray.ns_status_item() else {
        eprintln!("[launch  ] No NSStatusItem came back, so the status item has no identifier");
        return;
    };
    let Some(button) = item.button(mtm) else {
        eprintln!("[launch  ] The status item has no button, so it cannot carry an identifier");
        return;
    };
    button.setAccessibilityIdentifier(Some(&NSString::from_str("status-item")));
}

#[cfg(not(target_os = "macos"))]
fn name_the_status_item(_tray: &TrayIcon) {}

#[cfg(target_os = "macos")]
fn activate_app() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;

    if let Some(mtm) = MainThreadMarker::new() {
        NSApplication::sharedApplication(mtm).activate();
    }
}

#[cfg(not(target_os = "macos"))]
fn activate_app() {}

/// Redraws the status item for `showing`.
///
/// **Says so when it cannot**, rather than leaving the menu bar showing the previous state. An icon
/// that silently stops following the app is worse than no icon: it is a confident wrong answer, and
/// this is the one surface that has to be right on all three platforms.
fn redraw_status_item(tray: &TrayIcon, showing: status_icon::Showing) {
    match status_icon::draw(showing) {
        Ok(icon) => {
            if let Err(error) = tray.set_icon(Some(icon)) {
                eprintln!("[tray    ] The status item icon could not be changed: {error}");
                return;
            }
            println!(
                "[tray    ] Status item now shows paused={} locked={}",
                showing.paused, showing.locked
            );
        }
        Err(error) => eprintln!("[tray    ] The status item icon could not be drawn: {error}"),
    }
}
