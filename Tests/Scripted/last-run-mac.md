# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/catergoryTab
    commit:   e4606b7a35d8be827565fd41507b71691217b77e
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-26 12:32:51
    finished: 2026-09-26 12:33:02
    outcome:  failed
    scripts:  2 of 5 run, 1 with failures
    short:    4 ran fewer checks than they declare
    checks:   9 in total
              8 passed
              1 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 00s |
| 04-categories | 98 | 7 | 1 | 0m 09s |
| 05-faces-timing | 29 | 0 | 0 | - |
| 06-time-entries | 12 | 0 | 0 | - |
| 12-daily-limit | 29 | 0 | 0 | - |
| **total** | **169** | **8** | **1** | **0m 09s** |

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
