//! The reference tables `icon` and `colour`: seeded by the DDL and never written by the app.
//!
//! The rows called `None` (id 0 in each) are left out of both lists: they are what a category holds when it
//! has no icon or no colour, not a choice.

use rusqlite::Connection;

/// One icon a category can take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
    pub id: i64,
    /// The file name without its extension, such as `ic_admin`.
    pub name: String,
}

impl Icon {
    /// The name shown to a person: `ic_` removed, the rest split on `_`, each word capitalised.
    /// `ic_you_tube` is `You Tube`.
    pub fn display_name(&self) -> String {
        self.name
            .strip_prefix("ic_")
            .unwrap_or(&self.name)
            .split('_')
            .filter(|word| !word.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// One colour a category can take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Colour {
    pub id: i64,
    pub name: String,
    /// `#rrggbb`.
    pub hex: String,
    /// Whether an icon on this colour is drawn white rather than black.
    pub uses_white_lines: bool,
}

/// Every icon except `None`, by id.
pub fn icons(connection: &Connection) -> Result<Vec<Icon>, rusqlite::Error> {
    let mut statement =
        connection.prepare("SELECT icon_id, icon_name FROM icon WHERE icon_id >= 1 ORDER BY icon_id")?;
    statement.query_map([], |row| Ok(Icon { id: row.get(0)?, name: row.get(1)? }))?.collect()
}

/// Every colour except `None`, by id, which is palette order. A row whose hex is missing is left out.
pub fn colours(connection: &Connection) -> Result<Vec<Colour>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT colour_id, colour_name, device_hex, white_lines FROM colour \
         WHERE colour_id >= 1 AND device_hex IS NOT NULL ORDER BY colour_id",
    )?;
    statement
        .query_map([], |row| {
            Ok(Colour { id: row.get(0)?, name: row.get(1)?, hex: row.get(2)?, uses_white_lines: row.get(3)? })
        })?
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::seeded;

    #[test]
    fn the_seeded_icons_and_colours_leave_out_none() {
        let connection = seeded();
        let icons = icons(&connection).expect("icons should read");
        assert!(icons.len() >= 40);
        assert_eq!(icons[0], Icon { id: 1, name: "ic_admin".to_string() });
        assert!(icons.iter().all(|icon| icon.id >= 1 && icon.name != "None"));

        let colours = colours(&connection).expect("colours should read");
        assert_eq!(colours.len(), 20);
        assert_eq!(colours[0].name, "Red");
        assert_eq!(colours[0].hex, "#ff0000");
        assert!(colours.iter().all(|colour| colour.id >= 1));
    }

    #[test]
    fn display_names_drop_the_prefix_and_capitalise_each_word() {
        let icon = |name: &str| Icon { id: 1, name: name.to_string() };
        assert_eq!(icon("ic_admin").display_name(), "Admin");
        assert_eq!(icon("ic_you_tube").display_name(), "You Tube");
        assert_eq!(icon("plain").display_name(), "Plain");
    }
}
