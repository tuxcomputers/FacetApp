# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/appTab
    commit:   68f69dc6dfe0851888de04bb0dd0e91ef1f64617
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-27 08:39:28
    finished: 2026-09-27 08:49:41
    outcome:  passed
    scripts:  7 of 7 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   238 in total
              238 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 01s |
| 04-categories | 98 | 98 | 0 | 4m 55s |
| 05-faces-timing | 29 | 29 | 0 | 0m 53s |
| 06-time-entries | 12 | 12 | 0 | 0m 24s |
| 08-app-settings | 44 | 44 | 0 | 1m 10s |
| 09-report | 25 | 25 | 0 | 1m 13s |
| 12-daily-limit | 29 | 29 | 0 | 1m 35s |
| **total** | **238** | **238** | **0** | **10m 11s** |

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
