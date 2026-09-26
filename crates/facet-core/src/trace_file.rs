//! Copying and clearing the debug trace file, on a connection of their own beside the logger's.

use std::path::Path;

use rusqlite::{Connection, OpenFlags, params};

/// Opens the trace at `trace` without creating it, waiting up to two seconds for a lock.
fn connect(trace: &Path) -> Result<Connection, rusqlite::Error> {
    let connection = Connection::open_with_flags(
        trace,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.busy_timeout(std::time::Duration::from_secs(2))?;
    Ok(connection)
}

/// Writes a copy of the trace at `trace` to `destination`, replacing any file already there.
pub fn copy_to(trace: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        std::fs::remove_file(destination)
            .map_err(|error| format!("{} could not be replaced: {error}", destination.display()))?;
    }
    let connection =
        connect(trace).map_err(|error| format!("{} could not be opened: {error}", trace.display()))?;
    connection
        .execute("VACUUM INTO ?1", params![destination.display().to_string()])
        .map_err(|error| format!("the copy to {} failed: {error}", destination.display()))?;
    Ok(())
}

/// Removes every row of the trace at `trace` and compacts the file, then checks it is empty.
pub fn clear(trace: &Path) -> Result<(), String> {
    let connection =
        connect(trace).map_err(|error| format!("{} could not be opened: {error}", trace.display()))?;
    connection
        .execute_batch("DELETE FROM debug_log; VACUUM;")
        .map_err(|error| format!("{} could not be emptied: {error}", trace.display()))?;
    let left: i64 = connection
        .query_row("SELECT COUNT(*) FROM debug_log", [], |row| row.get(0))
        .map_err(|error| format!("{} could not be checked: {error}", trace.display()))?;
    if left == 0 { Ok(()) } else { Err(format!("{left} rows were still in the trace after it was emptied")) }
}

/// The file name a copy is offered under: `facet-debug-yyyy-mm-dd-hh.mm.ss.sqlite`, local time at `now`.
pub fn copy_name(connection: &Connection, now: i64) -> Result<String, rusqlite::Error> {
    connection.query_row(
        "SELECT 'facet-debug-' || strftime('%Y-%m-%d-%H.%M.%S', ?1, 'unixepoch', 'localtime') || '.sqlite'",
        params![now],
        |row| row.get(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_log::{DebugLog, Record, Tag, Trace};

    fn tempfile(name: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("facet-trace-file-{name}-{}.sqlite", std::process::id()));
        if path.exists() {
            std::fs::remove_file(&path).expect("a stale file should be removable");
        }
        path
    }

    #[test]
    fn a_copy_holds_the_rows_and_clearing_empties_the_trace() {
        let file = tempfile("trace");
        let copy = tempfile("copy");
        let trace = Trace::new(file.clone(), Some(DebugLog::open(&file).expect("the trace should open")));
        trace.record(Tag::Settings, || "one".to_string());
        trace.record(Tag::Settings, || "two".to_string());

        copy_to(&file, &copy).expect("the copy should be written");
        let copied: i64 = Connection::open(&copy)
            .expect("the copy should open")
            .query_row("SELECT COUNT(*) FROM debug_log", [], |row| row.get(0))
            .expect("the copy should read");
        assert_eq!(copied, 2);

        clear(&file).expect("the trace should clear");
        let left: i64 = Connection::open(&file)
            .expect("the trace should open")
            .query_row("SELECT COUNT(*) FROM debug_log", [], |row| row.get(0))
            .expect("the trace should read");
        assert_eq!(left, 0);

        std::fs::remove_file(&copy).expect("the copy should be removable");
        drop(trace);
        std::fs::remove_file(&file).expect("the trace should be removable");
    }

    #[test]
    fn a_missing_trace_is_an_error_not_a_new_file() {
        let missing = tempfile("missing");
        assert!(clear(&missing).is_err());
        assert!(!missing.exists());
    }

    #[test]
    fn a_copy_is_named_for_the_local_time() {
        let connection = crate::testing::seeded();
        let epoch: i64 = connection
            .query_row("SELECT CAST(strftime('%s', '2026-09-26 14:05:09', 'utc') AS INTEGER)", [], |r| {
                r.get(0)
            })
            .expect("should compute");
        assert_eq!(
            copy_name(&connection, epoch).expect("should name"),
            "facet-debug-2026-09-26-14.05.09.sqlite"
        );
    }
}
