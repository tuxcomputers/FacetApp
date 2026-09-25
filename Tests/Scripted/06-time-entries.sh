#!/bin/bash
# A finished segment becoming tracked time, and a flick past a face not becoming anything.
#
# `device_event` is what a source says happened; `time_entry` is what the app counts. They are
# deliberately not the same question, and this is where the second one gets answered.
#
# **Converted from the Swift suite 2026-09-25.** The Google half of the sync check is gone with Google: the Rust
# app has no account to sync to, so an entry stays unsynced and that is asserted outright.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=12
start "a segment becoming a time entry, and a blip not"

open_settings
select_tab Faces

# The threshold this whole script is measured against, read from the table rather than assumed: it is a
# setting somebody can change, and a script carrying its own copy of it would be testing a number nobody
# is using.
BLIP=$(setting blip_time seconds)
BLIP=${BLIP:-5}
step "blip_time is ${BLIP}s"

NAME=$(next_name Entry)
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

# ---------------------------------------------------------------------------- time that counts

since=$(mark)
# **Nothing is clicked to start it.** Creating a category on the Faces tab assigns it to a face and starts timing
# it, so the clock has been running since the save above.
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

check "it is filed under the category that was timed" "$ID" "$(sql "SELECT category_id FROM time_entry WHERE time_entry_id = $entry;")"

seconds=$(sql "SELECT duration_seconds FROM time_entry WHERE time_entry_id = $entry;")
if awk "BEGIN{exit !($seconds >= $BLIP)}"; then
    pass "its duration is over blip_time (${seconds}s)"
else
    fail "the entry lasted ${seconds}s, which is under the ${BLIP}s threshold it was supposed to clear"
fi

# The end is the start plus the length, both through the segment's own zone.
check "both ends carry a zone" "0" "$(sql "SELECT COUNT(*) FROM time_entry WHERE time_entry_id = $entry AND (start_timezone_id IS NULL OR end_timezone_id IS NULL);")"

# **No account, so nowhere to sync it.** The Rust app has no Google half yet; when it does, this check takes back
# the Swift version's branch on whether an account is connected.
check "it stays unsynced, there being nowhere to sync it" "0" "$(sql "SELECT synced_to_google_calendar FROM time_entry WHERE time_entry_id = $entry;")"

# **One entry per segment, as a constraint rather than a convention** (`UN1_time_entry`).
segment=$(sql "SELECT device_event_id FROM time_entry WHERE time_entry_id = $entry;")
check "exactly one entry for that segment" "1" "$(sql "SELECT COUNT(*) FROM time_entry WHERE device_event_id = $segment;")"
check "and the segment is marked processed" "1" "$(sql "SELECT processed FROM device_event WHERE device_event_id = $segment;")"

# ---------------------------------------------------------------------------- a blip
#
# The cube being turned past a face rather than time spent on it. No entry, ever -- and the segment is
# still marked processed, because the question has been answered and leaving it unprocessed would grow a
# tail of rows every later pass re-examines.

before=$(sql "SELECT COUNT(*) FROM time_entry WHERE category_id = $ID;")
since=$(mark)
press "category-row-$ID"
sleep 1
press timing-play-pause
sleep 2

# `Blip(n)` carries the threshold it fell under, which is the setting read above.
expect_log "a segment under blip_time is ignored" "$since" "time_entry: device_event % ignored, Blip($BLIP)"
check "it wrote no entry" "$before" "$(sql "SELECT COUNT(*) FROM time_entry WHERE category_id = $ID;")"

blip_row=$(sql "SELECT device_event_id FROM device_event WHERE device_face >= 13 ORDER BY device_event_id DESC LIMIT 1;")
check "the blipped segment is still marked processed" "1" "$(sql "SELECT processed FROM device_event WHERE device_event_id = $blip_row;")"

# **Not lost, just not counted.** The segment is on record; what it did not earn is an entry.
check "the segment itself is still on record" "1" "$(sql "SELECT COUNT(*) FROM device_event WHERE device_event_id = $blip_row;")"

finish
