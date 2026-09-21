//! The menu bar icon, drawn for whatever state the app is in.
//!
//! **Two independent facts, four icons.** Whether timing is paused, and whether the cube is locked.
//! They are drawn as a base glyph plus a badge rather than as four unrelated pictures, so adding a
//! third fact later composes instead of multiplying.
//!
//! **Why the icon and not just the text.** macOS and MATE can both put text beside the icon and
//! Windows can never, so an app that says what it is doing only in text says nothing at all on one of
//! its three platforms. The icon is the one channel all three have.
//!
//! **No border around the glyph.** One was drawn first and it cost most of the canvas: the menu bar is
//! about 18 points tall, so a frame plus its inset left the play triangle too small to read. The glyph
//! now has the whole icon, and the menu bar supplies the spacing the frame was providing.
//!
//! **Drawn in code, and that is temporary.** These are legible at 32px and no more than that. The real
//! artwork is `Facet.svg`, which wants an SVG rasteriser this binary does not yet pull in.
//!
//! `cargo run -p facet-mac --example draw-status-icons` writes every state to a PNG, so the icon can be
//! looked at without taking over somebody's menu bar to do it.
//!
//! **Alpha is the whole of it**, because the icon is installed as a template image and macOS draws it
//! in whichever colour the menu bar needs, light or dark. Nothing here picks a colour.

use tray_icon::Icon;

/// What the menu bar is saying right now.
///
/// **Not a state this module owns.** It is handed the answer; deciding it belongs with whatever reads
/// the database. Two booleans rather than one enum of four, because they are genuinely independent:
/// a locked cube can be running or paused, and both combinations happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Showing {
    pub paused: bool,
    pub locked: bool,
}

const SIZE: i32 = 32;
/// How far the glyph stays clear of the edge. Small on purpose: the glyph is the whole icon now, and
/// the menu bar already provides the spacing a border used to.
const MARGIN: i32 = 3;
/// The badge, as one set of numbers everything else is derived from.
///
/// **Derived rather than repeated.** The first version placed the badge and limited the glyph with two
/// independent expressions, and they disagreed: the badge overlapped the right pause bar and hung off
/// the edge of the canvas. `GLYPH_RIGHT` now comes from the badge's own position, so the gap between
/// them cannot be closed by editing one of the two.
const BADGE_RADIUS: i32 = 4;
const BADGE_CX: i32 = SIZE - 2 - BADGE_RADIUS;
const BADGE_CY: i32 = BADGE_RADIUS + 1;
/// Where the glyph has to stop so it never touches the badge.
const GLYPH_RIGHT: i32 = BADGE_CX - BADGE_RADIUS;

/// Draws the icon for `showing`.
pub fn draw(showing: Showing) -> Result<Icon, tray_icon::BadIcon> {
    let coverage = alpha(showing);
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    for (i, &on) in coverage.iter().enumerate() {
        rgba[i * 4 + 3] = on;
    }
    Icon::from_rgba(rgba, SIZE as u32, SIZE as u32)
}

/// One byte per pixel: 255 where the glyph is, 0 where it is not.
///
/// **Separate from `draw` so the shape can be tested and previewed without building an `Icon`.** The
/// tests assert on this, and `examples/draw-status-icons.rs` renders it to a PNG, so neither needs a
/// window and neither can drift from what the status item is actually given.
pub fn alpha(showing: Showing) -> Vec<u8> {
    let mut coverage = vec![0u8; (SIZE * SIZE) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            if glyph(x, y, showing.paused) || badge(x, y, showing.locked) {
                coverage[(y * SIZE + x) as usize] = 255;
            }
        }
    }
    coverage
}

/// Two bars when paused, a triangle when running. The universal pair, so nothing has to be learned.
fn glyph(x: i32, y: i32, paused: bool) -> bool {
    // The badge sits in the top-right, so the glyph gives that corner up whether or not one is drawn.
    // Reserving it unconditionally is what stops the glyph changing shape when the cube is locked.
    let top = MARGIN;
    let bottom = SIZE - MARGIN;
    if y < top || y >= bottom {
        return false;
    }

    let left = MARGIN + 1;
    let right = GLYPH_RIGHT;

    if paused {
        // Two bars filling the width the badge leaves, with a gap that keeps them reading as two.
        let gap = 4;
        let bar = (right - left - gap) / 2;
        let right_bar = left + bar + gap;
        (x >= left && x < left + bar) || (x >= right_bar && x < right_bar + bar)
    } else {
        // A triangle pointing right, as wide as the space allows: its height at each column shrinks
        // towards the right-hand tip, which is the shape that reads as "play" at any size.
        if x < left || x >= right {
            return false;
        }
        let span = right - left;
        let centre = (top + bottom - 1) / 2;
        let from_left = x - left;
        // Full height at the base, nothing at the tip.
        let half = ((span - from_left) * (bottom - top)) / (2 * span);
        (y - centre).abs() <= half
    }
}

/// A dot in the top-right corner when the cube is locked.
///
/// **A badge rather than a different glyph**, which is what lets two facts share one icon. It is also
/// the shape the Linux tray gives for free: StatusNotifierItem has an overlay icon property for
/// exactly this, so the same idea is drawn here and declared there.
fn badge(x: i32, y: i32, locked: bool) -> bool {
    if !locked {
        return false;
    }
    let dx = x - BADGE_CX;
    let dy = y - BADGE_CY;
    dx * dx + dy * dy <= BADGE_RADIUS * BADGE_RADIUS
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every state draws something, and no two states draw the same thing.
    ///
    /// The point is not the pixels, it is that a state change is visible: an icon that did not differ
    /// would leave the menu bar saying the same thing about two different situations, which is the
    /// failure this whole module exists to prevent and is invisible in a screenshot of one state.
    #[test]
    fn each_state_draws_a_different_icon() {
        let states = [
            Showing { paused: false, locked: false },
            Showing { paused: true, locked: false },
            Showing { paused: false, locked: true },
            Showing { paused: true, locked: true },
        ];

        let drawn: Vec<Vec<u8>> = states.iter().map(|s| pixels(*s)).collect();

        for (i, a) in drawn.iter().enumerate() {
            assert!(a.iter().any(|&b| b != 0), "{:?} drew nothing at all", states[i]);
            for (j, b) in drawn.iter().enumerate().skip(i + 1) {
                assert_ne!(a, b, "{:?} and {:?} draw the same icon", states[i], states[j]);
            }
        }
    }

    /// The badge is the only difference the lock makes, so locking cannot quietly redraw the glyph.
    #[test]
    fn locking_changes_only_the_corner() {
        let unlocked = pixels(Showing { paused: false, locked: false });
        let locked = pixels(Showing { paused: false, locked: true });

        for y in 0..SIZE {
            for x in 0..SIZE {
                let i = (y * SIZE + x) as usize;
                if unlocked[i] != locked[i] {
                    assert!(
                        x > SIZE / 2 && y < SIZE / 2,
                        "locking changed a pixel at {x},{y}, which is not the badge corner"
                    );
                }
            }
        }
    }

    /// The badge is drawn whole rather than clipped by the edge of the canvas.
    ///
    /// It was clipped first: centred at `SIZE - BADGE_RADIUS`, a quarter of the dot fell outside and it
    /// drew as a wedge. Nothing failed, it just looked wrong, which is the kind of thing a test catches
    /// and a passing build does not.
    #[test]
    fn the_badge_is_not_clipped() {
        let on = |x: i32, y: i32| badge(x, y, true);
        for i in 0..SIZE {
            assert!(!on(i, 0), "the badge touches the top edge at x={i}");
            assert!(!on(i, SIZE - 1), "the badge touches the bottom edge at x={i}");
            assert!(!on(0, i), "the badge touches the left edge at y={i}");
            assert!(!on(SIZE - 1, i), "the badge touches the right edge at y={i}");
        }
    }

    /// The glyph and the badge never share a pixel, in any state.
    ///
    /// They did: the badge sat on top of the right-hand pause bar, which read as one smudge rather than
    /// as two facts. Asserted rather than eyeballed, because it is only obvious in one of the four
    /// states and only at the size nobody previews at.
    #[test]
    fn the_badge_never_touches_the_glyph() {
        for paused in [false, true] {
            for y in 0..SIZE {
                for x in 0..SIZE {
                    assert!(
                        !(glyph(x, y, paused) && badge(x, y, true)),
                        "glyph and badge overlap at {x},{y} with paused={paused}"
                    );
                }
            }
        }
    }

    /// The shape itself, which is what these are about.
    fn pixels(showing: Showing) -> Vec<u8> {
        alpha(showing)
    }
}
