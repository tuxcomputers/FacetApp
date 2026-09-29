#!/bin/bash
# The cube lit in its faces' colours: all twelve with 0x11 once a login has asked its own questions, and the faces a
# category wears relit when it is recoloured. There is no read-back, so the app's own row per face is the evidence.
#
# **Starts from the cube running on Break**, and leaves Break red and face 8 locked, as seeded.
#
# **Converted from the Swift suite 2026-09-29**, against `feature/timeTracking`. The wording is the Rust app's.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=16
start "the cube's face colours"

require_a_paired_cube "there is no cube to light"
BREAK=$(sql "SELECT category_id FROM category WHERE category_name = 'Break';")
RED=$(sql "SELECT colour_id FROM colour WHERE colour_name = 'Red';")
NAVY=$(sql "SELECT colour_id FROM colour WHERE colour_name = 'Navy';")
NAVY_HEX=$(sql "SELECT device_hex FROM colour WHERE colour_name = 'Navy';")

# ---------------------------------------------------------------------------- all twelve at connect

since=$(mark)
if relink_a_cube; then
    pass "the app came back up and reached the cube"
else
    fail "the relaunch did not reach and free the cube"
    finish
    exit 1
fi
wait_for "$since" "The cube took face 12 %(the cube connected)%" 30 >/dev/null
check "all twelve faces are lit as the link comes up" "12" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'The cube took face % (the cube connected), with no read-back to confirm it';")"
check "as twelve 0x11 writes" "12" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND tag = 'ble-tx' AND message LIKE 'command withResponse: 11 %';")"
reached=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Reconnected to %';")
first=$(dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'The cube took face %';")
if [ -n "$reached" ] && [ -n "$first" ] && [ "$first" -gt "$reached" ]; then
    pass "after the login's own questions, never before"
else
    fail "the first colour (${first:-none}) did not follow the login (${reached:-none})"
fi
check_contains "Break's face is lit red" \
    "$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND message LIKE 'The cube took face 8 %' LIMIT 1;")" "Break #ff0000 as rgb16 ffff,0000,0000"
check_contains "and a face with no category is off" \
    "$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND message LIKE 'The cube took face 5 %' LIMIT 1;")" "no category off as rgb16 0000,0000,0000"
sleep 3
check "and nothing more is sent after them" "12" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND tag = 'ble-tx' AND message LIKE 'command withResponse: 11 %';")"

# ---------------------------------------------------------------------------- a recolour

# Break is on face 8, which is locked as seeded, and a category on a locked face keeps its colour; the face is unlocked
# from the Faces tab for the recolour and locked again after it.
open_settings
select_tab Faces
since=$(mark)
press timing-face-lock
expect_log "the Faces tab unlocks the face the cube rests on" "$since" "Button clicked: face 8 lock -> unlocked"
check "and the table holds it" "0" "$(sql "SELECT locked FROM face WHERE face_id = 8;")"
select_tab Categories
since=$(mark)
press "category-colour-$BREAK"
sleep 0.5
press colour-option-Navy
check "Break takes Navy" "$NAVY" "$(sql "SELECT colour_id FROM category WHERE category_id = $BREAK;")"
expect_log "and the face it wears is relit in it" "$since" "The cube took face 8 Break $NAVY_HEX as rgb16 % (Break was recoloured), with no read-back to confirm it" 15
check "one face, Break being on one" "1" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'The cube took face % (Break was recoloured)%';")"

since=$(mark)
press "category-colour-$BREAK"
sleep 0.5
press colour-option-Red
check "Break goes back to Red" "$RED" "$(sql "SELECT colour_id FROM category WHERE category_id = $BREAK;")"
expect_log "and its face is relit red" "$since" "The cube took face 8 Break #ff0000 as rgb16 ffff,0000,0000 (Break was recoloured)%" 15
select_tab Faces
since=$(mark)
press timing-face-lock
expect_log "the face is locked again" "$since" "Button clicked: face 8 lock -> locked"
check "and the table holds it" "1" "$(sql "SELECT locked FROM face WHERE face_id = 8;")"
close_settings
finish
