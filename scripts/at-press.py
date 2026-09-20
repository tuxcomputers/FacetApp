#!/usr/bin/env python3
"""Press one control of a running GTK app, found by name anywhere in its accessibility tree.

    scripts/at-press.py create-category         # by identifier, which is the accessible name here
    scripts/at-press.py --desc Break            # by the value it shows, for rows addressed by content
    scripts/at-press.py --tab Categories        # select a notebook tab, which has no action of its own
    scripts/at-press.py --app FacetLinux quit-app

The Linux counterpart of `ax-press.py`, taking the same arguments for the same jobs. It performs the
control's first accessible action, which is what a click does: pressing a button, ticking a check box,
folding a section.

**Searched for rather than pathed to**, and the search does not descend into a notebook tab that is
not on show -- see `atspi_tree.walk` for why showing-state alone is not enough to tell two tabs'
copies of one identifier apart.

**A tab is selected, not pressed.** A `page tab` implements no Action interface at all, so
`queryAction()` raises `NotImplementedError` on it; the `page tab list` above it implements Selection.
That is `Tests/Methods.md` Method 20, and `--tab` is it.

Exits non-zero when nothing matches, so a check can tell a missing control from a press that did
nothing.
"""

import argparse
import sys

sys.path.insert(0, __file__.rsplit("/", 1)[0])

from atspi_tree import (                                                # noqa: E402
    application,
    by_description,
    by_name,
    find,
    require,
    role_of,
)


def select_tab(root, label):
    """Select the notebook page whose tab carries this label."""
    tabs = require(root, lambda node: role_of(node) == "page tab list", "a notebook")
    pages = [child for child in tabs if child is not None]
    for index, page in enumerate(pages):
        if page.name == label:
            tabs.querySelection().selectChild(index)
            print(f"selected tab {label!r}")
            return 0
    named = ", ".join(repr(page.name) for page in pages)
    sys.exit(f"no tab called {label!r}. The notebook has: {named}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("name", nargs="?", help="the identifier to press")
    parser.add_argument("--app", default="FacetLinux", help="the application to drive")
    parser.add_argument("--desc", action="store_true", help="match the description instead")
    parser.add_argument("--tab", help="select this notebook tab rather than pressing anything")
    arguments = parser.parse_args()

    root = application(arguments.app)

    if arguments.tab:
        return select_tab(root, arguments.tab)

    if not arguments.name:
        parser.error("give a name to press, or --tab to select a tab")

    wanted = arguments.name
    if arguments.desc:
        node = require(root, by_description(wanted), f"description {wanted!r}")
    else:
        node = require(root, by_name(wanted), f"name {wanted!r}")

    try:
        action = node.queryAction()
    except NotImplementedError:
        # **Named, rather than reported as a press that did nothing.** A control with no action is a
        # control this script cannot drive, and saying which role it turned out to be is what tells
        # somebody whether they wanted `--tab`, `at-set.py` or a real pointer event.
        sys.exit(
            f"{wanted!r} is a {role_of(node)}, which exposes no accessible action.\n"
            f"  a notebook tab needs --tab; a text field needs at-set.py;\n"
            f"  a stepper arrow that has to be held needs at-hold.py."
        )

    if action.nActions < 1:
        sys.exit(f"{wanted!r} is a {role_of(node)} and offers no actions to perform")

    # **The role is read before the press, not after.** Pressing a control often rebuilds the pane it is in, and a
    # node read back afterwards is a handle to a destroyed widget: AT-SPI answers `invalid`, which reads as the press
    # having gone somewhere odd when it went exactly where it should. Seen 2026-09-20 on the Device tab\'s Scan
    # button, which redraws its own row.
    role = role_of(node)
    action.doAction(0)
    print(f"pressed {wanted!r} ({role})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
