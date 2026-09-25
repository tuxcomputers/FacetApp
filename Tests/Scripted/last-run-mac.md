# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/faceTab
    commit:   48712dcfd8db889c45198870233805a0b601e534
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-25 19:15:15
    finished: 2026-09-25 19:17:21
    outcome:  passed
    scripts:  4 of 4 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   71 in total
              71 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 00s |
| 05-faces-timing | 29 | 29 | 0 | 0m 29s |
| 06-time-entries | 12 | 12 | 0 | 0m 18s |
| 12-daily-limit | 29 | 29 | 0 | 1m 18s |
| **total** | **71** | **71** | **0** | **2m 05s** |

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
