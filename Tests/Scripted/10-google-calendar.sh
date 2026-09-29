#!/bin/bash
# The Google section, and recorded time reaching the calendar `03` made.
#
# **Fails rather than skips when no account is connected**, as `03` does: `00-setup` seeds the account, so a run
# without one has lost the fixture this script is about. To connect one by hand: Settings -> App -> Sign in with
# Google.
#
# **Converted from the Swift suite 2026-09-29**, against `feature/swiftParity`. The Rust sweep's rows are
# `Calendar sync started (REASON), N waiting`, `Calendar sync finished, N events into NAME` and `Calendar sync stopped
# after S of N`.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=10
start "the Google account, its calendar, and an entry syncing to it"

account() { sql "SELECT json_extract(setting_value, '\$.$1') FROM setting WHERE setting_name = 'google_account';"; }

open_settings
select_tab App

email=$(account email)
if [ -z "$email" ]; then
    fail "no Google account is connected, so the calendar and the sync cannot be checked"
    step "the google_account row is: $(sql "SELECT setting_value FROM setting WHERE setting_name = 'google_account';")"
    finish
    exit 1
fi
pass "an account is connected ($email)"
check_contains "and the Status row says so" "$(element_eventually app-google-status "Connected")" "Connected"

# ---------------------------------------------------------------------------- the calendar 03 made
#
# **Made in `03`, before anything was recorded**, because an entry sweeps every unsynced row into whatever calendar
# the app holds. What this script is about is that recorded time reaches it.

calendar_id=$(account calendar_id)
calendar_name=$(account calendar_name)
if [ -z "$calendar_id" ]; then
    fail "03 made no calendar, so there is nothing to sync into"
    finish
    exit 1
fi
pass "03 left a calendar to sync into ($calendar_name)"
check_contains "and the Calendar row shows it" "$(element app-google-calendar)" "$calendar_name"

# ---------------------------------------------------------------------------- an entry reaching it
#
# **The event is inserted, read back and compared before the row is ticked**, so a tick here means the event is at
# Google and is right, not that a request returned 200.

select_tab Faces
NAME=$(next_name Sync)
since=$(mark)
press create-category
sleep 0.5
set_field category-name-field "$NAME"
press save-category
sleep 1.5
ID=$(created_category_id "$since")
if [ -z "$ID" ]; then
    fail "could not create a category to record against"
    finish
    exit 1
fi

BLIP=$(sql "SELECT json_extract(setting_value, '\$.seconds') FROM setting WHERE setting_name = 'blip_time';")
BLIP=${BLIP:-5}

# **Nothing is clicked to start it.** Creating a category on the Faces tab starts timing it.
since=$(mark)
sleep $((BLIP + 4))
press timing-play-pause
sleep 2

entry=$(sql "SELECT time_entry_id FROM time_entry WHERE category_id = $ID ORDER BY time_entry_id DESC LIMIT 1;")
if [ -z "$entry" ]; then
    fail "no entry was recorded, so there is nothing for the sweep to carry"
    finish
    exit 1
fi
pass "an entry to sync (id $entry)"

expect_log "recording an entry starts a sweep" "$since" "Calendar sync started (%), % waiting" 30

# Two round trips per entry at about a second each, so this waits longer than anything else here.
if [ "$(wait_sql "1" "SELECT synced_to_google_calendar FROM time_entry WHERE time_entry_id = $entry;" 60)" = "1" ]; then
    pass "the entry is marked synced, which means its event was read back and checked"
else
    reason=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND tag = 'sync' ORDER BY debug_log_id DESC LIMIT 1;")
    fail "the entry is still unsynced after 60s (last sync line: ${reason:-none})"
fi

expect_log "and the sweep says what it did" "$since" "Calendar sync finished, % into $calendar_name%" 30
check "the sweep did not stop part way" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Calendar sync stopped%';")"
check "and nothing recorded is left waiting" "0" \
    "$(sql "SELECT COUNT(*) FROM time_entry WHERE synced_to_google_calendar = 0;")"

close_settings
finish
