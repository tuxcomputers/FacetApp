//! Filing the cube's history into `device_event`.
//!
//! A row is identified by `(event_number, start_epoch)`, so writing a frame already on record updates it in place.
//! Only the newest cube event is open; every row a newer event finalises, and every row written closed, is handed to
//! [`time_entry::consider`]. A cube row's duration comes only from the cube.

use rusqlite::{Connection, OptionalExtension, params};

use crate::debug_log::{Record, Tag};
use crate::device::history::Frame;
use crate::time_entry;

/// Where the next fetch resumes: the newest cube row (faces 1 to 12) by start, then event number, as
/// `(event_number, start_epoch)`. `None` when no cube row has been recorded.
pub fn resume_point(connection: &Connection) -> Result<Option<(u32, u64)>, rusqlite::Error> {
    connection
        .query_row(
            "SELECT event_number, start_epoch FROM device_event WHERE device_face BETWEEN 1 AND 12 \
             ORDER BY start_epoch DESC, event_number DESC, device_event_id DESC LIMIT 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map(|point| point.map(|(number, start)| (number.max(0) as u32, start.max(0) as u64)))
}

/// Writes `frame`, open when `is_open` and it is the newest cube event, and returns the rows it finalised, which have
/// been offered to [`time_entry::consider`]. A frame newer than every cube row finalises every open row first, the
/// app's own included.
pub fn record(
    connection: &Connection,
    frame: &Frame,
    is_open: bool,
    log: &impl Record,
) -> Result<Vec<i64>, rusqlite::Error> {
    let mark = resume_point(connection)?;
    let is_newest =
        mark.is_none_or(|(number, start)| (frame.start_epoch, frame.event_number) >= (start, number));
    let event_type = if frame.is_paused { "pause" } else { "face_flip" };
    let transaction = connection.unchecked_transaction()?;
    let existing: Option<(i64, bool)> = transaction
        .query_row(
            "SELECT device_event_id, finalised FROM device_event WHERE event_number = ?1 AND start_epoch = ?2",
            params![frame.event_number, frame.start_epoch as i64],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let mut finalised_now: Vec<i64> = Vec::new();
    let (id, verb, open) = match existing {
        Some((id, was_finalised)) => {
            let open = is_open && is_newest;
            transaction.execute(
                "UPDATE device_event SET event_type_id = (SELECT event_type_id FROM event_type WHERE event_name = ?2), \
                 device_face = ?3, duration_seconds = ?4, paused = ?5, finalised = ?6 WHERE device_event_id = ?1",
                params![id, event_type, frame.face, frame.duration_seconds, frame.is_paused, !open],
            )?;
            if !open && !was_finalised {
                finalised_now.push(id);
            }
            (id, "updated", open)
        }
        None => {
            if is_newest {
                let mut statement =
                    transaction.prepare("SELECT device_event_id FROM device_event WHERE finalised = 0")?;
                let stranded =
                    statement.query_map([], |row| row.get::<_, i64>(0))?.collect::<Result<Vec<_>, _>>()?;
                drop(statement);
                transaction.execute("UPDATE device_event SET finalised = 1 WHERE finalised = 0", [])?;
                finalised_now.extend(stranded);
            }
            let open = is_open && is_newest;
            transaction.execute(
                "INSERT INTO device_event (event_number, event_type_id, device_face, start_time, timezone_id, \
                                           start_epoch, duration_seconds, paused, finalised) \
                 VALUES (?1, (SELECT event_type_id FROM event_type WHERE event_name = ?2), ?3, \
                         strftime('%Y-%m-%dT%H:%M:%S', ?4, 'unixepoch', 'localtime'), 0, ?4, ?5, ?6, ?7)",
                params![
                    frame.event_number,
                    event_type,
                    frame.face,
                    frame.start_epoch as i64,
                    frame.duration_seconds,
                    frame.is_paused,
                    !open
                ],
            )?;
            let id = transaction.last_insert_rowid();
            if !open {
                finalised_now.push(id);
            }
            (id, "inserted", open)
        }
    };
    transaction.commit()?;
    log.record(Tag::Event, || {
        format!(
            "device_event {verb} id={id} ev={} face={} dur={}s paused={} open={open}",
            frame.event_number, frame.face, frame.duration_seconds, frame.is_paused
        )
    });
    for finalised in &finalised_now {
        time_entry::consider(connection, *finalised, log)?;
    }
    Ok(finalised_now)
}

/// Writes a planned batch in order, the last frame open, stopping at the first refusal. Returns how many landed.
pub fn record_batch(
    connection: &Connection,
    frames: &[Frame],
    log: &impl Record,
) -> Result<usize, rusqlite::Error> {
    for (index, frame) in frames.iter().enumerate() {
        record(connection, frame, index + 1 == frames.len(), log)?;
    }
    Ok(frames.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_log::Trace;
    use crate::testing::seeded;

    fn frame(event_number: u32, face: u8, start_epoch: u64, duration_seconds: u32) -> Frame {
        Frame { event_number, face, is_paused: false, start_epoch, duration_seconds }
    }

    fn rows(connection: &Connection) -> Vec<(i64, i64, f64, bool)> {
        let mut statement = connection
            .prepare("SELECT event_number, device_face, duration_seconds, finalised FROM device_event ORDER BY event_number")
            .expect("prepare");
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
            .expect("query")
            .collect::<Result<Vec<_>, _>>()
            .expect("rows")
    }

    #[test]
    fn the_newest_event_is_open_and_its_duration_grows_in_place() {
        let connection = seeded();
        let log = Trace::none();
        let start = 1_790_000_000;
        record_batch(&connection, &[frame(1, 2, start, 30), frame(2, 8, start + 30, 5)], &log)
            .expect("batch");
        assert_eq!(rows(&connection), vec![(1, 2, 30.0, true), (2, 8, 5.0, false)]);
        assert_eq!(resume_point(&connection).expect("read"), Some((2, start + 30)));

        // The same event again, longer: updated in place, still open.
        record(&connection, &frame(2, 8, start + 30, 40), true, &log).expect("refresh");
        assert_eq!(rows(&connection), vec![(1, 2, 30.0, true), (2, 8, 40.0, false)]);

        // A newer event closes it, and the closed one becomes a time entry on face 8's category.
        record(&connection, &frame(3, 2, start + 70, 0), true, &log).expect("turn");
        assert_eq!(rows(&connection).iter().filter(|row| !row.3).count(), 1);
        let entries: i64 =
            connection.query_row("SELECT COUNT(*) FROM time_entry", [], |row| row.get(0)).expect("count");
        assert_eq!(entries, 2);
    }

    #[test]
    fn an_older_event_arriving_late_is_filed_closed_and_leaves_the_open_one() {
        let connection = seeded();
        let log = Trace::none();
        record(&connection, &frame(5, 2, 2_000, 10), true, &log).expect("open");
        record(&connection, &frame(4, 8, 1_900, 100), false, &log).expect("late");
        assert_eq!(rows(&connection), vec![(4, 8, 100.0, true), (5, 2, 10.0, false)]);
    }
}
