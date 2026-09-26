# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/catergoryTab
    commit:   06f9f11b3a7d9bf38a4195ea643d5dba53d66923
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-25 23:51:38
    finished: 2026-09-25 23:57:52
    outcome:  passed
    scripts:  5 of 5 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   169 in total
              169 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 00s |
| 04-categories | 98 | 98 | 0 | 3m 40s |
| 05-faces-timing | 29 | 29 | 0 | 0m 43s |
| 06-time-entries | 12 | 12 | 0 | 0m 21s |
| 12-daily-limit | 29 | 29 | 0 | 1m 28s |
| **total** | **169** | **169** | **0** | **6m 12s** |

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
