# The Linux box

[← Back to README](../README.md) · [The Mac →](system-mac.md) · [BlueZ notes →](linux-bluez-port-notes.md) · [Port findings →](port-findings.md)

**What this machine is, and what is known about building and driving Facet on it.** This is the second
platform, and the only place BlueZ and MATE can be exercised.

**Every line is either measured, with the date, or it is marked unknown.** Nothing in between.

**The Rust half is not measured yet**, because this box has no Rust toolchain. The section on building
says what is known and what has to be checked, and is explicit about which is which.

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

`hostnamectl`, `lscpu`, `/etc/os-release`, `mate-session --version`. Measured 2026-09-07.

**The two machines differ by instruction set as well as by operating system**: `arm64` there, `x86_64`
here. Both are little-endian, which is what the BLE frame parsing cares about, but **no built artefact
is interchangeable** and a timing figure taken on the Mac is not one about this box.

**X11 matters for the scripted suite.** Synthetic mouse and keyboard events go through XTEST here.
Under Wayland there is no equivalent a normal process may call, so the AT-SPI approach works in the
shape planned rather than needing a compositor-specific route.

**MATE 1.26.1**, not 1.26.2 as an earlier document claimed. Which MATE applet is running matters for
tray click behaviour, and [rust-port.md](rust-port.md) explains why: this box has
`mate-indicator-applet` installed, which is the applet with the open left/right click bug, rather than
the Notification Area applet whose bugs are closed.

---

## Building Facet here

**Not measured. No Rust toolchain is installed on this box as of 2026-09-20**, so none of what follows
is a build that has been run.

What can be said from what is installed and from the dependency choices:

| | |
|---|---|
| Rust | **Absent.** Install through rustup, as on the Mac |
| A C compiler | Needed, because `rusqlite`'s `bundled` feature compiles SQLite's C source. `make` 4.3 is present; whether a full `build-essential` is has not been checked |
| System SQLite | **Not needed by the app.** `bundled` means the binary carries its own. The `sqlite3` CLI is still wanted by the scripts |
| Bluetooth | `btleplug` talks to BlueZ over D-Bus. `libdbus-1-dev` is installed. Whether the crate needs it, or speaks the bus in pure Rust, is **unverified** |
| Secrets | `keyring` targets the Secret Service, which is live here (below). `libsecret-1-dev` is installed. Again **unverified** against the crate |
| Slint | On the Mac the default renderer is femtovg over OpenGL plus a pure-Rust software renderer, needing no cmake. **What the Linux backend pulls in has not been checked**, and it is the most likely place a system package turns out to be wanted |

**The honest summary is that the Linux build is a thing to try, not a thing to plan around.** The first
person to run `cargo build` here should record what it asked for, in this section.

**`libsqlite3-dev` was installed for the Swift build and is probably now unnecessary.** Swift shipped no
SQLite module for Linux, so a modulemap had to name the real header. `rusqlite` bundles its own copy
instead. Left installed; noted so nobody assumes it is load-bearing.

---

## Command-line tools

| Tool | Version | Path | What needs it |
|---|---|---|---|
| `bash` | **5.2.21** | `/usr/bin/bash`, and `SHELL=/bin/bash` | Everything. Only one bash on this machine |
| `python3` | 3.12.3 | `/usr/bin/python3` (the system one) | The `at-*.py` drivers |
| `python3-gi` | 3.48.2 | importable as `gi` | Screenshots, and the GTK bits of the drivers |
| `python3-pyatspi` | 2.46.1 | importable as `pyatspi` | The accessibility tree |
| `python3-dbus` | 1.3.2 | importable as `dbus` | `tray-menu.py`, `linux-ble-probe.py` |
| `sqlite3` | 3.45.1 | `/usr/bin/sqlite3` | The database scripts. The Mac is on 3.51.0 |
| `secret-tool` | 0.21.4 | `/usr/bin/secret-tool` | Reading the keyring by hand |
| `git` | 2.43.0 | `/usr/bin/git` | |
| `gh` | 2.45.0 | `/usr/bin/gh` | Logged in as `tuxcomputers` |
| `jq` | 1.7.1 | `/usr/bin/jq` | |
| `curl` | 8.5.0 | `/usr/bin/curl` | |
| `make` | 4.3 | `/usr/bin/make` | |
| `wmctrl` | present | | |
| `xdotool` | **not installed** | | |
| `Xvfb` / `xvfb-run` | **not installed** | | **The thing standing between a window check and running without the owner's screen.** One `apt install xvfb` |
| ImageMagick | **only `imagemagick-6-common`** | | No `convert`, no `import`. A screenshot goes through `gi` instead |
| pyobjc | **absent and staying absent** | | The `ax-*.py` scripts are Mac-only by design |

**`/bin/sh` is `dash`.** A `#!/bin/sh` script carrying a bash-ism runs on the Mac, whose `/bin/sh` is
bash in POSIX mode, and fails here. Nothing checked in has a `#!/bin/sh` line, and this is the reason to
keep it that way.

**The `sqlite3` version gap no longer reaches the app.** It is 3.45.1 here against 3.51.0 on the Mac,
which mattered when the app linked the system library. With `rusqlite`'s bundled SQLite the app carries
one version everywhere and the gap affects only the CLI the scripts use. The schema itself was measured
against 3.45.1 and needs nothing newer: see the last section.

---

## Filesystem

| | |
|---|---|
| Volume | `/dev/nvme0n1p2`, **ext4**, 916 GB. `/` and `$HOME` are the same filesystem |
| Case sensitivity | **case-SENSITIVE**, on both the repository volume and `/tmp` |

The mirror of the Mac's hazard. Two filenames differing only in case coexist here perfectly, get
committed without complaint, and then cannot be checked out on the Mac at all. **Never add a name that
collides case-wise with one already in the tree.**

---

## Where things live

| | |
|---|---|
| Repository | `/home/harry/harry.git/FacetApp` (the Swift one is at `.../TimeFlipApp`) |
| Remote | HTTPS, matching the Mac |
| Git identity | Harry Phillips `<harry@tux.com.au>`, the same identity as the Mac |
| App data directory | `/home/harry/.local/share/Facet` |
| Databases | One plain `appdata.sqlite`, plus `debug.sqlite` |

**No XDG variable is set.** `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `XDG_CACHE_HOME`
are all unset, so the XDG default of `$HOME/.local/share` is what produces the path above. The
platform-aware answer needs no variable to be set to be right.

**This box can push as `tuxcomputers`**, with the token in the login keyring rather than a file and
`gh auth git-credential` as the credential helper. **The `workflow` scope is per token and per
machine**: a push touching `.github/workflows/**` is refused without it, and `gh auth switch` picks a
different token rather than adding a scope to one. Fixed with `gh auth refresh -h github.com -s
workflow`, which needs a person at a browser. Expect it again only if the token is revoked or somebody
runs a fresh `gh auth login`.

**`appdata.sqlite` here is a plain file, and that stops the scripted suite starting.**
`scripts/switch-database.sh` refuses a plain file outright, deliberately, and its advice describes a
migration the app does not perform. On the Mac the production/test split predates the check and was
made by hand. **Making this host runnable is `mv appdata.sqlite production.sqlite` and a symlink beside
it.**

---

## Timezone and locale

| | |
|---|---|
| Zone | `Australia/Brisbane`, `AEST`, `+1000` |
| DST | **Observes none** |
| `LANG` | `en_AU.UTF-8`, with `LANGUAGE=en_AU:en` and every `LC_*` inherited from it |
| Clock | `systemd-timesyncd` active, synchronised |

**The same zone as the Mac**, so the two-zones hazard is not live between these machines today and is
not fixed either.

**Zone identifiers are not reliably canonical**, which is why the schema seeds `timezone` and reads it
through `timezone_lookup`. Measured here against Swift's Foundation: a legacy IANA name such as `Cuba`,
a `backward` link to `America/Havana`, came back verbatim and uncanonicalised, and an unusable `TZ` fell
back silently to the system zone rather than erroring. **Both behaviours are the date library's, not
the operating system's, so both must be re-measured against whatever Rust uses.** The schema decision
is the conservative one either way.

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

**Untested, and it matters**: what happens when the keyring is **locked**. The failure arrives as a
prompt to the user, or as a D-Bus error if there is nobody to prompt, and which one a background app
gets has not been measured.

---

## Bluetooth and the cube

| | |
|---|---|
| Adapter | `hci0`, `88:E9:FE:5F:1B:52` |
| Adapter provenance | **built in**, on `dw-apb-uart` rather than USB, Broadcom. Not a dongle |
| BlueZ | **5.72** |
| Cube, as this box names it | `E8:DB:D8:CF:F9:0F`, address type **random** |
| Paired / Bonded / Trusted | **no / no / no**, and that is correct here |

**The address is random, not public, so it is no more durable than the Mac's per-host UUID.** The
specification allows a device to change a random address, and whether this one survives a battery change
or a factory reset is **untested**. Neither machine's name for this cube can be written into a shared
table and trusted on the other.

**BlueZ forgets the cube across a boot.** `bluetoothctl info` answers `not available` until a scan
rediscovers it, there being no bond to persist. **So on this platform, finding the cube is always a
scan**, the same lesson the device rename cost on the Mac, arrived at from the other direction.

**One advertising detail worth having**: manufacturer data under key `0xffff` with value
`54 2e 46 6c 69 70 00`, ASCII `T.Flip`. It still advertises **no service UUID**, which is finding 12 and
why discovery must not be filtered on one. Note that `0xffff` is the Bluetooth SIG value reserved for
testing rather than an assigned vendor id, so it is not an authoritative stamp.

---

## Display and UI automation

| | |
|---|---|
| Display | `eDP-1`, 2560x1600 at 60Hz, the only one |
| Automation stack | AT-SPI: `at-spi2-core` 2.52.0, `libatk-adaptor` 2.52.0, `python3-pyatspi` 2.46.1 |
| Registry | **running**: `at-spi-bus-launcher` and `at-spi2-registryd` both up |
| `toolkit-accessibility` | **false** |

**The accessibility bus is running but toolkit accessibility is switched off.** Turning it on is one
`gsettings set`. Note that a GTK3 app loaded the bridge anyway with this setting false, measured
2026-09-08, so the setting is not the gate it looks like; see [port-findings.md](port-findings.md).
Whether a Slint window needs it is unknown.

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
