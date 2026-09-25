# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/faceTab
    commit:   a5f92ebf77c9e9ad28de63c2cd132cbe9faf501a
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-25 18:55:38
    finished: 2026-09-25 18:55:51
    outcome:  failed
    scripts:  1 of 4 run, 0 with failures
    short:    3 ran fewer checks than they declare
    checks:   1 in total
              1 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 00s |
| 05-faces-timing | 29 | 0 | 0 | - |
| 06-time-entries | 12 | 0 | 0 | - |
| 12-daily-limit | 29 | 0 | 0 | - |
| **total** | **71** | **1** | **0** | **0m 00s** |

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
