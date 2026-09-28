#!/bin/bash
# Reaching the cube, getting a PIN accepted, and what the table and the Device tab say once it is paired.
#
# **Every script after this one runs on the pairing it makes**, and it leaves the link up with the window shut.
#
# **Either of two cubes passes, and the checks for each are counted the same.** A cube on the factory PIN is moved
# onto six digits of this app's own; a cube already on a PIN this machine set is left on it. Which one arrived is
# read from the trace, and then that branch's three checks run.
#
# **Asserts on the raw `commandResult: 02`**, which is what an accepted PIN measures as on this firmware (finding 4),
# so a firmware release that ever matches the document fails a check rather than admitting the wrong cube.
#
# **Converted from the Swift suite 2026-09-28**, against the Device tab `feature/deviceTab` builds:
#
# - **The Keychain is not read.** The app says when storing the PIN fails, and that is what is checked, since reading
#   the item from a script puts a macOS access prompt in front of the run.
# - **Reset is only offered here, and called off.** Pressing it asks first, Cancel sends nothing, and the reset itself
#   is `52-device-reset`.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=43
start "connecting to a TimeFlip and logging in with a PIN"

open_settings
select_tab Device

if [ "$(setting paired paired)" = "1" ]; then
    fail "a cube is already paired, so there is nothing here to pair -- run without --keep to start from a clean database"
    close_settings
    finish
    exit $?
fi

scan_for_a_cube
case $? in
    0) step "listed: $SCAN_ROW" ;;
    *) fail "no TimeFlip to connect to: $SCAN_REASON"; close_settings; finish; exit 1 ;;
esac
handle=${SCAN_ROW#device-scan-result-}

# ---------------------------------------------------------------------------- the login

since=$(mark)
press "$SCAN_ROW"
expect_log "pressing the row chooses that device" "$since" "Device clicked: %"
expect_log "and the scan stops, the list having been acted on" "$since" "The scan ended, %" 20
expect_log "the app opens a link to it" "$since" "Connecting to $handle"
expect_log "and the cube answers" "$since" "Connected to $handle, presenting a PIN next" 30
expect_log "the TimeFlip service is on the other end" "$since" "Found characteristic password" 30
expect_log "and so is the characteristic the answer comes back on" "$since" "Found characteristic commandResult"
expect_log "and the one commands go to" "$since" "Found characteristic command"
expect_log "a PIN is presented" "$since" "Presenting a PIN"
expect_log "on the password characteristic" "$since" "password withResponse: %"
expect_log "the cube acknowledges the write" "$since" "password: write acknowledged"
expect_log "and the answer is read from the command result" "$since" "commandResult: read requested"

step "waiting for the cube's verdict..."
announce "the cube accepts a PIN"
if wait_for "$since" "PIN accepted" 40 >/dev/null; then
    verdict_pass
else
    refused=$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message = 'PIN refused';")
    if [ "${refused:-0}" -gt 0 ]; then
        verdict_fail "the cube refused every PIN offered ($refused) -- it is on a PIN this machine did not set, so take its batteries out to put it back on 000000"
    else
        verdict_fail "no verdict on the PIN within 40s"
    fi
    close_settings
    finish
    exit 1
fi

accepted_at=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND tag = 'login' AND message = 'PIN accepted';")
answer=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND debug_log_id < ${accepted_at:-0} AND tag = 'ble-rx' AND message LIKE 'commandResult: %' ORDER BY debug_log_id DESC LIMIT 1;")
check_contains "the accepted answer is 0x02, as measured and not as documented" "$answer" "commandResult: 02"

# ---------------------------------------------------------------------------- the PIN it was left on

rotated=$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message = 'Setting the PIN on the cube to one of its own';")
if [ "${rotated:-0}" -gt 0 ]; then
    step "the cube was on the factory PIN, so the app moves it onto one of its own"
    expect_log "the cube proves its new PIN by logging in with it" "$since" "The cube is now on its new PIN" 20
    # The new PIN is the password written after the rotation, read out of the trace's ASCII.
    new_pin=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND tag = 'ble-tx' AND message LIKE 'password withResponse: %' ORDER BY debug_log_id DESC LIMIT 1;" | sed -E 's/.*\(([0-9]{6})\)$/\1/')
    expected_hex="30"
    for digit in $(printf '%s' "$new_pin" | fold -w1); do
        expected_hex="$expected_hex 3$digit"
    done
    sent=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND tag = 'ble-tx' AND message LIKE 'command withResponse: 30 %' ORDER BY debug_log_id LIMIT 1;")
    check_contains "0x30 went out on the command characteristic, carrying that PIN" "$sent" "$expected_hex"
    if [ -n "$new_pin" ] && [ "$new_pin" != "000000" ]; then
        pass "and it is not the factory PIN ($new_pin)"
    else
        fail "the PIN the cube was moved onto is '${new_pin:-unreadable}'"
    fi
else
    step "the cube was already on a PIN this machine set, so it is left on it"
    expect_log "the factory PIN is refused first, and the stored one tried next" "$since" \
        "Refused, and there is another PIN to try"
    check "nothing is sent to change the PIN" "0" \
        "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND tag = 'ble-tx' AND message LIKE 'command withResponse: 30 %';")"
    last=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND debug_log_id < ${accepted_at:-0} AND tag = 'ble-tx' AND message LIKE 'password withResponse: %' ORDER BY debug_log_id DESC LIMIT 1;")
    if [ -n "$last" ] && [[ "$last" != *"(000000)"* ]]; then
        pass "and the PIN it accepted is not the factory one"
    else
        fail "the accepted PIN was the factory one, yet nothing moved the cube off it: $last"
    fi
fi

# ---------------------------------------------------------------------------- what the login reads

expect_log "the cube says who made it" "$since" "manufacturerName: %(%)" 20
expect_log "and which firmware it runs" "$since" "firmwareRevision: %(%)" 20
expect_log "its charge is read" "$since" "batteryLevel: % (%)" 20
expect_log "and its lock, pause and auto-pause, with 0x10" "$since" "The cube is %locked and %" 20
expect_log "the pairing is recorded as a new one" "$since" "Paired with % ($handle)" 20
check "and the PIN was stored without complaint" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'The PIN on the cube could not be stored%';")"
check "with no notice raised" "no" "$(alert_is_open && echo yes || echo no)"

# ---------------------------------------------------------------------------- the table

check "the table says a cube is paired" "1" "$(setting paired paired)"
check "and connected" "1" "$(setting connection connected)"
check "the handle kept is the one pressed" "$handle" "$(setting device_uuid uuid)"
check_contains "the cube's own name is kept" "$(setting device_name name)" "TimeFlip"
firmware=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND tag = 'ble-rx' AND message LIKE 'firmwareRevision: %' ORDER BY debug_log_id DESC LIMIT 1;" | sed -E 's/.*\((.*)\)$/\1/')
check "the firmware kept is what the cube said" "$firmware" "$(setting device_info firmware)"

# ---------------------------------------------------------------------------- the tab

battery=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND tag = 'ble-rx' AND message LIKE 'batteryLevel: % (%)' ORDER BY debug_log_id DESC LIMIT 1;" | sed -E 's/.*\(([0-9]+%)\)$/\1/')
# A charge the cube pushed after the read replaces it, and is logged as `Charge N%` rather than as a trace row.
pushed=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND tag = 'device' AND message LIKE 'Charge %' ORDER BY debug_log_id DESC LIMIT 1;" | sed -E 's/^Charge ([0-9]+%).*/\1/')
battery=${pushed:-$battery}
check_contains "the tab reads as connected" "$(element_eventually device-connection "Connected")" "Connected"
check_contains "the charge on the tab is the latest the cube gave" "$(element device-battery)" "$battery"
check_contains "the status names the cube" "$(element device-scan-status)" "Connected to"
check "Scan is gone, there being a cube" "0" "$(on_tab device-scan)"
check "and Forget is offered" "1" "$(on_tab device-forget)"
check "Reset is live, the cube being connected" "0" "$(tree | grep -cE "id=device-reset([[:space:]].*)?disabled" || true)"
check "the settings are live" "0" "$(tree | grep -cE "id=device-auto-pause([[:space:]].*)?disabled" || true)"
check "and the note saying there is no cube is gone" "0" "$(on_tab device-settings-gate-note)"

press device-more
sleep 0.5
check_contains "More shows who made it, from the table" "$(element device-manufacturer)" "$(setting device_info manufacturer)"
check_contains "and the firmware" "$(element device-firmware)" "$firmware"
press device-more
sleep 0.5

# ---------------------------------------------------------------------------- Reset, offered and called off

since=$(mark)
press device-reset
expect_log "pressing Reset is heard" "$since" "Button clicked: Reset Device"
check "and it asks first, offering a way out" "Cancel|Reset Device" "$(alert_buttons)"
press_title Cancel
sleep 0.5
check "Cancel sends nothing to the cube" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND tag = 'ble-tx' AND message LIKE 'command withResponse:%';")"
check "the notice goes" "no" "$(alert_is_open && echo yes || echo no)"
check "and the cube is still connected" "1" "$(setting connection connected)"

close_settings
finish
