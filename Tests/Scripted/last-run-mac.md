# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/swiftParity
    commit:   f0525f201a996837409ed917cbf80e63251d8489
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-10-03 12:21:53
    finished: 2026-10-03 13:08:54
    outcome:  passed
    scripts:  35 of 35 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   833 in total
              833 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 31s (5m 36s) |
| 01-launch | 7 | 7 | 0 | 0m 01s |
| 02-menu-bar | 14 | 14 | 0 | 0m 03s |
| 03-settings-window | 39 | 39 | 0 | 0m 27s |
| 04-categories | 98 | 98 | 0 | 1m 39s |
| 05-faces-timing | 33 | 33 | 0 | 0m 28s |
| 06-time-entries | 12 | 12 | 0 | 0m 18s |
| 08-app-settings | 44 | 44 | 0 | 0m 31s |
| 09-report | 25 | 25 | 0 | 0m 16s |
| 10-google-calendar | 10 | 10 | 0 | 0m 17s |
| 11-google-reconnect | 20 | 20 | 0 | 0m 05s (1m 32s) |
| 12-daily-limit | 31 | 31 | 0 | 1m 19s |
| 13-device-tab | 38 | 38 | 0 | 0m 21s |
| 14-time-zone | 9 | 9 | 0 | 0m 14s |
| 50-device-scan | 24 | 24 | 0 | 0m 40s |
| 51-device-connect | 43 | 43 | 0 | 0m 13s |
| 52-device-reset | 31 | 31 | 0 | 0m 30s |
| 53-device-reconnect | 34 | 34 | 0 | 0m 37s |
| 54-device-battery | 13 | 13 | 0 | 0m 44s |
| 55-device-face | 36 | 36 | 0 | 0m 51s (13m 32s) |
| 56-manual-mode | 43 | 43 | 0 | 2m 17s (2m 34s) |
| 57-cube-pause | 40 | 40 | 0 | 1m 32s |
| 58-wrong-pin | 0 | 0 | 0 | 0m 00s |
| 59-double-tap | 5 | 5 | 0 | 0m 39s |
| 60-device-backlog | 33 | 33 | 0 | 0m 37s (0m 42s) |
| 61-lock-without-pause | 10 | 10 | 0 | 0m 08s |
| 62-forced-pause | 16 | 16 | 0 | 0m 12s (0m 20s) |
| 63-led-settings | 24 | 24 | 0 | 0m 08s |
| 64-face-colours | 16 | 16 | 0 | 0m 49s |
| 65-auto-pause | 22 | 22 | 0 | 1m 13s (2m 16s) |
| 66-device-rename | 22 | 22 | 0 | 0m 53s |
| 67-pause-on-lock | 7 | 7 | 0 | 0m 04s |
| 68-device-link-lost | 16 | 16 | 0 | 0m 22s (0m 16s) |
| 69-history-timer | 10 | 10 | 0 | 0m 50s |
| 99-quit | 7 | 7 | 0 | 0m 21s |
| **total** | **833** | **833** | **0** | **20m 10s (26m 48s)** |

A bracketed figure is time the script spent waiting for a person, already taken out of the time beside it.

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
