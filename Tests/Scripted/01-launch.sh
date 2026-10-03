#!/bin/bash
# The app starts, records what it is doing, and refuses to run twice.
#
# First after setup because everything else depends on it. A failure here means every later script would be testing
# against an app that is not really up.
#
# **Converted from the Swift suite 2026-09-29**, against `feature/swiftParity`:
#
# - **No `Launch mode:` row.** The Rust app does not record its mode as a row, so the launch is judged by the row it
#   writes once it is in the menu bar or the tray, and that row is also what a duplicate must never write.
# - **No time zone check.** The Rust trace does not resolve a zone per row; every row carries the seeded Unknown.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=7
start "the app launches, records what it does, and runs one copy at a time"

# **A cold start, always.** An app left running by an earlier run has to go first, or `ensure_app_running` finds it
# up, launches nothing, and every check below waits for rows no launch wrote.
if is_running; then
    step "quitting the app left running, so this starts cold"
    quit_app
    sleep 1
fi

since=$(mark)
ensure_app_running
check "the app is running" "yes" "$(is_running && echo yes || echo no)"
expect_log "the launch reaches the status item" "$since" "Facet is in the % Right click the icon for the menu" 20

# The debug log is how every other script checks anything, so its own writing gets a check of its own: a silent log
# would make every later script pass by finding nothing to object to.
rows=$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since;")
if [ "${rows:-0}" -gt 0 ]; then
    pass "the debug log is being written ($rows rows so far)"
else
    fail "the debug log has no rows from this launch, so nothing below could be checked"
fi

# ---------------------------------------------------------------------------- one copy at a time
#
# **The binary is run directly**, so its exit status and stderr can both be read. Nothing presses anything: the
# process is expected to be gone in milliseconds.

since=$(mark)
refusal="$(mktemp)"
"$BINARY" >"$refusal" 2>&1 &
second=$!

# Waited for rather than slept on, and killed if it outlives the wait, so a lock that stopped working does not leave a
# second app running for the rest of the suite.
waited=0
while kill -0 "$second" 2>/dev/null && [ "$waited" -lt 50 ]; do
    sleep 0.1
    waited=$((waited + 1))
done

if kill -0 "$second" 2>/dev/null; then
    kill "$second"
    fail "the second copy was still running after 5s, so the lock did not hold"
else
    wait "$second"
    status=$?
    # Zero: standing down is the lock working, and a non-zero status would tell whatever launched it the launch broke.
    check "a second copy exits, and exits 0" "0" "$status"
    check_contains "saying why on stderr" "$(cat "$refusal")" "already running"
fi
rm -f "$refusal"

# The lock is claimed before either database is opened, so a duplicate writes no row at all.
check "and it wrote nothing to the trace" "0" \
    "$(dsql "SELECT COUNT(*) FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Facet is in the %';")"
check "the original is still the only one running" "1" "$(platform_app_instances)"

finish
