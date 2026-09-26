#!/bin/bash
# The Report tab: picking a range, what it totals, folding a category open, and the two sort columns.
#
# It runs after 06 because it needs entries to report on, and 06 is what makes them.
#
# **Converted from the Swift suite 2026-09-26**, against the Report tab as `feature/reportTab` builds it. What
# changed under it, and what that did to the checks:
#
# - **It opens on the app day**, the day the current `daily_reset_time` window started on, where the Swift app
#   opened on the calendar date. That is a check of its own now, off the totals line selecting the tab writes.
# - **Weeks start on Sunday** whatever the locale. The headings are drawn text with no identifier, so what is
#   checked is that the first day cell in the From calendar is a Sunday.
# - **The reset carries a minute**, `{"hour":3,"minute":0}`, so the day is worked out from both.
# - **A label is `value=` on this dump**, where the macOS one printed `desc=` (see `scripts/at-dump.py`).
# - **Every category totalled must be drawn**, the count on screen against the count the totals line gives.
#   The totals sit below both calendars in a scroll view, and a row the pane clips is absent from the tree
#   altogether (04 measured it), so a short window would otherwise read as a report with less in it.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

require_test_database
ensure_app_running
# What this script checks when everything passes. See `finish` in lib.sh for what a mismatch means.
EXPECTED_CHECKS=25
start "the report range, its totals, and sorting them"

RESET_HOUR=$(setting daily_reset_time hour)
RESET_HOUR=${RESET_HOUR:-3}
RESET_MINUTE=$(setting daily_reset_time minute)
RESET_MINUTE=${RESET_MINUTE:-0}
RESET_SECONDS=$((RESET_HOUR * 3600 + RESET_MINUTE * 60))
RESET_AT=$(printf '%02d:%02d' "$RESET_HOUR" "$RESET_MINUTE")

# **The app day now**: before the reset it is still yesterday. Worked out the way the app works it out, from the
# local clock against the reset, rather than from anything the app said.
TODAY=$(sql "SELECT CASE WHEN time('now', 'localtime') < '$RESET_AT:00'
                         THEN date('now', 'localtime', '-1 day') ELSE date('now', 'localtime') END;")

open_settings
since=$(mark)
select_tab Report
expect_log "it opens on the app day ($TODAY, the day starting at $RESET_AT)" "$since" "Report totals $TODAY $RESET_AT%"

# Sunday first: the first cell the From calendar draws is the Sunday on or before the first of the month.
first_cell=$(tree | grep -oE "id=report-from-[0-9]{4}-[0-9]{2}-[0-9]{2}" | head -1 | sed 's/id=report-from-//')
check "the calendar starts its weeks on a Sunday ($first_cell)" "0" "$(sql "SELECT strftime('%w', '${first_cell:-none}');")"

# **The day the newest entry belongs to, not today's date.** The app's day starts at `daily_reset_time` (3 AM by
# default), so a run at half past midnight is reporting on the *previous* calendar day. Worked out the same way
# the app works it out, from the reset and the newest entry's own start.
DAY=$(sql "SELECT date(de.start_epoch - $RESET_SECONDS, 'unixepoch', 'localtime')
             FROM time_entry te JOIN device_event de ON de.device_event_id = te.device_event_id
            ORDER BY de.start_epoch DESC LIMIT 1;")
if [ -z "$DAY" ]; then
    fail "there are no time entries at all, so there is nothing to report on -- run 06 first"
    finish
    exit 1
fi
step "reporting on $DAY (the app's day starts at $RESET_AT)"

# ---------------------------------------------------------------------------- the range

since=$(mark)
press "report-from-$DAY"
sleep 1.5
expect_log "picking a day sets the range" "$since" "Report range $DAY%"

# **The end starts unset**, which is the common case said in one click: pick a day and the report covers that
# day. The app says so rather than silently picking an end.
range=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Report range%' ORDER BY debug_log_id DESC LIMIT 1;")
check_contains "one day, with no end picked" "$range" "not set, reporting one day"

expect_log "and it totals what is in that day" "$since" "Report totals $DAY%"

# ---------------------------------------------------------------------------- what it totalled

totalled=$(dsql "SELECT message FROM debug_log WHERE debug_log_id > $since AND message LIKE 'Report totals $DAY%' ORDER BY debug_log_id DESC LIMIT 1;" \
    | sed -E 's/.*: ([0-9]+) categories$/\1/')
drawn=$(tree | grep -c "id=report-total-[0-9]*-heading" || true)
if [ "${drawn:-0}" -gt 0 ]; then
    pass "$drawn category total(s) drawn"
else
    fail "no totals drawn for $DAY (the app totalled ${totalled:-none}), so nothing below can be checked"
    finish
    exit 1
fi
check "every category totalled is drawn" "${totalled:-none}" "$drawn"

# Only categories that recorded something appear. A page of 0:00 rows would bury the answer, which is why the
# read filters rather than the list hiding them afterwards.
zeroes=$(tree | grep -cE "heading  value=.*, 0:00(:00)?$" || true)
check "no category is listed with nothing in it" "0" "$zeroes"

# ---------------------------------------------------------------------------- folding one open

first_id=$(tree | grep -o "id=report-total-[0-9]*-toggle" | head -1 | sed -E 's/.*report-total-([0-9]+)-toggle/\1/')
if [ -n "$first_id" ]; then
    since=$(mark)
    press "report-total-$first_id-heading"
    sleep 1.5
    expect_log "a category folds open on its heading" "$since" "Report category %opened"

    # The entries are read when the group is opened, not alongside the totals: a closed group's entries are an
    # answer nobody asked for.
    entries=$(tree | grep -c "report-entry-" || true)
    if [ "${entries:-0}" -gt 0 ]; then
        pass "its entries are listed underneath ($entries)"
    else
        fail "the group opened and drew no entries"
    fi

    since=$(mark)
    press "report-total-$first_id-heading"
    sleep 1.5
    expect_log "and folds shut again" "$since" "Report category %closed"
else
    fail "could not find a category group to open"
fi

# ---------------------------------------------------------------------------- sorting
#
# Two questions, and the headings are how you say which one is being asked. Checked by reading the order of the
# rows off the tree, since that is the thing somebody actually sees.

order_now() { tree | grep -o "id=report-total-[0-9]*-heading" | sed -E 's/.*report-total-([0-9]+)-heading/\1/' | tr '\n' ' '; }

# The figures as drawn, in the order they are drawn, in seconds. Read off the app's own headings rather than
# recomputed from the table: what is being checked is that the list on screen is in order, and re-deriving the
# numbers here would be checking the sort against a second implementation of the sum. `display_seconds` is on
# in a clean database, so a heading ends H:MM:SS.
durations_in_order() {
    # The label is `value=` in the AT-SPI dump and `title=` in the macOS one.
    tree | grep -oE "id=report-total-[0-9]+-heading  (value|title)=.*" \
        | sed -E 's/.*, ([0-9]+):([0-9]{2}):([0-9]{2})$/\1 \2 \3/' \
        | awk '{ print $1 * 3600 + $2 * 60 + $3 }' \
        | tr '\n' ' '
}

# Whether a list of numbers only ever goes one way. This is the property a sort actually has -- and the one to
# check, because the two directions are **not** exact reverses of each other: categories tied on a figure fall
# back to the category order, ascending, both times. So comparing the whole sequence to its reverse fails on
# data with any ties in it.
monotonic() {
    printf '%s' "$1" | awk -v dir="$2" '
        { for (i = 1; i <= NF; i++) v[n++] = $i }
        END {
            for (i = 1; i < n; i++) {
                if (dir == "down" && v[i] > v[i - 1]) exit 1
                if (dir == "up"   && v[i] < v[i - 1]) exit 1
            }
            exit 0
        }'
}

# The ids in `$1` in the order the app lists categories by name: names that are whole numbers first, by value,
# then the rest case-insensitively with runs of digits compared as numbers, ties by id. The rule is
# `facet_core::category::display_order`; the names are read from the table.
display_order() {
    local ids
    ids=$(printf '%s' "$1" | tr ' ' ',' | sed 's/,$//')
    sql "SELECT category_id || char(9) || category_name FROM category WHERE category_id IN ($ids);" | python3 -c '
import re, sys
rows = [line.rstrip("\n").split("\t", 1) for line in sys.stdin if line.strip()]
def key(row):
    ident, name = int(row[0]), row[1]
    if re.fullmatch(r"-?\d+", name):
        return (0, int(name), (), ident)
    parts = tuple((0, int(p), "") if p.isdigit() else (1, 0, p.lower()) for p in re.findall(r"\d+|\D+", name))
    return (1, 0, parts, ident)
print(" ".join(row[0] for row in sorted(rows, key=key)) + " ")
'
}

# Reverses a space-separated list. awk rather than `tail -r`, which is BSD's and absent on Linux, or `tac`,
# which is GNU's and absent on the Mac.
reversed() { printf '%s' "$1" | tr ' ' '\n' | grep -v '^$' | awk '{ l[NR] = $0 } END { for (i = NR; i > 0; i--) print l[i] }' | tr '\n' ' '; }

# **It opens on the biggest figure**, which is the question a report is opened to ask.
check_contains "it opens sorted by time, descending" "$(element report-sort-time)" "Time ▼"
check "with the other column bare" "0" "$(element report-sort-category | grep -c '▲\|▼' || true)"

by_time_desc=$(order_now)

falling=$(durations_in_order)
if [ -z "$(printf '%s' "$falling" | tr -d ' ')" ]; then
    fail "no figures could be read off the headings"
elif monotonic "$falling" down; then
    pass "and the figures only ever fall ($falling)"
else
    fail "the figures are not in descending order ($falling)"
fi

since=$(mark)
press report-sort-time
sleep 1.5
expect_log "clicking Time turns the opening order over" "$since" "Report sorted by time, ascending"
check_contains "and the arrow turns over" "$(element report-sort-time)" "Time ▲"

rising=$(durations_in_order)
if [ -z "$(printf '%s' "$rising" | tr -d ' ')" ]; then
    fail "no figures could be read off the headings"
elif monotonic "$rising" up; then
    pass "and now they only ever rise ($rising)"
else
    fail "the figures are not in ascending order ($rising)"
fi

check "the smallest is now where the largest was" "$(printf '%s' "$falling" | awk '{print $NF}')" "$(printf '%s' "$rising" | awk '{print $1}')"

since=$(mark)
press report-sort-category
sleep 1.5
expect_log "clicking Category asks the other question" "$since" "Report sorted by category, ascending"
check_contains "and the arrow moves to that column" "$(element report-sort-category)" "Category ▲"
check "leaving the time column bare" "0" "$(element report-sort-time | grep -c '▲\|▼' || true)"

# **Checked against the names, not against the order before.** Sorting by time can happen to give the order
# sorting by name does, as it did on the Mac on 2026-09-26 (a 10 second category first, then two tied at 7 that
# fall back to name order), and a check that the order changed then fails a correct sort.
by_category=$(order_now)
check "the rows are in category order" "$(display_order "$by_category")" "$by_category"

since=$(mark)
press report-sort-category
sleep 1.5
expect_log "clicking it again reverses that too" "$since" "Report sorted by category, descending"
check "which is the category order upside down" "$(reversed "$by_category")" "$(order_now)"

# Left on the order it opens with, so a later run starts where this one did.
since=$(mark)
press report-sort-time
sleep 1.5
check "back on the order the tab opens with" "$by_time_desc" "$(order_now)"

finish
