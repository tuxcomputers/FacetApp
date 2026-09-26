# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/reportTab
    commit:   ff2c1c73f55bba3df361114127d04a8e3d12ce50
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-26 14:24:05
    finished: 2026-09-26 14:25:59
    outcome:  failed
    scripts:  2 of 6 run, 1 with failures
    short:    5 ran fewer checks than they declare
    checks:   92 in total
              91 passed
              1 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 00s |
| 04-categories | 98 | 90 | 1 | 1m 53s |
| 05-faces-timing | 29 | 0 | 0 | - |
| 06-time-entries | 12 | 0 | 0 | - |
| 09-report | 25 | 0 | 0 | - |
| 12-daily-limit | 29 | 0 | 0 | - |
| **total** | **194** | **91** | **1** | **1m 53s** |

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
