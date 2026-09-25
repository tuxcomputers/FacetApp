#!/usr/bin/env python3
"""Put text into a field of a running app, found by name anywhere in its accessibility tree.

    scripts/ax-set.py category-name-field "Admin"

Sets the element's AXValue, which is how to fill a field without synthetic keystrokes. Keystrokes go
wherever focus happens to be, and that is not necessarily the app being driven -- a named element
cannot be missed the same way.

An NSTextField's action fires on Return and on losing focus, not on this write, so something still has
to commit it: press the Save button beside it (scripts/ax-press.py), or move focus.

Exits non-zero when nothing matches. Needs accessibility permission for whatever runs it.
"""
import subprocess, sys
from ApplicationServices import (
    AXUIElementCreateApplication, AXUIElementCopyAttributeValue,
    AXUIElementSetAttributeValue, kAXErrorSuccess,
)

def attribute(element, name):
    err, value = AXUIElementCopyAttributeValue(element, name, None)
    return value if err == kAXErrorSuccess else None

def walk(element, wanted, depth=0):
    if depth > 25:
        return None
    if attribute(element, "AXIdentifier") == wanted:
        return element
    for child in (attribute(element, "AXChildren") or []):
        found = walk(child, wanted, depth + 1)
        if found is not None:
            return found
    return None

# `--focus` makes the element first responder before writing to it, which starts a real editing session.
#
# **Needed for any field nothing has clicked into.** Setting AXValue alone changes what a field displays
# without opening an edit, so the Return posted afterwards has nothing to commit and goes to whatever does
# have focus. A category name does not need this -- pressing the name calls makeFirstResponder itself --
# but a daily limit sits there unfocused, and typing into it silently did nothing until this existed.
focus = "--focus" in sys.argv
if focus:
    sys.argv.remove("--focus")

identifier, value = sys.argv[1], sys.argv[2]
pid = int(subprocess.check_output(["pgrep", "-x", "Facet"]).split()[0])
app = AXUIElementCreateApplication(pid)
target = None
for window in (attribute(app, "AXWindows") or []):
    target = walk(window, identifier)
    if target is not None:
        break
if target is None:
    print("not found", file=sys.stderr); sys.exit(1)
if focus:
    focused = AXUIElementSetAttributeValue(target, "AXFocused", True)
    if focused != kAXErrorSuccess:
        print(f"could not focus {identifier}: error {focused}", file=sys.stderr)
        sys.exit(1)

err = AXUIElementSetAttributeValue(target, "AXValue", value)
print(f"set {identifier} = {value!r}: {'ok' if err == kAXErrorSuccess else f'error {err}'}")
