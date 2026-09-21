//! The menu bar icon, drawn for whatever state the app is in.
//!
//! **Three glyphs, and the icon is as wide as it needs to be.** Play or Pause always, with Lock beside
//! it at the same size when the cube is locked. Two facts, two glyphs, side by side.
//!
//! **Composed from three glyphs rather than stored as four pictures.** The four combinations are
//! exactly Play, Pause, Play+Lock and Pause+Lock, so four files would work and were offered. Three
//! glyphs are kept instead because states multiply: a third fact, a low battery or a lost connection,
//! is one more glyph here and eight files the other way. `examples/draw-status-icons.rs` writes the
//! four images out, so they can still be looked at as files.
//!
//! **This works because macOS sizes a status item by aspect, not by fitting a square.** tray-icon asks
//! for an 18 point tall image and computes the width from the ratio, so a 70 by 32 bitmap becomes a 39
//! by 18 point item and the status item widens to hold it. Nothing is squashed and nothing shrinks.
//!
//! **Windows cannot do this, and it is worth knowing before the design spreads.** A tray icon there is
//! an `HICON` drawn into the shell's own square slot, so a 2:1 image is squashed into it rather than
//! given room. Two same-size glyphs side by side is a macOS and Linux shape; Windows needs both inside
//! one square, which means each is about half the size, or a badge in a corner. That is the Windows
//! adapter's decision and `docs/port-findings.md` records it, so it is not met as a squashed icon.
//!
//! **Linux is unmeasured.** ksni's `icon_pixmap` carries its own width and height and the specification
//! permits a non-square icon, so this should behave on MATE as it does here. Nobody has run it.
//!
//! **Why the icon and not just the text.** macOS and MATE can both put text beside the icon and Windows
//! can never, so an app that says what it is doing only in text says nothing at all on one of its three
//! platforms. The icon is the one channel all three have.
//!
//! **Drawn in code, and that is temporary.** These are legible at this size and no more than that. The
//! real artwork is `Facet.svg`, which wants an SVG rasteriser this binary does not yet pull in.
//!
//! **Each glyph carries its own colour, which means the icon is not a template image.** A template is
//! alpha only: macOS throws the colours away and draws the shape in the menu bar's own ink, black on a
//! light bar and white on a dark one. That is why an earlier build came out black whatever it was told.
//! `with_icon_as_template(false)` is what lets these colours through.
//!
//! **The cost is that nothing adapts any more, and white is the one that suffers.** Green and red read
//! on either menu bar. White pause does not: on a light bar it is white on near-white. The colours are
//! what was asked for and they are here; `examples/draw-status-icons.rs` renders every state on both a
//! light and a dark background so the problem can be looked at rather than argued about.

use tray_icon::Icon;

/// What the menu bar is saying right now.
///
/// **Not a state this module owns.** It is handed the answer; deciding it belongs with whatever reads
/// the database. Two booleans rather than one enum of four, because they are genuinely independent: a
/// locked cube can be running or paused, and both combinations happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Showing {
    pub paused: bool,
    pub locked: bool,
}

/// One glyph's square. Every glyph gets the same cell, which is what makes them the same size.
const CELL: i32 = 32;
/// Clear pixels between two glyphs, so they read as two things rather than one wide smudge.
const GAP: i32 = 6;
/// How far a glyph stays clear of its cell's edge.
const MARGIN: i32 = 3;

/// A rendered icon. The width varies with the state, which is the point.
pub struct Rendered {
    pub width: u32,
    pub height: u32,
    /// Four bytes per pixel, RGBA, premultiplied by nothing: transparent where there is no ink.
    pub rgba: Vec<u8>,
}

/// The glyphs, each drawn into its own cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Glyph {
    Play,
    Pause,
    Lock,
}

impl Glyph {
    /// The colour it is drawn in.
    ///
    /// **The system palette rather than pure channels**, because a flat `0,255,0` is harsh beside the
    /// rest of the menu bar and reads as a warning rather than as a state. These are Apple's own green
    /// and red, which is what everything else up there uses.
    fn colour(self) -> [u8; 3] {
        match self {
            Glyph::Play => [52, 199, 89],
            Glyph::Pause => [255, 255, 255],
            Glyph::Lock => [255, 59, 48],
        }
    }
}

/// Draws the icon for `showing`.
pub fn draw(showing: Showing) -> Result<Icon, tray_icon::BadIcon> {
    let rendered = render(showing);
    Icon::from_rgba(rendered.rgba, rendered.width, rendered.height)
}

/// The shape, without building an `Icon`.
///
/// **Separate from `draw` so it can be tested and previewed without a window.** The tests assert on
/// this and the example renders it, so neither can drift from what the status item is actually given.
pub fn render(showing: Showing) -> Rendered {
    let mut glyphs = vec![if showing.paused { Glyph::Pause } else { Glyph::Play }];
    if showing.locked {
        glyphs.push(Glyph::Lock);
    }

    let count = glyphs.len() as i32;
    let width = CELL * count + GAP * (count - 1);
    let mut rgba = vec![0u8; (width * CELL * 4) as usize];

    for (slot, glyph) in glyphs.iter().enumerate() {
        let origin = slot as i32 * (CELL + GAP);
        let [r, g, b] = glyph.colour();
        for y in 0..CELL {
            for x in 0..CELL {
                if ink(*glyph, x, y) {
                    let i = ((y * width + origin + x) * 4) as usize;
                    rgba[i] = r;
                    rgba[i + 1] = g;
                    rgba[i + 2] = b;
                    rgba[i + 3] = 255;
                }
            }
        }
    }

    Rendered { width: width as u32, height: CELL as u32, rgba }
}

/// Whether `glyph` covers `(x, y)` within its own cell.
fn ink(glyph: Glyph, x: i32, y: i32) -> bool {
    match glyph {
        Glyph::Play => play(x, y),
        Glyph::Pause => pause(x, y),
        Glyph::Lock => lock(x, y),
    }
}

/// A triangle pointing right: full height at the base, nothing at the tip.
fn play(x: i32, y: i32) -> bool {
    let (left, right) = (MARGIN + 1, CELL - MARGIN - 1);
    let (top, bottom) = (MARGIN + 1, CELL - MARGIN - 1);
    if x < left || x >= right || y < top || y >= bottom {
        return false;
    }
    let span = right - left;
    let centre = (top + bottom - 1) / 2;
    let half = ((span - (x - left)) * (bottom - top)) / (2 * span);
    (y - centre).abs() <= half
}

/// Two bars, with a gap wide enough that they never read as one block.
fn pause(x: i32, y: i32) -> bool {
    let (left, right) = (MARGIN + 1, CELL - MARGIN - 1);
    let (top, bottom) = (MARGIN + 1, CELL - MARGIN - 1);
    if y < top || y >= bottom {
        return false;
    }
    let inner_gap = 5;
    let bar = (right - left - inner_gap) / 2;
    let second = left + bar + inner_gap;
    (x >= left && x < left + bar) || (x >= second && x < second + bar)
}

/// A padlock: a solid body with a shackle over it.
///
/// **The shackle is an arc rather than a drawn outline**, so its thickness is one number and it cannot
/// come out lopsided. Only the half above the body is kept, which is what turns a ring into a shackle.
fn lock(x: i32, y: i32) -> bool {
    let body_top = 15;
    let body =
        x >= MARGIN + 2 && x < CELL - MARGIN - 2 && y >= body_top && y < CELL - MARGIN - 1;

    let dx = x - CELL / 2;
    let dy = y - body_top;
    let squared = dx * dx + dy * dy;
    let shackle = y < body_top && squared <= 8 * 8 && squared >= 5 * 5;

    body || shackle
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATES: [Showing; 4] = [
        Showing { paused: false, locked: false },
        Showing { paused: true, locked: false },
        Showing { paused: false, locked: true },
        Showing { paused: true, locked: true },
    ];

    /// Every state draws something, and no two states draw the same thing.
    ///
    /// An icon that did not differ would leave the menu bar saying the same thing about two different
    /// situations, which is the failure this module exists to prevent and is invisible in a screenshot
    /// of any one state.
    #[test]
    fn each_state_draws_a_different_icon() {
        let drawn: Vec<(u32, Vec<u8>)> =
            STATES.iter().map(|s| { let r = render(*s); (r.width, r.rgba) }).collect();

        for (i, a) in drawn.iter().enumerate() {
            assert!(a.1.iter().any(|&b| b != 0), "{:?} drew nothing at all", STATES[i]);
            for (j, b) in drawn.iter().enumerate().skip(i + 1) {
                assert_ne!(a, b, "{:?} and {:?} draw the same icon", STATES[i], STATES[j]);
            }
        }
    }

    /// Locking adds a second glyph and leaves the first exactly as it was.
    ///
    /// Whether time is running has to read the same whether or not the cube is locked, or the two facts
    /// are not independent after all.
    #[test]
    fn locking_adds_a_glyph_and_changes_nothing_else() {
        for paused in [false, true] {
            let plain = render(Showing { paused, locked: false });
            let locked = render(Showing { paused, locked: true });

            assert_eq!(plain.width, CELL as u32, "an unlocked icon is one cell wide");
            assert_eq!(locked.width, (CELL * 2 + GAP) as u32, "a locked icon is two cells and a gap");
            assert_eq!(plain.height, locked.height, "locking must not change the height");

            for y in 0..CELL {
                for x in 0..CELL {
                    let before = &plain.rgba[((y * CELL + x) * 4) as usize..][..4];
                    let after =
                        &locked.rgba[((y * locked.width as i32 + x) * 4) as usize..][..4];
                    assert_eq!(before, after, "the first glyph changed at {x},{y} when locked");
                }
            }
        }
    }

    /// The two glyphs never touch: every column of the gap is empty.
    #[test]
    fn there_is_clear_space_between_the_glyphs() {
        let locked = render(Showing { paused: true, locked: true });
        for x in CELL..CELL + GAP {
            for y in 0..CELL {
                let alpha = locked.rgba[((y * locked.width as i32 + x) * 4 + 3) as usize];
                assert_eq!(alpha, 0, "there is ink in the gap at {x},{y}");
            }
        }
    }

    /// No glyph runs to the edge of its own cell, in any state.
    ///
    /// One drawn past its cell would either be clipped by the canvas or collide with its neighbour, and
    /// both read as a rendering fault rather than as a state.
    #[test]
    fn no_glyph_touches_the_edge_of_its_cell() {
        for glyph in [Glyph::Play, Glyph::Pause, Glyph::Lock] {
            let mut any = false;
            for y in 0..CELL {
                for x in 0..CELL {
                    if ink(glyph, x, y) {
                        any = true;
                        assert!(
                            x > 0 && x < CELL - 1 && y > 0 && y < CELL - 1,
                            "{glyph:?} touches the edge of its cell at {x},{y}"
                        );
                    }
                }
            }
            assert!(any, "{glyph:?} drew nothing at all");
        }
    }

    /// Each glyph is drawn in its own colour, and every lit pixel carries it.
    ///
    /// **Worth asserting because the colours only appear if the icon is not a template.** A template
    /// image has its colours thrown away by macOS, which is how an earlier build came out black; this
    /// catches the pixels being wrong, and the comment on `with_icon_as_template` in main.rs is what
    /// keeps the other half honest.
    #[test]
    fn every_glyph_is_drawn_in_its_own_colour() {
        let expect = |showing: Showing, slot: i32, colour: [u8; 3]| {
            let r = render(showing);
            let origin = slot * (CELL + GAP);
            let mut seen = 0;
            for y in 0..CELL {
                for x in 0..CELL {
                    let i = ((y * r.width as i32 + origin + x) * 4) as usize;
                    if r.rgba[i + 3] != 0 {
                        seen += 1;
                        assert_eq!(
                            [r.rgba[i], r.rgba[i + 1], r.rgba[i + 2]], colour,
                            "wrong colour at {x},{y} of slot {slot} for {showing:?}"
                        );
                    }
                }
            }
            assert!(seen > 0, "slot {slot} of {showing:?} has no lit pixels");
        };

        let green = Glyph::Play.colour();
        let white = Glyph::Pause.colour();
        let red = Glyph::Lock.colour();

        expect(Showing { paused: false, locked: false }, 0, green);
        expect(Showing { paused: true, locked: false }, 0, white);
        expect(Showing { paused: false, locked: true }, 0, green);
        expect(Showing { paused: false, locked: true }, 1, red);
        expect(Showing { paused: true, locked: true }, 0, white);
        expect(Showing { paused: true, locked: true }, 1, red);
    }

    /// The lock reads as a padlock: a shackle above a wider body, joined rather than floating.
    #[test]
    fn the_lock_has_a_shackle_above_a_wider_body() {
        let row_ink = |y: i32| (0..CELL).filter(|&x| lock(x, y)).count();

        assert!(row_ink(8) > 0, "the shackle has no ink near the top");
        assert!(row_ink(20) > row_ink(8), "the body is not wider than the shackle");
        // No empty row across the middle, or the shackle and body would look detached.
        for y in 9..26 {
            assert!(row_ink(y) > 0, "the lock has a gap across it at y={y}");
        }
    }
}
