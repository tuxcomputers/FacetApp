#!/bin/bash
# The Faces tab: picking a category starts the clock on it, and pausing stops it.
#
# **With no cube the app is its own source**, timing on faces 13 and 14, so what is checked here is the same
# machinery a device drives -- a segment opens in `device_event`, grows, and closes -- reached through the app
# rather than through a radio.
#
# **Converted from the Swift suite 2026-09-25.** Two things did not come across and both are absent from the app
# rather than from this script: the menu bar's colours (`expect_colours`), which the Rust status item does not
# draw, and the per-tick history timer, which is `07`'s subject and needs a cube to mean anything. In their place
# is the tray following the clock, which the Rust app does have (handover-linux 10).
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=29
start "starting, pausing and resuming the clock"

open_settings
select_tab Faces

# A category of this run's own, so what is timed below cannot be confused with a category somebody was
# already using.
NAME=$(next_name Timing)
since=$(mark)
press create-category
sleep 0.5
set_field category-name-field "$NAME"
press save-category
sleep 1.5
ID=$(created_category_id "$since")
if [ -z "$ID" ]; then
    fail "could not create a category to time against"
    finish
    exit 1
fi
pass "a category to time against ($NAME, id $ID)"

# ---------------------------------------------------------------------------- creating one starts it
#
# **On this tab, making a category is saying what you are doing now**, so the create assigns it to a face and
# starts it. The Rust trace names the category by id here and by name when pausing, so the id is what is matched.

expect_log "creating it on the Faces tab starts timing it" "$since" "Timing: started category_id $ID on face %"

# ---------------------------------------------------------------------------- starting, by picking a row
#
# **Break rather than the category just made**, because that one is already running and clicking it is the
# no-op checked below rather than a start.

BREAK=$(sql "SELECT category_id FROM category WHERE category_name = 'Break';")
since=$(mark)
press "category-row-$BREAK"
sleep 1.5
expect_log "picking a category starts timing it" "$since" "Timing: started category_id $BREAK on face %"

since=$(mark)
press "category-row-$BREAK"
sleep 1
expect_log "clicking the one already running changes nothing" "$since" "%already timing Break%"

since=$(mark)
press "category-row-$ID"
sleep 1.5
expect_log "and picking another moves the clock to it" "$since" "Timing: started category_id $ID on face %"

# A row in `device_event`, open, on one of the app's own faces. This is the fact behind the clock: the
# readout is drawn from it every time rather than from anything the click remembered.
open_row=$(sql "SELECT device_event_id FROM device_event WHERE finalised != 1 ORDER BY device_event_id DESC LIMIT 1;")
if [ -n "$open_row" ]; then
    pass "a segment is open in device_event (id $open_row)"
else
    fail "nothing is open in device_event, so nothing is really being timed"
fi

face=$(sql "SELECT device_face FROM device_event WHERE device_event_id = ${open_row:-0};")
if [ "${face:-0}" -ge 13 ]; then
    pass "it is on one of the app's own faces ($face)"
else
    fail "the segment is on face ${face:-none}, which is a cube's face and not the app's"
fi

# `face` keys on `face_id`; `device_face` is the column over on `device_event`. Two names for the same number.
check "the face holds the category" "$ID" "$(sql "SELECT category_id FROM face WHERE face_id = ${face:-0};")"

check_contains "the Timing column names it" "$(tree)" "id=timing-category-name  value=$NAME"

# ---------------------------------------------------------------------------- the tray follows it

# **The status item is the same clock**, not a second one: it is redrawn from the database after every re-read
# the tab makes. Pause offered and live while running.
pause_line=$(platform_menu_item toggle-pause)
check_contains "the tray offers Pause while it runs" "$pause_line" "'Pause'"
case "$pause_line" in
    *insensitive*) fail "and the tray's Pause is greyed while the clock runs" ;;
    *) pass "and it is live" ;;
esac

# ---------------------------------------------------------------------------- it actually runs
#
# Two readings apart. The duration is written by the one-second tick re-reporting the open segment, so this is
# also the check that the tick is doing its job end to end.
first=$(sql "SELECT duration_seconds FROM device_event WHERE device_event_id = $open_row;")
sleep 4
second=$(sql "SELECT duration_seconds FROM device_event WHERE device_event_id = $open_row;")
if awk "BEGIN{exit !($second > $first)}"; then
    pass "the open segment's duration grows while it runs ($first -> $second)"
else
    fail "the duration did not move in 4s ($first -> $second), so nothing is being measured"
fi

# ---------------------------------------------------------------------------- pausing

since=$(mark)
press timing-play-pause
sleep 1.5
expect_log "the play/pause control stops it" "$since" "Timing: stopped $NAME%"

# **Pausing closes the segment rather than flagging it.** An open row is what running means, so stopping means
# there is no open row.
check "nothing is left open" "0" "$(sql "SELECT COUNT(*) FROM device_event WHERE finalised != 1;")"
check "the segment that was running is finalised" "1" "$(sql "SELECT finalised FROM device_event WHERE device_event_id = $open_row;")"

expect_log "and the tray follows it to Resume" "$since" "Status item follows the clock, paused=true item=Resume enabled=true"

# ---------------------------------------------------------------------------- resuming, from the tray
#
# **The left click is Pause's accelerator and goes through the same toggle as the tab's glyph**, so resuming
# from it is resuming the tab's clock. Sent as the StatusNotifierItem `Activate` a panel sends on a left click.

since=$(mark)
activate_status_item
sleep 1.5
expect_log "the tray's left click starts it again on the same category" "$since" "Timing: running $NAME%"

resumed=$(sql "SELECT device_event_id FROM device_event WHERE finalised != 1 ORDER BY device_event_id DESC LIMIT 1;")
if [ -n "$resumed" ] && [ "$resumed" != "$open_row" ]; then
    pass "resuming opens a new segment rather than reopening the closed one"
else
    fail "resuming did not open a fresh segment (got '$resumed', was '$open_row')"
fi

# **The same face, deliberately.** Rotating faces exists so a face's category cannot change under a finished
# segment; resuming is the same category continuing, so it reuses the face.
check "on the same face it was using" "$face" "$(sql "SELECT device_face FROM device_event WHERE device_event_id = ${resumed:-0};")"

# And the tray's own menu item pauses it, which is the route the left click accelerates.
since=$(mark)
menu_press toggle-pause
sleep 1.5
expect_log "the tray's Pause item stops it" "$since" "Timing: stopped $NAME%"

# ---------------------------------------------------------------------------- a long name and the window
#
# **A category name has no maximum length on screen, only in the table**, and a label asks for its whole string.
# The window is 640 wide (CLAUDE.md) and a label demanding more would widen it. Created on this tab because that
# starts it, which is what puts the name in the big label as well as in the list.

number=$(next_name Widest); number=${number##* }
LONG="Widest allowed category name $number"
LONG="$LONG$(printf 'x%.0s' $(seq 1 $((35 - ${#LONG}))))"

before=$(window_width settings-window)
if [ -n "$before" ]; then
    pass "the window's width can be read before the long name exists ($before)"
else
    fail "the window's width could not be read, so nothing below can tell whether the name moves it"
fi

since=$(mark)
press create-category
sleep 0.5
set_field category-name-field "$LONG"
press save-category
sleep 1.5
expect_log "a name at the limit is saved whole" "$since" "%Save new category $LONG -> Some(%"

check_contains "and the Timing column carries all of it" "$(tree)" "id=timing-category-name  value=$LONG"

check "creating it does not widen the window" "$before" "$(window_width settings-window)"

# Every tab, ending on Faces because the control pressed below lives on it.
for tab in Categories Report App Device Faces; do
    select_tab "$tab"
    check "the $tab tab is still the width it was" "$before" "$(window_width settings-window)"
done

# Left paused, so the next script starts from a known state and the status item is not counting.
press timing-play-pause
sleep 1

finish
