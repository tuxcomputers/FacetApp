#!/bin/bash
# Puts the app and the database into the state every other script starts from.
#
#   Tests/Scripted/00-setup.sh --capture   read the private seeds out of the database as it stands
#   Tests/Scripted/00-setup.sh             put everything into the known state
#
# `run.sh` calls `--capture` **before** it rebuilds and runs this normally afterwards.
#
# **This is not a test, and it reports one check.** Everything below is arrangement: nothing here asserts what
# the app does, and a green line from this script means only that the ground the other scripts stand on was
# actually laid. So it declares one expected check and answers it once at the bottom -- completed, or did not.
#
# **Carried over for the no-cube range only, 2026-09-25.** The Swift version also seeded fractional history for
# `09-report`, proved the Google account by making a calendar, and factory reset the cube. Those halves come back
# with the script that needs them, which is `09`, `10` and `50` respectively. What is here is what every check
# below `50` stands on: the trace, and the connected Google account `03` and `11` need.
#
# What this guarantees to everything below:
#
#   1. **`debug` logging is on**, in the trace directory `lib.sh` reads.
#   2. **The app is not running**, so the next script gets a cold start with that setting read at launch.
#   3. **A Google account is connected, with its sign-in in the secret store.** When either half is missing the
#      app is opened on the App tab and a sign-in is asked for, since only a person can give one. It is captured
#      and reseeded from then on, so this is asked once per machine rather than once per run.
#   4. **Whether a TimeFlip may be used has been asked**, once, and the answer written for `device_required`. A no is
#      not a setup failure: it stops the run at `50-device-scan`, after every script that needs no cube.
#   5. **The cube is resting on Break and has been reset**, when one may be used: paired so its face can be read,
#      unlocked if the pairing finds it locked, asked to be put on Break only when it is not there, then factory reset
#      and forgotten, so 50 starts from a factory cube.
#
# **This writes straight to the tables**, which every other script in this folder is forbidden from doing.
# It is right here for the same reason it is wrong there: the app is not running while the row goes in, so
# there is nothing to disagree with, and the app reads it when it starts. That is **made** true below rather
# than assumed -- the app is shut before the write.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
source "$(dirname "${BASH_SOURCE[0]}")/seed-private.sh"

require_test_database

if [ "${1:-}" = "--capture" ]; then
    capture_private_seeds
    exit 0
fi

# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=1
start "putting the app and the database into a known state"

# **Every script gets its row now, before any of them runs.** Each one is written with the count it declares and
# nothing against it, so a run that stops half way still lists the scripts it never reached instead of leaving
# them out.
testlog_prepare_scripts

# ---------------------------------------------------------------------------- the database

# **Shut first, and this is the line the rest of the script stands on.** `debug` is read when the app launches,
# so a copy left running from an earlier session would go on with the setting it started under.
quit_app

# **The whole suite stands on this one.** Every check is press by name, then poll for the `debug_log` row, and
# nothing is recorded unless this row says so. `011_setting.sql` seeds it **off**, which is right for somebody
# installing the app and wrong for a run that is about to read the trace.
#
# **Written with the folder alongside it**, because a row holding only `enabled` would leave the trace to fall back
# to the seeded folder while the rest of this suite addresses `$DEBUG_DB`. The two must name the same place.
sql "UPDATE setting SET setting_value = '{\"enabled\":true,\"directory\":\"$SUPPORT_TILDE\"}' WHERE setting_name = 'debug';"
if [ "$(setting debug enabled)" != "1" ]; then
    trouble "debug logging would not stay on, so nothing below can poll for a row"
else
    step "debug logging is on, and the trace is in $SUPPORT"
fi

# **The Google account captured before the rebuild, written back.** The refresh token lives in the secret store,
# which no rebuild touches, but the identity and the calendar the app reads are a row, and a fresh database has
# none. Written while the app is shut, like the row above. `seed-private.sh` says why the values live outside
# the repository.
apply_private_seeds

email=$(setting google_account email)
if [ -z "$email" ]; then
    google_trouble="No Google account is connected, and this run needs one."
elif [ "$(token_stored)" != "yes" ]; then
    google_trouble="The settings hold a Google account ($email) but its sign-in is not in the secret store."
else
    google_trouble=""
fi
if [ -n "$google_trouble" ]; then
    ensure_app_running
    open_settings
    select_tab App
    if action_required \
        "$google_trouble" \
        "03-settings-window and 11-google-reconnect both fail without a working one, so the" \
        "run cannot clear CI as it stands." \
        "" \
        "The Settings window is open on the App tab. Press Sign in with Google (Disconnect" \
        "first if it offers that), consent in the browser, and come back here." \
        "" \
        "Answer n to carry on without one and let 03 and 11 fail."; then
        email=$(setting google_account email)
    fi
    close_settings
    quit_app
    if [ -z "$email" ] || [ "$(token_stored)" != "yes" ]; then
        trouble "no working Google account is connected, so 03 and 11 have nothing to work with"
    else
        step "signed in to Google as $email"
    fi
fi

# **Paired, checked for its face, wiped, and forgotten, in that order**, as the Swift setup did. Reading the face needs a
# link, and a link needs a pairing, so the cube is paired here and given up again by a factory reset, which hands the
# rest of the run a factory cube: `51-device-connect` pairs it for real. `52-device-reset` checks every step of a reset;
# this only needs one to happen. Everything that goes wrong is `trouble`, answered for once at the bottom.
setup_the_cube() {
    if ! require_bluetooth; then
        trouble "Bluetooth is off, so the cube cannot be set up"
        return 1
    fi
    ensure_app_running
    open_settings
    select_tab Device
    local since verdict
    since=$(mark)
    pair_a_cube
    case $? in
        0) ;;
        *) trouble "the cube could not be paired to set it up: $PAIR_REASON"; close_settings; quit_app; return 1 ;;
    esac

    # **A locked cube is unlocked before anything else is asked of it.** The quit at the end of a run leaves the cube
    # paused and locked, and a locked cube refuses to be turned, so the face read and the turn asked for below could not
    # happen. The pairing's status read says which it is. Unlocking leaves the pause as it was.
    if ! unlock_the_cube_if_locked "$since"; then
        trouble "the cube is locked and would not unlock, so the device scripts would start from a locked cube"
    fi

    # **The face the cube is resting on, read on this link**, and asked about only when it is not Break: a face with no
    # category has the app pause the cube as soon as it counts there, so every script from 50 would inherit it stopped.
    # The login reads the face, so a cube already on Break satisfies this before anything is shown.
    if ! ask_and_detect \
        "SELECT message FROM debug_log WHERE debug_log_id = (SELECT MAX(debug_log_id) FROM debug_log WHERE tag = 'face' AND message LIKE 'Face % is up') AND debug_log_id > $since AND message = 'Face 8 is up';" \
        "Put the cube down on the Break face, and leave it there" \
        "That is face 8, the one lit red. Every device script starts from wherever the cube is now," \
        "and a face with no category stops the cube by itself."
    then
        trouble "the cube was never put on the Break face, so the device scripts would start from an unknown one"
    else
        step "the cube is resting on Break, which is where the device range starts"
    fi

    since=$(mark)
    if ! confirm_the_reset; then
        trouble "Reset Device asked nothing, so the cube was not reset"
    fi
    step "resetting the cube, which takes up to two minutes..."
    verdict=$(wait_for "$since" "Reset: %" 140)
    case "$verdict" in
        "Reset: confirmed") step "the cube is back on the factory PIN and forgotten, which is where 50 starts" ;;
        *) trouble "the cube was not reset (${verdict:-no verdict in 140s}), so 51 would pair a cube on this app's PIN" ;;
    esac
    close_settings
    quit_app

    # **The pairing filed the cube's history, and the run starts from none.** The login fetched it, leaving the segment
    # the cube was on open in device_event, which every script below would inherit as something being timed. The app
    # is shut, so the rows are cleared straight from the table, as the rebuild left it.
    sql "DELETE FROM time_entry; DELETE FROM device_event;"
    if [ "$(sql "SELECT (SELECT COUNT(*) FROM device_event) + (SELECT COUNT(*) FROM time_entry);")" != "0" ]; then
        trouble "the history the setup pairing filed would not clear, so the scripts below would inherit it"
    fi
}

if ask_about_the_device; then
    step "a TimeFlip is available for the device scripts"
    setup_the_cube
else
    step "no TimeFlip for this run, so it stops at 50-device-scan"
fi

# ---------------------------------------------------------------------------- the one verdict

if [ -n "$TROUBLE" ]; then
    fail "the setup did not complete, so nothing below is starting from the state it expects:$TROUBLE"
else
    pass "the app and the database are in the state the rest of the run expects"
fi

finish
