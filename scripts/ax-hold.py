#!/usr/bin/env python3
"""Press and hold a control, by name, for a number of seconds.

    scripts/ax-hold.py category-limit-3-up 3.0

**Because AXPress cannot hold.** A stepper arrow steps once when clicked and keeps stepping while held,
and those are two different code paths on purpose: `HoldArrow` takes over `mouseDown` for the hold and
leaves the action path to `performClick`, which is exactly what accessibility uses. So a script pressing
the arrow by name exercises the single step and can never reach the repeat, the acceleration, or the
change of step size -- the parts with the timing in them.

This posts a real left-button down at the control's centre, waits, and posts the up. That is a real mouse
event on the real screen: the cursor moves, and whatever is under it is what receives the click. Nothing
here belongs in a run somebody is using the machine during.

The frame comes out of the accessibility tree as a string rather than through AXValueGetValue, which is
what `ax-dump.py --frames` already does and is known to work on this app.
"""

import re
import subprocess
import sys
import time

import Quartz
from AppKit import NSWorkspace
from ApplicationServices import AXUIElementCopyAttributeValue, AXUIElementCreateApplication


def attribute(element, name):
    error, value = AXUIElementCopyAttributeValue(element, name, None)
    return value if error == 0 else None


def find(element, wanted, depth=0):
    if str(attribute(element, "AXIdentifier") or "") == wanted:
        return element
    for child in attribute(element, "AXChildren") or []:
        found = find(child, wanted, depth + 1)
        if found is not None:
            return found
    return None


def pair(element, name, keys):
    """`{value = x:12.000000 y:3.000000 ...}` to (12.0, 3.0)."""
    raw = attribute(element, name)
    if raw is None:
        return None
    numbers = {}
    for key in keys:
        found = re.search(rf"\b{key}:(-?[0-9.]+)", str(raw))
        if not found:
            return None
        numbers[key] = float(found.group(1))
    return tuple(numbers[key] for key in keys)


def main():
    if len(sys.argv) < 3:
        sys.exit("usage: ax-hold.py <identifier> <seconds> [--app Facet]")
    identifier, seconds = sys.argv[1], float(sys.argv[2])
    app_name = sys.argv[4] if len(sys.argv) > 4 else "Facet"

    pid = next(
        (a.processIdentifier() for a in NSWorkspace.sharedWorkspace().runningApplications()
         if a.localizedName() == app_name),
        None,
    )
    if pid is None:
        sys.exit(f"{app_name} is not running")

    app = AXUIElementCreateApplication(pid)
    target = next(
        (found for window in (attribute(app, "AXWindows") or [])
         if (found := find(window, identifier)) is not None),
        None,
    )
    if target is None:
        sys.exit(f"no element with AXIdentifier {identifier!r} in {app_name}")

    position = pair(target, "AXPosition", ("x", "y"))
    size = pair(target, "AXSize", ("w", "h"))
    if position is None or size is None:
        sys.exit(f"{identifier} has no frame to click in the middle of")
    point = (position[0] + size[0] / 2, position[1] + size[1] / 2)

    # The app has to be in front, or the first click is spent activating it rather than pressing anything.
    subprocess.run(["osascript", "-e", f'tell application "{app_name}" to activate'], check=False)
    time.sleep(0.4)

    for kind in (Quartz.kCGEventMouseMoved, Quartz.kCGEventLeftMouseDown):
        Quartz.CGEventPost(
            Quartz.kCGHIDEventTap,
            Quartz.CGEventCreateMouseEvent(None, kind, point, Quartz.kCGMouseButtonLeft),
        )
        time.sleep(0.05)

    time.sleep(seconds)

    Quartz.CGEventPost(
        Quartz.kCGHIDEventTap,
        Quartz.CGEventCreateMouseEvent(None, Quartz.kCGEventLeftMouseUp, point, Quartz.kCGMouseButtonLeft),
    )
    print(f"held {identifier} at {point[0]:.0f},{point[1]:.0f} for {seconds}s")


if __name__ == "__main__":
    main()
