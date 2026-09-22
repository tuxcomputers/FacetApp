//! Composition root for Linux. The only place that knows both a port and the thing that performs it.
//!
//! Today that is the two databases and the trace behind them. There is no window, no tray and no radio
//! yet: those are separate items, and this one is deliberately the half with no decisions left open.
//!
//! **Where the files live is decided here and nowhere else.** `facet-core` is handed paths; it does not
//! know which platform laid them out, and asking it to would be the core caring what it is running on.
//! `crates/facet-mac/src/main.rs` is the worked example this follows, and the only line in it that was
//! really about macOS was the directory.

use std::path::PathBuf;

use facet_core::database;
use facet_core::debug_log::{DebugLog, Record, Tag};
use facet_core::setting;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // **Before anything else**, so that a database that will not come up says so in a terminal rather
    // than from behind a tray icon nobody has clicked yet. On this platform there is not even a tray
    // icon to hide behind, which makes it more important rather than less.
    let log = open_databases()?;

    log.record(Tag::Launch, || {
        "Facet started on Linux. No tray and no window on this platform yet".to_string()
    });

    Ok(())
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
