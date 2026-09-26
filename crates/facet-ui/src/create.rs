//! Saving a typed name as a new category, which the Faces and Categories tabs both offer.
//!
//! A free name is inserted. A name an active category holds gets a statement and nothing is written. A name
//! only retired categories hold gets a question: Reactivate (offered only when there is exactly one of them),
//! Create new one, or Cancel. The caller hears about every write through `written`, with the id to start
//! timing or `None` when the write was refused.

use std::path::PathBuf;
use std::rc::Rc;

use facet_core::category::{self, CreateDecision};
use facet_core::database;
use facet_core::debug_log::{DebugLog, Record, Tag, plain};
use rusqlite::Connection;

use crate::notice::Notice;

/// Saves category names against one database, asking through one notice.
pub struct Creator {
    database: PathBuf,
    log: Rc<Option<DebugLog>>,
    notice: Rc<Notice>,
}

impl Creator {
    pub fn new(database: PathBuf, log: Rc<Option<DebugLog>>, notice: Rc<Notice>) -> Rc<Creator> {
        Rc::new(Creator { database, log, notice })
    }

    /// Acts on `typed` as described in the module documentation. `written` runs after a write, now or once
    /// the question is answered; it does not run for an empty name, an active namesake or Cancel.
    pub fn save(self: &Rc<Self>, typed: &str, written: impl Fn(Option<i64>) + 'static) {
        let Some(connection) = self.connect() else { return };
        let Some(decision) = self.report(category::decide_create(&connection, typed)) else { return };
        match decision {
            CreateDecision::Ignore => {}
            CreateDecision::Insert(name) => {
                let created = self.report(category::insert(&connection, &name));
                self.log.record(Tag::Click, || {
                    format!("Button clicked: Save new category {} -> {created:?}", plain(&name))
                });
                written(created);
            }
            CreateDecision::AlreadyActive(existing) => {
                self.log.record(Tag::Click, || {
                    format!(
                        "Button clicked: Save new category {} -> already active as category_id {}",
                        plain(&existing.name),
                        existing.id
                    )
                });
                self.notice.tell(
                    "That category already exists",
                    &format!("\u{201c}{}\u{201d} is already in the Active list.", existing.name),
                );
            }
            CreateDecision::RetiredNamesakes(existing) => {
                let first = existing[0].clone();
                let count = existing.len();
                self.log.record(Tag::Click, || {
                    format!(
                        "Button clicked: Save new category {} -> asking, {count} retired under that name: {:?}",
                        plain(&first.name),
                        existing.iter().map(|c| c.id).collect::<Vec<_>>()
                    )
                });
                let choices: &[&str] = if count == 1 {
                    &["Reactivate", "Create new one", "Cancel"]
                } else {
                    &["Create new one", "Cancel"]
                };
                let message = if count == 1 {
                    "There is one category with the same name.".to_string()
                } else {
                    format!("There are {count} categories with the same name.")
                };
                let name = category::normalise(typed);
                let this = Rc::clone(self);
                let offered: Vec<String> = choices.iter().map(|c| c.to_string()).collect();
                self.notice.ask(
                    &format!(
                        "The category \u{201c}{}\u{201d} already exists as a deactivated category",
                        first.name
                    ),
                    &message,
                    choices,
                    move |index| {
                        this.answer(offered.get(index).map(String::as_str), &name, first.id, &written)
                    },
                );
            }
        }
    }

    fn answer(&self, choice: Option<&str>, name: &str, first_id: i64, written: &dyn Fn(Option<i64>)) {
        let Some(connection) = self.connect() else { return };
        match choice {
            Some("Reactivate") => {
                let reinstated =
                    self.report(category::set_active(&connection, first_id, true)).unwrap_or(false);
                self.log.record(Tag::Click, || {
                    format!(
                        "Button clicked: Reactivate {} -> category_id {first_id}{}",
                        plain(name),
                        if reinstated { "" } else { " REFUSED" }
                    )
                });
                written(reinstated.then_some(first_id));
            }
            Some("Create new one") => {
                let created = self.report(category::insert(&connection, name));
                self.log.record(Tag::Click, || {
                    format!(
                        "Button clicked: Create new one {} -> {}, leaving category_id {first_id} retired",
                        plain(name),
                        created.map_or("refused".to_string(), |id| format!("category_id {id}"))
                    )
                });
                written(created);
            }
            _ => {
                self.log.record(Tag::Click, || format!("Button clicked: Cancel, {} not created", plain(name)))
            }
        }
    }

    fn connect(&self) -> Option<Connection> {
        match database::connect(&self.database) {
            Ok(connection) => Some(connection),
            Err(error) => {
                self.log.record_failure(Tag::Database, || {
                    format!("Create: the database would not open: {error}")
                });
                None
            }
        }
    }

    fn report<T>(&self, result: Result<T, rusqlite::Error>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.log.record_failure(Tag::Database, || format!("Create: a database call failed: {error}"));
                None
            }
        }
    }
}
