# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/appTab
    commit:   a711ce27d0ca463b8425ff393da8085ef3be6fd3
    tree:     dirty
    database: rebuilt from the DDL
    started:  2026-09-27 11:45:10
    finished: 2026-09-27 11:59:14
    outcome:  passed
    scripts:  9 of 9 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   293 in total
              293 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 01s |
| 03-settings-window | 35 | 35 | 0 | 1m 05s |
| 04-categories | 98 | 98 | 0 | 5m 56s |
| 05-faces-timing | 29 | 29 | 0 | 1m 00s |
| 06-time-entries | 12 | 12 | 0 | 0m 25s |
| 08-app-settings | 44 | 44 | 0 | 1m 26s |
| 09-report | 25 | 25 | 0 | 1m 22s |
| 11-google-reconnect | 20 | 20 | 0 | 0m 14s (0m 46s) |
| 12-daily-limit | 29 | 29 | 0 | 1m 39s |
| **total** | **293** | **293** | **0** | **13m 08s (0m 46s)** |

A bracketed figure is time the script spent waiting for a person, already taken out of the time beside it.

> The working tree had uncommitted changes when this ran, so it is not evidence about the
> commit it names. CI refuses a stamp in this state.

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
