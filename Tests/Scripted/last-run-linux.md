# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/swiftParity
    commit:   518b1cbedd93e50678adf9e7946d7f25a18a53c5
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-10-03 11:27:24
    finished: 2026-10-03 12:13:10
    outcome:  passed
    scripts:  35 of 35 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   865 in total
              865 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 37s (1m 12s) |
| 01-launch | 7 | 7 | 0 | 0m 01s |
| 02-menu-bar | 14 | 14 | 0 | 0m 02s |
| 03-settings-window | 39 | 39 | 0 | 0m 31s |
| 04-categories | 98 | 98 | 0 | 2m 32s |
| 05-faces-timing | 33 | 33 | 0 | 0m 33s |
| 06-time-entries | 12 | 12 | 0 | 0m 19s |
| 08-app-settings | 44 | 44 | 0 | 0m 40s |
| 09-report | 25 | 25 | 0 | 0m 33s |
| 10-google-calendar | 10 | 10 | 0 | 0m 19s |
| 11-google-reconnect | 20 | 20 | 0 | 0m 06s (1m 04s) |
| 12-daily-limit | 31 | 31 | 0 | 1m 21s |
| 13-device-tab | 38 | 38 | 0 | 0m 29s |
| 14-time-zone | 9 | 9 | 0 | 0m 16s |
| 50-device-scan | 24 | 24 | 0 | 0m 45s |
| 51-device-connect | 43 | 43 | 0 | 0m 15s |
| 52-device-reset | 31 | 31 | 0 | 0m 30s |
| 53-device-reconnect | 34 | 34 | 0 | 0m 54s |
| 54-device-battery | 13 | 13 | 0 | 0m 38s |
| 55-device-face | 36 | 36 | 0 | 0m 47s (0m 46s) |
| 56-manual-mode | 43 | 43 | 0 | 1m 25s (4m 55s) |
| 57-cube-pause | 40 | 40 | 0 | 1m 17s |
| 58-wrong-pin | 32 | 32 | 0 | 1m 07s |
| 59-double-tap | 5 | 5 | 0 | 0m 33s |
| 60-device-backlog | 33 | 33 | 0 | 0m 36s (3m 52s) |
| 61-lock-without-pause | 10 | 10 | 0 | 0m 08s |
| 62-forced-pause | 16 | 16 | 0 | 0m 07s (3m 28s) |
| 63-led-settings | 24 | 24 | 0 | 0m 09s |
| 64-face-colours | 16 | 16 | 0 | 0m 45s |
| 65-auto-pause | 22 | 22 | 0 | 1m 07s (6m 05s) |
| 66-device-rename | 22 | 22 | 0 | 0m 51s |
| 67-pause-on-lock | 7 | 7 | 0 | 0m 04s |
| 68-device-link-lost | 16 | 16 | 0 | 0m 21s (2m 35s) |
| 69-history-timer | 10 | 10 | 0 | 0m 50s |
| 99-quit | 7 | 7 | 0 | 0m 19s |
| **total** | **865** | **865** | **0** | **21m 47s (23m 57s)** |

A bracketed figure is time the script spent waiting for a person, already taken out of the time beside it.

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
