# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/reportTab
    commit:   3f5e3538481bd74cb2890f9b5f7647f12d8e52b3
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-26 14:19:57
    finished: 2026-09-26 14:22:36
    outcome:  failed
    scripts:  5 of 6 run, 1 with failures
    short:    2 ran fewer checks than they declare
    checks:   162 in total
              161 passed
              1 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 00s |
| 04-categories | 98 | 98 | 0 | 1m 41s |
| 05-faces-timing | 29 | 29 | 0 | 0m 27s |
| 06-time-entries | 12 | 12 | 0 | 0m 18s |
| 09-report | 25 | 21 | 1 | 0m 12s |
| 12-daily-limit | 29 | 0 | 0 | - |
| **total** | **194** | **161** | **1** | **2m 38s** |

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
