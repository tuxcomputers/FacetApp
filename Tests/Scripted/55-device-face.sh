#!/bin/bash
# The face the cube rests on: asked for when the link comes up, followed on every turn after, and its history filed
# into device_event so the Faces tab and the figure follow the cube rather than the app's own clock.
#
# **Asks for hands twice**: a turn onto Meeting (face 2) and back onto Break (face 8). Each is detected from the app's
# own `Face N is up` row, so a turn onto some other face simply does not satisfy it.
#
# **Starts from the cube resting on Break, running**, which `51-device-connect` asks for and `relink_a_cube` puts back
# after the quit it makes.
#
# **Converted from the Swift suite 2026-09-29**, against `feature/timeTracking`. The wording is the Rust app's.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=34
start "the cube's face, followed, and its history filed"

require_a_paired_cube "there is no face to follow"
BREAK=$(sql "SELECT category_id FROM category WHERE category_name = 'Break';")
MEETING=$(sql "SELECT category_id FROM category WHERE category_name = 'Meeting';")

# `open_cube_row <field>`: a column of the open segment on a cube face, empty when none is open.
open_cube_row() {
    sql "SELECT $1 FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC, device_event_id DESC LIMIT 1;"
}

# ---------------------------------------------------------------------------- the link coming up

since=$(mark)
if relink_a_cube; then
    pass "the app came back up, reached the cube and freed it"
else
    fail "the relaunch did not reach and free the cube within its time"
    finish
    exit 1
fi
expect_log "the login sets the cube's clock" "$since" "Setting the clock on the cube to %"
expect_log "and reads it back within tolerance" "$since" "The clock on the cube is set"
expect_log "it follows the face" "$since" "Following the face"
expect_log "and asks which face is up" "$since" "Asking the cube which face is up"
expect_log "the cube is resting on Break" "$since" "Face 8 is up"
expect_log "it follows the history" "$since" "Following the history"
expect_log "and fetches it as the link comes up" "$since" "Fetching history (the link came up)%" 20
expect_log "the fetch finishes" "$since" "History fetch done (the link came up):%" 30
expect_log "and the history timer starts" "$since" "History timer started, asking every %s" 20
check "a segment is open on face 8" "8" "$(wait_sql 8 "SELECT device_face FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC LIMIT 1;" 30)"
check "and it is counting, not a pause" "0" "$(open_cube_row paused)"

# ---------------------------------------------------------------------------- the tab and the figure

open_settings
select_tab Faces
check_contains "the tab names the face the cube is on" "$(element_eventually timing-device-face "Face 8")" "Face 8"
check_contains "and the category that face holds" "$(element timing-category-name)" "Break"
check "the menu bar names it" "Menu bar reads Break" \
    "$(dsql "SELECT message FROM debug_log WHERE message LIKE 'Menu bar reads %' ORDER BY debug_log_id DESC LIMIT 1;")"
check "in green, the clock being the cube" "Menu bar: name green, figure green" \
    "$(dsql "SELECT message FROM debug_log WHERE message LIKE 'Menu bar: name %' ORDER BY debug_log_id DESC LIMIT 1;")"
check_contains "Break's face is locked, so the button offers to unlock it" "$(element timing-face-lock)" "Unlock face"
check_contains "the square is the cube, lit for Break" "$(element timing-cube)" "face 8, lit for Break"
check_contains "with Break's icon on its centre face" "$(element timing-centre-icon)" "Break"
check_contains "and a glyph beside the figure says it is running" "$(element timing-face-glyph)" "Cube running"
first=$(element timing-face-elapsed)
sleep 3
if [ "$(element timing-face-elapsed)" != "$first" ]; then
    pass "the figure moves between fetches"
else
    fail "the figure stood still for 3s: $first"
fi
since=$(mark)
press "category-row-$MEETING"
sleep 1
check "the rows are dead while the face is locked, so a press does nothing" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Button clicked: category_id%';")"
check "and face 8 still holds Break" "$BREAK" "$(sql "SELECT category_id FROM face WHERE face_id = 8;")"

id=$(open_cube_row device_event_id)
before=$(open_cube_row duration_seconds)
since=$(mark)
expect_log "the history timer asks again by itself" "$since" "History timer fired, asking on a %s interval" 40
wait_for "$since" "History fetch done (the timer asked):%" 20 >/dev/null
check "the open segment is the same row" "$id" "$(open_cube_row device_event_id)"
after=$(open_cube_row duration_seconds)
if awk "BEGIN { exit !(${after:-0} > ${before:-0}) }"; then
    pass "and its duration grew in place (${before}s to ${after}s)"
else
    fail "its duration did not grow: ${before:-none} then ${after:-none}"
fi

# ---------------------------------------------------------------------------- a turn

since=$(mark)
announce "the cube turned onto Meeting"
if ask_and_detect "$(on_face_now "$since" 2)" "Turn the cube to the Meeting face" \
    "That is face 2, the one lit cyan. The app carries on once it says face 2 is up."; then
    verdict_pass
else
    verdict_fail "there was no terminal to ask, so the cube was never turned"
    finish
    exit 1
fi
expect_log "a turn fetches history" "$since" "Fetching history (the cube was turned)%" 20
check "a new segment is open on face 2" "2" "$(wait_sql 2 "SELECT device_face FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC LIMIT 1;" 30)"
check "and the Break segment is closed" "1" "$(sql "SELECT finalised FROM device_event WHERE device_event_id = $id;")"
check "and handed on, as time or as a blip" "1" "$(sql "SELECT processed FROM device_event WHERE device_event_id = $id;")"
check_contains "the tab follows the cube onto face 2" "$(element_eventually timing-device-face "Face 2")" "Face 2"

since=$(mark)
if ask_and_detect "$(on_face_now "$since" 8)" "Turn the cube back to the Break face" \
    "That is face 8, the one lit red. Leave it there for the scripts after this one."; then
    pass "the cube is back on Break"
else
    fail "there was no terminal to ask, so the cube was never turned back"
fi
check "and a segment is open on face 8 again" "8" "$(wait_sql 8 "SELECT device_face FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC LIMIT 1;" 30)"

close_settings
finish
