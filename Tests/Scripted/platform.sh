#!/bin/bash
# Which machine this is, and every operation that differs between them. Sourced, never run.
#
# **One variable, `PLATFORM`, and everything that varies hangs off it.** The scripted suite drives a real
# app through a real window, and that is the half of it no two operating systems share: the same *step* --
# quit the app, press a control, read the tray -- reaches it by a different *method* on each. What must not
# vary is the step, because a check that is written twice is two checks that can drift apart, and the whole
# claim of this suite is that it says what the app does rather than what one platform's version of it does.
#
# So a script says `platform_quit_app`, and this file decides what that means. Nothing above this line ever
# asks which machine it is on.
#
# **The values, not only the functions.** Where the app keeps its database, what the binary is called and
# what the run's stamp is named are platform facts too, and having them here rather than spelled out in
# three files is what stops one of them being updated and the others not.
#
# Sourced by `run.sh`, `lib.sh` and `testlog.sh`, in any order and more than once, so it is idempotent.

# Sourced more than once in a run -- `run.sh` takes it and so does `lib.sh` -- so it does its work once.
# **Guarded on a name of its own rather than on `PLATFORM`**, because guarding on the answer would let an
# exported `PLATFORM=mac` walk in from the environment and quietly redirect every path in the suite at a
# directory that does not exist. What this file decides, it decides from `uname`.
[ -n "${_PLATFORM_SH_SOURCED:-}" ] && return 0
_PLATFORM_SH_SOURCED=1

# **Ported from the Swift suite on 2026-09-21 and only partly exercised.** The paths, the crate names,
# the toolchain finder and the build were rewritten for cargo and checked. Everything below them that
# drives a window or the radio came across unchanged, because it goes through the ax-*/at-* scripts
# rather than through anything Swift, and docs/rust-port.md records those working against Slint. None
# of it has been run end to end here, because the checks it serves have not been written yet.
#
# **This file is the one place that decides where things are.** scripts/run.sh and
# scripts/switch-database.sh both source it rather than working the paths out again: a script that
# hardcoded the macOS directory once already resolved every path under a directory that does not exist
# on Linux, and reported the absence of a database nobody had asked about.

# **`PLATFORM_OVERRIDE` is for exercising the other platform's values, and nothing else.** It cannot make
# the other platform's *methods* work -- `ax-press.py` is not going to run here -- so it is good for
# checking that the paths and names come out right and is a lie in every other respect. Nothing in the
# suite sets it.
case "${PLATFORM_OVERRIDE:-$(uname -s)}" in
    mac|Darwin) PLATFORM=mac ;;
    linux|Linux) PLATFORM=linux ;;
    *)
        # **Loudly, and not as a default.** Guessing that an unknown system is close enough to one of these
        # would drive a real app with the wrong method and report whatever came of it as a test result.
        echo "This suite knows how to drive macOS and Linux. ${PLATFORM_OVERRIDE:-$(uname -s)} is neither," >&2
        echo "so it will not guess." >&2
        return 1
        ;;
esac

# **Removed 2026-09-20, having been called from nowhere for some time.** `platform_not_yet` existed for
# the functions below that had no Linux half yet, and there are none left: item 11 built the app and the
# window-driving section further down is the rest of it. What is genuinely absent on this platform is now
# refused *at the place it is absent*, saying why rather than pointing at a list -- `platform_click_right`
# is the one, and its refusal explains that an AppIndicator has no right half rather than implying
# somebody forgot to write one.

# ---------------------------------------------------------------------------- where things are

case "$PLATFORM" in
    mac)
        # `~/Library/Application Support/Facet`, which is what the platform's own application-support
        # lookup answers there.
        #
        # **This is ours now.** It was the Swift app's until 2026-09-21, when that app was renamed to
        # TimeFlip and its directory moved with it, leaving this name free. Nothing of the Swift app's
        # is shared any more except the codesigning identity and the Google project.
        SUPPORT="$HOME/Library/Application Support/Facet"
        # **No bundle yet.** There is no `.app`, so the binary is the whole of it and `APP` is empty,
        # which is the shape Linux has always had. A bundle is a packaging job that has not been done.
        APP=""
        CRATE="facet-mac"
        BINARY="target/debug/$CRATE"
        PROCESS_NAME="$CRATE"
        # What macOS calls the running app, which is the binary's name while there is no bundle. Every
        # `scripts/ax-*.py` and `status-item-click.py` looks the app up by it.
        export FACET_APP_NAME="$PROCESS_NAME"
        # The Swift app, which is still the one recording real time. It was renamed to TimeFlip on
        # 2026-09-21 so that this one could take the Facet name, its directory and its identifier.
        # Here so that anything warning about two icons in the menu bar does not grow a platform case
        # of its own. Goes when the Swift app does.
        LEGACY_PROCESS_NAME="TimeFlip"
        STAMP="Tests/Scripted/last-run-mac.md"
        ;;
    linux)
        # **`~/.local/share/Facet`, and this is measured rather than assumed** (2026-09-08, on the Linux
        # box): a real binary linked against `FacetCore` was asked what
        # `FileManager.urls(for: .applicationSupportDirectory)` answers, and it said `/home/<user>/.local/share`.
        # Corelibs applies the XDG layout on its own, so no code in the app had to change for it.
        SUPPORT="$HOME/.local/share/Facet"
        # **There is no bundle**, so the binary is the whole of it.
        #
        # **Where the resources are no longer matters, and that is a real change from Swift.** A Swift
        # binary run from anywhere but its build directory died on a fatalError the first time it wanted
        # the DDL, because the resource bundle was found by a path. The Rust build compiles the DDL into
        # the binary, so there is nothing to find. See docs/port-findings.md.
        APP=""
        CRATE="facet-linux"
        BINARY="target/debug/$CRATE"
        PROCESS_NAME="$CRATE"
        LEGACY_PROCESS_NAME="TimeFlipLinux"
        STAMP="Tests/Scripted/last-run-linux.md"
        ;;
esac

# **The same directory written the way a person writes it**, for the one place it is stored rather than
# used: `00-setup` puts the debug trace's directory into a `setting` row, and the app expands the tilde on
# its way back out. Kept beside the absolute form so the two cannot name different places.
#
# **The replacement is a bare `~` and must stay one.** Bash does no tilde expansion in the replacement half
# of `${var/pat/rep}`, so nothing needs escaping there, and a `\~` is not an escaped tilde but a backslash
# followed by one. That row is JSON, where `\~` is an invalid escape, so the setting stops parsing and the
# debug trace every check polls is never written.
SUPPORT_TILDE="${SUPPORT/#$HOME/~}"

DB="$SUPPORT/appdata.sqlite"
# The trace, in its own file beside the app's. See `lib.sh` for why the two are separate.
DEBUG_DB="$SUPPORT/debug.sqlite"

# ---------------------------------------------------------------------------- driving the app

# Is it up?
platform_app_is_running() {
    case "$PLATFORM" in
        mac)   pgrep -x "$PROCESS_NAME" >/dev/null 2>&1 ;;
        linux) [ -n "$PROCESS_NAME" ] && pgrep -x "$PROCESS_NAME" >/dev/null 2>&1 ;;
    esac
}

# **How many copies of it are up.** `01-launch` asks because a second launch must hand over to the first
# rather than join it, and "one" is the answer that says so.
platform_app_instances() {
    [ -n "$PROCESS_NAME" ] || { echo 0; return 0; }
    pgrep -x "$PROCESS_NAME" | wc -l | tr -d ' '
}

# The last resort, when a tidy quit did not work.
platform_kill_app() {
    case "$PLATFORM" in
        mac)   pkill -x "$PROCESS_NAME" ;;
        linux) [ -n "$PROCESS_NAME" ] && pkill -x "$PROCESS_NAME" ;;
    esac
}

# **Quit it the way a person would**, through the menu the app puts in front of them, so that the quit
# sequence actually runs. A killed app never gets to do what it does on the way out, and that is a thing
# these checks care about.
#
# Both halves are said out loud when they fail. `run.sh` used to throw the output of each away, and a click
# that never happened cost a run twenty seconds and a wrong diagnosis (see `CLAUDE.md`).
platform_quit_app() {
    case "$PLATFORM" in
        mac)
            # A menu item takes an accessibility press with the menu closed, so no click is needed.
            python3 scripts/ax-press.py --title "Quit Facet" 2>&1 \
                || echo "  Quit Facet would not press; falling back to a kill"
            ;;
        linux)
            # **One call where the Mac needs two**, and that is the whole of the difference between the
            # platforms here. A status item's menu items do not exist in the accessibility tree until a
            # real mouse event has opened it, so that side has to click and then press; the tray menu on
            # this side is a D-Bus object whose items can be read and chosen without opening it at all.
            # `Tests/Methods.md` Method 18.
            #
            # Said out loud when it fails, never swallowed: a Quit that did not happen makes the wait
            # after it time out and report whatever it was waiting on, which is what
            # `>/dev/null 2>&1` on the macOS press cost this suite twice (see `CLAUDE.md`).
            local output status
            output=$(python3 scripts/tray-menu.py --press Quit 2>&1)
            status=$?
            if [ "$status" -ne 0 ]; then
                echo "  the tray Quit would not press (exit $status)${output:+: $output}"
                echo "  falling back to a kill"
            fi
            ;;
    esac
}

# ---------------------------------------------------------------------------- driving the window

# **The window half of the port, added 2026-09-20.** Everything above this was already here; what was
# missing was that `lib.sh` reached the macOS accessibility scripts by name, so every check in the suite
# was macOS-only however platform-aware this file had become. The Linux counterparts are
# `scripts/at-*.py`, which take the same arguments for the same jobs.
#
# **The two families do not mean the same thing by the same attribute**, which is the reason these
# wrappers exist rather than a variable holding a prefix. On macOS `AXIdentifier`, `AXTitle` and
# `AXValue` are three attributes; on Linux a `GtkButton` reports its label as its accessible *name*, so
# a control that wants an identifier has to overwrite it and its value goes in the description. A check
# written against `--desc` would therefore be asking for the label on one platform and the value on the
# other -- so no check names either family, and these decide.

# Press a control by identifier.
platform_press() {
    case "$PLATFORM" in
        mac)   python3 scripts/ax-press.py "$1" 2>&1 ;;
        linux) python3 scripts/at-press.py "$1" 2>&1 ;;
    esac
}

# Press a control by the words on it, which is how every dialogue button is addressed.
platform_press_title() {
    case "$PLATFORM" in
        mac)   python3 scripts/ax-press.py --title "$1" 2>&1 ;;
        # **No flag needed on this side**, and that is a fact about the platform rather than a shortcut:
        # `facet_dialog_add_button` gives the button its title, and an unidentified `GtkButton` reports
        # its label as its accessible name. So the words *are* the name here.
        linux) python3 scripts/at-press.py "$1" 2>&1 ;;
    esac
}

# Press by the description, for controls that carry their label there.
platform_press_desc() {
    case "$PLATFORM" in
        mac)   python3 scripts/ax-press.py --desc "$1" 2>&1 ;;
        linux) python3 scripts/at-press.py --desc "$1" 2>&1 ;;
    esac
}

# A button of the dialogue that is up, addressed as part of the dialogue rather than of the window.
platform_press_sheet() {
    case "$PLATFORM" in
        mac)   python3 scripts/ax-press.py --sheet --title "$1" 2>&1 ;;
        # **A dialogue is a top-level of its own here, not a sheet on the window**, so there is nothing
        # to scope to and the ordinary press finds it. `GtkDialoguePresenter` runs `gtk_dialog_run`,
        # which spins a nested main loop, and the tree is readable and drivable throughout.
        linux) python3 scripts/at-press.py "$1" 2>&1 ;;
    esac
}

# Move to a Settings tab.
#
# **Two genuinely different gestures, which is why this is a step of its own.** A macOS segmented
# control's segments carry their label as a description and are pressed; a GTK `page tab` implements no
# Action interface at all and is *selected*, through the Selection interface of the tab list above it.
# `Tests/Methods.md` Method 20.
platform_select_tab() {
    case "$PLATFORM" in
        # The Slint tab is a radio button carrying `settings-tab-<name in lower case>`.
        mac)   python3 scripts/ax-press.py "settings-tab-$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" 2>&1 ;;
        linux) python3 scripts/at-press.py --tab "$1" 2>&1 ;;
    esac
}

# Write into a field.
platform_set_field() {
    case "$PLATFORM" in
        mac)   python3 scripts/ax-set.py "$1" "$2" 2>&1 ;;
        linux) python3 scripts/at-set.py "$1" "$2" 2>&1 ;;
    esac
}

# The same, having put focus in the field first.
platform_set_field_focused() {
    case "$PLATFORM" in
        mac)   python3 scripts/ax-set.py --focus "$1" "$2" 2>&1 ;;
        # `at-set.py` writes through the accessible interface, which does not need or move focus, so
        # there is no second form of it. Named the same so a check reads the same.
        linux) python3 scripts/at-set.py "$1" "$2" 2>&1 ;;
    esac
}

# A real keystroke, to whatever holds focus.
platform_key() {
    case "$PLATFORM" in
        mac)   python3 scripts/ax-key.py "$@" 2>&1 ;;
        linux) python3 scripts/at-key.py "$@" 2>&1 ;;
    esac
}

# Press and hold, for the stepper repeat that an accessible action cannot reach.
platform_hold() {
    case "$PLATFORM" in
        mac)   python3 scripts/ax-hold.py "$1" "$2" 2>&1 ;;
        linux) python3 scripts/at-hold.py "$1" "$2" 2>&1 ;;
    esac
}

# The whole tree, for the checks that grep it.
platform_tree() {
    case "$PLATFORM" in
        mac)   python3 scripts/ax-dump.py 2>/dev/null ;;
        linux) python3 scripts/at-dump.py 2>/dev/null ;;
    esac
}

# The tree with each element's position and size.
platform_tree_frames() {
    case "$PLATFORM" in
        mac)   python3 scripts/ax-dump.py --frames 2>/dev/null ;;
        linux) python3 scripts/at-dump.py --frames 2>/dev/null ;;
    esac
}

# The buttons of the dialogue that is up, one per line. Non-zero when there is no dialogue.
platform_alert_buttons() {
    case "$PLATFORM" in
        mac)   python3 scripts/ax-alert.py 2>/dev/null ;;
        linux) python3 scripts/at-alert.py 2>/dev/null ;;
    esac
}

# Its wording instead.
platform_alert_message() {
    case "$PLATFORM" in
        mac)   python3 scripts/ax-alert.py --message 2>/dev/null ;;
        linux) python3 scripts/at-alert.py --message 2>/dev/null ;;
    esac
}

# **Put text on the clipboard, for the one check that pastes.**
#
# `pbcopy` is a one-liner because macOS has a pasteboard server holding the bytes. X11 has no such thing: the
# clipboard is a protocol, the program that copied keeps the text and hands it over when asked, so a process that
# sets it and exits takes the text with it. `scripts/at-clipboard.py` says the rest, and it exists rather than a
# call to `xclip` because neither `xclip` nor `xsel` is installed on the Linux box and this needs only pygobject,
# which driving the window already requires.
platform_copy_to_clipboard() {
    case "$PLATFORM" in
        mac)   printf '%s' "$1" | pbcopy ;;
        linux) python3 scripts/at-clipboard.py --set "$1" >/dev/null ;;
    esac
}

# ---------------------------------------------------------------------------- the status item

# What the status item is showing, as a line a check can grep.
#
# **Neither platform has it in the accessibility tree**, and they answer that in opposite ways: macOS
# puts the item in the menu bar's own tree, which `ax-dump.py --menu-bar` reads, and the tray here is a
# D-Bus object with the words as properties on it. `Tests/Methods.md` Method 18.
platform_status_item() {
    case "$PLATFORM" in
        mac)   python3 scripts/ax-dump.py --menu-bar 2>/dev/null | grep -m1 "id=status-item" || true ;;
        linux) python3 scripts/tray-menu.py --label 2>/dev/null || true ;;
    esac
}

# Open the status item's menu, so that its items can be addressed.
#
# **A no-op on Linux, and that is the honest answer rather than a gap.** The menu is a
# `com.canonical.dbusmenu` object whose items can be read and chosen without it ever being opened, so
# there is nothing to open: `platform_menu_press` below works whether or not this was called.
platform_open_menu() {
    # A no-op on macOS too: the menu's items are in the accessibility tree and take a press with the menu
    # closed (measured 2026-09-25 against facet-mac).
    return 0
}

# **The titles a menu identifier can be showing**, printed one per line.
#
# **On macOS too**: tray-icon gives every menu item the same `AXIdentifier`, `fireMenuItemAction:` (measured
# 2026-09-25), so the identifier reaches nothing there either. On Linux nothing carries the identifier to the tray: `com.canonical.dbusmenu` answers with the label and `enabled`,
# and the numeric ids it gives out are libdbusmenu's own and are reassigned whenever the menu is rebuilt
# (measured 2026-09-13, see `scripts/tray-menu.py`). So this side maps the identifier to the words.
#
# **Both sides of the mapping come from `StatusItemMenu`**, which is core and shared, so the titles are not
# invented here -- `Item("Settings…", identifier: Identifier.settings)` is the line, and the same build puts
# both halves on screen.
#
# **Two items change their wording with their state**, which is why this answers a list rather than a string:
# Pause reads *Resume* while paused and Lock reads *Unlock* while locked, and they are the same item either way.
platform_menu_titles() {
    case "$1" in
        # The Rust tray's own words, from `crates/facet-linux/src/tray.rs`. The Swift app said `Settings…`
        # and `Quit`, and a title that matches nothing is a press that reports it found no item.
        open-settings)    printf 'Settings...\n' ;;
        quit-app)         printf 'Quit Facet\n' ;;
        open-about)       printf 'About Facet\n' ;;
        toggle-pause)     printf 'Pause\nResume\n' ;;
        toggle-cube-lock) printf 'Lock\nUnlock\n' ;;
        status-item)      printf '\n' ;;
        *)
            echo "  no tray item is known by the identifier $1." >&2
            echo "  the mapping is in platform_menu_titles, beside StatusItemMenu.Identifier." >&2
            return 1 ;;
    esac
}

# Choose an item of the status item's menu, **named by its identifier on both platforms**.
#
# `menu_press open-settings` reads the same in every check, and this decides how to reach it. On macOS the
# identifier is what `AXIdentifier` carries and the press is by name; on Linux it goes through the mapping above
# and whichever title the menu is currently showing is the one pressed.
platform_menu_press() {
    local titles output
    titles=$(platform_menu_titles "$1") || return 1
    case "$PLATFORM" in
        mac)
            while IFS= read -r title; do
                [ -z "$title" ] && continue
                output=$(python3 scripts/ax-press.py --title "$title" 2>&1) && {
                    printf '%s\n' "$output"
                    return 0
                }
            done <<EOF
$titles
EOF
            echo "  no menu item matching $1${output:+: $output}" >&2
            return 1 ;;
        linux)
            while IFS= read -r title; do
                [ -z "$title" ] && continue
                output=$(python3 scripts/tray-menu.py --press "$title" 2>&1) && {
                    printf '%s\n' "$output"
                    return 0
                }
            done <<EOF
$titles
EOF
            echo "  no tray item matching $1${output:+: $output}" >&2
            return 1 ;;
    esac
}

# **The whole of the status item's menu, as lines a check can grep.**
#
# The two platforms keep it in completely different places -- macOS in the menu bar's own accessibility tree,
# Linux in a D-Bus object -- so this is the step and the method differs. `Tests/Methods.md` Method 18.
#
# The Linux form carries the item labels and the tray's own words, which is what the macOS dump carries too.
platform_menu_tree() {
    case "$PLATFORM" in
        mac)   python3 scripts/ax-dump.py --menu-bar 2>/dev/null ;;
        linux)
            # **Each line gains the identifier the macOS dump carries**, because that is what a check reads:
            # `check_contains "the menu offers Settings" "$menu" "id=open-settings"`. Nothing carries the
            # identifier across D-Bus, so it is put back here from the same mapping `platform_menu_titles` holds
            # -- which is `StatusItemMenu.Identifier`, the app's own, rather than a name invented for the suite.
            #
            # **Only the items that have one.** The category lines and the separators carry no identifier on
            # either platform, and a line without one prints as it is.
            python3 scripts/tray-menu.py --label 2>/dev/null
            python3 scripts/tray-menu.py 2>/dev/null | while IFS= read -r line; do
                local identifier=""
                case "$line" in
                    *"'Settings...'"*)      identifier="open-settings" ;;
                    *"'Quit Facet'"*)       identifier="quit-app" ;;
                    *"'About Facet'"*)      identifier="open-about" ;;
                    *"'Pause'"*|*"'Resume'"*) identifier="toggle-pause" ;;
                    *"'Lock'"*|*"'Unlock'"*)  identifier="toggle-cube-lock" ;;
                esac
                if [ -n "$identifier" ]; then
                    printf '%s   id=%s\n' "$line" "$identifier"
                else
                    printf '%s\n' "$line"
                fi
            done ;;
    esac
}

# **One item of that menu, by identifier**, for the checks that read what a line says or whether it is dead.
#
# On macOS that is the line carrying `id=<identifier>`. On Linux the identifier reaches nothing, so the mapping
# above turns it into the titles the item can be showing and the matching line comes back -- including the
# `(insensitive)` that `tray-menu.py` prints, which is what a check asking whether the line is dead reads.
platform_menu_item() {
    case "$PLATFORM" in
        mac)
            local titles menu
            titles=$(platform_menu_titles "$1") || return 1
            menu=$(python3 scripts/ax-dump.py --menu-bar 2>/dev/null) || true
            while IFS= read -r title; do
                [ -z "$title" ] && continue
                # The title followed by two spaces or the end of the line, so that `Lock` cannot match `Unlock`.
                printf '%s\n' "$menu" | grep -m1 -E "title=$title(  |\$)" && return 0
            done <<EOF
$titles
EOF
            return 0 ;;
        linux)
            local titles menu
            titles=$(platform_menu_titles "$1") || return 1
            menu=$(python3 scripts/tray-menu.py 2>/dev/null) || true
            while IFS= read -r title; do
                [ -z "$title" ] && continue
                # Matched on the quoted label `tray-menu.py` prints, so that `Lock` cannot match `Unlock`.
                printf '%s\n' "$menu" | grep -m1 "'$title" && return 0
            done <<EOF
$titles
EOF
            return 0 ;;
    esac
}

# **The right half of the status item, which is a macOS gesture and has no counterpart here.**
#
# It is not merely unimplemented: an `AppIndicator` publishes one activation and the panel decides what
# a secondary click does, so there is no right half for the app to distinguish and nothing it could
# listen for. The pause-on-right-click gesture that `12-daily-limit` and `62-forced-pause` turn on does
# not exist on this platform, and the app does not pretend it does.
#
# So this refuses loudly rather than returning success, because a gesture that silently did nothing
# would make the wait after it time out and blame the cube -- which is the exact failure `CLAUDE.md`
# records twice.
platform_click_right() {
    case "$PLATFORM" in
        mac)   python3 scripts/status-item-click.py --right "$@" 2>&1 ;;
        linux)
            echo "  the status item has no right half on Linux: an AppIndicator publishes one" >&2
            echo "  activation and the panel owns the secondary click, so there is no gesture to" >&2
            echo "  post. The checks that need it are item 12 of docs/linux-port.md." >&2
            return 1 ;;
    esac
}

# **The left click on the status item.** On Linux that is the SNI `Activate` call a panel makes, sent to
# the app directly: `tray-menu.py --activate`. Said out loud when it fails.
platform_click_left() {
    case "$PLATFORM" in
        mac)   python3 scripts/status-item-click.py 2>&1 ;;
        linux) python3 scripts/tray-menu.py --activate 2>&1 ;;
    esac
}

# ---------------------------------------------------------------------------- accessibility

# **Slint reaches the accessibility bus only while an assistive technology is enabled**, measured
# 2026-09-22 (docs/port-findings.md): with `toolkit-accessibility` false the app is simply not on the bus,
# and every press fails as though the window never opened. So a run turns it on, and `run.sh` puts back
# whatever it found. macOS has no equivalent switch.
platform_accessibility_state() {
    case "$PLATFORM" in
        mac)   echo "not applicable" ;;
        linux) gsettings get org.gnome.desktop.interface toolkit-accessibility ;;
    esac
}

platform_set_accessibility() {
    case "$PLATFORM" in
        mac)   return 0 ;;
        linux) gsettings set org.gnome.desktop.interface toolkit-accessibility "$1" ;;
    esac
}

# ---------------------------------------------------------------------------- the radio

# **Is the Bluetooth radio on? 0 yes, 1 no, 2 cannot tell.**
#
# Three answers rather than two, for the reason `platform_app_is_declared` has three: not being able to ask
# says nothing about the answer, and this is the one probe in the suite whose wrong answer *asks the person
# to do something they have already done*. `lib.sh`'s comment records that happening once from a different
# cause on 2026-08-22.
#
# **It was `system_profiler SPBluetoothDataType` for both platforms until 2026-09-20**, which does not exist
# on Linux -- so the command printed nothing, the `case` fell through to its catch-all, and the suite told
# the owner to turn on a radio that was already on, then failed `00-setup` and stopped the run. A macOS tool
# reached directly from a shared file, which is the same fault as `lib.sh` reaching `ax-press.py`, and it
# survived that sweep because it is a system probe rather than a way of driving the window.
platform_bluetooth_is_on() {
    case "$PLATFORM" in
        mac)
            # Captured and matched rather than piped into `grep -q`, for the reason `tree_has` sets out:
            # this is sourced into files that set pipefail, `system_profiler` writes a great deal after the
            # line that matches, and a pipeline killed by SIGPIPE reports the signal rather than the match.
            #
            # The literal is what the tool prints, `          State: On`, one space after the colon.
            local report
            report="$(system_profiler SPBluetoothDataType 2>/dev/null)"
            [ -z "$report" ] && return 2
            case "$report" in
                *"State: On"*) return 0 ;;
                *) return 1 ;;
            esac ;;
        linux)
            # **Asked of BlueZ, which is what the app itself talks to.** `BlueZRadio` reaches the same
            # adapter over D-Bus, so the adapter's `Powered` property is the same fact the app will act on
            # -- where `rfkill` answers a different question, whether the device is *blocked*, and an
            # unblocked adapter can still be powered down.
            #
            # **Timed out**, because `bluetoothctl` waits on a D-Bus reply and a stuck bluetoothd would
            # otherwise hang the whole run at its first setup step with nothing said.
            command -v bluetoothctl >/dev/null 2>&1 || return 2
            local report
            report="$(timeout 5 bluetoothctl show 2>/dev/null)"
            [ -z "$report" ] && return 2
            case "$report" in
                *"Powered: yes"*) return 0 ;;
                *"Powered: no"*) return 1 ;;
                # No controller at all: `bluetoothctl show` prints `No default controller available`. That
                # is not the radio being off, it is there being no radio, and the two want different words
                # in front of somebody -- so it is the third answer rather than the second.
                *) return 2 ;;
            esac ;;
    esac
}

# ---------------------------------------------------------------------------- facts about the run

# When the binary under test was built. `stat` takes opposite flags on the two systems, and the BSD one
# silently produces nothing on Linux rather than failing, which would have written an empty column.
platform_binary_built_at() {
    [ -n "$BINARY" ] || return 0
    case "$PLATFORM" in
        mac)   stat -f '%Sm' -t '%Y-%m-%d %H:%M:%S' "$BINARY" 2>/dev/null || echo "" ;;
        linux) stat -c '%y' "$BINARY" 2>/dev/null | cut -d'.' -f1 || echo "" ;;
    esac
}

# **A unix epoch formatted as a date, which the two systems spell incompatibly rather than merely
# differently.** `date -r` exists on both and means opposite things: BSD reads it as *this epoch*, GNU as
# *this file\'s modification time*. So the macOS spelling on Linux goes looking for a file named
# `1789900000`, fails with `No such file or directory`, and prints **nothing** -- and a comparison against
# an empty string is false, which is a wrong answer rather than an error.
#
# That is what failed `00-setup` on the first real Linux run (2026-09-20): `the seeds are dated
# 2026-09-20, not today`, on the twentieth. Same shape as `platform_binary_built_at` above, whose comment
# records `stat` doing the same thing in the other direction.
platform_date_from_epoch() {
    case "$PLATFORM" in
        mac)   date -r "$1" "+$2" ;;
        linux) date -d "@$1" "+$2" ;;
    esac
}

# The operating system version, for the run record.
platform_os_version() {
    case "$PLATFORM" in
        mac)   sw_vers -productVersion 2>/dev/null || echo "" ;;
        linux) . /etc/os-release 2>/dev/null && echo "${PRETTY_NAME:-}" || echo "" ;;
    esac
}

# **Whether the binary is properly signed, which is a macOS question and only a macOS question.** Ad-hoc
# signing silently breaks anything reading the Keychain, and that once made a build flag look like a Google
# outage. Linux has no equivalent to get wrong, so it answers what is true rather than borrowing a word.
#
# **Captured and matched, never piped into `grep -q`.** This is reached from files that set `pipefail`, and
# a pipeline whose reader exits early reports the writer's SIGPIPE rather than the match -- which made every
# run from 89 to 94 record `ad-hoc` against an app signed with a real Apple Development certificate. See
# `tree_has` in `lib.sh` for the measurement.
platform_signing() {
    case "$PLATFORM" in
        mac)
            case "$(codesign -dvvv "$APP" 2>&1)" in
                *TeamIdentifier=[A-Z0-9]*) echo "signed" ;;
                *) echo "ad-hoc" ;;
            esac
            ;;
        linux) echo "not applicable" ;;
    esac
}

# ---------------------------------------------------------------------------- building and launching

# **Is there an app to build at all?**
platform_app_is_declared() {
    # **Asked of cargo rather than read out of the manifest**, because the manifest lists every crate
    # and only cargo knows which of them resolve. It answers quickly and builds nothing.
    #
    # **0 yes, 1 no, 2 cannot tell**, and the third is not folded into the second: not being able to
    # ask says nothing about the answer, and reporting "there is no app" when the truth is "cargo is
    # not on PATH" sends somebody to the wrong file.
    platform_cargo_is_available || return 2
    cargo metadata --no-deps --format-version 1 2>/dev/null | python3 -c '
import json, sys
try:
    described = json.load(sys.stdin)
except Exception:
    sys.exit(2)
sys.exit(0 if any(p.get("name") == sys.argv[1] for p in described.get("packages", [])) else 1)
' "$CRATE"
}

# **Cargo has to be found before it can be asked anything.** On the Mac `~/.cargo/bin` is on no
# dotfile's PATH, so a bare `cargo` fails with "command not found" while the toolchain is perfectly
# healthy. The Linux box does source it from .bashrc and .profile, but a shell with no environment
# still would not.
#
# **It uses the toolchain rather than printing the export line and giving up.** A script that knows the
# path well enough to print it knows it well enough to use it, and the alternative was the same two-line
# ceremony at the start of every run.
#
# **It says what it did rather than doing it quietly**, which is the half that matters: a toolchain
# nobody chose is the sort of thing that should be visible in a log when a build behaves oddly.
#
# **An already-set PATH wins**, because putting cargo there is a deliberate act and this must not
# second-guess it.
platform_cargo_is_available() {
    command -v cargo >/dev/null 2>&1 && return 0

    if [ -r "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
        if command -v cargo >/dev/null 2>&1; then
            echo "  cargo was not on PATH; sourced ~/.cargo/env"
            return 0
        fi
    fi

    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
        echo "  cargo was not on PATH; using ~/.cargo/bin"
        return 0
    fi

    echo "  cargo is not on PATH and there is none at ~/.cargo/bin." >&2
    echo "  install a toolchain from https://rustup.rs, or put yours on PATH." >&2
    return 1
}



# **Builds the app, and says everything it did wrong.** The caller decides what a failure means; this
# reports one honestly and returns non-zero. Fifteen lines of output on failure, because a build error is
# usually one line and reproducing it by hand was the cost of throwing it away.
platform_build_app() {
    # **No credentials step yet.** In Swift this ran scripts/generate-credentials.sh first, because
    # 10-google-calendar signs in and the binary under test had to carry the Google client the way a
    # real one does. The Rust app has no Google half, so there is nothing to put in it. Put this back
    # in the same change that adds sign-in, or that check will fail looking like a broken account.

    # **One command on both platforms, which Swift needed two of.** There is no bundler to run and
    # nothing to sign yet: cargo puts an executable straight into target/, and the DDL is compiled into
    # it rather than sitting beside it, so a binary works wherever it is run.
    #
    # **Signing will come back when the app reaches the keychain.** An ad-hoc signature is a different
    # application as far as the Keychain is concerned, so a token written by one build is unreadable by
    # the next and nothing says so: the sweep simply never runs. That is how 10-google-calendar failed
    # the first time it was written in Swift. Nothing here touches the keychain today, so nothing signs.
    #
    # **--locked, so a build cannot quietly resolve a different dependency than the one recorded.**
    local output status
    platform_cargo_is_available || return 1
    output=$(cargo build --locked -p "$CRATE" 2>&1)
    status=$?

    if [ "$status" -ne 0 ]; then
        printf '%s\n' "$output" | tail -15 | sed 's/^/    /' >&2
        return 1
    fi

    # **The build reporting success is not the binary existing.** Checked rather than assumed, for the same
    # reason a command sent to the cube is read back: a build that produced nothing at this path would
    # otherwise be found out by the launch, which reports it as the app failing to start.
    if [ ! -x "$BINARY" ]; then
        echo "  the build succeeded but there is no executable at $BINARY" >&2
        return 1
    fi
    return 0
}

# **Starts it detached**, so the suite keeps its own terminal and the app outlives the shell that began it.
platform_launch_app() {
    case "$PLATFORM" in
        mac) open "$APP" ;;
        linux)
            # Its console copy goes to a file rather than to the run log: every line it prints is also a
            # `debug_log` row, which is what the checks read, but a crash on the way up prints there and
            # nowhere else.
            mkdir -p logs
            nohup "$BINARY" >> logs/app.log 2>&1 &
            ;;
    esac
}

# **A warning that only one platform can earn.** Ad-hoc signing makes every build a different application
# to the Keychain, so Google sync stalls on a prompt and nothing says why. Linux has no equivalent, so it
# has nothing to warn about rather than a warning worded to look similar.
platform_warn_if_unsigned() {
    case "$PLATFORM" in
        mac)
            # Captured and matched rather than piped into `grep -q`: see `tree_has` in lib.sh for why a
            # pipeline cannot answer this under pipefail. It said ad-hoc about a properly signed app on 18
            # runs out of 20, which is the wrong way round for a warning nobody can act on.
            case "$(codesign -dvvv "$APP" 2>&1)" in
                *TeamIdentifier=[A-Z0-9]*) return 1 ;;
                *) return 0 ;;
            esac
            ;;
        linux) return 1 ;;
    esac
}
