#!/usr/bin/env python3
"""Press and hold a control, by name, for a number of seconds.

    scripts/at-hold.py category-limit-3-up 3.0

The Linux counterpart of `ax-hold.py`, and it exists for the same reason: **an accessible action
cannot hold.** A stepper arrow steps once when it is pressed and keeps stepping while it is held, and
those are two different code paths on purpose. `doAction(0)` is the first of them, so a script
pressing the arrow by name exercises the single step and can never reach the repeat, the
acceleration, or the change of step size -- the parts with the timing in them.

This posts a real left-button down at the control's centre, waits, and posts the up. **That is a real
pointer event on the real screen**: the cursor moves, and whatever is under it is what receives the
click. Nothing here belongs in a run somebody is using the machine during.

**The position comes out of the tree at hold time rather than being remembered**, which is the same
care `status-item-click.py` takes on the other platform: rows move as a list grows, and a coordinate
that was right a moment ago lands on the row below.

**Desktop coordinates, not window ones.** `getExtents` takes which it should answer in, and XTEST
speaks screen coordinates; asking for window coordinates and posting them puts the click above and
left of the control by however far the window is from the origin, which on a maximised window is
almost right and so is the version that gets shipped.
"""

import argparse
import sys
import time

sys.path.insert(0, __file__.rsplit("/", 1)[0])

import pyatspi                                                          # noqa: E402

from atspi_tree import application, by_name, extents, require, role_of  # noqa: E402

# What GTK answers for a widget it has sized but never placed; see the check below. Kept in step with
# the same constant in `at-dump.py`, which prints such a position as `unplaced`.
UNPLACED = -2_000_000_000


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("name", help="the identifier of the control to hold")
    parser.add_argument("seconds", type=float, help="how long to hold it")
    parser.add_argument("--app", default="FacetLinux", help="the application to drive")
    arguments = parser.parse_args()

    root = application(arguments.app)
    node = require(root, by_name(arguments.name), f"name {arguments.name!r}")

    try:
        box = extents(node)
    except NotImplementedError:
        sys.exit(f"{arguments.name!r} is a {role_of(node)} and has no position on screen to click")

    if box.width <= 0 or box.height <= 0:
        # **Nothing is drawn there**, which is what an element on a folded section reads as. Clicking
        # its centre would land at (0,0)-ish on whatever is behind the window, so this refuses instead.
        sys.exit(
            f"{arguments.name!r} is in the tree but has no size on screen ({box.width}x{box.height}).\n"
            f"  it is probably inside a section that is folded: open it first."
        )

    # **Sized but never placed**, which is every control on a notebook page that is not on show: GTK
    # answers `INT_MIN` for the position while reporting an ordinary size (measured 2026-09-20). The
    # size test above passes it, so without this the hold posts a real pointer event at the far corner
    # of the coordinate space -- landing on nothing, disturbing whatever is there, and reporting a hold
    # that happened. The search already prunes unselected tabs, so reaching this means the control is on
    # the tab that is on show and something else is keeping it off screen.
    if box.x <= UNPLACED or box.y <= UNPLACED:
        sys.exit(
            f"{arguments.name!r} has a size but no position, so GTK has never placed it.\n"
            f"  it is on the tab that is on show, so something is keeping it off screen:\n"
            f"  a folded section, or a window that has not been drawn yet."
        )

    x = box.x + box.width // 2
    y = box.y + box.height // 2

    pyatspi.Registry.generateMouseEvent(x, y, "abs")
    pyatspi.Registry.generateMouseEvent(x, y, "b1p")
    try:
        time.sleep(arguments.seconds)
    finally:
        # **Released whatever happened in between.** A button left down by an interrupted script makes
        # every subsequent gesture on the machine a drag, and the person who finds out is the owner.
        pyatspi.Registry.generateMouseEvent(x, y, "b1r")

    print(f"held {arguments.name!r} at ({x},{y}) for {arguments.seconds:g}s")
    return 0


if __name__ == "__main__":
    sys.exit(main())
