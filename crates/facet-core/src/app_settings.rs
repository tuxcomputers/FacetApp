//! The App tab's settings: what each control shows, and writing one field of one `setting` row with the write
//! checked by reading it back.

use rusqlite::{Connection, OptionalExtension, params};

use crate::debug_log::{Record, Tag, plain};
use crate::setting;

/// The daily reset control's range, on a 12-hour face.
pub const RESET_HOUR_RANGE: (i64, i64) = (1, 12);
/// The fetch interval control's range, in minutes.
pub const FETCH_MINUTES_RANGE: (i64, i64) = (1, 60);
/// The blip control's range, in seconds. 0 turns the filter off.
pub const BLIP_SECONDS_RANGE: (i64, i64) = (0, 30);

/// A value to store in one JSON field of a setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Flag(bool),
    Number(i64),
    Text(String),
}

impl Value {
    /// How the value appears in the debug log: `flag(true)`, `number(3)` or `text(...)`.
    fn describe(&self) -> String {
        match self {
            Value::Flag(flag) => format!("flag({flag})"),
            Value::Number(number) => format!("number({number})"),
            Value::Text(text) => format!("text({})", plain(text)),
        }
    }
}

/// Writes `value` into field `field` of setting `name`, leaving its other fields as they are, then reads the
/// field back. Returns whether the table now holds `value`. A missing row is not created, and reads back as
/// refused. Logs `App setting <name>.<field> -> <value>`, with ` REFUSED, the table does not hold it` on a
/// refusal.
pub fn write(
    connection: &Connection,
    name: &str,
    field: &str,
    value: &Value,
    log: &impl Record,
) -> Result<bool, rusqlite::Error> {
    let path = format!("$.{field}");
    match value {
        Value::Flag(flag) => connection.execute(
            "UPDATE setting SET setting_value = json_set(setting_value, ?2, json(?3)) WHERE setting_name = ?1",
            params![name, path, if *flag { "true" } else { "false" }],
        )?,
        Value::Number(number) => connection.execute(
            "UPDATE setting SET setting_value = json_set(setting_value, ?2, ?3) WHERE setting_name = ?1",
            params![name, path, number],
        )?,
        Value::Text(text) => connection.execute(
            "UPDATE setting SET setting_value = json_set(setting_value, ?2, ?3) WHERE setting_name = ?1",
            params![name, path, text],
        )?,
    };
    let stored = read_back(connection, name, &path, value)?;
    log.record(Tag::Settings, || {
        format!(
            "App setting {name}.{field} -> {}{}",
            value.describe(),
            if stored { "" } else { " REFUSED, the table does not hold it" }
        )
    });
    Ok(stored)
}

fn read_back(
    connection: &Connection,
    name: &str,
    path: &str,
    value: &Value,
) -> Result<bool, rusqlite::Error> {
    let row = connection
        .query_row(
            "SELECT json_type(setting_value, ?2), json_extract(setting_value, ?2) FROM setting WHERE setting_name = ?1",
            params![name, path],
            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, rusqlite::types::Value>(1)?)),
        )
        .optional()?;
    let Some((kind, stored)) = row else { return Ok(false) };
    use rusqlite::types::Value as Sql;
    Ok(match (value, kind.as_deref(), stored) {
        (Value::Flag(true), Some("true"), _) | (Value::Flag(false), Some("false"), _) => true,
        (Value::Number(number), Some("integer"), Sql::Integer(held)) => held == *number,
        (Value::Text(text), Some("text"), Sql::Text(held)) => held == *text,
        _ => false,
    })
}

/// How often history is fetched, in seconds, from `setting.fetch_history_interval_seconds.seconds`. An absent
/// row or field reads as 10.
pub fn fetch_interval_seconds(connection: &Connection) -> Result<i64, rusqlite::Error> {
    let value = connection
        .query_row(
            "SELECT json_extract(setting_value, '$.seconds') FROM setting \
             WHERE setting_name = 'fetch_history_interval_seconds'",
            [],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()?;
    Ok(value.flatten().unwrap_or(10))
}

/// The hour a 24-hour value shows as on the 12-hour face: 0 and 12 show as 12.
pub fn hour12(hour24: i64) -> i64 {
    match hour24.rem_euclid(12) {
        0 => 12,
        hour => hour,
    }
}

/// The 24-hour value stored for a face value: always the morning, so 12 stores 0.
pub fn hour24(face: i64) -> i64 {
    face.rem_euclid(12)
}

/// The whole minutes a stored interval shows as, never below 1.
pub fn fetch_minutes(seconds: i64) -> i64 {
    (seconds / 60).max(1)
}

/// `min` for 1, `mins` otherwise.
pub fn minutes_word(minutes: i64) -> &'static str {
    if minutes == 1 { "min" } else { "mins" }
}

/// `sec` for 1, `secs` otherwise.
pub fn seconds_word(seconds: i64) -> &'static str {
    if seconds == 1 { "sec" } else { "secs" }
}

/// The App settings section's values, read from the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Values {
    pub shows_seconds: bool,
    /// On the 12-hour face.
    pub reset_hour: i64,
    pub fetch_minutes: i64,
    pub blip_seconds: i64,
}

/// Reads the App settings section.
pub fn read(connection: &Connection) -> Result<Values, rusqlite::Error> {
    Ok(Values {
        shows_seconds: setting::shows_seconds(connection)?,
        reset_hour: hour12(setting::daily_reset_time(connection)?.0),
        fetch_minutes: fetch_minutes(fetch_interval_seconds(connection)?),
        blip_seconds: setting::blip_seconds(connection)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::seeded;

    const NO_LOG: Option<crate::debug_log::DebugLog> = None;

    #[test]
    fn a_seeded_database_reads_as_the_defaults() {
        let connection = seeded();
        assert_eq!(
            read(&connection).expect("should read"),
            Values { shows_seconds: true, reset_hour: 3, fetch_minutes: 1, blip_seconds: 5 }
        );
    }

    #[test]
    fn each_kind_of_value_is_written_and_read_back_leaving_the_other_fields() {
        let connection = seeded();
        assert!(
            write(&connection, "display_seconds", "enabled", &Value::Flag(false), &NO_LOG)
                .expect("should write")
        );
        assert!(!setting::shows_seconds(&connection).expect("should read"));
        assert!(
            write(&connection, "daily_reset_time", "hour", &Value::Number(7), &NO_LOG).expect("should write")
        );
        assert_eq!(setting::daily_reset_time(&connection).expect("should read"), (7, 0));
        assert!(
            write(&connection, "debug", "directory", &Value::Text("~/Traces".into()), &NO_LOG)
                .expect("write")
        );
        let trace = setting::debug_trace(&connection).expect("should read");
        assert_eq!(trace.directory, "~/Traces");
        assert!(!trace.enabled);
    }

    #[test]
    fn a_missing_row_reads_back_as_refused() {
        let connection = seeded();
        assert!(
            !write(&connection, "no_such_setting", "enabled", &Value::Flag(true), &NO_LOG)
                .expect("should run")
        );
    }

    #[test]
    fn the_face_values_convert_as_the_swift_rules_do() {
        assert_eq!((hour12(0), hour12(3), hour12(12), hour12(15)), (12, 3, 12, 3));
        assert_eq!((hour24(12), hour24(3)), (0, 3));
        assert_eq!((fetch_minutes(10), fetch_minutes(60), fetch_minutes(300)), (1, 1, 5));
        assert_eq!(
            (minutes_word(1), minutes_word(2), seconds_word(1), seconds_word(0)),
            ("min", "mins", "sec", "secs")
        );
    }
}
