//! The Faces tab's behaviour: fills [`FacesData`] from the database and carries out its clicks.
//!
//! Every refresh opens the database and reads what it shows; nothing is held between refreshes except the
//! decoded icon images, which come from files compiled into the binary rather than from the database.
//!
//! Timing is always by hand in this build: there is no radio, so the app never waits for a cube (see
//! [`Faces::attach`]).

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use facet_core::category::{self, Category};
use facet_core::database;
use facet_core::debug_log::{Record, Tag, Trace, plain};
use facet_core::status_line::{self, StatusLine};
use facet_core::{face, segment, setting, timing};
use rusqlite::Connection;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::create::Creator;
use crate::notice::Notice;
use crate::{FaceCategory, FacesData, SettingsWindow, icons};

/// What the Device controller says about the cube, for the menu bar line: read when the line is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CubeFlags {
    /// A paired launch still reaching its cube.
    pub is_connecting: bool,
    pub is_locked: bool,
    pub is_battery_low: bool,
    /// The lit half of the low battery blink.
    pub is_blink_on: bool,
}

/// What the menu bar shows for the app's own clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuBarTiming {
    /// Whether the icon draws the pause glyph: anything other than running, idle included.
    pub is_paused: bool,
    /// Whether Pause, Resume and the left click do anything (see [`timing::is_clickable`]).
    pub is_clickable: bool,
    /// The menu item's title: `Resume` while paused, otherwise `Pause`.
    pub pause_title: &'static str,
    /// The line beside the icon.
    pub line: StatusLine,
}

/// Asks which face the connected cube says is up.
type CubeFaceSource = Box<dyn Fn() -> Option<u8>>;
/// Asks the Device controller for the cube's flags.
type CubeFlagsSource = Box<dyn Fn() -> CubeFlags>;
/// Told which cube face took a category, and why.
type FaceAssigned = Box<dyn Fn(i64, String)>;

/// The Faces tab, attached to one Settings window.
pub struct Faces {
    ui: slint::Weak<SettingsWindow>,
    database: PathBuf,
    log: Rc<Trace>,
    /// Per launch and one-way: set when the reconnect did not find the cube and the app was told to time by hand.
    has_given_up_on_cube: Cell<bool>,
    tick: slint::Timer,
    creator: Rc<Creator>,
    on_timing_changed: RefCell<Vec<Box<dyn Fn()>>>,
    /// Asks the Device controller which face the connected cube says is up. `None` with no cube connected.
    cube_face: RefCell<Option<CubeFaceSource>>,
    cube_flags: RefCell<Option<CubeFlagsSource>>,
    /// Told when a cube face takes a category, with the face and why, so the cube can be relit.
    on_face_assigned: RefCell<Vec<FaceAssigned>>,
    this: RefCell<Weak<Faces>>,
}

impl Faces {
    /// Wires the tab's callbacks on `ui` to `database` and closes any segment an earlier launch left open on
    /// an app face. Call once, at launch, before the window is shown.
    ///
    /// `has_given_up_on_cube` is passed to [`timing::is_manual_mode`] with the paired setting at every reading. A build
    /// with no radio passes `true`, so a paired cube never blocks timing by hand.
    pub fn attach(
        ui: &SettingsWindow,
        database: PathBuf,
        log: Rc<Trace>,
        has_given_up_on_cube: bool,
        notice: Rc<Notice>,
    ) -> Rc<Faces> {
        let faces = Rc::new(Faces {
            ui: ui.as_weak(),
            creator: Creator::new(database.clone(), Rc::clone(&log), notice),
            database,
            log,
            has_given_up_on_cube: Cell::new(has_given_up_on_cube),
            tick: slint::Timer::default(),
            on_timing_changed: RefCell::new(Vec::new()),
            cube_face: RefCell::new(None),
            cube_flags: RefCell::new(None),
            on_face_assigned: RefCell::new(Vec::new()),
            this: RefCell::new(Weak::new()),
        });
        *faces.this.borrow_mut() = Rc::downgrade(&faces);

        if let Some(connection) = faces.connect() {
            faces.report(segment::close_stranded_on_app_faces(&connection, &*faces.log));
        }

        let data = ui.global::<FacesData>();
        let weak = Rc::downgrade(&faces);
        let with = move |action: fn(&Faces)| {
            let weak = weak.clone();
            move || {
                if let Some(faces) = weak.upgrade() {
                    action(&faces);
                }
            }
        };
        data.on_play_pause_pressed(with(Faces::toggle_pause));
        data.on_face_lock_pressed(with(Faces::toggle_face_lock));
        data.on_create_opened(with(Faces::open_create));
        data.on_create_cancelled(with(Faces::cancel_create));

        let weak = Rc::downgrade(&faces);
        data.on_category_picked(move |id| {
            if let Some(faces) = weak.upgrade() {
                faces.pick(i64::from(id));
            }
        });
        let weak = Rc::downgrade(&faces);
        data.on_name_edited(move |text| {
            if let Some(faces) = weak.upgrade() {
                faces.limit_typed(&text);
            }
        });
        let weak = Rc::downgrade(&faces);
        data.on_create_saved(move |text| {
            if let Some(faces) = weak.upgrade() {
                faces.save(&text);
            }
        });
        faces
    }

    /// Re-reads the category list and the timing picture. Call when the window opens and when the Faces tab
    /// is shown.
    pub fn refresh(&self) {
        let Some(connection) = self.connect() else { return };
        let Some(ui) = self.ui.upgrade() else { return };
        match category::active(&connection) {
            Ok(categories) => {
                let rows: Vec<FaceCategory> = categories.iter().map(|c| self.row(c)).collect();
                ui.global::<FacesData>().set_categories(ModelRc::new(VecModel::from(rows)));
            }
            Err(error) => self.log.record_failure(Tag::Settings, || {
                format!("Faces: the category list would not read: {error}")
            }),
        }
        self.show_timing(&connection);
    }

    /// Gives the tab a way to ask which face the connected cube says is up. `None` from it means no cube connected.
    pub fn set_cube_face_source(&self, source: impl Fn() -> Option<u8> + 'static) {
        *self.cube_face.borrow_mut() = Some(Box::new(source));
    }

    /// Gives the menu bar line a way to ask the Device controller about the cube.
    pub fn set_cube_flags_source(&self, source: impl Fn() -> CubeFlags + 'static) {
        *self.cube_flags.borrow_mut() = Some(Box::new(source));
    }

    fn flags(&self) -> CubeFlags {
        self.cube_flags.borrow().as_ref().map(|source| source()).unwrap_or_default()
    }

    /// Adds something to run when a cube face takes a category, with the face and why.
    pub fn set_on_face_assigned(&self, assigned: impl Fn(i64, String) + 'static) {
        self.on_face_assigned.borrow_mut().push(Box::new(assigned));
    }

    fn live_cube_face(&self) -> Option<i64> {
        self.cube_face.borrow().as_ref().and_then(|source| source()).map(i64::from)
    }

    /// Stops waiting for the paired cube for the rest of this launch, so the app is its own clock.
    pub fn give_up_on_cube(&self) {
        if !self.has_given_up_on_cube.replace(true) {
            self.log.record(Tag::Timing, || {
                "Timing by hand for this launch, the cube not having been found".to_string()
            });
        }
        self.refresh_timing();
    }

    /// Whether the menu bar and this tab follow the cube rather than the app's own clock.
    pub fn is_following_cube(&self) -> bool {
        self.connect()
            .and_then(|connection| self.is_manual_mode(&connection))
            .is_some_and(|is_manual| !is_manual)
    }

    /// What the menu bar should show, read from the database now. `None` when the database cannot be read,
    /// which is logged. While a cube is followed it is the cube's open segment: paused unless one is open and
    /// running, and clickable while the cube is connected.
    pub fn menu_bar_timing(&self) -> Option<MenuBarTiming> {
        let connection = self.connect()?;
        let shows_seconds = self.report(setting::shows_seconds(&connection))?;
        let is_connected = self.report(setting::is_cube_connected(&connection))?;
        let flags = self.flags();
        let facts = |timed: Option<status_line::Timed>, is_following_cube: bool| status_line::StatusFacts {
            timed,
            is_following_cube,
            is_cube_connected: is_connected,
            is_connecting: flags.is_connecting,
            is_cube_locked: flags.is_locked,
            is_battery_low: flags.is_battery_low,
            is_blink_on: flags.is_blink_on,
        };
        if !self.is_manual_mode(&connection)? {
            let reading = self.report(timing::read_cube(&connection, now()))?;
            let is_paused = reading.as_ref().is_none_or(|reading| reading.is_paused);
            let timed = reading.as_ref().and_then(|reading| {
                reading.category.as_ref().map(|category| status_line::Timed {
                    category: category.name.clone(),
                    figure: timing::format_duration(reading.seconds, shows_seconds),
                    is_paused: reading.is_paused,
                    is_limit_reached: reading.is_limit_reached,
                })
            });
            return Some(MenuBarTiming {
                is_paused,
                is_clickable: is_connected
                    && !flags.is_locked
                    && !(is_paused && reading.as_ref().is_some_and(|reading| reading.is_limit_reached)),
                pause_title: if is_paused { "Resume" } else { "Pause" },
                line: status_line::line(&facts(timed, true)),
            });
        }
        let reading = self.report(timing::read(&connection, now()))?;
        let is_paused = reading.timing_state != timing::TimingState::Running;
        let timed = reading.category.as_ref().map(|category| status_line::Timed {
            category: category.name.clone(),
            figure: timing::format_duration(reading.seconds, shows_seconds),
            is_paused,
            is_limit_reached: reading.is_limit_reached,
        });
        Some(MenuBarTiming {
            is_paused,
            is_clickable: timing::is_clickable(reading.timing_state, reading.is_limit_reached),
            pause_title: if reading.timing_state == timing::TimingState::Paused { "Resume" } else { "Pause" },
            line: status_line::line(&facts(timed, false)),
        })
    }

    /// Adds something to run whenever the timing picture has been re-read: after every click on the tab,
    /// every menu bar toggle and every tick. Every callback added runs, in the order added.
    pub fn set_on_timing_changed(&self, changed: impl Fn() + 'static) {
        self.on_timing_changed.borrow_mut().push(Box::new(changed));
    }

    /// Re-reads the timing picture, without the category list, and tells the timing-changed callback.
    pub fn refresh_timing(&self) {
        if let Some(connection) = self.connect() {
            self.show_timing(&connection);
        }
    }

    /// Closes the open app-face segment. Call during quit.
    pub fn quit(&self) {
        self.tick.stop();
        if let Some(connection) = self.connect() {
            self.report(segment::close_open_segment(&connection, now(), &*self.log));
        }
    }

    fn connect(&self) -> Option<Connection> {
        match database::connect(&self.database) {
            Ok(connection) => Some(connection),
            Err(error) => {
                self.log
                    .record_failure(Tag::Database, || format!("Faces: the database would not open: {error}"));
                None
            }
        }
    }

    /// Logs a failed database call, and gives back the value of a successful one.
    fn report<T>(&self, result: Result<T, rusqlite::Error>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.log.record_failure(Tag::Database, || format!("Faces: a database call failed: {error}"));
                None
            }
        }
    }

    fn is_manual_mode(&self, connection: &Connection) -> Option<bool> {
        let is_cube_paired = self.report(setting::is_cube_paired(connection))?;
        Some(timing::is_manual_mode(is_cube_paired, self.has_given_up_on_cube.get()))
    }

    /// Sets the timing column from a fresh reading, tells the timing-changed callback, and runs the
    /// one-second tick while the figure is moving, whether or not the window is on screen.
    fn show_timing(&self, connection: &Connection) {
        let Some(ui) = self.ui.upgrade() else { return };
        let is_manual_mode = self.is_manual_mode(connection).unwrap_or(true);
        if !is_manual_mode {
            self.show_cube_timing(connection);
            return;
        }
        let Some(reading) = self.report(timing::read(connection, now())) else { return };
        let data = ui.global::<FacesData>();
        data.set_is_following_cube(false);
        data.set_has_category(reading.category.is_some());
        data.set_running(reading.timing_state == timing::TimingState::Running);
        data.set_timing_category(
            reading.category.as_ref().map(|c| c.name.as_str()).unwrap_or_default().into(),
        );
        let colour = reading.category.as_ref().and_then(|c| colour(c.colour_hex.as_deref()));
        data.set_has_timing_colour(colour.is_some());
        data.set_timing_colour(colour.unwrap_or_default());
        data.set_elapsed(timing::format_duration(reading.seconds, true).into());
        data.set_glyph_enabled(timing::is_clickable(reading.timing_state, reading.is_limit_reached));
        data.set_rows_enabled(timing::click(is_manual_mode) == timing::Click::StartTiming);

        for changed in self.on_timing_changed.borrow().iter() {
            changed();
        }

        if reading.is_counting {
            if !self.tick.running() {
                let weak = self.this.borrow().clone();
                self.tick.start(slint::TimerMode::Repeated, Duration::from_secs(1), move || {
                    if let Some(faces) = weak.upgrade() {
                        faces.on_tick();
                    }
                });
            }
        } else {
            self.tick.stop();
        }
    }

    /// Sets the timing column from the cube's open segment. The glyph and the rows are dead: the cube is the clock,
    /// and pausing it is the menu bar's.
    fn show_cube_timing(&self, connection: &Connection) {
        let Some(ui) = self.ui.upgrade() else { return };
        let Some(reading) = self.report(timing::read_cube(connection, now())) else { return };
        let data = ui.global::<FacesData>();
        let category = reading.as_ref().and_then(|reading| reading.category.as_ref());
        data.set_has_category(category.is_some());
        data.set_running(reading.as_ref().is_some_and(|reading| !reading.is_paused));
        data.set_timing_category(category.map(|c| c.name.as_str()).unwrap_or_default().into());
        let colour = category.and_then(|c| colour(c.colour_hex.as_deref()));
        data.set_has_timing_colour(colour.is_some());
        data.set_timing_colour(colour.unwrap_or_default());
        data.set_elapsed(
            timing::format_duration(reading.as_ref().map_or(0, |reading| reading.seconds), true).into(),
        );
        data.set_glyph_enabled(false);
        let face = self.live_cube_face();
        let is_face_locked = match face {
            Some(face) => self.report(face::is_face_locked(connection, face)).flatten().unwrap_or(false),
            None => false,
        };
        data.set_is_following_cube(true);
        data.set_device_face(face.map_or(0, |face| face as i32));
        data.set_is_face_locked(is_face_locked);
        data.set_rows_enabled(face.is_some() && !is_face_locked);
        for changed in self.on_timing_changed.borrow().iter() {
            changed();
        }
        if reading.as_ref().is_some_and(|reading| reading.is_counting) {
            if !self.tick.running() {
                let weak = self.this.borrow().clone();
                self.tick.start(slint::TimerMode::Repeated, Duration::from_secs(1), move || {
                    if let Some(faces) = weak.upgrade() {
                        faces.on_tick();
                    }
                });
            }
        } else {
            self.tick.stop();
        }
    }

    fn on_tick(&self) {
        let Some(connection) = self.connect() else { return };
        if self.is_manual_mode(&connection) == Some(false) {
            self.show_cube_timing(&connection);
            return;
        }
        let instant = now();
        self.report(timing::enforce_daily_limit(&connection, instant, &*self.log));
        self.report(segment::refresh_open_segment(&connection, instant));
        self.show_timing(&connection);
    }

    fn pick(&self, category_id: i64) {
        self.log.record(Tag::Click, || format!("Button clicked: category_id {category_id}"));
        let Some(connection) = self.connect() else { return };
        match self.is_manual_mode(&connection).map(timing::click) {
            Some(timing::Click::StartTiming) => {
                self.report(timing::start_timing(&connection, category_id, now(), &*self.log));
            }
            Some(timing::Click::WaitingForTheDevice) => match self.live_cube_face() {
                Some(face) => self.assign_to_cube(&connection, face, category_id),
                None => self.log.record(Tag::Timing, || {
                    format!("Timing: category_id {category_id} was not started, a device is paired")
                }),
            },
            None => {}
        }
        self.show_timing(&connection);
    }

    /// Puts `category_id` on the cube's `face`, which the cube is resting on: no segment is opened, the cube being the
    /// clock. A locked face keeps what it has.
    fn assign_to_cube(&self, connection: &Connection, face: i64, category_id: i64) {
        let Some(Some(category)) = self.report(category::by_id(connection, category_id)) else { return };
        let name = plain(&category.name);
        if self.report(face::is_face_locked(connection, face)).flatten() == Some(true) {
            self.log.record(Tag::Timing, || {
                format!("Face {face} is locked, so it keeps what it has rather than taking {name}")
            });
            return;
        }
        if self.report(face::category_id(connection, face)).flatten() == Some(category_id) {
            self.log.record(Tag::Timing, || {
                format!("Face {face} already holds {name}, so the click changes nothing")
            });
            return;
        }
        if self.report(face::assign(connection, category_id, face)) != Some(true) {
            self.log.record(Tag::Timing, || format!("Face {face} refused category {name}"));
            return;
        }
        self.log.record(Tag::Timing, || format!("Face {face} now holds {name} (category_id {category_id})"));
        for assigned in self.on_face_assigned.borrow().iter() {
            assigned(face, format!("face {face} took {name}"));
        }
    }

    /// Locks or unlocks the face the cube is resting on, read at the press, and redraws from the table.
    fn toggle_face_lock(&self) {
        let Some(face) = self.live_cube_face() else {
            self.log.record(Tag::Click, || "The lock was pressed with no cube face to lock".to_string());
            return;
        };
        let Some(connection) = self.connect() else { return };
        let wanted = self.report(face::is_face_locked(&connection, face)).flatten() != Some(true);
        self.log.record(Tag::Click, || {
            format!("Button clicked: face {face} lock -> {}", if wanted { "locked" } else { "unlocked" })
        });
        if self.report(face::set_locked(&connection, face, wanted)) != Some(true) {
            self.log.record(Tag::Click, || format!("Face {face} would not take the lock"));
        }
        self.refresh();
    }

    /// Pauses the clock if it is running and resumes it if it is paused. Refused when idle or when a resume
    /// would pass a spent daily limit. The one path for the glyph, the menu item and the left click.
    pub fn toggle_pause(&self) {
        self.log.record(Tag::Click, || "Button clicked: play pause".to_string());
        let Some(connection) = self.connect() else { return };
        self.report(timing::toggle_pause(&connection, now(), &*self.log));
        self.show_timing(&connection);
    }

    fn open_create(&self) {
        self.log.record(Tag::Click, || "Button clicked: Create".to_string());
        if let Some(ui) = self.ui.upgrade() {
            let data = ui.global::<FacesData>();
            data.set_typed_name(SharedString::new());
            data.set_creating(true);
        }
    }

    fn cancel_create(&self) {
        self.log.record(Tag::Click, || "Create cancelled".to_string());
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<FacesData>().set_creating(false);
        }
    }

    /// Cuts the name being typed to the longest a category may have.
    fn limit_typed(&self, text: &str) {
        if text.chars().count() > category::MAXIMUM_NAME_LENGTH
            && let Some(ui) = self.ui.upgrade()
        {
            let cut: String = text.chars().take(category::MAXIMUM_NAME_LENGTH).collect();
            ui.global::<FacesData>().set_typed_name(cut.into());
        }
    }

    fn save(&self, typed: &str) {
        let Some(ui) = self.ui.upgrade() else { return };
        ui.global::<FacesData>().set_creating(false);
        let weak = self.this.borrow().clone();
        // Creating on the Faces tab starts timing whatever was written.
        self.creator.save(typed, move |written| {
            if let Some(faces) = weak.upgrade() {
                faces.refresh();
                if let Some(id) = written {
                    faces.pick(id);
                }
            }
        });
    }

    fn row(&self, category: &Category) -> FaceCategory {
        let colour = colour(category.colour_hex.as_deref());
        let icon = category.icon_name.as_deref().and_then(|name| self.icon(name));
        FaceCategory {
            id: i32::try_from(category.id).unwrap_or(i32::MAX),
            name: category.name.as_str().into(),
            colour: colour.unwrap_or_default(),
            has_colour: colour.is_some(),
            has_icon: icon.is_some(),
            icon: icon.unwrap_or_default(),
            white_lines: category.uses_white_lines,
        }
    }

    /// The decoded icon called `name`. `None`, and a logged failure, when it has no file or will not decode.
    fn icon(&self, name: &str) -> Option<slint::Image> {
        match icons::image(name) {
            Ok(Some(image)) => Some(image),
            Ok(None) => {
                self.log.record_failure(Tag::Settings, || format!("Faces: no icon file for {}", plain(name)));
                None
            }
            Err(error) => {
                self.log.record_failure(Tag::Settings, || {
                    format!("Faces: icon {} would not decode: {error:?}", plain(name))
                });
                None
            }
        }
    }
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
    use slint::Model;
    use slint::platform::software_renderer::MinimalSoftwareWindow;
    use slint::platform::{Platform, WindowAdapter};

    struct Headless(Rc<MinimalSoftwareWindow>);

    impl Platform for Headless {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn colours_parse_from_the_stored_hex() {
        assert_eq!(colour(Some("#00ffff")), Some(slint::Color::from_rgb_u8(0, 255, 255)));
        assert_eq!(colour(None), None);
        assert_eq!(colour(Some("teal")), None);
        assert_eq!(colour(Some("#fff")), None);
    }

    #[test]
    fn every_icon_file_decodes() {
        for (name, bytes) in icons::ICONS {
            assert!(slint::Image::load_from_svg_data(bytes).is_ok(), "{name} would not decode");
        }
    }

    /// The only test in this crate that builds a window: Slint takes one platform per process.
    #[test]
    fn picking_a_category_times_it_and_the_glyph_pauses_it() {
        slint::platform::set_platform(Box::new(Headless(MinimalSoftwareWindow::new(Default::default()))))
            .expect("the headless platform should install");
        let path = std::env::temp_dir().join(format!("facet-ui-faces-{}.sqlite", std::process::id()));
        if path.exists() {
            std::fs::remove_file(&path).expect("a stale test database should be removable");
        }
        database::open(&path, database::APPDATA_DDL).expect("the app DDL should apply");

        let ui = SettingsWindow::new().expect("the window should build");
        let notice = Notice::attach(&ui);
        let faces = Faces::attach(&ui, path.clone(), Rc::new(Trace::none()), true, Rc::clone(&notice));
        let changes = Rc::new(std::cell::Cell::new(0));
        let counted = Rc::clone(&changes);
        faces.set_on_timing_changed(move || counted.set(counted.get() + 1));
        faces.refresh();
        assert_eq!(
            faces.menu_bar_timing().map(|timing| (timing.is_paused, timing.is_clickable, timing.pause_title)),
            Some((true, false, "Pause"))
        );
        assert_eq!(faces.menu_bar_timing().map(|timing| timing.line.text()), Some("Facet".to_string()));
        let data = ui.global::<FacesData>();
        let names: Vec<String> = (0..data.get_categories().row_count())
            .filter_map(|i| data.get_categories().row_data(i))
            .map(|row| row.name.to_string())
            .collect();
        assert_eq!(names, ["Break", "Meeting"]);
        assert!(!data.get_has_category());
        assert!(data.get_rows_enabled());
        let meeting = data.get_categories().row_data(1).expect("a second row");
        assert!(meeting.has_icon && meeting.has_colour);

        faces.pick(i64::from(meeting.id));
        assert!(data.get_has_category());
        assert!(data.get_running());
        assert_eq!(data.get_timing_category(), "Meeting");
        // A second can turn over between the start and the read.
        assert!(["0:00:00", "0:00:01"].contains(&data.get_elapsed().as_str()), "{}", data.get_elapsed());
        assert!(data.get_glyph_enabled());

        assert_eq!(
            faces.menu_bar_timing().map(|timing| (timing.is_paused, timing.is_clickable, timing.pause_title)),
            Some((false, true, "Pause"))
        );

        let before = changes.get();
        faces.toggle_pause();
        assert!(!data.get_running());
        assert!(data.get_has_category());
        assert!(changes.get() > before, "a toggle should tell the timing-changed callback");
        assert_eq!(
            faces.menu_bar_timing().map(|timing| (timing.is_paused, timing.is_clickable, timing.pause_title)),
            Some((true, true, "Resume"))
        );

        faces.save("  Deep   work ");
        assert_eq!(data.get_timing_category(), "Deep work");
        assert!(data.get_running());

        faces.save("meeting");
        assert_eq!(notice.title(), "That category already exists");
        notice.choose(0);
        assert_eq!(notice.title(), "");

        std::fs::remove_file(&path).expect("the test database should be removable");
    }
}
