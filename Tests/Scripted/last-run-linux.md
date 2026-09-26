# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/reportTab
    commit:   95a6fb11b61d873cb28f3389665d739766c35951
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-26 15:16:28
    finished: 2026-09-26 15:25:33
    outcome:  passed
    scripts:  6 of 6 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   194 in total
              194 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 01s |
| 04-categories | 98 | 98 | 0 | 4m 44s |
| 05-faces-timing | 29 | 29 | 0 | 0m 51s |
| 06-time-entries | 12 | 12 | 0 | 0m 24s |
| 09-report | 25 | 25 | 0 | 1m 29s |
| 12-daily-limit | 29 | 29 | 0 | 1m 35s |
| **total** | **194** | **194** | **0** | **9m 04s** |

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
