# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/packaging
    commit:   0978cef0689fc8f4a2ccf980660bf5c2c42c984c
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-10-05 20:48:24
    finished: 2026-10-05 21:37:51
    outcome:  passed
    scripts:  35 of 35 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   865 in total
              865 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 32s (0m 45s) |
| 01-launch | 7 | 7 | 0 | 0m 01s |
| 02-menu-bar | 14 | 14 | 0 | 0m 02s |
| 03-settings-window | 39 | 39 | 0 | 0m 36s |
| 04-categories | 98 | 98 | 0 | 2m 57s |
| 05-faces-timing | 33 | 33 | 0 | 0m 37s |
| 06-time-entries | 12 | 12 | 0 | 0m 20s |
| 08-app-settings | 44 | 44 | 0 | 0m 47s |
| 09-report | 25 | 25 | 0 | 0m 37s |
| 10-google-calendar | 10 | 10 | 0 | 0m 20s |
| 11-google-reconnect | 20 | 20 | 0 | 0m 07s (2m 35s) |
| 12-daily-limit | 31 | 31 | 0 | 1m 24s |
| 13-device-tab | 38 | 38 | 0 | 0m 35s |
| 14-time-zone | 9 | 9 | 0 | 0m 17s |
| 50-device-scan | 24 | 24 | 0 | 0m 47s |
| 51-device-connect | 43 | 43 | 0 | 0m 19s |
| 52-device-reset | 31 | 31 | 0 | 0m 35s |
| 53-device-reconnect | 34 | 34 | 0 | 0m 55s |
| 54-device-battery | 13 | 13 | 0 | 0m 39s |
| 55-device-face | 36 | 36 | 0 | 0m 48s (0m 17s) |
| 56-manual-mode | 43 | 43 | 0 | 1m 27s (1m 58s) |
| 57-cube-pause | 40 | 40 | 0 | 1m 17s |
| 58-wrong-pin | 32 | 32 | 0 | 1m 08s |
| 59-double-tap | 5 | 5 | 0 | 0m 33s |
| 60-device-backlog | 33 | 33 | 0 | 0m 17s (8m 23s) |
| 61-lock-without-pause | 10 | 10 | 0 | 0m 09s |
| 62-forced-pause | 16 | 16 | 0 | 0m 08s (2m 32s) |
| 63-led-settings | 24 | 24 | 0 | 0m 09s |
| 64-face-colours | 16 | 16 | 0 | 0m 45s |
| 65-auto-pause | 22 | 22 | 0 | 1m 09s (4m 12s) |
| 66-device-rename | 22 | 22 | 0 | 0m 54s |
| 67-pause-on-lock | 7 | 7 | 0 | 0m 04s |
| 68-device-link-lost | 16 | 16 | 0 | 0m 20s (5m 55s) |
| 69-history-timer | 10 | 10 | 0 | 0m 50s |
| 99-quit | 7 | 7 | 0 | 0m 21s |
| **total** | **865** | **865** | **0** | **22m 46s (26m 37s)** |

A bracketed figure is time the script spent waiting for a person, already taken out of the time beside it.

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
