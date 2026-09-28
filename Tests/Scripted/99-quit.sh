#!/bin/bash
# The way out: the cube is factory reset, so it is left as the factory made it and back on the vendor PIN, and then
# the app quits.
#
# **Last, because it ends the app** and leaves nothing paired. The next run, on either machine, pairs a cube on the
# factory PIN. `52-device-reset` is what checks the reset itself; this only needs it to have happened.
#
# **Converted from the Swift suite 2026-09-28.** The Swift script also paused and locked the cube on the way out,
# which the Rust app does not do yet.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=7
start "leaving the cube as the factory made it, and quitting"

require_a_paired_cube "there is no cube to leave on the factory settings"

open_settings
select_tab Device
since=$(mark)
press device-reset
sleep 0.5
press_title "Reset Device"
step "waiting for the cube to come back on the factory PIN (up to two minutes)..."
announce "the cube is reset"
outcome=$(wait_for "$since" "Reset: %" 140)
if [ "$outcome" = "Reset: confirmed" ]; then
    verdict_pass
else
    verdict_fail "the reset ended as '${outcome:-nothing within 140s}', so the cube is not on the factory PIN"
fi
check "nothing is paired" "0" "$(setting paired paired)"
check "or connected" "0" "$(setting connection connected)"
check_contains "and the tab says so" "$(element_eventually device-scan-status "factory settings")" "back to factory settings"
close_settings

since=$(mark)
quit_app
if is_running; then
    fail "the app is still running after Quit"
else
    pass "the app has quit"
fi
check "with no link left to close, the cube having been let go by the reset" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Quit: the link to the cube%';")"
check "and the table still says nothing is connected" "0" "$(setting connection connected)"
finish
