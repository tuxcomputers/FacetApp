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
# `09-report`, proved the Google account by making a calendar, and factory reset the cube. None of those halves
# exists in the Rust app yet, so none of them is here; each comes back with the script that needs it, which is
# `09`, `10` and `50` respectively. What is left is the one thing every check below `50` stands on: the trace.
#
# What this guarantees to everything below:
#
#   1. **`debug` logging is on**, in the trace directory `lib.sh` reads.
#   2. **The app is not running**, so the next script gets a cold start with that setting read at launch.
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

# ---------------------------------------------------------------------------- the one verdict

if [ -n "$TROUBLE" ]; then
    fail "the setup did not complete, so nothing below is starting from the state it expects:$TROUBLE"
else
    pass "the app and the database are in the state the rest of the run expects"
fi

finish
