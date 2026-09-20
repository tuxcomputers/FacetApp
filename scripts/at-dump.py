#!/usr/bin/env python3
"""Print a running GTK app's accessibility tree, which is what a UI script sees.

    scripts/at-dump.py                      # the whole of FacetLinux
    scripts/at-dump.py --frames             # with each element's position and size
    scripts/at-dump.py --app "Some App"     # somebody else's tree

The Linux counterpart of `ax-dump.py`, and it answers the same question: is everything named? Every
control the app builds should carry an identifier, and this is what proves it rather than assuming it.

**Name is the identifier and description is the value here**, which is the opposite way round from
nothing on macOS but is not the same as macOS -- see `atspi_tree.py` for why the app has to put them
that way and why a check script goes through `platform.sh` rather than calling this directly.

**A window that is not open is not in the tree at all.** An app with only a tray icon dumps as a
single `application` node, which is correct and has already been mistaken for a broken bridge: open
the window first (`scripts/tray-menu.py --press 'Settings…'`).
"""

import argparse
import sys

sys.path.insert(0, __file__.rsplit("/", 1)[0])

import pyatspi                                                          # noqa: E402

from atspi_tree import application, extents, role_of, walk              # noqa: E402

# **The same line shape as `ax-dump.py`, attribute for attribute**, and that is load-bearing rather
# than a courtesy. `lib.sh` does not read this output as prose: `element` greps `id=X `, `on_tab`
# counts `id=X` on a word boundary, `window_width` pulls the number out of `size=w:N`, and `tree_has`
# matches whole strings against it. A dump that said the same things in a different shape would leave
# every one of those helpers answering no on a perfectly correct window -- and `tree_has`'s own comment
# records what a silent no costs: `settings_is_open` answering false opens a second window, and
# `wait_for_element` answering false polls until it times out and blames the app.
#
# So the mapping is fixed, and the one that needed a decision is the middle one:
#
#   id=      the accessible name, which is this app's identifier on this platform
#   title=   the words actually drawn, where they are not simply the identifier again
#   value=   the accessible description -- **because on Linux that is where the value is**
#   disabled only when it is, for the same reason the macOS one prints it that way
#
# **There is no `desc=` here, and leaving it out is the honest answer rather than an omission.** On
# macOS `AXDescription` and `AXValue` are two attributes holding two different things, so a dump prints
# both. Here there is one attribute doing both jobs: a `GtkButton` reports its label as its accessible
# *name*, so every control that wants an identifier overwrites it, and the value it would have shown
# goes into the description -- which is what `SettingsWidgets.identify(_:_:saying:)` does. Printing that
# as `desc=` as well would put the same string on the line twice under two names, and a check asserting
# something is *absent* would then be asking a question with two answers.


def text_of(node):
    """The words the control actually draws, read through the Text interface.

    Not the name, which is the identifier here, and not the description, which is the value.
    """
    try:
        return node.queryText().getText(0, -1)
    except Exception:                                                   # noqa: BLE001
        return ""


def number_of(node):
    try:
        return node.queryValue().currentValue
    except Exception:                                                   # noqa: BLE001
        return None


def is_disabled(node):
    try:
        return not node.getState().contains(pyatspi.STATE_SENSITIVE)
    except Exception:                                                   # noqa: BLE001
        return False


# What GTK answers for a widget it has sized but never placed. Compared against rather than equalled,
# because the two coordinates are computed and a page scrolled away can be a little under it.
UNPLACED = -2_000_000_000


def compact(number):
    """`45.0` down to `45`, so a spin button reads the way the field does."""
    if number == int(number):
        return str(int(number))
    return f"{number:g}"


def describe(node, with_frames):
    parts = []
    if node.name:
        parts.append(f"id={node.name}")

    words = text_of(node)
    if words and words != node.name:
        parts.append(f"title={words}")

    # The description first, because it is where this app puts a control's value; a spin button that
    # has a real Value interface as well answers with the number, and both never appear at once.
    if node.description:
        parts.append(f"value={node.description}")
    else:
        number = number_of(node)
        if number is not None:
            parts.append(f"value={compact(number)}")

    if is_disabled(node):
        parts.append("disabled")

    if with_frames:
        try:
            box = extents(node)
            # **A control on a notebook page that is not on show has a size but no position**, and the
            # position it reports is `INT_MIN` rather than anything obviously absent (measured
            # 2026-09-20: every child of the unselected Faces page came back `x:-2147483648` while
            # reporting a perfectly ordinary 598x550). GTK has allocated it a size and never placed it.
            #
            # Printed as `pos=unplaced` rather than as the number, because the number is the shape of a
            # coordinate and would be used as one -- `at-hold.py` would post a real pointer event at the
            # far corner of the coordinate space, which lands on nothing and reports a hold that happened.
            if box.x <= UNPLACED or box.y <= UNPLACED:
                parts.append("pos=unplaced")
            else:
                parts.append(f"pos=x:{box.x} y:{box.y}")
            parts.append(f"size=w:{box.width} h:{box.height}")
        except Exception:                                               # noqa: BLE001
            pass

    return "  ".join(parts) if parts else "<unnamed>"


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--app", default="FacetLinux", help="the application to dump")
    parser.add_argument("--frames", action="store_true", help="include position and size")
    parser.add_argument(
        "--all-tabs",
        action="store_true",
        help="include notebook pages that are not on show (off by default)",
    )
    arguments = parser.parse_args()

    root = application(arguments.app)

    # **Each window, rather than the application node**, which is what `ax-dump.py` prints and so is what
    # `lib.sh` is written against. The application node carries the process name in the same `id=` slot
    # the controls use, so dumping it puts a line reading `id=FacetLinux` at the top of every tree -- and
    # it reports itself insensitive, which would have `element` and `tree_has` answering about the app
    # where they were asked about a control.
    windows = [child for child in root if child is not None and role_of(child) in ("frame", "dialog", "alert")]
    if not windows:
        print(f"{arguments.app} is running with no windows open", file=sys.stderr)
    # **Pages that are not on show are left out, the way every other tool here leaves them out.**
    #
    # `lib.sh` reads this output as the window: `element` takes the *first* line carrying an identifier, and this
    # window has the same identifier on more than one tab on purpose -- `create-category`, `category-name-field`
    # and `save-category` are the Categories tab's create control and the Faces tab's. Dumping both meant
    # `element category-name-field` answered about the tab nobody was looking at, so a paste that had landed
    # perfectly was read back as an empty field (measured 2026-09-20).
    #
    # `--all-tabs` is there for looking at the whole window by hand, which is the only thing that wants it.
    for window in windows:
        for node, depth in walk(window, whole_tree=arguments.all_tabs):
            print(f"{'  ' * depth}{role_of(node)}  {describe(node, arguments.frames)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
