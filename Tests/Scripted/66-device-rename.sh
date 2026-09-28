#!/bin/bash
# Renaming the cube from the Device tab: `0x15` to the hardware, the row written only after the cube took it, the name
# surviving the next connection, and the cube renamed back.
#
# **`0x15` has no read-back** (finding 2): the cube reports its name as its GAP name on the next connection, and the
# platform can go on reporting the old one for a connection after a rename. So the row is written once the write is
# acknowledged, and a relaunch checks that the name reported then does not undo it.
#
# **Converted from the Swift suite 2026-09-28.** The wording is the Rust app's.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=22
start "renaming the cube, and the name surviving the next connection"

TEST_NAME="Facet cube"
TEST_BYTES="15 0A 46 61 63 65 74 20 63 75 62 65"

require_a_paired_cube "there is no cube to rename"
original=$(setting device_name name)
if [ -z "$original" ]; then
    red "  the cube has not said what it is called, so there is nothing to rename it back to"
    finish
    exit 2
fi
step "the cube is called $original"

open_settings
select_tab Device

# `rename_to <name>`: opens the Name row, types `name` and presses Return.
rename_to() {
    press device-name
    wait_for_element device-name-field >/dev/null
    set_field_focused device-name-field "$1"
    press_return
    sleep 0.5
}

check "the Name row is live, the cube being connected" "0" \
    "$(tree | grep -cE "id=device-name([[:space:]].*)?disabled" || true)"
press device-name
wait_for_element device-name-field >/dev/null
check "pressing it opens a field to type into" "1" "$(on_tab device-name-field)"

# ---------------------------------------------------------------------------- a name the cube cannot store

since=$(mark)
set_field_focused device-name-field "Cube 🎲"
press_return
sleep 0.5
message=$(element_eventually notice-message "plain")
check_contains "an emoji is refused, and the notice says why" "$message" "The TimeFlip can only store plain"
check_contains "that the limit is the device's" "$message" "not something this app has decided"
check_contains "and what is allowed" "$message" "18 characters"
check "nothing is sent to the cube" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Renaming the cube to%';")"
check "and the name on record is unchanged" "$original" "$(setting device_name name)"
press_title OK
sleep 0.5

# ---------------------------------------------------------------------------- a name it can

since=$(mark)
rename_to "$TEST_NAME"
expect_log "the new name goes to the cube as 0x15, its length, and its ASCII" "$since" \
    "command withResponse: $TEST_BYTES" 15
expect_log "the rename is announced" "$since" "Renaming the cube to $TEST_NAME"
expect_log "the cube takes the write" "$since" "The cube took the write; nothing can read this command back" 30
expect_log "and only then is the name written down" "$since" "The cube is called $TEST_NAME: renamed from the Device tab" 30
took=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND message = 'The cube took the write; nothing can read this command back';")
stored=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'The cube is called $TEST_NAME:%';")
if [ -n "$took" ] && [ -n "$stored" ] && [ "$stored" -gt "$took" ]; then
    pass "the row is written after the cube took it, not before"
else
    fail "the row (${stored:-none}) did not follow the cube taking the write (${took:-none})"
fi
check "the table holds the new name" "$TEST_NAME" \
    "$(wait_sql "$TEST_NAME" "SELECT json_extract(setting_value, '\$.name') FROM setting WHERE setting_name = 'device_name';" 10)"
check "and keeps the one it replaced for the scan" "$original" "$(setting device_name previous_name)"
check_contains "the Name row shows it" "$(element_eventually device-name "$TEST_NAME")" "$TEST_NAME"
check_contains "and the notice says the cube goes on advertising its old name" \
    "$(element_eventually notice-message "advertising")" "advertising"
press_title OK
sleep 0.5

# ---------------------------------------------------------------------------- the next connection

close_settings
since=$(mark)
if ! relink_a_cube; then
    fail "the relaunch did not reach the cube within 90s of the rename"
    finish
    exit 1
fi
expect_log "the next connection reports a name" "$since" "The cube now reports its name as %" 30
step "$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND message LIKE 'The cube now reports its name as %' ORDER BY debug_log_id LIMIT 1;")"
check "and the name on record is still the new one" "$TEST_NAME" "$(setting device_name name)"
check "with the old one still kept" "$original" "$(setting device_name previous_name)"

# ---------------------------------------------------------------------------- and back

open_settings
select_tab Device
since=$(mark)
rename_to "$original"
expect_log "renaming it back is written down too" "$since" "The cube is called $original: renamed from the Device tab" 30
check "the table holds the original name" "$original" "$(setting device_name name)"
check "and keeps the test name as the one it replaced" "$TEST_NAME" "$(setting device_name previous_name)"
press_title OK
sleep 0.5
close_settings
finish
