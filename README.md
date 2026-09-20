# Facet

**A desktop time tracker driven by a TimeFlip2 cube.** Turn the cube onto a face, and the time spent on
that face is recorded against the category assigned to it. It lives in the menu bar, keeps everything in a
local SQLite database, and can push recorded time to a Google Calendar it owns.

**This repository is the Rust rewrite, and it is at the beginning.** The working application is the Swift
one at [`tuxcomputers/TimeFlipApp`](https://github.com/tuxcomputers/TimeFlipApp), now frozen. What is here
is the scaffolding and, more importantly, **everything that was measured before the rewrite started**.

---

## Read this first

**[`docs/rust-port.md`](docs/rust-port.md) is why this repository exists.** It records the requirements the
language choice was judged against, the decision, and every measurement behind it. Each claim in it is
marked **measured** or **untested**, and that distinction is the whole value of the file.

The decision in one line: **Rust, because of Bluetooth, not because of the UI.** `btleplug` is one API over
CoreBluetooth, BlueZ and WinRT, and it was driven against the real cube on 2026-09-20 doing every step this
app needs. Every other concern has many cross-platform answers; the radio has almost none.

---

## What is in here

| | |
|---|---|
| [`docs/`](docs/) | The vendor protocol, what the hardware actually does, the schema, the architecture, and what the Swift port measured |
| [`crates/`](crates/) | `facet-core` plus one crate per platform. Nothing implemented yet |
| [`probe/`](probe/) | Two Rust programs that answered a question and can be re-run |
| [`scripts/`](scripts/) | The accessibility drivers for both platforms, the BLE probe, and the database tooling |
| [`Tests/Scripted/`](Tests/Scripted/) | The harness that drives a running app against a real cube |

### The documentation, in reading order

**Start here.**

- [`rust-port.md`](docs/rust-port.md): the decision and the measurements behind it
- [`architecture.md`](docs/architecture.md): the platform-blind core, the ports, and what the Swift split
  proved about them
- [`port-findings.md`](docs/port-findings.md): facts that cost real time to find and are not about Swift

**The product.**

- [`workflow.md`](docs/workflow.md): what the device owner is trying to do, which is the *why* behind the
  schema
- [`operation-spec.md`](docs/operation-spec.md): how a device event becomes a stored row
- [`database-design.md`](docs/database-design.md): every table, in DDL order
- [`state-reference.md`](docs/state-reference.md): the one name for every state the app branches on
- [`behaviour-inventory.md`](docs/behaviour-inventory.md): what the 2,063 Swift tests pin down, and where
  to read each one

**The hardware.** Consulted in this order, first one that answers wins:

1. [`timeflip2-firmware-observations.md`](docs/timeflip2-firmware-observations.md): fourteen findings
   measured on the cube, several of which contradict the spec. `timeflip2-firmware-evidence.sqlite` holds
   the rows behind them
2. [`TimeFlip2 BLE Protocol v4.3.md`](docs/TimeFlip2%20BLE%20Protocol%20v4.3.md): the vendor spec, with
   [v3.0](docs/TimeFlip%20BLE%20Protocol%20v3.0.md) and the
   [API PDF](docs/TimeFlip%20API%20Documentation%2005.2025.pdf) alongside it
3. [`timeflip.md`](docs/timeflip.md): the BLE surface as this app uses it
4. [`linux-bluez-port-notes.md`](docs/linux-bluez-port-notes.md): where BlueZ differs from CoreBluetooth,
   and which differences cost an afternoon

**Everything else.**

- [`scripted-suite.md`](docs/scripted-suite.md): the 32 checks, and what converts
- [`google-oauth-setup.md`](docs/google-oauth-setup.md): the Cloud project, and what the app must do
- [`about-tab.md`](docs/about-tab.md): the one piece of UI the licence requires, and the open
  questions on the update check
- [`system-mac.md`](docs/system-mac.md): the Mac, and everything needed to build and drive Facet
  on it
- [`system-linux.md`](docs/system-linux.md): the Linux box, and what is not yet measured there

---

## The stack

| | | |
|---|---|---|
| Radio | `btleplug` 0.13.1 | **Measured** against the cube, macOS, 2026-09-20 |
| UI | `slint` 1.18.0, one style on all platforms | **Measured**: a five-tab prototype, and an editable table driven by the suite's own scripts |
| Database | `rusqlite` 0.32.1, `bundled` | **Compiles**, with SQLite built in. The schema carries over unchanged |
| Menu bar | `tray-icon` 0.25.1, `ksni` 0.3.6 on Linux | Untested from Rust; the MATE click behaviour was measured 2026-09-18 |
| Secrets | `keyring` 4.2.0 | Untested |

**One self-contained binary per platform.** The user installs no runtime and no toolkit. Build-time
dependencies are unconstrained.

**Where the three platforms cannot be made identical**, and it is the operating systems rather than the
language: **Windows can never show text beside the tray icon**, and on Linux the desktop owns the right
click. The design rule that follows is that **nothing lives behind a left click that has no menu
equivalent**. [`rust-port.md`](docs/rust-port.md) has the table.

---

## Building

**Cargo is a rustup install in the home directory, not Homebrew, and `~/.cargo/bin` is not on the PATH
a non-interactive shell gets.** No dotfile sources `~/.cargo/env`, so a bare `cargo` fails with *command
not found* even though the toolchain is fine. Measured 2026-09-20: cargo 1.98.1, rustc 1.98.1, one stable
`aarch64-apple-darwin` toolchain. Either source it or call it by absolute path:

```sh
. "$HOME/.cargo/env"        # or: export PATH="$HOME/.cargo/bin:$PATH"
```

**A bare build builds the core only.** The three platform crates are each buildable on exactly one
machine, so `default-members` is the core and the native one is named explicitly:

```sh
cargo build                 # facet-core
cargo test                  # facet-core, the hermetic suite
cargo build -p facet-mac    # or facet-linux, or facet-windows, on that machine
```

**The probes are excluded from the workspace on purpose** and resolve their own dependencies, so
re-running one reproduces the transcript in its README rather than whatever the app is pinned to today.
Both build as of 2026-09-20:

```sh
(cd probe/timeflip-btleplug && cargo run)     # needs the cube
(cd probe/slint-editable-table && cargo run)  # opens a window
```

They are the evidence behind the two largest decisions and are worth running first.

---

## Provenance

**This is AI-generated code all the way down, and it is worth being honest about that.** The original
author, [growler](https://github.com/growler), vibecoded the base project, including the core Bluetooth
layer that talks to the TimeFlip2, mostly with OpenAI Codex, and has said themselves they had never
written for macOS before. Everything built on top of that fork is the same story: the code was written by
Claude, and the design decisions are mine (Harry Phillips), for better or worse.

**The Swift repository is frozen at `tuxcomputers/TimeFlipApp` and remains readable.**
`feature/linuxPort` carries the furthest state of the app and the Linux port, `feature/rustPort` carries
the probes and the evaluation, and `main` is the last release. Docs here cite it by branch and path where
reading the original is worth it.

**Anything found over there that turns out to matter gets written into `docs/` in the same change.** A
fact that only exists in a frozen tree is a fact somebody pays for twice.

## Licence

[Apache License 2.0](LICENSE). Copyright 2026 Harry Phillips.

**[`NOTICE`](NOTICE) is part of the licence, not decoration.** Two things shipped with Facet are not
covered by Apache-2.0, and anyone redistributing it has to satisfy both.

**Slint is tri-licensed** (`GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR
LicenseRef-Slint-Software-3.0`) and Facet takes the royalty-free option, which requires attribution.
Facet satisfies it by carrying the `AboutSlint` widget in the About tab, reachable from the menu.
**That widget is a licence condition rather than a courtesy**, so removing it, or making the About
tab unreachable, puts a build out of compliance.

**The activity icons are TimeFlip's copyrighted set.** Permission was granted for this project
specifically and **does not transfer with the code**. Apache-2.0 covers Facet's own code, not the
icons. If you fork this and want to distribute it with them, get your own permission from TimeFlip
first; without it, remove or replace them before sharing it on.
