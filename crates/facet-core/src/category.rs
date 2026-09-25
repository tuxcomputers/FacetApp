//! The `category` table: reading categories, creating them and retiring them.
//!
//! Every function reads or writes the table when called; nothing is cached.

use std::cmp::Ordering;

use rusqlite::{Connection, OptionalExtension, params};

/// The longest name a category may have, in characters.
pub const MAXIMUM_NAME_LENGTH: usize = 35;

/// One category, joined with its icon and colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub id: i64,
    pub name: String,
    /// The icon's name, such as `ic_meeting`. `None` for the icon row called `None`.
    pub icon_name: Option<String>,
    /// `#rrggbb`. `None` for the colour row called `None`.
    pub colour_hex: Option<String>,
    /// Whether the icon is drawn white rather than black on this colour.
    pub uses_white_lines: bool,
    /// Minutes per day, where 0 is no limit.
    pub daily_limit_minutes: i64,
    pub is_category_active: bool,
}

const SELECT: &str = "SELECT c.category_id, c.category_name, i.icon_name, l.device_hex, l.white_lines, \
                             c.daily_limit, c.active \
                        FROM category c \
                        JOIN icon i ON i.icon_id = c.icon_id \
                        JOIN colour l ON l.colour_id = c.colour_id";

fn from_row(row: &rusqlite::Row<'_>) -> Result<Category, rusqlite::Error> {
    let icon_name: String = row.get(2)?;
    Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        icon_name: (icon_name != "None").then_some(icon_name),
        colour_hex: row.get(3)?,
        uses_white_lines: row.get(4)?,
        daily_limit_minutes: row.get(5)?,
        is_category_active: row.get(6)?,
    })
}

/// Every active category except Unassigned, in display order (see [`display_order`]).
pub fn active(connection: &Connection) -> Result<Vec<Category>, rusqlite::Error> {
    let mut statement = connection.prepare(&format!("{SELECT} WHERE c.active = 1 AND c.category_id >= 1"))?;
    let mut categories = statement.query_map([], from_row)?.collect::<Result<Vec<_>, _>>()?;
    categories.sort_by(display_order);
    Ok(categories)
}

/// One category by id, active or retired. `None` when there is no such row.
pub fn by_id(connection: &Connection, id: i64) -> Result<Option<Category>, rusqlite::Error> {
    connection.query_row(&format!("{SELECT} WHERE c.category_id = ?1"), params![id], from_row).optional()
}

/// Every category named `name`, compared case-insensitively, the active one first and then by id.
pub fn matching(connection: &Connection, name: &str) -> Result<Vec<Category>, rusqlite::Error> {
    let mut statement = connection.prepare(&format!(
        "{SELECT} WHERE c.category_name = ?1 COLLATE NOCASE ORDER BY c.active DESC, c.category_id ASC"
    ))?;
    statement.query_map(params![name], from_row)?.collect()
}

/// Inserts an active category with no icon and no colour, and returns its id.
///
/// Fails when an active category already holds the name, by the table's unique index.
pub fn insert(connection: &Connection, name: &str) -> Result<i64, rusqlite::Error> {
    connection.execute(
        "INSERT INTO category (category_name, icon_id, colour_id) VALUES (?1, 0, 0)",
        params![name],
    )?;
    Ok(connection.last_insert_rowid())
}

/// Retires or reinstates a category. Returns whether a row changed.
///
/// Unassigned (id 0) is never changed. Reinstating fails when an active category already holds the name.
pub fn set_active(connection: &Connection, id: i64, active: bool) -> Result<bool, rusqlite::Error> {
    let changed = connection.execute(
        "UPDATE category SET active = ?1 WHERE category_id = ?2 AND category_id >= 1",
        params![active, id],
    )?;
    Ok(changed > 0)
}

/// The order categories are listed in: names that are whole numbers first, by value; then the rest by
/// natural order (case-insensitive, runs of digits compared as numbers); ties broken by id.
pub fn display_order(a: &Category, b: &Category) -> Ordering {
    let by_name = match (a.name.parse::<i64>(), b.name.parse::<i64>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        (Ok(_), Err(_)) => Ordering::Less,
        (Err(_), Ok(_)) => Ordering::Greater,
        (Err(_), Err(_)) => natural(&a.name, &b.name),
    };
    by_name.then(a.id.cmp(&b.id))
}

/// Case-insensitive comparison in which each run of digits compares by numeric value.
fn natural(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let run = |chars: &mut std::iter::Peekable<std::str::Chars<'_>>| {
                    let mut digits = String::new();
                    while let Some(&c) = chars.peek().filter(|c| c.is_ascii_digit()) {
                        digits.push(c);
                        chars.next();
                    }
                    digits
                };
                let (x, y) = (run(&mut a), run(&mut b));
                let (x_trimmed, y_trimmed) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
                let ordering = x_trimmed.len().cmp(&y_trimmed.len()).then_with(|| x_trimmed.cmp(y_trimmed));
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            (Some(x), Some(y)) => {
                let ordering = x.to_lowercase().cmp(y.to_lowercase());
                if ordering != Ordering::Equal {
                    return ordering;
                }
                a.next();
                b.next();
            }
        }
    }
}

/// A typed name as it would be stored: runs of whitespace collapsed to one space, ends trimmed, and cut to
/// [`MAXIMUM_NAME_LENGTH`] characters.
pub fn normalise(raw: &str) -> String {
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= MAXIMUM_NAME_LENGTH {
        return collapsed;
    }
    collapsed.chars().take(MAXIMUM_NAME_LENGTH).collect::<String>().trim_end().to_string()
}

/// What saving a typed name should do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateDecision {
    /// The name is empty once normalised.
    Ignore,
    /// Nothing holds the name, so insert it under this normalised spelling.
    Insert(String),
    /// An active category already holds the name.
    AlreadyActive(Category),
    /// Only retired categories hold the name. Every one of them, by id.
    RetiredNamesakes(Vec<Category>),
}

/// Decides what saving `raw` should do, by reading which categories already hold the name.
pub fn decide_create(connection: &Connection, raw: &str) -> Result<CreateDecision, rusqlite::Error> {
    let name = normalise(raw);
    if name.is_empty() {
        return Ok(CreateDecision::Ignore);
    }
    let matches = matching(connection, &name)?;
    Ok(match matches.first() {
        None => CreateDecision::Insert(name),
        Some(first) if first.is_category_active => CreateDecision::AlreadyActive(first.clone()),
        Some(_) => CreateDecision::RetiredNamesakes(matches),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::seeded;

    fn names(categories: &[Category]) -> Vec<&str> {
        categories.iter().map(|c| c.name.as_str()).collect()
    }

    #[test]
    fn the_seeded_active_list_is_break_and_meeting_without_unassigned() {
        let connection = seeded();
        let list = active(&connection).expect("the list should read");
        assert_eq!(names(&list), ["Break", "Meeting"]);
        let meeting = &list[1];
        assert_eq!(meeting.icon_name.as_deref(), Some("ic_meeting"));
        assert_eq!(meeting.colour_hex.as_deref(), Some("#00ffff"));
        assert!(meeting.is_category_active);
    }

    #[test]
    fn numbers_come_first_by_value_then_names_in_natural_order() {
        let connection = seeded();
        for name in ["Task 10", "task 9", "12", "3", "alpha", "Zulu"] {
            insert(&connection, name).expect("the insert should run");
        }
        let list = active(&connection).expect("the list should read");
        assert_eq!(names(&list), ["3", "12", "alpha", "Break", "Meeting", "task 9", "Task 10", "Zulu"]);
    }

    #[test]
    fn a_retired_category_leaves_the_list_and_can_come_back() {
        let connection = seeded();
        let id = insert(&connection, "Email").expect("the insert should run");
        assert!(set_active(&connection, id, false).expect("the update should run"));
        assert!(!names(&active(&connection).expect("the list should read")).contains(&"Email"));
        assert!(set_active(&connection, id, true).expect("the update should run"));
        assert!(names(&active(&connection).expect("the list should read")).contains(&"Email"));
    }

    #[test]
    fn unassigned_cannot_be_retired() {
        let connection = seeded();
        assert!(!set_active(&connection, 0, false).expect("the update should run"));
    }

    #[test]
    fn a_second_active_namesake_is_refused_by_the_table() {
        let connection = seeded();
        assert!(insert(&connection, "meeting").is_err());
    }

    #[test]
    fn normalising_collapses_whitespace_and_cuts_to_the_maximum() {
        assert_eq!(normalise("  Deep   work \t here "), "Deep work here");
        let long = "a".repeat(40);
        assert_eq!(normalise(&long).chars().count(), MAXIMUM_NAME_LENGTH);
        assert_eq!(normalise(&format!("{} b", "a".repeat(34))), "a".repeat(34));
    }

    #[test]
    fn saving_decides_from_which_categories_hold_the_name() {
        let connection = seeded();
        assert_eq!(decide_create(&connection, "   ").expect("should decide"), CreateDecision::Ignore);
        assert_eq!(
            decide_create(&connection, " Deep  work ").expect("should decide"),
            CreateDecision::Insert("Deep work".to_string())
        );
        match decide_create(&connection, "MEETING").expect("should decide") {
            CreateDecision::AlreadyActive(existing) => assert_eq!(existing.name, "Meeting"),
            other => panic!("expected AlreadyActive, got {other:?}"),
        }

        let first = insert(&connection, "Email").expect("the insert should run");
        set_active(&connection, first, false).expect("the update should run");
        let second = insert(&connection, "email").expect("the insert should run");
        set_active(&connection, second, false).expect("the update should run");
        match decide_create(&connection, "Email").expect("should decide") {
            CreateDecision::RetiredNamesakes(existing) => {
                assert_eq!(existing.iter().map(|c| c.id).collect::<Vec<_>>(), [first, second]);
            }
            other => panic!("expected RetiredNamesakes, got {other:?}"),
        }
    }
}
