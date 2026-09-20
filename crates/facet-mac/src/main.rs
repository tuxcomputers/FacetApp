//! Composition root for macOS. The only place that knows both a port and the thing that performs it.
//!
//! Today that is barely anything: a status item, a menu, and the Settings window. There is no radio,
//! no database and no core behind it yet. What it does establish is the shape the rest hangs off, and
//! the two rules that are easy to get wrong later: the menu is the primary route to everything, and
//! left click is an accelerator rather than a mechanism.

use std::time::Duration;

use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

slint::include_modules!();

/// How often the tray's event channels are drained, on the UI thread.
///
/// tray-icon delivers events on crossbeam channels rather than through the Slint event loop, so
/// something has to pump them. Polling on the main thread is what keeps every handler free to touch
/// the window directly; the alternative, a `Send + Sync` callback, cannot hold a Slint handle at all.
const TRAY_POLL: Duration = Duration::from_millis(100);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = SettingsWindow::new()?;

    // A menu bar app owns no dock icon and no menu bar of its own. This has to happen after Slint has
    // built its backend, because that is what creates the application object, and again from inside
    // the event loop below, because the windowing layer sets its own policy on the way up.
    set_accessory_activation_policy();

    ui.on_tab_selected(|tab| {
        // Stands in for the debug log until there is one. The scripted suite reads a line of exactly
        // this shape to prove that selecting a tab did something, so the wording is interface.
        println!("[settings] Settings tab selected: {tab}");
    });

    let menu = Menu::new();
    let settings_item = MenuItem::with_id("settings", "Settings...", true, None);
    let quit_item = MenuItem::with_id("quit", "Quit Facet", true, None);
    menu.append(&settings_item)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&quit_item)?;

    // Right click makes the host show the menu; left click reaches the app instead. That split is the
    // shape every platform can manage, and it is why nothing may live behind a left click that has no
    // menu equivalent: on some Linux desktops the app never receives one. See docs/rust-port.md.
    let tray = TrayIconBuilder::new()
        .with_id("facet-status-item")
        .with_menu(Box::new(menu))
        .with_icon(status_item_icon()?)
        .with_icon_as_template(true)
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
    let pump = slint::Timer::default();
    pump.start(slint::TimerMode::Repeated, TRAY_POLL, move || {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.as_ref() {
                "settings" => {
                    if let Some(ui) = ui_weak.upgrade() {
                        show_settings(&ui);
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
        set_accessory_activation_policy();
    });

    println!("[launch  ] Facet is in the menu bar. Right click the icon for the menu.");

    // Not `ui.run()`: the window is not shown at launch, and the loop must outlive it being closed.
    slint::run_event_loop_until_quit()?;
    Ok(())
}

/// Brings the Settings window up and puts the app in front of whatever had focus.
///
/// An accessory app is not activated by showing a window, so without the activation the window
/// appears behind the frontmost application and looks as though the menu item did nothing.
fn show_settings(ui: &SettingsWindow) {
    if let Err(error) = ui.show() {
        eprintln!("[settings] The Settings window could not be shown: {error}");
        return;
    }
    ui.window().set_maximized(false);
    activate_app();
    println!("[settings] Settings window opened");
}

#[cfg(target_os = "macos")]
fn set_accessory_activation_policy() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("[launch  ] Not on the main thread, so the activation policy was left alone");
        return;
    };
    NSApplication::sharedApplication(mtm)
        .setActivationPolicy(NSApplicationActivationPolicy::Accessory);
}

#[cfg(not(target_os = "macos"))]
fn set_accessory_activation_policy() {}

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

/// A placeholder status item icon: a rounded square outline with a dot, drawn in code.
///
/// Alpha is the whole of it, because it is installed as a template image and macOS then draws it in
/// whichever colour the menu bar needs. The real artwork is `Facet.svg`, which wants an SVG
/// rasteriser this binary does not yet pull in.
fn status_item_icon() -> Result<Icon, tray_icon::BadIcon> {
    const SIZE: i32 = 32;
    const INSET: i32 = 5;
    const STROKE: i32 = 3;

    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    let centre = (SIZE as f32 - 1.0) / 2.0;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let on_border = (x >= INSET && x < SIZE - INSET && y >= INSET && y < SIZE - INSET)
                && (x < INSET + STROKE
                    || x >= SIZE - INSET - STROKE
                    || y < INSET + STROKE
                    || y >= SIZE - INSET - STROKE);

            let dx = x as f32 - centre;
            let dy = y as f32 - centre;
            let in_dot = dx * dx + dy * dy <= 12.0;

            // Knock the four corners off the border so it reads as rounded rather than boxy.
            let corner = |cx: i32, cy: i32| (x - cx).abs() + (y - cy).abs() < 3;
            let clipped = corner(INSET, INSET)
                || corner(SIZE - 1 - INSET, INSET)
                || corner(INSET, SIZE - 1 - INSET)
                || corner(SIZE - 1 - INSET, SIZE - 1 - INSET);

            if (on_border && !clipped) || in_dot {
                let i = ((y * SIZE + x) * 4) as usize;
                rgba[i] = 0;
                rgba[i + 1] = 0;
                rgba[i + 2] = 0;
                rgba[i + 3] = 255;
            }
        }
    }

    Icon::from_rgba(rgba, SIZE as u32, SIZE as u32)
}
