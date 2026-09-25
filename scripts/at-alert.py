#!/usr/bin/env python3
"""Print the buttons of the dialogue a running GTK app has up, one per line, in the order drawn.

    scripts/at-alert.py                     # Reactivate / Create new one / Cancel
    scripts/at-alert.py --message           # the dialogue's text instead of its buttons

The Linux counterpart of `ax-alert.py`, written for the same checks and turning on the same thing:
a button being **absent**. Creating a category whose name one retired category holds offers three
answers; when several hold it, Reactivate is not offered at all, because nothing on a button could
say which one to bring back. Asserting that means counting the dialogue's own buttons, and grepping
the window's tree cannot -- a Cancel anywhere else on the tab would be counted, and a missing button
looks identical to a tree read a moment too early.

**A dialogue is a `dialog` frame of its own here, not a sheet hanging off the window.**
`GtkDialoguePresenter` runs `gtk_dialog_run`, which spins a nested main loop and puts a separate
top-level in the tree. It is still perfectly drivable while that loop is spinning, which is the part
worth knowing: find the button by its title -- `Dialogue.choices`, so the core's own wording -- and
`at-press.py` it.

Exits non-zero when no dialogue is up, which is what tells a check "the dialogue never appeared"
apart from "the dialogue appeared with no buttons".
"""

import argparse
import sys

sys.path.insert(0, __file__.rsplit("/", 1)[0])

from atspi_tree import application, find, role_of, walk                 # noqa: E402

# What GTK calls a top-level that is not the main window. `alert` is what a `GtkMessageDialog`
# reports and `dialog` is what a plain `GtkDialog` does; this app builds both, so both count.
DIALOGUE_ROLES = ("dialog", "alert")


def text_of(node):
    try:
        text = node.queryText().getText(0, -1)
    except Exception:                                                   # noqa: BLE001
        text = ""
    return text or node.description or node.name


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--app", default="FacetLinux", help="the application to read")
    parser.add_argument("--message", action="store_true", help="print the text, not the buttons")
    arguments = parser.parse_args()

    root = application(arguments.app)
    # **Searched over the whole tree**, unlike every other script here: a dialogue is a top-level of
    # its own and so is never inside a notebook page, and pruning unselected tabs would be answering a
    # question nobody asked while risking the one that was.
    dialogue = find(root, lambda node: role_of(node) in DIALOGUE_ROLES, whole_tree=True)
    if dialogue is None:
        sys.exit("no dialogue is up")

    if arguments.message:
        # Every piece of static text in it, which is how the wording is asserted. A message dialogue
        # draws its primary and secondary text as two labels, and a check usually wants both.
        for node, _ in walk(dialogue):
            if role_of(node) == "label":
                words = text_of(node)
                if words:
                    print(words)
        return 0

    for node, _ in walk(dialogue):
        if role_of(node) == "push button":
            print(text_of(node))
    return 0


if __name__ == "__main__":
    sys.exit(main())
