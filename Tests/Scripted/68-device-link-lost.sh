#!/bin/bash
# The link dropping while the app holds it: noticed, recorded without losing the pairing, and shown on the Device
# tab, and the app looking for the cube again by itself. Then Bluetooth comes back and the app reaches the cube with
# no relaunch.
#
# **Asks for hands twice**: Bluetooth off, then on. Nothing on this machine turns a radio off on somebody's behalf.
# `watch_bluetooth` asks for it back on any way out of the script, a failed check included.
#
# **New with the Rust app, 2026-09-28.** The Swift suite reached a dropped link through manual mode (`56`) and a cube
# out of range (`60`). What is built is the app asking a held link every five seconds whether it is still up, and
# looking for the cube again after a drop, waiting 2 seconds and doubling up to 30 between looks.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=13
start "the link to the cube dropping, and coming back at the next launch"

require_a_paired_cube "there is no link to lose"
paired_uuid=$(setting device_uuid uuid)

open_settings
select_tab Device
check_contains "the tab starts connected" "$(element_eventually device-connection "Connected")" "Connected"

since=$(mark)
watch_bluetooth
BLUETOOTH_IS_OFF=1
announce "turning Bluetooth off drops the link, and the app notices"
if ask_and_detect \
    "SELECT message FROM debug_log WHERE debug_log_id > $since AND message = 'The cube is no longer connected';" \
    "Turn Bluetooth OFF" \
    "On the Mac: the Bluetooth item in Control Centre. On Linux: the Bluetooth applet on the panel." \
    "The app asks its link every five seconds whether it is still up; this carries on once it notices."; then
    verdict_pass
else
    verdict_fail "there was no terminal to ask, so Bluetooth was never turned off"
    finish
    exit 1
fi
expect_log "the drop is recorded" "$since" "The link to the cube dropped"
expect_log "and the app says it will look for the cube again" "$since" "The cube went away; looking for it again in %s"
check "the table says the cube is not connected" "0" "$(setting connection connected)"
check_contains "and when the link was lost" "$(setting connection connection_lost)" "$(date '+%Y-%m-%d')"
check "but the pairing is kept" "1" "$(setting paired paired)"
check "and so is the handle" "$paired_uuid" "$(setting device_uuid uuid)"
check_contains "the tab says disconnected" "$(element_eventually device-connection "Disconnected")" "Disconnected"
check "and stops showing a charge it can no longer vouch for" "0" \
    "$(element device-battery | grep -cE '[0-9]+%' || true)"
check "the settings go dead" "1" "$(tree | grep -cE "id=device-auto-pause([[:space:]].*)?disabled" || true)"
check "and Forget is still offered, the cube still being paired" "1" "$(on_tab device-forget)"

close_settings
if ! action_required "Turn Bluetooth back ON" \
    "Turn it on the same way it went off, then answer y." \
    "The app finds the cube again by itself, with no relaunch."; then
    fail "Bluetooth was not turned back on"
    finish
    exit 1
fi
BLUETOOTH_IS_OFF=0
for _ in $(seq 1 30); do bluetooth_is_on && break; sleep 1; done

since=$(mark)
expect_log "once Bluetooth is back, the app reaches the cube again by itself" "$since" "Reconnected to %" 90
finish
