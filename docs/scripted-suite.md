# The scripted suite, and converting it

[← Back to README](../README.md) · [`Tests/Scripted/README.md`](../Tests/Scripted/README.md) · [Methods →](../Tests/Methods.md) · [Port findings →](port-findings.md)

**The suite that drives a running copy of the app, with a real cube, by accessibility.** It is the only
thing that can say the app works on hardware; everything the crate tests prove is proven against doubles.

**Every script is back, as of 2026-10-03: all 35 pass in full on both machines.** On that date the Linux run
was 865 checks and the Mac run 833, and the stamps are the source of those numbers, each with a row per script:
`Tests/Scripted/last-run-linux.md` and `last-run-mac.md`. `58-wrong-pin` is not run on macOS and declares no
checks there, so the Mac stamp shows it as 0 of 0; it runs in full on Linux. Every Swift script has a Rust
counterpart: the Swift `07-history-timer` is `69`, numbered after `51` because it needs a paired cube, and
`14-time-zone`, `67` and `68` are new with the Rust app. What each script had to drop or change because the
Rust app is built differently is said in its own header.

**`11` needs somebody to sign in to Google on every full run.** It presses Sign in, the browser opens on that
machine's screen, and it waits four minutes for the account to come back. Nobody signing in is a fail.

**A full run needs somebody there.** `00-setup` asks whether the run may use your TimeFlip, then (when the run
has no Google account) for a Google sign-in, then for a y once the cube is on Break; `51` asks for the Break turn
if the cube is elsewhere; `11` asks for a sign-in on every run; `55`, `62` and `65` ask for turns of the cube;
`56`, `60` and `68` ask for Bluetooth off and on (`60` also asks for turns). Each waits and carries on by itself
once it sees the change. Everything else runs unattended.

**What else is in this repository is the harness.** `Tests/Scripted/` carries `run.sh`, `lib.sh`, `platform.sh`,
`testlog.sh`, `seed-private.sh` and the suite's own README, which drive **two** platforms, beside the 35 numbered
scripts and each machine's stamp. CI refuses a pull request whose stamps do not show every script passing on both
machines (`scripts/check-scripted-stamps.sh`).

**Read `Tests/Scripted/README.md` before writing one.** It is the suite's own manual.

---

## The 35 scripts

Numbering is meaningful: `00` sets up, `01`–`14` need no cube, `50`–`69` need one, `99` shuts down. `07` is
empty on purpose: the Swift `07-history-timer` needs a paired cube and is `69`. A new script takes the next free
number in its own half and nothing is renumbered.

**Each script is in `Tests/Scripted/`.** The Swift originals, for seeing what a conversion dropped, are in the
reference tree (`~/harry.git/TimeFlipApp` on the Linux box):

```sh
cat ~/harry.git/TimeFlipLinux/Tests/Scripted/55-device-face.sh
```

### No cube required

| | What it proves | Needs |
|---|---|---|
| `00-setup` | Puts the app, the database and the cube into the state every other script starts from: debug logging on, the Google account written back, and, when the run may use the cube, the cube paired, put on Break and factory reset | Hands: whether the cube may be used, a Google sign-in if there is no account, a y once the cube is on Break |
| `01-launch` | The app starts, records what it is doing, and a second copy stands down before opening either database | None |
| `02-menu-bar` | The status item: its idle line reading Facet, what its menu holds (Settings, About, Quit, one Pause item), and Settings from the menu | None |
| `03-settings-window` | The six tabs, moving between them, closing the window three ways (window manager, Close, Escape), and the calendar the run fills | The Google account `00` seeds |
| `04-categories` | Creating a category, renaming it, retiring it, and bringing it back, and the notices a namesake raises | None |
| `05-faces-timing` | Picking a category starts the clock on it, pausing stops it, and the tray follows the clock | None |
| `06-time-entries` | A finished segment becoming tracked time (and reaching the calendar when an account is connected), and a flick past a face not becoming anything | None |
| `08-app-settings` | Every row on the App tab written to the table, and put back again | None |
| `09-report` | Picking a range, what it totals, folding a category open, and the two sort columns | None |
| `10-google-calendar` | The Google section, and recorded time reaching the calendar | The Google account `00` seeds |
| `11-google-reconnect` | Disconnecting an account and connecting it again, with the calendar surviving in between | Hands: a Google sign-in, every full run, waited for up to four minutes |
| `12-daily-limit` | Reaching the hard limit stops the clock, and the app then refuses to start it again | None |
| `13-device-tab` | The Device tab's two sections, and the folds that need no cube | None |
| `14-time-zone` | A time entry, its segment and the trace are filed under this machine's own zone, with the machine's own local time beside it. New with the Rust app | None |

### Cube required

| | What it proves | Needs |
|---|---|---|
| `50-device-scan` | Looking for a TimeFlip, and finding one | Cube in reach, Bluetooth on, and the yes `00` recorded |
| `51-device-connect` | Reaching the cube, getting a PIN accepted, leaving the cube on a PIN of the app's own. **Asserts on the raw `commandResult: 02`**, so a firmware release that ever matches the document fails a check rather than silently admitting the wrong cube | Cube; hands only if it is not on Break |
| `52-device-reset` | Putting a cube back to how it left the factory, and proving it took | Cube |
| `53-device-reconnect` | Getting back to the cube by itself, at launch, with nobody watching | Cube |
| `54-device-battery` | The charge read once the cube has accepted the PIN, followed from then on and shown on the Device tab, and the battery warning row kept in the table and sent nowhere | Cube |
| `55-device-face` | The resting face: asked for when the link comes up, followed on every turn after | Hands: turn the cube onto Meeting and back onto Break |
| `56-manual-mode` | A paired launch that cannot find its cube: the notice (Rescan, Time by Hand, Quit), what a click may do before a choice, and what Time by Hand stops | Hands: Bluetooth off, then on |
| `57-cube-pause` | The menu's Pause, Resume, Lock and Unlock on the cube, each read back, the pause going before the lock, a left click pausing, a double click locking, and the quit (menu and SIGTERM) leaving it paused and locked | Cube |
| `58-wrong-pin` | A paired cube that refuses this app's PIN: the not-found notice, Rescan, and Time by Hand, then the real PIN put back. **Not run on macOS** (declares no checks there), where the Keychain prompts after the PIN item is rewritten | Cube, Linux only; no hands |
| `59-double-tap` | The cube's double tap kept off: its registers read at every login, the window left at 0, and nothing sent to change them | Cube |
| `60-device-backlog` | A cube that goes out of range while timing, is turned while nobody can hear it, and comes back | Hands: Bluetooth off, turn onto Meeting, Bluetooth on, turn back onto Break |
| `61-lock-without-pause` | Locking with `pause_on_lock` off: the lock still goes, only the pause is skipped | Cube |
| `62-forced-pause` | The app stopping the cube itself: a face with nothing on it, and a category that has spent its day | Hands: turn onto a face with no category, then back onto Break |
| `63-led-settings` | Brightness and blink period, stepped, sent, and **only then** written down | Cube |
| `64-face-colours` | `0x11`: twelve colours as a cube connects, and the faces a category wears relit when it is recoloured | Cube |
| `65-auto-pause` | The delay stepped, sent as `0x05`, read back with `0x10`, and only then written down, and the cube stopping itself on it | Hands: turn onto Meeting and leave it a minute, then back onto Break |
| `66-device-rename` | `0x15` to the hardware, the row written only after it, and the name surviving the next connection | Cube |
| `67-pause-on-lock` | The pause-on-lock row, kept in the table and sent nowhere. **New with the Rust app** | Cube |
| `68-device-link-lost` | A held link dropping: noticed, recorded, the pairing kept, and the app reaching the cube again by itself once Bluetooth is back, with no relaunch. **New with the Rust app** | Hands: Bluetooth off, then on |
| `69-history-timer` | The history timer firing on the table's interval while the link is held, and a changed interval read at the next arming. **The Swift `07`**, whose timer ran only while something was timed | Cube |
| `99-quit` | The way out closes what was left open, and the cube is factory reset so it is left on the vendor PIN | Cube |

**Two of them (`65`, and `51`'s PIN half) wait for the cube's own answer**: `65` sends `0x05`, waits for the
`0x10` read-back carrying the new delay, and only then writes the row; `51` asserts on the raw
`commandResult: 02`. **`63` and `66` have no read-back** (the LED commands and `0x15` carry none): they assert
that the row is written after the cube acknowledged the write and never before. That ordering is the assertion,
not the value.

---

## What was measured against Slint

**Measured 2026-09-20 against the Slint probe**, using the suite's own mechanisms rather than a stand-in.
This is the part of [rust-port.md](rust-port.md) that this suite depends on.

| Mechanism | Script | Against Slint |
|---|---|---|
| Press by accessibility identifier | `scripts/ax-press.py` | **Works** |
| Write a value | `scripts/ax-set.py` | **Works**, and fires the widget's own edited callback |
| A real keystroke | `scripts/ax-key.py` | **Works.** Return committed, and the asynchronous write landed 800 ms later |
| Dump the tree | `scripts/ax-dump.py` | **Works.** Per-row ids built by concatenation inside a loop come through intact |
| Press a bare touch area | | **Does nothing, and reports success** |

**So the locator model converts rather than being reinvented**, which was not a given and was briefly
concluded to be false. [port-findings.md](port-findings.md) records why that conclusion was wrong and
what it cost.

### Five things to know about the drivers

1. **A bare touch area is a silent pass on macOS.** `ax-press.py` prints `pressed` and exits 0 against one, where
   `at-press.py` refuses a control that exposes no accessible action. The guard is still to be added to
   `ax-press.py`, and the design rule stands: anything a check must press is a real button or carries an
   accessibility action of its own.
2. **Slint refuses `accessible-id` unless `accessible-role` is set alongside it.** Every control a check
   must find needs both.
3. **The status item's handle is set by the app.** `facet-mac` sets `status-item` on the status item's button, so
   `ax-dump.py --menu-bar` and `status-item-click.py` find it. The menu's items all carry the same `AXIdentifier`
   (`fireMenuItemAction:`), so on both platforms **a menu item is addressed by its label**.
4. **Every macOS driver takes the app's name from `FACET_APP_NAME`**, which `platform.sh` exports (`facet-mac`);
   most also take `--app`.
5. **`lib.sh`, `run.sh` and `platform.sh` drive both the AX and the AT-SPI sets.** Keep it that way.

### Not covered

Windows (`facet-windows` is a stub and has no driver set), and what the drivers cannot reach: a bare touch area,
and on Linux a disabled element, which AT-SPI does not report ([port-findings.md](port-findings.md), Linux
fact 4).

---

## The drivers

`scripts/` carries both sets, and they are the suite's whole interface to a running app.

| macOS | Linux | |
|---|---|---|
| `ax-press.py` | `at-press.py` | Perform the press action on a control found by identifier |
| `ax-set.py` | `at-set.py` | Write a value |
| `ax-key.py` | `at-key.py` | A real keystroke |
| `ax-hold.py` | `at-hold.py` | Press and hold. **No script uses these now**: the Rust steppers are Slint SpinBoxes with no arrows to hold |
| `ax-dump.py` | `at-dump.py`, `atspi_tree.py` | Dump the tree, which is how a locator is found in the first place |
| `ax-alert.py` | `at-alert.py` | Read a native alert's buttons and message. Only the fallback: the Rust app's questions are in-window notices (`notice-choice-<n>`) read from the ordinary dump |
| `status-item-click.py` | `tray-menu.py` | The status item. **Different mechanisms**: on macOS a real mouse event for the left click and the double click only, the menu being read and pressed through the accessibility tree (`ax-dump.py --menu-bar`, `ax-press.py --title`); on Linux everything over D-Bus |
| | `at-clipboard.py` | Puts text on the X clipboard for `04`'s paste check (macOS uses `pbcopy`) |

**On Linux the tray is driven over D-Bus and works while the session is doing something else.** On macOS only
the left click and the double click cost a real mouse event and a frontmost app; menu items take a press by label
with the menu closed. That asymmetry is in the protocol, not in the scripts.

---

## Two suites, and only one of them can tell you it works

The crate tests run against doubles and prove the rules. **They cannot tell you the app works**, and the
evidence for that is on the record: the Swift suite was green at 1,718 tests on the day the app was first
put in front of a real cube on Linux, and that session found three real faults in an afternoon.

**Ask for the cube whenever a change touches the device half.** A green crate suite is not a substitute
and never has been.
