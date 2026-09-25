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
    /// 0 for the icon row called `None`.
    pub icon_id: i64,
    /// The icon's name, such as `ic_meeting`. `None` for the icon row called `None`.
    pub icon_name: Option<String>,
    /// 0 for the colour row called `None`.
    pub colour_id: i64,
    /// The colour's name, such as `Cyan`. `None` for the colour row called `None`.
    pub colour_name: Option<String>,
    /// `#rrggbb`. `None` for the colour row called `None`.
    pub colour_hex: Option<String>,
    /// Whether the icon is drawn white rather than black on this colour.
    pub uses_white_lines: bool,
    /// Minutes per day, where 0 is no limit.
    pub daily_limit_minutes: i64,
    pub is_category_active: bool,
}

const SELECT: &str = "SELECT c.category_id, c.category_name, i.icon_name, l.device_hex, l.white_lines, \
                             c.daily_limit, c.active, c.icon_id, c.colour_id, l.colour_name \
                        FROM category c \
                        JOIN icon i ON i.icon_id = c.icon_id \
                        JOIN colour l ON l.colour_id = c.colour_id";

fn from_row(row: &rusqlite::Row<'_>) -> Result<Category, rusqlite::Error> {
    let icon_name: String = row.get(2)?;
    let colour_name: String = row.get(9)?;
    Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        icon_id: row.get(7)?,
        icon_name: (icon_name != "None").then_some(icon_name),
        colour_id: row.get(8)?,
        colour_name: (colour_name != "None").then_some(colour_name),
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

/// Every retired category except Unassigned, in display order (see [`display_order`]).
pub fn inactive(connection: &Connection) -> Result<Vec<Category>, rusqlite::Error> {
    let mut statement = connection.prepare(&format!("{SELECT} WHERE c.active = 0 AND c.category_id >= 1"))?;
    let mut categories = statement.query_map([], from_row)?.collect::<Result<Vec<_>, _>>()?;
    categories.sort_by(display_order);
    Ok(categories)
}

/// The highest daily limit a category may have, in minutes.
pub const MAXIMUM_DAILY_LIMIT_MINUTES: i64 = 1440;

/// `minutes` held to 0 through [`MAXIMUM_DAILY_LIMIT_MINUTES`].
pub fn clamp_daily_limit(minutes: i64) -> i64 {
    minutes.clamp(0, MAXIMUM_DAILY_LIMIT_MINUTES)
}

/// The icon or colour id to store when `picked` is chosen for a category holding `current`: picking what it
/// already holds clears it to 0 (None).
pub fn toggled_choice(current: i64, picked: i64) -> i64 {
    if picked == current { 0 } else { picked }
}

fn update(
    connection: &Connection,
    sql: &str,
    value: &dyn rusqlite::ToSql,
    id: i64,
) -> Result<bool, rusqlite::Error> {
    Ok(connection.execute(sql, params![value, id])? > 0)
}

/// Renames a category. Returns whether a row changed. Unassigned is never changed; an active category taking
/// a name another active category holds is refused by the table's unique index, as an error.
pub fn set_name(connection: &Connection, id: i64, name: &str) -> Result<bool, rusqlite::Error> {
    update(
        connection,
        "UPDATE category SET category_name = ?1 WHERE category_id = ?2 AND category_id >= 1",
        &name,
        id,
    )
}

/// Sets a category's icon (0 for None). Returns whether a row changed.
pub fn set_icon(connection: &Connection, id: i64, icon_id: i64) -> Result<bool, rusqlite::Error> {
    update(
        connection,
        "UPDATE category SET icon_id = ?1 WHERE category_id = ?2 AND category_id >= 1",
        &icon_id,
        id,
    )
}

/// Sets a category's colour (0 for None). Returns whether a row changed.
pub fn set_colour(connection: &Connection, id: i64, colour_id: i64) -> Result<bool, rusqlite::Error> {
    update(
        connection,
        "UPDATE category SET colour_id = ?1 WHERE category_id = ?2 AND category_id >= 1",
        &colour_id,
        id,
    )
}

/// Sets a category's daily limit, clamped by [`clamp_daily_limit`], and returns the value written, or `None`
/// when no row changed.
pub fn set_daily_limit(
    connection: &Connection,
    id: i64,
    minutes: i64,
) -> Result<Option<i64>, rusqlite::Error> {
    let allowed = clamp_daily_limit(minutes);
    let changed = update(
        connection,
        "UPDATE category SET daily_limit = ?1 WHERE category_id = ?2 AND category_id >= 1",
        &allowed,
        id,
    )?;
    Ok(changed.then_some(allowed))
}

/// Retires a category and puts Unassigned on every unlocked face holding it. Returns the faces cleared, or
/// `None` when the category was not changed (Unassigned, or no such row). Both writes are one transaction.
pub fn retire(connection: &Connection, id: i64) -> Result<Option<Vec<i64>>, rusqlite::Error> {
    let transaction = connection.unchecked_transaction()?;
    if !set_active(&transaction, id, false)? {
        return Ok(None);
    }
    let cleared = crate::face::clear_category(&transaction, id)?;
    transaction.commit()?;
    Ok(Some(cleared))
}

/// Whether a retired category may be reinstated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReinstateDecision {
    Allowed,
    /// An active category holds the same name, compared case-insensitively.
    Refused {
        active_namesake: Category,
    },
}

/// Decides whether the retired category `id` may be reinstated, by reading whether an active category holds
/// its name.
pub fn decide_reinstate(
    connection: &Connection,
    id: i64,
) -> Result<Option<ReinstateDecision>, rusqlite::Error> {
    let Some(category) = by_id(connection, id)? else { return Ok(None) };
    let namesake =
        matching(connection, &category.name)?.into_iter().find(|c| c.id != id && c.is_category_active);
    Ok(Some(match namesake {
        Some(active_namesake) => ReinstateDecision::Refused { active_namesake },
        None => ReinstateDecision::Allowed,
    }))
}

/// What renaming a category to a typed name should do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameDecision {
    /// Empty once normalised, or exactly the current name.
    Ignore,
    /// No other category holds the name: confirm, then rename.
    Confirm { name: String },
    /// Only retired categories hold the name: confirm, and say how many.
    ConfirmAgainstRetired { name: String, retired: Vec<Category> },
    /// The category is retired and an active one holds the name: allowed, but it then cannot be reinstated.
    ConfirmAgainstActive { name: String, active_namesake: Category },
    /// The category is active and another active one holds the name: the table would refuse it.
    Refuse { active_namesake: Category },
}

/// Decides what renaming `current` to `raw` should do. Other categories are matched case-insensitively and
/// `current` itself is left out, so changing only the capitals is an ordinary rename.
pub fn decide_rename(
    connection: &Connection,
    current: &Category,
    raw: &str,
) -> Result<RenameDecision, rusqlite::Error> {
    let name = normalise(raw);
    if name.is_empty() || name == current.name {
        return Ok(RenameDecision::Ignore);
    }
    let others: Vec<Category> =
        matching(connection, &name)?.into_iter().filter(|c| c.id != current.id).collect();
    Ok(match others.first() {
        None => RenameDecision::Confirm { name },
        Some(first) if first.is_category_active => {
            if current.is_category_active {
                RenameDecision::Refuse { active_namesake: first.clone() }
            } else {
                RenameDecision::ConfirmAgainstActive { name, active_namesake: first.clone() }
            }
        }
        Some(_) => RenameDecision::ConfirmAgainstRetired { name, retired: others },
    })
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
    fn a_retired_category_is_listed_inactive() {
        let connection = seeded();
        let id = insert(&connection, "Email").expect("should insert");
        assert!(inactive(&connection).expect("should read").is_empty());
        set_active(&connection, id, false).expect("should retire");
        assert_eq!(names(&inactive(&connection).expect("should read")), ["Email"]);
    }

    #[test]
    fn the_field_writes_land_and_the_limit_is_clamped() {
        let connection = seeded();
        let id = insert(&connection, "Email").expect("should insert");
        assert!(set_name(&connection, id, "Mail").expect("should rename"));
        assert!(set_icon(&connection, id, 3).expect("should set icon"));
        assert!(set_colour(&connection, id, 1).expect("should set colour"));
        assert_eq!(set_daily_limit(&connection, id, 2000).expect("should set limit"), Some(1440));
        assert_eq!(set_daily_limit(&connection, id, -5).expect("should set limit"), Some(0));
        assert_eq!(set_daily_limit(&connection, id, 45).expect("should set limit"), Some(45));
        let row = by_id(&connection, id).expect("should read").expect("should exist");
        assert_eq!(
            (row.name.as_str(), row.colour_hex.as_deref(), row.daily_limit_minutes),
            ("Mail", Some("#ff0000"), 45)
        );
        assert!(row.icon_name.is_some());
        assert!(!set_name(&connection, 0, "Nobody").expect("the update should run"));
        assert_eq!(set_daily_limit(&connection, 0, 5).expect("the update should run"), None);
    }

    #[test]
    fn picking_the_current_choice_again_clears_it() {
        assert_eq!(toggled_choice(3, 5), 5);
        assert_eq!(toggled_choice(5, 5), 0);
    }

    #[test]
    fn retiring_clears_unlocked_faces_and_a_namesake_blocks_reinstating() {
        let connection = seeded();
        let id = insert(&connection, "Email").expect("should insert");
        crate::face::assign(&connection, id, 4).expect("should assign");
        assert_eq!(retire(&connection, id).expect("should retire"), Some(vec![4]));
        assert_eq!(retire(&connection, 0).expect("should run"), None);
        assert_eq!(
            decide_reinstate(&connection, id).expect("should decide"),
            Some(ReinstateDecision::Allowed)
        );
        let namesake = insert(&connection, "EMAIL").expect("should insert");
        match decide_reinstate(&connection, id).expect("should decide") {
            Some(ReinstateDecision::Refused { active_namesake }) => assert_eq!(active_namesake.id, namesake),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn renaming_decides_from_who_else_holds_the_name() {
        let connection = seeded();
        let email = insert(&connection, "Email").expect("should insert");
        let current = by_id(&connection, email).expect("should read").expect("should exist");
        assert_eq!(
            decide_rename(&connection, &current, " Email ").expect("should decide"),
            RenameDecision::Ignore
        );
        assert_eq!(
            decide_rename(&connection, &current, "EMAIL").expect("should decide"),
            RenameDecision::Confirm { name: "EMAIL".to_string() }
        );
        assert!(matches!(
            decide_rename(&connection, &current, "meeting").expect("should decide"),
            RenameDecision::Refuse { .. }
        ));
        let old = insert(&connection, "Old").expect("should insert");
        set_active(&connection, old, false).expect("should retire");
        assert!(matches!(
            decide_rename(&connection, &current, "old").expect("should decide"),
            RenameDecision::ConfirmAgainstRetired { retired, .. } if retired.len() == 1
        ));
        let retired = by_id(&connection, old).expect("should read").expect("should exist");
        assert!(matches!(
            decide_rename(&connection, &retired, "Meeting").expect("should decide"),
            RenameDecision::ConfirmAgainstActive { .. }
        ));
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
