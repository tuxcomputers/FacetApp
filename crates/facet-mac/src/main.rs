//! Composition root for macOS. The only place that knows both a port and the thing that performs it.
//!
//! Today that is a status item, a menu, the Settings window and the two databases behind them. There is
//! no radio yet. What it does establish is the shape the rest hangs off, and the two rules that are easy
//! to get wrong later: the menu is the primary route to everything, and left click is an accelerator
//! rather than a mechanism.
//!
//! **Where the files live is decided here and nowhere else.** `facet-core` is handed paths; it does not
//! know which platform laid them out, and asking it to would be the core caring what it is running on.

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use facet_core::database;
use facet_core::debug_log::{DebugLog, Record, Tag};
use facet_core::setting;
use facet_ui::categories::Categories;
use facet_ui::faces::Faces;
use facet_ui::notice::Notice;
use facet_ui::report::Report;
use facet_ui::{ComponentHandle, SettingsWindow};
use tray_icon::{
    MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

mod status_icon;

/// How often the tray's event channels are drained, on the UI thread.
///
/// tray-icon delivers events on crossbeam channels rather than through the Slint event loop, so
/// something has to pump them. Polling on the main thread is what keeps every handler free to touch
/// the window directly; the alternative, a `Send + Sync` callback, cannot hold a Slint handle at all.
const TRAY_POLL: Duration = Duration::from_millis(100);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // **Before the window**, so that a database that will not come up says so in a terminal rather than
    // from behind a status item nobody has clicked yet.
    //
    // **Shared rather than copied**, there being one trace database and one connection to it. Rc because
    // everything that records is on the UI thread; the day something off-thread needs to, it gets a
    // channel to this one rather than a second connection.
    let log = Rc::new(open_databases()?);

    let ui = SettingsWindow::new()?;

    // One notice for the whole window, shared by every tab that raises one.
    let notice = Notice::attach(&ui);

    // `true` for has_given_up_on_cube: this build has no radio, so it never waits for a cube.
    let faces = Faces::attach(
        &ui,
        data_directory().join("appdata.sqlite"),
        Rc::clone(&log),
        true,
        Rc::clone(&notice),
    );

    let categories =
        Categories::attach(&ui, data_directory().join("appdata.sqlite"), Rc::clone(&log), Rc::clone(&notice));
    // A change on the Categories tab can change what the Faces tab and the menu bar show.
    let changed_faces = Rc::downgrade(&faces);
    categories.set_on_changed(move || {
        if let Some(faces) = changed_faces.upgrade() {
            faces.refresh();
        }
    });

    let report = Report::attach(&ui, data_directory().join("appdata.sqlite"), Rc::clone(&log));
    // A time entry recorded while the Report is on screen changes its figures.
    let changed_report = Rc::downgrade(&report);
    faces.set_on_timing_changed(move || {
        if let Some(report) = changed_report.upgrade() {
            report.refresh_if_showing();
        }
    });

    // A menu bar app owns no dock icon. This has to happen after Slint has built its backend, because
    // that is what creates the application object, and again from inside the event loop below, because
    // the windowing layer sets its own policy on the way up.
    show_in_dock(false, &log);

    let tab_log = Rc::clone(&log);
    let tab_faces = Rc::clone(&faces);
    let tab_categories = Rc::clone(&categories);
    let tab_report = Rc::clone(&report);
    ui.on_tab_selected(move |tab| {
        // The scripted suite reads a message of exactly this shape to prove that selecting a tab did
        // something, so the wording is interface.
        tab_log.record(Tag::Settings, || format!("Settings tab selected: {tab}"));
        if tab == "Faces" {
            tab_faces.refresh();
        }
        if tab == "Categories" {
            tab_categories.refresh();
        }
        if tab == "Report" {
            tab_report.refresh();
        }
    });

    // Closing Settings puts the app back in the menu bar and nowhere else.
    //
    // **Hidden rather than destroyed**, which is what makes the next open cheap and is why this callback
    // has to say so: the default for a Slint window is to close it, and a closed window cannot be shown
    // again. The values it holds are not carried over, whatever it keeps in memory: the window reads them
    // again on every open, per the source-of-truth rule in CLAUDE.md.
    let close_log = Rc::clone(&log);
    ui.window().on_close_requested(move || {
        show_in_dock(false, &close_log);
        close_log.record(Tag::Settings, || "Settings closed".to_string());
        slint::CloseRequestResponse::HideWindow
    });

    // About sits on the top level menu, and that placement is a licence condition. Slint's
    // Royalty-free licence wants the AboutSlint widget in an About screen "accessible from the top
    // level menu of the Application"; this app has no application menu bar, being an accessory, so
    // the status item's menu is its top level menu. Reaching About only by opening Settings and
    // then finding a tab would rest on reading "accessible from" loosely. See NOTICE.md.
    //
    // Where About sits in the menu does not matter to the licence, only that it is on it, so it goes
    // below the separator beside Quit where the things that are not about tracking time belong.
    //
    // When Pause and Resume arrive they go first, per the rule in docs/rust-port.md that the menu is
    // the primary route to everything and left click is only an accelerator for its first item.
    // Pause and Lock are first, per the rule in docs/rust-port.md that the menu is the primary route
    // to everything and left click is only an accelerator for its first item.
    //
    // Pause is the app's own clock, read from the database through `faces`. Lock is the cube's and there
    // is no radio yet, so `showing.locked` is held in memory and changes only the icon.
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
    name_the_status_item(&tray, &log);

    // The status item is removed from the menu bar the moment it is dropped, so it has to outlive
    // the event loop rather than the function that built it.
    let _tray: TrayIcon = tray;

    let ui_weak = ui.as_weak();
    let tray_handle = Rc::new(_tray);
    let pump_tray = Rc::clone(&tray_handle);
    let pump_showing = Rc::clone(&showing);
    let pump_log = Rc::clone(&log);
    let pump_faces = Rc::clone(&faces);
    let pump_categories = Rc::clone(&categories);
    let pump_report = Rc::clone(&report);

    // The status item and the Pause item follow the clock: redrawn whenever `faces` re-reads timing, which
    // is after every toggle, every click on the Faces tab and every tick.
    let follow_faces = Rc::downgrade(&faces);
    let follow_tray = Rc::clone(&tray_handle);
    let follow_showing = Rc::clone(&showing);
    let follow_log = Rc::clone(&log);
    let follow_pause_item = pause_item.clone();
    // A change to the icon, the item's title or whether it is enabled writes one row, in the wording the
    // Linux tray writes, which the scripted checks read. The item's current text and enabled state are read
    // from the item itself.
    let follow = move || {
        let Some(timing) = follow_faces.upgrade().and_then(|faces| faces.menu_bar_timing()) else { return };
        let next = status_icon::Showing { paused: timing.is_paused, ..follow_showing.get() };
        let is_item_changed = follow_pause_item.text() != timing.pause_title
            || follow_pause_item.is_enabled() != timing.is_clickable;
        let is_icon_changed = next != follow_showing.get();
        if !is_item_changed && !is_icon_changed {
            return;
        }
        follow_pause_item.set_text(timing.pause_title);
        follow_pause_item.set_enabled(timing.is_clickable);
        if is_icon_changed {
            follow_showing.set(next);
            redraw_status_item(&follow_tray, next, &follow_log);
        }
        follow_log.record(Tag::Tray, || {
            format!(
                "Status item follows the clock, paused={} item={} enabled={}",
                timing.is_paused, timing.pause_title, timing.is_clickable
            )
        });
    };
    follow();
    faces.set_on_timing_changed(follow);

    let pump = slint::Timer::default();
    pump.start(slint::TimerMode::Repeated, TRAY_POLL, move || {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.as_ref() {
                // The same call as the Faces tab's glyph and the left click, so the three cannot disagree.
                "pause" => pump_faces.toggle_pause(),
                "lock" => {
                    let mut next = pump_showing.get();
                    next.locked = !next.locked;
                    pump_showing.set(next);
                    lock_item.set_text(if next.locked { "Unlock" } else { "Lock" });
                    redraw_status_item(&pump_tray, next, &pump_log);
                }
                "about" => {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.invoke_open_on_about();
                        show_settings(&ui, "About", &pump_log);
                        pump_faces.refresh();
                        pump_categories.refresh();
                        pump_report.open();
                    }
                }
                "settings" => {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.invoke_open_on_faces();
                        show_settings(&ui, "Faces", &pump_log);
                        pump_faces.refresh();
                        pump_categories.refresh();
                        pump_report.open();
                    }
                }
                "quit" => {
                    pump_log.record(Tag::Quit, || "Quitting on the menu item".to_string());
                    pump_faces.quit();
                    // Not a discarded Result: a quit that the loop refuses leaves the app running with
                    // nothing said about why, which is the shape CLAUDE.md has a section about. The Linux
                    // composition root reports the same failure the same way.
                    if let Err(error) = slint::quit_event_loop() {
                        pump_log
                            .record_failure(Tag::Quit, || format!("The event loop refused to quit: {error}"));
                    }
                }
                other => {
                    // Nothing fails silently: an id with no arm is a menu item somebody added and
                    // did not wire up, and it should say so rather than doing nothing.
                    pump_log.record_failure(Tag::Menu, || format!("No handler for menu item id {other}"));
                }
            }
        }

        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click { button, button_state, .. } = event {
                pump_log.record(Tag::Tray, || format!("Status item {button:?} {button_state:?}"));

                // **Left click is Pause's accelerator**, which is what it already is on Linux and what
                // the design rule in docs/rust-port.md asks for on every platform: the menu is the
                // primary route and left click is a shortcut to its first item.
                //
                // **On the release, not the press.** macOS delivers both edges of a left click here, so
                // acting on each would flip pause twice and land back where it started. Right click never
                // arrives as a pair, the menu taking it, so this is not a general rule about clicks.
                if button == MouseButton::Left && button_state == MouseButtonState::Up {
                    pump_faces.toggle_pause();
                }
            }
        }
    });

    // Once more, now that the windowing layer has finished starting up.
    let settle_log = Rc::clone(&log);
    let settle = slint::Timer::default();
    settle.start(slint::TimerMode::SingleShot, Duration::from_millis(0), move || {
        show_in_dock(false, &settle_log);
    });

    log.record(Tag::Launch, || "Facet is in the menu bar. Right click the icon for the menu".to_string());

    // Not `ui.run()`: the window is not shown at launch, and the loop must outlive it being closed.
    slint::run_event_loop_until_quit()?;
    Ok(())
}

/// Brings the Settings window up on `tab` and puts the app in front of whatever had focus.
///
/// An accessory app is not activated by showing a window, so without the activation the window
/// appears behind the frontmost application and looks as though the menu item did nothing.
fn show_settings(ui: &SettingsWindow, tab: &str, log: &Option<DebugLog>) {
    // **Before the window, not after.** The Dock icon and the window are the same act to macOS: the policy
    // is what decides whether the app has a place in the Dock at all, and changing it out from under a
    // window already on screen leaves that window belonging to an app the Dock has only just heard of.
    show_in_dock(true, log);

    if let Err(error) = ui.show() {
        log.record_failure(Tag::Settings, || format!("The Settings window could not be shown: {error}"));
        // Back out of the Dock, or the app sits there advertising a window that never appeared.
        show_in_dock(false, log);
        return;
    }
    ui.window().set_maximized(false);
    activate_app();
    // Reported here rather than left to the tab callback, which does not fire for a tab that is
    // already selected, and since every ordinary open lands on Faces that is most opens.
    log.record(Tag::Settings, || format!("Settings opened on {tab}"));
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
/// whether or not a window happens to be open. See NOTICE.md.
#[cfg(target_os = "macos")]
fn show_in_dock(wanted: bool, log: &Option<DebugLog>) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    let Some(mtm) = MainThreadMarker::new() else {
        log.record_failure(Tag::Launch, || {
            "Not on the main thread, so the activation policy was left alone".to_string()
        });
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
        log.record_failure(Tag::Launch, || {
            "macOS refused the activation policy, so the Dock icon is not what it should be".to_string()
        });
        return;
    }

    // **After the policy, never before.** The tile does not exist while the app is Accessory, so an icon set
    // at launch is set on nothing: the tile macOS then creates on the way to Regular comes up wearing the
    // generic executable placeholder instead. Measured by doing exactly that.
    if wanted {
        wear_the_facet_logo(mtm, log);
    }
}

#[cfg(not(target_os = "macos"))]
fn show_in_dock(_wanted: bool, _log: &Option<DebugLog>) {}

/// Gives the status item's button an accessibility identifier, so a script can find it by name.
///
/// The identifier is `status-item` because that is what `scripts/status-item-click.py` already looks
/// for: the locator model converts rather than being reinvented.
#[cfg(target_os = "macos")]
fn name_the_status_item(tray: &TrayIcon, log: &Option<DebugLog>) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSAccessibility;
    use objc2_foundation::NSString;

    let Some(mtm) = MainThreadMarker::new() else {
        log.record_failure(Tag::Launch, || {
            "Not on the main thread, so the status item was not named".to_string()
        });
        return;
    };
    let Some(item) = tray.ns_status_item() else {
        log.record_failure(Tag::Launch, || {
            "No NSStatusItem came back, so the status item has no identifier".to_string()
        });
        return;
    };
    let Some(button) = item.button(mtm) else {
        log.record_failure(Tag::Launch, || {
            "The status item has no button, so it cannot carry an identifier".to_string()
        });
        return;
    };
    button.setAccessibilityIdentifier(Some(&NSString::from_str("status-item")));
}

#[cfg(not(target_os = "macos"))]
fn name_the_status_item(_tray: &TrayIcon, _log: &Option<DebugLog>) {}

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
fn redraw_status_item(tray: &TrayIcon, showing: status_icon::Showing, log: &Option<DebugLog>) {
    match status_icon::draw(showing) {
        Ok(icon) => {
            if let Err(error) = tray.set_icon(Some(icon)) {
                log.record_failure(Tag::Tray, || {
                    format!("The status item icon could not be changed: {error}")
                });
                return;
            }
            log.record(Tag::Tray, || {
                format!("Status item now shows paused={} locked={}", showing.paused, showing.locked)
            });
        }
        Err(error) => {
            log.record_failure(Tag::Tray, || format!("The status item icon could not be drawn: {error}"))
        }
    }
}

/// Where this Mac keeps Facet's files.
///
/// **A platform fact, and so it lives in the platform crate.** `facet-core` is handed paths and never asks
/// which layout produced them; `~/Library/Application Support/Facet` is this machine's answer and
/// `~/.local/share/Facet` is the Linux one, and neither belongs in a crate that must not be able to tell
/// which it is running on.
fn data_directory() -> PathBuf {
    // $HOME rather than NSFileManager, so this is the same answer a shell script gets. Tests/Scripted and
    // scripts/run.sh both resolve it that way through Tests/Scripted/platform.sh, and a suite that looked
    // in a different directory from the app would be checking a database nobody had written to.
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join("Library/Application Support/Facet")
}

/// Brings up the app's database and, if the setting says so, the trace beside it.
///
/// **Reads the setting rather than being told.** Whether the trace is gathered is a row in `setting`, and
/// this is the one place that asks: the gate is here, so a launch with logging off holds no logger at all
/// rather than one that returns early on every call.
///
/// The app database is opened even when nothing is going to be recorded, because it is what says whether
/// anything should be. Its connection is then dropped: nothing reads it yet, and holding one open would be
/// this app keeping a file the Swift one may also want.
fn open_databases() -> Result<Option<DebugLog>, Box<dyn std::error::Error>> {
    let directory = data_directory();
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("{} could not be created: {error}", directory.display()))?;

    // **appdata.sqlite, which is usually a symlink**, and opening it follows the link. That is the whole
    // mechanism behind scripts/switch-database.sh: the app opens one name and sqlite resolves which
    // physical file it is, so switching databases moves no data and needs nothing from the app.
    let appdata = directory.join("appdata.sqlite");
    let connection = database::open(&appdata, database::APPDATA_DDL)?;

    let which = setting::database_type(&connection)?;
    let trace = setting::debug_trace(&connection)?;
    drop(connection);

    if !trace.enabled {
        // Said on stderr rather than recorded, there being nowhere to record it. It is the one message a
        // launch with logging off should still produce, because otherwise an empty table and a launch that
        // was never asked to write one look identical.
        eprintln!(
            "[launch  ] Logging is off in the {which} database. Turn on debug.enabled in setting to record a trace."
        );
        return Ok(None);
    }

    // An empty directory means the folder the app already keeps its databases in, which cannot be seeded as
    // a path because it differs per platform. A leading ~ is expanded here, at the point the file is
    // opened, and never stored expanded: an absolute path names one machine and this database is copied
    // between them.
    let folder = match trace.directory.as_str() {
        "" => directory.clone(),
        stored => expand_home(stored),
    };
    std::fs::create_dir_all(&folder)
        .map_err(|error| format!("{} could not be created: {error}", folder.display()))?;

    let file = folder.join("debug.sqlite");
    let log = DebugLog::open(&file)?;
    let log = Some(log);
    log.record(Tag::Database, || format!("Trace open at {}, against the {which} database", file.display()));
    Ok(log)
}

/// A stored path with its leading `~` turned into this machine's home.
///
/// Only a leading `~/`, and only that: a bare `~` or a `~user` form is left alone rather than guessed at,
/// since neither is something this app writes and both would be a guess about somebody else's directory.
fn expand_home(stored: &str) -> PathBuf {
    match stored.strip_prefix("~/") {
        Some(rest) => match std::env::var("HOME") {
            Ok(home) => PathBuf::from(home).join(rest),
            Err(_) => PathBuf::from(stored),
        },
        None => PathBuf::from(stored),
    }
}

/// Puts the Facet logo on the Dock icon.
///
/// **A running binary has no icon of its own.** An icon normally comes from an app bundle's `Info.plist`,
/// and this is a bare executable launched from a terminal, so without this the Dock shows the generic
/// placeholder. Setting `applicationIconImage` is what a bundle would otherwise do, and it keeps working
/// once there is a bundle, so nothing here has to be undone then.
///
/// The image is the same `Facet.svg` the rest of the project uses, rasterised at build time: one drawing,
/// and no PNG in the repository that can drift from it.
///
/// No stub for the other platforms: a Dock is a macOS object, and the only caller is the macOS half of
/// [`show_in_dock`].
#[cfg(target_os = "macos")]
fn wear_the_facet_logo(mtm: objc2::MainThreadMarker, log: &Option<DebugLog>) {
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    /// Rasterised from Facet.svg by build.rs.
    const LOGO: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/dock-icon.png"));

    let data = NSData::with_bytes(LOGO);
    let Some(image) = NSImage::initWithData(objc2::AllocAnyThread::alloc(), &data) else {
        log.record_failure(Tag::Launch, || {
            "The Facet logo could not be read, so the Dock icon is the generic one".to_string()
        });
        return;
    };
    // Unsafe only because AppKit does not promise this is main-thread-only in its annotations. The
    // MainThreadMarker above is the proof that it is being called from the right thread, which is the whole
    // of what the setter needs.
    unsafe {
        NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&image));
    }
}
