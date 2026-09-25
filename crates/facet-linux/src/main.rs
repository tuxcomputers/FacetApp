//! Composition root for Linux. The only place that knows both a port and the thing that performs it.
//!
//! Today that is a tray icon, a menu, the Settings window and the two databases behind them. There is no
//! radio yet. What it does establish is the shape the rest hangs off, and the two rules that are easy to
//! get wrong later: the menu is the primary route to everything, and left click is an accelerator rather
//! than a mechanism.
//!
//! **Where the files live is decided here and nowhere else.** `facet-core` is handed paths; it does not
//! know which platform laid them out, and asking it to would be the core caring what it is running on.
//! `crates/facet-mac/src/main.rs` is the worked example this follows, and the only line in it that was
//! really about macOS was the directory.
//!
//! **The tray runs on its own thread and the window does not.** `ksni::Tray` is `Send + 'static` and a
//! Slint handle is neither, so the tray cannot hold the window and does not try: it posts messages and the
//! pump below acts on them on the UI thread. That is the same arrangement the Mac arrives at from the
//! other direction, its crate delivering events on a channel that something has to drain.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use facet_core::database;
use facet_core::debug_log::{DebugLog, Record, Tag};
use facet_core::setting;
use facet_ui::faces::Faces;
use facet_ui::{ComponentHandle, SettingsWindow};
use ksni::blocking::{Handle, TrayMethods};

mod tray;

use tray::{FacetTray, FromTray};

/// How often the tray's channel is drained, on the UI thread.
///
/// The tray thread cannot touch the window, so something on the UI thread has to pick its messages up.
/// **The same interval the Mac polls its own tray crate on**, there being no reason for the two platforms
/// to feel different.
const TRAY_POLL: Duration = Duration::from_millis(100);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // **Before the window**, so that a database that will not come up says so in a terminal rather than
    // from behind a tray icon nobody has clicked yet.
    //
    // **Shared rather than copied**, there being one trace database and one connection to it. Rc because
    // everything that records is on the UI thread; the tray thread has none and posts messages instead.
    let log = std::rc::Rc::new(open_databases()?);

    let ui = SettingsWindow::new()?;

    // `true` for has_given_up_on_cube: this build has no radio, so it never waits for a cube.
    let faces =
        Faces::attach(&ui, data_directory().join("appdata.sqlite"), std::rc::Rc::clone(&log), true);

    let tab_log = std::rc::Rc::clone(&log);
    let tab_faces = std::rc::Rc::clone(&faces);
    ui.on_tab_selected(move |tab| {
        // The scripted suite reads a message of exactly this shape to prove that selecting a tab did
        // something, so the wording is interface.
        tab_log.record(Tag::Settings, || format!("Settings tab selected: {tab}"));
        if tab == "Faces" {
            tab_faces.refresh();
        }
    });

    // Closing Settings puts the app back in the tray and nowhere else.
    //
    // **Hidden rather than destroyed**, which is what makes the next open cheap and is why this callback
    // has to say so: the default for a Slint window is to close it, and a closed window cannot be shown
    // again. The values it holds are not carried over, whatever it keeps in memory: the window reads them
    // again on every open, per the source-of-truth rule in CLAUDE.md.
    let close_log = std::rc::Rc::clone(&log);
    ui.window().on_close_requested(move || {
        close_log.record(Tag::Settings, || "Settings closed".to_string());
        slint::CloseRequestResponse::HideWindow
    });

    let (to_ui, from_tray) = std::sync::mpsc::channel();
    // **Held for the life of the process.** The service stops when its handle is dropped, so binding it
    // here is what keeps the item on the panel; dropping it at the end of this statement would put the
    // icon up and take it straight back down. The Mac's `_tray` exists for the same reason.
    let _tray = start_the_tray(to_ui, &log);

    let ui_weak = ui.as_weak();
    let pump_log = std::rc::Rc::clone(&log);
    let pump_faces = std::rc::Rc::clone(&faces);
    let pump = slint::Timer::default();
    pump.start(slint::TimerMode::Repeated, TRAY_POLL, move || {
        drain(&from_tray, &ui_weak, &pump_log, &pump_faces);
    });

    log.record(Tag::Launch, || {
        "Facet is in the tray. Right click the icon for the menu".to_string()
    });

    // Not `ui.run()`: the window is not shown at launch, and the loop must outlive it being closed.
    slint::run_event_loop_until_quit()?;

    // **The handle goes out of scope here and that is the shutdown.** Dropping it stops the service, which
    // closes the D-Bus connection and takes the item off the panel. Calling `Handle::shutdown` first would
    // wait for the same thing to finish and change nothing a user could see.
    Ok(())
}

/// Takes everything the tray thread has posted and acts on it, on the UI thread.
///
/// **The only place tray events meet the window.** Every arm here is free to touch Slint because this runs
/// on the event loop; nothing in `tray.rs` is.
fn drain(
    from_tray: &Receiver<FromTray>,
    ui_weak: &slint::Weak<SettingsWindow>,
    log: &Option<DebugLog>,
    faces: &Faces,
) {
    while let Ok(message) = from_tray.try_recv() {
        match message {
            FromTray::Activated(showing) => {
                log.record(Tag::Tray, || {
                    format!(
                        "Status item left clicked, now showing paused={} locked={}",
                        showing.paused, showing.locked
                    )
                });
            }
            FromTray::SecondaryActivated => {
                log.record(Tag::Tray, || "Status item middle clicked".to_string());
            }
            FromTray::Changed(showing) => {
                log.record(Tag::Tray, || {
                    format!(
                        "Status item now shows paused={} locked={}",
                        showing.paused, showing.locked
                    )
                });
            }
            FromTray::OpenSettings => {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.invoke_open_on_faces();
                    show_settings(&ui, "Faces", log);
                    faces.refresh();
                }
            }
            FromTray::OpenAbout => {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.invoke_open_on_about();
                    show_settings(&ui, "About", log);
                    faces.refresh();
                }
            }
            FromTray::Quit => {
                log.record(Tag::Quit, || "Quitting on the menu item".to_string());
                faces.quit();
                if let Err(error) = slint::quit_event_loop() {
                    log.record_failure(Tag::Quit, || {
                        format!("The event loop refused to quit: {error}")
                    });
                }
            }
        }
    }
}

/// Brings the Settings window up on `tab`.
///
/// **No activation policy to flip on the way**, which is the one thing the Mac does here and this does not:
/// a Dock is a macOS object and there is no Linux equivalent to take the app in and out of.
fn show_settings(ui: &SettingsWindow, tab: &str, log: &Option<DebugLog>) {
    if let Err(error) = ui.show() {
        log.record_failure(Tag::Settings, || {
            format!("The Settings window could not be shown: {error}")
        });
        return;
    }
    ui.window().set_maximized(false);
    // Reported here rather than left to the tab callback, which does not fire for a tab that is already
    // selected, and since every ordinary open lands on Faces that is most opens.
    log.record(Tag::Settings, || format!("Settings opened on {tab}"));
}

/// Puts the status item on the panel.
///
/// **Says which way it failed, because the three failures want different answers from a reader.** No D-Bus
/// at all is a broken session; no watcher is a desktop with no StatusNotifierItem support; and `WontShow`
/// is the item registering with nothing to display it, which on a desktop still starting up can even come
/// right on its own. A single "the tray did not start" would send all three to the same wrong place.
///
/// **A failure is reported and the app continues.** A tray that never appears leaves the window
/// unreachable, which is a real fault, and it is one the trace should carry rather than one that stops the
/// launch before there is a trace to carry it.
///
/// **`ksni::blocking`, so there is no async runtime in this crate.** The service still runs on a thread of
/// its own -- which is why [`FacetTray`] is `Send` and posts messages rather than touching the window --
/// but starting it is an ordinary call that either works or does not.
fn start_the_tray(to_ui: Sender<FromTray>, log: &Option<DebugLog>) -> Option<Handle<FacetTray>> {
    match FacetTray::new(to_ui).spawn() {
        Ok(handle) => {
            log.record(Tag::Tray, || "Status item is on the panel".to_string());
            Some(handle)
        }
        Err(error) => {
            let why = match error {
                ksni::Error::Dbus(source) => {
                    format!("the session bus could not be reached: {source}")
                }
                ksni::Error::Watcher(source) => format!(
                    "no StatusNotifierWatcher took the item, so this desktop has no SNI support running: {source}"
                ),
                ksni::Error::WontShow => {
                    "the item registered but nothing is displaying it, there being no StatusNotifierHost"
                        .to_string()
                }
                // **`ksni::Error` is non-exhaustive, so this arm is required and is not padding.** A
                // version that adds a case would otherwise reach a reader as one of the three above,
                // which is a wrong diagnosis rather than a missing one. It says it does not know.
                other => format!(
                    "it failed for a reason this build has no case for, which means ksni has gained one: {other}"
                ),
            };
            log.record_failure(Tag::Tray, || format!("There is no status item: {why}"));
            None
        }
    }
}

/// Where this Linux box keeps Facet's files.
///
/// **A platform fact, and so it lives in the platform crate.** `facet-core` is handed paths and never asks
/// which layout produced them; `~/.local/share/Facet` is this machine's answer and
/// `~/Library/Application Support/Facet` is the Mac's, and neither belongs in a crate that must not be able
/// to tell which it is running on.
///
/// **`$HOME` rather than `$XDG_DATA_HOME`, and that is the deliberate choice a Linux reader will query.**
/// The XDG default *is* `$HOME/.local/share`, so the two agree on this box, where no XDG variable is set at
/// all (measured, see `docs/system-linux.md`). They would part company on a machine that set one, and the
/// app is not the only thing that resolves this path: `Tests/Scripted/platform.sh` decides it for the suite
/// and resolves it exactly this way. A suite looking in a different directory from the app would be
/// checking a database nobody had written to, which is the same fault that has already been paid for once
/// when a script hardcoded the macOS directory. **One question, one answer**: if this is ever taught about
/// `XDG_DATA_HOME`, `platform.sh` is taught in the same change.
fn data_directory() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".local/share/Facet")
}

/// Brings up the app's database and, if the setting says so, the trace beside it.
///
/// **Reads the setting rather than being told.** Whether the trace is gathered is a row in `setting`, and
/// this is the one place that asks: the gate is here, so a launch with logging off holds no logger at all
/// rather than one that returns early on every call.
///
/// The app database is opened even when nothing is going to be recorded, because it is what says whether
/// anything should be. Its connection is then dropped: nothing reads it yet, and holding one open would be
/// this app keeping a file something else may also want.
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
    log.record(Tag::Database, || {
        format!("Trace open at {}, against the {which} database", file.display())
    });
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
