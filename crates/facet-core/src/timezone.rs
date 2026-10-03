//! The `timezone` table's ids, and the machine's zone as one of them.
//!
//! Local times are stored without an offset, so the zone they were taken in is stored beside them as a `timezone_id`.

use rusqlite::{Connection, OptionalExtension, params};

use crate::debug_log::{Record, Tag};
use crate::port::Zone;

/// The seeded `Unknown` row, which every `timezone_id` column defaults to.
pub const UNKNOWN: i64 = 0;

/// The id for the zone named `name`. A legacy name such as `Cuba` answers with the zone that replaced it. A name
/// the seed does not carry is added to `timezone` and answers with the new row's id.
pub fn id_for(connection: &Connection, name: &str) -> Result<i64, rusqlite::Error> {
    let seeded = connection
        .query_row("SELECT timezone_id FROM timezone_lookup WHERE timezone_name = ?1", params![name], |row| {
            row.get(0)
        })
        .optional()?;
    if let Some(id) = seeded {
        return Ok(id);
    }
    connection.execute(
        "INSERT INTO timezone (timezone_name) SELECT ?1 \
         WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_name = ?1)",
        params![name],
    )?;
    connection.query_row("SELECT timezone_id FROM timezone WHERE timezone_name = ?1", params![name], |row| {
        row.get(0)
    })
}

/// The id for the zone the machine is in now, asked for at the moment it is wanted.
///
/// [`UNKNOWN`] when the machine cannot name its zone or the name cannot be filed, and the trace says which, so the
/// row is still written.
pub fn current_id(connection: &Connection, zone: &dyn Zone, log: &impl Record) -> i64 {
    let name = match zone.name() {
        Ok(name) => name,
        Err(error) => {
            log.record_failure(Tag::Database, || {
                format!("The time zone of this machine could not be named, so the row is filed under Unknown: {error}")
            });
            return UNKNOWN;
        }
    };
    match id_for(connection, &name) {
        Ok(id) => id,
        Err(error) => {
            log.record_failure(Tag::Database, || {
                format!("The time zone {name} could not be filed, so the row is filed under Unknown: {error}")
            });
            UNKNOWN
        }
    }
}

/// A zone that always answers the same name, for tests and renders.
pub struct FixedZone(pub &'static str);

impl Zone for FixedZone {
    fn name(&self) -> Result<String, String> {
        Ok(self.0.to_string())
    }
}

/// A zone that cannot be named, for a trace that is never written.
pub struct UnnamedZone;

impl Zone for UnnamedZone {
    fn name(&self) -> Result<String, String> {
        Err("no zone was given".to_string())
    }
}

/// The zone the core's own tests run in.
pub const SYDNEY: FixedZone = FixedZone("Australia/Sydney");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::seeded;

    const NO_LOG: Option<crate::debug_log::DebugLog> = None;

    fn name_of(connection: &Connection, id: i64) -> String {
        connection
            .query_row("SELECT timezone_name FROM timezone WHERE timezone_id = ?1", params![id], |row| {
                row.get(0)
            })
            .expect("the id should name a zone")
    }

    #[test]
    fn a_seeded_zone_answers_with_its_own_id() {
        let connection = seeded();
        let id = id_for(&connection, "Australia/Sydney").expect("should look up");
        assert_ne!(id, UNKNOWN);
        assert_eq!(name_of(&connection, id), "Australia/Sydney");
    }

    #[test]
    fn a_legacy_name_answers_with_the_zone_that_replaced_it() {
        let connection = seeded();
        assert_eq!(
            id_for(&connection, "Asia/Calcutta").expect("should look up"),
            id_for(&connection, "Asia/Kolkata").expect("should look up")
        );
    }

    #[test]
    fn a_zone_the_seed_lacks_is_added_once_above_the_seeded_block() {
        let connection = seeded();
        let first = id_for(&connection, "Mars/Olympus_Mons").expect("should file");
        assert!(first > 447, "{first} should be above the seeded zones");
        assert_eq!(id_for(&connection, "Mars/Olympus_Mons").expect("should look up"), first);
        assert_eq!(name_of(&connection, first), "Mars/Olympus_Mons");
    }

    #[test]
    fn the_machines_zone_is_filed_when_it_can_be_named() {
        let connection = seeded();
        let id = current_id(&connection, &FixedZone("America/Havana"), &NO_LOG);
        assert_eq!(name_of(&connection, id), "America/Havana");
    }

    #[test]
    fn a_zone_that_cannot_be_named_is_filed_as_unknown() {
        let connection = seeded();
        assert_eq!(current_id(&connection, &UnnamedZone, &NO_LOG), UNKNOWN);
    }
}
