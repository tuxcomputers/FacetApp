//! Time entries: finished segments that count as tracked time.
//!
//! A segment becomes a `time_entry` when it is finalised, not paused, and at least the blip threshold long
//! (`setting.blip_time`, where 0 counts everything). The entry takes the category its face holds at the
//! moment it is created. A segment that will never earn an entry is marked `processed` so it is not asked
//! about again.

use rusqlite::{Connection, OptionalExtension, params};

use crate::debug_log::{Record, Tag};
use crate::{face, setting};

/// What considering one segment came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consideration {
    Created {
        time_entry_id: i64,
        category_id: i64,
    },
    AlreadyRecorded,
    StillRunning,
    Paused,
    /// Shorter than the blip threshold, in seconds.
    Blip(i64),
    NoSuchSegment,
}

/// Decides whether segment `device_event_id` becomes a time entry, and writes the entry if it does.
///
/// The entry and the segment's `processed` flag are written in one transaction. An unassigned face files
/// the entry under Unassigned (id 0).
pub fn consider(
    connection: &Connection,
    device_event_id: i64,
    log: &impl Record,
) -> Result<Consideration, rusqlite::Error> {
    let blip = setting::blip_seconds(connection)?;
    let segment = connection
        .query_row(
            "SELECT device_face, start_epoch, duration_seconds, paused, finalised, timezone_id \
             FROM device_event WHERE device_event_id = ?1",
            params![device_event_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()?;
    let Some((device_face, start_epoch, duration, is_paused, is_finalised, timezone_id)) = segment else {
        log.record(Tag::Entry, || format!("time_entry: no device_event {device_event_id} to consider"));
        return Ok(Consideration::NoSuchSegment);
    };
    let existing = connection
        .query_row(
            "SELECT time_entry_id FROM time_entry WHERE device_event_id = ?1",
            params![device_event_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    if existing.is_some() {
        return Ok(Consideration::AlreadyRecorded);
    }
    let ignored = if !is_finalised {
        Some(Consideration::StillRunning)
    } else if is_paused {
        Some(Consideration::Paused)
    } else if blip > 0 && duration < blip as f64 {
        Some(Consideration::Blip(blip))
    } else {
        None
    };
    if let Some(reason) = ignored {
        if reason != Consideration::StillRunning {
            connection.execute(
                "UPDATE device_event SET processed = 1 WHERE device_event_id = ?1",
                params![device_event_id],
            )?;
        }
        log.record(Tag::Entry, || format!("time_entry: device_event {device_event_id} ignored, {reason:?}"));
        return Ok(reason);
    }

    let category_id = face::category_id(connection, device_face)?.unwrap_or(0);
    let whole_seconds = duration as i64;
    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        "INSERT INTO time_entry (category_id, device_event_id, started_at, start_timezone_id, \
                                 ended_at, end_timezone_id, duration_seconds) \
         VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%S', ?3, 'unixepoch', 'localtime'), ?4, \
                 strftime('%Y-%m-%dT%H:%M:%S', ?5, 'unixepoch', 'localtime'), ?4, ?6)",
        params![
            category_id,
            device_event_id,
            start_epoch,
            timezone_id,
            start_epoch + whole_seconds,
            duration
        ],
    )?;
    let time_entry_id = transaction.last_insert_rowid();
    transaction.execute(
        "UPDATE device_event SET processed = 1 WHERE device_event_id = ?1",
        params![device_event_id],
    )?;
    transaction.commit()?;
    log.record(Tag::Entry, || {
        format!(
            "time_entry created id={time_entry_id} from device_event {device_event_id} \
             face={device_face} category={category_id} dur={whole_seconds}s"
        )
    });
    Ok(Consideration::Created { time_entry_id, category_id })
}

/// Seconds of `category_id`'s time entries falling between `window_start` and `now`, each entry clipped to
/// that range.
pub fn seconds_in_window(
    connection: &Connection,
    category_id: i64,
    window_start: i64,
    now: i64,
) -> Result<i64, rusqlite::Error> {
    let total: f64 = connection.query_row(
        "SELECT IFNULL(SUM(MAX(0, MIN(de.start_epoch + te.duration_seconds, ?3) - MAX(de.start_epoch, ?2))), 0) \
         FROM time_entry te JOIN device_event de ON de.device_event_id = te.device_event_id \
         WHERE te.category_id = ?1 AND (de.start_epoch + te.duration_seconds) > ?2 AND de.start_epoch < ?3",
        params![category_id, window_start, now],
        |row| row.get(0),
    )?;
    Ok(total as i64)
}

/// When `category_id` was last timed: the latest end of any of its time entries, as unix seconds. `None` when
/// it has none.
pub fn last_used(connection: &Connection, category_id: i64) -> Result<Option<i64>, rusqlite::Error> {
    let latest: Option<f64> = connection.query_row(
        "SELECT MAX(de.start_epoch + te.duration_seconds) FROM time_entry te \
         JOIN device_event de ON de.device_event_id = te.device_event_id WHERE te.category_id = ?1",
        params![category_id],
        |row| row.get(0),
    )?;
    Ok(latest.map(|seconds| seconds as i64).filter(|&seconds| seconds > 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segment;
    use crate::testing::seeded;

    const NO_LOG: Option<crate::debug_log::DebugLog> = None;

    fn meeting(connection: &Connection) -> i64 {
        face::category_id(connection, 2).expect("face 2 should read").expect("face 2 holds Meeting")
    }

    fn run(connection: &Connection, face: i64, from: i64, to: i64) -> i64 {
        let id = segment::start_segment(connection, face, from, &NO_LOG).expect("should start");
        segment::close_open_segment(connection, to, &NO_LOG).expect("should close");
        id
    }

    #[test]
    fn a_finished_segment_becomes_an_entry_under_the_category_its_face_holds() {
        let connection = seeded();
        face::assign(&connection, meeting(&connection), 13).expect("should assign");
        let id = run(&connection, 13, 1_000, 1_060);
        let (category, duration): (i64, f64) = connection
            .query_row(
                "SELECT category_id, duration_seconds FROM time_entry WHERE device_event_id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("the entry should exist");
        assert_eq!((category, duration), (meeting(&connection), 60.0));
        assert_eq!(consider(&connection, id, &NO_LOG).expect("should run"), Consideration::AlreadyRecorded);
    }

    #[test]
    fn a_blip_earns_no_entry_and_is_marked_processed() {
        let connection = seeded();
        let id = run(&connection, 13, 1_000, 1_004);
        let (entries, processed): (i64, bool) = connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM time_entry), processed FROM device_event WHERE device_event_id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("should read");
        assert_eq!((entries, processed), (0, true));
    }

    #[test]
    fn a_blip_threshold_of_zero_counts_everything() {
        let connection = seeded();
        connection
            .execute(
                "UPDATE setting SET setting_value = '{\"seconds\":0}' WHERE setting_name = 'blip_time'",
                [],
            )
            .expect("should write");
        let id = run(&connection, 13, 1_000, 1_000);
        assert!(matches!(
            consider(&connection, id, &NO_LOG).expect("should run"),
            Consideration::AlreadyRecorded
        ));
    }

    #[test]
    fn an_open_segment_is_still_running() {
        let connection = seeded();
        let id = segment::start_segment(&connection, 13, 1_000, &NO_LOG).expect("should start");
        assert_eq!(consider(&connection, id, &NO_LOG).expect("should run"), Consideration::StillRunning);
    }

    #[test]
    fn last_used_is_the_end_of_the_latest_entry() {
        let connection = seeded();
        let category = meeting(&connection);
        assert_eq!(last_used(&connection, category).expect("should read"), None);
        face::assign(&connection, category, 13).expect("should assign");
        run(&connection, 13, 1_000, 1_100);
        run(&connection, 13, 2_000, 2_050);
        assert_eq!(last_used(&connection, category).expect("should read"), Some(2_050));
    }

    #[test]
    fn window_seconds_clip_each_entry_to_the_window() {
        let connection = seeded();
        let category = meeting(&connection);
        face::assign(&connection, category, 13).expect("should assign");
        run(&connection, 13, 1_000, 1_100);
        run(&connection, 13, 2_000, 2_050);
        assert_eq!(seconds_in_window(&connection, category, 0, 10_000).expect("should sum"), 150);
        assert_eq!(seconds_in_window(&connection, category, 1_050, 10_000).expect("should sum"), 100);
        assert_eq!(seconds_in_window(&connection, category, 0, 2_010).expect("should sum"), 110);
    }
}
