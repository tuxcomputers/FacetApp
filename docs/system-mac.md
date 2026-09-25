# The Mac

[← Back to README](../README.md) · [The Linux box →](system-linux.md) · [What is being asked of this machine →](handover-mac.md) · [Port findings →](port-findings.md)

**What this machine is, and everything needed to build and drive Facet on it.** The Mac is where the app
is built and where the cube normally lives.

**Every line is either measured, with the date, or it is marked unknown.** Nothing in between, and
nothing inferred from what should be the case. A guess written into a facts file is worse than no line
at all, because the next reader cannot tell it from a measurement.

---

## The machine

| | |
|---|---|
| Model | MacBook Pro, `Mac17,2` |
| Chip | Apple M5, 10 cores (4 performance, 6 efficiency) |
| Memory | 32 GB |
| Architecture | `arm64` |
| macOS | **26.6.2**, build `25G83` |
| Kernel | Darwin 25.6.0, `RELEASE_ARM64_T8142` |

`sw_vers`, `uname -a`. Measured 2026-09-20.

---

## Building Facet

**Everything in this section was measured on 2026-09-20 by running it**, not read off a requirements
list. A full `cargo build` of the workspace and of both probes succeeds with exactly what is described
here and nothing else installed.

### Rust

| | |
|---|---|
| cargo | **1.98.1** (`797e8a9bc`, 2026-08-05) |
| rustc | **1.98.1** (`48a229cea`, 2026-09-01) |
| rustup | **1.29.1** |
| Toolchains | One: `stable-aarch64-apple-darwin`. No nightly |
| Targets | One: `aarch64-apple-darwin`. **Nothing here cross-compiles** |
| Installed by | rustup, into `~/.cargo` and `~/.rustup`. **Not Homebrew** |
| `clippy` | **0.1.98** (`48a229cea`, 2026-09-01), a rustup component, added 2026-09-23 |
| `rustfmt` | **1.9.0-stable** (`48a229cea`, 2026-09-01), a rustup component, added 2026-09-23 |

**`~/.cargo/bin` is not on the PATH a non-interactive shell gets, and no dotfile fixes it.** Checked
`.zshrc`, `.zprofile`, `.bash_profile`, `.bashrc` and `.profile`: **none mentions cargo**, and none
sources `~/.cargo/env`. So a bare `cargo` fails with *command not found* while the toolchain is
perfectly healthy, which reads like a missing install and is not one. An interactive terminal may
differ if a terminal profile sets PATH; nothing in the dotfiles does, and that is the part measured.

```sh
. "$HOME/.cargo/env"          # or: export PATH="$HOME/.cargo/bin:$PATH"
```

**With one target installed**, the Linux and Windows crates are built on their own machines rather than
from here.

### The Apple toolchain, and what actually needs it

| | |
|---|---|
| Xcode | **27.0**, build `27A266a` |
| Command line tools | `/Applications/Xcode.app/Contents/Developer` |
| `cc` | Apple clang **21.0.0** (`clang-2100.3.34.2`) |
| `pkg-config` | 3.0.7 |

**A C compiler is a hard requirement, and it is `rusqlite` that needs it**, not Rust. The `bundled`
feature compiles SQLite's own C source into the binary, so `libsqlite3-sys` invokes `cc` on every clean
build. That is also what makes the shipped binary self-contained: **nothing links the system SQLite and
no SQLite needs installing on a user's machine.**

**Xcode has moved since this was last written down** (it read 26.6 build `17F113`). Nothing broke, which
is expected: the build wants a working `cc` and linker, not a specific Xcode.

### What the UI needs, and what it does not

**No cmake, no ninja, and neither is installed.** This was worth measuring because `cargo metadata`
lists `i-slint-renderer-skia` in the package graph, and C++ Skia is a cmake build. It is **never
compiled**: it is an unenabled optional dependency. What actually builds is

```
i-slint-renderer-femtovg     OpenGL, through glutin
i-slint-renderer-software    tiny-skia, which is pure Rust and not C++ Skia
```

verified by `cargo tree -e normal` and by the crates present in `target/debug/deps`.

**So enabling Slint's Skia renderer later is not a feature flag, it is a new build dependency.** It
would want cmake and ninja, neither of which is here. Worth knowing before somebody flips it to chase
rendering quality.

### The commands

```sh
cargo build                 # facet-core and facet-ui, by default-members
cargo test                  # the hermetic suite
cargo build -p facet-mac    # the native binary
```

**A bare build is the two crates that compile anywhere**, because the three platform crates are each
buildable on exactly one machine and this one cannot compile the BlueZ or WinRT adapters.

**The probes are excluded from the workspace** and resolve their own dependencies:

```sh
(cd probe/timeflip-btleplug && cargo run)        # needs the cube
(cd probe/slint-editable-table && cargo run)     # opens a window
(cd probe/keyring-secret-service && cargo run)   # touches the login keychain, and cleans up
```

The first two built on 2026-09-20, on btleplug 0.13.1 and slint 1.18.0; the third on 2026-09-22.

### The Keychain, through `keyring`

**Measured 2026-09-22 by running `probe/keyring-secret-service`, which was written on the Linux box and
is unchanged here.** Every check passed: store status available, absent before writing, `set_password`
and `get_password` round-tripping, `set_secret` and `get_secret` byte-clean over 5 bytes including a
non-UTF-8 pair, delete, and `NoEntry` afterwards. **That last one is what the app depends on**, needing to
tell *no PIN has ever been stored* from *the secret store is broken*.

| | |
|---|---|
| Crate | `keyring` 4.2.0 at its defaults, the version the workspace pins |
| Store selected | `apple-native-keyring-store` 1.0.2, target-gated in from the same dependency line that gives the Linux box `zbus-secret-service-keyring-store` |
| Links | **`Security.framework`** and `CoreFoundation`, plus `libiconv` and `libSystem`. No third-party library and nothing to install |
| Prompt on a first write and read back | **none**, the writing process being the reading one |

**So the one dependency line serves both machines**, which was read off a manifest before it was run and
is now measured. It was worth running anyway: the equivalent inference about `libsecret-1-dev` on the
Linux box turned out to be wrong in the direction nobody expected.

**A locked Keychain blocks the caller, the same as Linux.** Measured 2026-09-22: with `login.keychain-db`
locked by `security lock-keychain`, `get_password` did not return and was killed at a 25 second cap, a
`SecurityAgent` dialog appeared, and **the dialog stayed on screen after the caller was killed**. So a
stored secret needs its own timeout and must not be read on the launch path, on this machine as much as on
the other one. See
[port-findings.md](port-findings.md#macos-does-the-same-thing-so-these-are-rules-for-the-port).

**`security show-keychain-info` is not a safe way to check the lock state**, which cost one invalid
measurement here: against a locked keychain it raises a password dialog of its own, and answering that
unlocks the keychain. Watch for the `SecurityAgent` process instead.

---

## Command-line tools

| Tool | Version | Path | What needs it |
|---|---|---|---|
| `bash` | **5.3.15** | `/opt/homebrew/bin/bash` | The scripted suite, and every shell script here |
| `python3` | 3.14.7 | `/Library/Frameworks/Python.framework/Versions/3.14/bin/python3` (python.org, **not** the system one) | The `ax-*.py` drivers |
| pyobjc | 12.1 | into that python | The accessibility API |
| `sqlite3` | 3.51.0 | `/usr/bin/sqlite3` | `switch-database.sh`, `compare-database-to-ddl.sh`, reading the evidence database. **Not the app**, which bundles its own |
| `git` | 2.50.1 | `/usr/bin/git` | |
| `gh` | 2.100.0 | `/opt/homebrew/bin/gh` | The remote workflow, and git's credential helper |
| `jq` | 1.7.1 | `/usr/bin/jq` | |
| `brew` | 6.0.22 | `/opt/homebrew/bin/brew` | |
| `cmake`, `ninja` | **not installed** | | Only if Slint's Skia renderer is ever enabled |

**pyobjc is installed into the python.org framework build specifically**, which is why the drivers name
`python3` from that path and not the system one. `pyobjc-core`, `-framework-Cocoa`,
`-framework-Quartz`, `-framework-ApplicationServices` and `-framework-CoreText`, all 12.1, with `objc`,
`AppKit`, `Quartz` and `ApplicationServices` all importable.

### The scripted suite runs under bash 5

`Tests/Scripted/run.sh` invokes each check as `bash "$script"`, so the interpreter comes from `PATH`
rather than from the `#!/bin/bash` line in the script. That resolves to Homebrew's **5.3.15**, and the
suite is always run through `run.sh`, never a script at a time.

Apple's `/bin/bash` is **3.2.57**, from 2007, and is still on the machine as it is on every Mac.
Nothing here is written for it and nothing needs to be. Agent sessions are pinned to bash 5 through
`env.SHELL`. **Write for bash 5 on both machines**; the Linux box is on 5.2.21.

---

## Timezone and locale

| | |
|---|---|
| Zone | `Australia/Brisbane`, `AEST`, `+1000` |
| DST | **Observes none** |
| `AppleLocale` | `en_AU` |

**The same zone as the Linux box**, so the hazard of two machines in different zones writing local times
into one shared schema is not live today. It is not fixed either: nothing pins a zone, so moving either
machine would make it real without anything failing.

**A double-clicked app gets no `LANG` at all.** It is started by `launchd`, and `launchctl getenv`
answers empty for `LANG`, `LC_ALL`, `LC_CTYPE` and `LC_TIME`. A terminal launch does carry the shell's
`LANG=en_AU.UTF-8`. **So never make an environment variable the primary source of anything**, a path
included: what works from a terminal is absent on a double-click.

Two disciplines follow, and the first is why porting the Swift core to a second platform produced not
one date error:

- **Pin a fixed POSIX locale at every format site** rather than taking the machine's.
- **Zone identifiers are not reliably canonical.** A legacy IANA name such as `Cuba`, a `backward` link
  to `America/Havana`, can be handed through verbatim by a date library without being rewritten. That is
  why the schema seeds `timezone` and reads it through `timezone_lookup` rather than filling it
  get-or-create, and why the alias tables exist.

**That second point was measured against Swift's Foundation and must be re-measured against whatever
date library Rust uses.** The behaviour is the library's, not the operating system's. The schema
decision stands on its own regardless, being the conservative choice either way.

---

## Filesystem

| | |
|---|---|
| Volume | `Macintosh HD`, APFS |
| Case sensitivity | **case-INSENSITIVE** |

**Each machine is silent about the hazard it creates and loud about the other's.** A wrong-case path in
a source file or a script works here and fails on the Linux box. Its mirror image is that two filenames
differing only in case coexist there, get committed, and then cannot be checked out here at all.
Neither compiler nor test run says so on the side that did it. **Never add a name that collides
case-wise with one already in the tree.**

---

## Where things live

| | |
|---|---|
| Repository | `/Users/harryphillips/harry.git/FacetApp` |
| Remote | `https://github.com/tuxcomputers/FacetApp.git` (**HTTPS**, not SSH) |
| Git identity | Harry Phillips `<harry@tux.com.au>` |
| App data directory | `~/Library/Application Support/Facet` |
| Google credentials | `~/.config/facet/google-client.json`, outside every repository |
| Swift reference tree | `~/harry.git/TimeFlipLinux`, a git worktree pinned to `feature/linuxPort`. **Read, do not run** |
| The working Swift app | `~/harry.git/TimeFlipApp`, on `main`. **Built and run as the day-to-day app** |

**The remote is HTTPS deliberately.** `gh auth switch` does not change which SSH key is offered, so an
SSH remote authenticates as the wrong GitHub account for this repo.

### Three folders, three jobs

The Rust app cannot track time yet, so the Swift one stays in service while it is built. That wants the
two checkouts kept apart, because each is useless for the other's purpose.

| Folder | Branch | What it is for |
|---|---|---|
| `FacetApp` | `feature/rustPort` | The Rust rewrite. Where the work happens |
| `TimeFlipApp` | `renameToTimeFlip` | **The working app, called TimeFlip.** `scripts/run.sh` there, and it is the copy actually used to record time |
| `TimeFlipLinux` | `feature/linuxPort` | **The reference.** Read it, search it, do not run it and do not commit in it |

**`main` is the right branch for the working copy** because it is the last released state, rather than
`feature/linuxPort`, which is mid-port and carries a half-built second platform. Measured 2026-09-21:
`swift build` on `main` completes with one warning and no errors.

**The reference is a worktree, not a clone**, so it costs no second copy of the history and cannot drift
from its branch. `TimeFlipApp` had to come off `feature/linuxPort` to give it up, git allowing one
worktree per branch, and `main` is where a working copy wanted to be anyway. Undo the arrangement with
`git worktree remove ../TimeFlipLinux` from `TimeFlipApp`.

**Nothing should be committed in the reference tree.** It is checked out on a real branch, so a commit
there lands on `feature/linuxPort` in a repository that is meant to be frozen.

**The two were separated on 2026-09-21.** The Swift app is TimeFlip, on `au.com.tux.timeflip`, with its
data in `~/Library/Application Support/TimeFlip`; Facet is the Rust one and owns the Facet name, that
identifier and that directory. Nothing is shared but the codesigning identity and the Google project.

**Both still want the menu bar**, so only one runs at a time, and TimeFlip is the one holding real
recorded time.

**No environment variable names the data directory, and none is standard here.** `XDG_DATA_HOME`,
`XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `XDG_CACHE_HOME` are all unset; macOS has no equivalent. The
answer comes from the platform's own application-support lookup, which resolves to
`~/.local/share/Facet` on Linux from the same call.

---

## Bluetooth and the cube

| | |
|---|---|
| Controller | `5C:9B:A6:81:3B:00`, chipset `BCM_4388C2` |
| Cube, as this Mac names it | `FA1DDE60-5DBB-D5E9-B53C-881E16916B5E` |
| Cube name | `TimeFlip v2.0` |
| In the OS's own paired list | **no.** `system_profiler SPBluetoothDataType` lists no TimeFlip at all |

**There is no OS-level bond, and there cannot be one.** The absence is the answer rather than an empty
command: the same listing does hold other devices. The cube runs no pairing agent and the PIN is the
whole of the authentication, which the Linux box reached from the other direction. **So whatever either
machine holds about the cube is rows in its own database and nothing the OS knows.**

**The identifier above is this Mac's name for the cube and is meaningless anywhere else.**
CoreBluetooth hands out a per-host mapping rather than the device's address, and the Linux box sees the
same cube as `E8:DB:D8:CF:F9:0F`. So `device_uuid` is a platform-specific value in a shared table, and
a database moved between the machines carries a pairing only one of them can act on.

### The cube is factory reset before it moves to the other machine

**Stated by the owner as a standing practice, not a measurement.** The reset is done by hand, so the
cube is known to have been reset rather than assumed to have been.

That is what makes the identifier above a non-problem rather than a decision anyone has to take. Three
things follow, all of them behaviour the app needs anyway:

- **A reset gives up the pairing on this side**, so whichever machine has the cube next pairs from
  scratch.
- **The cube is back on the vendor default `000000`**, so a PIN in this machine's keychain names
  something the hardware no longer has. The default is always on the candidate list, so this recovers
  itself.
- **The receiving machine pays for a resync.** A reset restarts the event counter and drops the clock,
  the face colours, the LED and blink settings and the task parameters. All of those are asked for on
  connect regardless, so a handover exercises existing mechanisms rather than needing a new one.

**Unknown**: whether this Mac's per-host identifier for the cube survives a reset. If it does not,
`device_uuid` is stale after every handover on this side too.

---

## Display and UI automation

| | |
|---|---|
| Display | Built-in Liquid Retina XDR, 3024 x 1964, Retina |
| Automation | pyobjc against the accessibility API: `ax-press.py`, `ax-dump.py`, `ax-set.py`, `ax-hold.py`, `ax-key.py`, `ax-alert.py`, `status-item-click.py` |

**The scripted suite drives the real mouse and keyboard on this screen** and needs a person present to
turn the cube. It is never run unattended, and never by an agent.
