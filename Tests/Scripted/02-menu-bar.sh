#!/bin/bash
# The status item: that it is there, what it says with nothing timed, what its menu holds, and Settings from the menu.
#
# **Converted from the Swift suite 2026-09-29**, against `feature/swiftParity`:
#
# - **No halves.** The whole item is the left click, which is Pause's accelerator (`docs/rust-port.md`), and `57`
#   checks it against the cube. So the left half opening the menu is gone, and the menu is read through the port.
# - **The line's colours are checked where each state arises**: cyan in `05`, red in `12`, green in `55`, and yellow in
#   `68`. What is checked here is the idle line, `Facet` in the system's own colour.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=12
start "the menu bar item, what it says, and its menu"

close_settings

# ---------------------------------------------------------------------------- what it says

item=$(status_item)
if [ -n "$item" ]; then
    pass "the status item is in the menu bar"
    grey "          $item"
else
    fail "no status item found in the menu bar"
fi

# **Nothing is being timed at this point in a run**, a rebuilt database holding no open segment and no cube.
check "nothing is being timed" "0" \
    "$(sql "SELECT COUNT(*) FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12;")"
check_contains "so the item says Facet" "$item" "Facet"
check "and the trace says so" "Menu bar reads Facet" \
    "$(dsql "SELECT message FROM debug_log WHERE message LIKE 'Menu bar reads %' ORDER BY debug_log_id DESC LIMIT 1;")"
check "in the system's own colour" "Menu bar: name ordinary, figure ordinary" \
    "$(dsql "SELECT message FROM debug_log WHERE message LIKE 'Menu bar: name %' ORDER BY debug_log_id DESC LIMIT 1;")"

# ---------------------------------------------------------------------------- what the menu holds

# **Polled rather than asked once.** On Linux the tray is a D-Bus object the app registers as it starts, and asked
# straight after a relaunch it is not there yet.
reachable=no
for _ in $(seq 1 20); do
    if [ -n "$(platform_menu_item open-settings)" ]; then
        reachable=yes
        break
    fi
    sleep 0.5
done
check "the menu can be read" "yes" "$reachable"

check_contains "the menu offers Settings" "$(platform_menu_item open-settings)" "Settings..."
check_contains "the menu offers About" "$(platform_menu_item open-about)" "About Facet"
check_contains "the menu offers Quit" "$(platform_menu_item quit-app)" "Quit Facet"
# One item, not two: the same control says Pause or Resume depending on what is happening.
check "there is exactly one pause item" "1" \
    "$(platform_menu_tree | grep -cE "title=(Pause|Resume)(  |\$)|'(Pause|Resume)'")"

# ---------------------------------------------------------------------------- into the window

since=$(mark)
menu_press open-settings
expect_log "choosing Settings opens the window on Faces" "$since" "Settings opened on Faces" 10
check "the Settings window opened" "yes" "$(settings_is_open && echo yes || echo no)"
close_settings

finish
