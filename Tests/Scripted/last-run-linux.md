# Scripted suite: last run

Written by `Tests/Scripted/run.sh` at the end of every run, and committed.
**Do not edit it by hand.** CI reads it to decide whether this branch's checks were actually
run, and a stamp that does not describe a real run is worse than no stamp at all.

    branch:   feature/catergoryTab
    commit:   2dd1013264fb4cf1f5dfb89f9daa82795826d5f4
    tree:     dirty
    database: rebuilt from the DDL
    started:  2026-09-25 23:45:13
    finished: 2026-09-25 23:51:10
    outcome:  passed
    scripts:  5 of 5 run, 0 with failures
    short:    0 ran fewer checks than they declare
    checks:   169 in total
              169 passed
              0 failed

| script | expected | passed | failed | time |
|---|---|---|---|---|
| 00-setup | 1 | 1 | 0 | 0m 01s |
| 04-categories | 98 | 98 | 0 | 3m 26s |
| 05-faces-timing | 29 | 29 | 0 | 0m 41s |
| 06-time-entries | 12 | 12 | 0 | 0m 21s |
| 12-daily-limit | 29 | 29 | 0 | 1m 26s |
| **total** | **169** | **169** | **0** | **5m 55s** |

> The working tree had uncommitted changes when this ran, so it is not evidence about the
> commit it names. CI refuses a stamp in this state.

The full record, including the app's own log rows and the accessibility tree at each failure,
is in `logs/testlog.sqlite` on the machine that ran it. That file is not in the repository.
