#!/bin/bash
# Looking for a TimeFlip, and finding one: the Scan button, what it lists, stopping it, the scan ending by itself,
# and All Devices.
#
# **Starts with nothing paired**, which is what `13-device-tab` leaves, and leaves nothing paired either: pairing is
# `51-device-connect`.
#
# **Converted from the Swift suite 2026-09-28**, against the Device tab `feature/deviceTab` builds:
#
# - **A heard device is drawn, not logged**, so the tree is polled for the `device-scan-result-` row rather than the
#   trace for a peripheral line.
# - **A scan ends** when it is stopped, when a device is chosen, after its fifteen seconds, or when the Device tab is
#   left or the window closed.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=24
start "scanning for a TimeFlip, and finding one"

if device_required; then
    pass "a TimeFlip was offered for this run, back in 00-setup"
else
    fail "no TimeFlip was offered for this run in 00-setup, so nothing from here on can be checked"
    finish
    exit $?
fi

if ! require_bluetooth; then
    fail "Bluetooth is off, so nothing can be scanned for"
    finish
    exit $?
fi

open_settings
select_tab Device

check "nothing is paired, which is what a scan is for" "0" "$(setting paired paired)"
check "so the tab offers Scan" "1" "$(on_tab device-scan)"
check "and not Forget" "0" "$(on_tab device-forget)"

# ---------------------------------------------------------------------------- a scan, stopped by hand

since=$(mark)
press device-scan
expect_log "pressing Scan asks for the filtered scan" "$since" "Button clicked: Scan for Devices (allDevices=false)"
expect_log "and the radio starts listening" "$since" "Scanning, unfiltered" 20
check_contains "the button offers to stop it" "$(element device-scan)" "Stop Scan"
check_contains "and the status says it is looking" "$(element device-scan-status)" "Looking for devices"

announce "a TimeFlip is listed while the scan runs"
row=""
for _ in $(seq 1 180); do
    row=$(tree | grep -m1 -o "device-scan-result-[^ ]*" || true)
    [ -n "$row" ] && break
    sleep 0.1
done
if [ -n "$row" ]; then
    verdict_pass
    grey "          $row"
else
    verdict_fail "no device-scan-result row within 18s -- is the cube awake?"
    press device-scan
    finish
    exit 1
fi
check_contains "and its row carries the cube's name" "$(element "$row")" "TimeFlip"

since=$(mark)
press device-scan
expect_log "pressing it again stops the scan" "$since" "Button clicked: Stop Scan"
expect_log "and the scan ends, saying how many it found" "$since" "The scan ended, % device(s) found" 10
check_contains "the button offers to scan again" "$(element device-scan)" "Scan for Devices"
check_contains "the status says what was found" "$(element device-scan-status)" "Found "
check "and the TimeFlip is still listed to be chosen" "1" "$(on_tab "$row")"

# ---------------------------------------------------------------------------- a scan nobody stops

since=$(mark)
started=$SECONDS
press device-scan
step "waiting out the 15 second scan..."
expect_log "a scan nobody stops ends by itself" "$since" "The scan ended, % device(s) found" 25
elapsed=$((SECONDS - started))
if [ "$elapsed" -ge 13 ]; then
    pass "and not before it had listened for its fifteen seconds (${elapsed}s)"
else
    fail "it ended after ${elapsed}s, well short of the fifteen it is meant to listen for"
fi
check_contains "the button offers to scan again" "$(element device-scan)" "Scan for Devices"

# ---------------------------------------------------------------------------- All Devices

filtered=$(tree | grep -c "id=device-scan-result-" || true)
press device-scan-all
sleep 0.5
since=$(mark)
press device-scan
expect_log "ticking All Devices makes the next scan an unfiltered one" "$since" \
    "Button clicked: Scan for Devices (allDevices=true)"
wait_for "$since" "The scan ended, % device(s) found" 25 >/dev/null
all=$(tree | grep -c "id=device-scan-result-" || true)
if [ "${all:-0}" -ge "${filtered:-0}" ] && [ "${all:-0}" -ge 1 ]; then
    pass "it lists at least what the filtered scan did ($all against $filtered)"
else
    fail "the unfiltered scan listed $all device(s), fewer than the $filtered the filtered one did"
fi
press device-scan-all
sleep 0.5

# ---------------------------------------------------------------------------- leaving the tab, and closing the window

since=$(mark)
press device-scan
wait_for "$since" "Scanning, unfiltered" 20 >/dev/null
select_tab Report
expect_log "leaving the Device tab stops the scan" "$since" "Stopping the scan: the Report tab was selected" 10
expect_log "and the scan ends" "$since" "The scan ended, %" 10
select_tab Device

since=$(mark)
press device-scan
wait_for "$since" "Scanning, unfiltered" 20 >/dev/null
close_settings
expect_log "closing the window stops the scan" "$since" "Stopping the scan: the Settings window closed" 10
expect_log "and the scan ends" "$since" "The scan ended, %" 10
finish
