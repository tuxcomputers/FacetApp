# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/deviceTab
    commit:   d2e23e948af8e0de84f6c5e1b90e5dae02fcd9a4
    tree:     clean
    database: rebuilt from the DDL
    started:  2026-09-28 21:40:26
    finished: 2026-09-28 21:56:07
    outcome:  passed
    scripts:  21 of 21 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   559 in total
              559 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 01s (1m 45s) |
| 03-settings-window | 35 | 35 | 0 | 0m 36s |
| 04-categories | 98 | 98 | 0 | 3m 12s |
| 05-faces-timing | 29 | 29 | 0 | 0m 38s |
| 06-time-entries | 12 | 12 | 0 | 0m 21s |
| 08-app-settings | 44 | 44 | 0 | 0m 49s |
| 09-report | 25 | 25 | 0 | 0m 47s |
| 11-google-reconnect | 20 | 20 | 0 | 0m 08s (0m 34s) |
| 12-daily-limit | 29 | 29 | 0 | 1m 22s |
| 13-device-tab | 38 | 38 | 0 | 0m 38s |
| 50-device-scan | 20 | 20 | 0 | 0m 44s |
| 51-device-connect | 43 | 43 | 0 | 0m 21s |
| 52-device-reset | 31 | 31 | 0 | 0m 33s |
| 53-device-reconnect | 33 | 33 | 0 | 0m 58s |
| 54-device-battery | 13 | 13 | 0 | 0m 12s |
| 63-led-settings | 24 | 24 | 0 | 0m 09s |
| 65-auto-pause | 16 | 16 | 0 | 0m 05s |
| 66-device-rename | 22 | 22 | 0 | 0m 30s |
| 67-pause-on-lock | 7 | 7 | 0 | 0m 05s |
| 68-device-link-lost | 12 | 12 | 0 | 0m 10s (0m 42s) |
| 99-quit | 7 | 7 | 0 | 0m 19s |
| **total** | **559** | **559** | **0** | **12m 38s (3m 01s)** |

A bracketed figure is time the script spent waiting for a person, already taken out of the time beside it.

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
