# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/timeTracking
    commit:   ac90a4dc0a6883135b0ce83bfa6fcd1137fa9b4b
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-29 18:57:13
    finished: 2026-09-29 19:55:03
    outcome:  passed
    scripts:  27 of 27 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   658 in total
              658 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 21s (0m 04s) |
| 03-settings-window | 35 | 35 | 0 | 0m 26s |
| 04-categories | 98 | 98 | 0 | 2m 12s |
| 05-faces-timing | 29 | 29 | 0 | 0m 30s |
| 06-time-entries | 12 | 12 | 0 | 0m 19s |
| 08-app-settings | 44 | 44 | 0 | 0m 36s |
| 09-report | 25 | 25 | 0 | 0m 31s |
| 11-google-reconnect | 20 | 20 | 0 | 0m 04s (37m 58s) |
| 12-daily-limit | 29 | 29 | 0 | 1m 20s |
| 13-device-tab | 38 | 38 | 0 | 0m 25s |
| 50-device-scan | 20 | 20 | 0 | 0m 38s |
| 51-device-connect | 43 | 43 | 0 | 0m 15s |
| 52-device-reset | 31 | 31 | 0 | 0m 28s |
| 53-device-reconnect | 33 | 33 | 0 | 0m 52s |
| 54-device-battery | 13 | 13 | 0 | 0m 38s |
| 55-device-face | 29 | 29 | 0 | 0m 46s (1m 28s) |
| 57-cube-pause | 23 | 23 | 0 | 0m 36s |
| 59-double-tap | 5 | 5 | 0 | 0m 32s |
| 61-lock-without-pause | 10 | 10 | 0 | 0m 07s |
| 62-forced-pause | 16 | 16 | 0 | 0m 06s (0m 52s) |
| 63-led-settings | 24 | 24 | 0 | 0m 06s |
| 64-face-colours | 16 | 16 | 0 | 0m 43s |
| 65-auto-pause | 16 | 16 | 0 | 0m 04s |
| 66-device-rename | 22 | 22 | 0 | 0m 49s |
| 67-pause-on-lock | 7 | 7 | 0 | 0m 04s |
| 68-device-link-lost | 12 | 12 | 0 | 0m 33s (3m 06s) |
| 99-quit | 7 | 7 | 0 | 0m 20s |
| **total** | **658** | **658** | **0** | **14m 21s (43m 28s)** |

A bracketed figure is time the script spent waiting for a person, already taken out of the time beside it.

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
