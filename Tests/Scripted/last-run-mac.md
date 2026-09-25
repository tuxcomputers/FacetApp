# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/faceTab
    commit:   2f2032c3134f37c60011912424ed38adfa3190ea
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-25 19:00:11
    finished: 2026-09-25 19:00:18
    outcome:  failed
    scripts:  2 of 4 run, 1 with failures
    short:    3 ran fewer checks than they declare
    checks:   2 in total
              1 passed
              1 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 01s |
| 05-faces-timing | 29 | 0 | 1 | 0m 05s |
| 06-time-entries | 12 | 0 | 0 | - |
| 12-daily-limit | 29 | 0 | 0 | - |
| **total** | **71** | **1** | **1** | **0m 06s** |

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
