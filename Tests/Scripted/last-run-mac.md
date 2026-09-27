# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/appTab
    commit:   60a0fc1c632c663e74204cc23222f1e3e2e720d6
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-27 20:08:20
    finished: 2026-09-27 20:13:30
    outcome:  passed
    scripts:  9 of 9 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   293 in total
              293 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 01s |
| 03-settings-window | 35 | 35 | 0 | 0m 23s |
| 04-categories | 98 | 98 | 0 | 1m 39s |
| 05-faces-timing | 29 | 29 | 0 | 0m 27s |
| 06-time-entries | 12 | 12 | 0 | 0m 18s |
| 08-app-settings | 44 | 44 | 0 | 0m 31s |
| 09-report | 25 | 25 | 0 | 0m 16s |
| 11-google-reconnect | 20 | 20 | 0 | 0m 05s (0m 11s) |
| 12-daily-limit | 29 | 29 | 0 | 1m 18s |
| **total** | **293** | **293** | **0** | **4m 58s (0m 11s)** |

A bracketed figure is time the script spent waiting for a person, already taken out of the time beside it.

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
