//! The face the cube reports as up, from the faces characteristic.

/// The face in a faces read or notification: its one byte, 1 to 12, which is the `face_id` of that face. `None`
/// for anything else, `0` being undefined or a PIN not yet accepted.
pub fn cube_face(bytes: &[u8]) -> Option<u8> {
    match bytes {
        [face] if (1..=12).contains(face) => Some(*face),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_face_is_one_byte_from_one_to_twelve() {
        assert_eq!(cube_face(&[1]), Some(1));
        assert_eq!(cube_face(&[12]), Some(12));
        assert_eq!(cube_face(&[0]), None);
        assert_eq!(cube_face(&[13]), None);
        assert_eq!(cube_face(&[]), None);
        assert_eq!(cube_face(&[2, 0]), None);
    }
}
