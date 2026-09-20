#!/usr/bin/env python3
"""Print the buttons of the sheet a running app has up, one per line, in the order they are drawn.

    scripts/ax-alert.py                     # Reactivate / Create new one / Cancel
    scripts/ax-alert.py --message           # the alert's text instead of its buttons

Written for the checks that turn on a button being **absent**. Creating a category whose name one
retired category holds offers three answers; when several hold it, Reactivate is not offered at all,
because nothing on a button could say which one to bring back. Asserting that means counting the
buttons of the alert, and grepping the window's tree cannot: a `Cancel` anywhere else on the tab
would be counted, and a missing button looks identical to a tree that was read a moment too early.

So this addresses the sheet itself and nothing else. Exits non-zero when no sheet is up, which is
what tells a script "the alert never appeared" apart from "the alert appeared with no buttons".

An NSAlert run with `beginSheetModal` is an AXSheet hanging off its parent window, not a window of
its own -- `AXWindows` alone does not always list it, so the search walks into each window as well.
"""

import argparse
import sys

from AppKit import NSWorkspace
from ApplicationServices import AXUIElementCopyAttributeValue, AXUIElementCreateApplication


def attribute(element, name):
    error, value = AXUIElementCopyAttributeValue(element, name, None)
    return value if error == 0 else None


def sheets(element, depth=0):
    """Every AXSheet at or under `element`. Depth-limited: a sheet is near the top of a window."""
    found = []
    if str(attribute(element, "AXRole") or "") == "AXSheet":
        found.append(element)
    if depth < 4:
        for child in attribute(element, "AXChildren") or []:
            found.extend(sheets(child, depth + 1))
    return found


def descendants(element, role):
    found = []
    if str(attribute(element, "AXRole") or "") == role:
        found.append(element)
    for child in attribute(element, "AXChildren") or []:
        found.extend(descendants(child, role))
    return found


def pid_of(app_name):
    for app in NSWorkspace.sharedWorkspace().runningApplications():
        if app.localizedName() == app_name:
            return app.processIdentifier()
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--app", default="Facet", help="the running app (default: Facet)")
    parser.add_argument(
        "--message", action="store_true", help="print the alert's text rather than its buttons"
    )
    arguments = parser.parse_args()

    pid = pid_of(arguments.app)
    if pid is None:
        sys.exit(f"{arguments.app} is not running")
    app = AXUIElementCreateApplication(pid)

    found = []
    for window in attribute(app, "AXWindows") or []:
        found.extend(sheets(window))
    if not found:
        sys.exit(f"{arguments.app} has no sheet open")

    # The frontmost one. More than one is not a shape this app produces, and picking the last keeps
    # a stale sheet from answering for the one just opened if it ever does.
    sheet = found[-1]

    if arguments.message:
        for text in descendants(sheet, "AXStaticText"):
            value = attribute(text, "AXValue")
            if value is not None and str(value):
                print(str(value))
        return

    # Title rather than description: this app's alert buttons carry AXTitle, which is what
    # `ax-press.py --title` already presses them by. Printed even when empty, so a button that
    # carries no title still counts towards the number of buttons rather than vanishing from it.
    for button in descendants(sheet, "AXButton"):
        print(str(attribute(button, "AXTitle") or ""))


if __name__ == "__main__":
    main()
