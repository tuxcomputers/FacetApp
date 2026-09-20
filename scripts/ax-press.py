#!/usr/bin/env python3
"""Click one element of a running app, found by name anywhere in its accessibility tree.

    scripts/ax-press.py create-category         # by AXIdentifier
    scripts/ax-press.py --desc Faces            # by AXDescription, for elements that have no identifier
    scripts/ax-press.py --title Close           # by AXTitle
    scripts/ax-press.py --app "Some App" quit-app

Searched for rather than pathed to, which is the point. An AppleScript path like
`button 1 of group 1 of group 1 of window "Facet Settings"` breaks the moment a container is
added between them -- and it breaks by finding the wrong element rather than nothing. Names do not
move: whatever the button ends up nested inside, `create-category` is still `create-category`.

Which is also why everything the app builds carries an AXIdentifier. Where an element cannot have
one -- a segmented control's segments, for instance -- `--desc` matches the label it does carry.

Exits non-zero when nothing matches, so a script can tell a missing element from a click that did
nothing. Needs accessibility permission for whatever runs it.
"""

import argparse
import sys

from AppKit import NSWorkspace
from ApplicationServices import (
    AXUIElementCopyAttributeValue,
    AXUIElementCreateApplication,
    AXUIElementPerformAction,
)


def attribute(element, name):
    error, value = AXUIElementCopyAttributeValue(element, name, None)
    return value if error == 0 else None


def find(element, axattribute, wanted):
    if (value := attribute(element, axattribute)) is not None and str(value) == wanted:
        return element
    for child in attribute(element, "AXChildren") or []:
        if (found := find(child, axattribute, wanted)) is not None:
            return found
    return None


def sheets(element, depth=0):
    """Every AXSheet at or under `element`. Depth-limited: a sheet hangs near the top of a window."""
    found = []
    if str(attribute(element, "AXRole") or "") == "AXSheet":
        found.append(element)
    if depth < 4:
        for child in attribute(element, "AXChildren") or []:
            found.extend(sheets(child, depth + 1))
    return found


def pid_of(app_name):
    for app in NSWorkspace.sharedWorkspace().runningApplications():
        if app.localizedName() == app_name:
            return app.processIdentifier()
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("identifier", nargs="?", help="the element's AXIdentifier")
    parser.add_argument("--desc", help="match AXDescription instead (for elements with no identifier)")
    parser.add_argument("--title", help="match AXTitle instead")
    parser.add_argument("--app", default="Facet", help="the running app (default: Facet)")
    parser.add_argument(
        "--sheet",
        action="store_true",
        help="search only the open sheet, so a confirmation's button is not confused with the one behind it",
    )
    arguments = parser.parse_args()

    chosen = [
        ("AXIdentifier", arguments.identifier),
        ("AXDescription", arguments.desc),
        ("AXTitle", arguments.title),
    ]
    chosen = [(axattribute, value) for axattribute, value in chosen if value]
    if len(chosen) != 1:
        parser.error("name the element exactly one way: an identifier, --desc, or --title")
    axattribute, wanted = chosen[0]

    pid = pid_of(arguments.app)
    if pid is None:
        sys.exit(f"{arguments.app} is not running")
    app = AXUIElementCreateApplication(pid)

    # Windows first, then the menu bar, so a status item -- or an item of the menu it has open -- can be
    # pressed by name too. The extras menu bar is where a status item lives, and is a different attribute from
    # the application menu bar an accessory app does not have.
    # **`--sheet` searches the sheet and nothing else, and it is not a convenience.** A confirmation
    # names its agreeing button after the control that opened it -- "Reset Device" on both -- and a
    # whole-tree search finds the one *behind* the sheet first, presses it, and opens a second
    # confirmation while the first is still up. That is not a hypothetical: it happened on 2026-08-17
    # and read as a reset that silently did nothing.
    if arguments.sheet:
        roots = []
        for window in attribute(app, "AXWindows") or []:
            roots.extend(sheets(window))
        if not roots:
            sys.exit(f"{arguments.app} has no sheet open")
        # The frontmost, matching ax-alert.py: a stale sheet must not answer for the one just opened.
        roots = [roots[-1]]
    else:
        roots = list(attribute(app, "AXWindows") or [])
        for name in ("AXExtrasMenuBar", "AXMenuBar"):
            if (bar := attribute(app, name)) is not None:
                roots.append(bar)
    target = next((found for root in roots if (found := find(root, axattribute, wanted)) is not None), None)
    if target is None:
        sys.exit(f"no element with {axattribute} {wanted!r} in {arguments.app}")

    if AXUIElementPerformAction(target, "AXPress") != 0:
        sys.exit(f"{wanted!r} was found but would not accept a press")
    print(f"pressed {wanted}")


if __name__ == "__main__":
    main()
