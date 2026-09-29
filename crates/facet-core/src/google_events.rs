//! Time entries into the Google calendar Facet owns: which entries are waiting, the event each becomes, sending it,
//! reading it back, and ticking the entry only once what Google kept matches.
//!
//! An event's id is derived from its time entry (`facet<id>`), so a second insert of one already there is answered
//! 409 and taken as Facet meeting its own earlier work. Times go as UTC; Google shows them in the calendar's zone.
//! JSON is built and read with sqlite's own functions, as the rest of the Google code does.

use rusqlite::{Connection, OptionalExtension, params};

use crate::google;
use crate::port::{Http, HttpResponse};

/// The most entries one pass takes.
pub const BATCH: usize = 50;

const CALENDARS: &str = "https://www.googleapis.com/calendar/v3/calendars";

/// A time entry not yet in the calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub time_entry_id: i64,
    pub device_event_id: i64,
    pub category_name: String,
    pub start_epoch: i64,
    pub end_epoch: i64,
}

/// The time entries with `synced_to_google_calendar = 0`, oldest first, at most `limit`.
pub fn pending(connection: &Connection, limit: usize) -> Result<Vec<Pending>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT te.time_entry_id, te.device_event_id, c.category_name, de.start_epoch, te.duration_seconds \
           FROM time_entry te \
           JOIN device_event de ON de.device_event_id = te.device_event_id \
           JOIN category c ON c.category_id = te.category_id \
          WHERE te.synced_to_google_calendar = 0 \
          ORDER BY de.start_epoch, te.time_entry_id LIMIT ?1",
    )?;
    let rows = statement.query_map(params![limit as i64], |row| {
        let start: i64 = row.get(3)?;
        let duration: f64 = row.get(4)?;
        Ok(Pending {
            time_entry_id: row.get(0)?,
            device_event_id: row.get(1)?,
            category_name: row.get(2)?,
            start_epoch: start,
            end_epoch: start + duration.round() as i64,
        })
    })?;
    rows.collect()
}

/// Ticks `time_entry_id` as in the calendar, and reads it back. Returns whether the table holds the tick.
pub fn mark_synced(connection: &Connection, time_entry_id: i64) -> Result<bool, rusqlite::Error> {
    connection.execute(
        "UPDATE time_entry SET synced_to_google_calendar = 1 WHERE time_entry_id = ?1",
        params![time_entry_id],
    )?;
    let ticked: Option<bool> = connection
        .query_row(
            "SELECT synced_to_google_calendar FROM time_entry WHERE time_entry_id = ?1",
            params![time_entry_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(ticked == Some(true))
}

/// The event a time entry should become.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expected {
    pub event_id: String,
    pub summary: String,
    pub description: String,
    pub start_epoch: i64,
    pub end_epoch: i64,
}

/// The event `entry` should become: its category as the title, and the entry and its device event named in the
/// description.
pub fn expected(entry: &Pending) -> Expected {
    Expected {
        event_id: format!("facet{}", entry.time_entry_id),
        summary: entry.category_name.clone(),
        description: format!(
            "Facet time entry {}\nDevice event {}",
            entry.time_entry_id, entry.device_event_id
        ),
        start_epoch: entry.start_epoch,
        end_epoch: entry.end_epoch,
    }
}

/// What Google holds for an event, as far as the comparison needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub start_epoch: Option<i64>,
    pub end_epoch: Option<i64>,
    pub is_cancelled: bool,
}

/// How an event Google kept differs from the one sent. Checked in the order listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mismatch {
    WrongEvent,
    Cancelled,
    Summary,
    Description,
    Start,
    End,
}

impl Mismatch {
    pub fn describe(self) -> &'static str {
        match self {
            Mismatch::WrongEvent => "it is a different event",
            Mismatch::Cancelled => "the event has been deleted",
            Mismatch::Summary => "the title does not match",
            Mismatch::Description => "the description does not match",
            Mismatch::Start => "the start does not match",
            Mismatch::End => "the end does not match",
        }
    }
}

/// The first way `event` differs from `expected`, or `None` when it is the event that was sent.
pub fn mismatch(event: &Event, expected: &Expected) -> Option<Mismatch> {
    if event.id != expected.event_id {
        Some(Mismatch::WrongEvent)
    } else if event.is_cancelled {
        Some(Mismatch::Cancelled)
    } else if event.summary.as_deref() != Some(expected.summary.as_str()) {
        Some(Mismatch::Summary)
    } else if event.description.as_deref() != Some(expected.description.as_str()) {
        Some(Mismatch::Description)
    } else if event.start_epoch != Some(expected.start_epoch) {
        Some(Mismatch::Start)
    } else if event.end_epoch != Some(expected.end_epoch) {
        Some(Mismatch::End)
    } else {
        None
    }
}

/// Why an event call did not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventFailure {
    /// The event is not in the calendar (404 or 410). The calendar may be gone too; nothing here says which.
    Missing,
    Failed(String),
}

impl EventFailure {
    pub fn describe(&self) -> String {
        match self {
            EventFailure::Missing => "the event is not in the calendar".to_string(),
            EventFailure::Failed(reason) => reason.clone(),
        }
    }
}

/// What an insert came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Insertion {
    Created,
    /// An event with that id is already in the calendar (409): an earlier pass sent it and did not tick the entry.
    AlreadyThere,
}

fn events_url(calendar_id: &str) -> String {
    format!("{CALENDARS}/{}/events", google::percent_encode(calendar_id))
}

fn reason(reply: &HttpResponse) -> String {
    google::json_text(&reply.body, "$.error.message")
        .or_else(|| google::json_text(&reply.body, "$.error"))
        .map_or_else(|| format!("HTTP {}", reply.status), |message| format!("{message} ({})", reply.status))
}

/// The JSON body for `expected`, built by sqlite. Times are UTC, `YYYY-MM-DDTHH:MM:SSZ`.
pub fn body(expected: &Expected) -> Result<String, rusqlite::Error> {
    Connection::open_in_memory()?.query_row(
        "SELECT json_object('id', ?1, 'summary', ?2, 'description', ?3, \
                'start', json_object('dateTime', strftime('%Y-%m-%dT%H:%M:%SZ', ?4, 'unixepoch')), \
                'end', json_object('dateTime', strftime('%Y-%m-%dT%H:%M:%SZ', ?5, 'unixepoch')))",
        params![
            expected.event_id,
            expected.summary,
            expected.description,
            expected.start_epoch,
            expected.end_epoch
        ],
        |row| row.get(0),
    )
}

/// Reads an event from Google's JSON. `None` for a reply with no id. A time carries its own offset, which sqlite
/// turns into seconds since 1970.
pub fn event(json: &str) -> Option<Event> {
    let id = google::json_text(json, "$.id").filter(|id| !id.is_empty())?;
    let epoch = |path: &str| -> Option<i64> {
        let text = google::json_text(json, path)?;
        Connection::open_in_memory()
            .ok()?
            .query_row("SELECT CAST(strftime('%s', ?1) AS INTEGER)", params![text], |row| row.get(0))
            .ok()
            .flatten()
    };
    Some(Event {
        id,
        summary: google::json_text(json, "$.summary"),
        description: google::json_text(json, "$.description"),
        start_epoch: epoch("$.start.dateTime"),
        end_epoch: epoch("$.end.dateTime"),
        is_cancelled: google::json_text(json, "$.status").as_deref() == Some("cancelled"),
    })
}

/// Inserts `expected` into `calendar_id`.
pub fn insert(
    http: &dyn Http,
    token: &str,
    calendar_id: &str,
    expected: &Expected,
) -> Result<Insertion, EventFailure> {
    let body = body(expected)
        .map_err(|error| EventFailure::Failed(format!("the event would not build: {error}")))?;
    let reply = http
        .send("POST", &events_url(calendar_id), Some(token), Some(("application/json", &body)))
        .map_err(EventFailure::Failed)?;
    match reply.status {
        200..=299 => Ok(Insertion::Created),
        409 => Ok(Insertion::AlreadyThere),
        404 | 410 => Err(EventFailure::Missing),
        _ => Err(EventFailure::Failed(reason(&reply))),
    }
}

/// Reads event `event_id` back from `calendar_id`: what Google kept, rather than what it accepted.
pub fn get(http: &dyn Http, token: &str, calendar_id: &str, event_id: &str) -> Result<Event, EventFailure> {
    let url = format!("{}/{}", events_url(calendar_id), google::percent_encode(event_id));
    let reply = http.send("GET", &url, Some(token), None).map_err(EventFailure::Failed)?;
    match reply.status {
        200..=299 => event(&reply.body).ok_or_else(|| EventFailure::Failed("an unreadable reply".into())),
        404 | 410 => Err(EventFailure::Missing),
        _ => Err(EventFailure::Failed(reason(&reply))),
    }
}

/// How one entry fared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivery {
    /// In the calendar and matching; the entry is to be ticked. `already` when an earlier pass had sent it.
    Delivered { already: bool },
    /// Sent, but what Google kept differs: left unticked for a later pass.
    Mismatched(Mismatch),
    /// Sent, but not there to read back: left unticked for a later pass.
    NotThere,
    /// A request failed: the pass stops here.
    Failed(String),
}

/// Sends `entry` to `calendar_id` and reads it back. Call on a background thread.
pub fn deliver(http: &dyn Http, token: &str, calendar_id: &str, entry: &Pending) -> Delivery {
    let expected = expected(entry);
    let already = match insert(http, token, calendar_id, &expected) {
        Ok(insertion) => insertion == Insertion::AlreadyThere,
        Err(failure) => return Delivery::Failed(failure.describe()),
    };
    match get(http, token, calendar_id, &expected.event_id) {
        Ok(kept) => match mismatch(&kept, &expected) {
            Some(difference) => Delivery::Mismatched(difference),
            None => Delivery::Delivered { already },
        },
        Err(EventFailure::Missing) => Delivery::NotThere,
        Err(EventFailure::Failed(reason)) => Delivery::Failed(reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::seeded;
    use std::sync::Mutex;

    /// A calendar in memory: it keeps what it is sent, answers 409 to an id it already has, and reads back.
    struct FakeCalendar(Mutex<Vec<(String, String)>>);

    impl Http for FakeCalendar {
        fn send(
            &self,
            method: &str,
            url: &str,
            _bearer: Option<&str>,
            body: Option<(&str, &str)>,
        ) -> Result<HttpResponse, String> {
            let mut kept = self.0.lock().expect("lock");
            let reply = |status: u16, body: &str| Ok(HttpResponse { status, body: body.to_string() });
            match method {
                "POST" => {
                    let body = body.map(|(_, text)| text.to_string()).unwrap_or_default();
                    let id = google::json_text(&body, "$.id").unwrap_or_default();
                    if kept.iter().any(|(held, _)| *held == id) {
                        return reply(409, "{}");
                    }
                    kept.push((id, body.clone()));
                    reply(200, &body)
                }
                "GET" => {
                    let id = url.rsplit('/').next().unwrap_or_default();
                    match kept.iter().find(|(held, _)| held == id) {
                        Some((_, body)) => reply(200, body),
                        None => reply(404, "{}"),
                    }
                }
                _ => reply(400, "{}"),
            }
        }
    }

    fn entry(connection: &Connection) -> Pending {
        connection
            .execute_batch(
                "INSERT INTO device_event (event_number, event_type_id, device_face, start_time, timezone_id, \
                                           start_epoch, duration_seconds, paused, finalised, processed) \
                 VALUES (1, 1, 8, '2026-09-29T19:00:00', 0, 1790672400, 90, 0, 1, 1); \
                 INSERT INTO time_entry (category_id, device_event_id, started_at, ended_at, duration_seconds) \
                 VALUES ((SELECT category_id FROM category WHERE category_name = 'Break'), 1, \
                         '2026-09-29T09:00:00', '2026-09-29T09:01:30', 90);",
            )
            .expect("fixture");
        pending(connection, BATCH).expect("pending").remove(0)
    }

    #[test]
    fn an_entry_becomes_an_event_named_after_it_and_is_read_back() {
        let connection = seeded();
        let entry = entry(&connection);
        assert_eq!(entry.category_name, "Break");
        assert_eq!(entry.end_epoch - entry.start_epoch, 90);
        let expected = expected(&entry);
        assert_eq!(expected.event_id, format!("facet{}", entry.time_entry_id));
        let body = body(&expected).expect("body");
        assert!(body.contains(r#""start":{"dateTime":"2026-09-29T09:00:00Z"}"#), "{body}");
        assert!(body.contains(r#""end":{"dateTime":"2026-09-29T09:01:30Z"}"#), "{body}");

        let calendar = FakeCalendar(Mutex::new(Vec::new()));
        assert_eq!(deliver(&calendar, "token", "cal", &entry), Delivery::Delivered { already: false });
        assert_eq!(deliver(&calendar, "token", "cal", &entry), Delivery::Delivered { already: true });
        assert!(mark_synced(&connection, entry.time_entry_id).expect("tick"));
        assert!(pending(&connection, BATCH).expect("pending").is_empty());
    }

    #[test]
    fn a_time_with_an_offset_reads_as_the_same_moment() {
        let event = event(
            r#"{"id":"facet1","start":{"dateTime":"2026-09-29T19:00:00+10:00"},"end":{"dateTime":"2026-09-29T09:01:30Z"}}"#,
        )
        .expect("event");
        assert_eq!(event.start_epoch, Some(1790672400));
        assert_eq!(event.end_epoch, Some(1790672400 + 90));
        assert_eq!(event.summary, None);
        let expected = Expected {
            event_id: "facet1".into(),
            summary: "Break".into(),
            description: String::new(),
            start_epoch: 0,
            end_epoch: 0,
        };
        assert_eq!(mismatch(&event, &expected), Some(Mismatch::Summary));
    }
}
