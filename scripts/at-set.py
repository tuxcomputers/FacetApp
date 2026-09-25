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
import sys

sys.path.insert(0, __file__.rsplit("/", 1)[0])

from atspi_tree import application, by_name, require, role_of           # noqa: E402


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("name", help="the identifier of the field")
    parser.add_argument("value", help="what to put in it")
    parser.add_argument("--app", default="FacetLinux", help="the application to drive")
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
    settled = value.currentValue
    print(f"set {arguments.name!r} to {settled:g}")
    if abs(settled - number) > 1e-9:
        sys.exit(f"  but it was asked for {number:g}: the control clamped it")
    return 0


if __name__ == "__main__":
    sys.exit(main())
