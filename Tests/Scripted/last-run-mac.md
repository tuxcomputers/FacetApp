# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/timeTracking
    commit:   0b32ed45b8cab25c42fe2be0ba04ab8a4a9c9833
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-29 20:02:48
    finished: 2026-09-29 20:26:48
    outcome:  passed
    scripts:  27 of 27 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   658 in total
              658 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 47s (0m 03s) |
| 03-settings-window | 35 | 35 | 0 | 0m 22s |
| 04-categories | 98 | 98 | 0 | 1m 41s |
| 05-faces-timing | 29 | 29 | 0 | 0m 27s |
| 06-time-entries | 12 | 12 | 0 | 0m 18s |
| 08-app-settings | 44 | 44 | 0 | 0m 31s |
| 09-report | 25 | 25 | 0 | 0m 17s |
| 11-google-reconnect | 20 | 20 | 0 | 0m 04s (2m 36s) |
| 12-daily-limit | 29 | 29 | 0 | 1m 18s |
| 13-device-tab | 38 | 38 | 0 | 0m 21s |
| 50-device-scan | 20 | 20 | 0 | 0m 38s |
| 51-device-connect | 43 | 43 | 0 | 0m 14s |
| 52-device-reset | 31 | 31 | 0 | 0m 27s |
| 53-device-reconnect | 33 | 33 | 0 | 0m 38s |
| 54-device-battery | 13 | 13 | 0 | 0m 47s |
| 55-device-face | 29 | 29 | 0 | 0m 50s (5m 05s) |
| 57-cube-pause | 23 | 23 | 0 | 0m 41s |
| 59-double-tap | 5 | 5 | 0 | 0m 38s |
| 61-lock-without-pause | 10 | 10 | 0 | 0m 08s |
| 62-forced-pause | 16 | 16 | 0 | 0m 06s (0m 50s) |
| 63-led-settings | 24 | 24 | 0 | 0m 06s |
| 64-face-colours | 16 | 16 | 0 | 0m 48s |
| 65-auto-pause | 16 | 16 | 0 | 0m 05s |
| 66-device-rename | 22 | 22 | 0 | 0m 56s |
| 67-pause-on-lock | 7 | 7 | 0 | 0m 04s |
| 68-device-link-lost | 12 | 12 | 0 | 0m 42s (1m 12s) |
| 99-quit | 7 | 7 | 0 | 0m 18s |
| **total** | **658** | **658** | **0** | **14m 12s (9m 46s)** |

A bracketed figure is time the script spent waiting for a person, already taken out of the time beside it.

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
