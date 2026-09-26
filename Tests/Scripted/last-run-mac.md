# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/appTab
    commit:   32e391225d28517a982ad081df2dfb201a5cf5f6
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-27 09:29:56
    finished: 2026-09-27 09:34:28
    outcome:  passed
    scripts:  7 of 7 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   238 in total
              238 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 00s |
| 04-categories | 98 | 98 | 0 | 1m 41s |
| 05-faces-timing | 29 | 29 | 0 | 0m 27s |
| 06-time-entries | 12 | 12 | 0 | 0m 18s |
| 08-app-settings | 44 | 44 | 0 | 0m 30s |
| 09-report | 25 | 25 | 0 | 0m 16s |
| 12-daily-limit | 29 | 29 | 0 | 1m 19s |
| **total** | **238** | **238** | **0** | **4m 31s** |

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
