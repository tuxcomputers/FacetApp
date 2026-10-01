//! The debug trace: what the app says it is doing, printed and recorded.
//!
//! **Recording is the half that matters.** A terminal transcript is whatever is still in a scrollback
//! buffer; a `debug_log` row outlives the session and is what every scripted check polls for. So this
//! prints *and* writes, and a bare `println!` anywhere else in the app is that second half being skipped.
//!
//! **Injected, never global.** Built once in the composition root, gated there on the `debug` setting, and
//! handed on as an [`Option`], so a launch with logging off holds no logger at all rather than one that
//! returns early on every call. [`Record`] is what lets a call site say what happened without asking first.
//!
//! **A message is plain text: no apostrophes, and no quotation marks around a value.** Messages are read
//! back out of this table by SQL `LIKE` patterns, and a pattern goes inside a single-quoted string literal,
//! so *The cube's clock is set* closes the quote at `cube` and sqlite refuses the whole statement,
//! **answering nothing rather than failing**. Inserting is by bound parameter and so is safe either way;
//! the hazard is entirely on the reading side, which is why the debug build asserts rather than escapes.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{Connection, params};

use crate::database;
use crate::port::Zone;
use crate::timezone::{self, UnnamedZone};

/// What a message is about. **One enum, so the padding is computed rather than typed**, and adding a case
/// re-pads every tag automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    Launch,
    Settings,
    Tray,
    Menu,
    Database,
    Quit,
    Timing,
    Event,
    Entry,
    Click,
    Limit,
    Report,
    Google,
    Radio,
    Login,
    Pin,
    Pair,
    Command,
    Device,
    /// A write, subscription or read request sent over Bluetooth.
    BleTx,
    /// An acknowledgement, answer or notification that came back over Bluetooth.
    BleRx,
    /// The face the cube reports as up.
    Face,
    /// Asking the cube for its history, and what came back.
    History,
    /// A face lit in its category's colour.
    Colour,
    /// The app pausing the cube itself: a face with no category.
    Forced,
    /// Time entries sent to the Google calendar.
    Sync,
    /// What the menu bar line says and its colours.
    Status,
}

impl Tag {
    /// Every case, which is what the width below is measured over. **Adding a case means adding it here**;
    /// a tag missing from this list is one the console columns do not line up with.
    pub const ALL: &'static [Tag] = &[
        Tag::Launch,
        Tag::Settings,
        Tag::Tray,
        Tag::Menu,
        Tag::Database,
        Tag::Quit,
        Tag::Timing,
        Tag::Event,
        Tag::Entry,
        Tag::Click,
        Tag::Limit,
        Tag::Report,
        Tag::Google,
        Tag::Radio,
        Tag::Login,
        Tag::Pin,
        Tag::Pair,
        Tag::Command,
        Tag::Device,
        Tag::BleTx,
        Tag::BleRx,
        Tag::Face,
        Tag::History,
        Tag::Colour,
        Tag::Forced,
        Tag::Sync,
        Tag::Status,
    ];

    /// The word inside the brackets, and what goes in the `tag` column. Lower case, because a `LIKE`
    /// pattern in a check is written once and should not have to guess at capitals.
    pub const fn word(self) -> &'static str {
        match self {
            Tag::Launch => "launch",
            Tag::Settings => "settings",
            Tag::Tray => "tray",
            Tag::Menu => "menu",
            Tag::Database => "database",
            Tag::Quit => "quit",
            Tag::Timing => "timing",
            Tag::Event => "event",
            Tag::Entry => "entry",
            Tag::Click => "click",
            Tag::Limit => "limit",
            Tag::Report => "report",
            Tag::Google => "google",
            Tag::Radio => "radio",
            Tag::Login => "login",
            Tag::Pin => "pin",
            Tag::Pair => "pair",
            Tag::Command => "command",
            Tag::Device => "device",
            Tag::BleTx => "ble-tx",
            Tag::BleRx => "ble-rx",
            Tag::Face => "face",
            Tag::History => "history",
            Tag::Colour => "colour",
            Tag::Forced => "forced",
            Tag::Sync => "sync",
            Tag::Status => "status",
        }
    }

    /// The longest tag there is, which is what every tag is padded out to so console lines stay aligned.
    pub const WIDTH: usize = {
        let mut widest = 0;
        let mut index = 0;
        while index < Tag::ALL.len() {
            let width = Tag::ALL[index].word().len();
            if width > widest {
                widest = width;
            }
            index += 1;
        }
        widest
    };
}

/// `text` with apostrophes and double quotes removed, for putting a user-supplied value such as a category
/// name into a message.
pub fn plain(text: &str) -> String {
    text.chars().filter(|&c| c != '\'' && c != '"').collect()
}

/// Says what happened, if there is anywhere to say it.
///
/// **Implemented on `Option<DebugLog>` rather than on the logger**, which is what keeps the gate in the
/// composition root: a call site says what it did and does not ask first, and a launch with logging off
/// costs a null check.
///
/// The message is built by a closure, so a launch that is recording nothing builds no strings at all.
pub trait Record {
    /// Says what happened. Recorded and printed when there is a logger, and costs a null check when
    /// there is not.
    fn record(&self, tag: Tag, message: impl FnOnce() -> String);

    /// Says something went wrong.
    ///
    /// **This one speaks whether or not anything is being recorded**, which is the whole difference
    /// between it and [`Record::record`]: a trace is optional and a failure is not. It goes to stderr
    /// always, and into the trace as well when there is one.
    fn record_failure(&self, tag: Tag, message: impl FnOnce() -> String);
}

impl Record for Option<DebugLog> {
    fn record(&self, tag: Tag, message: impl FnOnce() -> String) {
        if let Some(log) = self {
            let message = message();
            let stamped = log.write_row(tag, &message);
            let clock = stamped.as_deref().map(clock).unwrap_or("--:--:--");
            println!("{clock} [{:width$}] {message}", tag.word(), width = Tag::WIDTH);
        }
    }

    fn record_failure(&self, tag: Tag, message: impl FnOnce() -> String) {
        let message = message();
        let stamped = match self {
            Some(log) => log.write_row(tag, &message),
            None => None,
        };
        let clock = stamped.as_deref().map(clock).unwrap_or("--:--:--");
        eprintln!("{clock} [{:width$}] {message}", tag.word(), width = Tag::WIDTH);
    }
}

impl<T: Record + ?Sized> Record for std::rc::Rc<T> {
    fn record(&self, tag: Tag, message: impl FnOnce() -> String) {
        (**self).record(tag, message);
    }

    fn record_failure(&self, tag: Tag, message: impl FnOnce() -> String) {
        (**self).record_failure(tag, message);
    }
}

/// The launch's debug trace: where its file is, and the logger writing it while recording is on.
///
/// Recording can be switched while the app runs. Off holds no logger at all. The file is fixed for the
/// launch; a folder chosen in the settings applies from the next launch.
pub struct Trace {
    /// The file in use, which is the fallback once the chosen one has been refused.
    file: RefCell<PathBuf>,
    /// Where the trace goes when the chosen folder cannot be used.
    fallback: PathBuf,
    log: RefCell<Option<DebugLog>>,
    /// Names the machine's zone each time the log is opened.
    zone: Arc<dyn Zone>,
}

impl Trace {
    /// A trace kept in `file`, recording when `log` is given. `zone` names the zone the rows are filed under whenever
    /// recording is switched on.
    pub fn new(file: PathBuf, log: Option<DebugLog>, zone: Arc<dyn Zone>) -> Trace {
        Trace { fallback: file.clone(), file: RefCell::new(file), log: RefCell::new(log), zone }
    }

    /// The same trace, kept in `fallback` when its folder cannot be used.
    pub fn with_fallback(mut self, fallback: PathBuf) -> Trace {
        self.fallback = fallback;
        self
    }

    /// A trace that records nothing and has no file, for tests and renders.
    pub fn none() -> Trace {
        Trace::new(PathBuf::new(), None, Arc::new(UnnamedZone))
    }

    /// The trace file in use.
    pub fn file(&self) -> PathBuf {
        self.file.borrow().clone()
    }

    pub fn is_recording(&self) -> bool {
        self.log.borrow().is_some()
    }

    /// Starts or stops recording. Starting opens the file, creating its folder, and then records `Logging
    /// turned on`; stopping records `Logging turned off` and then closes it. Asking for the state it is
    /// already in does nothing.
    ///
    /// **A folder that cannot be used falls back to the fallback folder**, said on stderr and in the trace itself.
    /// An error means neither could be used.
    pub fn set_recording(&self, on: bool) -> Result<(), String> {
        if on == self.is_recording() {
            return Ok(());
        }
        if on {
            let chosen = self.file();
            let (log, used, refused) = open_with_fallback(&chosen, &self.fallback, &*self.zone)?;
            *self.file.borrow_mut() = used.clone();
            *self.log.borrow_mut() = Some(log);
            self.record(Tag::Settings, || "Logging turned on".to_string());
            if let Some(reason) = refused {
                say_fallback(self, &chosen, &used, &reason);
            }
        } else {
            self.record(Tag::Settings, || "Logging turned off".to_string());
            *self.log.borrow_mut() = None;
        }
        Ok(())
    }
}

/// Opens the trace in `file`, creating its folder, and in `fallback` when `file` cannot be used. Returns the log, the
/// file it is in, and why `file` was refused when it was. An error says why neither could be used.
pub fn open_with_fallback(
    file: &Path,
    fallback: &Path,
    zone: &dyn Zone,
) -> Result<(DebugLog, PathBuf, Option<String>), String> {
    match open_in_folder(file, zone) {
        Ok(log) => Ok((log, file.to_path_buf(), None)),
        Err(refused) if file != fallback => match open_in_folder(fallback, zone) {
            Ok(log) => Ok((log, fallback.to_path_buf(), Some(refused.to_string()))),
            Err(also) => Err(format!("{refused}, and neither could {}: {also}", fallback.display())),
        },
        Err(refused) => Err(refused.to_string()),
    }
}

fn open_in_folder(file: &Path, zone: &dyn Zone) -> Result<DebugLog, database::Error> {
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(|error| database::Error::Open {
            path: folder.display().to_string(),
            source: rusqlite::Error::InvalidPath(PathBuf::from(error.to_string())),
        })?;
    }
    DebugLog::open(file, zone)
}

/// Says that the trace is kept in `used` because `chosen` was refused: on stderr, which is read when the trace itself
/// cannot be, and as the trace's own first row.
pub fn say_fallback(trace: &Trace, chosen: &Path, used: &Path, reason: &str) {
    eprintln!(
        "[{:width$}] the trace cannot be kept in {}, so it is kept in {}: {reason}",
        Tag::Database.word(),
        chosen.display(),
        used.display(),
        width = Tag::WIDTH
    );
    trace.record(Tag::Database, || {
        format!(
            "The trace could not be kept in {}, so it is kept in {}: {}",
            plain(&chosen.display().to_string()),
            plain(&used.display().to_string()),
            plain(reason)
        )
    });
}

impl Record for Trace {
    fn record(&self, tag: Tag, message: impl FnOnce() -> String) {
        self.log.borrow().record(tag, message);
    }

    fn record_failure(&self, tag: Tag, message: impl FnOnce() -> String) {
        self.log.borrow().record_failure(tag, message);
    }
}

/// `13:25:38` out of `2026-09-21T13:25:38.123`. Whole seconds on the console, milliseconds in the row: the
/// row is what a check reads and ordering within a second matters there, and the console is read by a
/// person.
fn clock(stamped: &str) -> &str {
    stamped.get(11..19).unwrap_or(stamped)
}

/// The trace database, open from launch to quit.
pub struct DebugLog {
    connection: Connection,
    /// The zone every row is filed under, named when the log opened.
    timezone_id: i64,
    /// Whether a failed write has already been complained about. **Announced once rather than never and
    /// rather than every time**: a trace that cannot be written is one fact, and repeating it per message
    /// would bury the run it is trying to describe under the complaint.
    reported_failure: Cell<bool>,
}

impl DebugLog {
    /// Opens `path` and puts the trace schema in it, creating the file if it is not there.
    ///
    /// **The file is brought up by whoever opens it, not by the first message.** The Swift app deferred it
    /// so that a launch recording nothing left no `debug.sqlite` behind; here the logger is only built at
    /// all when the setting says so, so the launch that opens it is already one that is recording.
    ///
    /// **The machine's zone is named once, here, and every row carries it.** A zone that cannot be named files the
    /// rows under Unknown, said on stderr, there being nowhere else to say it.
    pub fn open(path: &Path, zone: &dyn Zone) -> Result<Self, database::Error> {
        let connection = database::open(path, database::DEBUG_DDL)?;
        let timezone_id = match zone.name() {
            Ok(name) => timezone::id_for(&connection, &name).unwrap_or_else(|error| {
                eprintln!(
                    "[{:width$}] the time zone {name} could not be filed, so the trace rows are filed under Unknown: {error}",
                    Tag::Database.word(),
                    width = Tag::WIDTH
                );
                timezone::UNKNOWN
            }),
            Err(error) => {
                eprintln!(
                    "[{:width$}] the time zone of this machine could not be named, so the trace rows are filed under Unknown: {error}",
                    Tag::Database.word(),
                    width = Tag::WIDTH
                );
                timezone::UNKNOWN
            }
        };
        Ok(DebugLog { connection, timezone_id, reported_failure: Cell::new(false) })
    }

    /// Writes the message as a row and gives back the timestamp it was written with, so the line printed
    /// beside it carries the same one.
    ///
    /// **The timestamp is read from sqlite rather than from a clock here**: the line in the terminal and
    /// the row in the table are then the same instant rather than two readings of it, and local time comes
    /// from the one place that already knows how to ask for it.
    ///
    /// `None` when the row could not be written, which has already been complained about by then.
    fn write_row(&self, tag: Tag, message: &str) -> Option<String> {
        debug_assert!(
            !message.contains('\''),
            "a debug message must not contain an apostrophe: it is read back by a SQL LIKE pattern inside \
             a single-quoted literal, which the apostrophe closes. Reword it. Message: {message}"
        );

        let stamped: Result<String, _> = self.connection.query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%f', 'now', 'localtime')",
            [],
            |row| row.get(0),
        );
        let stamped = match stamped {
            Ok(stamped) => stamped,
            Err(error) => {
                self.complain_once(&error);
                return None;
            }
        };

        let written = self.connection.execute(
            "INSERT INTO debug_log (logged_at, timezone_id, tag, message) VALUES (?1, ?2, ?3, ?4)",
            params![stamped, self.timezone_id, tag.word(), message],
        );
        if let Err(error) = written {
            self.complain_once(&error);
            return None;
        }

        Some(stamped)
    }

    /// Says a trace write failed, the first time it does.
    ///
    /// **The trace failing must not be silent**, because a run reconstructed from an empty table looks
    /// exactly like a run where nothing happened. It also must not become the run: hence once.
    fn complain_once(&self, error: &rusqlite::Error) {
        if self.reported_failure.replace(true) {
            return;
        }
        eprintln!(
            "[{:width$}] the debug trace could not be written, so this run leaves no record: {error}",
            Tag::Database.word(),
            width = Tag::WIDTH
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timezone::SYDNEY;

    #[test]
    fn every_tag_is_padded_to_the_longest_one() {
        assert_eq!(Tag::WIDTH, "database".len());
        for tag in Tag::ALL {
            assert!(tag.word().len() <= Tag::WIDTH);
        }
    }

    /// The width is derived rather than written down, which is the point: this fails the day a longer tag
    /// is added without the constant being touched, and that is the failure not happening.
    #[test]
    fn the_width_follows_the_tags_rather_than_being_stated() {
        let longest = Tag::ALL.iter().map(|tag| tag.word().len()).max().expect("there is at least one tag");
        assert_eq!(Tag::WIDTH, longest);
    }

    #[test]
    fn a_recorded_message_is_a_row_that_can_be_read_back() {
        let log = Some(DebugLog::open(&tempfile(), &SYDNEY).expect("the trace should open"));
        log.record(Tag::Launch, || "Facet is in the menu bar".to_string());

        let Some(log) = &log else { unreachable!() };
        let (tag, message, stamped): (String, String, String) = log
            .connection
            .query_row("SELECT tag, message, logged_at FROM debug_log", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("the row should be there");

        assert_eq!(tag, "launch");
        assert_eq!(message, "Facet is in the menu bar");
        // What a check sorts on, so the shape is interface: a date, a T, and a time carrying milliseconds.
        assert_eq!(stamped.len(), "2026-09-21T13:25:38.123".len(), "got {stamped}");
        assert_eq!(&stamped[10..11], "T", "got {stamped}");
    }

    #[test]
    fn every_row_is_filed_under_the_zone_the_machine_named_when_the_log_opened() {
        let log = Some(DebugLog::open(&tempfile(), &SYDNEY).expect("the trace should open"));
        log.record(Tag::Launch, || "one".to_string());
        log.record(Tag::Launch, || "two".to_string());

        let Some(log) = &log else { unreachable!() };
        let zones: Vec<String> = log
            .connection
            .prepare(
                "SELECT timezone_name FROM debug_log JOIN timezone USING (timezone_id) ORDER BY debug_log_id",
            )
            .expect("prepare")
            .query_map([], |row| row.get(0))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("rows");
        assert_eq!(zones, ["Australia/Sydney", "Australia/Sydney"]);
    }

    #[test]
    fn a_zone_that_cannot_be_named_files_the_rows_under_unknown() {
        let log =
            Some(DebugLog::open(&tempfile(), &crate::timezone::UnnamedZone).expect("the trace should open"));
        log.record(Tag::Launch, || "one".to_string());

        let Some(log) = &log else { unreachable!() };
        let id: i64 = log
            .connection
            .query_row("SELECT timezone_id FROM debug_log", [], |row| row.get(0))
            .expect("the row should be there");
        assert_eq!(id, crate::timezone::UNKNOWN);
    }

    /// The gate is in the composition root, so this is what a launch with logging off costs.
    #[test]
    fn a_launch_with_no_logger_records_nothing_and_builds_nothing() {
        let absent: Option<DebugLog> = None;
        let mut built = false;
        absent.record(Tag::Tray, || {
            built = true;
            String::new()
        });
        assert!(!built, "the message should not be built when there is nowhere to record it");
    }

    #[test]
    fn messages_arrive_in_the_order_they_were_recorded() {
        let log = Some(DebugLog::open(&tempfile(), &SYDNEY).expect("the trace should open"));
        log.record(Tag::Launch, || "first".to_string());
        log.record(Tag::Settings, || "second".to_string());
        log.record(Tag::Quit, || "third".to_string());

        let Some(log) = &log else { unreachable!() };
        let mut statement = log
            .connection
            .prepare("SELECT message FROM debug_log ORDER BY debug_log_id")
            .expect("the query should prepare");
        let messages: Vec<String> = statement
            .query_map([], |row| row.get(0))
            .expect("the query should run")
            .collect::<Result<_, _>>()
            .expect("every row should read");
        assert_eq!(messages, vec!["first", "second", "third"]);
    }

    #[test]
    fn recording_switches_on_and_off_and_says_so_in_the_file() {
        let file = tempfile();
        let trace = Trace::new(file.clone(), None, Arc::new(SYDNEY));
        assert!(!trace.is_recording());
        trace.record(Tag::Settings, || "not written".to_string());
        trace.set_recording(true).expect("recording should start");
        assert!(trace.is_recording());
        trace.record(Tag::Settings, || "written".to_string());
        trace.set_recording(false).expect("recording should stop");
        assert!(!trace.is_recording());
        trace.record(Tag::Settings, || "not written either".to_string());

        let connection = Connection::open(&file).expect("the trace should open");
        let mut statement = connection
            .prepare("SELECT message FROM debug_log ORDER BY debug_log_id")
            .expect("should prepare");
        let messages: Vec<String> = statement
            .query_map([], |row| row.get(0))
            .expect("should query")
            .map(|m| m.expect("row"))
            .collect();
        assert_eq!(messages, ["Logging turned on", "written", "Logging turned off"]);
    }

    /// A path that cannot be made into a folder: its parent is a regular file.
    fn unusable_trace_file() -> std::path::PathBuf {
        let blocker = tempfile();
        std::fs::write(&blocker, b"not a folder").expect("the blocker should be written");
        blocker.join("trace").join("debug.sqlite")
    }

    #[test]
    fn a_usable_folder_is_used_and_nothing_is_said() {
        let chosen = tempfile();
        let fallback = tempfile();
        let (_, used, refused) = open_with_fallback(&chosen, &fallback, &SYDNEY).expect("should open");
        assert_eq!((used, refused), (chosen, None));
    }

    #[test]
    fn a_folder_that_cannot_be_made_falls_back_and_says_why() {
        let chosen = unusable_trace_file();
        let fallback = tempfile();
        let (log, used, refused) = open_with_fallback(&chosen, &fallback, &SYDNEY).expect("should fall back");
        assert_eq!(used, fallback);
        assert!(refused.is_some_and(|reason| reason.contains("could not be opened")), "the reason is given");
        Some(log).record(Tag::Launch, || "it works".to_string());
    }

    #[test]
    fn with_neither_usable_the_error_names_both() {
        let chosen = unusable_trace_file();
        let fallback = unusable_trace_file();
        let error = open_with_fallback(&chosen, &fallback, &SYDNEY).err().expect("neither should open");
        assert!(error.contains("neither could"), "{error}");
    }

    #[test]
    fn turning_recording_on_falls_back_and_the_file_in_use_follows() {
        let fallback = tempfile();
        let trace = Trace::new(unusable_trace_file(), None, Arc::new(SYDNEY)).with_fallback(fallback.clone());
        trace.set_recording(true).expect("recording should start in the fallback");
        assert_eq!(trace.file(), fallback);
        assert!(trace.is_recording());
        let rows: Vec<String> = Connection::open(&fallback)
            .expect("the fallback should open")
            .prepare("SELECT message FROM debug_log ORDER BY debug_log_id")
            .expect("prepare")
            .query_map([], |row| row.get(0))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("rows");
        assert_eq!(rows[0], "Logging turned on");
        assert!(rows[1].starts_with("The trace could not be kept in "), "{rows:?}");
        assert!(rows[1].contains("so it is kept in "), "{rows:?}");
    }

    fn tempfile() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);

        let path = std::env::temp_dir().join(format!(
            "facet-debug-log-test-{}-{}.sqlite",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&path);
        path
    }
}
