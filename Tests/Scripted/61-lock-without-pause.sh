#!/bin/bash
# Locking the cube with pause_on_lock off: the lock still goes, only the pause is skipped, and Unlock resumes it as
# ever. The setting is put back on at the end.
#
# **Starts from the cube running on Break**, and leaves it there.
#
# **Converted from the Swift suite 2026-09-29**, against `feature/timeTracking`. The Swift script locked with the
# status item's double click; that gesture is not built, so the menu's Lock is what is pressed.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=10
start "locking the cube with pause_on_lock off"

require_a_paired_cube "there is no cube to lock"

open_settings
select_tab Device
since=$(mark)
press device-pause-on-lock
expect_log "pause_on_lock is turned off on the Device tab" "$since" "App setting pause_on_lock.enabled -> flag(false)"
check "and the table holds it" "0" "$(setting pause_on_lock enabled)"
close_settings

since=$(mark)
menu_press toggle-cube-lock
expect_log "Lock says it is skipping the pause" "$since" "pause_on_lock is off, so the cube is locked without pausing it" 15
expect_log "and locks the cube" "$since" "The cube is locked" 15
check "no pause was sent" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message = 'Sending 06 01';")"

since=$(mark)
menu_press toggle-cube-lock
expect_log "Unlock unlocks" "$since" "The cube is unlocked" 15
expect_log "and resumes" "$since" "The cube is running" 15
check "the open segment counts" "0" \
    "$(wait_sql 0 "SELECT paused FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC LIMIT 1;" 20)"

open_settings
select_tab Device
since=$(mark)
press device-pause-on-lock
expect_log "pause_on_lock is turned back on" "$since" "App setting pause_on_lock.enabled -> flag(true)"
check "and the table holds it" "1" "$(setting pause_on_lock enabled)"
close_settings
finish
