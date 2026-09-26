//! The Report tab's arithmetic and reads: calendar days, the window a range of days covers, what each
//! category recorded in it, the entries behind a total, and the order totals are listed in.
//!
//! A day here is an app day: it runs from `daily_reset_time` on that date to `daily_reset_time` on the next,
//! local time, so across a daylight-saving change it is 23 or 25 hours long. Only finished time entries count;
//! a segment still running has no entry yet. Every read goes to the database when called.

use std::cmp::Ordering;

use rusqlite::{Connection, params};

use crate::category::{self, Category};
use crate::{setting, timing};

/// A calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day {
    pub year: i64,
    /// 1 to 12.
    pub month: i64,
    /// 1 to the month's length.
    pub day: i64,
}

impl Day {
    /// Days since 1970-01-01, which is day 0.
    pub fn ordinal(self) -> i64 {
        let year = if self.month <= 2 { self.year - 1 } else { self.year };
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let month_index = (self.month + 9) % 12;
        let day_of_year = (153 * month_index + 2) / 5 + self.day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    /// The date `ordinal` days after 1970-01-01.
    pub fn from_ordinal(ordinal: i64) -> Day {
        let shifted = ordinal + 719_468;
        let era = shifted.div_euclid(146_097);
        let day_of_era = shifted - era * 146_097;
        let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_index = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month_index + 2) / 5 + 1;
        let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        Day { year, month, day }
    }

    pub fn plus_days(self, days: i64) -> Day {
        Day::from_ordinal(self.ordinal() + days)
    }

    /// 0 for Monday through 6 for Sunday.
    pub fn weekday(self) -> i64 {
        (self.ordinal() + 3).rem_euclid(7)
    }

    /// The first of this day's month.
    pub fn first_of_month(self) -> Day {
        Day { day: 1, ..self }
    }

    /// The first of the month `months` away from this day's month.
    pub fn plus_months(self, months: i64) -> Day {
        let index = self.year * 12 + (self.month - 1) + months;
        Day { year: index.div_euclid(12), month: index.rem_euclid(12) + 1, day: 1 }
    }

    /// How many days this day's month has.
    pub fn days_in_month(self) -> i64 {
        self.plus_months(1).ordinal() - self.first_of_month().ordinal()
    }

    /// `yyyy-mm-dd`.
    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// Parses `yyyy-mm-dd`. `None` for anything else, including a date that does not exist.
    pub fn parse(text: &str) -> Option<Day> {
        let mut parts = text.split('-');
        let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() {
            return None;
        }
        let parsed = Day { year: year.parse().ok()?, month: month.parse().ok()?, day: day.parse().ok()? };
        let valid =
            (1..=12).contains(&parsed.month) && parsed.day >= 1 && parsed.day <= parsed.days_in_month();
        valid.then_some(parsed)
    }

    /// The month and year, such as `September 2026`.
    pub fn month_title(self) -> String {
        format!("{} {}", MONTHS[(self.month - 1) as usize], self.year)
    }

    /// The day in words, such as `Friday 25 September 2026`.
    pub fn full_label(self) -> String {
        format!(
            "{} {} {} {}",
            WEEKDAYS[self.weekday() as usize],
            self.day,
            MONTHS[(self.month - 1) as usize],
            self.year
        )
    }
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const WEEKDAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// The 42 days a month is drawn over: six weeks from the Monday on or before its first day, so the days of
/// the months either side fill the first and last rows.
pub fn month_grid(month: Day) -> Vec<Day> {
    let first = month.first_of_month();
    let start = first.plus_days(-first.weekday());
    (0..42).map(|offset| start.plus_days(offset)).collect()
}

/// The app day `now` falls in: the date the current `daily_reset_time` window started on.
pub fn today(connection: &Connection, now: i64) -> Result<Day, rusqlite::Error> {
    let start = timing::window_start(connection, now)?;
    let text: String =
        connection
            .query_row("SELECT date(?1, 'unixepoch', 'localtime')", params![start], |row| row.get(0))?;
    Day::parse(&text).ok_or(rusqlite::Error::InvalidQuery)
}

/// The half-open window, in unix seconds, that the days `first` through `last` cover: `daily_reset_time` on
/// `first` up to `daily_reset_time` on the day after `last`, local time. A `last` before `first` is taken as
/// `first`.
pub fn bounds(connection: &Connection, first: Day, last: Day) -> Result<(i64, i64), rusqlite::Error> {
    let last = last.max(first);
    let (hour, minute) = setting::daily_reset_time(connection)?;
    let at_reset = |day: Day| -> Result<i64, rusqlite::Error> {
        connection.query_row(
            "SELECT CAST(strftime('%s', ?1 || printf(' %02d:%02d:00', ?2, ?3), 'utc') AS INTEGER)",
            params![day.iso(), hour, minute],
            |row| row.get(0),
        )
    };
    Ok((at_reset(first)?, at_reset(last.plus_days(1))?))
}

/// What one category recorded in a window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Total {
    pub category: Category,
    pub seconds: i64,
}

/// Every category with recorded time in `[start, end)`, each entry clipped to the window, in display order.
/// Categories with nothing in the window are left out. Retired categories and Unassigned are included.
pub fn totals(connection: &Connection, start: i64, end: i64) -> Result<Vec<Total>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT te.category_id, \
                SUM(MIN(de.start_epoch + te.duration_seconds, ?2) - MAX(de.start_epoch, ?1)) AS seconds \
         FROM time_entry te JOIN device_event de ON de.device_event_id = te.device_event_id \
         WHERE de.start_epoch < ?2 AND de.start_epoch + te.duration_seconds > ?1 \
         GROUP BY te.category_id HAVING seconds > 0",
    )?;
    let sums: Vec<(i64, f64)> = statement
        .query_map(params![start, end], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut totals = Vec::with_capacity(sums.len());
    for (id, seconds) in sums {
        if let Some(category) = category::by_id(connection, id)? {
            totals.push(Total { category, seconds: seconds.round() as i64 });
        }
    }
    totals.sort_by(|a, b| category::display_order(&a.category, &b.category));
    Ok(totals)
}

/// One time entry as the Report shows it, clipped to the window it was read for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub time_entry_id: i64,
    /// Unix seconds.
    pub began: i64,
    /// Unix seconds.
    pub ended: i64,
}

/// `category_id`'s entries overlapping `[start, end)`, clipped to it, oldest first.
pub fn entries(
    connection: &Connection,
    category_id: i64,
    start: i64,
    end: i64,
) -> Result<Vec<Entry>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT te.time_entry_id, MAX(de.start_epoch, ?2), MIN(de.start_epoch + te.duration_seconds, ?3) \
         FROM time_entry te JOIN device_event de ON de.device_event_id = te.device_event_id \
         WHERE te.category_id = ?1 AND de.start_epoch < ?3 AND de.start_epoch + te.duration_seconds > ?2 \
         ORDER BY 2, 1",
    )?;
    statement
        .query_map(params![category_id, start, end], |row| {
            Ok(Entry {
                time_entry_id: row.get(0)?,
                began: row.get::<_, f64>(1)? as i64,
                ended: row.get::<_, f64>(2)? as i64,
            })
        })?
        .collect()
}

/// Seconds as `H:MM:SS`, or `H:MM` rounded to the nearest minute. Hours are never dropped.
pub fn format_seconds(seconds: i64, shows_seconds: bool) -> String {
    let seconds = seconds.max(0);
    if shows_seconds {
        timing::format_duration(seconds, true)
    } else {
        timing::format_duration((seconds + 30) / 60 * 60, false)
    }
}

/// How an entry reads: `date` (`dd/mm`), the two clock times (`hh:mm` or `hh:mm:ss`), the duration, and a
/// spoken form such as `25 September, 09:00:00 to 10:30:00, 1:30:00`. Local time.
pub struct EntryText {
    pub date: String,
    pub began: String,
    pub ended: String,
    pub duration: String,
    pub label: String,
}

/// The text of `entry`.
pub fn entry_text(
    connection: &Connection,
    entry: &Entry,
    shows_seconds: bool,
) -> Result<EntryText, rusqlite::Error> {
    let clock = if shows_seconds { "%H:%M:%S" } else { "%H:%M" };
    let (day, month, date, began, ended): (i64, i64, String, String, String) = connection.query_row(
        "SELECT CAST(strftime('%d', ?1, 'unixepoch', 'localtime') AS INTEGER), \
                CAST(strftime('%m', ?1, 'unixepoch', 'localtime') AS INTEGER), \
                strftime('%d/%m', ?1, 'unixepoch', 'localtime'), \
                strftime(?3, ?1, 'unixepoch', 'localtime'), \
                strftime(?3, ?2, 'unixepoch', 'localtime')",
        params![entry.began, entry.ended, clock],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    )?;
    let duration = format_seconds(entry.ended - entry.began, shows_seconds);
    let month_name = MONTHS.get((month - 1) as usize).copied().unwrap_or("?");
    let label = format!("{day} {month_name}, {began} to {ended}, {duration}");
    Ok(EntryText { date, began, ended, duration, label })
}

/// Which column the totals are sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumnState {
    Category,
    Time,
}

/// The order the totals are listed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortOrder {
    pub sort_column_state: SortColumnState,
    pub is_sort_ascending: bool,
}

impl Default for SortOrder {
    /// Time, biggest first.
    fn default() -> Self {
        SortOrder { sort_column_state: SortColumnState::Time, is_sort_ascending: false }
    }
}

impl SortOrder {
    /// The order after a click on `column`'s heading: the column in force reverses; the other column takes
    /// its own starting direction, ascending for Category and descending for Time.
    pub fn pressed(self, column: SortColumnState) -> SortOrder {
        if column == self.sort_column_state {
            SortOrder { is_sort_ascending: !self.is_sort_ascending, ..self }
        } else {
            SortOrder { sort_column_state: column, is_sort_ascending: column == SortColumnState::Category }
        }
    }

    /// `column`'s heading: its name, with ` ▲` or ` ▼` when it is the column in force.
    pub fn heading(self, column: SortColumnState) -> String {
        let name = match column {
            SortColumnState::Category => "Category",
            SortColumnState::Time => "Time",
        };
        if column != self.sort_column_state {
            return name.to_string();
        }
        format!("{name} {}", if self.is_sort_ascending { "\u{25b2}" } else { "\u{25bc}" })
    }

    /// Sorts `totals` into this order. A tie on Time is broken in ascending display order whichever way Time
    /// runs.
    pub fn sort(self, totals: &mut [Total]) {
        totals.sort_by(|a, b| {
            let by_category = category::display_order(&a.category, &b.category);
            match self.sort_column_state {
                SortColumnState::Category => {
                    if self.is_sort_ascending {
                        by_category
                    } else {
                        by_category.reverse()
                    }
                }
                SortColumnState::Time => {
                    let by_time = a.seconds.cmp(&b.seconds);
                    let by_time = if self.is_sort_ascending { by_time } else { by_time.reverse() };
                    if by_time == Ordering::Equal { by_category } else { by_time }
                }
            }
        });
    }

    /// `time` or `category`, and `ascending` or `descending`, for the debug log.
    pub fn describe(self) -> String {
        format!(
            "{}, {}",
            match self.sort_column_state {
                SortColumnState::Category => "category",
                SortColumnState::Time => "time",
            },
            if self.is_sort_ascending { "ascending" } else { "descending" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::seeded;
    use crate::{face, segment};

    const NO_LOG: Option<crate::debug_log::DebugLog> = None;

    fn day(year: i64, month: i64, day: i64) -> Day {
        Day { year, month, day }
    }

    fn epoch(connection: &Connection, local: &str) -> i64 {
        connection
            .query_row("SELECT CAST(strftime('%s', ?1, 'utc') AS INTEGER)", params![local], |r| r.get(0))
            .expect("should compute")
    }

    fn record(connection: &Connection, category: i64, from: i64, to: i64) {
        face::assign(connection, category, 13).expect("should assign");
        segment::start_segment(connection, 13, from, &NO_LOG).expect("should start");
        segment::close_open_segment(connection, to, &NO_LOG).expect("should close");
    }

    #[test]
    fn dates_round_trip_through_their_ordinal() {
        assert_eq!(day(1970, 1, 1).ordinal(), 0);
        assert_eq!(Day::from_ordinal(0), day(1970, 1, 1));
        for ordinal in [-1000, 0, 19_000, 20_725, 100_000] {
            assert_eq!(Day::from_ordinal(ordinal).ordinal(), ordinal);
        }
        assert_eq!(day(2026, 9, 25).weekday(), 4);
        assert_eq!(day(2024, 2, 1).days_in_month(), 29);
        assert_eq!(day(2026, 12, 5).plus_months(1), day(2027, 1, 1));
        assert_eq!(day(2026, 1, 5).plus_months(-1), day(2025, 12, 1));
        assert_eq!(Day::parse("2026-09-25"), Some(day(2026, 9, 25)));
        assert_eq!(Day::parse("2026-02-30"), None);
        assert_eq!(day(2026, 9, 25).month_title(), "September 2026");
        assert_eq!(day(2026, 9, 25).full_label(), "Friday 25 September 2026");
    }

    #[test]
    fn a_month_is_drawn_over_six_weeks_from_a_monday() {
        let grid = month_grid(day(2026, 9, 15));
        assert_eq!(grid.len(), 42);
        assert_eq!(grid[0], day(2026, 8, 31));
        assert_eq!(grid[1], day(2026, 9, 1));
        assert!(grid.iter().all(|d| d.weekday() == (d.ordinal() - grid[0].ordinal()) % 7));
    }

    #[test]
    fn a_day_runs_from_reset_to_reset() {
        let connection = seeded();
        let (start, end) = bounds(&connection, day(2026, 9, 25), day(2026, 9, 25)).expect("should compute");
        assert_eq!(start, epoch(&connection, "2026-09-25 03:00:00"));
        assert_eq!(end, epoch(&connection, "2026-09-26 03:00:00"));
        let (_, wider) = bounds(&connection, day(2026, 9, 25), day(2026, 9, 27)).expect("should compute");
        assert_eq!(wider, epoch(&connection, "2026-09-28 03:00:00"));
        assert_eq!(
            bounds(&connection, day(2026, 9, 25), day(2026, 9, 20)).expect("should compute"),
            (start, end)
        );
    }

    #[test]
    fn today_is_the_day_the_reset_window_started_on() {
        let connection = seeded();
        let before_reset = epoch(&connection, "2026-09-25 02:00:00");
        let after_reset = epoch(&connection, "2026-09-25 04:00:00");
        assert_eq!(today(&connection, before_reset).expect("should compute"), day(2026, 9, 24));
        assert_eq!(today(&connection, after_reset).expect("should compute"), day(2026, 9, 25));
    }

    #[test]
    fn totals_clip_each_entry_to_the_window_and_leave_out_empty_categories() {
        let connection = seeded();
        let code = category::insert(&connection, "Code").expect("should insert");
        let mail = category::insert(&connection, "Mail").expect("should insert");
        record(
            &connection,
            code,
            epoch(&connection, "2026-09-25 02:30:00"),
            epoch(&connection, "2026-09-25 03:30:00"),
        );
        record(
            &connection,
            mail,
            epoch(&connection, "2026-09-25 10:00:00"),
            epoch(&connection, "2026-09-25 10:10:00"),
        );
        let (start, end) = bounds(&connection, day(2026, 9, 25), day(2026, 9, 25)).expect("should compute");
        let totals = totals(&connection, start, end).expect("should read");
        let summary: Vec<(&str, i64)> =
            totals.iter().map(|t| (t.category.name.as_str(), t.seconds)).collect();
        assert_eq!(summary, [("Code", 1800), ("Mail", 600)]);

        let (start, end) = bounds(&connection, day(2026, 9, 24), day(2026, 9, 24)).expect("should compute");
        let yesterday = totals_for(&connection, start, end);
        assert_eq!(yesterday, [("Code".to_string(), 1800)]);

        let entries = entries(&connection, code, start, end).expect("should read");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].ended, epoch(&connection, "2026-09-25 03:00:00"));
        let text = entry_text(&connection, &entries[0], true).expect("should format");
        assert_eq!(
            (text.date.as_str(), text.began.as_str(), text.ended.as_str()),
            ("25/09", "02:30:00", "03:00:00")
        );
        assert_eq!(text.label, "25 September, 02:30:00 to 03:00:00, 0:30:00");
    }

    fn totals_for(connection: &Connection, start: i64, end: i64) -> Vec<(String, i64)> {
        totals(connection, start, end)
            .expect("should read")
            .into_iter()
            .map(|t| (t.category.name, t.seconds))
            .collect()
    }

    #[test]
    fn minutes_round_to_the_nearest() {
        assert_eq!(format_seconds(3725, true), "1:02:05");
        assert_eq!(format_seconds(89, false), "0:01");
        assert_eq!(format_seconds(90, false), "0:02");
    }

    #[test]
    fn sorting_starts_on_time_descending_and_reverses_the_column_in_force() {
        let order = SortOrder::default();
        assert_eq!(order.heading(SortColumnState::Time), "Time \u{25bc}");
        assert_eq!(order.heading(SortColumnState::Category), "Category");
        let reversed = order.pressed(SortColumnState::Time);
        assert!(reversed.is_sort_ascending);
        let by_category = reversed.pressed(SortColumnState::Category);
        assert_eq!(
            by_category,
            SortOrder { sort_column_state: SortColumnState::Category, is_sort_ascending: true }
        );
        assert_eq!(by_category.pressed(SortColumnState::Time), SortOrder::default());
        assert_eq!(by_category.describe(), "category, ascending");
    }

    #[test]
    fn a_tie_on_time_breaks_in_ascending_category_order_either_way() {
        let connection = seeded();
        let make = |name: &str, seconds: i64| Total {
            category: category::by_id(
                &connection,
                category::insert(&connection, name).expect("should insert"),
            )
            .expect("should read")
            .expect("should exist"),
            seconds,
        };
        let mut totals = vec![make("B", 10), make("A", 10), make("C", 99)];
        SortOrder::default().sort(&mut totals);
        assert_eq!(totals.iter().map(|t| t.category.name.as_str()).collect::<Vec<_>>(), ["C", "A", "B"]);
        SortOrder::default().pressed(SortColumnState::Time).sort(&mut totals);
        assert_eq!(totals.iter().map(|t| t.category.name.as_str()).collect::<Vec<_>>(), ["A", "B", "C"]);
    }
}
