//! The Report tab's behaviour: draws the two calendars, reads the totals for the range picked, and folds
//! each total open onto its entries.
//!
//! Held here while the window is open, and reset by [`Report::open`]: the range (a start day and an optional
//! end), the month each calendar has been paged to, the sort order and which totals are open. Those are what
//! the person has picked, not facts from the database. Every total, entry and "today" is read from the
//! database at each redraw.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

use facet_core::database;
use facet_core::debug_log::{Record, Tag, Trace, plain};
use facet_core::report::{self, Day, SortColumnState, SortOrder, Total};
use facet_core::setting;
use rusqlite::{Connection, params};
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::{ReportData, ReportDay, ReportEntry, ReportTotal, SettingsWindow, icons};

/// The tab's index in the Settings window.
const REPORT_TAB: i32 = 2;

/// The Report tab, attached to one Settings window.
pub struct Report {
    ui: slint::Weak<SettingsWindow>,
    database: PathBuf,
    log: Rc<Trace>,
    /// The first day picked and the optional last. `None` until the tab is first drawn.
    range: Cell<Option<(Day, Option<Day>)>>,
    from_month: Cell<Option<Day>>,
    to_month: Cell<Option<Day>>,
    order: Cell<SortOrder>,
    expanded: RefCell<HashSet<i64>>,
    /// The count and newest id of `time_entry` when the totals were last drawn, so that
    /// [`Report::refresh_if_showing`] redraws only when an entry has been recorded since. Read back from the
    /// table every time it is asked; a stale value costs one redraw.
    drawn_entries: Cell<Option<(i64, i64)>>,
    /// Whole unix seconds now.
    clock: Box<dyn Fn() -> i64>,
    this: RefCell<Weak<Report>>,
}

impl Report {
    /// Wires the tab's callbacks on `ui` to `database`, telling the time by the system clock. Call once, at
    /// launch.
    pub fn attach(ui: &SettingsWindow, database: PathBuf, log: Rc<Trace>) -> Rc<Report> {
        Report::attach_with_clock(ui, database, log, now)
    }

    /// As [`Report::attach`], telling the time by `clock`, which answers whole unix seconds.
    pub fn attach_with_clock(
        ui: &SettingsWindow,
        database: PathBuf,
        log: Rc<Trace>,
        clock: impl Fn() -> i64 + 'static,
    ) -> Rc<Report> {
        let report = Rc::new(Report {
            ui: ui.as_weak(),
            database,
            log,
            range: Cell::new(None),
            from_month: Cell::new(None),
            to_month: Cell::new(None),
            order: Cell::new(SortOrder::default()),
            expanded: RefCell::new(HashSet::new()),
            drawn_entries: Cell::new(None),
            clock: Box::new(clock),
            this: RefCell::new(Weak::new()),
        });
        *report.this.borrow_mut() = Rc::downgrade(&report);

        let data = ui.global::<ReportData>();
        let weak = Rc::downgrade(&report);
        data.on_day_picked(move |calendar, iso| {
            if let Some(report) = weak.upgrade() {
                report.day_picked(&calendar, &iso);
            }
        });
        let weak = Rc::downgrade(&report);
        data.on_month_stepped(move |calendar, delta| {
            if let Some(report) = weak.upgrade() {
                report.month_stepped(&calendar, i64::from(delta));
            }
        });
        let weak = Rc::downgrade(&report);
        data.on_sort_pressed(move |column| {
            if let Some(report) = weak.upgrade() {
                report.sort_pressed(&column);
            }
        });
        let weak = Rc::downgrade(&report);
        data.on_total_toggled(move |id| {
            if let Some(report) = weak.upgrade() {
                report.total_toggled(i64::from(id));
            }
        });
        report
    }

    /// Starts afresh on today with no end, both calendars following the range and every total folded, then
    /// draws. Call when the Settings window opens.
    pub fn open(&self) {
        self.range.set(None);
        self.from_month.set(None);
        self.to_month.set(None);
        self.expanded.borrow_mut().clear();
        self.refresh();
    }

    /// Redraws the calendars and re-reads the totals for the range as it stands. Call when the Report tab is
    /// shown.
    pub fn refresh(&self) {
        let Some(connection) = self.connect() else { return };
        let Some(today) = self.report(report::today(&connection, (self.clock)())) else { return };
        let (start, end) = match self.range.get() {
            Some(range) => range,
            None => {
                self.range.set(Some((today, None)));
                (today, None)
            }
        };
        self.draw_calendars(today, start, end);
        self.draw_totals(&connection, start, end);
    }

    /// Redraws when the Report tab is on screen and a time entry has been recorded since the last draw.
    pub fn refresh_if_showing(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        if ui.get_active_tab() != REPORT_TAB || !ui.window().is_visible() {
            return;
        }
        let Some(connection) = self.connect() else { return };
        let signature = self.report(entry_signature(&connection));
        if signature.is_some() && signature != self.drawn_entries.get() {
            self.refresh();
        }
    }

    fn day_picked(&self, calendar: &str, iso: &str) {
        let Some(day) = Day::parse(iso) else {
            self.log.record_failure(Tag::Report, || format!("Report: {iso} is not a day"));
            return;
        };
        let (start, end) = self.range.get().unwrap_or((day, None));
        let (start, end) = match calendar {
            "from" => (day, end.map(|end| end.max(day))),
            _ => (start, Some(day.max(start))),
        };
        self.range.set(Some((start, end)));
        // Both calendars follow the range again once a day is picked.
        self.from_month.set(None);
        self.to_month.set(None);
        self.log.record(Tag::Report, || {
            format!(
                "Report range {} -> {}",
                start.iso(),
                end.map_or_else(|| "not set, reporting one day".to_string(), Day::iso)
            )
        });
        self.refresh();
    }

    fn month_stepped(&self, calendar: &str, delta: i64) {
        let Some(connection) = self.connect() else { return };
        let Some(today) = self.report(report::today(&connection, (self.clock)())) else { return };
        let Some((start, end)) = self.range.get() else { return };
        let is_from = calendar == "from";
        let shown = if is_from { self.shown_from(today, start) } else { self.shown_to(today, start, end) };
        let target = shown.plus_months(delta);
        let floor = if is_from { None } else { Some(start.first_of_month()) };
        if target > today.first_of_month() || floor.is_some_and(|floor| target < floor) {
            return;
        }
        if is_from {
            self.from_month.set(Some(target));
        } else {
            self.to_month.set(Some(target));
        }
        self.log.record(Tag::Report, || {
            format!(
                "{} calendar showing {:04}-{:02}",
                if is_from { "From" } else { "To" },
                target.year,
                target.month
            )
        });
        self.draw_calendars(today, start, end);
    }

    fn sort_pressed(&self, column: &str) {
        let column = if column == "category" { SortColumnState::Category } else { SortColumnState::Time };
        let order = self.order.get().pressed(column);
        self.order.set(order);
        self.log.record(Tag::Report, || format!("Report sorted by {}", order.describe()));
        self.refresh();
    }

    fn total_toggled(&self, id: i64) {
        let opened = {
            let mut expanded = self.expanded.borrow_mut();
            if expanded.remove(&id) {
                false
            } else {
                expanded.insert(id);
                true
            }
        };
        let name = self
            .connect()
            .and_then(|connection| self.report(facet_core::category::by_id(&connection, id)))
            .flatten()
            .map_or_else(|| format!("category_id {id}"), |category| plain(&category.name));
        self.log.record(Tag::Report, || {
            format!("Report category {name} {}", if opened { "opened" } else { "closed" })
        });
        self.refresh();
    }

    /// The month the From calendar shows: where it was paged to, else the start's month, never past today.
    fn shown_from(&self, today: Day, start: Day) -> Day {
        self.from_month.get().unwrap_or(start.first_of_month()).min(today.first_of_month())
    }

    /// The month the To calendar shows: where it was paged to, else the end's (or start's) month, never
    /// before the start's month or past today's.
    fn shown_to(&self, today: Day, start: Day, end: Option<Day>) -> Day {
        self.to_month
            .get()
            .unwrap_or(end.unwrap_or(start).first_of_month())
            .clamp(start.first_of_month(), today.first_of_month())
    }

    fn draw_calendars(&self, today: Day, start: Day, end: Option<Day>) {
        let Some(ui) = self.ui.upgrade() else { return };
        let data = ui.global::<ReportData>();
        let last = end.unwrap_or(start);
        let cells =
            |month: Day, allowed: &dyn Fn(Day) -> bool, selected: Option<Day>| -> ModelRc<ReportDay> {
                let days: Vec<ReportDay> = report::month_grid(month)
                    .into_iter()
                    .map(|day| ReportDay {
                        iso: day.iso().into(),
                        number: day.day as i32,
                        label: day.full_label().into(),
                        is_in_month: day.month == month.month && day.year == month.year,
                        is_allowed: allowed(day),
                        is_selected: selected == Some(day),
                        is_in_range: day >= start && day <= last,
                        is_range_start: day == start,
                        is_range_end: day == last,
                    })
                    .collect();
                ModelRc::new(VecModel::from(days))
            };

        let from = self.shown_from(today, start);
        data.set_from_month(from.month_title().into());
        data.set_from_days(cells(from, &|day| day <= today, Some(start)));
        data.set_from_previous_enabled(true);
        data.set_from_next_enabled(from.plus_months(1) <= today);

        let to = self.shown_to(today, start, end);
        data.set_to_month(to.month_title().into());
        data.set_to_days(cells(to, &|day| day >= start && day <= today, end));
        data.set_to_previous_enabled(to > start.first_of_month());
        data.set_to_next_enabled(to.plus_months(1) <= today);
        data.set_to_subtitle(if end.is_none() { "not set, reporting one day" } else { "" }.into());
    }

    fn draw_totals(&self, connection: &Connection, start: Day, end: Option<Day>) {
        let Some(ui) = self.ui.upgrade() else { return };
        let data = ui.global::<ReportData>();
        let Some((from, until)) = self.report(report::bounds(connection, start, end.unwrap_or(start))) else {
            return;
        };
        let Some(mut totals) = self.report(report::totals(connection, from, until)) else { return };
        let Some(shows_seconds) = self.report(setting::shows_seconds(connection)) else { return };
        let order = self.order.get();
        order.sort(&mut totals);

        let mut expanded = self.expanded.borrow_mut();
        expanded.retain(|id| totals.iter().any(|total| total.category.id == *id));
        let rows: Vec<ReportTotal> = totals
            .iter()
            .map(|total| {
                self.row(connection, total, expanded.contains(&total.category.id), from, until, shows_seconds)
            })
            .collect();
        drop(expanded);
        data.set_totals(ModelRc::new(VecModel::from(rows)));
        data.set_category_heading(order.heading(SortColumnState::Category).into());
        data.set_time_heading(order.heading(SortColumnState::Time).into());
        self.drawn_entries.set(self.report(entry_signature(connection)));

        let window = self.report(connection.query_row(
            "SELECT strftime('%Y-%m-%d %H:%M', ?1, 'unixepoch', 'localtime'), \
                    strftime('%Y-%m-%d %H:%M', ?2, 'unixepoch', 'localtime')",
            params![from, until],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        ));
        if let Some((from, until)) = window {
            self.log.record(Tag::Report, || {
                format!("Report totals {from} -> {until}: {} categories", totals.len())
            });
        }
    }

    fn row(
        &self,
        connection: &Connection,
        total: &Total,
        is_expanded: bool,
        from: i64,
        until: i64,
        shows_seconds: bool,
    ) -> ReportTotal {
        let category = &total.category;
        let colour = colour(category.colour_hex.as_deref());
        let icon = category.icon_name.as_deref().and_then(|name| match icons::image(name) {
            Ok(image) => image,
            Err(error) => {
                self.log.record_failure(Tag::Report, || {
                    format!("Report: icon {} would not decode: {error:?}", plain(name))
                });
                None
            }
        });
        let entries: Vec<ReportEntry> = if is_expanded {
            self.report(report::entries(connection, category.id, from, until))
                .unwrap_or_default()
                .iter()
                .filter_map(|entry| {
                    let text = self.report(report::entry_text(connection, entry, shows_seconds))?;
                    Some(ReportEntry {
                        id: i32::try_from(entry.time_entry_id).unwrap_or(i32::MAX),
                        date: text.date.into(),
                        began: text.began.into(),
                        ended: text.ended.into(),
                        duration: text.duration.into(),
                        label: text.label.into(),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        ReportTotal {
            id: i32::try_from(category.id).unwrap_or(i32::MAX),
            name: category.name.as_str().into(),
            colour: colour.unwrap_or_default(),
            has_colour: colour.is_some(),
            has_icon: icon.is_some(),
            icon: icon.unwrap_or_default(),
            white_lines: category.uses_white_lines,
            duration: report::format_seconds(total.seconds, shows_seconds).into(),
            is_expanded,
            entries: ModelRc::new(VecModel::from(entries)),
        }
    }

    fn connect(&self) -> Option<Connection> {
        match database::connect(&self.database) {
            Ok(connection) => Some(connection),
            Err(error) => {
                self.log.record_failure(Tag::Database, || {
                    format!("Report: the database would not open: {error}")
                });
                None
            }
        }
    }

    fn report<T>(&self, result: Result<T, rusqlite::Error>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.log.record_failure(Tag::Database, || format!("Report: a database call failed: {error}"));
                None
            }
        }
    }
}

/// How many time entries there are and the newest id.
fn entry_signature(connection: &Connection) -> Result<(i64, i64), rusqlite::Error> {
    connection.query_row("SELECT COUNT(*), IFNULL(MAX(time_entry_id), 0) FROM time_entry", [], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })
}

/// `#rrggbb` as a colour. `None` for no value or one that does not parse.
fn colour(hex: Option<&str>) -> Option<slint::Color> {
    let digits = hex?.strip_prefix('#')?;
    if digits.len() != 6 {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    Some(slint::Color::from_rgb_u8((value >> 16) as u8, (value >> 8) as u8, value as u8))
}

/// Whole unix seconds now. A clock before 1970 reads as 0.
fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use facet_core::{category, face, segment};
    use slint::Model;
    use slint::platform::software_renderer::MinimalSoftwareWindow;
    use slint::platform::{Platform, WindowAdapter};

    struct Headless(Rc<MinimalSoftwareWindow>);

    impl Platform for Headless {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
            Ok(self.0.clone())
        }
    }

    const NO_LOG: Option<facet_core::debug_log::DebugLog> = None;

    fn names(data: &ReportData<'_>) -> Vec<String> {
        let totals = data.get_totals();
        (0..totals.row_count()).filter_map(|i| totals.row_data(i)).map(|t| t.name.to_string()).collect()
    }

    #[test]
    fn the_report_totals_today_sorts_folds_and_widens() {
        slint::platform::set_platform(Box::new(Headless(MinimalSoftwareWindow::new(Default::default()))))
            .expect("the headless platform should install");
        let path = std::env::temp_dir().join(format!("facet-ui-report-{}.sqlite", std::process::id()));
        if path.exists() {
            std::fs::remove_file(&path).expect("a stale test database should be removable");
        }
        let connection = database::open(&path, database::APPDATA_DDL).expect("the app DDL should apply");
        let today = report::today(&connection, now()).expect("today should read");
        let (start, _) = report::bounds(&connection, today, today).expect("bounds should read");
        let record = |name: &str, offset: i64, seconds: i64| {
            let id = category::insert(&connection, name).expect("should insert");
            face::assign(&connection, id, 13).expect("should assign");
            segment::start_segment(&connection, 13, start + offset, &NO_LOG).expect("should start");
            segment::close_open_segment(&connection, start + offset + seconds, &NO_LOG)
                .expect("should close");
        };
        record("Code", 60, 600);
        record("Mail", 1_000, 1_200);
        let yesterday =
            report::bounds(&connection, today.plus_days(-1), today.plus_days(-1)).expect("bounds").0;
        let old = category::insert(&connection, "Old").expect("should insert");
        face::assign(&connection, old, 13).expect("should assign");
        segment::start_segment(&connection, 13, yesterday + 60, &NO_LOG).expect("should start");
        segment::close_open_segment(&connection, yesterday + 360, &NO_LOG).expect("should close");

        let ui = SettingsWindow::new().expect("the window should build");
        let tab = Report::attach(&ui, path.clone(), Rc::new(Trace::none()));
        tab.open();
        let data = ui.global::<ReportData>();
        assert_eq!(names(&data), ["Mail", "Code"]);
        assert_eq!(data.get_time_heading(), "Time \u{25bc}");
        assert_eq!(data.get_to_subtitle(), "not set, reporting one day");
        let from_days = data.get_from_days();
        assert_eq!(from_days.row_count(), 42);
        let today_cell = (0..42)
            .filter_map(|i| from_days.row_data(i))
            .find(|d| d.iso == today.iso())
            .expect("today is drawn");
        assert!(today_cell.is_selected && today_cell.is_allowed);
        let tomorrow = today.plus_days(1).iso();
        assert!((0..42).filter_map(|i| from_days.row_data(i)).all(|d| d.iso != tomorrow || !d.is_allowed));
        assert!(!data.get_from_next_enabled());

        tab.sort_pressed("category");
        assert_eq!(names(&data), ["Code", "Mail"]);
        assert_eq!(data.get_category_heading(), "Category \u{25b2}");

        let code = data.get_totals().row_data(0).expect("a Code row");
        assert_eq!(code.duration, "0:10:00");
        tab.total_toggled(i64::from(code.id));
        let code = data.get_totals().row_data(0).expect("a Code row");
        assert!(code.is_expanded);
        assert_eq!(code.entries.row_count(), 1);

        tab.day_picked("from", &today.plus_days(-1).iso());
        assert_eq!(names(&data), ["Old"]);
        tab.day_picked("to", &today.iso());
        assert_eq!(names(&data), ["Code", "Mail", "Old"]);
        assert_eq!(data.get_to_subtitle(), "");

        tab.open();
        assert_eq!(data.get_to_subtitle(), "not set, reporting one day");
        assert!(!data.get_totals().row_data(0).expect("a row").is_expanded);

        std::fs::remove_file(&path).expect("the test database should be removable");
    }
}
