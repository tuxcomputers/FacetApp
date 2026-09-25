//! Segments: rows of `device_event`, each one stretch of time on one face.
//!
//! A segment is open while `finalised = 0`. The app writes segments only on its own faces (13 and 14), and
//! never closes or measures a segment on a cube face: a cube reports its own durations.
//!
//! Times are whole unix seconds. `start_time` is written as local time, `yyyy-mm-ddThh:mm:ss`.

use rusqlite::{Connection, OptionalExtension, params};

use crate::debug_log::{Record, Tag};
use crate::face;
use crate::time_entry;

/// One `device_event` row, in the columns timing needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub device_event_id: i64,
    pub event_number: i64,
    pub face: i64,
    pub start_epoch: i64,
    pub is_paused: bool,
}

const COLUMNS: &str = "device_event_id, event_number, device_face, start_epoch, paused";

fn from_row(row: &rusqlite::Row<'_>) -> Result<Segment, rusqlite::Error> {
    Ok(Segment {
        device_event_id: row.get(0)?,
        event_number: row.get(1)?,
        face: row.get(2)?,
        start_epoch: row.get(3)?,
        is_paused: row.get(4)?,
    })
}

/// The open segment. When more than one row is open, the newest by `start_epoch` then id.
pub fn open_segment(connection: &Connection) -> Result<Option<Segment>, rusqlite::Error> {
    connection
        .query_row(
            &format!(
                "SELECT {COLUMNS} FROM device_event WHERE finalised = 0 \
                 ORDER BY start_epoch DESC, device_event_id DESC LIMIT 1"
            ),
            [],
            from_row,
        )
        .optional()
}

/// The face of the most recently recorded segment (by id, open or closed) on any of `faces`.
pub fn latest_face_in(connection: &Connection, faces: &[i64]) -> Result<Option<i64>, rusqlite::Error> {
    if faces.is_empty() {
        return Ok(None);
    }
    let list = faces.iter().map(i64::to_string).collect::<Vec<_>>().join(",");
    connection
        .query_row(
            &format!(
                "SELECT device_face FROM device_event WHERE device_face IN ({list}) \
                 ORDER BY device_event_id DESC LIMIT 1"
            ),
            [],
            |row| row.get(0),
        )
        .optional()
}

/// The app face in use: the face of the last app segment, or the first app face when there has been none.
pub fn current_app_face(connection: &Connection) -> Result<i64, rusqlite::Error> {
    Ok(latest_face_in(connection, &face::APP_FACES)?.unwrap_or(face::APP_FACES[0]))
}

/// Opens a new unpaused segment on `face` starting at `now`, and returns its id.
///
/// Every row still open is finalised first, in the same transaction, keeping the duration it has; each of
/// those is then offered to [`time_entry::consider`]. The event number is `now`, or one more than the
/// highest already used within that second.
///
/// `timezone_id` is written as 0 (Unknown): the core has no way to name this machine's time zone.
pub fn start_segment(
    connection: &Connection,
    face: i64,
    now: i64,
    log: &impl Record,
) -> Result<i64, rusqlite::Error> {
    let transaction = connection.unchecked_transaction()?;
    let stranded = open_row_ids(&transaction, false)?;
    transaction.execute("UPDATE device_event SET finalised = 1 WHERE finalised != 1", [])?;
    let highest: i64 = transaction.query_row(
        "SELECT IFNULL(MAX(event_number), 0) FROM device_event WHERE start_epoch = ?1",
        params![now],
        |row| row.get(0),
    )?;
    let event_number = now.max(highest + 1);
    transaction.execute(
        "INSERT INTO device_event (event_number, event_type_id, device_face, start_time, timezone_id, \
                                   start_epoch, duration_seconds, paused, finalised) \
         VALUES (?1, (SELECT event_type_id FROM event_type WHERE event_name = 'face_flip'), ?2, \
                 strftime('%Y-%m-%dT%H:%M:%S', ?3, 'unixepoch', 'localtime'), 0, ?3, 0, 0, 0)",
        params![event_number, face, now],
    )?;
    let id = transaction.last_insert_rowid();
    transaction.commit()?;
    log.record(Tag::Event, || format!("device_event opened id={id} face={face} event_number={event_number}"));
    for stranded_id in stranded {
        time_entry::consider(connection, stranded_id, log)?;
    }
    Ok(id)
}

/// Closes the open segment as of `now`, with its duration as whole seconds since its start (never
/// negative), and offers it to [`time_entry::consider`]. Returns the id closed.
///
/// `None` when nothing is open, or when the open segment is on a cube face, which is left open.
pub fn close_open_segment(
    connection: &Connection,
    now: i64,
    log: &impl Record,
) -> Result<Option<i64>, rusqlite::Error> {
    let Some(open) = open_segment(connection)? else {
        return Ok(None);
    };
    if !face::is_app_face(open.face) {
        log.record(Tag::Event, || {
            format!(
                "device_event id={} is on cube face {}, so it is left open",
                open.device_event_id, open.face
            )
        });
        return Ok(None);
    }
    let duration = (now - open.start_epoch).max(0);
    let changed = connection.execute(
        "UPDATE device_event SET duration_seconds = ?1, finalised = 1 WHERE device_event_id = ?2",
        params![duration, open.device_event_id],
    )?;
    if changed == 0 {
        log.record_failure(Tag::Event, || {
            format!("device_event id={} would not close", open.device_event_id)
        });
        return Ok(None);
    }
    log.record(Tag::Event, || format!("device_event closed id={} after {duration}s", open.device_event_id));
    time_entry::consider(connection, open.device_event_id, log)?;
    Ok(Some(open.device_event_id))
}

/// Writes the open app-face segment's duration as of `now`, leaving it open. Does nothing when nothing is
/// open or the open segment is on a cube face.
pub fn refresh_open_segment(connection: &Connection, now: i64) -> Result<(), rusqlite::Error> {
    if let Some(open) = open_segment(connection)?.filter(|open| face::is_app_face(open.face)) {
        connection.execute(
            "UPDATE device_event SET duration_seconds = ?1 WHERE device_event_id = ?2",
            params![(now - open.start_epoch).max(0), open.device_event_id],
        )?;
    }
    Ok(())
}

/// Finalises every open segment on an app face, keeping the duration each already has, and offers each to
/// [`time_entry::consider`]. Returns their ids.
///
/// For launch: a segment left open by a launch that ended without closing it would otherwise be measured to
/// now, counting the time the app was not running.
pub fn close_stranded_on_app_faces(
    connection: &Connection,
    log: &impl Record,
) -> Result<Vec<i64>, rusqlite::Error> {
    let stranded = open_row_ids(connection, true)?;
    if stranded.is_empty() {
        return Ok(stranded);
    }
    connection.execute(
        "UPDATE device_event SET finalised = 1 WHERE finalised = 0 AND device_face > ?1",
        params![face::HIGHEST_DEVICE_FACE],
    )?;
    log.record(Tag::Event, || {
        format!("device_event closed {} left open by an earlier launch: {stranded:?}", stranded.len())
    });
    for &id in &stranded {
        time_entry::consider(connection, id, log)?;
    }
    Ok(stranded)
}

fn open_row_ids(connection: &Connection, only_app_faces: bool) -> Result<Vec<i64>, rusqlite::Error> {
    let filter = if only_app_faces {
        format!(" AND device_face > {}", face::HIGHEST_DEVICE_FACE)
    } else {
        String::new()
    };
    let mut statement = connection
        .prepare(&format!("SELECT device_event_id FROM device_event WHERE finalised != 1{filter}"))?;
    statement.query_map([], |row| row.get(0))?.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::seeded;

    const NO_LOG: Option<crate::debug_log::DebugLog> = None;

    fn duration(connection: &Connection, id: i64) -> (f64, bool) {
        connection
            .query_row(
                "SELECT duration_seconds, finalised FROM device_event WHERE device_event_id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("the row should read")
    }

    #[test]
    fn a_fresh_database_has_nothing_open_and_uses_the_first_app_face() {
        let connection = seeded();
        assert_eq!(open_segment(&connection).expect("should read"), None);
        assert_eq!(current_app_face(&connection).expect("should read"), 13);
    }

    #[test]
    fn a_started_segment_is_open_on_its_face_with_the_epoch_as_its_event_number() {
        let connection = seeded();
        let id = start_segment(&connection, 13, 1_000, &NO_LOG).expect("should start");
        let open = open_segment(&connection).expect("should read").expect("a segment should be open");
        assert_eq!(
            open,
            Segment {
                device_event_id: id,
                event_number: 1_000,
                face: 13,
                start_epoch: 1_000,
                is_paused: false
            }
        );
        assert_eq!(current_app_face(&connection).expect("should read"), 13);
    }

    #[test]
    fn two_segments_in_one_second_take_different_event_numbers() {
        let connection = seeded();
        start_segment(&connection, 13, 1_000, &NO_LOG).expect("should start");
        start_segment(&connection, 14, 1_000, &NO_LOG).expect("should start");
        let open = open_segment(&connection).expect("should read").expect("a segment should be open");
        assert_eq!((open.face, open.event_number), (14, 1_001));
    }

    #[test]
    fn closing_measures_whole_seconds_from_the_start() {
        let connection = seeded();
        let id = start_segment(&connection, 13, 1_000, &NO_LOG).expect("should start");
        assert_eq!(close_open_segment(&connection, 1_042, &NO_LOG).expect("should close"), Some(id));
        assert_eq!(duration(&connection, id), (42.0, true));
        assert_eq!(open_segment(&connection).expect("should read"), None);
    }

    #[test]
    fn a_clock_that_went_backwards_closes_at_zero() {
        let connection = seeded();
        let id = start_segment(&connection, 13, 1_000, &NO_LOG).expect("should start");
        close_open_segment(&connection, 900, &NO_LOG).expect("should close");
        assert_eq!(duration(&connection, id), (0.0, true));
    }

    #[test]
    fn a_cube_segment_is_left_open() {
        let connection = seeded();
        let id = start_segment(&connection, 5, 1_000, &NO_LOG).expect("should start");
        assert_eq!(close_open_segment(&connection, 1_100, &NO_LOG).expect("should run"), None);
        assert_eq!(duration(&connection, id), (0.0, false));
    }

    #[test]
    fn starting_a_segment_finalises_any_left_open() {
        let connection = seeded();
        let first = start_segment(&connection, 13, 1_000, &NO_LOG).expect("should start");
        start_segment(&connection, 14, 1_010, &NO_LOG).expect("should start");
        assert!(duration(&connection, first).1);
    }

    #[test]
    fn refreshing_writes_the_duration_and_leaves_it_open() {
        let connection = seeded();
        let id = start_segment(&connection, 13, 1_000, &NO_LOG).expect("should start");
        refresh_open_segment(&connection, 1_030).expect("should refresh");
        assert_eq!(duration(&connection, id), (30.0, false));
    }

    #[test]
    fn a_stranded_segment_is_closed_with_the_duration_it_already_had() {
        let connection = seeded();
        let id = start_segment(&connection, 14, 1_000, &NO_LOG).expect("should start");
        refresh_open_segment(&connection, 1_020).expect("should refresh");
        let cube = start_segment(&connection, 3, 2_000, &NO_LOG);
        // Starting the cube segment finalised the app one, so reopen it to model a crash.
        connection
            .execute("UPDATE device_event SET finalised = 0 WHERE device_event_id = ?1", params![id])
            .expect("reopen");
        assert!(cube.is_ok());
        assert_eq!(close_stranded_on_app_faces(&connection, &NO_LOG).expect("should close"), vec![id]);
        assert_eq!(duration(&connection, id), (20.0, true));
        assert_eq!(open_segment(&connection).expect("should read").map(|s| s.face), Some(3));
    }
}
