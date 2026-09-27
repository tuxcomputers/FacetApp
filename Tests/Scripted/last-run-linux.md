# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   fix/trayFault
    commit:   61c423a84de558a4a26627f868d78d8791b21b96
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-27 20:59:12
    finished: 2026-09-27 21:05:59
    outcome:  passed
    scripts:  9 of 9 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   293 in total
              293 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 00s |
| 03-settings-window | 35 | 35 | 0 | 0m 35s |
| 04-categories | 98 | 98 | 0 | 2m 27s |
| 05-faces-timing | 29 | 29 | 0 | 0m 32s |
| 06-time-entries | 12 | 12 | 0 | 0m 19s |
| 08-app-settings | 44 | 44 | 0 | 0m 39s |
| 09-report | 25 | 25 | 0 | 0m 32s |
| 11-google-reconnect | 20 | 20 | 0 | 0m 06s (0m 15s) |
| 12-daily-limit | 29 | 29 | 0 | 1m 21s |
| **total** | **293** | **293** | **0** | **6m 31s (0m 15s)** |

A bracketed figure is time the script spent waiting for a person, already taken out of the time beside it.

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
