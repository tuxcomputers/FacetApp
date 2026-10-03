#!/bin/bash
# Manual mode with a cube still paired: a paired launch that cannot find its cube asks what to do, in a notice titled
# "The TimeFlip was not found" with Rescan, Time by Hand and Quit. Before a choice the Faces tab and the menu refuse to
# start a clock of the app's own; Rescan looks again and asks again; Time by Hand makes the app its own clock for the
# rest of the launch and stops it reaching for the cube, even once the radio is back.
#
# **Asks for hands twice**: Bluetooth off, then on. Nothing on this machine turns a radio off on somebody's behalf.
# `watch_bluetooth` asks for it back on any way out of the script, a failed check included.
#
# **Only a launch asks.** A link that drops while the app holds it is looked for again quietly, 2 seconds and doubling
# up to 30 between looks, so the notice needs a quit and a launch with the radio already off.
#
# **Starts and ends with the cube resting on Break, running, paired, unlocked and connected.** The relaunch at the end
# reaches the cube again with `relink_a_cube`.
#
# **Converted from the Swift suite 2026-09-29**, against `feature/swiftParity`. Differences from the Swift one:
#
# - **The question is an in-window notice** on the Device tab, read with `notice_text` and `alert_buttons` and
#   answered with `press_title`. The notice writes `Notice shown: TITLE, offering CHOICES` when it goes up and
#   `Notice answered: CHOICE` when a button is pressed.
# - **The drop mid-launch is checked for asking nothing** before the relaunch, the retry being the Rust wording.
# - **The Device tab's settings going dead with the link are `68-device-link-lost`'s**, and are not repeated here.
# - **There is no Forget at the end.** The pairing is kept throughout, and a relaunch with the radio back on is the way
#   back to the cube.
# - **The menu bar is read from its trace rows**, `Menu bar: name <colour>, figure <colour>` and `Menu bar reads NAME`.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=43
start "a paired app that cannot find its cube: what it refuses, and what Time by Hand changes"

require_a_paired_cube "there is no cube to lose"
BREAK=$(sql "SELECT category_id FROM category WHERE category_name = 'Break';")

# `menu_bar_colours [want] [timeout]`: the newest menu bar colour row, waiting up to `timeout` seconds (default 15) for
# it to be `want`. Prints the row last seen either way.
menu_bar_colours() {
    wait_sql "${1:-}" "SELECT message FROM debug_log WHERE message LIKE 'Menu bar: name %' ORDER BY debug_log_id DESC LIMIT 1;" "${2:-15}"
}

# `menu_bar_name [want] [timeout]`: the newest `Menu bar reads` row, waiting the same way.
menu_bar_name() {
    wait_sql "${1:-}" "SELECT message FROM debug_log WHERE message LIKE 'Menu bar reads %' ORDER BY debug_log_id DESC LIMIT 1;" "${2:-15}"
}

# `open_app_segments`: how many segments are open on the app's own faces, 13 and 14.
open_app_segments() {
    sql "SELECT COUNT(*) FROM device_event WHERE finalised = 0 AND device_face > 12;"
}

close_settings

# ---------------------------------------------------------------------------- the link goes, and nothing is asked

since=$(mark)
watch_bluetooth
BLUETOOTH_IS_OFF=1
announce "turning Bluetooth off drops the link, and the app notices"
if ask_and_detect \
    "SELECT message FROM debug_log WHERE debug_log_id > $since AND message = 'The cube is no longer connected';" \
    "Turn Bluetooth OFF" \
    "On the Mac: the Bluetooth item in Control Centre. On Linux: the Bluetooth applet on the panel." \
    "Leave it off until this script asks for it back. The app notices within five seconds."; then
    verdict_pass
else
    verdict_fail "there was no terminal to ask, so Bluetooth was never turned off"
    finish
    exit 1
fi
expect_log "the app says it will look for the cube again" "$since" "The cube went away; looking for it again in %s"
expect_log "and a look that finds nothing is followed by another, twice as far off" "$since" \
    "The cube went away; looking for it again in 4s" 60
check "with no notice raised, the launch having reached its cube" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Notice shown: %';")"

# ---------------------------------------------------------------------------- the launch that asks

quit_app
sleep 1
launched=$(mark)
ensure_app_running
expect_log "a paired launch goes looking for its cube" "$launched" "Looking for the paired cube" 20
expect_log "and does not find it" "$launched" "The paired cube was not reconnected: %" 90
expect_log "so it opens Settings on the Device tab" "$launched" "Settings opened on Device" 15
expect_log "and puts up the not-found notice" "$launched" \
    "Notice shown: The TimeFlip was not found, offering Rescan, Time by Hand, Quit" 10
check_contains "with a notice saying the TimeFlip was not found" \
    "$(notice_text "The TimeFlip was not found" 10)" "The TimeFlip was not found"
check "offering Rescan, Time by Hand and Quit" "Quit|Rescan|Time by Hand" "$(alert_buttons)"
check "the pairing is kept" "1" "$(setting paired paired)"
check "and nothing is connected" "0" "$(setting connection connected)"

# ---------------------------------------------------------------------------- refused before a choice

select_tab Faces
# AT-SPI does not report a dead row (docs/port-findings.md, Linux fact 4), so on Linux the app's own row says it.
if [ "$PLATFORM" = "linux" ]; then
    check "the category rows are drawn dead" "Category rows are dead" \
        "$(dsql "SELECT message FROM debug_log WHERE message LIKE 'Category rows are %' ORDER BY debug_log_id DESC LIMIT 1;")"
else
    check_contains "the category rows are drawn dead" "$(element "category-row-$BREAK")" "disabled"
fi
since=$(mark)
press "category-row-$BREAK"
sleep 1.5
check "a press on one is not taken" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Button clicked: category_id%';")"
check "and no clock of the app's own is started" "0" "$(open_app_segments)"
check "the menu bar shows the cube's last reading, in yellow" "Menu bar: name yellow, figure yellow" \
    "$(menu_bar_colours "Menu bar: name yellow, figure yellow")"
check "and names the category on the cube's face" "Menu bar reads Break" "$(menu_bar_name "Menu bar reads Break")"
check_contains "the menu's Pause is greyed" "$(platform_menu_item toggle-pause)" "insensitive"
check_contains "and so is Lock" "$(platform_menu_item toggle-cube-lock)" "insensitive"
since=$(mark)
menu_press toggle-pause
sleep 1.5
check "choosing Pause anyway starts no clock and sends the cube nothing" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND (message = 'Button clicked: play pause' OR message LIKE 'Timing: %' OR message LIKE 'Sending 06 %');")"

# ---------------------------------------------------------------------------- Rescan

since=$(mark)
press_title Rescan
expect_log "Rescan is the answer taken" "$since" "Notice answered: Rescan" 5
expect_log "Rescan looks for the cube again" "$since" "Looking for the paired cube" 15
expect_log "and does not find it" "$since" "The paired cube was not reconnected: %" 90
check_contains "so the notice is put up again" \
    "$(notice_text "The TimeFlip was not found" 10)" "The TimeFlip was not found"

# ---------------------------------------------------------------------------- Time by Hand

since=$(mark)
press_title "Time by Hand"
expect_log "Time by Hand is the answer taken" "$since" "Notice answered: Time by Hand" 5
expect_log "Time by Hand gives the cube up for this launch" "$since" \
    "Timing by hand for this launch, the cube not having been found" 10
check "the notice goes" "no" "$(alert_is_open && echo yes || echo no)"
check "the pairing is kept" "1" "$(setting paired paired)"

select_tab Faces
if [ "$PLATFORM" = "linux" ]; then
    expect_log "the category rows come alive" "$since" "Category rows are live" 10
else
    case "$(element "category-row-$BREAK")" in
        "") fail "there is no Break row on the Faces tab" ;;
        *disabled*) fail "the category rows are still dead after Time by Hand" ;;
        *) pass "the category rows come alive" ;;
    esac
fi

# The line now shows the app's own clock: the category on the app face last timed, in cyan, or the app's name in the
# ordinary colour when that face holds none.
app_face=$(sql "SELECT device_face FROM device_event WHERE device_face IN (13, 14) ORDER BY device_event_id DESC LIMIT 1;")
app_category=$(sql "SELECT c.category_name FROM face f JOIN category c ON c.category_id = f.category_id WHERE f.face_id = ${app_face:-13} AND f.category_id != 0;")
if [ -n "$app_category" ]; then
    check "the menu bar leaves yellow for cyan, the clock being the app's ($app_category)" \
        "Menu bar: name cyan, figure cyan" "$(menu_bar_colours "Menu bar: name cyan, figure cyan")"
else
    check "the menu bar leaves yellow for the ordinary colour, the app face holding nothing" \
        "Menu bar: name ordinary, figure ordinary" "$(menu_bar_colours "Menu bar: name ordinary, figure ordinary")"
fi

since=$(mark)
press "category-row-$BREAK"
expect_log "a press on Break now starts the app's own clock" "$since" "Timing: started category_id $BREAK on face %" 10
check "a segment is open on one of the app's own faces" "1" \
    "$(wait_sql 1 "SELECT COUNT(*) FROM device_event WHERE finalised = 0 AND device_face > 12;" 10)"
check "the menu bar is cyan" "Menu bar: name cyan, figure cyan" \
    "$(menu_bar_colours "Menu bar: name cyan, figure cyan")"
check_contains "the Faces tab is the clock, its glyph running" \
    "$(element_eventually timing-play-pause "Running")" "Running, click to pause"
check "and it names no face of the cube" "0" "$(on_tab timing-device-face)"

# ---------------------------------------------------------------------------- the radio back, and the cube left alone

close_settings
if ! hands_required "Turn Bluetooth back ON" \
    "Turn it on the same way it went off, then press Return." \
    "The app should carry on ignoring the cube. Nothing will appear to happen for 40 seconds."; then
    fail "Bluetooth was not turned back on"
    finish
    exit 1
fi
for _ in $(seq 1 30); do
    bluetooth_is_on && { BLUETOOTH_IS_OFF=0; break; }
    sleep 1
done

quiet=$(mark)
step "watching for 40s to see whether the app reaches for the cube..."
sleep 40
check "the app does not go looking for the cube" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $quiet AND message = 'Looking for the paired cube';")"
check "or connect to anything" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $quiet AND message LIKE 'Connecting to %';")"
check "and the question is not put again" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $quiet AND message LIKE 'Notice shown: %';")"
check "so nothing is connected" "0" "$(setting connection connected)"
check "and the menu bar is still cyan" "Menu bar: name cyan, figure cyan" "$(menu_bar_colours "Menu bar: name cyan, figure cyan" 5)"

# ---------------------------------------------------------------------------- back to the cube

since=$(mark)
if relink_a_cube; then
    BLUETOOTH_IS_OFF=0
    pass "a relaunch reaches the cube and frees it"
else
    fail "the relaunch did not reach and free the cube within its time"
fi
expect_log "and the menu bar is green again, the cube followed once more" "$since" \
    "Menu bar: name green, figure green" 30
check "a segment is open on face 8 again" "8" \
    "$(wait_sql 8 "SELECT device_face FROM device_event WHERE finalised = 0 AND device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC LIMIT 1;" 30)"
finish
