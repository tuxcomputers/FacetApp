#!/bin/bash
# Pausing and locking the cube from the menu bar: Pause and Resume with 0x06, Lock with the pause first and then
# 0x04, a pause refused while locked, Unlock resuming, a left click on the status item pausing and a double click
# locking, and the quit leaving the cube paused and locked, from the menu and
# on a SIGTERM, which is how a logout or a shutdown asks on Linux. Each command is
# read back with 0x10, and each is followed by a history fetch that files what the cube did.
#
# **Starts from the cube running on Break**, and ends there: `relink_a_cube` frees the cube the quit locked.
#
# **Converted from the Swift suite 2026-09-29**, against `feature/timeTracking`. The Swift script told one click from
# two on the status item's right half; here the whole item is the left click, the halves having been left behind.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=35
start "pausing and locking the cube from the menu bar"

require_a_paired_cube "there is no cube to pause"

open_paused() {
    sql "SELECT paused FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC, device_event_id DESC LIMIT 1;"
}
wait_open_paused() {
    wait_sql "$1" "SELECT paused FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC, device_event_id DESC LIMIT 1;" 20
}

# `menu_eventually <item> <words>`: the menu item's line once it carries `words`, waiting up to 10s for the redraw that
# follows a command's read-back.
menu_eventually() {
    local line="" waited=0
    while [ "$waited" -lt 50 ]; do
        line=$(platform_menu_item "$1")
        case "$line" in *"'$2'"*) break ;; esac
        sleep 0.2
        waited=$((waited + 1))
    done
    printf '%s\n' "$line"
}

check "the cube starts running" "0" "$(wait_open_paused 0)"

# ---------------------------------------------------------------------------- pause and resume

since=$(mark)
menu_press toggle-pause
expect_log "Pause sends 0x06 on" "$since" "Sending 06 01" 15
expect_log "and the cube reads back paused" "$since" "The cube is paused" 15
expect_log "a fetch files what it did" "$since" "Fetching history (the cube was paused from the menu bar)%" 15
check "the open segment is a pause" "1" "$(wait_open_paused 1)"
check_contains "and the menu offers Resume" "$(menu_eventually toggle-pause Resume)" "Resume"

since=$(mark)
menu_press toggle-pause
expect_log "Resume sends 0x06 off" "$since" "Sending 06 02" 15
expect_log "and the cube reads back running" "$since" "The cube is running" 15
expect_log "a fetch files it" "$since" "Fetching history (the cube was resumed from the menu bar)%" 15
check "the open segment counts again" "0" "$(wait_open_paused 0)"

# ---------------------------------------------------------------------------- lock, which pauses first

check "pause_on_lock is on, as seeded" "1" "$(setting pause_on_lock enabled)"
since=$(mark)
menu_press toggle-cube-lock
expect_log "Lock pauses the cube first" "$since" "Sending 06 01" 15
expect_log "and then locks it" "$since" "Sending 04 01" 15
paused_at=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND message = 'Sending 06 01';")
locked_at=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND message = 'Sending 04 01';")
if [ -n "$paused_at" ] && [ -n "$locked_at" ] && [ "$paused_at" -lt "$locked_at" ]; then
    pass "the pause went before the lock, a locked cube reporting itself paused"
else
    fail "the pause (${paused_at:-none}) did not go before the lock (${locked_at:-none})"
fi
expect_log "the cube reads back locked" "$since" "The cube is locked" 15
check_contains "and the menu offers Unlock" "$(menu_eventually toggle-cube-lock Unlock)" "Unlock"

since=$(mark)
menu_press toggle-pause
expect_log "a pause while locked is refused" "$since" "The cube is locked, so pausing it means nothing; unlock it first" 10
check "and nothing is sent for it" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Sending 06 %';")"

# ---------------------------------------------------------------------------- unlock, which resumes

since=$(mark)
menu_press toggle-cube-lock
expect_log "Unlock unlocks" "$since" "The cube is unlocked" 15
expect_log "and resumes" "$since" "The cube is running" 15
check "the open segment counts again" "0" "$(wait_open_paused 0)"

# ---------------------------------------------------------------------------- the status item's clicks

since=$(mark)
activate_status_item
expect_log "a left click pauses the cube, once the double click interval has passed" "$since" \
    "Fetching history (the cube was paused from the menu bar)%" 15
check "the open segment is a pause" "1" "$(wait_open_paused 1)"
since=$(mark)
activate_status_item
expect_log "and another resumes it" "$since" "The cube is running" 15
check "the open segment counts again" "0" "$(wait_open_paused 0)"

since=$(mark)
doubled=$since
double_click_left
expect_log "a double click is taken as one gesture" "$since" "Status item double clicked, so the cube lock is toggled" 10
expect_log "and locks the cube" "$since" "The cube is locked" 15
since=$(mark)
double_click_left
expect_log "a second double click unlocks it" "$since" "The cube is unlocked" 15
expect_log "and it runs again" "$since" "The cube is running" 15
sleep 1
check "and neither double click also sent a single click's pause" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $doubled AND message LIKE 'Fetching history (the cube was paused from the menu bar)%';")"

# ---------------------------------------------------------------------------- the quit

since=$(mark)
quit_app
expect_log "quitting leaves the cube paused and locked" "$since" "Quit: the cube is paused and locked" 15
if relink_a_cube; then
    pass "a relaunch reaches the cube and frees it again"
else
    fail "the relaunch did not reach and free the cube"
fi

# ---------------------------------------------------------------------------- a SIGTERM

since=$(mark)
pkill -TERM -x "$PROCESS_NAME"
status=$?
[ "$status" -ne 0 ] && red "  the SIGTERM could not be sent (pkill exit $status)"
expect_log "a SIGTERM quits through the quit sequence" "$since" "Quitting on SIGTERM" 10
expect_log "and leaves the cube paused and locked" "$since" "Quit: the cube is paused and locked" 15
waited=0
while is_running && [ "$waited" -lt 100 ]; do
    sleep 0.1
    waited=$((waited + 1))
done
check "and the app is gone" "no" "$(is_running && echo yes || echo no)"
sleep 1
relaunched=$(mark)
ensure_app_running
if wait_for "$relaunched" "Reconnected to %" 90 >/dev/null && free_the_cube; then
    pass "a relaunch reaches the cube and frees it again"
else
    fail "the relaunch did not reach and free the cube"
fi
finish
