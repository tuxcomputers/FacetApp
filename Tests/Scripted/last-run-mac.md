# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/packaging
    commit:   dfdf20d94864b2fa49bdae3684facc02b32bd8cb
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-10-05 18:44:07
    finished: 2026-10-05 19:08:03
    outcome:  passed
    scripts:  35 of 35 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   833 in total
              833 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 31s (0m 22s) |
| 01-launch | 7 | 7 | 0 | 0m 02s |
| 02-menu-bar | 14 | 14 | 0 | 0m 03s |
| 03-settings-window | 39 | 39 | 0 | 0m 33s |
| 04-categories | 98 | 98 | 0 | 1m 40s |
| 05-faces-timing | 33 | 33 | 0 | 0m 27s |
| 06-time-entries | 12 | 12 | 0 | 0m 18s |
| 08-app-settings | 44 | 44 | 0 | 0m 31s |
| 09-report | 25 | 25 | 0 | 0m 17s |
| 10-google-calendar | 10 | 10 | 0 | 0m 17s |
| 11-google-reconnect | 20 | 20 | 0 | 0m 06s (0m 17s) |
| 12-daily-limit | 31 | 31 | 0 | 1m 18s |
| 13-device-tab | 38 | 38 | 0 | 0m 22s |
| 14-time-zone | 9 | 9 | 0 | 0m 14s |
| 50-device-scan | 24 | 24 | 0 | 0m 40s |
| 51-device-connect | 43 | 43 | 0 | 0m 13s |
| 52-device-reset | 31 | 31 | 0 | 0m 32s |
| 53-device-reconnect | 34 | 34 | 0 | 0m 38s |
| 54-device-battery | 13 | 13 | 0 | 0m 43s |
| 55-device-face | 36 | 36 | 0 | 0m 49s (0m 11s) |
| 56-manual-mode | 43 | 43 | 0 | 2m 25s (0m 08s) |
| 57-cube-pause | 40 | 40 | 0 | 1m 33s |
| 58-wrong-pin | 0 | 0 | 0 | 0m 00s |
| 59-double-tap | 5 | 5 | 0 | 0m 39s |
| 60-device-backlog | 33 | 33 | 0 | 0m 39s (0m 29s) |
| 61-lock-without-pause | 10 | 10 | 0 | 0m 08s |
| 62-forced-pause | 16 | 16 | 0 | 0m 13s (0m 35s) |
| 63-led-settings | 24 | 24 | 0 | 0m 07s |
| 64-face-colours | 16 | 16 | 0 | 0m 50s |
| 65-auto-pause | 22 | 22 | 0 | 1m 08s (0m 57s) |
| 66-device-rename | 22 | 22 | 0 | 0m 54s |
| 67-pause-on-lock | 7 | 7 | 0 | 0m 04s |
| 68-device-link-lost | 16 | 16 | 0 | 0m 28s (0m 23s) |
| 69-history-timer | 10 | 10 | 0 | 0m 50s |
| 99-quit | 7 | 7 | 0 | 0m 20s |
| **total** | **833** | **833** | **0** | **20m 32s (3m 22s)** |

A bracketed figure is time the script spent waiting for a person, already taken out of the time beside it.

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
