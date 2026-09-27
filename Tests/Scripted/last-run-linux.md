# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/appTab
    commit:   9fd6b20dc66acc47968e9613eed788dcad1dcf37
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-27 20:00:43
    finished: 2026-09-27 20:07:46
    outcome:  passed
    scripts:  9 of 9 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   293 in total
              293 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 00s |
| 03-settings-window | 35 | 35 | 0 | 0m 26s |
| 04-categories | 98 | 98 | 0 | 2m 26s |
| 05-faces-timing | 29 | 29 | 0 | 0m 32s |
| 06-time-entries | 12 | 12 | 0 | 0m 19s |
| 08-app-settings | 44 | 44 | 0 | 0m 39s |
| 09-report | 25 | 25 | 0 | 0m 32s |
| 11-google-reconnect | 20 | 20 | 0 | 0m 05s (0m 39s) |
| 12-daily-limit | 29 | 29 | 0 | 1m 21s |
| **total** | **293** | **293** | **0** | **6m 20s (0m 39s)** |

A bracketed figure is time the script spent waiting for a person, already taken out of the time beside it.

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
