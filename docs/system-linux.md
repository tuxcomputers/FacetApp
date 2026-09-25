# The Linux box

[← Back to README](../README.md) · [The Mac →](system-mac.md) · [What is being asked of this machine →](handover-linux.md) · [BlueZ notes →](linux-bluez-port-notes.md) · [Port findings →](port-findings.md)

**What this machine is, and everything needed to build and drive Facet on it.** This is the second
platform, and the only place BlueZ and MATE can be exercised.

**Every line is either measured, with the date, or it is marked unknown.** Nothing in between, and
nothing inferred from what should be the case. A guess written into a facts file is worse than no line
at all, because the next reader cannot tell it from a measurement.

---

## The machine

| | |
|---|---|
| Model | **A MacBook Pro running Linux**, `MacBookPro14,2` |
| Chip | Intel Core i7-7567U @ 3.50GHz, 2 cores / 4 threads |
| Memory | 15 GiB |
| Architecture | `x86_64` |
| Distribution | **Linux Mint 22.3 "Zena"**, Ubuntu 24.04 `noble` base |
| Kernel | 7.0.0-31-generic, built 2026-08-10 |
| Desktop | **MATE 1.26.1** |
| Display server | **X11**, `XDG_SESSION_TYPE=x11` |

`hostnamectl`, `lscpu`, `/etc/os-release`, `mate-session --version`. Measured 2026-09-07, re-checked
2026-09-20.

**The two machines differ by instruction set as well as by operating system**: `arm64` there, `x86_64`
here. Both are little-endian, which is what the BLE frame parsing cares about, but **no built artefact
is interchangeable** and a timing figure taken on the Mac is not one about this box. This box is the
slower of the two by a wide margin, and the build timings below are the ones to plan CI around.

**X11 matters for the scripted suite.** Synthetic mouse and keyboard events go through XTEST here.
Under Wayland there is no equivalent a normal process may call, so the AT-SPI approach works in the
shape planned rather than needing a compositor-specific route.

**MATE 1.26.1**, not 1.26.2 as an earlier document claimed. Which MATE applet is running matters for
tray click behaviour, and [rust-port.md](rust-port.md) explains why: this box has
`mate-indicator-applet` **1.26.0** installed, which is the applet with the open left/right click bug,
rather than the Notification Area applet whose bugs are closed.

---

## Building Facet here

**Everything in this section was measured on 2026-09-20 by running it**, from a tree with no `target/`
directory at all, so every figure below is a cold build. **The whole workspace and both probes compile
with exactly what is described here and nothing else installed.** Nothing had to be added.

This supersedes the previous revision of this file, which said no Rust toolchain was installed. One
is, and it is the same version as the Mac's.

### Rust

| | |
|---|---|
| cargo | **1.98.1** (`797e8a9bc`, 2026-08-05) |
| rustc | **1.98.1** (`48a229cea`, 2026-09-01) |
| rustup | **1.29.1** (`d95a37b6a`, 2026-08-13) |
| Toolchains | One: `stable-x86_64-unknown-linux-gnu`. No nightly |
| Targets | One: `x86_64-unknown-linux-gnu`. **Nothing here cross-compiles** |
| Installed by | rustup, into `~/.cargo` and `~/.rustup` |
| `clippy` | **Installed**, 0.1.98. `cargo clippy` exits 0 on the crates this box builds |
| `rustfmt` | **Installed**, 1.9.0-stable (`48a229ceae`). **`cargo fmt --check` exits 1**, and see below |

**The toolchain versions match the Mac exactly**, which is worth stating rather than assuming: both are
on cargo and rustc 1.98.1 with the same commit hashes, so a compiler-version difference is not
available as an explanation when the two machines disagree about something.

**Both machines now have `clippy` and `rustfmt`, at the same versions.** That was not true when this
file was first written: neither was on the Mac, which is why no commit from there had ever been
format-checked. **The Mac installed both on 2026-09-23**, so the toolchains no longer differ at all and
either box can run either gate. A `rust-toolchain.toml` pinning 1.98.1 with both as components is
planned, so they cannot drift apart again.

**The PATH behaves the opposite way to the Mac's, and both need knowing.** Here `~/.cargo/env` *is*
sourced, by both `.bashrc` (line 118) and `.profile` (line 28), so an ordinary shell finds `cargo`
without being told. **A shell with no environment at all still does not**: `env -i bash -c 'cargo'`
fails, because the bare default PATH has no `~/.cargo/bin`. So a systemd unit, a cron job or anything
else starting from an empty environment needs the same line the Mac needs everywhere:

```sh
. "$HOME/.cargo/env"          # or: export PATH="$HOME/.cargo/bin:$PATH"
```

### The C toolchain, and what actually needs it

| | |
|---|---|
| `cc` / `gcc` | **13.3.0** (`Ubuntu 13.3.0-6ubuntu2~24.04.1`) |
| `g++` | 13.3.0 |
| `clang` | **not installed**, and nothing wants it |
| `build-essential` | **12.10ubuntu1, installed** |
| `make` | GNU Make 4.3 |
| `pkg-config` | 1.8.1 |

**A C compiler is a hard requirement, and it is `rusqlite` that needs it**, not Rust. The `bundled`
feature compiles SQLite's own C source into the binary, so `libsqlite3-sys` invokes `cc` on every clean
build. That is also what makes the shipped binary self-contained: **nothing links the system SQLite and
no SQLite needs installing on a user's machine.** Confirmed here by `libsqlite3-sys` 0.30.1 compiling
in the cold build below, and by the built artefacts in `target/debug/deps`.

**`libsqlite3-dev` was installed for the Swift build and is confirmed unnecessary now.** Swift shipped
no SQLite module for Linux, so a modulemap had to name the real header. `rusqlite` bundles its own copy
instead, and the build above never consults the system headers. Left installed; noted so nobody assumes
it is load-bearing.

### What the UI needs, and what it does not

**No cmake, no ninja, and neither is installed** — the same answer as the Mac, and it was worth
measuring separately because this is a different Slint backend. C++ Skia is **never compiled**: it is an
unenabled optional dependency. What actually builds on this box is

```
i-slint-backend-winit        the Linux backend
i-slint-renderer-femtovg     OpenGL, through glutin
i-slint-renderer-software    tiny-skia, which is pure Rust and not C++ Skia
```

verified by `cargo tree -e normal` and by the crates present in the probe's `target/debug/deps`. The
only `skia` in the graph is `tiny-skia` 0.11.4 / 0.12.0, which is the pure-Rust one.

**So enabling Slint's Skia renderer later is not a feature flag, it is a new build dependency**, on
this machine as much as on the Mac. It would want cmake and ninja, neither of which is here.

**The previous revision guessed this was the most likely place a system package would turn out to be
wanted. It was wrong: no package had to be added.** The Slint probe links only libraries already
present on an ordinary desktop:

```
libfontconfig.so.1  libfreetype.so.6  libpng16.so.16  libexpat.so.1
libbrotlidec.so.1   libbrotlicommon.so.1  libbz2.so.1.0  libz.so.1  libc/libm/libgcc_s
```

**Note that `libfontconfig1-dev` is *not* installed and the build did not ask for it**, only the
runtime `libfontconfig1`. That is a measurement, not a recommendation: a machine built from scratch may
still want the dev package, and nobody has tested one.

### The two unverified crate questions, both now answered

Both were open when this file was first written, and each was settled by building the crate rather than
by reading about it. **They came out opposite ways**, which is the reason neither was guessable:

- **`btleplug` does need `libdbus-1-dev`.** It is not pure Rust on this path. The probe binary links
  `/lib/x86_64-linux-gnu/libdbus-1.so.3`, by way of `dbus` 0.9.12 → `dbus-tokio` 0.7.6 →
  `bluez-generated` 0.4.0 → `bluez-async` 0.8.2 → `btleplug` 0.13.1. **So `libdbus-1-dev` is
  load-bearing and must go in any build instructions.**
- **`keyring` does *not* need `libsecret-1-dev`**, which is the opposite of what this file assumed.
  Built and run 2026-09-22 by `probe/keyring-secret-service`: the binary links **`libc` and `libgcc_s`
  and nothing else**. `keyring`'s default `v1` feature selects `zbus-secret-service-keyring-store` on
  Linux, reaching the Secret Service over zbus in pure Rust through `secret-service` 5.2.0. The
  `dbus-secret-service-keyring-store` that *would* use the C library is not enabled.

**One needed the system library and the other never did**, and no amount of reading either crate's
description would have said which. Slint's AccessKit bridge reaches AT-SPI the same pure-Rust way, so
of the three D-Bus users in this app only `btleplug` links a C library. **`libsecret-1-dev` stays
installed and is not load-bearing**; `libdbus-1-dev` is.

### The commands, and what they cost cold

```sh
cargo build                   # facet-core alone, by default-members
cargo test                    # the hermetic suite
cargo build -p facet-linux    # the native binary
```

**A bare build is the core only**, because the other three crates are each buildable on exactly one
platform and this machine cannot compile the CoreBluetooth or WinRT adapters.

| Command | Cold | Notes |
|---|---|---|
| `cargo build` | **15.93s** | 19 rlibs, `libsqlite3-sys` 0.30.1 and `rusqlite` 0.32.1 among them |
| `cargo build -p facet-linux` | 5.11s | On top of the above, once the crate had a tray in it |
| `cargo test` | 30s | 19 tests, the slow one being `facet-ui` compiling Slint |
| `cargo clippy` | 5.34s | Exit 0 |
| `cargo fmt --check` | instant | **Exit 1, and see below** |

Measured 2026-09-20 and re-measured 2026-09-22, the workspace having gained `facet-ui` and a Linux tray
in between. The 15.93s is still a true cold figure; the rest are what they cost from a warm `target/`.

**`cargo fmt --check` exits 1, and that is a reformat waiting to happen rather than a standard nobody
agreed.** There **is** a `rustfmt.toml` now, adopted 2026-09-25, and it is written to the style the tree
already has rather than against it:

```toml
max_width = 110
use_small_heuristics = "Max"
```

**`Max` is the load-bearing line.** It gives every width heuristic the full 110, so
`Showing { paused: false, locked: false }` stays on one line. Under rustfmt's defaults
`struct_lit_width` is 18 and every compact literal in the tree would be exploded, which is why running
`cargo fmt` was the wrong answer while there was no config.

**Measured here 2026-09-25 with rustfmt 1.9.0-stable (`48a229ceae`): 26 files-worth of diffs**, against
55 under the defaults. The Mac reported 27 on the same commit. **The difference is entirely
`facet-mac/src/main.rs`**, 8 here against 9 there, that file being uncommitted work on the Mac when it
measured; **the other 18 agree exactly**, which is the answer to whether the two machines format alike.
They do.

| File | Diffs |
|---|---|
| `crates/facet-mac/src/main.rs` | 8 |
| `crates/facet-linux/src/main.rs` | 6 |
| `crates/facet-ui/src/status_icon.rs` | 4 |
| `crates/facet-core/src/database.rs` | 4 |
| `crates/facet-ui/examples/draw-settings-tabs.rs` | 3 |
| `crates/facet-core/src/setting.rs` | 1 |

**Most of the 26 re-join lines an earlier default-width format had split**, so the config is pulling the
tree back towards its own style rather than fighting it. The one real departure is the `Error` enum in
`facet-core/src/database.rs`, whose struct variants get exploded one field per line whatever the
heuristics say.

**Do not run `cargo fmt` here.** The reformat lands as one commit containing nothing else, from the Mac.

**`cargo clippy` is the gate that is already green**: exit 0 across `facet-core`, `facet-ui` and
`facet-linux` with all targets, carrying two warnings, both `manual Range::contains` in
`facet-ui/src/status_icon.rs`. Those are in code moved verbatim from `facet-mac` and were left alone so
the move stayed reviewable as a move. **CI gates on neither yet.**

**The probes are excluded from the workspace** and resolve their own dependencies:

```sh
(cd probe/timeflip-btleplug && cargo build)     # running it needs the cube
(cd probe/slint-editable-table && cargo build)  # running it opens a window
```

| Probe | Cold build | `target/` |
|---|---|---|
| `timeflip-btleplug` | **29.98s** | 319 MB |
| `slint-editable-table` | **4m 53s** | **3.1 GB** |

Both built 2026-09-20 on btleplug 0.13.1 and slint 1.18.0, matching the Mac. **Only built, not run**:
one needs the cube and the other opens a window on the owner's screen.

**The Slint probe's 4m 53s and 3.1 GB are the figures to plan around.** That is a cold build of the UI
dependency graph on two cores, and it is roughly twenty times the core's. A CI job on a machine like
this one should expect to cache `target/` rather than rebuild Slint per run.

---

## Command-line tools

| Tool | Version | Path | What needs it |
|---|---|---|---|
| `bash` | **5.2.21** | `/usr/bin/bash`, and `SHELL=/bin/bash` | Everything. Only one bash on this machine |
| `python3` | 3.12.3 | `/usr/bin/python3` (the system one) | The `at-*.py` drivers |
| `python3-gi` | 3.48.2 | importable as `gi` | Screenshots, and the GTK bits of the drivers |
| `python3-pyatspi` | 2.46.1 | importable as `pyatspi` | The accessibility tree |
| `python3-dbus` | 1.3.2 | importable as `dbus` | `tray-menu.py`, `linux-ble-probe.py` |
| `sqlite3` | 3.45.1 | `/usr/bin/sqlite3` | The database scripts. The Mac is on 3.51.0. **Not the app**, which bundles its own |
| `secret-tool` | 0.21.4 | `/usr/bin/secret-tool` | Reading the keyring by hand |
| `git` | 2.43.0 | `/usr/bin/git` | |
| `gh` | 2.45.0 | `/usr/bin/gh` | Logged in as `tuxcomputers` |
| `jq` | 1.7.1 packaged, **reports `jq-1.7`** | `/usr/bin/jq` | |
| `curl` | 8.5.0 | `/usr/bin/curl` | |
| `make` | 4.3 | `/usr/bin/make` | |
| `gcc` / `cc` | 13.3.0 | `/usr/bin/cc` | `rusqlite`'s bundled SQLite |
| `wmctrl` | 1.07 | `/usr/bin/wmctrl` | |
| `cmake`, `ninja` | **not installed** | | Only if Slint's Skia renderer is ever enabled |
| `xdotool` | **not installed** | | Nothing checked in calls it |
| `Xvfb` / `xvfb-run` | **not installed** | | **The thing standing between a window check and running without the owner's screen.** One `apt install xvfb` |
| ImageMagick | **only `imagemagick-6-common`** | | No `convert`, no `import`. A screenshot goes through `gi` instead |
| pyobjc | **absent and staying absent** | | The `ax-*.py` scripts are Mac-only by design |

**`jq` is the packaged 1.7.1 but answers `jq-1.7` to `--version`.** Both are recorded because a script
that greps the version string will see the second, and an earlier revision of this file recorded only
the first.

**`/bin/sh` is `dash`.** A `#!/bin/sh` script carrying a bash-ism runs on the Mac, whose `/bin/sh` is
bash in POSIX mode, and fails here. Nothing checked in has a `#!/bin/sh` line, and this is the reason to
keep it that way.

**Write for bash 5 on both machines.** This box is on 5.2.21 and the Mac's Homebrew bash is 5.3.15;
the Mac also still carries Apple's 3.2.57, which nothing here is written for.

**The `sqlite3` version gap no longer reaches the app.** It is 3.45.1 here against 3.51.0 on the Mac,
which mattered when the app linked the system library. With `rusqlite`'s bundled SQLite the app carries
one version everywhere and the gap affects only the CLI the scripts use. The schema itself was measured
against 3.45.1 and needs nothing newer: see the last section.

---

## Filesystem

| | |
|---|---|
| Volume | `/dev/nvme0n1p2`, **ext4**, 916 GB, 819 GB free. `/` and `$HOME` are the same filesystem |
| Case sensitivity | **case-SENSITIVE**, on both the repository volume and `/tmp` |

The mirror of the Mac's hazard. Two filenames differing only in case coexist here perfectly, get
committed without complaint, and then cannot be checked out on the Mac at all. **Never add a name that
collides case-wise with one already in the tree.**

**The build artefacts are not small on this box**: 106 MB for the workspace `target/` plus 3.4 GB
across the two probe trees. All three are gitignored. Worth knowing before assuming a clean checkout
costs nothing to build.

---

## Where things live

| | |
|---|---|
| Repository | `/home/harry/harry.git/FacetApp` (the Swift one is at `.../TimeFlipApp`) |
| Remote | `https://github.com/tuxcomputers/FacetApp.git` (**HTTPS**, matching the Mac) |
| Git identity | Harry Phillips `<harry@tux.com.au>`, the same identity as the Mac |
| App data directory | `/home/harry/.local/share/Facet` |
| Databases | `production.sqlite` and `test.sqlite`, with `appdata.sqlite` a symlink to whichever is live |
| Google credentials | `~/.config/facet/google-client.json`, outside every repository |

**No XDG variable is set.** `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `XDG_CACHE_HOME`
are all unset, so the XDG default of `$HOME/.local/share` is what produces the path above. The
platform-aware answer needs no variable to be set to be right. The Mac reaches
`~/Library/Application Support/Facet` from the same call.

**The Google credentials live at the same path as the Mac's**, `~/.config/facet/`, which on that
machine is a non-standard location and on this one is the XDG default. `scripted-seed.json` sits beside
it. Both are mode 600 and neither is in any repository.

**This box can push as `tuxcomputers`**, with the token in the login keyring rather than a file and
`gh auth git-credential` as the credential helper. **The `workflow` scope is per token and per
machine**: a push touching `.github/workflows/**` is refused without it, and `gh auth switch` picks a
different token rather than adding a scope to one. Fixed with `gh auth refresh -h github.com -s
workflow`, which needs a person at a browser. Expect it again only if the token is revoked or somebody
runs a fresh `gh auth login`.

**The production/test split has been made here** (2026-09-20), so the previous revision's note that a
plain `appdata.sqlite` stops the scripted suite starting no longer applies. The directory now holds
`production.sqlite`, `test.sqlite` and a `singleinstance.lock`, with `appdata.sqlite` symlinked to
`test.sqlite` as of this measurement. `scripts/switch-database.sh` moves the symlink and is the only
thing that should; `setting.db_type` is how a launch reports which one it landed on.

---

## Timezone and locale

| | |
|---|---|
| Zone | `Australia/Brisbane`, `AEST`, `+1000` |
| DST | **Observes none** |
| `LANG` | `en_AU.UTF-8`, with `LANGUAGE=en_AU:en` and every `LC_*` inherited from it |
| Clock | `systemd-timesyncd` active, synchronised |

**The same zone as the Mac**, so the two-zones hazard is not live between these machines today and is
not fixed either: nothing pins a zone, so moving either machine would make it real without anything
failing.

**This box has a `LANG` where a double-clicked Mac app has none.** That asymmetry is itself the reason
for the discipline: **never make an environment variable the primary source of anything**, because what
is reliably present here is reliably absent there. Pin a fixed POSIX locale at every format site rather
than taking the machine's.

**Zone identifiers are not reliably canonical**, which is why the schema seeds `timezone` and reads it
through `timezone_lookup`. Measured here against Swift's Foundation: a legacy IANA name such as `Cuba`,
a `backward` link to `America/Havana`, came back verbatim and uncanonicalised, and an unusable `TZ` fell
back silently to the system zone rather than erroring. **Both behaviours are the date library's, not
the operating system's, so both must be re-measured against whatever Rust uses.** Nothing in the
workspace parses a date yet, so that re-measurement has not happened. The schema decision is the
conservative one either way.

---

## The Secret Service is already running

| | |
|---|---|
| Daemon | **`gnome-keyring-daemon` 46.1, running** with `--components=pkcs11,secrets` |
| D-Bus name | `org.freedesktop.secrets`, present on the session bus |
| Collections | `login` and `session` |
| `libsecret-1-0` / `-dev` | 0.21.4 |
| `seahorse` | 43.0, for looking inside it by hand |

**This is a port onto a running service rather than work that has to stand something up.** The daemon is
up, the name is claimed, and the keyring is unlocked per session rather than per application, which is
the password VSCode asks for on its first launch.

**It already holds a real credential for this project**, which is the strongest evidence short of
building against it: `gh auth login` put its token there rather than in a file, `secret-tool` reads it
back, and `git push` has been driven from it. So the store works, unprompted, for a background process
on this desktop.

**Now built against, and it works.** `probe/keyring-secret-service` ran the full round trip on
2026-09-22: a password written, read back and compared; five bytes including a non-UTF-8 pair
round-tripped; the credential deleted; and a read afterwards answering **`NoEntry`** rather than an
error. **That last part is what the app depends on**, needing to tell *no PIN has ever been stored*
from *the store is broken*, because the first is an ordinary first run.

**It reaches the service in pure Rust and links no system library**, so `libsecret-1-dev` is not
load-bearing for it. Keeping the package costs nothing; believing it was required would have.

**The Mac half is still unmeasured.** The same `v1` default selects `apple-native-keyring-store/keychain`
there, target-gated, so the Keychain *should* come from the same dependency line. That is
[handover-mac.md](handover-mac.md) item 3.

**Measured 2026-09-22, and the answer is neither of the two this file expected.** A locked keyring does
not prompt-and-continue and does not return an error: **the read blocks, for as long as it is given**.
`probe/keyring-secret-service` was killed at a 25 second cap having returned nothing, exit 124, with a
GNOME **Unlock Login Keyring** dialog on screen. **The dialog outlived the process that raised it.**

| | |
|---|---|
| Locked read | blocks indefinitely, no error |
| What appears | an Unlock Login Keyring dialog |
| When the caller dies | the dialog stays up, orphaned |
| After unlocking | the secret reads back intact, the cycle costing nothing |

**So a background Facet on a locked keyring hangs rather than degrades**, and on a machine with no
prompter it hangs with nothing on screen to say why. Any read of a stored PIN needs **its own timeout and
its own fallback**, and must not sit on the launch path. See
[port-findings.md](port-findings.md); the constraint belongs to the secret store port rather than to this
machine.

**Done with the owner present, and it is not a thing to repeat casually**: locking `login` takes the `gh`
token with it and puts a password dialog in front of whoever is at the screen.

---

## Bluetooth and the cube

| | |
|---|---|
| Adapter | `hci0`, `88:E9:FE:5F:1B:52`, named `harry-MacBookPro` |
| Adapter provenance | **built in**, on `dw-apb-uart` rather than USB, Broadcom. Not a dongle |
| BlueZ | **5.72** |
| Cube, as this box names it | `E8:DB:D8:CF:F9:0F`, address type **random** |
| Cube name | `TimeFlip v2.0`, the same string the Mac sees |
| As the app's own identifier | `FACE7000-0000-0000-0000-E8DBD8CFF90F`, derived from the address |
| Paired / Bonded / Trusted | **no / no / no**, and that is correct here |

Adapter and BlueZ measured 2026-09-07; the cube rows re-confirmed 2026-09-20 with `bluetoothctl info`
against the cube in range.

**The address is random rather than public, so durability could not be assumed — but it has now been
measured.** `E8:DB:D8:CF:F9:0F` was the same before and after a factory reset
([linux-bluez-port-notes.md](linux-bluez-port-notes.md), 2026-09-07), which this file previously
recorded as untested. **Whether it survives a battery change is still untested**, and the specification
still allows a random address to change, so the finding is one data point and not a guarantee.

**It changes nothing about the shared schema.** Neither machine's name for this cube can be written into
a shared table and trusted on the other — the Mac sees `FA1DDE60-5DBB-D5E9-B53C-881E16916B5E` for the
same hardware — so `device_uuid` remains a platform-specific value in a shared table, and a database
moved between the machines carries a pairing only one of them can act on.

**BlueZ forgets the cube across a boot.** `bluetoothctl info` answers `not available` until a scan
rediscovers it, there being no bond to persist. **So on this platform, finding the cube is always a
scan**, the same lesson the device rename cost on the Mac, arrived at from the other direction.

**There is no OS-level bond and there cannot be one**, which the Mac reached from its own side: it lists
no TimeFlip in `system_profiler` either. The cube runs no pairing agent and the PIN is the whole of the
authentication. **So whatever either machine holds about the cube is rows in its own database and
nothing the OS knows.**

**One advertising detail worth having**: manufacturer data under key `0xffff` with value
`54 2e 46 6c 69 70 00`, ASCII `T.Flip`. It still advertises **no service UUID**, which is finding 12 and
why discovery must not be filtered on one. Note that `0xffff` is the Bluetooth SIG value reserved for
testing rather than an assigned vendor id, so it is not an authoritative stamp.

**The cube is factory reset before it moves between the machines**, stated by the owner as a standing
practice and recorded in [system-mac.md](system-mac.md), which has what follows from it. The part that
matters on this side: a reset puts the cube back on the vendor default PIN `000000`, restarts the event
counter and drops the clock, face colours, LED, blink and task settings. All of those are asked for on
connect regardless, so a handover exercises existing mechanisms rather than needing a new one.

---

## Display and UI automation

| | |
|---|---|
| Display | `eDP-1`, 2560x1600 at 60Hz, 286mm x 179mm, the only one |
| Automation stack | AT-SPI: `at-spi2-core` 2.52.0, `libatk-adaptor` 2.52.0, `python3-pyatspi` 2.46.1 |
| Registry | **running**: `at-spi-bus-launcher`, `at-spi2-registryd` and the AT-SPI `dbus-daemon` all up |
| `toolkit-accessibility` | **false** |
| Drivers | `at-press.py`, `at-dump.py`, `at-set.py`, `at-hold.py`, `at-key.py`, `at-alert.py`, `at-clipboard.py`, `atspi_tree.py`, `tray-menu.py`, `linux-ble-probe.py` |

**These are the Linux counterparts of the Mac's `ax-*.py` scripts**, name for name, plus
`at-clipboard.py`, `atspi_tree.py` and `tray-menu.py` which have no Mac equivalent. Both sets live in
`scripts/`. The Mac's `status-item-click.py` is the tray driver there; `tray-menu.py` is the one here,
and it goes through D-Bus because **tray items are addressed by label on Linux, no identifier surviving
the trip**.

**The accessibility bus is running but toolkit accessibility is switched off**, and **for Slint that is
the gate**. Turning it on is one `gsettings set`. A GTK3 app loaded the bridge anyway with this setting
false, measured 2026-09-08, which is why this file used to say the setting is not the gate it looks
like. **That holds for GTK and not for Slint**, measured 2026-09-22 with the real app on screen: with
the setting false Facet was not on the bus at all, and setting it true put the whole window there
immediately with no restart. See [port-findings.md](port-findings.md).

| | |
|---|---|
| `org.a11y.Status` `IsEnabled` | **false** while `toolkit-accessibility` is false |
| `ScreenReaderEnabled` | false |
| What turns it on | `gsettings set org.gnome.desktop.interface toolkit-accessibility true` |

**So anything driving this app through AT-SPI has to set that first and put it back after.** A run that
forgets finds no application, and every check then fails exactly as it would against a window that never
opened.

**Slint ships its own AT-SPI bridge, and it is in the graph.** Measured 2026-09-20 from the Slint
probe's build: `accesskit` 0.24.1, `accesskit_unix` 0.22.1, `accesskit_atspi_common` 0.19.1 and
`atspi` 0.29.0 all compile as part of `i-slint-backend-winit`. **So a Slint window does not depend on
`libatk-adaptor` the way a GTK app does**: it speaks AT-SPI over zbus in pure Rust, linking no system
D-Bus library, which is why the probe binary links no `libdbus` while the btleplug one does. AccessKit's
Unix adapter stays dormant until an assistive technology is active, which is the mechanism behind the
gate above rather than a separate fact.

**A Slint window does appear on the bus, and can be driven.** `scripts/at-press.py --app facet-linux
About` pressed a real tab and the app recorded `Settings tab selected: About`, so the path is proven end
to end. **`at-press.py --tab` does not work against it**: Slint presents `page tab` children directly
under the frame where GTK presents a `page tab list`, so the notebook lookup finds nothing. Press tabs by
name.

---

## What hosts a tray icon here

**Measured 2026-09-22.** This decides where a `ksni` status item actually lands, and the answer is not
either of the two applets the question is usually between.

| | |
|---|---|
| Panel | `mate-panel`, with `notification-area` and `xapp-status` both on it |
| `org.kde.StatusNotifierWatcher` | **owned by `xapp-sn-watcher`**, from `libxapp1`, at `/usr/lib/x86_64-linux-gnu/xapps/` |
| What displays the item | `mate-xapp-status-applet` |
| `org.mate.panel enable-sni-support` | **false** |
| `mate-indicator-applet` | **installed, and not on the panel** |
| Already registered | `blueman` and `indicator_solaar`, so the path is carrying real items |

**A StatusNotifierItem has exactly one place to go on this box**, and the app does not choose it: `ksni`
publishes on the session bus and the host is whoever owns `org.kde.StatusNotifierWatcher`. That is
`xapp-sn-watcher` and nothing else, one process being able to own a bus name.

**The Notification Area applet can speak SNI and here it is not**, `enable-sni-support` reading false, so
it is the XEmbed half only and an SNI item cannot reach it. **The Indicator Applet is installed but has
never been on this panel**: absent from `/org/mate/panel/general/object-id-list`, absent from every object
under it, and `dconf` holds no stale key naming it. **Installed and running are different questions**, and
an earlier note in [rust-port.md](rust-port.md) confused them and concluded this box was running the
applet with the open click bug. It is not, and never was.

**What that costs in reach**: a click result from here is a result about the XApp watcher, which Linux
Mint ships and plain MATE does not. It generalises to Mint, not to every MATE install.

**Confirmed with the app itself on 2026-09-22.** Facet registered as
`org.kde.StatusNotifierItem-<pid>-1/StatusNotifierItem`, appeared in the watcher's
`RegisteredStatusNotifierItems` beside blueman and solaar, and left it again when the app quit. The item
published `Title=Facet`, `Status=Active`, `Category=ApplicationStatus`, its menu at `/MenuBar`, and an
`IconPixmap` of 32x32 that became **70x32** when the padlock was added, so this host takes a non-square
icon.

---

## The schema applies under this box's SQLite

**Measured 2026-09-07.** Every DDL file applied to a fresh database in file order, each with
`PRAGMA foreign_keys = ON`:

| | |
|---|---|
| Errors | **none** |
| `PRAGMA integrity_check` | `ok` |
| `PRAGMA foreign_key_check` | no violations |
| Time | 1.32s |

**So nothing in the DDL needs a SQLite newer than 3.45.1.** That mattered more when the app used the
system library; with the bundled build it is now a statement about the scripts rather than about the
app. The 1.32s is why a test that bootstraps its own database costs what it does on this box.

**The app's own SQLite is `libsqlite3-sys` 0.30.1's bundled copy**, whatever version that vendors, and
it is the same on both machines. The 3.45.1 above is the CLI the scripts call.
