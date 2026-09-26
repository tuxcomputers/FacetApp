#!/bin/bash
# The App tab: every row written to the table, and put back again.
#
# **Each setting is changed and then changed back**, and both directions are checked. That is not tidying
# up: a one-way change would leave the next script running against a different `blip_time` or a different
# fetch interval, and the second half proves the control works in both directions rather than only
# happening to move the way the first press pushed it.
#
# What is being checked is the write reaching the table. The window writes straight through and reads
# back before it believes anything (`app_settings::write`), so a row that did not change means the control
# is lying about what it did.
#
# **Converted from the Swift suite 2026-09-26**, against the App tab as `feature/appTab` builds it. What
# changed under it, and what that did to the checks:
#
# - **The steppers are Slint SpinBoxes**, with no `-up`/`-down` arrows to press. Each is stepped by writing
#   the value one past what it shows and then writing it back, which is what an arrow commits.
# - **A section is folded on its `-section-heading`**, the whole heading being the control, and is found by its
#   `-section-panel`. The Swift ids were `-section-heading-button` and `-section`.
# - **An empty `debug.directory` means the folder the databases are in**, and the row shows that folder rather
#   than nothing. So the row is compared with the table where the table names one, and with that folder where
#   it does not.
# - **A refused write now raises a notice** ("That setting was not saved"). Nothing a script can do provokes one
#   short of breaking the table, so it is not checked here.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=44
start "every row on the App tab, written and read back"

open_settings
select_tab App

# The number a stepper shows, off its line in the tree.
shown() { element "$1" | sed -nE 's/.*value=([0-9]+).*/\1/p'; }

# ---------------------------------------------------------------------------- the switch
#
# Pressed twice, so whichever way it started it ends where it began.

for pair in "app-show-seconds display_seconds enabled"; do
    set -- $pair
    control="$1" row="$2" field="$3"

    was=$(setting "$row" "$field")
    press "$control"
    sleep 1
    now=$(setting "$row" "$field")
    if [ "$now" != "$was" ]; then
        pass "$control changes $row ($was -> $now)"
    else
        fail "$control left $row at '$was'"
    fi

    press "$control"
    sleep 1
    check "$control puts $row back" "$was" "$(setting "$row" "$field")"
done

# **And the two rows that left are gone rather than drawn on both tabs.** A row on the App tab as well as on the
# Device tab would be two controls answering one question, which is exactly the fault the first rule in
# `CLAUDE.md` is about.
check "the lock switch is not on this tab" "0" "$(on_tab app-pause-on-lock)"
check "nor the battery warning, which went with it" "0" "$(on_tab app-battery-warning)"

# ---------------------------------------------------------------------------- the numbers
#
# One step up and one back down, written as the value an arrow would commit.

for trio in \
    "app-daily-reset daily_reset_time hour" \
    "app-blip-time blip_time seconds"
do
    set -- $trio
    control="$1" row="$2" field="$3"

    was=$(setting "$row" "$field")
    face=$(shown "$control")
    set_field "$control" "$((face + 1))"
    sleep 1
    up=$(setting "$row" "$field")
    if [ "$up" != "$was" ]; then
        pass "$control steps $row up ($was -> $up)"
    else
        fail "$control left $row at '$was' when stepped from $face"
    fi

    set_field "$control" "$face"
    sleep 1
    check "$control steps $row back down" "$was" "$(setting "$row" "$field")"
done

# ---------------------------------------------------------------------------- the odd one out
#
# **The interval is stored in seconds and stepped in whole minutes**, so it does not simply go back. The
# seeded value is 10 seconds, deliberately below the one-minute floor the control offers, which it shows as 1.
# One step up is therefore 2 minutes, and one back down is 1 minute, not the 10 seconds it started at -- the
# control cannot express the value it was showing.

was=$(setting fetch_history_interval_seconds seconds)
face=$(shown app-fetch-interval)
set_field app-fetch-interval "$((face + 1))"
sleep 1
up=$(setting fetch_history_interval_seconds seconds)
set_field app-fetch-interval "$face"
sleep 1
down=$(setting fetch_history_interval_seconds seconds)

if [ -n "$up" ] && [ -n "$down" ] && [ "$((up - down))" = "60" ]; then
    pass "app-fetch-interval steps whole minutes (${was}s -> ${up}s -> ${down}s)"
else
    fail "app-fetch-interval did not step a minute (${was}s -> ${up}s -> ${down}s)"
fi

# Put back by hand, since the control cannot reach a sub-minute value. Written straight to the table, which is
# the one thing in this folder that goes behind the app's back -- and it is safe here because the app reads
# this setting at the point of use rather than holding it.
if [ "$was" != "$down" ]; then
    sql "UPDATE setting SET setting_value = json_set(setting_value, '\$.seconds', $was) WHERE setting_name = 'fetch_history_interval_seconds';"
    check "the developer's ${was}s interval is restored" "$was" "$(setting fetch_history_interval_seconds seconds)"
fi

# ---------------------------------------------------------------------------- what the row means
#
# The App tab shows a 12-hour face; the table stores 24-hour. A stepper that wrote what the field showed would
# put a 12 in the row for midnight, which is the kind of fault nothing notices until a day rolls over at the
# wrong time.
hour=$(setting daily_reset_time hour)
if [ "${hour:-99}" -ge 0 ] && [ "${hour:-99}" -le 23 ]; then
    pass "the stored reset hour is on a 24-hour clock ($hour)"
else
    fail "daily_reset_time holds '$hour', which is not an hour of the day"
fi

# The minute is not on this tab at all, and a write of the hour must not drop it: one row holds the whole
# object, so a write that only knew about the hour would take the minute with it.
minute=$(setting daily_reset_time minute)
if [ -n "$minute" ]; then
    pass "and the minute beside it survived the writes ($minute)"
else
    fail "daily_reset_time has lost its minute, so a write replaced the object instead of one field"
fi

# ---------------------------------------------------------------------------- the order it reads in
#
# **Tree order is what a screen reader reads.** The Swift app once added the Google section to the view first,
# so VoiceOver announced it before the settings drawn above it, and nothing looked wrong on screen.
first=$(tree | grep "section-heading" | head -1)
check_contains "the tab reads in the order it is drawn: App settings first" "$first" "app-settings-section-heading"

# ---------------------------------------------------------------------------- the sections fold
#
# **Both start open**: these two sections are the whole of the tab, so opening it folded would show two
# headings and nothing to change. Pressed on the heading, the whole of which is the control.

check_contains "the App settings section is on the tab" "$(tree)" "id=app-settings-section-panel"
check_contains "and the Google section is too" "$(tree)" "id=app-google-section-panel"

# Read off the contents rather than off the heading: a section with its rows showing is what open means.
check "the App settings section starts open" "1" "$(on_tab app-show-seconds)"
check "the Google section starts open" "1" "$(on_tab app-google-status)"

for pair in "app-settings-section app-show-seconds" "app-google-section app-google-status"; do
    set -- $pair
    section="$1" inside="$2"

    since=$(mark)
    press "$section-heading"
    sleep 1
    expect_log "pressing the $section heading folds it" "$since" "App section $section folded"
    check "and its contents go with it" "0" "$(on_tab "$inside")"

    since=$(mark)
    press "$section-heading"
    sleep 1
    expect_log "pressing it again opens it" "$since" "App section $section opened"
    check "and its contents come back" "1" "$(on_tab "$inside")"
done

# **The footnote goes with the section it explains.** It sits outside the panel, so it hangs off the fold's state.
# Whether there is one at all depends on the account and the build, so what it says is read off the tab first:
# whatever the state puts there, folding takes away and opening puts back.
note_before=$(on_tab app-google-note)

press app-google-section-heading
sleep 1
check "folding Google takes its note with it, whether or not there is one" "0" "$(on_tab app-google-note)"

press app-google-section-heading
sleep 1
check "and opening it puts back exactly what the account state says" "$note_before" "$(on_tab app-google-note)"

# ---------------------------------------------------------------------------- the Debug section
#
# **The one section on this tab that starts folded.** The two above it are what somebody opens the tab to
# change; this one carries the trace, which is of no interest until something needs looking into.

check_contains "the Debug section is on the tab" "$(tree)" "id=app-debug-section-panel"
check "it starts folded, where the other two start open" "0" "$(on_tab app-debug-enabled)"

since=$(mark)
press app-debug-section-heading
sleep 1
expect_log "pressing the Debug heading opens it" "$since" "App section app-debug-section opened"
check "and its rows come with it" "1" "$(on_tab app-debug-enabled)"

# **This is the one check in the suite that can blind the suite**, so it is trapped before it is made. Every
# check after this one polls `debug_log`, and pressing this switch stops the app writing rows at that moment.
#
# **The way back is the control, not the table.** A row written by sqlite under a running app tells the app
# nothing (recording is switched by the App tab, and otherwise read at launch), so the trap presses the switch,
# and only falls back to writing the row and quitting the app -- which makes the next launch read it -- when the
# press did not take.
restore_debug_logging() {
    [ "$(setting debug enabled)" = "1" ] && return 0
    press app-debug-enabled
    sleep 1
    [ "$(setting debug enabled)" = "1" ] && { step "debug logging has been switched back on"; return 0; }
    sql "UPDATE setting SET setting_value = json_set(setting_value, '\$.enabled', json('true')) \
        WHERE setting_name = 'debug';"
    quit_app
    if [ "$(setting debug enabled)" = "1" ]; then
        step "debug logging was written back on; the app was quit so the next launch reads it"
        return 0
    fi
    yellow "##############################################################################"
    yellow "##"
    yellow "##  DEBUG LOGGING IS STILL OFF"
    yellow "##"
    yellow "##  Every script after this one polls debug_log and will find nothing, which"
    yellow "##  reads as the app being broken. Turn it back on before running any more:"
    yellow "##    Settings > App > Debug > Debug logging"
    yellow "##"
    yellow "##############################################################################"
    echo ""
}
trap restore_debug_logging EXIT INT TERM

was=$(setting debug enabled)
since=$(mark)
press app-debug-enabled
sleep 1
now=$(setting debug enabled)
if [ "$now" != "$was" ] && [ -n "$now" ]; then
    pass "the switch writes debug.enabled ($was -> $now)"
else
    fail "debug.enabled still reads '$now' after the switch was pressed"
fi
# **Written while it was still recording**: the row goes down, and only then is the trace told to stop. A trace
# that stopped before saying why would be missing the one line explaining its own end. The trailing `%` is
# needed: `wait_for` adds no wildcards of its own.
expect_log "and the write is recorded" "$since" "App setting debug.enabled ->%"
expect_log "and the trace says it is stopping" "$since" "Logging turned off"

# **The switch is live, and this is what says so.** With logging off, a gesture that always writes a row writes
# none: the folds above proved `App section % folded` arrives within a second.
silent=$(mark)
press app-debug-section-heading
sleep 2
press app-debug-section-heading
sleep 2
check "with logging off, nothing is recorded at all" "0" \
    "$(dsql "SELECT count(*) FROM debug_log WHERE debug_log_id > $silent;")"

since=$(mark)
press app-debug-enabled
sleep 1
check "and pressing it again puts it back" "$was" "$(setting debug enabled)"
# **The first row of the resumed trace.** The write that turned it back on was not recorded, the trace being off
# when it was made, so this line is what says the trace resumes here.
expect_log "and the trace starts again at that moment" "$since" "Logging turned on"

# **Present, and deliberately not pressed.** Reveal opens the file manager and Save a copy puts up the portal's
# save dialog, and both take the keyboard away from the app every remaining step addresses. Clear empties the
# very trace every later check polls.
check "the Trace file row offers Reveal" "1" "$(on_tab app-debug-reveal)"
check "and a copy of the trace to send in" "1" "$(on_tab app-debug-copy)"
check "and a way to empty it" "1" "$(on_tab app-debug-clear)"

# **The folder is read off the row and compared with the table**, rather than against a path written here. An
# empty row in the table means the folder the databases are kept in, written with a leading `~`.
folder=$(element app-debug-directory | sed -n 's/.*value=\(.*\)$/\1/p')
expected=$(setting debug directory)
[ -z "$expected" ] && expected="$SUPPORT_TILDE"
check "the Directory row names the folder the table means" "$expected" "$folder"

# **Left folded, as it was found**, which is what the window would put it back to on the next open anyway.
press app-debug-section-heading
sleep 1
check "the Debug section is folded again" "0" "$(on_tab app-debug-enabled)"

check "the App settings section is open again" "1" "$(on_tab app-show-seconds)"
check "and the Google section is too" "1" "$(on_tab app-google-status)"

finish
