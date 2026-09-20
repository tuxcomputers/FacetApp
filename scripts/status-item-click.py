#!/usr/bin/env python3
"""Click the app's menu bar status item, on whichever half you name.

    scripts/status-item-click.py                # the left half: opens the menu
    scripts/status-item-click.py --right        # the right half
    scripts/status-item-click.py --double       # a double click

The one gesture in this app that cannot be driven by name. Everything else has an AXIdentifier and can
be pressed with `ax-press.py`, but a status item exposes **no accessibility action at all** (asked for
AXActions: it has none), so reaching its menu takes a real mouse event. Once the menu is open its items
are ordinary named elements again: `ax-press.py open-settings`, `ax-press.py quit-app`.

The item's position is read from the accessibility tree at click time rather than remembered. The status
item's width follows its title -- which is a live duration once something is being timed -- so a
coordinate that was right a minute ago can miss it. Halves matter because the two sides do different
things: the left opens the menu, the right is where pause will live.

Needs accessibility permission for whatever runs it.
"""

import argparse
import sys
import time

import Quartz
from AppKit import NSWorkspace
from ApplicationServices import AXUIElementCopyAttributeValue, AXUIElementCreateApplication

# The status item's own identifier, set in MenuBarController.
STATUS_ITEM = "status-item"


def attribute(element, name):
    error, value = AXUIElementCopyAttributeValue(element, name, None)
    return value if error == 0 else None


def status_item(app):
    """The status item, from the *extras* menu bar -- an accessory app has no application menu bar."""
    bar = attribute(app, "AXExtrasMenuBar")
    for item in attribute(bar, "AXChildren") or []:
        if attribute(item, "AXIdentifier") == STATUS_ITEM:
            return item
    return None


def frame(element):
    position = attribute(element, "AXPosition")
    size = attribute(element, "AXSize")
    if position is None or size is None:
        return None
    # AXValue prints as `{value = x:2241.000000 y:3.000000 type = ...}`; the numbers are what is wanted.
    def numbers(axvalue):
        inner = str(axvalue).split("{value = ")[-1].split(" type =")[0]
        return [float(part.split(":")[1]) for part in inner.split()]

    x, y = numbers(position)
    width, height = numbers(size)
    return x, y, width, height


def click(x, y, double):
    def post(kind, state):
        event = Quartz.CGEventCreateMouseEvent(None, kind, (x, y), Quartz.kCGMouseButtonLeft)
        Quartz.CGEventSetIntegerValueField(event, Quartz.kCGMouseEventClickState, state)
        Quartz.CGEventPost(Quartz.kCGHIDEventTap, event)

    # **The cursor is moved onto the item first, as a real click does.** A bare down-up at a coordinate the
    # pointer is nowhere near is not the sequence AppKit sees from a person, and a status item tracks the
    # mouse: it takes the highlight from the move and the press from what follows. Posting the move and
    # giving the window server a moment to deliver it is what makes the press land on a highlighted item
    # rather than on one that has not noticed the pointer yet.
    post(Quartz.kCGEventMouseMoved, 0)
    time.sleep(0.05)
    post(Quartz.kCGEventLeftMouseDown, 1)
    post(Quartz.kCGEventLeftMouseUp, 1)
    if double:
        time.sleep(0.15)
        post(Quartz.kCGEventLeftMouseDown, 2)
        post(Quartz.kCGEventLeftMouseUp, 2)


def pid_of(app_name):
    for app in NSWorkspace.sharedWorkspace().runningApplications():
        if app.localizedName() == app_name:
            return app.processIdentifier()
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--right", action="store_true", help="click the right half instead of the left")
    parser.add_argument("--double", action="store_true", help="post a double click")
    parser.add_argument("--app", default="Facet", help="the running app (default: Facet)")
    arguments = parser.parse_args()

    pid = pid_of(arguments.app)
    if pid is None:
        sys.exit(f"{arguments.app} is not running")
    item = status_item(AXUIElementCreateApplication(pid))
    if item is None:
        sys.exit(f"{arguments.app} has no status item with identifier {STATUS_ITEM!r}")
    measured = frame(item)
    if measured is None:
        sys.exit("the status item reported no position or size")

    # **Measured twice, because the item moves while this is running.** Its width follows its title, which is a
    # live duration once something is being timed, and the menu bar is right-aligned -- so a second of ticking
    # between the read and the post shifts the left edge and can put a right-half click past the right edge
    # entirely. Reading again and refusing to click on a frame that has changed costs a millisecond and removes
    # a whole class of click that goes nowhere and reports nothing.
    again = frame(item)
    if again is None:
        sys.exit("the status item stopped reporting a position between the two reads")
    if again != measured:
        measured = again

    x, y, width, height = measured
    if width <= 0 or height <= 0:
        sys.exit(f"the status item has no area to click ({width:.0f}x{height:.0f})")

    # A quarter in from the chosen edge: far enough from the midpoint that a rounding difference cannot put
    # the click on the wrong side of it.
    offset = width * 0.75 if arguments.right else width * 0.25
    click(x + offset, y + height / 2, arguments.double)
    print(f"clicked the {'right' if arguments.right else 'left'} half at ({x + offset:.0f}, {y + height / 2:.0f})")


if __name__ == "__main__":
    main()
