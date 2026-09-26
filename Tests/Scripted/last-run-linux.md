# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/reportTab
    commit:   88b21e07a45dc0dc71a284c56e1195e12f3b2ea2
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-26 14:16:19
    finished: 2026-09-26 14:24:20
    outcome:  passed
    scripts:  6 of 6 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   194 in total
              194 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 00s |
| 04-categories | 98 | 98 | 0 | 3m 59s |
| 05-faces-timing | 29 | 29 | 0 | 0m 45s |
| 06-time-entries | 12 | 12 | 0 | 0m 22s |
| 09-report | 25 | 25 | 0 | 1m 21s |
| 12-daily-limit | 29 | 29 | 0 | 1m 31s |
| **total** | **194** | **194** | **0** | **7m 58s** |

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
