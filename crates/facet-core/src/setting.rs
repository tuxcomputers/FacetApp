//! Reading the `setting` table.
//!
//! **A read, at the moment of use.** Nothing here caches: every call goes to the table, which is the first
//! rule in `CLAUDE.md` and the reason this is a function rather than a struct holding values.
//!
//! **The JSON is unpacked by sqlite rather than by a parser here.** Each setting's value is a JSON object,
//! and `json_extract` is built into sqlite, so the field is pulled out where the value already lives instead
//! of being brought into the app and taken apart a second time. It also means no JSON dependency, and it
//! means an unparseable value fails on the read rather than somewhere later.

use rusqlite::{Connection, OptionalExtension};

/// The `debug` setting: whether the trace is gathered, and which folder it is kept in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugTrace {
    pub enabled: bool,
    /// **As it is stored**, so a leading `~` is still a `~`. Empty means the folder the app already keeps
    /// its databases in, which is a different directory on each platform and so cannot be seeded as a path.
    /// Expanding it is the composition root's, at the point the file is opened.
    pub directory: String,
}

impl Default for DebugTrace {
    /// What a database with no `debug` row would give, which is what the DDL seeds: off, and no folder
    /// named. **Named here rather than at each call site** so a missing row cannot come to mean two
    /// different things in two places.
    fn default() -> Self {
        DebugTrace { enabled: false, directory: String::new() }
    }
}

/// Reads the `debug` setting.
///
/// A row that is absent gives [`DebugTrace::default`], which is the seeded value: a database that predates
/// the row is off rather than an error. A row that is *there* and unreadable is an error, because that is a
/// value somebody set and the app cannot honour.
pub fn debug_trace(connection: &Connection) -> Result<DebugTrace, rusqlite::Error> {
    let row = connection
        .query_row(
            "SELECT json_extract(setting_value, '$.enabled'), \
                    coalesce(json_extract(setting_value, '$.directory'), '') \
             FROM setting WHERE setting_name = 'debug'",
            [],
            |row| Ok((row.get::<_, Option<bool>>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;

    Ok(match row {
        Some((enabled, directory)) => DebugTrace { enabled: enabled.unwrap_or(false), directory },
        None => DebugTrace::default(),
    })
}

/// Which database this launch landed on: `production` or `test`.
///
/// **The safety check the scripted suite runs on**, per `Tests/CLAUDE.md`: reading `production` during what
/// is supposed to be a testing session means the `appdata.sqlite` symlink was never repointed, and testing
/// must not proceed.
pub fn database_type(connection: &Connection) -> Result<String, rusqlite::Error> {
    let value = connection
        .query_row(
            "SELECT json_extract(setting_value, '$.type') FROM setting WHERE setting_name = 'db_type'",
            [],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?;
    Ok(value.flatten().unwrap_or_else(|| "unknown".to_string()))
}

/// One integer field of one setting's JSON value. `None` when the row or the field is absent.
fn integer(connection: &Connection, name: &str, field: &str) -> Result<Option<i64>, rusqlite::Error> {
    let value = connection
        .query_row(
            "SELECT json_extract(setting_value, ?2) FROM setting WHERE setting_name = ?1",
            rusqlite::params![name, format!("$.{field}")],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()?;
    Ok(value.flatten())
}

/// Whether a cube is paired: `setting.paired.paired`. An absent row reads as not paired.
pub fn is_cube_paired(connection: &Connection) -> Result<bool, rusqlite::Error> {
    Ok(integer(connection, "paired", "paired")?.unwrap_or(0) != 0)
}

/// The shortest segment that becomes a time entry, from `setting.blip_time.seconds`.
///
/// Clamped to 0 through 30; an absent row or field reads as 5. Zero means every segment counts.
pub fn blip_seconds(connection: &Connection) -> Result<i64, rusqlite::Error> {
    Ok(integer(connection, "blip_time", "seconds")?.unwrap_or(5).clamp(0, 30))
}

/// The local time each day's totals roll over at, as `(hour, minute)`, from `setting.daily_reset_time`.
///
/// Hour clamped to 0 through 23 and minute to 0 through 59. An absent field reads as 3:00.
pub fn daily_reset_time(connection: &Connection) -> Result<(i64, i64), rusqlite::Error> {
    let hour = integer(connection, "daily_reset_time", "hour")?.unwrap_or(3).clamp(0, 23);
    let minute = integer(connection, "daily_reset_time", "minute")?.unwrap_or(0).clamp(0, 59);
    Ok((hour, minute))
}

/// Whether durations and clock times show seconds, from `setting.display_seconds.enabled`. An absent row or
/// field reads as true.
pub fn shows_seconds(connection: &Connection) -> Result<bool, rusqlite::Error> {
    Ok(integer(connection, "display_seconds", "enabled")?.unwrap_or(1) != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[test]
    fn a_seeded_database_reads_as_logging_off_with_no_folder_named() {
        let connection = seeded();
        assert_eq!(debug_trace(&connection).expect("the debug row should read"), DebugTrace::default());
    }

    #[test]
    fn turning_it_on_is_read_back_as_on() {
        let connection = seeded();
        connection
            .execute(
                "UPDATE setting SET setting_value = '{\"enabled\":true,\"directory\":\"~/Traces\"}' \
                 WHERE setting_name = 'debug'",
                [],
            )
            .expect("the debug row should be writable");

        let trace = debug_trace(&connection).expect("the debug row should read");
        assert!(trace.enabled);
        // Stored as typed, tilde and all: an absolute path names one machine, and this database is copied
        // between them.
        assert_eq!(trace.directory, "~/Traces");
    }

    /// A database old enough not to have the row is off, not broken.
    #[test]
    fn a_database_with_no_debug_row_reads_as_the_seeded_value() {
        let connection = seeded();
        connection
            .execute("DELETE FROM setting WHERE setting_name = 'debug'", [])
            .expect("the debug row should be removable");
        assert_eq!(debug_trace(&connection).expect("a missing row should read"), DebugTrace::default());
    }

    #[test]
    fn a_freshly_seeded_database_calls_itself_production() {
        let connection = seeded();
        assert_eq!(database_type(&connection).expect("db_type should read"), "production");
    }

    #[test]
    fn the_seeded_values_read_as_unpaired_five_seconds_and_three_in_the_morning() {
        let connection = seeded();
        assert!(!is_cube_paired(&connection).expect("paired should read"));
        assert_eq!(blip_seconds(&connection).expect("blip_time should read"), 5);
        assert_eq!(daily_reset_time(&connection).expect("daily_reset_time should read"), (3, 0));
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let connection = seeded();
        connection
            .execute_batch(
                "UPDATE setting SET setting_value = '{\"seconds\":90}' WHERE setting_name = 'blip_time';
                 UPDATE setting SET setting_value = '{\"hour\":30,\"minute\":-4}' \
                 WHERE setting_name = 'daily_reset_time';
                 UPDATE setting SET setting_value = '{\"paired\":true}' WHERE setting_name = 'paired';",
            )
            .expect("the rows should be writable");
        assert_eq!(blip_seconds(&connection).expect("blip_time should read"), 30);
        assert_eq!(daily_reset_time(&connection).expect("daily_reset_time should read"), (23, 0));
        assert!(is_cube_paired(&connection).expect("paired should read"));
    }

    #[test]
    fn seconds_are_shown_unless_the_setting_turns_them_off() {
        let connection = seeded();
        assert!(shows_seconds(&connection).expect("display_seconds should read"));
        connection
            .execute(
                "UPDATE setting SET setting_value = '{\"enabled\":false}' WHERE setting_name = 'display_seconds'",
                [],
            )
            .expect("the row should be writable");
        assert!(!shows_seconds(&connection).expect("display_seconds should read"));
    }

    fn seeded() -> Connection {
        let path = std::env::temp_dir().join(format!(
            "facet-setting-test-{}-{:?}.sqlite",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_file(&path);
        database::open(&path, database::APPDATA_DDL).expect("the app DDL should apply")
    }
}
