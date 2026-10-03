//! When the app stops the cube itself, and when it starts it again.
//!
//! A cube that turns to a face starts recording it in firmware. The app leaves that alone when the face holds a
//! category, and pauses the cube when it holds none. Giving that face a category starts the cube again, but only a
//! cube the app stopped: a pause from anywhere else is not the app's to lift.

/// Why the app paused the cube itself, which decides whether it may start it again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseClaim {
    /// The cube is stopped on `face`, which holds no category. Lifted once that face is given one.
    NoCategory { face: i64 },
    /// The category on show has spent its daily limit. Held until the limit is not spent.
    DailyLimit,
}

/// The cube as its open segment shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resting {
    pub face: i64,
    pub is_paused: bool,
    pub has_category: bool,
    pub is_limit_reached: bool,
}

/// What to do about the cube's pause state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Send nothing. The claim stands as it was.
    Leave,
    /// Send nothing, and drop the claim: it no longer describes the cube.
    Release,
    /// Stop the cube, claiming the pause as `PauseClaim`.
    Pause(PauseClaim),
    /// Start the cube the app stopped for `PauseClaim`.
    Resume(PauseClaim),
}

/// What to do about the cube as it rests now, given the claim the app holds.
///
/// A cube seen running on a face with a category ends any claim, however it came to be running: turning the cube
/// starts it in firmware, and a pause after that is not the app's. A claim for a face with no category is lifted
/// only while the cube is still stopped on that face, and is dropped once it is stopped anywhere else.
pub fn decide(cube: Resting, claim: Option<PauseClaim>) -> Decision {
    if !cube.is_paused {
        return if !cube.has_category {
            Decision::Pause(PauseClaim::NoCategory { face: cube.face })
        } else if cube.is_limit_reached {
            Decision::Pause(PauseClaim::DailyLimit)
        } else if claim.is_some() {
            Decision::Release
        } else {
            Decision::Leave
        };
    }
    match claim {
        Some(PauseClaim::NoCategory { face }) if face != cube.face => Decision::Release,
        Some(PauseClaim::NoCategory { face }) if cube.has_category && !cube.is_limit_reached => {
            Decision::Resume(PauseClaim::NoCategory { face })
        }
        _ => Decision::Leave,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(face: i64, is_paused: bool, has_category: bool) -> Resting {
        Resting { face, is_paused, has_category, is_limit_reached: false }
    }

    const STOPPED_ON_3: Option<PauseClaim> = Some(PauseClaim::NoCategory { face: 3 });

    #[test]
    fn a_face_with_a_category_is_left_running() {
        assert_eq!(decide(cube(5, false, true), None), Decision::Leave);
    }

    #[test]
    fn a_face_with_no_category_is_paused_and_claimed() {
        assert_eq!(decide(cube(3, false, false), None), Decision::Pause(PauseClaim::NoCategory { face: 3 }));
    }

    #[test]
    fn a_spent_limit_is_paused_and_claimed() {
        let spent = Resting { is_limit_reached: true, ..cube(5, false, true) };
        assert_eq!(decide(spent, None), Decision::Pause(PauseClaim::DailyLimit));
    }

    #[test]
    fn giving_the_stopped_face_a_category_starts_the_cube() {
        assert_eq!(
            decide(cube(3, true, true), STOPPED_ON_3),
            Decision::Resume(PauseClaim::NoCategory { face: 3 })
        );
    }

    #[test]
    fn a_face_still_with_no_category_stays_stopped() {
        assert_eq!(decide(cube(3, true, false), STOPPED_ON_3), Decision::Leave);
    }

    #[test]
    fn a_face_with_a_category_but_a_spent_limit_stays_stopped() {
        let spent = Resting { is_limit_reached: true, ..cube(3, true, true) };
        assert_eq!(decide(spent, STOPPED_ON_3), Decision::Leave);
    }

    #[test]
    fn a_pause_the_app_did_not_place_is_not_lifted() {
        assert_eq!(decide(cube(5, true, true), None), Decision::Leave);
    }

    #[test]
    fn turning_to_a_face_with_a_category_ends_the_claim_so_a_later_pause_is_left_alone() {
        // Stopped on face 3 and claimed, then turned to face 5, which starts it in firmware.
        assert_eq!(decide(cube(5, false, true), STOPPED_ON_3), Decision::Release);
        // The claim is gone, so a pause on face 5 after that is not the app's to lift.
        assert_eq!(decide(cube(5, true, true), None), Decision::Leave);
    }

    #[test]
    fn a_claim_for_one_face_is_not_used_to_start_the_cube_on_another() {
        assert_eq!(decide(cube(5, true, true), STOPPED_ON_3), Decision::Release);
    }

    #[test]
    fn turning_to_another_face_with_no_category_claims_that_face() {
        assert_eq!(
            decide(cube(7, false, false), STOPPED_ON_3),
            Decision::Pause(PauseClaim::NoCategory { face: 7 })
        );
    }
}
