//! Lighting a face: `0x11`, the face and its colour as three 16-bit channels. There is no read-back for it.
//!
//! A face's colour is its category's `colour.device_hex`. A face with no category, or a category with no colour,
//! is sent all zeros, which is the only way to say off.

use rusqlite::{Connection, OptionalExtension, params};

/// Each channel of `hex` (`#rrggbb`) scaled from 8 bits to 16, so `ff` is `ffff`. Off for `None` or a hex that will
/// not read.
pub fn rgb16(hex: Option<&str>) -> [u16; 3] {
    let Some(digits) = hex.and_then(|hex| hex.strip_prefix('#')).filter(|digits| digits.len() == 6) else {
        return [0; 3];
    };
    let channel =
        |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).map_or(0, |byte| u16::from(byte) * 257);
    [channel(0), channel(2), channel(4)]
}

/// `0x11` for `face`, 1 to 12, in `hex`.
pub fn command(face: u8, hex: Option<&str>) -> Vec<u8> {
    let mut bytes = vec![0x11, face];
    for channel in rgb16(hex) {
        bytes.extend_from_slice(&channel.to_be_bytes());
    }
    bytes
}

/// What `face` wears: its category's name and colour. The name is `None` for a face holding no category (id 0),
/// and the colour `None` when there is nothing to light.
pub fn of_face(
    connection: &Connection,
    face: i64,
) -> Result<(Option<String>, Option<String>), rusqlite::Error> {
    Ok(connection
        .query_row(
            "SELECT c.category_id, c.category_name, col.device_hex FROM face f \
             JOIN category c ON c.category_id = f.category_id \
             LEFT JOIN colour col ON col.colour_id = c.colour_id WHERE f.face_id = ?1",
            params![face],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?)),
        )
        .optional()?
        .map_or((None, None), |(id, name, hex)| if id == 0 { (None, None) } else { (Some(name), hex) }))
}

/// The cube faces, 1 to 12, that hold `category_id`.
pub fn faces_holding(connection: &Connection, category_id: i64) -> Result<Vec<i64>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT face_id FROM face WHERE category_id = ?1 AND face_id BETWEEN 1 AND 12 ORDER BY face_id",
    )?;
    statement.query_map(params![category_id], |row| row.get(0))?.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::seeded;

    #[test]
    fn a_colour_goes_as_three_sixteen_bit_channels() {
        assert_eq!(rgb16(Some("#ff0000")), [0xFFFF, 0, 0]);
        assert_eq!(rgb16(Some("#800000")), [0x8080, 0, 0]);
        assert_eq!(rgb16(None), [0, 0, 0]);
        assert_eq!(rgb16(Some("red")), [0, 0, 0]);
        assert_eq!(command(8, Some("#ff0000")), vec![0x11, 8, 0xFF, 0xFF, 0, 0, 0, 0]);
    }

    #[test]
    fn a_face_wears_its_categorys_colour_and_an_empty_one_is_off() {
        let connection = seeded();
        let (name, hex) = of_face(&connection, 8).expect("face 8");
        assert_eq!(name.as_deref(), Some("Break"));
        assert!(hex.is_some());
        assert_eq!(of_face(&connection, 5).expect("face 5"), (None, None));
        assert_eq!(faces_holding(&connection, 0).expect("unassigned").len(), 10);
    }
}
