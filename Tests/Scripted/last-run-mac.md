# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/deviceTab
    commit:   b8d21e9abd1c27c1943fda717a51345919ec21d6
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-28 22:01:13
    finished: 2026-09-28 22:11:00
    outcome:  passed
    scripts:  21 of 21 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   559 in total
              559 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 00s (0m 03s) |
| 03-settings-window | 35 | 35 | 0 | 0m 23s |
| 04-categories | 98 | 98 | 0 | 1m 40s |
| 05-faces-timing | 29 | 29 | 0 | 0m 27s |
| 06-time-entries | 12 | 12 | 0 | 0m 18s |
| 08-app-settings | 44 | 44 | 0 | 0m 31s |
| 09-report | 25 | 25 | 0 | 0m 16s |
| 11-google-reconnect | 20 | 20 | 0 | 0m 04s (0m 22s) |
| 12-daily-limit | 29 | 29 | 0 | 1m 18s |
| 13-device-tab | 38 | 38 | 0 | 0m 21s |
| 50-device-scan | 20 | 20 | 0 | 0m 38s |
| 51-device-connect | 43 | 43 | 0 | 0m 13s |
| 52-device-reset | 31 | 31 | 0 | 0m 29s |
| 53-device-reconnect | 33 | 33 | 0 | 0m 36s |
| 54-device-battery | 13 | 13 | 0 | 0m 12s |
| 63-led-settings | 24 | 24 | 0 | 0m 05s |
| 65-auto-pause | 16 | 16 | 0 | 0m 04s |
| 66-device-rename | 22 | 22 | 0 | 0m 22s |
| 67-pause-on-lock | 7 | 7 | 0 | 0m 03s |
| 68-device-link-lost | 12 | 12 | 0 | 0m 12s (0m 46s) |
| 99-quit | 7 | 7 | 0 | 0m 14s |
| **total** | **559** | **559** | **0** | **8m 26s (1m 11s)** |

A bracketed figure is time the script spent waiting for a person, already taken out of the time beside it.

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
