//! The Categories tab's behaviour: fills [`CategoriesData`] from the database and carries out its changes.
//!
//! Every refresh reads both lists, the icon choices and the colour choices. Every change is written through
//! and the table is read back afterwards; a write the table refuses puts the row back as the table holds it
//! and says so in a notice. Creating a category here does not start timing it.
//!
//! This build has no radio, so a change that would relight the cube's faces is logged instead.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

use facet_core::category::{self, Category, ReinstateDecision, RenameDecision};
use facet_core::debug_log::{Record, Tag, Trace, plain};
use facet_core::{database, face, reference, time_entry};
use rusqlite::Connection;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::create::Creator;
use crate::notice::Notice;
use crate::{
    ActiveCategory, CategoriesData, ColourChoice, IconChoice, RetiredCategory, SettingsWindow, icons,
};

/// The Categories tab, attached to one Settings window.
pub struct Categories {
    ui: slint::Weak<SettingsWindow>,
    database: PathBuf,
    log: Rc<Trace>,
    notice: Rc<Notice>,
    creator: Rc<Creator>,
    on_changed: RefCell<Option<Box<dyn Fn()>>>,
    this: RefCell<Weak<Categories>>,
}

impl Categories {
    /// Wires the tab's callbacks on `ui` to `database`. Call once, at launch.
    pub fn attach(
        ui: &SettingsWindow,
        database: PathBuf,
        log: Rc<Trace>,
        notice: Rc<Notice>,
    ) -> Rc<Categories> {
        let categories = Rc::new(Categories {
            ui: ui.as_weak(),
            creator: Creator::new(database.clone(), Rc::clone(&log), Rc::clone(&notice)),
            database,
            log,
            notice,
            on_changed: RefCell::new(None),
            this: RefCell::new(Weak::new()),
        });
        *categories.this.borrow_mut() = Rc::downgrade(&categories);

        let data = ui.global::<CategoriesData>();
        macro_rules! wire {
            ($setter:ident, |$faces:ident $(, $arg:ident)*| $body:expr) => {{
                let weak = Rc::downgrade(&categories);
                data.$setter(move |$($arg),*| {
                    if let Some($faces) = weak.upgrade() {
                        $body;
                    }
                });
            }};
        }
        wire!(on_section_toggled, |c, name, open| c.section_toggled(&name, open));
        wire!(on_active_toggled, |c, id, checked| c.active_toggled(i64::from(id), checked));
        wire!(on_retired_toggled, |c, id, checked| c.retired_toggled(i64::from(id), checked));
        wire!(on_rename_opened, |c, id| c.rename_opened(i64::from(id)));
        wire!(on_rename_edited, |c, text| c.limit_editing(&text));
        wire!(on_rename_committed, |c, id, text| c.rename_committed(i64::from(id), &text));
        wire!(on_icon_opened, |c, id| c.icon_opened(i64::from(id)));
        wire!(on_icon_picked, |c, id, icon| c.icon_picked(i64::from(id), i64::from(icon)));
        wire!(on_colour_opened, |c, id| c.colour_opened(i64::from(id)));
        wire!(on_colour_picked, |c, id, colour| c.colour_picked(i64::from(id), i64::from(colour)));
        wire!(on_limit_edited, |c, id, minutes| c.limit_edited(i64::from(id), i64::from(minutes)));
        wire!(on_create_opened, |c| c.open_create());
        wire!(on_name_edited, |c, text| c.limit_typed(&text));
        wire!(on_create_saved, |c, text| c.save(&text));
        wire!(on_create_cancelled, |c| c.cancel_create());
        categories
    }

    /// Sets what runs after a change that can alter what is being timed or how it is drawn: a rename, a new
    /// icon, colour or limit, a retire or a reinstate. Replaces any earlier callback.
    pub fn set_on_changed(&self, changed: impl Fn() + 'static) {
        *self.on_changed.borrow_mut() = Some(Box::new(changed));
    }

    /// Re-reads both lists and the icon and colour choices. Call when the window opens and when the
    /// Categories tab is shown.
    pub fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let Some(connection) = self.connect() else { return };
        let data = ui.global::<CategoriesData>();

        if let Some(active) = self.report(category::active(&connection)) {
            let rows: Vec<ActiveCategory> =
                active.iter().filter_map(|c| self.active_row(&connection, c)).collect();
            data.set_active(ModelRc::new(VecModel::from(rows)));
        }
        if let Some(inactive) = self.report(category::inactive(&connection)) {
            let rows: Vec<RetiredCategory> =
                inactive.iter().filter_map(|c| self.retired_row(&connection, c)).collect();
            data.set_inactive(ModelRc::new(VecModel::from(rows)));
        }
        if let Some(icons) = self.report(reference::icons(&connection)) {
            let cells: Vec<IconChoice> = icons
                .iter()
                .map(|icon| IconChoice {
                    id: to_i32(icon.id),
                    file: icon.name.as_str().into(),
                    label: icon.display_name().into(),
                    image: self.icon(&icon.name).unwrap_or_default(),
                })
                .collect();
            data.set_icons(ModelRc::new(VecModel::from(cells)));
        }
        if let Some(colours) = self.report(reference::colours(&connection)) {
            let rows: Vec<ColourChoice> = colours
                .iter()
                .filter_map(|c| {
                    Some(ColourChoice {
                        id: to_i32(c.id),
                        name: c.name.as_str().into(),
                        colour: colour(Some(&c.hex))?,
                    })
                })
                .collect();
            data.set_colours(ModelRc::new(VecModel::from(rows)));
        }
    }

    fn active_row(&self, connection: &Connection, category: &Category) -> Option<ActiveCategory> {
        let locked = self.report(face::locked_faces_holding(connection, category.id))?;
        let colour = colour(category.colour_hex.as_deref());
        let icon = category.icon_name.as_deref().and_then(|name| self.icon(name));
        let icon_label = match category.icon_name.as_deref() {
            Some(name) => format!(
                "{} icon, {}",
                category.name,
                reference::Icon { id: category.icon_id, name: name.to_string() }.display_name()
            ),
            None => format!("{} icon, none", category.name),
        };
        Some(ActiveCategory {
            id: to_i32(category.id),
            name: category.name.as_str().into(),
            colour: colour.unwrap_or_default(),
            has_colour: colour.is_some(),
            colour_name: category.colour_name.clone().unwrap_or_default().into(),
            has_icon: icon.is_some(),
            icon: icon.unwrap_or_default(),
            icon_label: icon_label.into(),
            daily_limit: to_i32(category.daily_limit_minutes),
            locked_note: locked_note(&locked, &category.name).into(),
        })
    }

    fn retired_row(&self, connection: &Connection, category: &Category) -> Option<RetiredCategory> {
        let last_used = match self.report(time_entry::last_used(connection, category.id))? {
            Some(epoch) => self.report(time_entry::format_local(connection, epoch))?,
            None => "Never".to_string(),
        };
        Some(RetiredCategory {
            id: to_i32(category.id),
            name: category.name.as_str().into(),
            last_used: last_used.into(),
        })
    }

    fn section_toggled(&self, name: &str, open: bool) {
        self.log.record(Tag::Settings, || {
            format!("Categories section {name} {}", if open { "opened" } else { "folded" })
        });
    }

    fn active_toggled(&self, id: i64, checked: bool) {
        if checked {
            self.refresh();
            return;
        }
        let Some(connection) = self.connect() else { return };
        let Some(current) = self.read(&connection, id) else { return };
        let name = plain(&current.name);
        match self.report(category::retire(&connection, id)) {
            Some(Some(cleared)) => {
                self.log.record(Tag::Click, || {
                    format!("Category {name} retired, cleared from face(s) {cleared:?}")
                });
                if !cleared.is_empty() {
                    self.log.record(Tag::Settings, || {
                        format!("No cube connected, so {name} was retired lights nothing")
                    });
                }
            }
            _ => {
                self.log.record(Tag::Click, || format!("Category {name} retire REFUSED"));
                self.refused(&current.name, "retiring it");
            }
        }
        self.refresh();
        self.changed();
    }

    fn retired_toggled(&self, id: i64, checked: bool) {
        if !checked {
            self.refresh();
            return;
        }
        let Some(connection) = self.connect() else { return };
        let Some(current) = self.read(&connection, id) else { return };
        let name = plain(&current.name);
        match self.report(category::decide_reinstate(&connection, id)).flatten() {
            Some(ReinstateDecision::Refused { active_namesake }) => {
                self.log.record(Tag::Click, || {
                    format!(
                        "Category {name} reinstate REFUSED: category_id {} is active under that name",
                        active_namesake.id
                    )
                });
                self.refresh();
                self.notice.tell(
                    "That name is already in use",
                    &format!(
                        "An active category is already called \u{201c}{}\u{201d}, so this one cannot be reinstated \
                         under that name.\n\nRename one of them first, then try again.",
                        active_namesake.name
                    ),
                );
            }
            Some(ReinstateDecision::Allowed) => {
                let reinstated = self.report(category::set_active(&connection, id, true)).unwrap_or(false);
                self.log.record(Tag::Click, || {
                    format!(
                        "Category {name} reinstated{}",
                        if reinstated { "" } else { " REFUSED by the index" }
                    )
                });
                if !reinstated {
                    self.refused(&current.name, "reinstating it");
                }
                self.refresh();
                self.changed();
            }
            None => self.refresh(),
        }
    }

    fn rename_opened(&self, id: i64) {
        let Some(connection) = self.connect() else { return };
        let Some(current) = self.read(&connection, id) else { return };
        if let Some(ui) = self.ui.upgrade() {
            let data = ui.global::<CategoriesData>();
            data.set_editing_name(current.name.as_str().into());
            data.set_editing_id(to_i32(id));
        }
    }

    /// Cuts the name being typed into a row to the longest a category may have.
    fn limit_editing(&self, text: &str) {
        if text.chars().count() > category::MAXIMUM_NAME_LENGTH
            && let Some(ui) = self.ui.upgrade()
        {
            let cut: String = text.chars().take(category::MAXIMUM_NAME_LENGTH).collect();
            ui.global::<CategoriesData>().set_editing_name(cut.into());
        }
    }

    fn rename_committed(&self, id: i64, typed: &str) {
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<CategoriesData>().set_editing_id(-1);
        }
        let Some(connection) = self.connect() else { return };
        let Some(current) = self.read(&connection, id) else { return };
        let Some(decision) = self.report(category::decide_rename(&connection, &current, typed)) else {
            return;
        };
        let old = current.name.clone();
        let (new_name, title, message, confirm) = match decision {
            RenameDecision::Ignore => {
                self.log.record(Tag::Settings, || {
                    format!("Category {} rename ignored, nothing changed", plain(&old))
                });
                return;
            }
            RenameDecision::Refuse { active_namesake } => {
                let title = "That name is already in use";
                self.log.record(Tag::Settings, || {
                    format!(
                        "Category {} rename -> {}, asking: {title}",
                        plain(&old),
                        plain(&category::normalise(typed))
                    )
                });
                let message = format!(
                    "An active category is already called \u{201c}{}\u{201d}, so this name is taken.\n\nRename that \
                     one first, or pick another name.",
                    active_namesake.name
                );
                let this = self.this.borrow().clone();
                self.notice.ask(title, &message, &["Cancel"], move |_| {
                    if let Some(this) = this.upgrade() {
                        this.log.record(Tag::Click, || {
                            format!("Button clicked: Cancel, {} rename refused, name taken", plain(&old))
                        });
                    }
                });
                return;
            }
            RenameDecision::Confirm { name } => {
                let message = history_warning(&old, &name);
                (name, "Rename this category?", message, "Rename")
            }
            RenameDecision::ConfirmAgainstRetired { name, retired } => {
                let count = if retired.len() == 1 {
                    format!("There is one inactive category called \u{201c}{name}\u{201d}.")
                } else {
                    format!("There are {} inactive categories called \u{201c}{name}\u{201d}.", retired.len())
                };
                let message = format!(
                    "{count} Renaming to it leaves two categories with that name, and telling them apart in a report \
                     later is on you.\n\n{}",
                    history_warning(&old, &name)
                );
                (name, "That category already exists", message, "Rename anyway")
            }
            RenameDecision::ConfirmAgainstActive { name, active_namesake } => {
                let message = format!(
                    "\u{201c}{}\u{201d} is an active category. This one is inactive, so it may share the name: only \
                     active names have to be unique.\n\nWhile it does, it cannot be brought back, because ticking \
                     Active would be refused. Renaming either of them frees it again.\n\n{}",
                    active_namesake.name,
                    history_warning(&old, &name)
                );
                (name, "An active category is called that", message, "Rename anyway")
            }
        };
        self.log.record(Tag::Settings, || {
            format!("Category {} rename -> {}, asking: {title}", plain(&old), plain(&new_name))
        });
        let this = self.this.borrow().clone();
        self.notice.ask(title, &message, &["Cancel", confirm], move |index| {
            if let Some(this) = this.upgrade() {
                this.rename_answered(id, &old, &new_name, index == 1, confirm);
            }
        });
    }

    fn rename_answered(&self, id: i64, old: &str, new_name: &str, confirmed: bool, button: &str) {
        if !confirmed {
            self.log.record(Tag::Click, || format!("Button clicked: Cancel, {} not renamed", plain(old)));
            return;
        }
        let Some(connection) = self.connect() else { return };
        let stored = match category::set_name(&connection, id, new_name) {
            Ok(stored) => stored,
            Err(error) => {
                self.log
                    .record_failure(Tag::Database, || format!("Categories: the rename was refused: {error}"));
                false
            }
        };
        self.log.record(Tag::Click, || {
            format!(
                "Button clicked: {button} {} -> {}{}",
                plain(old),
                plain(new_name),
                if stored { "" } else { " REFUSED by the index" }
            )
        });
        if !stored {
            self.refused(old, "the new name");
        }
        self.refresh();
        self.changed();
    }

    fn icon_opened(&self, id: i64) {
        let Some(connection) = self.connect() else { return };
        let Some(current) = self.read(&connection, id) else { return };
        if let Some(ui) = self.ui.upgrade() {
            let data = ui.global::<CategoriesData>();
            data.set_current_icon_id(to_i32(current.icon_id));
            data.set_icon_for(to_i32(id));
        }
    }

    fn icon_picked(&self, id: i64, picked: i64) {
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<CategoriesData>().set_icon_for(-1);
        }
        let Some(connection) = self.connect() else { return };
        let Some(current) = self.read(&connection, id) else { return };
        let icon_id = category::toggled_choice(current.icon_id, picked);
        let stored = self.report(category::set_icon(&connection, id, icon_id)).unwrap_or(false);
        self.log.record(Tag::Settings, || {
            format!(
                "Category {} icon -> icon_id {icon_id}{}",
                plain(&current.name),
                if stored { "" } else { " REFUSED" }
            )
        });
        if !stored {
            self.refused(&current.name, "the icon");
        }
        self.refresh();
        self.changed();
    }

    fn colour_opened(&self, id: i64) {
        let Some(connection) = self.connect() else { return };
        let Some(current) = self.read(&connection, id) else { return };
        if let Some(ui) = self.ui.upgrade() {
            let data = ui.global::<CategoriesData>();
            data.set_current_colour_id(to_i32(current.colour_id));
            data.set_colour_for(to_i32(id));
        }
    }

    fn colour_picked(&self, id: i64, picked: i64) {
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<CategoriesData>().set_colour_for(-1);
        }
        let Some(connection) = self.connect() else { return };
        let Some(current) = self.read(&connection, id) else { return };
        let colour_id = category::toggled_choice(current.colour_id, picked);
        let stored = self.report(category::set_colour(&connection, id, colour_id)).unwrap_or(false);
        let name = plain(&current.name);
        self.log.record(Tag::Settings, || {
            format!("Category {name} colour -> colour_id {colour_id}{}", if stored { "" } else { " REFUSED" })
        });
        if stored {
            self.log.record(Tag::Settings, || {
                format!("No cube connected, so {name} was recoloured lights nothing")
            });
        } else {
            self.refused(&current.name, "the colour");
        }
        self.refresh();
        self.changed();
    }

    fn limit_edited(&self, id: i64, minutes: i64) {
        let Some(connection) = self.connect() else { return };
        let Some(current) = self.read(&connection, id) else { return };
        match self.report(category::set_daily_limit(&connection, id, minutes)).flatten() {
            Some(allowed) => {
                self.log.record(Tag::Settings, || {
                    format!("Category {} daily limit -> {allowed}min", plain(&current.name))
                });
                self.changed();
            }
            None => {
                self.log.record(Tag::Settings, || {
                    format!("Category {} daily limit -> {}min REFUSED", plain(&current.name), minutes)
                });
                self.refused(&current.name, "the daily limit");
                self.refresh();
            }
        }
    }

    fn open_create(&self) {
        self.log.record(Tag::Click, || "Button clicked: Create".to_string());
        if let Some(ui) = self.ui.upgrade() {
            let data = ui.global::<CategoriesData>();
            data.set_typed_name(SharedString::new());
            data.set_creating(true);
        }
    }

    fn cancel_create(&self) {
        self.log.record(Tag::Click, || "Create cancelled".to_string());
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<CategoriesData>().set_creating(false);
        }
    }

    /// Cuts the name being typed into Create to the longest a category may have.
    fn limit_typed(&self, text: &str) {
        if text.chars().count() > category::MAXIMUM_NAME_LENGTH
            && let Some(ui) = self.ui.upgrade()
        {
            let cut: String = text.chars().take(category::MAXIMUM_NAME_LENGTH).collect();
            ui.global::<CategoriesData>().set_typed_name(cut.into());
        }
    }

    fn save(&self, typed: &str) {
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<CategoriesData>().set_creating(false);
        }
        let weak = self.this.borrow().clone();
        // Creating on the Categories tab starts nothing: the list is re-read and that is all.
        self.creator.save(typed, move |_| {
            if let Some(categories) = weak.upgrade() {
                categories.refresh();
            }
        });
    }

    /// Says a write was refused. The caller re-reads, which puts the row back as the table holds it.
    fn refused(&self, category_name: &str, what: &str) {
        self.notice.tell(
            "The change was not saved",
            &format!(
                "The database refused {what} for \u{201c}{category_name}\u{201d}, so it shows what the database \
                 holds."
            ),
        );
    }

    fn changed(&self) {
        if let Some(changed) = self.on_changed.borrow().as_ref() {
            changed();
        }
    }

    fn read(&self, connection: &Connection, id: i64) -> Option<Category> {
        match self.report(category::by_id(connection, id))? {
            Some(category) => Some(category),
            None => {
                self.log
                    .record_failure(Tag::Settings, || format!("Categories: no category_id {id} to change"));
                None
            }
        }
    }

    fn icon(&self, name: &str) -> Option<slint::Image> {
        match icons::image(name) {
            Ok(image) => image,
            Err(error) => {
                self.log.record_failure(Tag::Settings, || {
                    format!("Categories: icon {} would not decode: {error:?}", plain(name))
                });
                None
            }
        }
    }

    fn connect(&self) -> Option<Connection> {
        match database::connect(&self.database) {
            Ok(connection) => Some(connection),
            Err(error) => {
                self.log.record_failure(Tag::Database, || {
                    format!("Categories: the database would not open: {error}")
                });
                None
            }
        }
    }

    fn report<T>(&self, result: Result<T, rusqlite::Error>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.log
                    .record_failure(Tag::Database, || format!("Categories: a database call failed: {error}"));
                None
            }
        }
    }
}

/// What a rename keeps and what it changes, said in every rename question.
fn history_warning(old: &str, new: &str) -> String {
    format!(
        "\u{201c}{old}\u{201d} keeps all of its history: nothing recorded against it is lost.\n\nBut everything links \
         to a category by its id rather than by its name, so reports covering time before the rename will show \
         \u{201c}{new}\u{201d} too, not the name that was in use then."
    )
}

/// Why a row cannot be changed, from the locked faces holding it. Empty when none do.
fn locked_note(locked: &[i64], name: &str) -> String {
    match locked {
        [] => String::new(),
        [face] => format!(
            "Face {face} is locked and still holding \u{201c}{name}\u{201d}. Unlock it on the Faces tab to change this \
             category."
        ),
        faces => format!(
            "Faces {} are locked and still holding \u{201c}{name}\u{201d}. Unlock them on the Faces tab to change \
             this category.",
            faces.iter().map(i64::to_string).collect::<Vec<_>>().join(", ")
        ),
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

fn to_i32(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
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

    fn names(model: ModelRc<ActiveCategory>) -> Vec<String> {
        (0..model.row_count()).filter_map(|i| model.row_data(i)).map(|row| row.name.to_string()).collect()
    }

    #[test]
    fn the_tab_creates_renames_recolours_limits_retires_and_reinstates() {
        slint::platform::set_platform(Box::new(Headless(MinimalSoftwareWindow::new(Default::default()))))
            .expect("the headless platform should install");
        let path = std::env::temp_dir().join(format!("facet-ui-categories-{}.sqlite", std::process::id()));
        if path.exists() {
            std::fs::remove_file(&path).expect("a stale test database should be removable");
        }
        database::open(&path, database::APPDATA_DDL).expect("the app DDL should apply");
        let ui = SettingsWindow::new().expect("the window should build");
        let notice = Notice::attach(&ui);
        let tab = Categories::attach(&ui, path.clone(), Rc::new(Trace::none()), Rc::clone(&notice));
        let changes = Rc::new(std::cell::Cell::new(0));
        let counted = Rc::clone(&changes);
        tab.set_on_changed(move || counted.set(counted.get() + 1));
        tab.refresh();
        let data = ui.global::<CategoriesData>();
        assert_eq!(names(data.get_active()), ["Break", "Meeting"]);
        assert!(data.get_icons().row_count() >= 40);
        assert_eq!(data.get_colours().row_count(), 20);
        let meeting = data.get_active().row_data(1).expect("a Meeting row");
        assert!(meeting.locked_note.starts_with("Face 2 is locked"), "{}", meeting.locked_note);

        tab.save("Email");
        assert_eq!(names(data.get_active()), ["Break", "Email", "Meeting"]);
        let email = data.get_active().row_data(1).expect("an Email row");
        assert_eq!(email.locked_note, "");
        assert!(!email.has_icon && !email.has_colour);
        let id = i64::from(email.id);

        tab.rename_committed(id, "Mail");
        assert_eq!(notice.title(), "Rename this category?");
        notice.choose(1);
        assert_eq!(names(data.get_active()), ["Break", "Mail", "Meeting"]);

        tab.rename_committed(id, "meeting");
        assert_eq!(notice.title(), "That name is already in use");
        notice.choose(0);

        tab.colour_picked(id, 1);
        tab.icon_picked(id, 1);
        let mail = data.get_active().row_data(1).expect("a Mail row");
        assert!(mail.has_colour && mail.has_icon);
        assert_eq!(mail.colour_name, "Red");
        tab.colour_picked(id, 1);
        assert!(!data.get_active().row_data(1).expect("a Mail row").has_colour);

        tab.limit_edited(id, 2000);
        let connection = database::connect(&path).expect("should connect");
        assert_eq!(
            category::by_id(&connection, id).expect("should read").expect("exists").daily_limit_minutes,
            1440
        );

        let before = changes.get();
        tab.active_toggled(id, false);
        assert!(changes.get() > before);
        assert_eq!(names(data.get_active()), ["Break", "Meeting"]);
        let retired = data.get_inactive().row_data(0).expect("a retired row");
        assert_eq!((retired.name.as_str(), retired.last_used.as_str()), ("Mail", "Never"));

        tab.save("MAIL");
        assert_eq!(
            notice.title(),
            "The category \u{201c}Mail\u{201d} already exists as a deactivated category"
        );
        notice.choose(1);
        assert_eq!(names(data.get_active()), ["Break", "MAIL", "Meeting"]);
        tab.retired_toggled(id, true);
        assert_eq!(notice.title(), "That name is already in use");
        notice.choose(0);
        assert_eq!(data.get_inactive().row_count(), 1);

        std::fs::remove_file(&path).expect("the test database should be removable");
    }

    #[test]
    fn the_locked_note_names_each_face() {
        assert_eq!(locked_note(&[], "Meeting"), "");
        assert!(locked_note(&[2], "Meeting").starts_with("Face 2 is locked"));
        assert!(locked_note(&[2, 8], "Meeting").starts_with("Faces 2, 8 are locked"));
    }
}
