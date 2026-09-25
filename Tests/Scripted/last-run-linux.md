# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/catergoryTab
    commit:   486851b2f271fac13c3422255bc3e0df4ad826b2
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-25 23:39:21
    finished: 2026-09-25 23:41:28
    outcome:  failed
    scripts:  2 of 5 run, 1 with failures
    short:    4 ran fewer checks than they declare
    checks:   61 in total
              60 passed
              1 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 01s |
| 04-categories | 98 | 59 | 1 | 2m 05s |
| 05-faces-timing | 29 | 0 | 0 | - |
| 06-time-entries | 12 | 0 | 0 | - |
| 12-daily-limit | 29 | 0 | 0 | - |
| **total** | **169** | **60** | **1** | **2m 06s** |

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
