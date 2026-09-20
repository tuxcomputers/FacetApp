# The two systems

[← Back to README](../README.md) · [The Rust port →](rust-port.md) · [Port findings →](port-findings.md)

**Measured facts about the two machines Facet is developed on, so neither has to be guessed at.** The Mac
is where the app is built and where the cube normally lives; the Linux box is the second platform and the
only place BlueZ and MATE can be exercised.

**Every line here is either measured, with the date and the machine that measured it, or it is marked
unknown.** Nothing in between, and nothing inferred from what should be the case. A guess written into a
facts file is worse than no line at all, because the next reader cannot tell it from a measurement.

**Carried over from the Swift repository and trimmed.** The Swift toolchain rows, the Swift suite counts
and the Swift CI configuration were dropped: they described a build that no longer exists. Everything left
is a fact about the hardware, the operating system or the desktop, and all of it still holds. The full
original, including the dropped rows, is `docs/systems-info.md` on `feature/linuxPort` in the frozen
TimeFlipApp repository.

**The Mac's Rust toolchain is recorded below. The Linux box's is not**, and goes in the same way once
that machine has one.

---

## System information about the Mac

**All measured 2026-09-07 on the machine below**, by running the command in each row. Anything not
measured says so.

### The machine

| | |
|---|---|
| Model | MacBook Pro, `Mac17,2` |
| Chip | Apple M5, 10 cores (4 performance, 6 efficiency) |
| Memory | 32 GB |
| Architecture | `arm64` |
| Host name | `TM000403-Harry` |

### Operating system

| | |
|---|---|
| macOS | **26.6.2**, build `25G83` |
| Kernel | Darwin 25.6.0, `xnu-12377.161.14~5`, `RELEASE_ARM64_T8142` |
| Swift target triple | `arm64-apple-macosx26.0` |
| Deployment target | `.macOS(.v14)`, from `Package.swift` |

`sw_vers`, `uname -a`.

### Timezone and locale

**Measured 2026-09-07 18:03.**

| | |
|---|---|
| Zone | `Australia/Brisbane`, `AEST`, `+1000` |
| DST | **Observes none.** `+1000` with a zero DST offset in January, April, July and October 2026 |
| `AppleLocale` | `en_AU` |
| `AppleLanguages` | `("en-AU")` |

`date +"%Z %z"`, `readlink /etc/localtime`, `defaults read -g AppleLocale`, and the four offsets read
out of the tz database with `zoneinfo.ZoneInfo('Australia/Brisbane')`.

**The same zone as the Linux box**, so the hazard question 11 and question 1 were both circling is not
live between these two machines today. It is still a hazard rather than a non-issue: nothing in the app
pins a zone, so moving either machine would make it real without anything failing.

**`LANG` is not one answer here, it is three, and none of them is a user setting.** What a launch gets
depends entirely on what started it:

| How the app starts | What it gets |
|---|---|
| Double-clicked, so started by `launchd` | **Nothing.** `launchctl getenv` answers empty for `LANG`, `LC_ALL`, `LC_CTYPE` and `LC_TIME` |
| From a terminal shell, which is what `scripts/run.sh` is | `LANG=en_AU.UTF-8`. Not from a profile: a login `bash -lc` has none, and `LC_TERMINAL=iTerm2` sits beside it in the app's environment |
| From a non-interactive shell, which is what an agent session gets | Nothing, and every `LC_*` is `C` |

The middle row was measured off the running app rather than reasoned about: `ps eww` on the live
`Facet` process showed `LANG=en_AU.UTF-8` with `LC_TERMINAL=iTerm2` beside it, while its parent was
`launchd` (pid 1). That combination is a terminal launch whose shell has since exited, not a Finder
launch, which is why `ps eww` on a running copy says how *that* copy was started and is not the general
answer.

**None of it reaches the app's output, and that is by construction.** Every date this app formats pins
`Locale(identifier: "en_US_POSIX")` at the call site: seven of them, in `DebugLog`,
`DeviceEventRecorder`, `DevicePairingRecorder`, `ReportEntryText`, `ReportCalendar`, `GoogleEventRules`
and `DebugTraceRules`. So no formatted string depends on `LANG`, on `AppleLocale`, or on which of the
three rows above a launch landed in.

**What does reach the data is the zone**, through `TimeZone.current.identifier`, which
`TimezoneStore.currentID` and `DebugLog` each resolve per write and store as a `timezone_id`.

**A `timezone_id` means the same zone in every database on this machine, and until this afternoon it did
not.** Measured 2026-09-07 18:03, before the change: `timezone_id 1` was `Australia/Brisbane` in
`production.sqlite`, carrying 124 `device_event` rows, and **`AEST`** in `test.sqlite`, carrying 3.
`AEST` is not an IANA identifier and the string appears nowhere in the tree, so `TimezoneStore.id(for:)`
had written it from a `TimeZone.current.identifier` that answered an abbreviation. What made it answer
that was never established, and no running app has had `TZ` in its environment since.

**Migrated here 2026-09-07 18:56, onto the seeded table `27411cc` brought in.** The old ids were assigned
in the order this machine happened to visit zones, so every child row had to be remapped by name rather
than kept: `1` became `297` across 124 `device_event` rows, 53 `time_entry` rows on each of its two zone
columns, and 9,734 `debug_log` rows. `production.sqlite` and `debug.sqlite` were migrated in place, each
backed up first into `backup/` and each checked by re-reading the zone **name** behind every row and
requiring it unchanged; `test.sqlite` was rebuilt from the DDL instead, which `database/CLAUDE.md` says
is what test gets. All three now hold 448 zones and 151 aliases with the `timezone_lookup` view,
`PRAGMA foreign_key_check` is clean on each, and no `AEST` row survives anywhere.

**Re-running the DDL would not have done it**, which is worth writing down because it looks like it
should: the seed guards are `WHERE NOT EXISTS (... timezone_id = N)`, so the old row squatting on id 1
makes the seed skip `Africa/Abidjan` and then collide on `UN1_timezone` when it reaches Brisbane's own
id, leaving a half-seeded table and no error anybody would see.

**So a database copied between the two machines carries zone ids that mean the same thing on arrival**,
which is what question 1 was pointing at, the seed being in the shared DDL rather than in either app. A
zone the seed has never heard of is now visible rather than silent, too: `TimezoneStore.id(for:)`
resolves through the view and inserts a miss **above** the seeded block, so an id of 448 or more is the
signal to add that zone to `002_timezone.sql` rather than a low id that means something else everywhere
else.

### What `TimeZone.current.identifier` answers, and Darwin agrees with corelibs

**Measured 2026-09-08 with a standalone Foundation program, Swift 6.3.3.**

| `TZ` | `TimeZone.current.identifier` | In `knownTimeZoneIdentifiers` | Offset used |
|---|---|---|---|
| unset | `Australia/Brisbane` | yes | +10:00 |
| `Cuba` | **`Cuba`** | **no** | -04:00 |
| `AEST` | `Australia/Brisbane` | yes | +10:00 |
| `nonsense/zone` | `Australia/Brisbane` | yes | +10:00 |

**Darwin answers exactly what corelibs answers**, row for row against the Linux table below. A legacy
`backward` link is handed through verbatim and uncanonicalised, an unusable `TZ` falls back silently to
the system zone, and `knownTimeZoneIdentifiers` is the only thing separating the two. So seeding the
table and reading it through `timezone_lookup` is right on both machines for the same measured reason
rather than by coincidence.

**`TZ=AEST` is refused here too**, which rules out the mechanism question 4 proposed. Darwin does not
accept a `TZ` that Linux refuses, so no environment variable put `AEST` in `test.sqlite`: `TZ` is unset
in this shell, nothing under `~/Library/LaunchAgents` sets it, and no script under `Sources/`, `Tests/`
or `scripts/` sets it.

**What does answer that exact string is `TimeZone.current.abbreviation()`**, which returns `AEST` on this
machine where `identifier` returns `Australia/Brisbane`. Nothing in the tree calls it, and `git log -S`
across all branches finds no Swift source that ever did, so whatever wrote that row is not in the
history. The row itself is already gone, the three databases having been migrated on 2026-09-07.

### Rust toolchain

**Measured 2026-09-20 on this machine.**

| | |
|---|---|
| cargo | **1.98.1** (`797e8a9bc`, 2026-08-05) |
| rustc | **1.98.1** (`48a229cea`, 2026-09-01) |
| rustup | **1.29.1** (`d95a37b6a`, 2026-08-13) |
| Toolchains | One: `stable-aarch64-apple-darwin`. No nightly, no second channel |
| Targets installed | One: `aarch64-apple-darwin`. **No cross target for Linux or Windows** |
| Installed by | rustup, into `~/.cargo` and `~/.rustup`. **Not Homebrew** |
| `clippy` | **Not installed.** `cargo clippy` errors with *not installed for the toolchain* |
| `rustfmt` | **Not installed.** Same error from `cargo fmt` |

**`~/.cargo/bin` is not on the PATH a non-interactive shell gets, and no dotfile fixes it.** Checked
`.zshrc`, `.zprofile`, `.bash_profile`, `.bashrc` and `.profile` on 2026-09-20: **none of them mentions
cargo**, and none sources `~/.cargo/env`. So a bare `cargo` fails with *command not found* while the
toolchain is perfectly healthy, which reads like a missing install and is not one.

Anything scripted has to source the env file or use the absolute path:

```sh
. "$HOME/.cargo/env"
~/.cargo/bin/cargo build
```

**An interactive terminal may differ**, if the PATH is set in a terminal profile rather than a dotfile.
Nothing in the dotfiles does it, which is the part that was measured.

**Two consequences worth holding on to.** `clippy` and `rustfmt` both need installing before either can
be a gate, and with one target installed **nothing here can cross-compile**, so the Linux and Windows
crates are built on their own machines rather than from this one.

### Command-line tools

| Tool | Version | Path |
|---|---|---|
| `sqlite3` | 3.51.0 | `/usr/bin/sqlite3` |
| `python3` | 3.14.7 | `/Library/Frameworks/Python.framework/Versions/3.14/bin/python3` (python.org, not the system one) |
| `bash` | **5.3.15** | `/opt/homebrew/bin/bash` (Homebrew). Apple's `/bin/bash` is 3.2 and is not what anything here runs under |
| `git` | 2.50.1 | `/usr/bin/git` |
| `gh` | 2.100.0 | `/opt/homebrew/bin/gh` |
| `jq` | jq-1.7.1-apple | `/usr/bin/jq` |
| `swift-bundler` | **not installed** | -- |
| pyobjc-core | 12.1 | with `objc`, `ApplicationServices`, `Quartz`, `AppKit`, `Foundation` all importable |

### The scripted suite runs under bash 5

`Tests/Scripted/run.sh:144` invokes each check as `bash "$script"`, so the interpreter comes from
`PATH` rather than from the `#!/bin/bash` line in the scripts. In a login shell that is Homebrew's
**5.3.15**, and the full suite is always run through `run.sh`, never a script at a time.

Apple's `/bin/bash` 3.2 is still on the machine, as it is on every Mac, but nothing here is written
for it and nothing needs to be. Agent sessions are pinned to bash 5 as well, through `env.SHELL` in
`~/.claude/settings.json`. **Write for bash 5 on both machines.**

**`swift-bundler` is not installed**, although `Bundler.toml` exists and describes the `.app`
(identifier `au.com.tux.facet`, product `FacetMac`, `LSUIElement = 1`). Day-to-day work is
`swift build` / `swift test` / `scripts/run.sh`, which do not need it.

### Filesystem

| | |
|---|---|
| Volume | `Macintosh HD`, APFS |
| Case sensitivity | **case-INSENSITIVE** (`CaseProbe` and `caseprobe` are the same file) |

**This is a trap in both directions and it is silent on this side.** A wrong-case path or filename
compiles and runs here and fails on Linux, and two files whose names differ only in case cannot coexist
here at all. Anything the Linux box adds with a case-colliding name will appear to this machine as a
merge conflict or a vanishing file rather than as what it is.

### Where things live

| | |
|---|---|
| Repository | `/Users/harryphillips/harry.git/TimeFlipApp` |
| Remote | `https://github.com/tuxcomputers/TimeFlipApp.git` (**HTTPS**, not SSH) |
| Git identity | Harry Phillips `<harry@tux.com.au>` |
| App data directory | `~/Library/Application Support/Facet` |
| Databases | `appdata.sqlite` (a symlink, currently → `test.sqlite`), `production.sqlite`, `test.sqlite`, `debug.sqlite`, plus `prod.sqlite` and a `backup/` directory |
| Schema | `database/` at the repository root is the **real** directory; `Sources/FacetCore/Resources/Database` is a symlink to it |

**The remote is HTTPS deliberately.** `gh auth switch` does not change which SSH key is offered, so an
SSH remote authenticates as the wrong GitHub account for this repo. Whatever the Linux box uses, it
needs to reach `tuxcomputers/TimeFlipApp` as an account with access.

**`appdata.sqlite` is a symlink and which database it points at changes.** `scripts/switch-database.sh`
moves it between `production` and `test`. Never assume which one is live: read the link.

**No environment variable names the data directory here, and none is standard.** Measured 2026-09-07:
`XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `XDG_CACHE_HOME` are all **unset**, and the
only location variables in the environment are `HOME` and `TMPDIR`. macOS has no equivalent of the XDG
variables; the answer comes from `FileManager.urls(for: .applicationSupportDirectory, ...)`, which is
what `DatabaseBootstrap`, `InstanceLock` and `DeveloperConfigFile` all call, and which corelibs already
resolves to `~/.local/share/Facet` on Linux.

The app reads exactly **one** environment variable, `FACET_GOOGLE_CLIENT_JSON`, and reads it as an
*override* with a bundled fallback behind it (`GoogleCredentials.resolve`).

**Untested here, and the reason not to make an environment variable the primary source of a path:** a
`.app` launched from the Finder or the Dock is started by `launchd` and does not inherit variables
exported from a shell profile, so a variable that works under `scripts/run.sh` in a terminal would be
absent on a double-click.

**Half confirmed 2026-09-07**, in *Timezone and locale* above: `launchctl getenv` answers empty for
every locale variable, and launchd's environment is what a double-clicked `.app` starts from, while the
running app was started from a terminal and does carry that shell's `LANG`. What has still not been done
is a Finder launch measured end to end, so this stands as the reason not to depend on a variable rather
than as a settled fact.

### Bluetooth and the cube

| | |
|---|---|
| Controller address | `5C:9B:A6:81:3B:00`, chipset `BCM_4388C2` |
| Cube, as this Mac names it | `FA1DDE60-5DBB-D5E9-B53C-881E16916B5E`, during run 170. The `device_uuid` row is **empty** now |
| Cube name | `TimeFlip v2.0` (`device_name.previous_name` is the same) |
| Paired | **no**, measured 2026-09-07 18:03: `setting.paired` is `{"paired":false}` in both databases |
| In the OS's own paired list | **no.** `system_profiler SPBluetoothDataType` lists no TimeFlip at all |

**Paired here is this app's own row and nothing the OS holds.** Measured 2026-09-07 18:03:
`system_profiler SPBluetoothDataType` lists **no TimeFlip at all**, and its `Not Connected` list does
hold the `MX Master 3S`, so the absence is the answer rather than an empty command. There is no OS bond
because there is nothing to bond with: the cube runs no pairing agent and the PIN is the whole of the
authentication (finding 12), which is the conclusion the Linux box reached from the other direction. So
the Linux box connecting to the cube cannot disturb anything this Mac holds, because this Mac holds
nothing but rows.

**Which settles the second half of what was asked: the `paired` row in a moved database means nothing on
arrival.** It is one machine's record that its own app had a cube, and with the reset practice below it
is `false` by the time the cube travels anyway.

**Both databases say `paired: false` today and `device_uuid` is empty in both**, which is the wipe at the
end of the 2026-09-07 runs and the reset before the cube went to the Linux box. `device_name.previous_name`
is still `TimeFlip v2.0` in `production.sqlite`, kept deliberately so a scan can find the cube again, and
the identifier in the table above is what this Mac called it during run 170 rather than something stored
anywhere now.

**The identifier above is this Mac's name for the cube and is meaningless anywhere else.** CoreBluetooth
hands out a per-host mapping, not the device's address, so the `device_uuid` row in a database copied to
the Linux box names nothing there. The Linux box sees the same cube as `E8:DB:D8:CF:F9:0F`
(`linux-port.md`, frozen repo), a real address. Already written up in
[timeflip2-firmware-observations.md](timeflip2-firmware-observations.md) finding at line 285, and in
`DeviceScanRules.swift:10`.

**So `device_uuid` is a platform-specific value living in a shared table**, and a database moved between
the two machines carries a pairing that only one of them can act on. Nothing has been decided about the
column, and the handover practice below is what removes the need to decide anything about it for now.

### The cube is factory reset before it is used on the other machine

**Stated by the owner 2026-09-07 as a standing practice, not a measurement.** The reset is done **by
hand**: the cube is connected deliberately and reset, so that it is known to have been reset rather
than assumed to have been. `Tests/Scripted/99-quit.sh` also wipes it at the end of a full scripted run
and fails the run if the cube cannot prove it was erased, but a handover does not lean on a run having
ended that way.

**So the pairing rows in a moved database describe a cube that is no longer in that state**, which is
what makes `device_uuid` above a non-problem rather than any decision about the column. Three things
follow, and all three are behaviour this app already has:

- **A reset gives the pairing up on this side.** The app forgets the device along with the wipe, which
  is what `99-quit` asserts (`paired` back to `0`), so whichever machine has the cube next pairs from
  scratch rather than inheriting anything.
- **The cube is back on the vendor default `000000`** (`DeviceLoginRules.defaultPIN`), so the PIN in
  this Mac's login Keychain names something the hardware no longer has. `DevicePINStore`'s own doc
  comment already describes that as the honest outcome of a per-machine Keychain, and the default is on
  the presented list either way (`DeviceLoginRules.candidates`).
- **The receiving machine pays for a resync**, because a reset restarts the event counter and drops the
  clock, the face colours, the LED and blink settings and the task parameters. `DeviceSystemStateRules`
  asks the cube about all of those on connect, so nothing about a handover needs a mechanism that does
  not already exist.

**Unknown here, and it is the mirror of what the Linux box flagged about its random address**: whether
this Mac's per-host CoreBluetooth identifier for the cube survives a reset has not been tested. If it
does not, `device_uuid` is stale after every handover on this side too, not only on the other.

### Display and UI automation

| | |
|---|---|
| Display | Built-in Liquid Retina XDR, 3024 x 1964, Retina |
| Automation | pyobjc against the accessibility API: `scripts/ax-press.py`, `ax-dump.py`, `ax-set.py`, `ax-hold.py`, `ax-key.py`, `ax-alert.py`, `scripts/status-item-click.py` |

The scripted suite drives the **real** mouse and keyboard on this screen and needs a person present to
turn the cube. It is never run unattended, and never by an agent.


---

## System information about the Linux

**All measured 2026-09-07 on the machine below**, by running the command in each row. Anything not
measured says so.

### The machine

| | |
|---|---|
| Model | **A MacBook Pro running Linux**, `MacBookPro14,2` |
| Chip | Intel Core i7-7567U @ 3.50GHz, 2 cores / 4 threads |
| Memory | 15 GiB |
| Architecture | `x86_64` |
| Host name | `harry-MacBookPro` |

`hostnamectl`, `lscpu`, `free -h`.

**The two machines differ by instruction set as well as by operating system**: `arm64` there, `x86_64`
here. Nothing in the port has depended on it so far -- both are little-endian, which is what the BLE
frame parsing cares about -- but no built artefact is interchangeable between them, and a timing figure
taken on the Mac is not one about this box.

### Operating system

| | |
|---|---|
| Distribution | **Linux Mint 22.3 "Zena"**, Ubuntu 24.04 `noble` base |
| Kernel | 7.0.0-31-generic, `#31~24.04.1-Ubuntu SMP PREEMPT_DYNAMIC`, built 2026-08-10 |
| Desktop | **MATE 1.26.1** |
| Display server | **X11**, `XDG_SESSION_TYPE=x11` |
| Swift target triple | `x86_64-unknown-linux-gnu` |

`/etc/os-release`, `uname -a`, `mate-session --version`, `echo $XDG_SESSION_TYPE`.

**X11, which is what the scripted suite's replacement needed to hear.** Synthetic mouse and keyboard
events -- the half of `scripts/ax-press.py` and `scripts/status-item-click.py` that is not reading the
tree -- go through XTEST here. Under Wayland there is no equivalent a normal process may call, so the
AT-SPI plan in item 12 is possible in the shape planned rather than needing a compositor-specific route.

**`linux-port.md` (frozen repo, `feature/linuxPort`) said MATE 1.26.2 and that was wrong**: `mate-session` reports 1.26.1.
Corrected there in the same change.

### Command-line tools

| Tool | Version | Path |
|---|---|---|
| `sqlite3` | 3.45.1 | `/usr/bin/sqlite3` (**installed 2026-09-07**; the Mac is on 3.51.0) |
| `libsqlite3-0` | 3.45.1-1ubuntu2.7 | `/usr/lib/x86_64-linux-gnu/libsqlite3.so.0` |
| `libsqlite3-dev` | 3.45.1-1ubuntu2.7 | **installed 2026-09-07**, giving `/usr/include/sqlite3.h` and the unversioned `libsqlite3.so` |
| `python3` | 3.12.3 | `/usr/bin/python3` (the system one) |
| `python3-dbus` | 1.3.2 | importable as `dbus` |
| `awk` | **GNU Awk 5.2.1** | `/usr/bin/awk` -> `/usr/bin/gawk`, **installed 2026-09-09**, taking the alternative from mawk 1.3.4 which is still at `/usr/bin/mawk`. Here because the `ubuntu-latest` runner reports the same version string, so the stamp gate can be tested on the dialect CI uses; which dialects mis-split and why is in `check_interactive_checklists.sh` beside the `FS` |
| `podman` | 4.9.3 | `/usr/bin/podman` (**installed 2026-09-09**), rootless with the `overlay` driver and `subuid`/`subgid` entries for the owner. What lets `scripts/ci-local.sh` run CI's Linux job in the image it names rather than approximating it natively |
| `python3-gi` | 3.48.2 | importable as `gi` |
| `python3-pyatspi` | 2.46.1 | importable as `pyatspi` |
| `bash` | **5.2.21(1)** | `/usr/bin/bash`, and `SHELL=/bin/bash` |
| `git` | 2.43.0 | `/usr/bin/git` |
| `gh` | 2.45.0 | `/usr/bin/gh` (**installed 2026-09-07**; the Mac is on 2.100.0). Logged in as `tuxcomputers` |
| `jq` | 1.7.1 | `/usr/bin/jq` (**installed 2026-09-07**; reports itself as `jq-1.7`) |
| `secret-tool` | 0.21.4 | `/usr/bin/secret-tool`, from `libsecret-tools` (**installed 2026-09-07**) |
| `curl` | 8.5.0 | `/usr/bin/curl` |
| `make` | GNU Make 4.3 | `/usr/bin/make` |
| `wmctrl` | present | -- |
| `xdotool` | **not installed** | -- |
| `xwd` | present | `/usr/bin/xwd`. The only stock screen-grabber here, and it writes `.xwd` with nothing on the box to convert it -- so a screenshot is taken through `gi` instead (`Tests/Methods.md` Method 20) |
| ImageMagick | **only `imagemagick-6-common`** | The infrastructure package alone: there is no `convert` and no `import` binary (measured 2026-09-16) |
| `Xvfb` / `xvfb-run` | **not installed** | **Measured 2026-09-16**, and it is the thing standing between a window check and running without the owner's screen -- item 12 of the frozen repo's `docs/linux-port.md` calls that the open question worth the most. One `apt install xvfb` |
| `flatpak` | present | -- |
| `snap` | **not installed** | -- |
| pyobjc | **absent and staying absent** | `import objc` fails; the seven `scripts/ax-*.py` and `status-item-click.py` are Mac-only |

**bash is 5.2.21, so the bash-5 target the Mac set is met** and nothing here has to be written down to
bash 3. Unlike the Mac there is only one bash on the machine, so which one a script gets is not a
question here.

**`/bin/sh` is `dash`.** A `#!/bin/sh` script carrying a bash-ism runs on the Mac, whose `/bin/sh` is
bash in POSIX mode, and fails here. Nothing checked in has a `#!/bin/sh` line today; this is the reason
to keep it that way.

**Four tools this repository calls were missing until 2026-09-07 and are installed now**, all from
Ubuntu's own archive with no third-party repository: `sqlite3` and `libsqlite3-dev`, which six
checked-in files and the `FacetCore` build need between them; `jq`, wanted by `scripts/ci-local.sh`
alone; and `gh`, wanted by nothing checked in and only by the remote workflow in `CLAUDE.md`.

**Two version gaps between the machines, neither of them currently biting.** The `sqlite3` CLI is
3.45.1 here against **3.51.0** on the Mac, and `gh` is 2.45.0 against **2.100.0** -- Ubuntu 24.04 ships
what it ships. The SQLite gap is the one that could matter, since the schema is shared, so it was
measured rather than assumed: see *The schema applies under this box's SQLite* below.


### Filesystem

| | |
|---|---|
| Volume | `/dev/nvme0n1p2`, **ext4**, 916 GB, 5% used. `/` and `$HOME` are the same filesystem |
| Case sensitivity | **case-SENSITIVE**, measured on both the repository volume and `/tmp` |

`df -T .`, and a `CaseProbe`/`caseprobe` pair in both places.

**Each machine is silent about the hazard it creates and loud about the other's.** A wrong-case path
in a source file or a script works on the Mac and fails here, which is the Mac's half of it. This box's
half is the mirror image: two filenames differing only in case coexist here perfectly, get committed
without complaint, and then cannot be checked out on the Mac at all. Neither compiler nor test run
will say so on the side that did it, so the rule is the same rule from both ends -- never add a name
that collides case-wise with one already in the tree.

### Where things live

| | |
|---|---|
| Repository | `/home/harry/harry.git/TimeFlipApp` (**moved from `/home/harry/git/TimeFlipApp`**, which this table said until 2026-09-16; a stale `.build` module cache naming the old path is what noticed, and `rm -rf .build/*/debug/ModuleCache` is the fix) |
| Remote | `https://github.com/tuxcomputers/TimeFlipApp.git` (**HTTPS**, matching the Mac) |
| Git identity | Harry Phillips `<harry@tux.com.au>`, the same identity as the Mac |
| App data directory | `/home/harry/.local/share/Facet`, and it **exists** as of 2026-09-13 |
| Databases | `appdata.sqlite` and `debug.sqlite`, and **`appdata.sqlite` is a plain file rather than a symlink** |
| Schema | `database/` at the root is the real directory; `Sources/FacetCore/Resources/Database` is a symlink to `../../../database` and resolves correctly here |

**There is no production database on this box, and no test one** (owner, 2026-09-20). The Mac keeps
`appdata.sqlite` as a *symlink* pointing at either `production.sqlite` or `test.sqlite`, and
`scripts/switch-database.sh` is what moves it; this host has never had that split. What is here is one plain
`appdata.sqlite` holding the real data -- the paired cube, the Google account, the recorded time.

**It is what stops `Tests/Scripted/run.sh` starting here**, which is worth knowing before reading item 12 as
merely unfinished. The switcher refuses a plain file outright, deliberately:

    error: appdata.sqlite exists but is not a symlink -- refusing to touch it.

and its advice (*launch the app once with Developer Mode on so it can migrate this*) describes something that
does not exist in either target: nothing in `FacetCore`, `FacetMac` or `FacetLinux` migrates a plain file into
`production.sqlite` plus a link. On the Mac the split predates the check and was made by hand.

**So making this host runnable is `mv appdata.sqlite production.sqlite` and a symlink beside it**, which is what
the app would have done, and it is the one thing standing between this box and a scripted run.

**No XDG variable is set**, measured 2026-09-07: `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME`
and `XDG_CACHE_HOME` are all unset. So corelibs' `applicationSupportDirectory` falls back to
`$HOME/.local/share`, which is where `/home/harry/.local/share/Facet` above comes from, and the
platform-aware answer in to-do item 3 needs no variable to be set to be right.

**This box can push, as of 2026-09-07**, and it took two commands after `gh` was installed:

```sh
gh auth login --hostname github.com --git-protocol https --web   # the tuxcomputers account
gh auth setup-git                                                # writes the helper into ~/.gitconfig
```

| | |
|---|---|
| Account | `tuxcomputers`, active, matching the org this repo belongs to |
| Token scopes | `gist`, `read:org`, `repo`, **`workflow`** -- `repo` carries the push, `workflow` is needed for any commit that touches `.github/workflows/**` |
| Where the token lives | **the login keyring**, service `gh:github.com`, account `tuxcomputers` |
| Credential helper | `credential.https://github.com.helper=!/usr/bin/gh auth git-credential`, in `~/.gitconfig` |
| Verified by | pushing two commits and reading them back with `gh api repos/tuxcomputers/TimeFlipApp/commits/<sha>` |

**`workflow` was added on 2026-09-09, and it is a one-off per machine rather than something to keep doing.**
A push touching `.github/workflows/tests.yml` was refused -- *refusing to allow an OAuth App to create or
update workflow ... without `workflow` scope* -- because the 2026-09-07 login above never asked for it.
Cleared with one command, which re-mints the token in the same keyring entry:

```sh
gh auth refresh -h github.com -s workflow      # needs a person at a browser; device-code flow
```

**Why the Mac never needed it, since the two boxes use the same account and it looks like it should behave
the same.** Scopes belong to the *token*, and every `gh auth login` mints its own -- so the Mac's
`tuxcomputers` token and this one are different credentials that happen to name the same user.
`gh auth switch` picks a different token; it does not add a scope to one. The Mac's was minted carrying
`workflow`, which is why switching profiles there simply works, and the history shows it: commits touching
`.github/workflows/` landed on 2026-08-16 and 2026-09-07, both before this box had any credential at all.

GitHub keeps `workflow` out of `repo` deliberately: a workflow file is code that runs with the repository's
secrets, so a token that can push source is not automatically allowed to push something that could read them.
Expect to see it again here only if the token is revoked, the keyring is reset, or somebody runs a fresh
`gh auth login`, which re-mints and can drop it.

**It had none of that until then**, and the failure is worth knowing by sight because it names neither
gh nor a permission: `git push` answers `fatal: could not read Username for 'https://github.com': No
such device or address`. That is git finding no helper and no terminal to prompt at, not a rejected
credential -- a `403` would be the other thing, which is what `CLAUDE.md` describes for an account
without access.

**No SSH key is involved and none is wanted here**, matching the Mac's reasoning: the remote is HTTPS on
purpose, so authentication follows the active `gh` account rather than whichever key the agent offers.

### Timezone and locale

**Measured 2026-09-07.** This should have been written down when question 11 was deleted and was not:
the answer went into the question asked back to the Mac instead of into this section, which is the one
place it belongs. Corrected here.

| | |
|---|---|
| Zone | `Australia/Brisbane`, `AEST`, `+1000` |
| DST | **Observes none** |
| `LANG` | `en_AU.UTF-8` |
| `LANGUAGE` | `en_AU:en` |
| `LC_*` | All inherited from `LANG`; none set individually |
| Clock | `systemd-timesyncd` active, synchronised |

`timedatectl`, `locale`.

**The same zone as the Mac**, which measured `Australia/Brisbane` for itself on the same day. So the
hazard both question 11 and the Mac's question 1 were circling -- two machines in different zones
writing local times into one shared schema -- is not live between these two today, and is not fixed
either. Nothing in the app pins a zone; moving either machine would make it real with nothing failing.

### What `TimeZone.current.identifier` answers, and it is not always canonical

**Measured 2026-09-07 with a standalone Foundation program, Swift 6.2:**

| `TZ` | `TimeZone.current.identifier` | In `knownTimeZoneIdentifiers` | Offset used |
|---|---|---|---|
| unset | `Australia/Brisbane` | yes | +10:00 |
| `Australia/Brisbane` | `Australia/Brisbane` | yes | +10:00 |
| `Cuba` | **`Cuba`** | **no** | -04:00 |
| `US/Pacific` | **`US/Pacific`** | **no** | -07:00 |
| `AEST` | `Australia/Brisbane` | yes | +10:00 |
| `nonsense/zone` | `Australia/Brisbane` | yes | +10:00 |

**A legacy IANA name is handed through verbatim and nothing canonicalises it.** `Cuba` is a `backward`
link to `America/Havana` and Foundation neither rewrites it nor rejects it -- it resolves the zone
correctly and reports the name it was given. `knownTimeZoneIdentifiers` is the only discriminator
available: a name it does not contain is an alias. **This is why `timezone` is seeded and read through
`timezone_lookup`** rather than filled by get-or-create, and it is a measurement rather than a worry.

**An unusable `TZ` falls back silently to the system zone**, which is worth knowing because it means a
wrong `TZ` in a test harness produces plausible times rather than an error.

**`TZ=AEST` is rejected here**, and that bears on something the Mac could not explain: its
`test.sqlite` holds `AEST` as a `timezone_name`, written at runtime from an identifier that answered an
abbreviation. Whatever produced it, corelibs on this box is not it. Asked as question 4 below.

### There is a keychain here, and the Secret Service is already running

| | |
|---|---|
| Daemon | **`gnome-keyring-daemon` 46.1-2ubuntu0.2, running** with `--components=pkcs11,secrets` |
| D-Bus name | `org.freedesktop.secrets`, present on the session bus |
| Collections | `/org/freedesktop/secrets/collection/login` and `.../session` |
| Keyring files | `~/.local/share/keyrings/login.keyring`, plus `user.keystore` |
| `libsecret-1-0` | 0.21.4-1build3, `/usr/lib/x86_64-linux-gnu/libsecret-1.so.0` |
| `libdbus-1-dev` | 1.14.10-4ubuntu4.1, **installed 2026-09-07**, and what `CDBus` is built against. `pkg-config --cflags dbus-1` gives the two include directories libdbus needs |
| `libsecret-1-dev` | 0.21.4-1build3, **installed 2026-09-07**. `pkg-config --modversion libsecret-1` answers 0.21.4 and the header is at `/usr/include/libsecret-1/libsecret/secret.h` |
| `libsecret-tools` (`secret-tool`) | 0.21.4-1build3, **installed 2026-09-07** |
| `seahorse` | 43.0-3build2, for looking inside it by hand |

`pgrep -a gnome-keyring`, `dbus-send --session --dest=org.freedesktop.secrets ... Get Collections`,
`dpkg -l`, `pkg-config --exists libsecret-1`.

**Item 7 already named libsecret, so the counterpart was never in doubt. What is new is that it is
live.** The daemon is running on this box right now, with a `login` collection in it, and it is the same
keyring VSCode asks the password for on its first launch of a session -- unlocked per session rather
than per application. That is worth writing down because it makes item 7 a port onto a running service
rather than a piece of work that has to stand something up first.

**So item 7 has two routes, and the cheaper one needs nothing installed.**

| Route | What it costs | What it gets |
|---|---|---|
| Link **libsecret** | a modulemap over its C API -- the same shape as the `SQLite3` wall, and now the same *remaining* work, `libsecret-1-dev` being installed | The API the desktop expects, with prompting and unlocking handled |
| Talk to **`org.freedesktop.secrets`** over D-Bus | nothing to install: the daemon is up, the name is claimed, and item 10 is bringing a D-Bus layer for BlueZ anyway | The same store, over the bus the port is already going to speak |

The second is worth weighing precisely because of item 10. `org.freedesktop.secrets` and
`org.bluez` sit on different buses -- session and system respectively -- so they are not one connection,
but they are one set of D-Bus mechanics, and `scripts/linux-ble-probe.py` already demonstrates all of it
against BlueZ. **Nothing is decided here**; this records that the choice exists and that neither route
is blocked.

**The keyring is already holding a real credential for this repository**, which is the strongest thing
that can be said for the D-Bus route short of building it: `gh auth login` put its token there rather
than in a file, `secret-tool search --all service gh:github.com` reads it back, and `git push` has been
driven from it. So the store works, unprompted, for a background process on this desktop.

**Untested, and it matters before either route is built**: what happens when the keyring is *locked*.
`DevicePINStore` and `GoogleTokenStore` both answer `unavailable(Int32)` for a Keychain that will not
answer, which is the case that already exists on the Mac -- but the failure here arrives as a prompt to
the user, or as a D-Bus error if there is nobody to prompt, and which one a background app gets has not
been measured.

### Bluetooth and the cube

| | |
|---|---|
| Adapter | `hci0`, `88:E9:FE:5F:1B:52`, name `harry-MacBookPro` |
| Adapter provenance | **built in**, on `dw-apb-uart` rather than USB, manufacturer `0x000f` (Broadcom). Not a dongle |
| BlueZ | **5.72** (`bluetoothctl --version`) |
| Cube, as this box names it | `E8:DB:D8:CF:F9:0F`, address type **random** |
| Cube name | `TimeFlip v2.0` |
| Paired / Bonded / Trusted | **no / no / no**, and that is correct here |
| Where the cube is | **at this machine**, 2026-09-07 17:05 |

`bluetoothctl show`, `bluetoothctl --timeout 15 scan on`, `bluetoothctl info E8:DB:D8:CF:F9:0F`,
`readlink -f /sys/class/bluetooth/hci0`.

**The address is unchanged from the 2026-09-06 run** and the cube answered a 15-second scan on
2026-09-07 at 17:05, so it is physically here now rather than at the Mac. Whether that address survives
a battery change or a `0xFF` factory reset is **untested**, and there is a reason not to assume it does:
BlueZ reports it as a **random** address rather than a public one, and a random address is the kind the
specification allows a device to change. So it is no more a durable identifier than the Mac's per-host
UUID is -- neither machine's name for this cube can be written into a shared table and trusted on the
other.

**BlueZ had forgotten the cube across this boot.** `bluetoothctl info` answered `not available` until a
scan rediscovered it -- there is no bond to persist, the PIN being the whole of the authentication -- so
on this platform finding the cube is *always* a scan. That is the same lesson the device rename cost on
the Mac (2026-08-01), arrived at from the other direction.

**The cube is factory reset every time it moves between the two machines.** Stated by the owner
2026-09-07 as a standing practice, not a measurement, and it is what makes the PIN a non-question: a
reset cube is on the vendor default `000000`, which is the only PIN this box knows and the default
`scripts/linux-ble-probe.py` uses. `DevicePINStore`'s own doc comment already describes this as the
honest outcome of a per-machine Keychain -- a cube carried to a second machine is met by an app that
knows only the vendor default, and the recovery the vendor gave it is taking the batteries out.

**So each handover costs the receiving machine a resync, and that is by design rather than by accident.**
A reset restarts the cube's event counter, which is why the history cursor is read from `device_event`
and checked against what the cube can reach rather than being a stored number (7.1 of
`state-audit.md` (frozen repo, `feature/linuxPort`)), and it drops the face colours, the LED and blink settings, the task
parameters and the clock -- all of which `DeviceSystemStateRules` already asks the cube about on
connect. Nothing about a handover needs a new mechanism; it exercises the ones a factory reset already
has.

**One advertising detail not previously written down**, and it is a way to spot the cube that does not
depend on its name: it carries manufacturer data under key `0xffff` whose value is
`54 2e 46 6c 69 70 00`, ASCII `T.Flip`. It still advertises **no service UUID**, which is finding 12 and
why discovery must not be filtered on one.

### Display and UI automation

| | |
|---|---|
| Display | `eDP-1`, 2560x1600 at 60Hz, 286x179mm, the only one |
| Automation stack | AT-SPI: `at-spi2-core` 2.52.0, `libatk-adaptor` 2.52.0, `python3-pyatspi` 2.46.1 |
| Registry | **running**: `at-spi-bus-launcher` and `at-spi2-registryd` are both up |
| `toolkit-accessibility` | **false** |
| Present | `wmctrl` |
| Absent | `xdotool`, `accerciser` |

`xrandr --current`, `gsettings get org.gnome.desktop.interface toolkit-accessibility`, `pgrep -a at-spi`.

**The accessibility bus is running but toolkit accessibility is switched off**, so a GTK app started
today would expose no tree to read. Turning it on is one `gsettings set`, and it is a precondition for
to-do item 12 rather than a piece of work in it.

### The schema applies under this box's SQLite

**Measured 2026-09-07**, now that there is a `sqlite3` to do it with, and it matters because the two
machines are two minor versions apart (3.45.1 here, 3.51.0 on the Mac) over a schema they share.

All **13** files in `database/` applied to a fresh database in file order, each with
`PRAGMA foreign_keys = ON`, exactly as `scripts/compare-database-to-ddl.sh` does it:

| | |
|---|---|
| Files applied | 13 of 13, **no errors** |
| Tables created | 13 |
| `PRAGMA integrity_check` | `ok` |
| `PRAGMA foreign_key_check` | no violations |
| Seeds | `setting` 19, `icon` 43, `colour` 21, `event_type` 8, `timezone` 1 |
| Time | 1.32s |

**So nothing in the DDL needs a SQLite newer than 3.45.1**, and the 1.32s is the same cost
`linux-port.md` (frozen repo, `feature/linuxPort`) already measured at ~1.15s for a bootstrap -- which is why each test
bootstrapping its own database costs what it costs, on this platform as on the other.

---

