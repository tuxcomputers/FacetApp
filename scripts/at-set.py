#!/usr/bin/env python3
"""Put text into a field of a running GTK app, found by name anywhere in its accessibility tree.

    scripts/at-set.py category-name-field "Admin"
    scripts/at-set.py device-autopause-field 45      # a spin button takes a number

The Linux counterpart of `ax-set.py`. Writes the field's contents outright rather than sending
keystrokes, for the same reason: keystrokes go wherever the X focus happens to be, which is not
necessarily the app being driven, and a named element cannot be missed that way.

**Two interfaces, and which one a control has is what it is.** An entry implements EditableText and
takes a string; a spin button implements Value and takes a number, and setting its value is what a
spin button treats as settled. Asked for in that order, and a control with neither says so rather
than reporting a write that went nowhere.

**Writing is not committing.** A `GtkEntry`'s `activate` fires on Return and on losing focus, not on
this write, so something still has to commit it: press the Save button beside it (`at-press.py`), or
send a real Return (`at-key.py`). That is the same as macOS and for the same reason.

Exits non-zero when nothing matches.
"""

import argparse
import subprocess
import sys
import time

sys.path.insert(0, __file__.rsplit("/", 1)[0])

import gi                                                               # noqa: E402

gi.require_version("Gdk", "3.0")

import pyatspi                                                          # noqa: E402
from gi.repository import Gdk                                           # noqa: E402

from atspi_tree import application, by_name, require, role_of           # noqa: E402

# The Settings window's title, which is what wmctrl activates by. It is the frame's accessible name too.
WINDOW_TITLE = "Facet Settings"


# **The roles a Slint text field answers with.** AccessKit's AT-SPI bridge (`accesskit_unix` 0.22) publishes
# no EditableText interface at all, so a Slint `LineEdit` can be read but not written through the bus, and
# the only way in is the way a person uses: focus it and type.
TYPED_ROLES = {"entry", "text", "password text"}


def type_into(node, name, text, allow_cut=False):
    """Type `text` into a field that cannot be written, and read back what it now holds.

    **Real keystrokes, so this is the sharp case in CLAUDE.md**: XTEST delivers to whatever holds the X
    focus. So the app's window is activated by title first, the field is given focus through its own
    Component interface, and what the field holds afterwards is read back and compared. A key that went
    somewhere else leaves the field unchanged, and that is refused loudly rather than reported as set.
    """
    activated = subprocess.run(["wmctrl", "-a", WINDOW_TITLE], capture_output=True, text=True)
    if activated.returncode != 0:
        sys.exit(f"could not bring {WINDOW_TITLE!r} to the front (wmctrl exit {activated.returncode}): "
                 f"{activated.stderr.strip() or 'no output'}")
    time.sleep(0.3)
    try:
        node.queryComponent().grabFocus()
    except Exception as error:                                          # noqa: BLE001
        sys.exit(f"{name!r} would not take focus, so nothing can be typed into it: {error}")
    time.sleep(0.2)

    # Select everything first so the typing replaces rather than appends. Control is released in a
    # finally, as at-key.py does, so a failure cannot leave the session holding it.
    control = _keycode(0xFFE3)
    pyatspi.Registry.generateKeyboardEvent(control, None, pyatspi.KEY_PRESS)
    try:
        pyatspi.Registry.generateKeyboardEvent(ord("a"), None, pyatspi.KEY_SYM)
    finally:
        pyatspi.Registry.generateKeyboardEvent(control, None, pyatspi.KEY_RELEASE)
    if text:
        pyatspi.Registry.generateKeyboardEvent(0, text, pyatspi.KEY_STRING)
    else:
        # Emptying is typing nothing over a selection, which XTEST cannot send as a string: BackSpace
        # deletes what the select-all above selected.
        pyatspi.Registry.generateKeyboardEvent(0xFF08, None, pyatspi.KEY_SYM)

    # **Read back, not trusted.** The field clamps its own length, and a keystroke that landed elsewhere
    # leaves it holding what it held before; both are a write that did not happen as asked.
    deadline = time.monotonic() + 3
    held = None
    while time.monotonic() < deadline:
        time.sleep(0.2)
        try:
            held = node.queryText().getText(0, -1)
        except Exception:                                               # noqa: BLE001
            held = node.name
        if held == text:
            print(f"typed {text!r} into {name!r}")
            return 0
    # **A cut is only accepted when it was asked for**, and only as a leading part of what was typed: a field that
    # holds to a length is doing its job, and one holding anything else is a keystroke that went astray.
    if allow_cut and held and text.startswith(held):
        print(f"typed {text!r} into {name!r}, and it kept the first {len(held)}: {held!r}")
        return 0
    sys.exit(f"typed {text!r} into {name!r}, but it holds {held!r}")


def _keycode(keysym):
    display = Gdk.Display.get_default()
    if display is None:
        sys.exit("no X display, so no key can be sent")
    found, entries = Gdk.Keymap.get_for_display(display).get_entries_for_keyval(keysym)
    if not found or not entries:
        sys.exit(f"no key on this keyboard produces keyval {keysym:#x}")
    return entries[0].keycode


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("name", help="the identifier of the field")
    parser.add_argument("value", help="what to put in it")
    parser.add_argument("--app", default="facet-linux", help="the application to drive")
    parser.add_argument("--allow-cut", action="store_true",
                        help="accept a text field keeping only the start of what was typed, as a length limit does")
    arguments = parser.parse_args()

    root = application(arguments.app)
    node = require(root, by_name(arguments.name), f"name {arguments.name!r}")

    try:
        editable = node.queryEditableText()
    except NotImplementedError:
        editable = None

    if editable is not None:
        if not editable.setTextContents(arguments.value):
            sys.exit(f"{arguments.name!r} refused the text {arguments.value!r}")
        print(f"set {arguments.name!r} to {arguments.value!r}")
        return 0

    try:
        value = node.queryValue()
    except NotImplementedError:
        value = None

    if value is None:
        if role_of(node) in TYPED_ROLES:
            return type_into(node, arguments.name, arguments.value, arguments.allow_cut)
        sys.exit(
            f"{arguments.name!r} is a {role_of(node)}, which is neither editable text nor a value.\n"
            f"  a button is pressed with at-press.py; a label cannot be written at all."
        )

    try:
        number = float(arguments.value)
    except ValueError:
        sys.exit(f"{arguments.name!r} is a {role_of(node)} and takes a number, not {arguments.value!r}")
    value.currentValue = number
    # **Read back rather than trusted**, which is the same rule this project applies to the cube and to
    # the database: a spin button clamps to its own range, so a write that reported nothing and landed
    # somewhere else is exactly the disagreement worth catching here rather than three checks later.
    #
    # **Polled, briefly, not read once.** AccessKit publishes the new value on the frame after the write, so a
    # read straight away answers the old one. Measured 2026-09-25 on a Slint SpinBox: `set to 0` when asked for
    # 45, with 45 already in the table the field writes to.
    deadline = time.monotonic() + 3
    while True:
        settled = value.currentValue
        if abs(settled - number) <= 1e-9 or time.monotonic() >= deadline:
            break
        time.sleep(0.2)
    print(f"set {arguments.name!r} to {settled:g}")
    if abs(settled - number) > 1e-9:
        sys.exit(f"  but it was asked for {number:g}: the control clamped it")
    return 0


if __name__ == "__main__":
    sys.exit(main())
