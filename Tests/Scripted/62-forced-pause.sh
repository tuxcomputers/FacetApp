#!/bin/bash
# The app stopping the cube itself: a face with no category, and a category that has spent its daily limit.
#
# **Asks for hands twice**: a turn onto a face with no category (any face but Meeting's and Break's, which are the
# only two with one as seeded), and back onto Break at the end.
#
# **The daily limit is staged straight into the tables**, as `12-daily-limit` stages it: a one-minute limit and a
# finished minute already spent today, so the check does not wait a minute for it.
#
# **Converted from the Swift suite 2026-09-29**, against `feature/timeTracking`. The wording is the Rust app's.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=16
start "the app stopping the cube: no category, and a spent daily limit"

require_a_paired_cube "there is no cube to stop"
BREAK=$(sql "SELECT category_id FROM category WHERE category_name = 'Break';")
MEETING=$(sql "SELECT category_id FROM category WHERE category_name = 'Meeting';")
X=$(sql "SELECT category_id FROM category WHERE active = 1 AND category_id NOT IN (0, $BREAK, $MEETING) ORDER BY category_id LIMIT 1;")
X_NAME=$(sql "SELECT category_name FROM category WHERE category_id = ${X:-0};")
if [ -z "$X" ]; then
    red "  there is no active category besides Break and Meeting to put on a face; 04-categories makes them"
    finish
    exit 2
fi

open_paused() {
    wait_sql "$1" "SELECT paused FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC, device_event_id DESC LIMIT 1;" 20
}

# ---------------------------------------------------------------------------- a face with no category

since=$(mark)
announce "the cube turned onto a face with no category"
if ask_and_detect \
    "SELECT message FROM debug_log WHERE debug_log_id = (SELECT MAX(debug_log_id) FROM debug_log WHERE tag = 'face' AND message LIKE 'Face % is up') AND debug_log_id > $since AND message NOT IN ('Face 2 is up', 'Face 8 is up');" \
    "Turn the cube to a face with no category" \
    "Any face that is dark: not Meeting (cyan) and not Break (red)." \
    "The app should stop the cube as soon as it counts there."; then
    verdict_pass
else
    verdict_fail "there was no terminal to ask, so the cube was never turned"
    finish
    exit 1
fi
FACE=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND tag = 'face' AND message LIKE 'Face % is up' ORDER BY debug_log_id DESC LIMIT 1;" | sed -E 's/^Face ([0-9]+) is up$/\1/')
step "the cube is on face $FACE"
expect_log "the app stops the cube, the face holding no category" "$since" \
    "Forced pause: face $FACE has no category, so the cube is being stopped" 30
check "and the open segment is a pause" "1" "$(open_paused 1)"

open_settings
select_tab Faces
check_contains "the tab names that face" "$(element_eventually timing-device-face "Face $FACE")" "Face $FACE"
since=$(mark)
press "category-row-$X"
expect_log "a click puts $X_NAME on the face" "$since" "Face $FACE now holds $X_NAME (category_id $X)"
expect_log "and the app starts the cube again" "$since" \
    "Forced pause lifted: the face has a category now, so the cube is being started" 20
check "the open segment counts" "0" "$(open_paused 0)"
expect_log "and the face is relit in $X_NAME's colour" "$since" "The cube took face $FACE $X_NAME % (face $FACE took $X_NAME)%" 15
close_settings

# ---------------------------------------------------------------------------- a spent daily limit

sql "UPDATE category SET daily_limit = 1 WHERE category_id = $X;"
started=$(( $(date +%s) - 180 ))
while [ -n "$(sql "SELECT 1 FROM device_event WHERE event_number = $started AND start_epoch = $started;")" ]; do
    started=$(( started - 1 ))
done
sql "INSERT INTO device_event (event_number, event_type_id, device_face, start_time, timezone_id, start_epoch,
                               duration_seconds, paused, finalised, processed)
     VALUES ($started, 1, 13, strftime('%Y-%m-%dT%H:%M:%S', $started, 'unixepoch', 'localtime'), 0, $started,
             60, 0, 1, 1);"
event=$(sql "SELECT device_event_id FROM device_event WHERE start_epoch = $started AND event_number = $started;")
sql "INSERT INTO time_entry (category_id, device_event_id, started_at, start_timezone_id, ended_at, end_timezone_id,
                             duration_seconds, synced_to_google_calendar)
     VALUES ($X, $event, strftime('%Y-%m-%dT%H:%M:%S', $started, 'unixepoch', 'localtime'), 0,
             strftime('%Y-%m-%dT%H:%M:%S', $started + 60, 'unixepoch', 'localtime'), 0, 60, 1);"
check "a minute already spent against a one-minute limit is staged" "1" \
    "$(sql "SELECT COUNT(*) FROM time_entry WHERE device_event_id = ${event:-0};")"

since=$(mark)
expect_log "the app stops the cube, the category having spent its day" "$since" \
    "Daily limit reached: $X_NAME has spent %m, stopping the clock" 20
check "and the open segment is a pause" "1" "$(open_paused 1)"

since=$(mark)
menu_press toggle-pause
expect_log "Resume is refused while the limit holds" "$since" \
    "The cube is left stopped: the category on show has spent its daily limit" 10
check "and nothing is sent to start it" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message = 'Sending 06 02';")"

sql "UPDATE category SET daily_limit = 0 WHERE category_id = $X;"
since=$(mark)
menu_press toggle-pause
expect_log "with the limit lifted, Resume starts it" "$since" "The cube is running" 15

# ---------------------------------------------------------------------------- back onto Break

since=$(mark)
if ask_and_detect "$(on_face_now "$since" 8)" "Turn the cube back to the Break face" \
    "That is face 8, the one lit red. Leave it there for the scripts after this one."; then
    pass "the cube is back on Break"
else
    fail "there was no terminal to ask, so the cube was never turned back"
fi
check "and counting there" "0" "$(open_paused 0)"
finish
