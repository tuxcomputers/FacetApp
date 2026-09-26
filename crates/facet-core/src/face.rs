//! The `face` table: which category each face holds, and whether it is locked.
//!
//! Faces 1 to 12 are the cube's. Faces 13 and 14 are the app's own, used for timing without a cube.
//! Every function reads or writes the table when called; nothing is cached.

use rusqlite::{Connection, OptionalExtension, params};

/// The highest face a cube reports. Every face above it belongs to the app.
pub const HIGHEST_DEVICE_FACE: i64 = 12;

/// The app's own faces, in rotation order. `device_event.device_face` is checked against 1 to 14 by the
/// DDL, so this list cannot grow without changing that check.
pub const APP_FACES: [i64; 2] = [13, 14];

/// Whether `face` is one of the app's own, which is also whether a segment on it was measured by this app
/// rather than reported by a cube.
pub fn is_app_face(face: i64) -> bool {
    face > HIGHEST_DEVICE_FACE
}

/// The app face a new segment goes on, given the face the previous app segment used.
///
/// `None`, or a face that is not an app face, starts the rotation at its first face.
pub fn next_app_face(after: Option<i64>) -> i64 {
    match after.and_then(|face| APP_FACES.iter().position(|&f| f == face)) {
        Some(index) => APP_FACES[(index + 1) % APP_FACES.len()],
        None => APP_FACES[0],
    }
}

/// The category `face` holds. `None` when the face holds the Unassigned row (id 0) or does not exist.
pub fn category_id(connection: &Connection, face: i64) -> Result<Option<i64>, rusqlite::Error> {
    let id = connection
        .query_row("SELECT category_id FROM face WHERE face_id = ?1", params![face], |row| {
            row.get::<_, i64>(0)
        })
        .optional()?;
    Ok(id.filter(|&id| id != 0))
}

/// Whether `face` refuses reassignment. `None` when there is no such face.
pub fn is_face_locked(connection: &Connection, face: i64) -> Result<Option<bool>, rusqlite::Error> {
    connection
        .query_row("SELECT locked FROM face WHERE face_id = ?1", params![face], |row| row.get::<_, bool>(0))
        .optional()
}

/// Puts `category_id` on `face`. Returns whether the row changed.
///
/// A locked face is refused by the statement itself and returns `false`, as does a face that does not exist.
pub fn assign(connection: &Connection, category_id: i64, face: i64) -> Result<bool, rusqlite::Error> {
    let changed = connection.execute(
        "UPDATE face SET category_id = ?1 WHERE face_id = ?2 AND locked = 0",
        params![category_id, face],
    )?;
    Ok(changed > 0)
}

/// Locks or unlocks `face`. Returns whether a row was there to change.
pub fn set_locked(connection: &Connection, face: i64, locked: bool) -> Result<bool, rusqlite::Error> {
    let changed =
        connection.execute("UPDATE face SET locked = ?1 WHERE face_id = ?2", params![locked, face])?;
    Ok(changed > 0)
}

/// Puts Unassigned on every unlocked face holding `category_id`, and returns those faces in order.
///
/// Locked faces keep the category.
pub fn clear_category(connection: &Connection, category_id: i64) -> Result<Vec<i64>, rusqlite::Error> {
    let mut statement = connection
        .prepare("SELECT face_id FROM face WHERE category_id = ?1 AND locked = 0 ORDER BY face_id")?;
    let faces: Vec<i64> =
        statement.query_map(params![category_id], |row| row.get(0))?.collect::<Result<_, _>>()?;
    connection.execute(
        "UPDATE face SET category_id = 0 WHERE category_id = ?1 AND locked = 0",
        params![category_id],
    )?;
    Ok(faces)
}

/// The locked faces holding `category_id`, in order.
pub fn locked_faces_holding(connection: &Connection, category_id: i64) -> Result<Vec<i64>, rusqlite::Error> {
    let mut statement = connection
        .prepare("SELECT face_id FROM face WHERE category_id = ?1 AND locked = 1 ORDER BY face_id")?;
    statement.query_map(params![category_id], |row| row.get(0))?.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::seeded;

    #[test]
    fn the_rotation_alternates_between_the_app_faces_and_starts_at_the_first() {
        assert_eq!(next_app_face(None), 13);
        assert_eq!(next_app_face(Some(13)), 14);
        assert_eq!(next_app_face(Some(14)), 13);
        assert_eq!(next_app_face(Some(5)), 13);
    }

    #[test]
    fn only_faces_above_twelve_are_app_faces() {
        assert!(!is_app_face(12));
        assert!(is_app_face(13));
        assert!(is_app_face(14));
    }

    #[test]
    fn the_seeded_faces_hold_meeting_on_two_and_break_on_eight_and_nothing_else() {
        let connection = seeded();
        let meeting: i64 = connection
            .query_row("SELECT category_id FROM category WHERE category_name = 'Meeting'", [], |r| r.get(0))
            .expect("Meeting should be seeded");
        assert_eq!(category_id(&connection, 2).expect("face 2 should read"), Some(meeting));
        assert_eq!(category_id(&connection, 13).expect("face 13 should read"), None);
        assert_eq!(category_id(&connection, 99).expect("a missing face should read"), None);
        assert_eq!(is_face_locked(&connection, 2).expect("face 2 should read"), Some(true));
        assert_eq!(is_face_locked(&connection, 13).expect("face 13 should read"), Some(false));
        assert_eq!(is_face_locked(&connection, 99).expect("a missing face should read"), None);
    }

    #[test]
    fn a_locked_face_refuses_a_category_and_keeps_what_it_had() {
        let connection = seeded();
        let before = category_id(&connection, 2).expect("face 2 should read");
        assert!(!assign(&connection, 0, 2).expect("the update should run"));
        assert_eq!(category_id(&connection, 2).expect("face 2 should read"), before);

        assert!(set_locked(&connection, 2, false).expect("the lock should change"));
        assert!(assign(&connection, 0, 2).expect("the update should run"));
        assert_eq!(category_id(&connection, 2).expect("face 2 should read"), None);
    }

    #[test]
    fn clearing_a_category_leaves_locked_faces_holding_it() {
        let connection = seeded();
        let meeting = category_id(&connection, 2).expect("face 2 should read").expect("face 2 holds Meeting");
        assign(&connection, meeting, 5).expect("should assign");
        assign(&connection, meeting, 13).expect("should assign");
        assert_eq!(locked_faces_holding(&connection, meeting).expect("should read"), vec![2]);
        assert_eq!(clear_category(&connection, meeting).expect("should clear"), vec![5, 13]);
        assert_eq!(category_id(&connection, 5).expect("should read"), None);
        assert_eq!(category_id(&connection, 2).expect("should read"), Some(meeting));
    }

    #[test]
    fn an_app_face_takes_a_category() {
        let connection = seeded();
        let meeting = category_id(&connection, 2).expect("face 2 should read").expect("face 2 holds Meeting");
        assert!(assign(&connection, meeting, 13).expect("the update should run"));
        assert_eq!(category_id(&connection, 13).expect("face 13 should read"), Some(meeting));
    }
}
