//! The App tab's behaviour.
//!
//! The App settings section is the open-window licence in `CLAUDE.md`: [`App::open`] reads every value when
//! the window opens, and from then until it closes the values held here are the answer. A change is written
//! straight through and read back; a stored write is adopted, a refused one puts the row back to the value
//! held here and says so in a notice. A value changed in the table meanwhile is overwritten by the next write.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

use facet_core::app_settings::{self, Value, Values};
use facet_core::debug_log::{Record, Tag, Trace};
use facet_core::port::{FileChooser, Opener};
use facet_core::{database, setting, trace_file};
use rusqlite::Connection;
use slint::ComponentHandle;

use crate::notice::Notice;
use crate::{AppData, SettingsWindow};

/// The App tab, attached to one Settings window.
pub struct App {
    ui: slint::Weak<SettingsWindow>,
    database: PathBuf,
    log: Rc<Trace>,
    notice: Rc<Notice>,
    opener: Rc<dyn Opener>,
    chooser: Rc<dyn FileChooser>,
    /// What the window holds, from [`App::open`] until the window closes.
    held: RefCell<Option<Values>>,
    /// Whether debug logging is on, as the window holds it.
    held_debug: RefCell<Option<bool>>,
    on_changed: RefCell<Vec<Box<dyn Fn()>>>,
    this: RefCell<Weak<App>>,
}

/// One control of the App settings section.
#[derive(Clone, Copy)]
enum Row {
    ShowsSeconds,
    ResetHour,
    FetchMinutes,
    BlipSeconds,
}

impl Row {
    fn title(self) -> &'static str {
        match self {
            Row::ShowsSeconds => "Show seconds",
            Row::ResetHour => "Daily reset at",
            Row::FetchMinutes => "Fetch history every",
            Row::BlipSeconds => "Ignore flips under",
        }
    }
}

impl App {
    /// Wires the tab's callbacks on `ui` to `database`. Call once, at launch.
    pub fn attach(
        ui: &SettingsWindow,
        database: PathBuf,
        log: Rc<Trace>,
        notice: Rc<Notice>,
        opener: Rc<dyn Opener>,
        chooser: Rc<dyn FileChooser>,
    ) -> Rc<App> {
        let app = Rc::new(App {
            ui: ui.as_weak(),
            database,
            log,
            notice,
            opener,
            chooser,
            held: RefCell::new(None),
            held_debug: RefCell::new(None),
            on_changed: RefCell::new(Vec::new()),
            this: RefCell::new(Weak::new()),
        });
        *app.this.borrow_mut() = Rc::downgrade(&app);

        let data = ui.global::<AppData>();
        let weak = Rc::downgrade(&app);
        data.on_section_toggled(move |id, open| {
            if let Some(app) = weak.upgrade() {
                app.log.record(Tag::Settings, || {
                    format!("App section {id} {}", if open { "opened" } else { "folded" })
                });
            }
        });
        let weak = Rc::downgrade(&app);
        data.on_shows_seconds_toggled(move |on| {
            if let Some(app) = weak.upgrade() {
                app.change(Row::ShowsSeconds, i64::from(on));
            }
        });
        let weak = Rc::downgrade(&app);
        data.on_reset_hour_edited(move |face| {
            if let Some(app) = weak.upgrade() {
                app.change(Row::ResetHour, i64::from(face));
            }
        });
        let weak = Rc::downgrade(&app);
        data.on_fetch_minutes_edited(move |minutes| {
            if let Some(app) = weak.upgrade() {
                app.change(Row::FetchMinutes, i64::from(minutes));
            }
        });
        let weak = Rc::downgrade(&app);
        data.on_debug_toggled(move |on| {
            if let Some(app) = weak.upgrade() {
                app.debug_toggled(on);
            }
        });
        let weak = Rc::downgrade(&app);
        data.on_debug_choose_pressed(move || {
            if let Some(app) = weak.upgrade() {
                app.choose_folder();
            }
        });
        let weak = Rc::downgrade(&app);
        data.on_debug_reveal_pressed(move || {
            if let Some(app) = weak.upgrade() {
                app.reveal_trace();
            }
        });
        let weak = Rc::downgrade(&app);
        data.on_debug_copy_pressed(move || {
            if let Some(app) = weak.upgrade() {
                app.copy_trace();
            }
        });
        let weak = Rc::downgrade(&app);
        data.on_debug_clear_pressed(move || {
            if let Some(app) = weak.upgrade() {
                app.clear_trace();
            }
        });
        let weak = Rc::downgrade(&app);
        data.on_blip_seconds_edited(move |seconds| {
            if let Some(app) = weak.upgrade() {
                app.change(Row::BlipSeconds, i64::from(seconds));
            }
        });
        app
    }

    /// Adds something to run after a stored change: what shows seconds, or counts time, reads these.
    pub fn set_on_changed(&self, changed: impl Fn() + 'static) {
        self.on_changed.borrow_mut().push(Box::new(changed));
    }

    /// Reads every value on the tab and puts the sections back to their opening folds. Call when the Settings
    /// window opens.
    pub fn open(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let data = ui.global::<AppData>();
        data.set_app_expanded(true);
        data.set_google_expanded(true);
        data.set_debug_expanded(false);
        let Some(connection) = self.connect() else { return };
        let Some(values) = self.report(app_settings::read(&connection)) else { return };
        *self.held.borrow_mut() = Some(values.clone());
        self.show(&values);
        let Some(debug) = self.report(setting::debug_trace(&connection)) else { return };
        *self.held_debug.borrow_mut() = Some(debug.enabled);
        data.set_debug_enabled(debug.enabled);
        let directory = if debug.directory.is_empty() {
            self.log.file().parent().map(|folder| tilde(&folder.display().to_string())).unwrap_or_default()
        } else {
            debug.directory
        };
        data.set_debug_directory(directory.into());
        self.show_trace();
    }

    /// Enables the trace file's buttons only while the file exists.
    fn show_trace(&self) {
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<AppData>().set_has_debug_trace(self.log.file().is_file());
        }
    }

    /// Writes `debug.enabled` through and, once the table holds it, starts or stops recording.
    fn debug_toggled(&self, on: bool) {
        let Some(held) = *self.held_debug.borrow() else { return };
        let stored = self
            .connect()
            .and_then(|connection| {
                self.report(app_settings::write(
                    &connection,
                    "debug",
                    "enabled",
                    &Value::Flag(on),
                    &*self.log,
                ))
            })
            .unwrap_or(false);
        if !stored {
            if let Some(ui) = self.ui.upgrade() {
                ui.global::<AppData>().set_debug_enabled(held);
            }
            self.refused("Debug logging");
            return;
        }
        *self.held_debug.borrow_mut() = Some(on);
        if let Err(error) = self.log.set_recording(on) {
            self.log.record_failure(Tag::Settings, || {
                format!("Logging could not be turned {}: {error}", if on { "on" } else { "off" })
            });
        }
        self.show_trace();
    }

    /// Asks for a folder and stores it as the trace folder for the next launch.
    fn choose_folder(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let data = ui.global::<AppData>();
        let shown = data.get_debug_directory().to_string();
        let Some(picked) = self.chooser.choose_folder(&untilde(&shown), "Where Facet keeps its debug trace")
        else {
            self.log.record(Tag::Settings, || format!("Debug trace folder left as {shown}"));
            return;
        };
        let stored_as = tilde(&picked.display().to_string());
        let stored = self
            .connect()
            .and_then(|connection| {
                self.report(app_settings::write(
                    &connection,
                    "debug",
                    "directory",
                    &Value::Text(stored_as.clone()),
                    &*self.log,
                ))
            })
            .unwrap_or(false);
        if stored {
            data.set_debug_directory(stored_as.into());
        } else {
            self.refused("Directory");
        }
    }

    /// Whether the trace file exists, saying so in a notice when it does not.
    fn has_trace(&self) -> bool {
        let file = self.log.file();
        if file.is_file() {
            return true;
        }
        self.notice.tell(
            "There is no trace to show",
            &format!(
                "Facet expected its debug trace at {}, and there is no file there.\n\nIt is written as the app \
                 runs, so there is nothing to show until something has been logged.",
                file.display()
            ),
        );
        false
    }

    fn reveal_trace(&self) {
        if !self.has_trace() {
            return;
        }
        let file = self.log.file().to_path_buf();
        self.log.record(Tag::Settings, || format!("Revealing the trace at {}", file.display()));
        if let Err(error) = self.opener.reveal(&file) {
            self.log.record_failure(Tag::Settings, || format!("The trace could not be revealed: {error}"));
            self.notice.tell("The trace could not be shown", &error);
        }
    }

    fn copy_trace(&self) {
        if !self.has_trace() {
            return;
        }
        let Some(connection) = self.connect() else { return };
        let Some(name) = self.report(trace_file::copy_name(&connection, now())) else { return };
        let Some(destination) =
            self.chooser.choose_save_file(&name, "Save a copy of the debug trace to send in")
        else {
            self.log.record(Tag::Settings, || "The trace was not copied".to_string());
            return;
        };
        match trace_file::copy_to(self.log.file(), &destination) {
            Ok(()) => self.log.record(Tag::Settings, || format!("Trace copied to {}", destination.display())),
            Err(error) => {
                self.log.record_failure(Tag::Settings, || format!("The trace was not copied: {error}"));
                self.notice.tell("The trace was not copied", &error);
            }
        }
        self.show_trace();
    }

    fn clear_trace(&self) {
        if !self.has_trace() {
            return;
        }
        let this = self.this.borrow().clone();
        self.notice.ask(
            "Clear the debug trace?",
            "This removes every message Facet has recorded so far. It cannot be undone, and anything you have \
             been asked to send in goes with it.\n\nNothing else is affected: your recorded time, categories and \
             settings are in a different file.",
            &["Cancel", "Clear Trace"],
            move |index| {
                let Some(app) = this.upgrade() else { return };
                if index != 1 {
                    app.log.record(Tag::Settings, || "The trace was not cleared".to_string());
                    return;
                }
                match trace_file::clear(app.log.file()) {
                    Ok(()) => app.log.record(Tag::Settings, || "Trace cleared".to_string()),
                    Err(error) => {
                        app.log.record_failure(Tag::Settings, || format!("The trace was not cleared: {error}"));
                        app.notice.tell("The trace was not cleared", &error);
                    }
                }
                app.show_trace();
            },
        );
    }

    fn refused(&self, title: &str) {
        self.notice.tell(
            "That setting was not saved",
            &format!(
                "The database would not take the new value for \u{201c}{title}\u{201d}, so the setting is unchanged \
                 and the row has gone back to what is stored.\n\nNothing else has been affected. Trying again is safe."
            ),
        );
    }

    fn show(&self, values: &Values) {
        let Some(ui) = self.ui.upgrade() else { return };
        let data = ui.global::<AppData>();
        data.set_shows_seconds(values.shows_seconds);
        data.set_reset_hour(to_i32(values.reset_hour));
        data.set_fetch_minutes(to_i32(values.fetch_minutes));
        data.set_blip_seconds(to_i32(values.blip_seconds));
    }

    /// Writes one row's new value through, reads it back, and adopts it or puts the row back.
    fn change(&self, row: Row, value: i64) {
        let Some(mut held) = self.held.borrow().clone() else { return };
        let (name, field, stored_value) = match row {
            Row::ShowsSeconds => ("display_seconds", "enabled", Value::Flag(value != 0)),
            Row::ResetHour => {
                let face = value.clamp(app_settings::RESET_HOUR_RANGE.0, app_settings::RESET_HOUR_RANGE.1);
                ("daily_reset_time", "hour", Value::Number(app_settings::hour24(face)))
            }
            Row::FetchMinutes => {
                let minutes =
                    value.clamp(app_settings::FETCH_MINUTES_RANGE.0, app_settings::FETCH_MINUTES_RANGE.1);
                ("fetch_history_interval_seconds", "seconds", Value::Number(minutes * 60))
            }
            Row::BlipSeconds => {
                let seconds =
                    value.clamp(app_settings::BLIP_SECONDS_RANGE.0, app_settings::BLIP_SECONDS_RANGE.1);
                ("blip_time", "seconds", Value::Number(seconds))
            }
        };
        let stored = self
            .connect()
            .and_then(|connection| {
                self.report(app_settings::write(&connection, name, field, &stored_value, &*self.log))
            })
            .unwrap_or(false);
        if !stored {
            self.show(&held);
            self.refused(row.title());
            return;
        }
        match row {
            Row::ShowsSeconds => held.shows_seconds = value != 0,
            Row::ResetHour => held.reset_hour = value,
            Row::FetchMinutes => held.fetch_minutes = value,
            Row::BlipSeconds => held.blip_seconds = value,
        }
        *self.held.borrow_mut() = Some(held);
        for changed in self.on_changed.borrow().iter() {
            changed();
        }
    }

    fn connect(&self) -> Option<Connection> {
        match database::connect(&self.database) {
            Ok(connection) => Some(connection),
            Err(error) => {
                self.log
                    .record_failure(Tag::Database, || format!("App: the database would not open: {error}"));
                None
            }
        }
    }

    fn report<T>(&self, result: Result<T, rusqlite::Error>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.log.record_failure(Tag::Database, || format!("App: a database call failed: {error}"));
                None
            }
        }
    }
}

/// `path` with a leading `~` written out as the home folder.
fn untilde(path: &str) -> PathBuf {
    match (path.strip_prefix('~'), std::env::var("HOME")) {
        (Some(rest), Ok(home)) if !home.is_empty() => PathBuf::from(format!("{home}{rest}")),
        _ => PathBuf::from(path),
    }
}

/// Whole unix seconds now. A clock before 1970 reads as 0.
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

/// `path` with the home folder written as `~`.
fn tilde(path: &str) -> String {
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && path.starts_with(&home) => format!("~{}", &path[home.len()..]),
        _ => path.to_string(),
    }
}

fn to_i32(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use facet_core::setting;
    use slint::platform::software_renderer::MinimalSoftwareWindow;
    use slint::platform::{Platform, WindowAdapter};

    struct Headless(Rc<MinimalSoftwareWindow>);

    struct NoOpener;
    impl Opener for NoOpener {
        fn reveal(&self, _file: &std::path::Path) -> Result<(), String> {
            Ok(())
        }
        fn open_url(&self, _url: &str) -> Result<(), String> {
            Ok(())
        }
    }

    struct NoChooser;
    impl FileChooser for NoChooser {
        fn choose_folder(&self, _start: &std::path::Path, _message: &str) -> Option<PathBuf> {
            None
        }
        fn choose_save_file(&self, _name: &str, _message: &str) -> Option<PathBuf> {
            None
        }
    }

    impl Platform for Headless {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn the_settings_are_read_on_open_written_through_and_put_back_when_refused() {
        slint::platform::set_platform(Box::new(Headless(MinimalSoftwareWindow::new(Default::default()))))
            .expect("the headless platform should install");
        let path = std::env::temp_dir().join(format!("facet-ui-app-{}.sqlite", std::process::id()));
        if path.exists() {
            std::fs::remove_file(&path).expect("a stale test database should be removable");
        }
        let connection = database::open(&path, database::APPDATA_DDL).expect("the app DDL should apply");
        let ui = SettingsWindow::new().expect("the window should build");
        let notice = Notice::attach(&ui);
        let trace_file =
            std::env::temp_dir().join(format!("facet-ui-app-trace-{}.sqlite", std::process::id()));
        if trace_file.exists() {
            std::fs::remove_file(&trace_file).expect("a stale trace should be removable");
        }
        let trace = Rc::new(Trace::new(trace_file.clone(), None));
        let app = App::attach(
            &ui,
            path.clone(),
            Rc::clone(&trace),
            Rc::clone(&notice),
            Rc::new(NoOpener),
            Rc::new(NoChooser),
        );
        let changes = Rc::new(std::cell::Cell::new(0));
        let counted = Rc::clone(&changes);
        app.set_on_changed(move || counted.set(counted.get() + 1));
        app.open();
        let data = ui.global::<AppData>();
        assert!(data.get_shows_seconds());
        assert_eq!((data.get_reset_hour(), data.get_fetch_minutes(), data.get_blip_seconds()), (3, 1, 5));

        app.change(Row::ShowsSeconds, 0);
        assert!(!setting::shows_seconds(&connection).expect("should read"));
        app.change(Row::ResetHour, 12);
        assert_eq!(setting::daily_reset_time(&connection).expect("should read"), (0, 0));
        app.change(Row::FetchMinutes, 5);
        assert_eq!(app_settings::fetch_interval_seconds(&connection).expect("should read"), 300);
        app.change(Row::BlipSeconds, 0);
        assert_eq!(setting::blip_seconds(&connection).expect("should read"), 0);
        assert_eq!(changes.get(), 4);

        connection
            .execute("DELETE FROM setting WHERE setting_name = 'blip_time'", [])
            .expect("should delete");
        data.set_blip_seconds(9);
        app.change(Row::BlipSeconds, 9);
        assert_eq!(data.get_blip_seconds(), 0);
        assert_eq!(notice.title(), "That setting was not saved");
        assert_eq!(changes.get(), 4);

        assert!(!data.get_debug_enabled());
        assert!(!data.get_has_debug_trace());
        app.debug_toggled(true);
        assert!(trace.is_recording());
        assert!(setting::debug_trace(&connection).expect("should read").enabled);
        assert!(data.get_has_debug_trace());
        app.clear_trace();
        assert_eq!(notice.title(), "Clear the debug trace?");
        notice.choose(1);
        assert_eq!(notice.title(), "");
        app.debug_toggled(false);
        assert!(!trace.is_recording());

        std::fs::remove_file(&trace_file).expect("the test trace should be removable");
        std::fs::remove_file(&path).expect("the test database should be removable");
    }
}
