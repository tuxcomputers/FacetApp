"""Finding and driving elements of a running GTK app, by name, over AT-SPI.

Shared by `at-press.py`, `at-dump.py`, `at-set.py`, `at-alert.py`, `at-hold.py` and `at-key.py`,
which are the Linux counterparts of the `ax-*.py` scripts and take the same arguments. What differs
between the two families is not the arguments but what the attributes *mean*, and it is worth saying
once here rather than in six files:

**On Linux the accessible name is the identifier and the description is the value.** That is this
app's own split and it is forced rather than chosen: a `GtkButton` reports its label as its
accessible name, so every control that wants an identifier has to overwrite it, and the value it
would otherwise show has to go somewhere -- `SettingsWidgets.identify(_:_:saying:)` puts it in the
description. On macOS `AXIdentifier`, `AXTitle` and `AXValue` are three separate attributes and
nothing has to share.

**So a check script must not call either family directly.** It says `platform_press`, and
`Tests/Scripted/platform.sh` decides what that means -- which is the same reason that file exists at
all. A check written against `--desc` would be asking for the label on one platform and the value on
the other.

`Tests/Methods.md` Method 20 is where the technique was measured.
"""

import sys
import time

import pyatspi

# How deep a walk goes before it decides it is lost. The Settings window's own tree is about twelve
# deep at the Categories tab; the limit is here because a cycle in an accessibility tree hangs the
# script rather than erroring, and a hang in a check reads as the app being slow.
MAX_DEPTH = 40


# How long to keep asking the registry for the app before giving up, and how often.
#
# **Because the answer is not stable while a modal dialogue is up.** `GtkDialoguePresenter` runs
# `gtk_dialog_run`, which spins a nested main loop, and during it the app drops out of the desktop
# enumeration for a moment -- so a single lookup answers "no application called FacetLinux is on the
# accessibility bus" about an app that is on screen with a dialogue waiting to be answered. Measured
# 2026-09-20: `press_title Rename` failed that way, the dialogue went unanswered, and the check read the
# unchanged name back out of the table and blamed the rename.
LOOKUP_SECONDS = 5.0
LOOKUP_INTERVAL = 0.2


def application(name="FacetLinux"):
    """The running app's accessible root, or a refusal saying which of the two things went wrong.

    **Not found and not running are different answers**, for the reason `platform_app_is_declared`
    returns three values rather than two: an app that is up but exposes no accessible tree is an
    at-spi bus problem, and reporting it as "the app is not running" sends somebody to the wrong file.

    **Polled rather than asked once**, for the reason above.
    """
    deadline = time.monotonic() + LOOKUP_SECONDS
    seen = []
    while True:
        try:
            desktop = pyatspi.Registry.getDesktop(0)
        except Exception as error:                                      # noqa: BLE001
            sys.exit(f"cannot reach the accessibility bus: {error}")
        seen = []
        for candidate in desktop:
            if candidate is None:
                continue
            seen.append(candidate.name)
            if candidate.name == name:
                return candidate
        if time.monotonic() >= deadline:
            break
        time.sleep(LOOKUP_INTERVAL)

    # **What was there instead**, because the two ways this fails want different files looked at: an empty
    # list is the bridge not running, and a list without this app in it is the app not running.
    listed = ", ".join(repr(candidate) for candidate in seen) or "nothing at all"
    sys.exit(
        f"no application called {name} is on the accessibility bus after {LOOKUP_SECONDS:g}s.\n"
        f"  the bus lists: {listed}\n"
        f"  if it is running, at-spi may not be: check `pgrep -x {name}` first,\n"
        f"  and that GTK_MODULES does not exclude the atk bridge."
    )


def walk(node, depth=0, whole_tree=True):
    """Every node at or under this one, parents before children, in the order they are drawn.

    **`whole_tree=False` does not descend into a notebook page that is not selected**, and that is
    what makes a search by name unambiguous rather than merely usually right. GTK keeps every page
    of a notebook built and in the tree whether or not it is on show, and this window has the same
    identifier on more than one of them on purpose: `create-category`, `category-name-field` and
    `save-category` are the Categories tab's create control and the Faces tab's, which are the same
    control doing the same job. A plain walk finds whichever was built first.

    **Showing-state is not enough to tell them apart**, which is worth recording because it is the
    obvious fix and it fails quietly. Measured 2026-09-20 with the Faces tab selected: its
    `create-category` reads showing, the Categories tab's does not, and so far so good -- but
    `category-name-field` reads *not* showing on both, because that field is revealed by pressing
    Create and is hidden until then. So a rule of "prefer the showing one" silently falls back to
    the wrong tab for exactly the elements a check is about to type into.

    Pruning the unselected pages cuts those subtrees and nothing else, which is also what a person
    looking at the window sees.
    """
    if node is None or depth > MAX_DEPTH:
        return
    yield node, depth
    if not whole_tree and _is_unselected_page(node):
        return
    try:
        children = list(node)
    except Exception:                                                   # noqa: BLE001
        # A node that was destroyed while the walk was in progress. The window rebuilds rows as the
        # database changes underneath it, so this is ordinary rather than exceptional, and the rest
        # of the tree is still worth reading.
        return
    for child in children:
        yield from walk(child, depth + 1, whole_tree)


def _is_unselected_page(node):
    try:
        if node.getRoleName() != "page tab":
            return False
        return not node.getState().contains(pyatspi.STATE_SELECTED)
    except Exception:                                                   # noqa: BLE001
        return False


def find(root, matches, whole_tree=False):
    """The first node the predicate accepts, on the tab that is on show. None if there is none.

    Searched for rather than pathed to, for the same reason as `ax-press.py`: a path breaks the
    moment a container is added between two elements, and it breaks by finding the wrong element
    rather than nothing. Names do not move.
    """
    for node, _ in walk(root, whole_tree=whole_tree):
        try:
            if matches(node):
                return node
        except Exception:                                               # noqa: BLE001
            continue
    return None


def by_name(wanted):
    return lambda node: node.name == wanted


def by_description(wanted):
    return lambda node: node.description == wanted


def role_of(node):
    try:
        return node.getRoleName()
    except Exception:                                                   # noqa: BLE001
        return "?"


def require(root, matches, described):
    """Find it, or exit non-zero saying what was looked for.

    **Exits rather than returning None** so that a missing element can never be mistaken for an
    action that did nothing -- which is the distinction every check downstream of this is drawing.
    """
    node = find(root, matches)
    if node is not None:
        return node
    # **Says which of the two it is.** A name that exists only on a tab that is not on show is a
    # check that forgot to switch tabs, and it is a different fault from a name that is nowhere --
    # one is the script's mistake and the other is the app's. Reporting them the same way sends
    # somebody to read the wrong file, which is the whole reason `platform_app_is_declared` answers
    # three things rather than two.
    elsewhere = find(root, matches, whole_tree=True)
    if elsewhere is not None:
        sys.exit(
            f"{described} is in the tree but not on the tab that is on show.\n"
            f"  switch to its tab first, then press it."
        )
    sys.exit(f"nothing in the tree matches {described}")


def extents(node):
    """Where the node is on the screen, for the two scripts that need a real pointer event."""
    component = node.queryComponent()
    return component.getExtents(pyatspi.DESKTOP_COORDS)
