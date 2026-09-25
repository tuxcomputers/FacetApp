#!/usr/bin/env python3
"""Post a real keystroke, with modifiers, to whatever holds focus.

    scripts/ax-key.py v --command          # paste
    scripts/ax-key.py a --command          # select all
    scripts/ax-key.py return               # commit an inline edit

**Because a shortcut is not an action.** Accessibility can press a control by name (`ax-press.py`) and can
write a field's value outright (`ax-set.py`), and neither of those goes anywhere near the path a keyboard
shortcut takes: `NSApplication.sendEvent` offers a key-down to `mainMenu.performKeyEquivalent` first, and
only the item found there turns the keystroke into `paste(_:)` on the field editor. So ⌘V is only provable
by posting ⌘V -- `MainMenu`'s unit tests can say the item exists and carry the right key, and that is the
whole of what they can say.

This posts a real key event on the real keyboard. It goes wherever focus is, so the app is activated first
and the caller has to have put focus in the right place already (opening an inline edit does that; see
`Tests/Methods.md` Method 10). Nothing here belongs in a run somebody is using the machine during.

Exits non-zero, with the reason, on anything that stops the key going out -- the app not running, a key
nobody can name -- rather than posting nothing and saying it worked.
"""

import subprocess
import sys
import time

import Quartz
from AppKit import NSWorkspace

# The virtual key codes for what a script actually needs. Named rather than numeric at the call site,
# because `36` in a test script is a number nobody can check without a table.
KEYS = {
    "a": 0, "c": 8, "v": 9, "x": 7, "z": 6,
    "return": 36, "tab": 48, "space": 49, "delete": 51,
}

# Each modifier is a flag **and a key**, and posting only the flag is what broke a later Return.
#
# A chord set up as "flag on the key-down and key-up" looks right and works, and leaves the session
# believing the modifier is still held: nothing ever said it came back up. `CGEventCreateKeyboardEvent`
# with a NULL source inherits the session's flags, so the *next* posted key -- `press_return`'s, fifteen
# checks and two minutes later -- went out as command-Return and committed nothing. Measured on run 145:
# the field was focused and held the right text, the app was frontmost, and no rename row was written.
#
# So the modifier key is pressed and released for real, around the chord, and the flags are set on every
# event this posts. That leaves the session where it found it.
MODIFIERS = {
    "--command": (Quartz.kCGEventFlagMaskCommand, 55),
    "--shift": (Quartz.kCGEventFlagMaskShift, 56),
    "--option": (Quartz.kCGEventFlagMaskAlternate, 58),
    "--control": (Quartz.kCGEventFlagMaskControl, 59),
}

# What the session must be back to once a chord is finished. Anything left over here is the bug above.
SETTLED = 0

# **The only bits worth asking about are the ones this can press.** `CGEventSourceFlagsState` does not
# answer with modifiers alone: measured with nothing posted and no key held, it returns 0x20000000, which
# is not Command (0x00100000) nor any other documented mask. Comparing the raw state against nothing
# therefore fails every time, which is what it did on the first run of the check below. Caps lock is left
# out along with it -- that one is the user's, and a chord has no business reading it either way.
MODIFIER_BITS = (
    Quartz.kCGEventFlagMaskCommand
    | Quartz.kCGEventFlagMaskShift
    | Quartz.kCGEventFlagMaskControl
    | Quartz.kCGEventFlagMaskAlternate
)


def main():
    if len(sys.argv) < 2:
        sys.exit(f"usage: ax-key.py <{'|'.join(KEYS)}> [{' '.join(MODIFIERS)}] [--app Facet]")

    key = sys.argv[1].lower()
    if key not in KEYS:
        sys.exit(f"no key code for {key!r}; known keys are {', '.join(sorted(KEYS))}")

    arguments = sys.argv[2:]
    app_name = "Facet"
    if "--app" in arguments:
        index = arguments.index("--app")
        if index + 1 >= len(arguments):
            sys.exit("--app needs a name after it")
        app_name = arguments[index + 1]
        del arguments[index:index + 2]

    flags = 0
    held = []
    for argument in arguments:
        if argument not in MODIFIERS:
            sys.exit(f"unknown option {argument!r}; modifiers are {', '.join(sorted(MODIFIERS))}")
        mask, code = MODIFIERS[argument]
        flags |= mask
        held.append(code)

    running = any(
        a.localizedName() == app_name for a in NSWorkspace.sharedWorkspace().runningApplications()
    )
    if not running:
        sys.exit(f"{app_name} is not running")

    # The key goes to whatever is focused, so the app has to be in front first -- otherwise it lands in
    # whatever the caller is being driven from, which is the one failure that looks like the app ignoring it.
    subprocess.run(["osascript", "-e", f'tell application "{app_name}" to activate'], check=False)
    time.sleep(0.4)

    def post(code, down, with_flags):
        event = Quartz.CGEventCreateKeyboardEvent(None, code, down)
        # Set explicitly on every event, including the ones that clear. An event built from a NULL source
        # otherwise inherits whatever the session is carrying, which is the whole fault being avoided here.
        Quartz.CGEventSetFlags(event, with_flags)
        Quartz.CGEventPost(Quartz.kCGHIDEventTap, event)
        time.sleep(0.05)

    # Down, the chord, up. The modifier is a real press and a real release, so the session ends holding
    # nothing -- see MODIFIERS.
    for code in held:
        post(code, True, flags)
    for down in (True, False):
        post(KEYS[key], down, flags)
    for code in reversed(held):
        post(code, False, SETTLED)

    named = "+".join([argument.lstrip("-") for argument in arguments] + [key])

    # **Checked, not assumed.** A modifier left held does not fail here: it fails at the next posted key,
    # in another script, as that key doing nothing -- which is how run 145 spent its budget blaming a
    # rename. Reporting it at the point it happens is the whole of the difference.
    time.sleep(0.1)
    state = Quartz.CGEventSourceFlagsState(Quartz.kCGEventSourceStateCombinedSessionState)
    left = state & MODIFIER_BITS
    if left != SETTLED:
        sys.exit(f"posted {named}, but the session is still holding modifiers (flags {left:#x})")

    print(f"posted {named} to {app_name}")


if __name__ == "__main__":
    main()
