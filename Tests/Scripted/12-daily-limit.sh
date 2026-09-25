#!/bin/bash
# The hard daily limit: reaching it stops the clock, and the app then refuses to start it again.
#
# **Staged without a cube**: the app is the clock, so the crossing is reached by seeding a total seconds short of
# the limit and pressing the category on the Faces tab, and the pause is the app closing its own segment.
#
# - **A category made for this run**, so the limit under test is spent by time this script inserted and nothing else.
# - **The seeded total sits seconds short**, so the crossing happens while the run is watching. Three times, from 19,
#   20 and 21 seconds out: the tick is once a second, so 20 lands *on* a tick and the other two fall either side.
#
# **Converted from the Swift suite 2026-09-25, and three things changed because the app has not got them yet.**
#
# - **The limit is written into `category` rather than typed on the Categories tab**, which is drawn but not wired:
#   it has no limit field. So this is a fixture, the same as the seeded time, and the app is trusted with none of
#   it; what is under test is what the app does with a limit it finds, read at the point of use. When the tab
#   gains the field, both writes go back through it, and the raise below becomes the check that an edit redraws
#   the tray (the Swift run 15 fault).
# - **The category is made on the Faces tab and paused at once**, because that is the only create there is and it
#   starts the clock. The second or so it runs is a blip and earns no entry, so the seeded total is untouched.
# - **The menu bar colours and label are gone**, the Rust status item drawing neither. The refusal is checked on
#   the tray's Pause item and its left click instead, which the Rust app does have.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=29
start "a category spending its daily limit, and the refusal that follows"

LIMIT_MINUTES=5
LIMIT_SECONDS=$(( LIMIT_MINUTES * 60 ))
REMAINING_CASES=(19 20 21)

open_settings
select_tab Faces

# Creates a category of this run's own, gives it the limit, and seeds it to within `$1` seconds of spending it.
# Sets STAGED_ID, STAGED_NAME and STAGED_SEEDED; returns 1 if anything failed.
#
# **Answers through globals rather than by printing**, deliberately: `ID=$(stage_category ...)` would run all of
# this in a subshell, losing the assignments and capturing every line `press` writes into the id.
stage_category() {
    local remaining="$1" name seeded created id started ended event

    STAGED_ID=""
    STAGED_NAME=""
    STAGED_SEEDED=""

    name=$(next_name Limit)
    created=$(mark)
    press create-category
    sleep 0.5
    set_field category-name-field "$name"
    press save-category
    sleep 1
    id=$(created_category_id "$created")
    [ -z "$id" ] && return 1

    # Stopped straight away: the create started it, and this script chooses when timing begins.
    press timing-play-pause
    sleep 1
    [ "$(sql "SELECT COUNT(*) FROM device_event WHERE finalised = 0;")" = "0" ] || return 1

    # **The fixtures, straight into the tables.** See the header for why the limit is one.
    sql "UPDATE category SET daily_limit = $LIMIT_MINUTES WHERE category_id = $id;"

    # **An ordinary finished segment and the entry it produced**, on an app face, so the total the app reads is
    # reached the same way any other total is. Marked synced so nothing would ever try to send a fixture anywhere.
    seeded=$(( LIMIT_SECONDS - remaining ))
    started=$(( $(date +%s) - seeded - 60 ))
    # **Shifted back until the identity is free**: `(event_number, start_epoch)` is unique, and here both are this
    # one number, so two stagings collide whenever the wall clock lines them up (Swift run 88).
    while [ -n "$(sql "SELECT 1 FROM device_event WHERE event_number = $started AND start_epoch = $started;")" ]; do
        started=$(( started - 1 ))
    done
    ended=$(( started + seeded ))

    sql "INSERT INTO device_event (
             event_number, event_type_id, device_face, start_time, timezone_id,
             start_epoch, duration_seconds, paused, finalised, processed
         ) VALUES (
             $started, 1, 13, strftime('%Y-%m-%dT%H:%M:%S', $started, 'unixepoch', 'localtime'), 0,
             $started, $seeded, 0, 1, 1
         );"
    event=$(sql "SELECT device_event_id FROM device_event WHERE start_epoch = $started AND event_number = $started;")
    # **Read back before anything is built on it**: `sql` reports a refused write on stderr and answers nothing.
    [ -z "$event" ] && return 1

    sql "INSERT INTO time_entry (
             category_id, device_event_id, started_at, start_timezone_id,
             ended_at, end_timezone_id, duration_seconds, synced_to_google_calendar
         ) VALUES (
             $id, $event,
             strftime('%Y-%m-%dT%H:%M:%S', $started, 'unixepoch', 'localtime'), 0,
             strftime('%Y-%m-%dT%H:%M:%S', $ended, 'unixepoch', 'localtime'), 0,
             $seeded, 1
         );"

    STAGED_ID="$id"
    STAGED_NAME="$name"
    STAGED_SEEDED="$seeded"
}

# ---------------------------------------------------------------------------- the crossing, from either side of a tick

for remaining in "${REMAINING_CASES[@]}"; do
    stage_category "$remaining"
    if [ -z "$STAGED_ID" ]; then
        fail "could not stage a category $remaining seconds short of its limit"
        finish
        exit 1
    fi
    ID="$STAGED_ID"
    NAME="$STAGED_NAME"
    on_tick=""
    [ "$remaining" -eq 20 ] && on_tick=", landing on a tick"
    pass "a category $remaining seconds short of its limit ($NAME, id $ID)$on_tick"

    check "its limit is stored, and it is $STAGED_SEEDED seconds into it" "$LIMIT_MINUTES|$STAGED_SEEDED" \
        "$(sql "SELECT daily_limit FROM category WHERE category_id = $ID;")|$(sql "SELECT CAST(IFNULL(SUM(duration_seconds), 0) AS INTEGER) FROM time_entry WHERE category_id = $ID;")"

    since=$(mark)
    press "category-row-$ID"
    sleep 1.5
    expect_log "picking it starts the clock" "$since" "Timing: started category_id $ID on face %"

    check "it is running, with $remaining seconds of budget left" "1" \
        "$(wait_sql "1" "SELECT COUNT(*) FROM device_event WHERE finalised = 0;")"

    # **The wait is the test.** Nothing here presses anything: what is being checked is that the app stops itself.
    step "waiting out the last $remaining seconds of the budget..."
    if wait_for "$since" "Daily limit reached, $NAME paused at %" $(( remaining + 25 )) >/dev/null; then
        pass "reaching the limit from $remaining seconds out stops the clock, and says so"
    else
        fail "the limit came and went with the clock still running ($remaining seconds out)"
        finish
        exit 1
    fi

    check "the open segment was closed" "0" \
        "$(wait_sql "0" "SELECT COUNT(*) FROM device_event WHERE finalised = 0;")"

    # **How far past the limit it stopped, which is the point of running three.** A second of overshoot is the tick
    # that noticed; more is the tick having been missed.
    total=$(sql "SELECT CAST(IFNULL(SUM(duration_seconds), 0) AS INTEGER) FROM time_entry WHERE category_id = $ID;")
    over=$(( ${total:-0} - LIMIT_SECONDS ))
    if [ "${total:-0}" -lt "$LIMIT_SECONDS" ]; then
        fail "it stopped ${total}s in, short of the ${LIMIT_SECONDS}s limit ($remaining seconds out)"
    elif [ "$over" -le 2 ]; then
        pass "and it stopped ${over}s past the limit (${total}s of ${LIMIT_SECONDS}s)"
    else
        fail "it overshot the limit by ${over}s (${total}s of ${LIMIT_SECONDS}s), which is more than a tick"
    fi
done

# Everything below runs against the last of the three, sitting spent and stopped.

# ---------------------------------------------------------------------------- the refusal
#
# **The item is greyed, and it still reads Resume.** It says what clicking would do, and it will not do it.
pause_line=$(platform_menu_item toggle-pause)
check_contains "the tray's pause item reads Resume" "$pause_line" "'Resume'"
check_contains "and it is greyed" "$pause_line" "insensitive"

# **The Faces tab's glyph is pressed rather than read.** It draws greyed, but AT-SPI reports it `enabled` and
# `sensitive` whatever `accessible-enabled` says (measured 2026-09-25, docs/port-findings.md), so its state
# cannot be asserted from the tree. Pressing it can: the tab ignores a press while the glyph is greyed, so a
# press that got through would write a click row and open a segment.
since=$(mark)
press timing-play-pause
sleep 1.5
check "pressing the Faces tab's greyed glyph does nothing" "0|0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message = 'Button clicked: play pause';")|$(sql "SELECT COUNT(*) FROM device_event WHERE finalised = 0;")"

# **The left click is posted whether or not the item is live**, so this reaches the clock's own refusal rather
# than a courtesy in the tray.
since=$(mark)
activate_status_item
sleep 1.5
expect_log "the tray's left click is refused" "$since" "Resume refused, $NAME is idle or has spent its daily limit"
check "and nothing started" "0" "$(sql "SELECT COUNT(*) FROM device_event WHERE finalised = 0;")"

# ---------------------------------------------------------------------------- raising the limit lifts it
#
# **What comes back is the ability to start the clock, not the clock itself.** Written as a fixture (see the
# header), so this proves the refusal is worked out when it is asked rather than latched when the limit was hit.

sql "UPDATE category SET daily_limit = $((LIMIT_MINUTES * 2)) WHERE category_id = $ID;"
check "the raised limit is stored" "$((LIMIT_MINUTES * 2))" \
    "$(sql "SELECT daily_limit FROM category WHERE category_id = $ID;")"
check "and raising it did not start the clock by itself" "0" \
    "$(sql "SELECT COUNT(*) FROM device_event WHERE finalised = 0;")"

since=$(mark)
activate_status_item
sleep 1.5
check "the clock can be started again" "1" \
    "$(wait_sql "1" "SELECT COUNT(*) FROM device_event WHERE finalised = 0;")"

# Left as it was found: stopped, so nothing after this is timing against a category this script made.
activate_status_item
sleep 1

finish
