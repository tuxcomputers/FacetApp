#!/bin/bash
# A cube that goes out of reach while it is timing, is turned to another face while nobody can hear it, and comes
# back.
#
# **What the app shows while it cannot hear the cube**: the menu bar line in yellow, and the Device tab saying
# Disconnected. **What it refuses to write**: no device_event and no time_entry row for the gap, and the open Break
# row left at the duration the cube last gave. **What the cube backfills once it is heard again**: the app looks for
# it by itself (2s, doubling to 30s), logs in, and fetches history, which closes the Break stretch at the duration the
# cube measured and files the turn as a Meeting segment carrying the cube's own start time. Turning the cube back to
# Break closes that segment and files it as tracked time under Meeting.
#
# **Asks for hands four times**: Bluetooth off, the cube onto Meeting (face 2, lit cyan), Bluetooth on, and the cube
# back onto Break (face 8, lit red). The turn onto Meeting is made while the app cannot hear the cube, so it is asked
# for with `action_required` and detected afterwards from what the backfill files. `watch_bluetooth` asks for the
# radio back on any way out of the script.
#
# **Starts and ends with the cube resting on Break, running, paired, unlocked and connected.**
#
# **Converted from the Swift suite 2026-09-29**, against `feature/swiftParity`. The wording is the Rust app's.
# Differences: the history timer stops when the link goes (`History timer stopped, the cube is not connected`), so the
# quiet window checks that it does not fire rather than that it does; the menu bar stays on the category in yellow
# while the app looks for the cube again, never `Connecting...`, which is only a launch reaching its cube;
# the Device tab is checked for Disconnected; and the gap segment is followed through to its time_entry.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=33
start "a cube out of reach: what the app shows, what it refuses to write, and what the cube backfills"

require_a_paired_cube "there is no cube to take out of reach"
BREAK=$(sql "SELECT category_id FROM category WHERE category_name = 'Break';")
MEETING=$(sql "SELECT category_id FROM category WHERE category_name = 'Meeting';")

# `open_cube_row <field>`: a column of the open segment on a cube face, empty when none is open.
open_cube_row() {
    sql "SELECT $1 FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC, device_event_id DESC LIMIT 1;"
}

# `column_of <field> <device_event_id>`: one column of one device_event row.
column_of() {
    sql "SELECT $1 FROM device_event WHERE device_event_id = $2;"
}

# `entry_category_of <device_event_id>`: the category of the time_entry filed from that row, polled until it is
# `want` or the wait runs out, answering what it last saw.
entry_category_of() {
    wait_sql "$2" "SELECT category_id FROM time_entry WHERE device_event_id = $1;" 30
}

# ---------------------------------------------------------------------------- the starting position

check "the cube arrives resting on Break, counting" "8|0" "$(open_cube_row "device_face || '|' || paused")"
check "and the menu bar draws it green, the cube being heard" "Menu bar: name green, figure green" \
    "$(dsql "SELECT message FROM debug_log WHERE message LIKE 'Menu bar: name %' ORDER BY debug_log_id DESC LIMIT 1;")"

ROW_A=$(open_cube_row device_event_id)
N_A=$(column_of event_number "$ROW_A")
INTERVAL=$(setting fetch_history_interval_seconds seconds)
INTERVAL=${INTERVAL:-10}
step "the Break segment is device_event $ROW_A, event $N_A; the history timer asks every ${INTERVAL}s"

# ---------------------------------------------------------------------------- the link goes

since=$(mark)
watch_bluetooth
BLUETOOTH_IS_OFF=1
announce "turning Bluetooth off drops the link, and the app notices"
if ask_and_detect \
    "SELECT message FROM debug_log WHERE debug_log_id > $since AND message = 'The cube is no longer connected';" \
    "Turn Bluetooth OFF" \
    "On the Mac: the Bluetooth item in Control Centre. On Linux: the Bluetooth applet on the panel." \
    "Leave the cube on Break for now. The turn is asked for once the app has noticed the drop."; then
    verdict_pass
else
    verdict_fail "there was no terminal to ask, so Bluetooth was never turned off"
    finish
    exit 1
fi
DROP=$(dsql "SELECT IFNULL(MAX(debug_log_id), 0) FROM debug_log WHERE debug_log_id > $since AND message = 'The cube is no longer connected';")
DROPPED_AT=$(date +%s)
expect_log "the drop is recorded" "$DROP" "The link to the cube dropped"
expect_log "and the app says it will look for the cube again in 2s" "$DROP" \
    "The cube went away; looking for it again in 2s"
# Either row: the link going stops the timer, or the timer firing as the link goes stops itself. Which one is written
# depends on whether a fetch had the timer unarmed at that moment.
expect_log "the history timer stops with the link" "$DROP" "History timer stopped, %"
expect_log "the menu bar turns yellow, the cube being out of reach" "$DROP" "Menu bar: name yellow, figure yellow" 15
check "the table says the cube is not connected" "0" "$(setting connection connected)"

DURATION_AT_DROP=$(column_of duration_seconds "$ROW_A")
ROWS_BEFORE=$(sql "SELECT COUNT(*) FROM device_event;")
ENTRIES_BEFORE=$(sql "SELECT COUNT(*) FROM time_entry;")

open_settings
select_tab Device
check_contains "the Device tab says disconnected" "$(element_eventually device-connection "Disconnected")" "Disconnected"
close_settings

expect_log "a look that finds nothing doubles the wait" "$DROP" "The cube went away; looking for it again in 4s" 45

# ---------------------------------------------------------------------------- nothing is invented

step "watching for $((INTERVAL + 2))s, longer than the history timer's interval..."
sleep $((INTERVAL + 2))
check "the history timer does not fire with no cube to ask" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $DROP AND message LIKE 'History timer fired%';")"
check "and nothing is fetched" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $DROP AND message LIKE 'Fetching history%';")"
check "and the menu bar never says Connecting... while it looks again" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $DROP AND message = 'Menu bar reads Connecting...';")"
check "the Break row is left open at the duration the cube last gave" "0|$DURATION_AT_DROP" \
    "$(column_of "finalised || '|' || duration_seconds" "$ROW_A")"

# ---------------------------------------------------------------------------- the turn nobody can hear

if ! action_required \
    "Turn the cube so the Meeting face is up" \
    "That is face 2, the one lit cyan. Bluetooth stays off, so nothing will appear on screen." \
    "The cube records the turn itself. Leave it on Meeting, then answer y."
then
    fail "the cube was not turned, so there is no backlog for the reconnect to bring in"
    finish
    exit 1
fi
check "nothing reached device_event while the cube was out of reach" "$ROWS_BEFORE" \
    "$(sql "SELECT COUNT(*) FROM device_event;")"
check "and nothing reached time_entry" "$ENTRIES_BEFORE" "$(sql "SELECT COUNT(*) FROM time_entry;")"

# ---------------------------------------------------------------------------- and it comes back

back=$(mark)
if ! action_required "Turn Bluetooth back ON" \
    "Turn it on the same way it went off, then answer y." \
    "The app finds the cube again by itself, with no relaunch. Leave the cube on Meeting."; then
    fail "Bluetooth was not turned back on, so the backlog was never brought in"
    finish
    exit 1
fi
BACK_AT=$(date +%s)
BLUETOOTH_IS_OFF=0
for _ in $(seq 1 30); do bluetooth_is_on && break; sleep 1; done

expect_log "the app reaches the cube again by itself" "$back" "Reconnected to %" 120
check "and the table says it is connected" "1" \
    "$(wait_sql "1" "SELECT json_extract(setting_value, '\$.connected') FROM setting WHERE setting_name = 'connection';" 30)"
expect_log "it fetches history as the link comes up" "$back" "Fetching history (the link came up)%" 30
expect_log "and the fetch finishes" "$back" "History fetch done (the link came up):%" 60

# ---------------------------------------------------------------------------- what the cube says happened

check "the Break stretch is closed off" "1" \
    "$(wait_sql "1" "SELECT finalised FROM device_event WHERE device_event_id = $ROW_A;" 30)"
DURATION_AFTER=$(column_of duration_seconds "$ROW_A")
if awk "BEGIN { exit !(${DURATION_AFTER:-0} > ${DURATION_AT_DROP:-0}) }"; then
    pass "at a duration the cube measured through the gap (${DURATION_AT_DROP}s to ${DURATION_AFTER}s)"
else
    fail "the Break stretch did not grow (${DURATION_AT_DROP:-none}s to ${DURATION_AFTER:-none}s)"
fi
check "and it is filed as tracked time under Break" "$BREAK" "$(entry_category_of "$ROW_A" "$BREAK")"

arrived=$(wait_sql "yes" \
    "SELECT CASE WHEN COUNT(*) > 0 THEN 'yes' ELSE 'no' END FROM device_event WHERE device_face = 2 AND device_event_id > $ROW_A AND event_number > ${N_A:-0};" 30)
NEW_B=""
[ "$arrived" = "yes" ] && NEW_B=$(sql "SELECT device_event_id FROM device_event WHERE device_face = 2 AND device_event_id > $ROW_A AND event_number > ${N_A:-0} ORDER BY device_event_id DESC LIMIT 1;")
if [ -n "$NEW_B" ]; then
    pass "the turn arrived as a segment of its own on face 2 (device_event $NEW_B, event $(column_of event_number "$NEW_B"))"
else
    fail "no segment on face 2 above event ${N_A:-0}, so the turn nobody could hear never reached the table"
fi
NEW_B=${NEW_B:-0}
check "and it is the open one, the cube still being on Meeting" "0" "$(column_of finalised "$NEW_B")"

B_START=$(column_of start_epoch "$NEW_B")
if [ -n "$B_START" ] && [ "$B_START" -ge $((DROPPED_AT - 10)) ] && [ "$B_START" -le $((BACK_AT + 10)) ]; then
    pass "it starts at the cube's own time for the turn, inside the gap ($B_START, between $DROPPED_AT and $BACK_AT)"
else
    fail "its start ${B_START:-none} is not between the drop at $DROPPED_AT and Bluetooth coming back at $BACK_AT"
fi

expect_log "the menu bar names Meeting" "$back" "Menu bar reads Meeting" 30
expect_log "and is green again, the cube being heard" "$back" "Menu bar: name green, figure green" 30

# ---------------------------------------------------------------------------- back onto Break

since=$(mark)
if ask_and_detect "$(on_face_now "$since" 8)" "Turn the cube back to the Break face" \
    "That is face 8, the one lit red. Leave it there for the scripts after this one."; then
    pass "the cube is back on Break"
else
    fail "there was no terminal to ask, so the cube was never turned back"
fi
check "the Meeting segment from the gap is closed" "1" \
    "$(wait_sql "1" "SELECT finalised FROM device_event WHERE device_event_id = $NEW_B;" 30)"
check "and filed as tracked time under Meeting" "$MEETING" "$(entry_category_of "$NEW_B" "$MEETING")"
check "the time entry starts when the cube says the turn was made" "$(column_of start_time "$NEW_B")" \
    "$(sql "SELECT started_at FROM time_entry WHERE device_event_id = $NEW_B;")"
check "and a segment is open on Break again, counting" "8|0" \
    "$(wait_sql "8|0" "SELECT device_face || '|' || paused FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC, device_event_id DESC LIMIT 1;" 30)"
finish
