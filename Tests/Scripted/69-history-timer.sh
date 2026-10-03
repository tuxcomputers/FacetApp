#!/bin/bash
# The history timer: started as the link comes up, firing on the interval `fetch_history_interval_seconds` holds, each
# firing asking the cube for its history, and each re-arming read the interval from the table again, so a change to
# the setting takes effect at the next arming with no relaunch.
#
# **Starts from the cube paired, connected and running on Break**, and leaves it there. Nothing here turns or pauses
# the cube, so no hands are needed.
#
# **The interval is changed straight in the table** and put back by a trap on any way out. That is safe because the
# app reads this setting at the point of use, each time the timer is armed.
#
# **Converted from the Swift suite 2026-09-29**, against `feature/swiftParity`, and numbered 69 so it runs after `51`
# has paired the cube. The Swift timer ran only while something was being timed and stopped when nothing was; the Rust
# one runs for as long as the link is held, whatever the cube is doing, and stops only when the link goes or the app
# quits. So the Swift checks that it stays silent with nothing timed, stops on a pause and starts again on a resume have
# no counterpart here. What replaces them is the spacing of two firings and the interval being read again at each
# arming.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=10
start "the history timer firing on the interval the table holds"

require_a_paired_cube "there is no history to fetch"

INTERVAL=$(setting fetch_history_interval_seconds seconds)
INTERVAL=${INTERVAL:-10}
if [ "$INTERVAL" -gt 5 ]; then
    CHANGED=5
else
    CHANGED=$((INTERVAL + 5))
fi
step "the interval is ${INTERVAL}s, and ${CHANGED}s is what it is changed to"

# Puts the interval back to what it was when the script started, on any way out.
restore_interval() {
    [ "$(setting fetch_history_interval_seconds seconds)" = "$INTERVAL" ] && return 0
    sql "UPDATE setting SET setting_value = json_set(setting_value, '\$.seconds', $INTERVAL) WHERE setting_name = 'fetch_history_interval_seconds';"
    status=$?
    if [ "$status" -ne 0 ] || [ "$(setting fetch_history_interval_seconds seconds)" != "$INTERVAL" ]; then
        red "  the history interval could not be put back to ${INTERVAL}s (sqlite exit $status); set it by hand"
        return 1
    fi
    step "the history interval is back at ${INTERVAL}s"
}
trap restore_interval EXIT INT TERM

# `set_interval <seconds>`: writes the interval straight into the table.
set_interval() {
    sql "UPDATE setting SET setting_value = json_set(setting_value, '\$.seconds', $1) WHERE setting_name = 'fetch_history_interval_seconds';"
    status=$?
    [ "$status" -ne 0 ] && red "  the interval could not be written (sqlite exit $status)"
}

# `first_firing_after <id> <seconds>`: the id of the first firing row after `id` that names `seconds`, empty when none.
first_firing_after() {
    dsql "SELECT MIN(debug_log_id) FROM debug_log WHERE debug_log_id > $1 AND message = 'History timer fired, asking on a ${2}s interval';"
}

# ---------------------------------------------------------------------------- running on the table's interval

# The start row is written at login, when the link comes up. The latest one is this launch's, and it names the
# interval the table held then, which is the one the table holds now unless something changed it since.
started=$(dsql "SELECT message FROM debug_log WHERE message LIKE 'History timer started, asking every %s' ORDER BY debug_log_id DESC LIMIT 1;")
check_contains "the timer that started with the link names the table's interval" "$started" "every ${INTERVAL}s"

since=$(mark)
expect_log "it fires by itself" "$since" "History timer fired, asking on a ${INTERVAL}s interval" $((INTERVAL + 8))
expect_log "and the firing asks the cube for its history" "$since" "Fetching history (the timer asked)%" 10
expect_log "and that fetch comes back" "$since" "History fetch done (the timer asked):%" 30

# **Twice, and the spacing between them**, so this is a timer that re-arms itself on the interval rather than one
# that fired once, or one firing on some other period.
first=$(first_firing_after "$since" "$INTERVAL")
expect_log "it goes on firing" "${first:-$since}" "History timer fired, asking on a ${INTERVAL}s interval" $((INTERVAL + 8))
second=$(first_firing_after "${first:-$since}" "$INTERVAL")
gap=""
if [ -n "$first" ] && [ -n "$second" ]; then
    gap=$(dsql "SELECT CAST(ROUND((julianday(b.logged_at) - julianday(a.logged_at)) * 86400) AS INTEGER)
                FROM debug_log a, debug_log b WHERE a.debug_log_id = $first AND b.debug_log_id = $second;")
fi
if [ -n "$gap" ] && [ "$gap" -ge $((INTERVAL - 1)) ] && [ "$gap" -le $((INTERVAL + 3)) ]; then
    pass "the two firings are one interval apart (${gap}s on ${INTERVAL}s)"
else
    fail "the two firings were not one interval apart (${gap:-no gap}s on ${INTERVAL}s)"
fi

# ---------------------------------------------------------------------------- the interval read again

# The timer armed before the change fires on the old interval and arms the next from the table, so the firing that
# names the new one comes within one old interval and one new.
since=$(mark)
set_interval "$CHANGED"
check "the table takes the ${CHANGED}s interval" "$CHANGED" "$(setting fetch_history_interval_seconds seconds)"
expect_log "the next arming reads it, with no relaunch" "$since" \
    "History timer fired, asking on a ${CHANGED}s interval" $((INTERVAL + CHANGED + 8))

since=$(mark)
set_interval "$INTERVAL"
check "the table takes the ${INTERVAL}s interval back" "$INTERVAL" "$(setting fetch_history_interval_seconds seconds)"
expect_log "and the arming after that reads it back the same way" "$since" \
    "History timer fired, asking on a ${INTERVAL}s interval" $((CHANGED + INTERVAL + 8))

finish
