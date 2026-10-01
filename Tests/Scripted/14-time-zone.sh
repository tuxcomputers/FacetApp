#!/bin/bash
# The zone a time entry is filed under is the machine's own, and the local time stored beside it is the machine's own
# local time for that moment.
#
# **New with the Rust app, 2026-10-01.** The Swift suite never looked at it, and for the first weeks of the Rust app
# every row was filed under Unknown (`timezone_id` 0) without anything failing, because the column defaults to 0 and
# `06-time-entries` only asks that it is not NULL.
#
# **The machine's zone and its local times are read from the operating system** (`platform_zone_name`,
# `platform_date_from_epoch`), not from the app, so the app is compared with the machine and not with itself. Both
# follow the environment's `TZ` when one is set, and the zone name does not, so run this with `TZ` unset.
#
# **A legacy name is compared through `timezone_lookup`**, the way the app files it: a machine set to `Asia/Calcutta`
# is filed under `Asia/Kolkata`, and that is a match.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=9
start "a time entry filed under this machine's zone"

ZONE=$(platform_zone_name)
if [ -z "$ZONE" ]; then
    fail "could not read this machine's time zone from the operating system"
    finish
    exit 1
fi
step "this machine is in $ZONE"

open_settings
select_tab Faces

BLIP=$(setting blip_time seconds)
BLIP=${BLIP:-5}

NAME=$(next_name Zone)
since=$(mark)
press create-category
sleep 0.5
set_field category-name-field "$NAME"
press save-category
sleep 1
ID=$(created_category_id "$since")
if [ -z "$ID" ]; then
    fail "could not create a category to record against"
    finish
    exit 1
fi

# **Nothing is clicked to start it.** Creating a category on the Faces tab assigns it to a face and starts timing it.
since=$(mark)
sleep $((BLIP + 4))
press timing-play-pause
sleep 2

expect_log "closing a segment over blip_time writes an entry" "$since" "time_entry created id=%category=$ID %"

entry=$(sql "SELECT time_entry_id FROM time_entry WHERE category_id = $ID ORDER BY time_entry_id DESC LIMIT 1;")
if [ -n "$entry" ]; then
    pass "the entry is in the table (id $entry)"
else
    fail "no time_entry row for category $ID"
    finish
    exit 1
fi

# ---------------------------------------------------------------------------- the zone

# What the table calls this machine's zone: the canonical name, which is the machine's own unless it uses a legacy one.
EXPECTED=$(sql "SELECT timezone_name FROM timezone WHERE timezone_id = (SELECT timezone_id FROM timezone_lookup WHERE timezone_name = '$ZONE');")
if [ -z "$EXPECTED" ]; then
    fail "$ZONE, this machine's zone, is not in the timezone table, so no row can have been filed under it"
    finish
    exit 1
fi
step "which the table calls $EXPECTED"

segment=$(sql "SELECT device_event_id FROM time_entry WHERE time_entry_id = $entry;")

check "the entry starts in this machine's zone" "$EXPECTED" \
    "$(sql "SELECT tz.timezone_name FROM time_entry te JOIN timezone tz ON tz.timezone_id = te.start_timezone_id WHERE te.time_entry_id = $entry;")"
check "and ends in it" "$EXPECTED" \
    "$(sql "SELECT tz.timezone_name FROM time_entry te JOIN timezone tz ON tz.timezone_id = te.end_timezone_id WHERE te.time_entry_id = $entry;")"
check "the segment it came from is filed under it too" "$EXPECTED" \
    "$(sql "SELECT tz.timezone_name FROM device_event de JOIN timezone tz ON tz.timezone_id = de.timezone_id WHERE de.device_event_id = $segment;")"

# ---------------------------------------------------------------------------- the local time beside it

START=$(sql "SELECT start_epoch FROM device_event WHERE device_event_id = $segment;")
SECONDS_LONG=$(sql "SELECT CAST(duration_seconds AS INTEGER) FROM time_entry WHERE time_entry_id = $entry;")
END=$((START + SECONDS_LONG))
FORMAT="%Y-%m-%dT%H:%M:%S"

check "the segment's start is the machine's local time for that moment" "$(platform_date_from_epoch "$START" "$FORMAT")" \
    "$(sql "SELECT start_time FROM device_event WHERE device_event_id = $segment;")"
check "the entry's start is the same" "$(platform_date_from_epoch "$START" "$FORMAT")" \
    "$(sql "SELECT started_at FROM time_entry WHERE time_entry_id = $entry;")"
check "and its end is the machine's local time for the end" "$(platform_date_from_epoch "$END" "$FORMAT")" \
    "$(sql "SELECT ended_at FROM time_entry WHERE time_entry_id = $entry;")"

# ---------------------------------------------------------------------------- the trace

# **The newest row, because a trace file carries rows from launches before the zone was recorded.** The trace has a
# `timezone` table of its own with the same ids, so it is asked in its own database.
check "the trace files its rows under this machine's zone too" \
    "$(dsql "SELECT timezone_name FROM timezone WHERE timezone_id = (SELECT timezone_id FROM timezone_lookup WHERE timezone_name = '$ZONE');")" \
    "$(dsql "SELECT tz.timezone_name FROM debug_log dl JOIN timezone tz ON tz.timezone_id = dl.timezone_id ORDER BY dl.debug_log_id DESC LIMIT 1;")"

finish
