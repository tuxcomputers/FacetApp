# Scripted checks

Checks that drive the real app and read the real database. `cargo test` is hermetic and never opens a
window or touches a radio, so a feature can be entirely green there and broken the moment it runs.
These are what say it works.

They need no AI, no Claude, and nothing installed beyond what building the app already needs.

---

**Status, 2026-09-25: four checks are back, and they pass on Linux.** `00-setup`, `05-faces-timing`,
`06-time-entries` and `12-daily-limit` were converted from the Swift suite for the Faces tab, and a full
`run.sh` on the Linux box passed 71 of 71 (run 4 in `logs/testlog.sqlite`). They have not been run on the
Mac yet. The rest of the 32 numbered checks are still missing, because a check for a feature the Rust app
does not have yet cannot pass. [`docs/scripted-suite.md`](../../docs/scripted-suite.md) lists all 32 with
what each proved, and each goes back as its feature lands.

**So read this for how the suite works and why.** Where a numbered script named below is one of the
missing ones, or a file is gone rather than merely unwritten, the text says so.

**On Linux a run needs `wmctrl`, and it types real keystrokes.** A Slint text field cannot be written
over AT-SPI, so `at-set.py` focuses it and types into it (see
[`port-findings.md`](../../docs/port-findings.md)). `run.sh` turns `toolkit-accessibility` on for the run,
because a Slint window is not on the bus without it, and puts back whatever it found.

---

```sh
Tests/Scripted/run.sh
```

That **rebuilds `test.sqlite` from the DDL**, builds the app if cargo says anything changed, launches
it, runs every script in order, quits it, and writes everything to `logs/screen.txt` as well as the
terminal.

**There is no bundle any more**, so nothing compares timestamps against one: cargo decides whether a
build is needed and `platform.sh` runs it. The binary is `target/debug/facet-mac` or
`target/debug/facet-linux`, and the DDL is compiled into it rather than sitting beside it, so it works
wherever it is run.

The terminal gets colour; **`logs/screen.txt` is plain text**, written live, so it can be watched with
`tail -f logs/screen.txt` during a run and opened in an editor afterwards without a screenful of escape
sequences.

**Starting from nothing is the default**, because these scripts create categories and time entries and
delete nothing: run after run the database fills up, lists get longer, and a check can start passing
because of a row some earlier run happened to leave. A run from the DDL says what the app does from
nothing, which is the only version of that answer another developer will also get.

```sh
Tests/Scripted/run.sh --keep      # against the database as it stands
```

`--keep` is for looking at what a failed run left.

## The Google account, across a rebuild

A rebuilt database has no `google_account` row, so `10-google-calendar` would skip on every clean run.
The refresh token survives -- it is in the login Keychain, which no rebuild touches -- but the identity
and calendar the app reads are rows, and they do not.

**Connect an account once**, on Settings -> App. From then on `run.sh` captures that row *before* each
rebuild and `00-setup` writes it back afterwards, so the sync is covered on every run without anybody
signing in again.

The captured file is `~/.config/facet/scripted-seed.json`, **outside the repository** and beside the
OAuth client credentials it belongs with. It holds a real email address and a real calendar id, and this
repository is public: a seed committed into it would put one developer's account into everybody's
checkout.

## A new calendar every run

`03-settings-window` **deletes** the calendar the last run made, presses Create, and renames the fresh
one to `Facet-test`. All three go through the app's own controls.

**It happens there, before anything records an entry, so the run's events survive the run.** Recording
an entry sweeps every unsynced row into whatever calendar the app currently holds. Replacing the
calendar later would delete one that several scripts had already filled, and the events you would want
to look at afterwards would go with it. Set up first, the calendar ends the run holding everything the
run produced.

Reusing it is what looks reasonable and does not work. Google keeps a deleted *event* for ever as
`cancelled` and will never reissue its id, while Facet derives an event's id from `time_entry_id` --
which a clean run restarts at 1. Emptying the calendar therefore burned exactly the ids the next run was
about to ask for, and every one of those entries then failed to sync for ever. A new calendar has no
cancelled ids in it, so the collision cannot arise.

**The app does the deleting, which is why there is no Keychain prompt.** This used to be a Python script
reading the refresh token with the `security` tool. That is a different program from the one that owns
the Keychain item, so macOS asked permission -- and because signing in again creates a *fresh* item, the
"Always Allow" was thrown away and it asked again after every run that exercised `11`. The app holds that
token already and never has to ask.

**You will not end up with a pile of `Facet-test` calendars.** Steady state is one: each run deletes the
previous before making its own. If a delete fails the id stays in the row, so the next capture picks it
up and the next run tries again -- which matters because `calendarList.list` returns nothing usable under
the `calendar.app.created` scope, so a calendar that escapes cleanup is invisible from then on and can
only be removed by hand in Google Calendar.

## Before you run

**Point the app at the test database.** The scripts refuse to run otherwise, and say so:

```sh
scripts/switch-database.sh test
```

They write real categories, segments and time entries, and the default run **deletes and rebuilds
test.sqlite**. Production holds time you actually recorded, and nothing here will touch it -- but that
guard is the only thing standing between the two, which is why every script checks before it starts.

**Take your hands off.** These drive the real cursor and the real window on your real screen. A click
you make while a script is running lands in whatever the script just opened.

## Running some of them

```sh
Tests/Scripted/run.sh 04              # scripts whose name contains "04"
Tests/Scripted/run.sh categories      # or a word from the name
Tests/Scripted/run.sh --keep-running  # leave the app up afterwards, to look at a failure
Tests/Scripted/run.sh --keep 09       # one script, against the database as it stands
```

**`00-setup` is a prerequisite and not merely the first script**, and skipping it fails in a way that
points at the app. A rebuilt database has `debug` off -- `crates/facet-core/resources/database/011_setting.sql` seeds
`{"enabled":false,"directory":""}` -- and `00-setup` is what turns it on. So an app launched without it
has no logger at all, by design, and every check polling the trace reports something like

    FAIL  no debug_log row matching 'Launch mode:%' within 20s

which reads as a launch that went wrong rather than as a trace nobody enabled. `Tests/Scripted/run.sh 01`
on its own does exactly this (measured 2026-09-20). Run `00` first, then the subset with `--keep`.

Running a subset still rebuilds the database unless `--keep` is given, and most scripts depend on what
the ones above them made -- `09-report` needs the entries `06` records. `--keep` is usually what you
want when running one on its own.

Each script also runs on its own, and launches the app itself if it is not already up:

```sh
bash Tests/Scripted/04-categories.sh
```

**Getting it to the state it wants is yours to do.** A script arranges nothing it merely needs: it
starts from what the one above it left, which is what the whole suite running in order gives it. So a
device script run on its own stops straight away and says which state is missing, rather than pairing a
cube to make itself work.

## One pairing, for the whole device range

**`51-device-connect` pairs the cube, and every script from `52` to `99` uses that one.** None of them
forgets it and pairs again to be sure of what it is starting from -- the run is a sequence, and a cube
sitting there connected is exactly as good as one paired thirty seconds ago.

`51` ends by restarting the app, which is the other half of what it hands on. Not for the mode -- since
2026-08-29 the app reads `paired` when it is asked, so a pairing on its own is enough to make it follow
the cube -- but for the link: what the range inherits is a cube the app has reached, with a face, a
charge and a status behind it.

A script whose subject *is* giving a cube up puts it back before it finishes: the wipe in `52`, and the
checked forgets in `53`, `54`, `55` and `56`. That is `restore_the_pairing`, and it is not a check --
whether a cube can be paired again is not what any of those scripts is about -- but it does stop the run
if it cannot, because everything after it would otherwise fail at a cube that is not there.

**Three of them still take the link down and let it back up**, which is a different thing from pairing:
`54`, `55` and `57` each assert on what the app does *as a link comes up* -- the charge pulled on
connecting, the face read as the link opens, the clock set after the login -- and a connection that is
already up wrote those rows before the script could mark anything. `relink_a_cube` quits and relaunches,
and the app reconnects to the cube it already has. No scan, no pairing. It is the same call `51` ends
with.

## What a failure looks like

```
  a new category appears in the list
    PASS
  the renamed category keeps its icon
    FAIL  expected 'Coffee', got 'None'
```

**What is being checked prints before the verdict**, on its own line, so a run can be watched as well as
read afterwards. Several checks wait on a network round trip or a ten-second timer, and a single line
printed on the way out would leave the terminal silent for as long as the slowest step takes with nothing
saying which step it is.

Then, at the end of the script, the count and every failure again. **A failing script stops the run**:
each one starts from the state the last one left, so carrying on would report the next failure in the
wrong place.

## The order, and why it is that order

Each script depends on what the ones above it proved, so they read top to bottom as the app coming up
and then being used.

**Below `50` needs no TimeFlip; `50` and above needs one.** The number says what a script requires
before anybody opens it, and that is the whole of the rule: a check that does not touch the device is
written somewhere in `01`-`49`, and a check that does is written at `50` or above. Both ranges have
room, so a new script takes the next free number in its own half and nothing is renumbered to make
space.

The point of the split is that "did the device half run?" is answerable from the file names alone,
by a person or by a script. Before it, the two were interleaved -- `12` needed nothing and `13`
needed a cube -- so the only way to know what a run had actually covered was to read every file.

**Quit is `99`, and everything else comes before it.** It is the one script that ends the app, so
anything after it would run against nothing at all. It needs a cube as well (it wipes it), which the
`50` rule already allows for: `99` is above `50`, and it sorts last where it belongs.

**There is no skip. Every check passes or fails.** A skip is a check reporting that it could not
answer, and a run full of them reads green while proving nothing -- which is exactly what happened on
2026-08-22, when `55-device-face` skipped its whole self and the run still stamped `outcome: passed`.
So the missing cube, the radio that will not come up, the Google account nobody connected and the
prompt nobody answered are all failures now. They say what is needed, and the run is red until it is
there.

What this does *not* cover is a device legitimately having nothing to say -- a cube that never told
this Mac its name, say. That is not a check failing to run, it is the app handling a real case
correctly, so it passes and the line says which case it met.

| | |
|---|---|
| `00-setup` | seeds what a rebuilt database cannot have: the Google account, and history with fractional durations |
| `01-launch` | the database opens, one instance only, the debug log records |
| `02-menu-bar` | the status item, its menu, and the pause on its right half |
| `03-settings-window` | the window opens, the tabs switch, it closes, and the run's calendar is made |
| `04-categories` | create, rename, retire, reinstate, renaming a retired one, and the alerts a namesake raises |
| `05-faces-timing` | a category on a face, the clock starting and pausing |
| `06-time-entries` | a finished segment becoming tracked time, and a blip not |
| `07-history-timer` | it fires while timing and stops when nothing is |
| `08-app-settings` | each row on the App tab written and read back |
| `09-report` | the range, the totals, folding a category open, the sorting |
| `10-google-calendar` | the account, and recorded time reaching the calendar `03` made |
| `11-google-reconnect` | disconnect keeps the calendar, and signing back in still reaches it (**asks you to sign in**) |
| `12-daily-limit` | a category spending its `daily_limit` stops the clock, and every way of starting it again refuses |
| `13-device-tab` | the Device tab's two sections folding, including a fold inside a fold, and every Settings control dead with no cube |
| `50-device-scan` | the radio comes up, the scan lists what answers it, and stops on its own |
| `51-device-connect` | pairing, the PIN the cube is on, and what the Device tab says afterwards |
| `52-device-reset` | the factory reset, and the cube coming back on the vendor PIN |
| `53-device-reconnect` | a paired app reaching its own cube at launch, with the window shut |
| `54-device-battery` | the charge: read on connecting, pushed after that, and shown without flapping |
| `55-device-face` | the face the cube is on, in the menu bar and on the Faces tab (**asks you to turn the cube**) |
| `56-manual-mode` | a paired app that cannot find its cube: what a click refuses, and what taking manual mode stops (**asks you to switch Bluetooth off and on**) |
| `57-cube-pause` | the status item's right half: one click stops and starts the cube, two lock and unlock it (**ends by asking you to turn a paused cube**) |
| `58-wrong-pin` | a cube that refuses this app's PIN: the offer, Retry, and taking manual mode (**asks you to answer a dialog twice**) |
| `59-double-tap` | the four registers: stepped, sent, read back off the cube, then written down, and dead while the gesture is off |
| `60-device-backlog` | a cube out of range: what the app shows, what it refuses to write, and what the cube backfills when it returns (**asks you to switch Bluetooth off and on, and to turn the cube in between**) |
| `61-lock-without-pause` | locking the cube from the status item with `pause_on_lock` off |
| `62-forced-pause` | the app stopping the cube itself: a face with no category, and a category that has spent its `daily_limit` (**asks you to turn the cube four times**) |
| `63-led-settings` | the cube LED: brightness and blink period stepped, sent to the cube, then written down |
| `64-face-colours` | the cube lit in its faces colours: twelve on connecting, and one when a category is recoloured |
| `65-auto-pause` | the cube auto-pause delay: stepped, sent as `0x05`, read back with `0x10`, then written down, and the cube stopping itself on it (**asks you to turn the cube, then to leave it alone for a minute**) |
| `66-device-rename` | the cube renamed from the Device tab: `0x15` to the hardware, the row written only after it, and the cube still found afterwards |
| `99-quit` | the way out closes what was open |

## How a check is written

Three things, and everything in `lib.sh` exists to keep them honest.

**Evidence comes from the database.** A check waits for a `debug_log` row or queries a table. Asking a
person "did that work?" records their optimism, and these scripts are meant to be run by somebody who
was not watching.

**Every wait is baselined.** `mark` takes the newest `debug_log` id *before* the action, and `wait_for`
only looks past it. Without that, a matching row from ten minutes ago passes a step that did nothing --
and this database is used by a person as well as by these scripts, so a total count is never safe to
assert on.

**Nothing is cleaned up afterwards.** The rows a run creates are left where they are. They are evidence,
and a script that tidied up would be deleting the thing somebody wants to look at when it fails.

```bash
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
require_test_database
ensure_app_running
start "what this script checks"

since=$(mark)
press some-button
expect_log "the button did the thing" "$since" "%the thing%"

check "the table agrees" "1" "$(sql 'SELECT ...;')"

finish
```

## CI checks that you ran them

CI cannot run this suite: there is no screen, no Keychain and no Google account on a build machine. What
it does is refuse a pull request that has no record of a run.

**Both machines, every time.** `run.sh` writes a stamp at the end of every run, from the recorded run rather
than from anything it was told: **`Tests/Scripted/last-run-mac.md`** on the Mac and
**`Tests/Scripted/last-run-linux.md`** on the Linux box. Both are committed, and a pull request needs both.
One platform's pass says nothing about the other: both platform divergences the Swift port found lived in
shared code that passed on one and failed on the other (see "Run every test on every platform that can run
it" in `docs/port-findings.md`).

**The gate is `scripts/check-scripted-stamps.sh`**, run by the `Scripted suite run on both machines` job in
`.github/workflows/tests.yml` on every pull request, and required by `All tests pass`. Run it yourself before
pushing:

```sh
scripts/check-scripted-stamps.sh --branch "$(git branch --show-current)"
```

First it checks every numbered script is runnable: it parses, is executable, calls `finish`, guards the
database with `require_test_database` and declares `EXPECTED_CHECKS`. Then, **for each of the two stamps**,
it requires all of:

- the run was on **this** branch;
- it **passed**, with zero failing checks;
- **every numbered script ran, and each passed exactly the checks it declares.** This suite has no skip
  verdict, so a check that could not answer shows here as a script short of its `EXPECTED_CHECKS`. In
  practice a run meant for a pull request needs the cube in reach, an account connected and every prompt
  answered, once there are checks that want them. The failure names each short script;
- the tree was **clean** when it ran, since a run against uncommitted changes is not evidence about the
  commit it names;
- the commit it names is **in this branch's history**, and nothing under `crates/` (the DDL included),
  `Tests/Scripted/`, `Cargo.toml` or `Cargo.lock` has changed since. The stamps and the other Markdown in
  `Tests/Scripted/` are left out of that, so committing one machine's stamp does not make the other's stale.

That last one is why the stamp carries a commit rather than a date. The old checklists recorded a date and
a branch, so a run from before the last five commits looked exactly like one from after them. Editing a
README does not force a re-run; changing the app does. **So a change to the app made on one machine needs a
run on the other as well**, and the handover files are where one machine asks the other for it.

None of it is enforced on a push to main, where the stamps go on naming the feature branch that ran them.

**So: run the suite on both machines, then commit each stamp.** If either machine did not run it, CI will
say so rather than let a green build imply otherwise.

**Commit the stamp before running the suite again**, which is the part that is easy to miss and cost a real
afternoon on 2026-08-22. `run.sh` writes the file at the *end* of a run, so from that moment the tree has an
uncommitted change in it: the stamp itself. Start another run without committing and it dutifully records
`tree: dirty`, and CI then refuses the whole thing as not being evidence about the commit it names -- even
though the run passed and the only uncommitted file was the previous run's own stamp. It does not settle by
itself either. Every subsequent run sees the same uncommitted file and reports dirty again, so the way out
is to commit it, not to run once more.

If you find yourself there with a passing run already recorded, the stamp can be rewritten from any run in
`logs/testlog.sqlite` rather than by hand or by spending another twenty minutes with the cube:

```sh
sqlite3 logs/testlog.sqlite "SELECT run_id, started_at, dirty, outcome FROM run ORDER BY run_id DESC LIMIT 5;"
bash -c 'source Tests/Scripted/testlog.sh; testlog_stamp <run_id>'
```

It is generated from what the database recorded, so it stays a true account of a run that happened -- which
is the whole reason the file says not to edit it by hand. Check the run you pick was `dirty` 0, `outcome`
passed, and unfiltered.

**A contributor without both machines cannot clear this, and is not meant to.** The suite needs a Mac and a
Linux box, and a cube in range once there are checks that want one, so a fork's pull request lands here red
however good the change is -- which is the honest state of it: the change has not been tried on both
platforms. What clears it is somebody who *has* both running the suite against that branch and committing
the two stamps. The two things that make that
possible are leaving "Allow edits by maintainers" ticked and not force-pushing the branch while it is
being run. In Swift that was written down in `CONTRIBUTING.md`, which has not been carried over; it
wants writing again when this repository takes outside contributions.

## When one of these fails

The app is still there if you passed `--keep-running`. Beyond that:

```sh
# The trace is its own file, beside the app's database. platform.sh knows where that is on both
# platforms, so ask it rather than writing the path out and being wrong on one of them.
source Tests/Scripted/platform.sh
sqlite3 "$DEBUG_DB" \
  "SELECT logged_at, tag, message FROM debug_log ORDER BY debug_log_id DESC LIMIT 40;"

python3 scripts/ax-dump.py --app facet-mac   # what the script can see and press
```

[`Tests/Methods.md`](../Methods.md) is the reference for both, and for the traps that have already cost
time: what needs a real mouse event, and why a status item is not in the menu bar's accessibility tree.

**One trap from the Swift suite no longer applies.** Launching the `.app` directly could run a binary
older than the change being tested. There is no `.app`: the binary is what cargo built, and `run.sh`
builds before it launches.
