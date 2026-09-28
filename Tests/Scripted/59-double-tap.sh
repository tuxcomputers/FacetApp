#!/bin/bash
# The cube's double tap, which pauses it in firmware with nothing to tell the app, kept off: the registers read with
# 0x17 at every login, and sent with the window at 0 when the cube has any other.
#
# **This cube's registers already read a window of 0**, and survive a factory reset (finding 11a), so what is checked
# on it is the read and the app sending nothing. The crate tests cover the send.
#
# **Converted from the Swift suite 2026-09-29**, against `feature/timeTracking`. The Swift Device tab once stepped the
# four registers; that control was removed there too, so this checks only that the gesture stays off.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=5
start "the cube's double tap, kept off"

require_a_paired_cube "there is no cube to read"

since=$(mark)
if relink_a_cube; then
    pass "the app came back up and reached the cube"
else
    fail "the relaunch did not reach and free the cube"
    finish
    exit 1
fi
expect_log "the login asks for the double tap registers" "$since" "command withResponse: 17" 30
registers=$(wait_for "$since" "The cube's double tap is Threshold %" 30)
check_contains "and reads them" "$registers" "Threshold"
check_contains "with the window at 0, so the gesture cannot fire" "$registers" "Window 0"
check "nothing is sent to change them" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND tag = 'ble-tx' AND message LIKE 'command withResponse: 16 %';")"
finish
