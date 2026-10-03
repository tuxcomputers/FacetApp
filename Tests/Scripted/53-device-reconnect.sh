#!/bin/bash
# Getting back to the cube by itself: a quit closes the link, a launch with the window shut finds the cube again,
# Forget gives it up, and a launch with nothing paired does not go looking.
#
# **Starts from the pairing `51-device-connect` made, and puts it back** with `restore_the_pairing` after the
# forget, so the scripts after this one still have it.
#
# **Converted from the Swift suite 2026-09-28**, against the Device tab `feature/deviceTab` builds:
#
# - **The quit is checked for closing the link.** It also leaves the cube paused and locked, which `57` checks.
# - **What is pushed to the cube at login** (the clock, the LED settings, auto-pause) is checked by the scripts that
#   own each setting, not here.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=34
start "reconnecting to a paired TimeFlip at launch, and forgetting it"

require_a_paired_cube "there is nothing to come back to"
paired_uuid=$(setting device_uuid uuid)
step "paired to ${paired_uuid:-unknown}"

# ---------------------------------------------------------------------------- the quit

since=$(mark)
quit_app
sleep 1
expect_log "quitting closes the link to the cube" "$since" "Quit: the link to the cube is closed"
check "and records the app is no longer connected" "0" "$(setting connection connected)"
check_contains "and when it was asked to quit" "$(setting connection quit_request)" "$(date '+%Y-%m-%d')"
check "the pairing survives it" "1" "$(setting paired paired)"
check "and so does the handle" "$paired_uuid" "$(setting device_uuid uuid)"

# ---------------------------------------------------------------------------- the launch, with the window shut

since=$(mark)
ensure_app_running
step "launched; nothing will be pressed from here until the reconnect is checked"
expect_log "a paired app goes looking for its cube by itself" "$since" "Looking for the paired cube" 20
expect_log "and the menu bar says it is connecting" "$since" "Menu bar reads Connecting..." 20
expect_log "and opens a link to the one it has on record" "$since" "Connecting to $paired_uuid" 40
first=$(wait_for "$since" "password withResponse: %" 30)
if [ -n "$first" ] && [[ "$first" != *"(000000)"* ]]; then
    pass "the stored PIN is presented first, not the factory one"
else
    fail "the first PIN presented was '${first:-none}'"
fi
expect_log "and the cube lets it in" "$since" "PIN accepted" 60
expect_log "it is recorded as getting back to the device, not as a new pairing" "$since" \
    "Reconnected to % ($paired_uuid)" 60
check "nothing is recorded as a fresh pairing" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Paired with %';")"
check "and the cube's PIN is left as it was" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message = 'Setting the PIN on the cube to one of its own';")"
check "the table says the cube is reachable again" "1" \
    "$(wait_sql "1" "SELECT json_extract(setting_value, '\$.connected') FROM setting WHERE setting_name = 'connection';" 15)"
check "and it is still the same device" "$paired_uuid" "$(setting device_uuid uuid)"
if settings_is_open; then
    fail "the Settings window is open, so this proved nothing about an app nobody is watching"
else
    pass "and none of it needed the Settings window, which has been shut throughout"
fi

open_settings
select_tab Device
check_contains "the tab reads as connected, from the row the reconnect wrote" \
    "$(element_eventually device-connection "Connected")" "Connected"
check_contains "and shows the charge the reconnect read" "$(element device-battery)" "%"
check "and offers Forget rather than Scan" "1" "$(on_tab device-forget)"

# ---------------------------------------------------------------------------- Forget

since=$(mark)
press device-forget
expect_log "pressing Forget is heard" "$since" "Button clicked: Forget Device"
expect_log "and the device is forgotten" "$since" "Forgot the device;%"
check "the table says nothing is paired" "0" "$(setting paired paired)"
check "or connected" "0" "$(setting connection connected)"
check "the handle is gone" "" "$(setting device_uuid uuid)"
check "and so is what the cube said it was" "" "$(setting device_info firmware)"
check_contains "but the name it carries is kept, so a scan still knows it" "$(setting device_name name)" "TimeFlip"
check_contains "the tab says there is no device" "$(element_eventually device-connection "Manual mode")" \
    "Manual mode, no device"
check_contains "and the name row says not paired" "$(element device-name)" "Not paired"
check "Scan is back" "1" "$(on_tab device-scan)"
check "and Forget is gone" "0" "$(on_tab device-forget)"
check "the settings are dead again" "1" "$(tree | grep -cE "id=device-auto-pause([[:space:]].*)?disabled" || true)"

# ---------------------------------------------------------------------------- a launch with nothing paired

close_settings
quit_app
sleep 1
since=$(mark)
ensure_app_running
wait_for "$since" "Facet is in the menu bar%" 20 >/dev/null
sleep 5
check "a launch with nothing paired does not go looking" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message = 'Looking for the paired cube';")"
check "and connects to nothing" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Connecting to %';")"

open_settings
select_tab Device
restore_the_pairing
check "the cube is paired again for the scripts after this one" "$paired_uuid" "$(setting device_uuid uuid)"
close_settings
finish
