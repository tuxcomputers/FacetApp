#!/bin/bash
# Putting the cube back to how it left the factory, and proving it took.
#
# **The proof is the cube taking the vendor PIN on a connection of its own.** `0xFF` is acknowledged at once, the link
# stays up through the wipe, and the cube goes on taking its old PIN for several seconds (firmware finding 6). So the
# app lets go of the link and presents `000000` every three seconds until the cube accepts it, and only then forgets
# it. The ordering is the assertion.
#
# **Ends by pairing again**, with `restore_the_pairing`, so the scripts after this one still have a cube. The cube is
# on the factory PIN by then, so that pairing moves it onto one of this app's own, which is checked.
#
# **Converted from the Swift suite 2026-09-28.** The wording is the Rust app's.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=31
start "factory resetting the cube, and proving it took"

require_a_paired_cube "there is nothing to reset"
name_before=$(setting device_name name)
step "the cube is called ${name_before:-nothing yet}"

open_settings
select_tab Device
check "Reset is live, the cube being connected" "0" "$(tree | grep -cE "id=device-reset([[:space:]].*)?disabled" || true)"
check "Forget is offered beside it" "1" "$(on_tab device-forget)"
check "and Scan is not, there being a cube" "0" "$(on_tab device-scan)"

# ---------------------------------------------------------------------------- asked, and called off

since=$(mark)
press device-reset
expect_log "pressing Reset is heard" "$since" "Button clicked: Reset Device"
check_contains "it warns that a reset cannot be undone" "$(notice_text "cannot be undone")" \
    "cannot be undone"
check "and offers a way out" "Cancel|Reset Device" "$(alert_buttons)"
press_title Cancel
expect_log "Cancel calls it off" "$since" "The reset was called off"
check "nothing went to the cube" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND tag = 'ble-tx' AND message LIKE 'command %FF%';")"
check "the cube is still paired" "1" "$(setting paired paired)"
check "and the notice has gone" "no" "$(alert_is_open && echo yes || echo no)"

# ---------------------------------------------------------------------------- confirmed

since=$(mark)
confirm_the_reset || red "  Reset Device asked nothing, so the reset below will not be sent"
expect_log "the reset command is sent" "$since" "Sending ff" 20
expect_log "as 0xFF on the command characteristic" "$since" "command withResponse: FF"
expect_log "the cube acknowledges it" "$since" "command: write acknowledged" 20
expect_log "and the app lets go of the link, the cube keeping it up" "$since" \
    "The cube took the reset and keeps the link up%" 20
step "waiting for the cube to come back on the factory PIN (up to two minutes)..."
announce "the wipe is proved"
outcome=$(wait_for "$since" "Reset: %" 140)
if [ "$outcome" = "Reset: confirmed" ]; then
    verdict_pass
else
    verdict_fail "the reset ended as '${outcome:-nothing within 140s}'"
    close_settings
    finish
    exit 1
fi
reset_at=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND message = 'command withResponse: FF';")
proof=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > ${reset_at:-0} AND tag = 'ble-tx' AND message LIKE 'password withResponse: %' ORDER BY debug_log_id DESC LIMIT 1;")
check_contains "the proof was the factory PIN, presented after the reset" "$proof" "(000000)"
expect_log "and the cube took it" "$since" "The cube is on the factory PIN, so the reset took"
check "the proving login is not recorded as a pairing" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Paired with %';")"
expect_log "the cube is forgotten" "$since" "Reset the cube and forgot it"

check "the table says nothing is paired" "0" "$(setting paired paired)"
check "the handle is gone" "" "$(setting device_uuid uuid)"
check "nothing is connected" "0" "$(setting connection connected)"
check "what the cube said it was is gone" "" "$(setting device_info firmware)"
check "and so is its name" "" "$(setting device_name name)"
check "which is kept for the scan instead" "$name_before" "$(setting device_name previous_name)"

check_contains "the tab says the cube is back to factory settings" \
    "$(element_eventually device-scan-status "factory settings")" "back to factory settings"
check_contains "the name row says not paired" "$(element device-name)" "Not paired"
check_contains "and the connection row that there is no device" "$(element device-connection)" "Manual mode, no device"
check "Scan is back" "1" "$(on_tab device-scan)"
check "and Reset is gone" "0" "$(on_tab device-reset)"

# ---------------------------------------------------------------------------- paired again, for what follows

since=$(mark)
restore_the_pairing
check "pairing a wiped cube moves it onto a PIN of this app's own" "1" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message = 'The cube is now on its new PIN';")"
close_settings
finish
