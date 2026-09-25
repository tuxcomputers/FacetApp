# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/faceTab
    commit:   311908bf7eafc9ab8335d03b179800d84d7d606b
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-25 18:57:35
    finished: 2026-09-25 18:57:47
    outcome:  failed
    scripts:  2 of 4 run, 1 with failures
    short:    3 ran fewer checks than they declare
    checks:   11 in total
              10 passed
              1 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 01s |
| 05-faces-timing | 29 | 9 | 1 | 0m 10s |
| 06-time-entries | 12 | 0 | 0 | - |
| 12-daily-limit | 29 | 0 | 0 | - |
| **total** | **71** | **10** | **1** | **0m 11s** |

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
