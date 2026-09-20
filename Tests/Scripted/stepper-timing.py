#!/usr/bin/env python3
"""Reads what a held stepper arrow actually did, out of the app's own debug log.

    Tests/Scripted/stepper-timing.py <since_debug_log_id> "<category name>"

Prints one `key=value` per line, for a shell check to read:

    values=1,2,3,4,5,6,7,8,9,10,15,20,25
    steps=1,1,1,1,1,1,1,1,1,5,5,5
    singles_count=9   singles_avg_ms=101
    fives_count=3     fives_avg_ms=304
    ratio=3.0

**The timing is the point, and it is only readable because each tick writes a row.** `SteppedNumberField`
reports every value the hold passes through, not just the one it ends on, and `debug_log.logged_at` keeps
milliseconds -- so the gap between consecutive rows is the tick interval the app actually used. The rule
being checked (`StepperHoldRules`) says 0.1s while stepping by 1 and 0.3s once stepping by 5, so the
by-five ticks should come out about three times slower. A control that changed step size without slowing
down would step by 5 at the speed of 1 and run away from whoever is holding it, which is the fault worth
catching and is invisible to a check that only reads the final number.

`singles_avg_ms` excludes the first gap, which carries the 0.4s wait before a hold starts repeating and
is not a tick interval at all.
"""

import pathlib
import re
import sqlite3
import sys
from datetime import datetime

# The trace's own database, not the app's. `debug_log` moved out of `appdata.sqlite` on 2026-08-22 so the
# file a user sends in carries the log and nothing else -- see docs/database-design.md.
DB = pathlib.Path.home() / "Library" / "Application Support" / "Facet" / "debug.sqlite"


def main():
    if len(sys.argv) < 3:
        sys.exit("usage: stepper-timing.py <since_debug_log_id> <category name>")
    since, name = int(sys.argv[1]), sys.argv[2]

    connection = sqlite3.connect(f"file:{DB}?mode=ro", uri=True)
    rows = connection.execute(
        "SELECT logged_at, message FROM debug_log"
        " WHERE debug_log_id > ? AND message LIKE ? ORDER BY debug_log_id",
        # Unquoted: every debug message is plain text now, so that a SQL LIKE pattern never has to escape
        # its way around one (see the debug message rules in CLAUDE.md).
        (since, f'Category {name} daily limit -> %'),
    ).fetchall()

    ticks = []
    for logged_at, message in rows:
        found = re.search(r"-> (-?\d+)min", message)
        if not found or "REFUSED" in message:
            continue
        # Milliseconds matter here, and the column keeps them.
        ticks.append((datetime.fromisoformat(logged_at), int(found.group(1))))

    if len(ticks) < 2:
        print(f"values={','.join(str(v) for _, v in ticks)}")
        print("steps=")
        print("singles_count=0 singles_avg_ms=0")
        print("fives_count=0 fives_avg_ms=0")
        print("ratio=0")
        return

    values = [value for _, value in ticks]
    gaps, steps = [], []
    for (before, first), (after, second) in zip(ticks, ticks[1:]):
        gaps.append((after - before).total_seconds() * 1000)
        steps.append(abs(second - first))

    # Grouped by the size of the step *arrived at*, which is what its interval belongs to.
    singles = [gap for gap, step in zip(gaps, steps) if step == 1]
    fives = [gap for gap, step in zip(gaps, steps) if step == 5]
    # The first gap holds the hold delay before repeating begins, not a tick interval.
    if singles:
        singles = singles[1:] or singles

    average = lambda series: round(sum(series) / len(series)) if series else 0
    print(f"values={','.join(str(v) for v in values)}")
    print(f"steps={','.join(str(s) for s in steps)}")
    print(f"singles_count={len(singles)} singles_avg_ms={average(singles)}")
    print(f"fives_count={len(fives)} fives_avg_ms={average(fives)}")
    print(f"ratio={round(average(fives) / average(singles), 1) if average(singles) else 0}")


if __name__ == "__main__":
    main()
