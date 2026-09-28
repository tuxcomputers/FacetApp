#!/bin/bash
# LED brightness and blink interval: stepped on the Device tab, sent to the cube, and only then written down.
#
# **Neither has a read-back** (`docs/timeflip.md`), so the cube's acknowledgement of the write is all there is to
# wait for. **The ordering is the assertion**: the table row is written after the cube took the command, never
# before. Each is stepped once and put back, and both directions are checked.
#
# **Converted from the Swift suite 2026-09-28**, against the Device tab `feature/deviceTab` builds. The commands and
# the ordering are the Swift ones; the wording is the Rust app's.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=24
start "LED brightness and blink interval, sent and only then written down"

require_a_paired_cube "there is no cube to send the LED settings to"

open_settings
select_tab Device
press device-led
sleep 0.5

# `step_led <field> <row field> <command byte> <value>`: sets the field and checks the command, its acknowledgement,
# and the table row that follows them, in that order. Six checks.
step_led() {
    local control="$1" field="$2" byte="$3" value="$4" since hex took stored
    hex=$(printf '%s %02X' "$byte" "$value")
    since=$(mark)
    set_field "$control" "$value"
    expect_log "$control $value goes to the cube as $hex" "$since" "command withResponse: $hex" 15
    expect_log "and the cube acknowledges it" "$since" "command: write acknowledged"
    expect_log "the app says nothing can read it back" "$since" \
        "The cube took the write; nothing can read this command back"
    expect_log "and then writes it down" "$since" "App setting led_settings.$field -> number($value)"
    took=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND message = 'The cube took the write; nothing can read this command back';")
    stored=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'App setting led_settings.$field -> %';")
    if [ -n "$took" ] && [ -n "$stored" ] && [ "$stored" -gt "$took" ]; then
        pass "the row is written after the cube took it, not before"
    else
        fail "the row (${stored:-none}) did not follow the cube taking the command (${took:-none})"
    fi
    check "the table holds $value" "$value" "$(setting led_settings "$field")"
}

# ---------------------------------------------------------------------------- brightness, 1 to 100, command 0x09

was=$(setting led_settings brightness)
if [ "${was:-0}" -ge 100 ]; then want=$((was - 1)); else want=$((was + 1)); fi
step_led device-led-brightness brightness 09 "$want"
step_led device-led-brightness brightness 09 "$was"

# ---------------------------------------------------------------------------- blink interval, 5 to 60, command 0x0A

was=$(setting led_settings blink_interval)
if [ "${was:-0}" -ge 60 ]; then want=$((was - 1)); else want=$((was + 1)); fi
step_led device-led-blink blink_interval 0A "$want"
step_led device-led-blink blink_interval 0A "$was"

press device-led
close_settings
finish
