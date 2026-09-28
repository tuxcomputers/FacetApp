#!/bin/bash
# The cube's charge, read as the link comes up and shown on the Device tab, and the battery warning setting, which
# is kept in the table and sent nowhere.
#
# **Needs the link to come up while it watches**, so it starts with `relink_a_cube`: a quit and a launch, which
# reconnects to the cube `51-device-connect` paired without touching the pairing.
#
# **Converted from the Swift suite 2026-09-28**, against the Device tab `feature/deviceTab` builds:
#
# - **The charge is read on connecting, and then followed**: the app subscribes to the cube's battery level. A charge
#   the cube pushes cannot be provoked from here, so the subscription is what is checked.
# - **The warning cannot be tripped from here**: it is at most 20% and a cube in use is well above it. The Battery
#   row blinking red at or below it is covered by the crate tests, and its row is what is checked.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=13
start "the cube's charge, and the battery warning"

require_a_paired_cube "there is no charge to read"

# ---------------------------------------------------------------------------- read as the link comes up

since=$(mark)
if relink_a_cube; then
    pass "the app came back up and reached the cube again"
else
    fail "the relaunch did not reach the cube within 90s"
    finish
    exit 1
fi
read_at=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND tag = 'ble-tx' AND message = 'batteryLevel: read requested';")
accepted_at=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND message = 'PIN accepted';")
if [ -n "$read_at" ] && [ -n "$accepted_at" ] && [ "$read_at" -gt "$accepted_at" ]; then
    pass "the charge is asked for once the cube has accepted the PIN"
else
    fail "the battery read (row ${read_at:-none}) did not follow the PIN being accepted (row ${accepted_at:-none})"
fi
answer=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND tag = 'ble-rx' AND message LIKE 'batteryLevel: % (%)' ORDER BY debug_log_id LIMIT 1;")
check_contains "and the cube answers with a percentage" "$answer" "%)"
percent=$(printf '%s' "$answer" | sed -E 's/.*\(([0-9]+)%\)$/\1/')
if [ -n "$percent" ] && [ "$percent" -ge 0 ] && [ "$percent" -le 100 ]; then
    pass "a charge the cube can have ($percent%)"
else
    fail "the charge read as '${percent:-nothing}' from '$answer'"
fi

expect_log "and the app follows it from then on" "$since" "Following the battery" 20

open_settings
select_tab Device
check_contains "the tab shows the charge that was read" "$(element_eventually device-battery "$percent%")" "$percent%"

# ---------------------------------------------------------------------------- the battery warning

was=$(setting low_battery_level percent)
if [ "${was:-0}" -ge 20 ]; then want=$((was - 1)); else want=$((was + 1)); fi

since=$(mark)
set_field device-battery-warning "$want"
expect_log "a new warning level is written to the table" "$since" \
    "App setting low_battery_level.percent -> number($want)"
check "and the table holds it" "$want" "$(setting low_battery_level percent)"
check_contains "the field shows it" "$(element device-battery-warning)" "$want"
check "nothing is sent to the cube for it" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND tag = 'ble-tx';")"

since=$(mark)
set_field device-battery-warning "$was"
expect_log "putting it back is written too" "$since" "App setting low_battery_level.percent -> number($was)"
check "and the table is back where it was" "$was" "$(setting low_battery_level percent)"
check "with no notice raised along the way" "no" "$(alert_is_open && echo yes || echo no)"

close_settings
finish
