# The scripted suite, and converting it

[← Back to README](../README.md) · [`Tests/Scripted/README.md`](../Tests/Scripted/README.md) · [Methods →](../Tests/Methods.md) · [Port findings →](port-findings.md)

**The suite that drives a running copy of the app, with a real cube, by accessibility.** It is the only
thing that can say the app works on hardware; everything the crate tests prove is proven against doubles.

**What is in this repository today is the harness and not the checks.** `Tests/Scripted/` carries
`run.sh`, `lib.sh`, `testlog.sh`, `seed-private.sh`, `stepper-timing.py` and the suite's own README, all
carried over from `feature/linuxPort` where they had already been made to drive **two** platforms. The 32
checks are not here, because a check for a feature the Rust app does not have yet cannot pass, and a tree
full of red that nobody can act on teaches nothing. **They are listed below and each one is added back as
its feature lands.**

**Read `Tests/Scripted/README.md` before writing one.** It is the suite's own manual and it came across
intact.

---

## The 32 checks, and the order they came in

Numbering is meaningful: `00` sets up, `01`–`13` need no cube, `50`–`66` need one, `99` shuts down. Keep
the numbers as each check is re-added, so a gap is visibly a gap.

**To read any of them in full:**

```sh
cat ~/harry.git/TimeFlipLinux/Tests/Scripted/55-device-face.sh
```

### No cube required

| | What it proves |
|---|---|
| `00-setup` | Puts the app, the database and the cube into the state every other script starts from |
| `01-launch` | The app starts, opens the database it was told to, records what it is doing, and refuses to run twice |
| `02-menu-bar` | The status item: what it says, what its menu holds, and that the two halves do different things |
| `03-settings-window` | The five tabs, moving between them, and closing the window |
| `04-categories` | Creating a category, renaming it, retiring it, and bringing it back |
| `05-faces-timing` | Picking a category starts the clock on it, and pausing stops it |
| `06-time-entries` | A finished segment becoming tracked time, and a flick past a face not becoming anything |
| `07-history-timer` | It runs while something is being timed, and stops when nothing is |
| `08-app-settings` | Every row on the App tab written to the table, and put back again |
| `09-report` | Picking a range, what it totals, folding a category open, and the two sort columns |
| `10-google-calendar` | The Google section, and recorded time reaching the calendar |
| `11-google-reconnect` | Disconnecting an account and connecting it again, with the calendar surviving in between |
| `12-daily-limit` | Reaching the hard limit stops the clock, and the app then refuses to start it again |
| `13-device-tab` | The Device tab's two sections, and the folds that need no cube |

### Cube required

| | What it proves |
|---|---|
| `50-device-scan` | Looking for a TimeFlip, and finding one |
| `51-device-connect` | Reaching the cube, getting a PIN accepted, leaving the cube on a PIN of the app's own. **Asserts on the raw `commandResult: 02`**, so a firmware release that ever matches the document fails a check rather than silently admitting the wrong cube |
| `52-device-reset` | Putting a cube back to how it left the factory, and proving it took |
| `53-device-reconnect` | Getting back to the cube by itself, at launch, with nobody watching |
| `54-device-battery` | Read once on connecting, then pushed, and drawn as one steady figure |
| `55-device-face` | The resting face: asked for when the link comes up, followed on every turn after |
| `56-manual-mode` | Manual mode with a device still paired: what a click may do before it is chosen, and what the app stops doing after |
| `57-cube-pause` | A single click stops the cube and starts it again |
| `58-wrong-pin` | A paired cube that refuses this app's PIN: the offer, Rescan, and taking timing by hand |
| `59-double-tap` | The cube's double tap, which this app turns off and never turns back on |
| `60-device-backlog` | A cube that goes out of range while timing, is turned while nobody can hear it, and comes back |
| `61-lock-without-pause` | Locking with `pause_on_lock` off: the lock still goes, only the pause is skipped |
| `62-forced-pause` | The app stopping the cube itself: a face with nothing on it, and a category that has spent its day |
| `63-led-settings` | Brightness and blink period, stepped, sent, and **only then** written down |
| `64-face-colours` | `0x11`, twelve colours when a cube connects and one when a face changes |
| `65-auto-pause` | The delay stepped, sent as `0x05`, read back with `0x10`, and only then written down |
| `66-device-rename` | `0x15` to the hardware, the row written only after it |
| `99-quit` | The way out closes what was left open, and the cube is left as the factory made it |

**Four of them (`63`, `65`, `66`, and `51`'s PIN half) are the read-back rule made visible**: the app asks
the cube, waits for the cube to confirm, and only then writes the row. That ordering is the assertion, not
the value.

---

## What converts, what does not

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

### Five things to fix while converting

1. **A bare touch area is a silent pass.** `ax-press.py` prints `pressed` and exits 0 against one. **A
   guard belongs in that script**, and the design rule is that anything a check must press is a real
   button or carries an accessibility action of its own.
2. **Slint refuses `accessible-id` unless `accessible-role` is set alongside it.** Every control a check
   must find needs both.
3. **The status item loses its handle.** `MenuBarController` set an accessibility identifier on the status
   item button and `scripts/status-item-click.py` found it that way. `tray-icon` exposes no equivalent.
   **The Linux answer already works and transfers: address a tray item by its label.** This is the one
   real line item against a suite whose front door is the status item.
4. **`ax-set.py` and `ax-key.py` hardcode the app.** `ax-set.py` runs `pgrep -x Facet` and `ax-key.py`
   refuses to run unless Facet is running, so neither takes `--app` the way `ax-press.py` does. Two
   one-liners.
5. **`lib.sh` and `run.sh` are the dual-platform versions.** They already know about both the AX and the
   AT-SPI driver sets. Do not regress that while adapting them.

### What was never tested

Sorting, a row leaving the list while it is being edited, the icon grid, the status item itself, and any
of it on Linux or Windows.

---

## The drivers

`scripts/` carries both sets, and they are the suite's whole interface to a running app.

| macOS | Linux | |
|---|---|---|
| `ax-press.py` | `at-press.py` | Perform the press action on a control found by identifier |
| `ax-set.py` | `at-set.py` | Write a value |
| `ax-key.py` | `at-key.py` | A real keystroke |
| `ax-hold.py` | `at-hold.py` | Press and hold, for the stepper |
| `ax-dump.py` | `at-dump.py`, `atspi_tree.py` | Dump the tree, which is how a locator is found in the first place |
| `ax-alert.py` | `at-alert.py` | Drive a dialogue |
| `status-item-click.py` | `tray-menu.py` | The menu bar. **Different mechanisms**: a real mouse event on macOS, D-Bus on Linux |
| | `at-clipboard.py` | |

**On Linux the tray is driven over D-Bus and works while the session is doing something else.** On macOS
it costs a real mouse event and a frontmost app. That asymmetry is in the protocol, not in the scripts.

---

## Two suites, and only one of them can tell you it works

The crate tests run against doubles and prove the rules. **They cannot tell you the app works**, and the
evidence for that is on the record: the Swift suite was green at 1,718 tests on the day the app was first
put in front of a real cube on Linux, and that session found three real faults in an afternoon.

**Ask for the cube whenever a change touches the device half.** A green crate suite is not a substitute
and never has been.
