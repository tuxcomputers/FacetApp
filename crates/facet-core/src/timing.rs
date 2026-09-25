//! Timing by hand: what is being timed, how long it has run today, and starting, pausing and resuming.
//!
//! Everything is read from the database when asked. Running means an unpaused open segment on the current
//! app face; there is no other flag. Times are whole unix seconds.

use rusqlite::{Connection, params};

use crate::category::{self, Category};
use crate::debug_log::{Record, Tag, plain};
use crate::{face, segment, setting, time_entry};

/// What the app's own clock is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingState {
    /// The current app face holds no category.
    Idle,
    /// A category is being timed now.
    Running,
    /// A category is picked and its clock is stopped.
    Paused,
}

/// The timing picture at one moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    /// The category on the current app face. `None` when it holds nothing.
    pub category: Option<Category>,
    pub timing_state: TimingState,
    /// The category's total for the current day, including the open segment. 0 with no category.
    pub seconds: i64,
    /// Whether `seconds` is going up: an unpaused open segment on a face holding the category.
    pub is_counting: bool,
    /// Whether the category's daily limit is spent. Always `false` for a category with no limit.
    pub is_limit_reached: bool,
}

/// What a click on a category row does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    StartTiming,
    /// A cube is on record and the app is not timing by hand, so nothing happens.
    WaitingForTheDevice,
}

/// Whether the app is its own clock: no cube paired, or this launch has given up on finding it.
pub fn is_manual_mode(is_cube_paired: bool, has_given_up_on_cube: bool) -> bool {
    !is_cube_paired || has_given_up_on_cube
}

/// What a category click does in the given mode.
pub fn click(is_manual_mode: bool) -> Click {
    if is_manual_mode { Click::StartTiming } else { Click::WaitingForTheDevice }
}

/// Whether `total_seconds` has used up a limit of `daily_limit_minutes`. A limit of 0 is never reached.
pub fn is_limit_reached(total_seconds: i64, daily_limit_minutes: i64) -> bool {
    daily_limit_minutes > 0 && total_seconds >= daily_limit_minutes * 60
}

/// Whether a spent limit refuses this press. A spent limit refuses resuming and never pausing.
pub fn is_resume_refused(is_limit_reached: bool, is_resuming: bool) -> bool {
    is_limit_reached && is_resuming
}

/// Whether the play/pause control does anything: never when idle, and not a resume once the limit is spent.
pub fn is_clickable(timing_state: TimingState, is_limit_reached: bool) -> bool {
    timing_state != TimingState::Idle
        && !is_resume_refused(is_limit_reached, timing_state == TimingState::Paused)
}

/// The start of the day `now` falls in: the most recent local `daily_reset_time` at or before `now`.
pub fn window_start(connection: &Connection, now: i64) -> Result<i64, rusqlite::Error> {
    let (hour, minute) = setting::daily_reset_time(connection)?;
    connection.query_row(
        "WITH reset AS (SELECT CAST(strftime('%s', date(?1, 'unixepoch', 'localtime') \
                                     || printf(' %02d:%02d:00', ?2, ?3), 'utc') AS INTEGER) AS today, \
                               CAST(strftime('%s', date(?1, 'unixepoch', 'localtime', '-1 day') \
                                     || printf(' %02d:%02d:00', ?2, ?3), 'utc') AS INTEGER) AS yesterday) \
         SELECT CASE WHEN ?1 >= today THEN today ELSE yesterday END FROM reset",
        params![now, hour, minute],
        |row| row.get(0),
    )
}

/// `category_id`'s total for the day `now` falls in: its time entries in the window, plus the open segment
/// when that segment is unpaused and its face holds the category.
pub fn day_seconds(connection: &Connection, category_id: i64, now: i64) -> Result<i64, rusqlite::Error> {
    let start = window_start(connection, now)?;
    let mut total = time_entry::seconds_in_window(connection, category_id, start, now)?;
    if let Some(open) = counting_segment(connection, category_id)? {
        total += (now - open.start_epoch.max(start)).max(0);
    }
    Ok(total)
}

/// Whether `category_id`'s total is going up now.
pub fn is_counting(connection: &Connection, category_id: i64) -> Result<bool, rusqlite::Error> {
    Ok(counting_segment(connection, category_id)?.is_some())
}

fn counting_segment(
    connection: &Connection,
    category_id: i64,
) -> Result<Option<segment::Segment>, rusqlite::Error> {
    let Some(open) = segment::open_segment(connection)?.filter(|open| !open.is_paused) else {
        return Ok(None);
    };
    let on_face = face::category_id(connection, open.face)?.unwrap_or(0);
    Ok((on_face == category_id).then_some(open))
}

/// The timing picture at `now`.
pub fn read(connection: &Connection, now: i64) -> Result<Reading, rusqlite::Error> {
    let current = segment::current_app_face(connection)?;
    let category = match face::category_id(connection, current)? {
        Some(id) => category::by_id(connection, id)?,
        None => None,
    };
    let Some(category) = category else {
        return Ok(Reading {
            category: None,
            timing_state: TimingState::Idle,
            seconds: 0,
            is_counting: false,
            is_limit_reached: false,
        });
    };
    let is_running =
        segment::open_segment(connection)?.is_some_and(|open| open.face == current && !open.is_paused);
    let seconds = day_seconds(connection, category.id, now)?;
    Ok(Reading {
        timing_state: if is_running { TimingState::Running } else { TimingState::Paused },
        seconds,
        is_counting: is_counting(connection, category.id)?,
        is_limit_reached: is_limit_reached(seconds, category.daily_limit_minutes),
        category: Some(category),
    })
}

/// What [`start_timing`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartOutcome {
    /// The category is on `face` and a segment is open on it.
    Started { face: i64 },
    /// That category was already running; nothing was written.
    AlreadyTiming,
    /// The next app face refused the category. The previous segment was closed and no new one opened.
    FaceRefused { face: i64 },
}

/// Starts timing `category_id` at `now`: closes the open segment, puts the category on the next app face in
/// the rotation, and opens a segment there. Both halves use `now`.
///
/// Does nothing when that category is already running.
pub fn start_timing(
    connection: &Connection,
    category_id: i64,
    now: i64,
    log: &impl Record,
) -> Result<StartOutcome, rusqlite::Error> {
    let before = read(connection, now)?;
    let name = before.category.as_ref().map(|c| plain(&c.name));
    if before.timing_state == TimingState::Running
        && before.category.as_ref().is_some_and(|c| c.id == category_id)
    {
        log.record(Tag::Timing, || {
            format!("Timing: already timing {}, so the click changes nothing", name.unwrap_or_default())
        });
        return Ok(StartOutcome::AlreadyTiming);
    }
    let next = face::next_app_face(segment::latest_face_in(connection, &face::APP_FACES)?);
    segment::close_open_segment(connection, now, log)?;
    if !face::assign(connection, category_id, next)? {
        log.record_failure(Tag::Timing, || format!("Timing: face {next} refused category_id {category_id}"));
        return Ok(StartOutcome::FaceRefused { face: next });
    }
    segment::start_segment(connection, next, now, log)?;
    log.record(Tag::Timing, || format!("Timing: started category_id {category_id} on face {next}"));
    Ok(StartOutcome::Started { face: next })
}

/// Pauses the clock if it is running, or resumes it if it is paused, at `now`, and returns the reading
/// afterwards.
///
/// `None` when nothing was done: idle, or a resume refused because the daily limit is spent. A resume opens
/// a segment on the same app face without writing the face.
pub fn toggle_pause(
    connection: &Connection,
    now: i64,
    log: &impl Record,
) -> Result<Option<Reading>, rusqlite::Error> {
    let before = read(connection, now)?;
    if !is_clickable(before.timing_state, before.is_limit_reached) {
        let name = before.category.as_ref().map_or_else(|| "nothing".to_string(), |c| plain(&c.name));
        log.record(Tag::Limit, || format!("Resume refused, {name} is idle or has spent its daily limit"));
        return Ok(None);
    }
    if before.timing_state == TimingState::Running {
        segment::close_open_segment(connection, now, log)?;
    } else {
        segment::start_segment(connection, segment::current_app_face(connection)?, now, log)?;
    }
    let after = read(connection, now)?;
    log.record(Tag::Timing, || {
        format!(
            "Timing: {} {}, {}s today",
            if after.timing_state == TimingState::Running { "running" } else { "stopped" },
            after.category.as_ref().map_or_else(|| "nothing".to_string(), |c| plain(&c.name)),
            after.seconds
        )
    });
    Ok(Some(after))
}

/// Stops the clock when the category being timed has spent its daily limit. Returns whether it stopped.
pub fn enforce_daily_limit(
    connection: &Connection,
    now: i64,
    log: &impl Record,
) -> Result<bool, rusqlite::Error> {
    let reading = read(connection, now)?;
    if reading.timing_state != TimingState::Running || !reading.is_limit_reached {
        return Ok(false);
    }
    segment::close_open_segment(connection, now, log)?;
    log.record(Tag::Limit, || {
        format!(
            "Daily limit reached, {} paused at {}s",
            reading.category.as_ref().map_or_else(|| "nothing".to_string(), |c| plain(&c.name)),
            reading.seconds
        )
    });
    Ok(true)
}

/// Seconds as `H:MM:SS`, or `H:MM` without seconds. Negative input reads as zero.
pub fn format_duration(seconds: i64, shows_seconds: bool) -> String {
    let seconds = seconds.max(0);
    let (hours, minutes, rest) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if shows_seconds { format!("{hours}:{minutes:02}:{rest:02}") } else { format!("{hours}:{minutes:02}") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::seeded;

    const NO_LOG: Option<crate::debug_log::DebugLog> = None;
    /// Midday local time is well clear of the 03:00 reset, so the window does not move under the tests.
    fn noon(connection: &Connection) -> i64 {
        connection
            .query_row(
                "SELECT CAST(strftime('%s', date('now', 'localtime') || ' 12:00:00', 'utc') AS INTEGER)",
                [],
                |r| r.get(0),
            )
            .expect("should compute")
    }

    fn category(connection: &Connection, name: &str) -> i64 {
        category::insert(connection, name).expect("should insert")
    }

    #[test]
    fn a_fresh_database_is_idle() {
        let connection = seeded();
        let reading = read(&connection, noon(&connection)).expect("should read");
        assert_eq!(reading.timing_state, TimingState::Idle);
        assert_eq!(reading.category, None);
        assert!(!is_clickable(reading.timing_state, false));
    }

    #[test]
    fn starting_a_category_runs_it_on_the_first_app_face_and_counts() {
        let connection = seeded();
        let now = noon(&connection);
        let code = category(&connection, "Code");
        assert_eq!(
            start_timing(&connection, code, now, &NO_LOG).expect("should start"),
            StartOutcome::Started { face: 13 }
        );
        let reading = read(&connection, now + 90).expect("should read");
        assert_eq!(reading.category.map(|c| c.name), Some("Code".to_string()));
        assert_eq!(reading.timing_state, TimingState::Running);
        assert_eq!(reading.seconds, 90);
        assert!(reading.is_counting);
    }

    #[test]
    fn clicking_the_running_category_changes_nothing() {
        let connection = seeded();
        let now = noon(&connection);
        let code = category(&connection, "Code");
        start_timing(&connection, code, now, &NO_LOG).expect("should start");
        assert_eq!(
            start_timing(&connection, code, now + 10, &NO_LOG).expect("should run"),
            StartOutcome::AlreadyTiming
        );
        assert_eq!(segment::open_segment(&connection).expect("should read").map(|s| s.face), Some(13));
    }

    #[test]
    fn switching_category_rotates_the_face_and_files_the_first_stretch_under_the_first_category() {
        let connection = seeded();
        let now = noon(&connection);
        let (code, email) = (category(&connection, "Code"), category(&connection, "Email"));
        start_timing(&connection, code, now, &NO_LOG).expect("should start");
        assert_eq!(
            start_timing(&connection, email, now + 60, &NO_LOG).expect("should start"),
            StartOutcome::Started { face: 14 }
        );
        assert_eq!(day_seconds(&connection, code, now + 120).expect("should sum"), 60);
        assert_eq!(day_seconds(&connection, email, now + 120).expect("should sum"), 60);
        assert_eq!(face::category_id(&connection, 13).expect("should read"), Some(code));
    }

    #[test]
    fn pausing_stops_the_figure_and_resuming_continues_on_the_same_face() {
        let connection = seeded();
        let now = noon(&connection);
        let code = category(&connection, "Code");
        start_timing(&connection, code, now, &NO_LOG).expect("should start");
        let paused = toggle_pause(&connection, now + 30, &NO_LOG).expect("should run").expect("should pause");
        assert_eq!(
            (paused.timing_state, paused.seconds, paused.is_counting),
            (TimingState::Paused, 30, false)
        );
        assert_eq!(read(&connection, now + 500).expect("should read").seconds, 30);

        let resumed =
            toggle_pause(&connection, now + 500, &NO_LOG).expect("should run").expect("should resume");
        assert_eq!(resumed.timing_state, TimingState::Running);
        assert_eq!(segment::open_segment(&connection).expect("should read").map(|s| s.face), Some(13));
        assert_eq!(read(&connection, now + 510).expect("should read").seconds, 40);
    }

    #[test]
    fn toggling_while_idle_does_nothing() {
        let connection = seeded();
        assert_eq!(toggle_pause(&connection, noon(&connection), &NO_LOG).expect("should run"), None);
    }

    #[test]
    fn a_spent_limit_stops_the_clock_and_refuses_a_resume_but_not_a_pause() {
        let connection = seeded();
        let now = noon(&connection);
        let code = category(&connection, "Code");
        connection
            .execute("UPDATE category SET daily_limit = 1 WHERE category_id = ?1", params![code])
            .expect("should write");
        start_timing(&connection, code, now, &NO_LOG).expect("should start");
        assert!(!enforce_daily_limit(&connection, now + 59, &NO_LOG).expect("should run"));
        assert!(enforce_daily_limit(&connection, now + 61, &NO_LOG).expect("should run"));
        let reading = read(&connection, now + 70).expect("should read");
        assert_eq!(reading.timing_state, TimingState::Paused);
        assert!(reading.is_limit_reached);
        assert!(!is_clickable(reading.timing_state, reading.is_limit_reached));
        assert_eq!(toggle_pause(&connection, now + 70, &NO_LOG).expect("should run"), None);
        assert!(is_clickable(TimingState::Running, true));
    }

    #[test]
    fn a_limit_of_zero_is_never_reached() {
        assert!(!is_limit_reached(1_000_000, 0));
        assert!(is_limit_reached(120, 2));
        assert!(!is_limit_reached(119, 2));
    }

    #[test]
    fn the_window_starts_at_the_most_recent_reset() {
        let connection = seeded();
        let now = noon(&connection);
        let start = window_start(&connection, now).expect("should compute");
        assert_eq!(now - start, 9 * 3600);
        let before_reset = now - 10 * 3600;
        let earlier = window_start(&connection, before_reset).expect("should compute");
        let (day_before, local): (String, String) = connection
            .query_row(
                "SELECT date(?1, 'unixepoch', 'localtime', '-1 day'), \
                        strftime('%Y-%m-%d %H:%M', ?2, 'unixepoch', 'localtime')",
                params![now, earlier],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("should compute");
        assert_eq!(local, format!("{day_before} 03:00"));
    }

    #[test]
    fn manual_mode_is_unpaired_or_given_up() {
        assert!(is_manual_mode(false, false));
        assert!(is_manual_mode(true, true));
        assert!(!is_manual_mode(true, false));
        assert_eq!(click(false), Click::WaitingForTheDevice);
        assert_eq!(click(true), Click::StartTiming);
    }

    #[test]
    fn durations_format_with_and_without_seconds() {
        assert_eq!(format_duration(0, true), "0:00:00");
        assert_eq!(format_duration(3_725, true), "1:02:05");
        assert_eq!(format_duration(3_725, false), "1:02");
        assert_eq!(format_duration(-5, true), "0:00:00");
    }
}
