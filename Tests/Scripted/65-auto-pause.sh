#!/bin/bash
# The cube's auto-pause: stepped on the Device tab, sent as `0x05`, read back with `0x10`, and only then written down,
# and the cube stopping itself on it.
#
# **This is the setting with a read-back**, so the ordering is the assertion: the command, then the cube's own
# `0x10` answer carrying the new delay, then the app saying the cube confirms it, and only then the table row.
# Stepped to a minute and put back to the seeded 0, and both directions are checked.
#
# **Asks for hands twice**: the cube turned onto Meeting and left alone for a minute, and turned back onto Break.
#
# **Converted from the Swift suite 2026-09-28**, against the Device tab `feature/deviceTab` builds, and given the
# self-stop on `feature/swiftParity`.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=22
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

check "auto-pause starts off, as seeded" "0" "$(setting auto_pause_minutes minutes)"
step_auto_pause 1

# ---------------------------------------------------------------------------- the cube stopping itself
#
# **Turned rather than left where it is**, because the delay restarts on every face change, so a cube that has been
# still for ten minutes says nothing about when the minute began. Meeting, face 2, holds a category, so nothing in the
# app stops the cube there.

open_paused() {
    sql "SELECT paused FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC, device_event_id DESC LIMIT 1;"
}

turned=$(mark)
if ask_and_detect "$(on_face_now "$turned" 2)" \
    "Turn the cube so the Meeting face is up, then leave it alone" \
    "That is face 2, the one lit cyan." \
    "THEN DO NOT TOUCH IT. This script waits about a minute for the cube to stop itself," \
    "and every turn starts that minute over."; then
    check "the cube is counting on Meeting, so there is a clock for the delay to stop" "0" \
        "$(wait_sql "0" "SELECT paused FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC, device_event_id DESC LIMIT 1;" 30)"
    # A minute for the delay, and the history fetch every 10s coming round to file the pause.
    step "waiting up to 150s: a minute for the cube to stop itself, then a history fetch to see it..."
    if [ "$(wait_sql "1" "SELECT paused FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC, device_event_id DESC LIMIT 1;" 150)" = "1" ]; then
        pass "the cube stopped itself once the minute was up"
    else
        fail "the cube never stopped itself, so the delay it confirmed had no effect on the hardware"
    fi
    # Everything in the app that pauses a cube sends 0x06 on, so its absence says the cube did it alone.
    check "and nothing in the app asked it to" "0" \
        "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $turned AND message = 'Sending 06 01';")"
else
    fail "nobody was there to turn the cube, so it was never started for the delay to stop"
    fail "and so it was never watched stopping itself"
    fail "and nothing could be said about whether the app stopped it"
fi

step_auto_pause 0

since=$(mark)
if ask_and_detect "$(on_face_now "$since" 8)" "Turn the cube back to the Break face" \
    "That is face 8, the one lit red. Leave it there for the scripts after this one."; then
    pass "the cube is back on Break"
else
    fail "there was no terminal to ask, so the cube was never turned back"
fi
check "and counting there" "0" \
    "$(wait_sql "0" "SELECT paused FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC, device_event_id DESC LIMIT 1;" 20)"

close_settings
finish
