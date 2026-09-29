#!/bin/bash
# The Device tab's Pause the device when locking it: a setting no command carries, so the whole of changing it is the
# table taking it and nothing going to the cube. Turned off and back on, and both directions are checked.
#
# **New with the Rust app, 2026-09-28.** The Swift suite checked the setting only through what the lock does with it
# (`61-lock-without-pause`), and locking is not built yet, so this checks the row until it is.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=7
start "pause on lock, kept in the table and sent nowhere"

require_a_paired_cube "the setting is dead without a cube"

open_settings
select_tab Device

was=$(setting pause_on_lock enabled)
check "the setting starts on, as seeded" "1" "$was"

since=$(mark)
press device-pause-on-lock
expect_log "unticking it is written to the table" "$since" "App setting pause_on_lock.enabled -> flag(false)"
check "and the table holds it" "0" "$(setting pause_on_lock enabled)"
check "nothing is sent to the cube for it" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND tag = 'ble-tx' AND message LIKE 'command withResponse:%';")"

since=$(mark)
press device-pause-on-lock
expect_log "ticking it again is written too" "$since" "App setting pause_on_lock.enabled -> flag(true)"
check "and the table is back where it was" "1" "$(setting pause_on_lock enabled)"
check "with no notice raised along the way" "no" "$(alert_is_open && echo yes || echo no)"

close_settings
finish
