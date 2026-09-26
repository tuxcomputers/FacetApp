# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/catergoryTab
    commit:   bfec72809ea11e3a775784ef340ad80f53f0a93d
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-26 12:34:16
    finished: 2026-09-26 12:38:00
    outcome:  passed
    scripts:  5 of 5 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   169 in total
              169 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 00s |
| 04-categories | 98 | 98 | 0 | 1m 41s |
| 05-faces-timing | 29 | 29 | 0 | 0m 27s |
| 06-time-entries | 12 | 12 | 0 | 0m 18s |
| 12-daily-limit | 29 | 29 | 0 | 1m 17s |
| **total** | **169** | **169** | **0** | **3m 43s** |

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
