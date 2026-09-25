//! Opening a database and putting the schema in it.
//!
//! **Platform-blind, and a path is not a platform.** Where the files live is the composition root's to
//! decide; what a Facet database *is* is decided here, so both platform crates get the same schema from the
//! same list rather than each assembling one.

use std::path::Path;

use rusqlite::Connection;

/// The DDL for `appdata.sqlite`, in the order it is applied.
///
/// **Compiled in.** A shipped binary has nothing to find at runtime, which is what
/// [`docs/port-findings.md`](../../../docs/port-findings.md) records as the difference between a binary that
/// works on the build machine and one that ships.
///
/// **The order is the file numbering and the numbering is dependency order**, so a table's foreign keys
/// always name something already created. Adding a file here without its number is how that stops being
/// true silently.
pub const APPDATA_DDL: &[(&str, &str)] = &[
    ("001_event_type.sql", include_str!("../resources/database/001_event_type.sql")),
    ("002_timezone.sql", include_str!("../resources/database/002_timezone.sql")),
    ("003_device_event.sql", include_str!("../resources/database/003_device_event.sql")),
    ("004_icon.sql", include_str!("../resources/database/004_icon.sql")),
    ("005_colour.sql", include_str!("../resources/database/005_colour.sql")),
    ("006_project.sql", include_str!("../resources/database/006_project.sql")),
    ("007_category.sql", include_str!("../resources/database/007_category.sql")),
    ("008_face.sql", include_str!("../resources/database/008_face.sql")),
    ("009_time_entry.sql", include_str!("../resources/database/009_time_entry.sql")),
    ("010_device_notification.sql", include_str!("../resources/database/010_device_notification.sql")),
    ("011_setting.sql", include_str!("../resources/database/011_setting.sql")),
    ("012_timezone_alias.sql", include_str!("../resources/database/012_timezone_alias.sql")),
    ("013_timezone_lookup.sql", include_str!("../resources/database/013_timezone_lookup.sql")),
];

/// The DDL for `debug.sqlite`, the trace.
///
/// **A separate database and a separate list**, which is what the numbering says: below 500 is the app's,
/// 500 and above is the trace's. The three timezone files appear in both because `debug_log.timezone_id`
/// references `timezone`, and the trace has to stand on its own: it is the file somebody is asked to send
/// in, and it is read without the app's database beside it.
pub const DEBUG_DDL: &[(&str, &str)] = &[
    ("500_timezone.sql", include_str!("../resources/database/500_timezone.sql")),
    ("501_debug_log.sql", include_str!("../resources/database/501_debug_log.sql")),
    ("502_timezone_alias.sql", include_str!("../resources/database/502_timezone_alias.sql")),
    ("503_timezone_lookup.sql", include_str!("../resources/database/503_timezone_lookup.sql")),
];

/// Opens `path`, creating it if it is not there, and applies `ddl` to it.
///
/// Foreign keys are enforced on the connection, which is also how `scripts/switch-database.sh` seeds, so a
/// database built by hand and one built by a launch are built under the same rules.
///
/// **An empty `ddl` is a failure, not a success with nothing applied.** That is a measured fault rather than
/// a defensive check: a bootstrap that applied no files reported success, and what it left behind looked
/// exactly like a database that had been set up. See [`docs/port-findings.md`](../../../docs/port-findings.md).
pub fn open(path: &Path, ddl: &[(&str, &str)]) -> Result<Connection, Error> {
    if ddl.is_empty() {
        return Err(Error::NoDdl);
    }

    let connection =
        Connection::open(path).map_err(|source| Error::Open { path: path.display().to_string(), source })?;
    connection.execute_batch("PRAGMA foreign_keys = ON;").map_err(|source| Error::Pragma { source })?;

    for (name, sql) in ddl {
        connection.execute_batch(sql).map_err(|source| Error::Ddl { file: (*name).to_string(), source })?;
    }

    Ok(connection)
}

/// Opens a database that has already been brought up by [`open`], without applying any DDL.
///
/// For reads and writes at the point of use. Foreign keys are enforced, as they are by [`open`]. The file
/// must already exist: a missing file is an error rather than a new empty database.
pub fn connect(path: &Path) -> Result<Connection, Error> {
    let connection = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|source| Error::Open { path: path.display().to_string(), source })?;
    connection.execute_batch("PRAGMA foreign_keys = ON;").map_err(|source| Error::Pragma { source })?;
    Ok(connection)
}

/// What can go wrong bringing a database up. **Every arm names the file it was working on**, because the
/// message is read by somebody who has one failure and a directory of nineteen files.
#[derive(Debug)]
pub enum Error {
    /// The DDL list was empty, so the database would have come up with no schema in it.
    NoDdl,
    Open {
        path: String,
        source: rusqlite::Error,
    },
    Pragma {
        source: rusqlite::Error,
    },
    Ddl {
        file: String,
        source: rusqlite::Error,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoDdl => {
                write!(formatter, "the DDL list is empty, so the database would have no schema in it")
            }
            Error::Open { path, source } => write!(formatter, "{path} could not be opened: {source}"),
            Error::Pragma { source } => {
                write!(formatter, "foreign keys could not be turned on: {source}")
            }
            Error::Ddl { file, source } => write!(formatter, "{file} would not apply: {source}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::NoDdl => None,
            Error::Open { source, .. } | Error::Pragma { source } | Error::Ddl { source, .. } => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fault this rejection exists for produced a database that looked set up and had nothing in it.
    #[test]
    fn an_empty_ddl_list_is_refused_rather_than_applied() {
        let file = tempfile();
        let error = open(&file, &[]).expect_err("an empty DDL list must not report success");
        assert!(matches!(error, Error::NoDdl));
    }

    #[test]
    fn the_app_database_comes_up_with_its_tables_and_its_seeds() {
        let file = tempfile();
        let connection = open(&file, APPDATA_DDL).expect("the app DDL should apply to an empty file");

        let tables: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN \
                 ('setting', 'category', 'face', 'time_entry', 'icon', 'colour', 'timezone')",
                [],
                |row| row.get(0),
            )
            .expect("the table count should be readable");
        assert_eq!(tables, 7, "every table the app needs should have been created");

        // The seeds matter as much as the tables: a database with the schema and none of the reference
        // rows is one where every category has no colour to point at.
        let colours: i64 = connection
            .query_row("SELECT count(*) FROM colour", [], |row| row.get(0))
            .expect("the colour count should be readable");
        assert!(colours > 0, "the colour reference table should have been seeded");
    }

    /// The trace stands on its own, which is the whole reason it repeats the timezone tables.
    #[test]
    fn the_trace_database_holds_debug_log_and_the_timezones_it_references() {
        let file = tempfile();
        let connection = open(&file, DEBUG_DDL).expect("the debug DDL should apply to an empty file");

        connection
            .execute(
                "INSERT INTO debug_log (logged_at, tag, message) VALUES ('2026-09-21T13:25:38.123', 'launch', 'x')",
                [],
            )
            .expect("a row should be insertable with foreign keys on");
    }

    /// Applying it twice is what every launch does, the files being IF NOT EXISTS throughout.
    #[test]
    fn applying_the_ddl_to_a_database_that_already_has_it_changes_nothing() {
        let file = tempfile();
        let first = open(&file, APPDATA_DDL).expect("the first apply should work");
        let settings: i64 = first
            .query_row("SELECT count(*) FROM setting", [], |row| row.get(0))
            .expect("the setting count should be readable");
        drop(first);

        let second = open(&file, APPDATA_DDL).expect("the second apply should work");
        let again: i64 = second
            .query_row("SELECT count(*) FROM setting", [], |row| row.get(0))
            .expect("the setting count should be readable");
        assert_eq!(settings, again, "a second apply should not seed a second copy of every row");
    }

    /// A path in the temp directory, named for this process and this test so two running at once cannot
    /// collide. Removed on the way in rather than on the way out, so a failed run leaves the file to look at.
    fn tempfile() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);

        let path = std::env::temp_dir().join(format!(
            "facet-core-test-{}-{}.sqlite",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&path);
        path
    }
}
