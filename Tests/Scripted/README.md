# Scripted checks

Checks that drive the real app and read the real database. `cargo test` is hermetic and never opens a
window or touches a radio, so a feature can be entirely green there and broken the moment it runs.
These are what say it works.

They need no AI and no Claude. They do need the tools the drivers use: on the Mac, pyobjc
([`system-mac.md`](../../docs/system-mac.md)); on Linux, `wmctrl`, `python3-pyatspi`, `python3-dbus`, `python3-gi`,
`secret-tool` and `bluetoothctl` ([`system-linux.md`](../../docs/system-linux.md)).

---

**Status, 2026-10-03: every script is back.** All 35 numbered scripts pass in full on both machines. On that date
the Linux run was 865 checks and the Mac run 833; the stamps are the source of those numbers, each machine's latest
run being in `last-run-mac.md` and `last-run-linux.md`. `58-wrong-pin` is not run on macOS: it rewrites the app's
Keychain item with the `security` tool, and the app's next read of it raises a Keychain prompt that an unattended
run cannot answer. It declares no checks there and the Mac stamp shows it as 0 of 0; it runs in full on Linux.
Every Swift script has a Rust counterpart, and [`docs/scripted-suite.md`](../../docs/scripted-suite.md) lists them
all with what each proves.

**A full run needs somebody there.** `00-setup` asks whether the run may use your TimeFlip and, on a yes, for the
cube to be on Break (a y is waited for and trusted) before it factory resets it; it also asks for a Google sign-in
when the run has no account. `11` asks for a Google sign-in every run (the browser opens on that machine's screen
and the run waits four minutes). `55`, `62` and `65` ask for turns of the cube, `56`, `60` and `68` for Bluetooth
off and on. Everything else runs unattended.

**So read this for how the suite works and why.**

**On Linux a run needs `wmctrl`, and it types real keystrokes.** A Slint text field cannot be written
over AT-SPI, so `at-set.py` focuses it and types into it (see
[`port-findings.md`](../../docs/port-findings.md)). `run.sh` turns `toolkit-accessibility` on for the run,
because a Slint window is not on the bus without it, and puts back whatever it found.

**A disabled Slint element never reaches AT-SPI**, so on Linux a check that asks whether a control is dead reads
the trace row the app writes for it (the Faces tab's `Category rows are live` and `Category rows are dead`), where
on macOS the dump carries `disabled` ([`port-findings.md`](../../docs/port-findings.md), Linux fact 4).

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

A rebuilt database has no `google_account` row, so `03`, `10` and `11` would fail on every clean run. The
refresh token survives -- it is in the platform secret store, which no rebuild touches -- but the identity
and calendar the app reads are rows, and they do not.

**Connect an account once**, on Settings -> App. From then on `run.sh` captures that row *before* each
rebuild and `00-setup` writes it back afterwards, so `03` finds an account on every run. `11` still asks for
a sign-in each time, because signing back in is what it tests.

When a run has no account, or has one whose sign-in is missing from the secret store, `00-setup` opens Settings on
the App tab and asks for the sign-in; answering n carries on and lets those scripts fail.

The captured file is `~/.config/facet/scripted-seed.json`, **outside the repository** and beside the
OAuth client credentials it belongs with. It holds a real email address and a real calendar id, and this
repository is public: a seed committed into it would put one developer's account into everybody's
checkout.

## A new calendar every run

`03-settings-window` **deletes** the calendar the last run made, presses Create, and renames the fresh
one to the app's process name, `facet-mac` or `facet-linux`, so the two machines' test calendars in one Google
account can be told apart. All three go through the app's own controls.

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

**You will not end up with a pile of test calendars.** Steady state is one per machine: each run deletes the
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

## Driving the Linux box over ssh

An ssh shell lacks the desktop session's environment, and the drivers need it:

```sh
export DISPLAY=:0 XAUTHORITY=$HOME/.Xauthority XDG_RUNTIME_DIR=/run/user/1000 \
       DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus
```

The tower's other details are in [`system-linux.md`](../../docs/system-linux.md).

## Running some of them

```sh
Tests/Scripted/run.sh 04              # scripts whose path contains "04"
Tests/Scripted/run.sh categories      # or a word from the name
Tests/Scripted/run.sh --keep-running  # leave the app up afterwards, to look at a failure
Tests/Scripted/run.sh --keep 09       # one script, against the database as it stands
Tests/Scripted/run.sh --keep-going    # a failed check no longer stops its script
```

**There is one filter, and the last argument that is not a flag wins.** It is a substring of the script's path, so
`5` runs `05` and every script from `50` to `59`. **A filtered run still writes the machine's stamp**, with a
`filter:` line, and the CI gate refuses it, so restore the committed stamp from git or finish with a full run
before pushing.

**The device range cannot be run from a filter alone.** `50-device-scan` reads the answer `00-setup` records in
`logs/device-gate`, which `run.sh` deletes at the start of every run, so it passes only in a full run. `51` pairs
the cube, and `52` and above need one already paired and connected and stop at once without one.

**`00-setup` is a prerequisite and not merely the first script**, and skipping it fails in a way that
points at the app. A rebuilt database has `debug` off -- `crates/facet-core/resources/database/011_setting.sql` seeds
`{"enabled":false,"directory":""}` -- and `00-setup` is what turns it on. So an app launched without it
has no logger at all, by design, and every check polling the trace reports something like

    FAIL  no debug_log row matching 'Facet is in the % Right click the icon for the menu' within 20s

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

`51` leaves the link up with the Settings window shut, and asks for the cube to be put down on Break if it is on
another face, since a face with no category has the app pause the cube as soon as it counts there. What the range
inherits is a paired cube the app has reached, with a face, a charge and a status behind it. (Since 2026-08-29 the
app reads `paired` when it is asked, so a pairing on its own is enough to make it follow the cube.)

A script whose subject *is* giving a cube up puts it back before it finishes: the wipe in `52` and the forget in
`53`. That is `restore_the_pairing`, and it is not a check, but it does stop the run if it cannot, because
everything after it would otherwise fail at a cube that is not there.

**Seven scripts take the link down and let it back up instead**, which is a different thing from pairing: `54`,
`55`, `57`, `59`, `64` and `66` call `relink_a_cube` to assert on what the app does *as a link comes up*, and `56`
ends with it. It quits and relaunches, the app reconnects to the cube it already has, and `free_the_cube` then
frees what the quit left paused and locked: Unlock, then Resume (Unlock never changes whether the cube is
paused), and a wait for the table to show the cube running. No scan, no pairing.

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

**Below `50` needs no TimeFlip; `50` and above needs one.** `00-setup` is the one exception: it is not a check,
it asks whether the run may use the cube, and on a yes sets it up for `50`. The number says what a script requires
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

**A script that cannot run on a platform declares `EXPECTED_CHECKS=0` there and is not counted as short**:
`58-wrong-pin` on macOS, shown in the stamp as 0 of 0. That is a declared difference between platforms, not a skip.

| | |
|---|---|
| `00-setup` | puts the app and the database into the state a run starts from: debug logging on, the connected Google account written back, and, when the run may use the cube, the cube paired, put on Break and factory reset (**asks whether it may use your TimeFlip, for a Google sign-in if the run has no account, and for a y once the cube is on Break**) |
| `01-launch` | the launch reaches the status item, the debug log records, and a second copy stands down before opening either database |
| `02-menu-bar` | the status item, the idle line reading Facet, its menu, and Settings from the menu |
| `03-settings-window` | the window opens, the tabs switch, it closes by the window manager, its Close button and Escape, and the run's calendar is made |
| `04-categories` | create, rename, retire, reinstate, renaming a retired one, and the alerts a namesake raises |
| `05-faces-timing` | a category on a face, the clock starting and pausing, and the menu bar naming it in cyan |
| `06-time-entries` | a finished segment becoming tracked time, and a blip not |
| `08-app-settings` | each row on the App tab written and read back |
| `09-report` | the range, the totals, folding a category open, the sorting |
| `10-google-calendar` | the account, and recorded time reaching the calendar `03` made, each event read back before it is ticked |
| `11-google-reconnect` | disconnect keeps the calendar, and signing back in still reaches it (**asks you to sign in**) |
| `12-daily-limit` | a category spending its `daily_limit` stops the clock, and every way of starting it again refuses |
| `13-device-tab` | the Device tab's two sections folding, including a fold inside a fold, and every Settings control dead with no cube |
| `14-time-zone` | a time entry, its segment and the trace filed under this machine's own zone, read from the operating system, with its local time beside it |
| `50-device-scan` | the scan lists the cube, stops when pressed, ends by itself after fifteen seconds, and All Devices widens it |
| `51-device-connect` | pairing: every step of the login, the PIN rotated or kept, what the table and the tab say afterwards, and Reset offered and called off (**asks you to put the cube on Break if it is not**) |
| `52-device-reset` | the factory reset: asked, called off, then sent, proved on the vendor PIN, and forgotten, and the wiped cube paired onto a new PIN |
| `53-device-reconnect` | a quit closing the link, a paired app reaching its own cube at launch with the window shut and the menu bar saying Connecting..., Forget, and a launch with nothing paired |
| `54-device-battery` | the charge read as the link comes up, followed from then on, and shown on the tab, and the battery warning row |
| `55-device-face` | the login's clock and face, history filed into `device_event` and growing in place, the Faces tab following the cube, and a turn opening a new segment (**asks you to turn the cube**) |
| `56-manual-mode` | a paired app that cannot find its cube: what a click refuses, and what taking manual mode stops (**asks you to switch Bluetooth off and on**) |
| `57-cube-pause` | the menu's Pause, Resume, Lock and Unlock on the cube, each read back, the pause before the lock, a left click pausing and a double click locking, and the quit leaving it paused and locked, from the menu and on a SIGTERM |
| `58-wrong-pin` | a cube that refuses this app's PIN: the not-found notice, Rescan with the PIN still wrong, Time by Hand, then the real PIN put back and a fresh launch reaching the cube. Asks for nothing. **Not run on macOS** (declares no checks there), where the Keychain prompts after the PIN item is rewritten |
| `59-double-tap` | the cube's double tap kept off: its registers read at every login with the window at 0, and nothing sent to change them |
| `60-device-backlog` | a cube out of range: what the app shows, what it refuses to write, and what the cube backfills when it returns (**asks you to switch Bluetooth off, turn the cube onto Meeting, switch Bluetooth on, and turn the cube back onto Break**) |
| `61-lock-without-pause` | locking the cube from the menu with `pause_on_lock` off |
| `62-forced-pause` | the app stopping the cube itself: a face with no category, lifted once the face is given one, and a category that has spent its `daily_limit` (**asks you to turn the cube twice**) |
| `63-led-settings` | the cube LED: brightness and blink period stepped, sent to the cube, then written down |
| `64-face-colours` | the cube lit in its faces' colours: twelve on connecting, and a face relit when its category is recoloured |
| `65-auto-pause` | the cube auto-pause delay: stepped, sent as `0x05`, read back with `0x10`, then written down, and the cube stopping itself on it (**asks you to turn the cube, then to leave it alone for a minute**) |
| `66-device-rename` | the cube renamed from the Device tab: `0x15` to the hardware, the row written only after it, and the cube still found afterwards |
| `67-pause-on-lock` | the pause-on-lock row: written to the table and sent nowhere |
| `68-device-link-lost` | the link dropping is noticed and recorded, the pairing kept, and, once Bluetooth is back, the app reaching the cube again by itself with no relaunch (**asks you to switch Bluetooth off and on**) |
| `69-history-timer` | the history timer firing on the interval the table holds, and a changed interval read at the next arming with no relaunch |
| `99-quit` | the cube factory reset, so it is left on the vendor PIN, and the app quitting |

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
EXPECTED_CHECKS=2
start "what this script checks"

since=$(mark)
press some-button
expect_log "the button did the thing" "$since" "%the thing%"

check "the table agrees" "1" "$(sql 'SELECT ...;')"

finish
```

**`EXPECTED_CHECKS` is how many checks the script passes when everything works**, a bare number at the start of a
line. `finish` fails a script that passes any other number, and `scripts/check-scripted-stamps.sh` refuses a
script without one.

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
database with `require_test_database` and declares `EXPECTED_CHECKS` (and that `platform.sh`, `lib.sh` and
`run.sh` parse and are executable). Then, **for each of the two stamps**,
it requires all of:

- the run was on **this** branch;
- it **passed**, with zero failing checks;
- **every numbered script ran, and each passed exactly the checks it declares.** This suite has no skip
  verdict, so a check that could not answer shows here as a script short of its `EXPECTED_CHECKS`. In
  practice a run meant for a pull request needs the cube in reach, a Google account connected and every prompt
  answered. `58-wrong-pin` on macOS declares no checks and is not counted as short. The failure names each short
  script;
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

python3 scripts/ax-dump.py --app facet-mac      # macOS: what the script can see and press
python3 scripts/at-dump.py --app facet-linux    # Linux
tail -40 logs/app.log                           # what the app printed itself; a crash on the way up lands only here
```

[`Tests/Methods.md`](../Methods.md) is the reference for both, and for the traps that have already cost
time: what needs a real mouse event, and why a status item is not in the menu bar's accessibility tree.

**One trap from the Swift suite no longer applies.** Launching the `.app` directly could run a binary
older than the change being tested. There is no `.app`: the binary is what cargo built, and `run.sh`
builds before it launches.
