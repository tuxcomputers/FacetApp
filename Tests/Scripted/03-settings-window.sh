#!/bin/bash
# The Settings window: its six tabs, moving between them, closing it, and then making the calendar this
# run will fill.
#
# **The calendar is set up here because of when this script runs, not because it belongs to this window.**
# Recording an entry will sweep every unsynced row into whatever calendar the app holds, once calendar sync
# is built, so a calendar replaced later would be replaced after several scripts had already filled the old
# one. This is the first script with the window open and the last before anything is recorded.
#
# **Converted from the Swift suite 2026-09-27**, against the Settings window as `feature/appTab` builds it:
#
# - **Six tabs, not five**: About is a tab of its own, and is checked like the rest.
# - **The window has no Close button.** It is closed through the window manager (`close_settings`), and the
#   app records `Settings closed` when it hides the window.
# - **The delete confirmation is an in-window notice**, read by `alert_buttons` and answered by `press_title`.
# - **The calendar is renamed to the process name**, `facet-mac` or `facet-linux`, so the two machines' test
#   calendars in one Google account can be told apart.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=35
start "the Settings window, its tabs, and the calendar this run will fill"

open_settings
check "the window is open" "yes" "$(settings_is_open && echo yes || echo no)"

# **Off Faces first.** Selecting the tab already on show fires nothing, and the window always opens on Faces,
# so a loop starting there would wait for a row that is never written.
select_tab Device

# **The width the window opens at, before any tab is chosen.** Every tab is drawn into the same window, so
# switching between them must not move it: a tab that arrives wider is demanding room the window then has to
# find, and the person who left the window where they wanted it gets it dragged out from under them.
opening_width=$(window_width settings-window)
if [ -n "$opening_width" ]; then
    pass "the window opens at a width that can be read ($opening_width points)"
else
    fail "the window's width could not be read, so nothing below can tell whether a tab moves it"
fi

# Every tab, in the order they sit in the window, each checked three times: the app said it selected it, the
# pane it names is in the tree, and the window is still the width it was. The first alone would pass on a tab
# that logged the change and drew nothing.
for tab in Faces Categories Report App Device About; do
    since=$(mark)
    select_tab "$tab"
    expect_log "the $tab tab can be selected" "$since" "Settings tab selected: $tab"

    # **Waited for, not read once.** The accessibility tree catches up with a tab change after the change is
    # logged, and the App tab, which checks the Google sign-in as it opens, has taken longer than the pause
    # `select_tab` allows (the Mac, 2026-09-27).
    pane=0
    for _ in $(seq 1 25); do
        pane=$(tree | grep -c "id=settings-pane-$(echo "$tab" | tr '[:upper:]' '[:lower:]')" || true)
        [ "${pane:-0}" -gt 0 ] && break
        sleep 0.2
    done
    if [ "${pane:-0}" -gt 0 ]; then
        pass "the $tab pane is on screen"
    else
        fail "the $tab tab was selected but its pane is not in the tree"
    fi

    check "and the window is the width it was" "$opening_width" "$(window_width settings-window)"
done

# ---------------------------------------------------------------------------- coming back

# Closing and reopening re-reads everything rather than showing what the last open loaded, which is the first
# rule in CLAUDE.md applied to this window. What is checked here is the cheaper half of it: that the window
# goes away and comes back at all.
since=$(mark)
close_settings
expect_log "closing the window is recorded" "$since" "Settings closed"
check "the window is gone" "no" "$(settings_is_open && echo yes || echo no)"

open_settings
check "it opens again" "yes" "$(settings_is_open && echo yes || echo no)"

# **It always opens on Faces**, whatever was showing when it was closed: what is being timed is what somebody
# opening this window most often wants. The row is written by the open itself rather than by the tab callback,
# because selecting the tab already on show fires nothing.
opened=$(dsql "SELECT message FROM debug_log WHERE message LIKE 'Settings opened on %' ORDER BY debug_log_id DESC LIMIT 1;")
check_contains "it opens on Faces however it was left" "$opened" "Faces"

# ============================================================================ the run's calendar
#
# **A fresh calendar every run.** Google keeps a deleted event for ever as `cancelled` and will not reissue
# its id, and a rebuilt database restarts `time_entry_id` at 1, so a reused calendar would collide with its
# predecessor's ids. A calendar made a moment ago has none of them.
#
# **The app does the deleting**, through its own controls, with the refresh token it holds.

select_tab App

email=$(sql "SELECT json_extract(setting_value, '\$.email') FROM setting WHERE setting_name = 'google_account';")
if [ -z "$email" ]; then
    fail "no Google account is connected, so there is no calendar to set up"
    fail "connect one on Settings -> App; it is captured before the next rebuild and put back after"
    finish
    exit $?
fi

stored_name() { sql "SELECT json_extract(setting_value, '\$.calendar_name') FROM setting WHERE setting_name = 'google_account';"; }
stored_id() { sql "SELECT json_extract(setting_value, '\$.calendar_id') FROM setting WHERE setting_name = 'google_account';"; }

# ---- something to delete
#
# **Signing in does not make a calendar, so a run can arrive here with none.** The delete below needs a
# subject for its checks to say anything, so one is made when the last run left none.
#
# **Not a check**: it is arranging the bench, not judging the app, and counting it would make
# `EXPECTED_CHECKS` depend on what the machine happened to be left holding. The create is judged below.

if [ -z "$(stored_id)" ]; then
    step "no calendar is stored, so one is made for the delete to take"
    since=$(mark)
    press app-google-calendar-create
    # **The row is waited for and its answer discarded, because the table decides here.** A create that never
    # happened leaves `stored_id` empty, and the guard below says so and stops.
    wait_for "$since" "Google calendar created,%" 45 >/dev/null
    if [ -z "$(stored_id)" ]; then
        fail "the Create button made no calendar, so there is nothing to delete and nowhere to sync"
        finish
        exit 1
    fi
fi

# ---- last run's, deleted

doomed=$(stored_name)
since=$(mark)
press app-google-calendar-delete
# **Waited for, not slept on**, for the same reason as the panes above: the notice reaches the accessibility
# tree after the press, and on Linux straight after the window has been reopened it has taken longer than a
# second.
for _ in $(seq 1 25); do
    [ -n "$(alert_buttons)" ] && break
    sleep 0.2
done

check "deleting asks first, and offers a way out" "Cancel|Delete Calendar" "$(alert_buttons)"
check_contains "and the question names the calendar" "$(platform_alert_message)" "$doomed"

press_title "Delete Calendar"
expect_log "confirming deletes it at Google" "$since" "Google calendar deleted,%" 45

# **Forgotten only once Google has taken it.** A row cleared before the request would leave the app unable to
# name what it failed to delete.
check "and the app no longer holds a calendar" "|" "$(stored_id)|$(stored_name)"
check_contains "the Calendar row offers to make another" "$(tree)" "id=app-google-calendar-create"

# ---- this run's, made and named

since=$(mark)
press app-google-calendar-create
expect_log "Create makes one" "$since" "Google calendar created,%" 45
if [ -z "$(stored_id)" ]; then
    fail "no calendar id was recorded, so nothing recorded today has anywhere to go"
    finish
    exit 1
fi
check "and it is called Facet, the name the app makes them under" "Facet" "$(stored_name)"

# **A real rename, at Google**: the row is written from what Google answers rather than from what was typed,
# so a name that only changed inside Facet fails here rather than looking like a pass.
WANTED="$PROCESS_NAME"
made=$(stored_id)
since=$(mark)
press app-google-calendar
wait_for_element app-google-calendar-field 5
set_field_focused app-google-calendar-field "$WANTED"
press_return
expect_log "renaming it goes to Google" "$since" "Google calendar renamed to $WANTED" 45
check "the row records the new name" "$WANTED" "$(stored_name)"
check_contains "and the Calendar row shows it" "$(element app-google-calendar)" "$WANTED"

# **The name is a label and the id is the identity.** A rename that moved to a different calendar would leave
# every event already written behind, under a calendar nothing points at any more.
check "the rename did not change which calendar it is" "$made" "$(stored_id)"

finish
