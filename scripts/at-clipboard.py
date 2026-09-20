#!/usr/bin/env python3
"""Put text on the X clipboard and hold it there, which is what `pbcopy` does on macOS.

    scripts/at-clipboard.py --set "Pasted 1789872761"
    scripts/at-clipboard.py --set "text" --hold 60     # keep it for a minute rather than the default

**Why this is a script and not one line.** On X11 the clipboard is not a place, it is a *protocol*: the program
that copied keeps the text and hands it over when somebody asks. There is no daemon holding it. So a process that
sets the clipboard and exits immediately takes the text with it, and the paste that follows finds nothing -- which
is exactly the shape of a check that fails while looking like the app ignored the keystroke.

`pbcopy` gets away with being a one-liner because macOS has a pasteboard server that keeps the bytes.

So this detaches, owns the selection, and runs a GLib main loop long enough for the check to paste. It prints the
child's pid and returns at once, so a check does not wait on it.

**`xclip` and `xsel` do exactly this and are not installed on the Linux box** (measured 2026-09-20). This needs
only pygobject, which is already required for driving the window at all, so it is one fewer thing somebody has to
install before the suite can run.
"""

import argparse
import os
import sys


def hold(text, seconds):
    """Own the clipboard until the time is up. Runs in the detached child.

    **GTK is imported here and not at the top**, because importing it starts threads and forking a
    multi-threaded process is how a child deadlocks holding a lock the parent's other thread had. The parent
    of this fork does nothing but print a pid, so it has no need of GTK at all.
    """
    import gi

    gi.require_version("Gtk", "3.0")
    gi.require_version("Gdk", "3.0")
    from gi.repository import Gdk, GLib, Gtk

    Gtk.init([])
    clipboard = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
    clipboard.set_text(text, -1)
    # **Stored, so the text survives this process after all** -- where the display server supports it. Belt and
    # braces rather than the mechanism: a clipboard manager may take a copy, and most desktops run one.
    clipboard.store()
    loop = GLib.MainLoop()
    GLib.timeout_add_seconds(seconds, loop.quit)
    loop.run()


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--set", required=True, metavar="TEXT", help="what to put on the clipboard")
    parser.add_argument("--hold", type=int, default=120, help="seconds to keep it (default 120)")
    arguments = parser.parse_args()

    # Detached, so the caller carries on. A double fork would orphan it to init; one is enough here because the
    # caller is a shell script that is not going to wait on its children.
    pid = os.fork()
    if pid == 0:
        os.setsid()
        # The child must not write to the run's log: everything it could say, it says by holding the selection.
        devnull = os.open(os.devnull, os.O_RDWR)
        os.dup2(devnull, 0)
        os.dup2(devnull, 1)
        hold(arguments.set, arguments.hold)
        os._exit(0)

    print(f"holding the clipboard in pid {pid}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
