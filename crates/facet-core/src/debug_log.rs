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

use rusqlite::{Connection, params};

use crate::database;

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
    file: PathBuf,
    log: RefCell<Option<DebugLog>>,
}

impl Trace {
    /// A trace kept in `file`, recording when `log` is given.
    pub fn new(file: PathBuf, log: Option<DebugLog>) -> Trace {
        Trace { file, log: RefCell::new(log) }
    }

    /// A trace that records nothing and has no file, for tests and renders.
    pub fn none() -> Trace {
        Trace::new(PathBuf::new(), None)
    }

    /// The trace file for this launch.
    pub fn file(&self) -> &Path {
        &self.file
    }

    pub fn is_recording(&self) -> bool {
        self.log.borrow().is_some()
    }

    /// Starts or stops recording. Starting opens the file, creating its folder, and then records `Logging
    /// turned on`; stopping records `Logging turned off` and then closes it. Asking for the state it is
    /// already in does nothing.
    pub fn set_recording(&self, on: bool) -> Result<(), database::Error> {
        if on == self.is_recording() {
            return Ok(());
        }
        if on {
            if let Some(folder) = self.file.parent() {
                std::fs::create_dir_all(folder).map_err(|error| database::Error::Open {
                    path: folder.display().to_string(),
                    source: rusqlite::Error::InvalidPath(PathBuf::from(error.to_string())),
                })?;
            }
            *self.log.borrow_mut() = Some(DebugLog::open(&self.file)?);
            self.record(Tag::Settings, || "Logging turned on".to_string());
        } else {
            self.record(Tag::Settings, || "Logging turned off".to_string());
            *self.log.borrow_mut() = None;
        }
        Ok(())
    }
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
    pub fn open(path: &Path) -> Result<Self, database::Error> {
        Ok(DebugLog {
            connection: database::open(path, database::DEBUG_DDL)?,
            reported_failure: Cell::new(false),
        })
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
            "INSERT INTO debug_log (logged_at, tag, message) VALUES (?1, ?2, ?3)",
            params![stamped, tag.word(), message],
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
        let log = Some(DebugLog::open(&tempfile()).expect("the trace should open"));
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
        let log = Some(DebugLog::open(&tempfile()).expect("the trace should open"));
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
        let trace = Trace::new(file.clone(), None);
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
