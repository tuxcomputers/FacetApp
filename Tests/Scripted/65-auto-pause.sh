#!/bin/bash
# The cube's auto-pause: stepped on the Device tab, sent as `0x05`, read back with `0x10`, and only then written down.
#
# **This is the setting with a read-back**, so the ordering is the assertion: the command, then the cube's own
# `0x10` answer carrying the new delay, then the app saying the cube confirms it, and only then the table row.
# Stepped once and put back, and both directions are checked.
#
# **Converted from the Swift suite 2026-09-28**, against the Device tab `feature/deviceTab` builds. The Swift script
# also watched the cube pause itself when the delay ran out; that needs face notifications, which are not built, so
# it comes back with them.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=16
start "auto-pause, sent, read back, and only then written down"

require_a_paired_cube "there is no cube to send auto-pause to"

open_settings
select_tab Device

# `step_auto_pause <minutes>`: sets the field and checks the command, the `0x10` read-back, the confirmation and the
# table row that follows them, in that order. Eight checks.
step_auto_pause() {
    local value="$1" since hex high low asked answer found confirmed stored
    high=$(printf '%02X' $((value >> 8)))
    low=$(printf '%02X' $((value & 255)))
    hex="05 $high $low"
    since=$(mark)
    set_field device-auto-pause "$value"
    expect_log "auto-pause $value goes to the cube as $hex" "$since" "command withResponse: $hex" 15
    expect_log "and is read back with 0x10" "$since" "command withResponse: 10"
    wait_for "$since" "The cube is %locked and %" 10 >/dev/null
    asked=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND message = 'command withResponse: 10';")
    answer=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > ${asked:-0} AND tag = 'ble-rx' AND message LIKE 'commandResult: %' ORDER BY debug_log_id LIMIT 1;")
    # The 0x10 answer is four bytes with no echoed command byte: lock, pause, then the delay as a big-endian u16.
    if [[ "$answer" =~ ^commandResult:\ [0-9A-F]{2}\ [0-9A-F]{2}\ $high\ $low ]]; then
        pass "the cube reads back $high $low"
    else
        fail "the 0x10 answer does not carry $high $low: ${answer:-none}"
    fi
    if [ "$value" -gt 0 ]; then
        expect_log "which the app reads as a ${value}m delay" "$since" "The cube is % pausing itself after ${value}m"
    else
        announce "which the app reads as no delay"
        if found=$(wait_for "$since" "The cube is %locked and %" 10) && [[ "$found" != *"pausing itself"* ]]; then
            verdict_pass
        else
            verdict_fail "the status after turning auto-pause off read '${found:-nothing}'"
        fi
    fi
    expect_log "the app says the cube confirms it" "$since" "The cube confirms it took"
    expect_log "and then writes it down" "$since" "App setting auto_pause_minutes.minutes -> number($value)"
    confirmed=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND message = 'The cube confirms it took';")
    stored=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'App setting auto_pause_minutes.minutes -> %';")
    if [ -n "$confirmed" ] && [ -n "$stored" ] && [ "$stored" -gt "$confirmed" ]; then
        pass "the row is written after the read-back confirmed it, not before"
    else
        fail "the row (${stored:-none}) did not follow the confirmation (${confirmed:-none})"
    fi
    check "the table holds $value" "$value" "$(setting auto_pause_minutes minutes)"
}

was=$(setting auto_pause_minutes minutes)
if [ "${was:-0}" -ge 240 ]; then want=$((was - 1)); else want=$((was + 1)); fi
step_auto_pause "$want"
step_auto_pause "$was"

close_settings
finish
