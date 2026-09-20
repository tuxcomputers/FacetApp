#!/usr/bin/env python3
"""Print a running app's accessibility tree, which is what a UI script sees.

    scripts/ax-dump.py                      # every window of Facet
    scripts/ax-dump.py --frames             # with each element's position and size
    scripts/ax-dump.py --app "Some App"     # somebody else's tree

Written to answer "is everything named?" -- every element the app builds should carry an
AXIdentifier, and this is what proves it rather than assuming it. It has already earned its keep
three times over: an NSView pane came back with no identifier at all (an ordinary NSView is not an
accessibility element, so setAccessibilityIdentifier is never asked for without
setAccessibilityElement(true)); NSTabViewItem's identifier turned out never to reach AXIdentifier;
and a segmented control's segments carry their label as AXDescription with no AXTitle, so
`radio button "Faces"` finds nothing while matching on description finds it.

Needs accessibility permission for whatever runs it (Terminal, or your editor). Without it the
tree comes back empty rather than erroring.
"""

import argparse
import sys

from AppKit import NSWorkspace
from ApplicationServices import AXUIElementCopyAttributeValue, AXUIElementCreateApplication

# Printed for every element, in this order, and left out when absent. Identifier first after the
# role, because it is the thing a script should be addressing elements by.
NAMED_ATTRIBUTES = (
    ("AXIdentifier", "id"),
    ("AXTitle", "title"),
    ("AXDescription", "desc"),
    ("AXValue", "value"),
)


def attribute(element, name):
    error, value = AXUIElementCopyAttributeValue(element, name, None)
    return value if error == 0 else None


def describe(element, show_frames):
    parts = []
    for name, label in NAMED_ATTRIBUTES:
        value = attribute(element, name)
        text = "" if value is None else str(value)
        # An AXValue holding a child element prints as an opaque pointer; skip it rather than
        # putting a memory address in the output.
        if text and not text.startswith("<AXUIElement"):
            parts.append(f"{label}={text}")
    # **Only when it is off.** A tree where every line carried `enabled=true` would be harder to read for a
    # fact that is nearly always the same, and the checks that care are the ones asserting a control is dead --
    # a greyed Resume under a spent daily limit, a locked face's frozen row.
    if attribute(element, "AXEnabled") is False:
        parts.append("disabled")
    if show_frames:
        for name, label in (("AXPosition", "pos"), ("AXSize", "size")):
            if (value := attribute(element, name)) is not None:
                parts.append(f"{label}={compact(str(value))}")
    return "  ".join(parts) if parts else "<unnamed>"


def compact(axvalue):
    """`{value = x:12.000000 y:3.000000 ...}` down to `x:12 y:3`."""
    inner = axvalue.split("{value = ")[-1].split(" type =")[0]
    return inner.replace(".000000", "")


def walk(element, show_frames, depth=0):
    role = attribute(element, "AXRole") or "?"
    print(f"{'  ' * depth}{role}  {describe(element, show_frames)}")
    for child in attribute(element, "AXChildren") or []:
        walk(child, show_frames, depth + 1)


def pid_of(app_name):
    for app in NSWorkspace.sharedWorkspace().runningApplications():
        if app.localizedName() == app_name:
            return app.processIdentifier()
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--app", default="Facet", help="the running app to dump (default: Facet)")
    parser.add_argument("--frames", action="store_true", help="also print each element's position and size")
    parser.add_argument(
        "--menu-bar",
        action="store_true",
        help="dump the app's status items instead of its windows, including any menu it has open",
    )
    arguments = parser.parse_args()

    pid = pid_of(arguments.app)
    if pid is None:
        sys.exit(f"{arguments.app} is not running")
    app = AXUIElementCreateApplication(pid)

    if arguments.menu_bar:
        # A status item is in the *extras* menu bar, which is a separate attribute from the application menu
        # bar -- and an accessory app has no application menu bar at all, so asking for AXMenuBar alone comes
        # back empty and looks like the app has nothing up there.
        bars = [attribute(app, "AXExtrasMenuBar"), attribute(app, "AXMenuBar")]
        found = False
        for bar in bars:
            if bar is not None:
                walk(bar, arguments.frames)
                found = True
        if not found:
            print(f"{arguments.app} has nothing in the menu bar", file=sys.stderr)
        return
    windows = attribute(app, "AXWindows") or []
    if not windows:
        print(f"{arguments.app} is running with no windows open", file=sys.stderr)
    for window in windows:
        walk(window, arguments.frames)


if __name__ == "__main__":
    main()
