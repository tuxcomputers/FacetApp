# Architecture: a platform-blind core and a port for every capability

[← Back to README](../README.md) · [The Rust port →](rust-port.md) · [Port findings →](port-findings.md) · [Behaviour inventory →](behaviour-inventory.md)

**The picture is [`architecture-model.svg`](architecture-model.svg)**, and it is the owner's own figure
rather than an illustration added afterwards. The core is a circle whose modules link freely to one
another. Each platform capability leaves the circle along one arm, and each arm ends at a square holding
one slot per platform, of which the build selects one. **What travels an arm is the same whichever slot
was built.**

This model was not designed for Rust. It was designed in Swift, built, and then measured against a real
second platform. [Port findings](port-findings.md) records what that measurement said. The short version
is the reason it carries over unchanged: **filling five Linux slots in a day cost the core zero lines.**

---

## The crates

```
Cargo.toml                  workspace root
crates/
  facet-core/               no platform crate may appear in its dependencies
    src/
    tests/
    resources/database/     the DDL, compiled in
  facet-mac/                composition root + adapters for macOS
  facet-linux/              composition root + adapters for Linux (MATE)
  facet-windows/            composition root + adapters for Windows
Tests/Scripted/             bash, drives a running app against a real cube
probe/                      throwaway programs that answered a question, kept because they can be re-run
scripts/                    the accessibility drivers and the database tooling
```

`facet-core` is a library. The three platform crates are binaries, and each one is a **composition root**:
the only place in the program that knows both a port and the thing that performs it.

---

## The rule

**`facet-core` holds no adapter and chooses no adapter.** It states what it needs as a trait, and
something outside it hands over the thing that does it. The core must not know which platform it is
running on, and it must not be able to find out.

That is the whole rule. The rest of this file is what it means in practice and how it is checked.

**Every capability the platform provides is a *port*: a trait in the core, named for what it does rather
than for what performs it.** A store of secrets, not a Keychain. A radio, not CoreBluetooth. A menu bar,
not `NSStatusItem`. The name is load-bearing, because a trait called `KeychainStore` has already decided
the answer and the second adapter arrives reading like a lie.

**An adapter lives in a platform crate and `main.rs` injects it.** A core type that selects its own
implementation, however small the `cfg`, is the core caring what platform it is on.

**Modules of the same name behave identically on every platform.** A caller written against a port reads
the same everywhere, and a difference between platforms is a difference between adapters and nowhere
else. That is what makes the port worth having: not that the implementation *can* be swapped, but that
nothing above it has to be read twice to find out whether it was.

### What is a port, and what is merely a portability shim

Conflating these makes the rule unusable.

- **An adapter** performs a capability by talking to something only this platform has. It belongs in a
  platform crate.
- **A portability shim** is the same code reaching the same standard library through a different
  spelling, which in Rust is usually a `cfg` around a path or a signal. Noise rather than architecture,
  and it may stay in the core. It is not a licence to put a *decision* behind one.

The test between them: **would a third platform need a different implementation, or merely a different
spelling?** Different implementation is a port. Different spelling is a shim.

### Rust makes one half of this cheaper and one half harder

**Cheaper: the dependency graph enforces it.** In Swift the rule needed a test that read the sources
looking for platform conditionals, because nothing stopped a core file importing AppKit. In Rust,
`btleplug`, `tray-icon`, `keyring` and `slint` are *dependencies*, and a dependency that is not in
`crates/facet-core/Cargo.toml` cannot be reached from the core at all. The check is the manifest, and it
is read by cargo rather than by a test somebody has to keep honest.

**Harder: the Swift version caught what the manifest cannot.** The measured hole in the Swift check was
never the conditionals. It was the ten core files reaching a platform capability with **no** conditional
at all, through `flock`, `errno`, `FileManager.default` and `Bundle.main`. The Rust equivalents live in
`std` and carry no dependency edge, so cargo will not see them either. `std::fs::File::lock`, the process
lock, the config directory and anything reading an executable's own path are the same class of problem
under a different name.

**So the check is two-part**, and the second part is the one that needs writing:

1. **Dependency discipline**, free: `crates/facet-core/Cargo.toml` names no platform crate, and a
   `cargo deny`-style rule or a test over the manifest asserts it.
2. **A banned-spellings scan** over `crates/facet-core/src`, seeded with the `std` items that are a
   platform capability wearing a portable signature. `flock` was the sharpest one in Swift and its
   successor is the same idea. The list is widened one symbol at a time, because deciding a symbol
   belongs on it is a judgement rather than a pattern.

**A green run means "nothing new has declared itself"**, not "the core is platform-blind". The judgement
is still a person's.

---

## The arms

Eight were drawn. Two were taken off as not being platform capabilities at all, one was discovered late,
and the rest are real. This is the state the Swift port left them in, which is the starting specification
rather than a history.

| Arm | Port | macOS | Linux (MATE) | Windows |
|---|---|---|---|---|
| Radio | scan, connect, discover, read, write, subscribe | CoreBluetooth | BlueZ over D-Bus | WinRT |
| Menu bar | a label and a list of items, both as closures | `NSStatusItem` | StatusNotifierItem | `Shell_NotifyIcon` |
| The clock | scheduled wake-ups | `RunLoop` | GLib main loop | |
| Secrets | store and fetch one item | Keychain | Secret Service | Credential Manager |
| Dialogues | ask a question, carry which button is the way out | AppKit alert | GTK dialogue | |
| Starting and stopping | end the app | app delegate | a menu item | |
| Opening a URL | hand the desktop a URL or a file | `NSWorkspace` | `xdg-open` | shell execute |
| Single instance | refuse a second copy | `flock` | `flock` | |
| ~~Storage~~ | **not an arm** | | | |
| ~~Files and folders~~ | **not an arm** | | | |

**In Rust the whole radio column collapses into one crate.** `btleplug` is one async API over
CoreBluetooth, BlueZ and WinRT, and it was measured against the cube on 2026-09-20 doing every step this
app needs. That is the single largest saving of the language change and it is why the change was made.
The detail is in [rust-port.md](rust-port.md).

**`tray-icon` and `keyring` collapse two more**, untested. The menu bar arm stays an arm regardless,
because what the three platforms *allow* differs: Windows can never put text beside the icon, and on
Linux the host owns the right click. [rust-port.md](rust-port.md) has the table and the design rule that
follows from it, which is that **nothing may live behind a left click that has no menu equivalent.**

### Why storage is not an arm

**Facet always uses SQLite as its local database, on every platform.** `rusqlite` with the `bundled`
feature compiles SQLite into the binary, so nothing is installed on the user's machine and the schema
carries over unchanged. There is no second implementation to select, so there is no square.

**A Facet server, if one is turned on, is a peer and not a backend.** It is an additional thing that can
be switched on and off, exactly like the Google connection. Nothing replaces the database as the source
of truth. A server client is therefore an ordinary core module that talks HTTP, beside the two Google
clients, and syncing anything *down* means writing it to the database and then reading the database.

**What crosses the wire is times and categories, and nothing else.** The server exists so a team leader
can look at reports, and a report is made of recorded time filed under categories. **Nothing about the
device syncs**, and that is a rule rather than an omission: `device_uuid` in particular means nothing on
another machine, because on macOS it is the identifier CoreBluetooth assigned *that Mac* and on Linux it
is the cube's real Bluetooth address. Syncing it would put a number naming nothing into a row that the
reconnect path reads.

---

## What the Swift split proved, and what it cost

**Measured, and the reason the model is being carried over rather than reconsidered.**

- **A second platform cost the core zero lines** (2026-09-11, re-checked 2026-09-16). Twelve core modules
  including the whole device pipeline were constructed in a second composition root and ran. There was no
  `#if os(Linux)` anywhere in `Sources/`.
- **Of the two macOS radio modules, 2.6% was actually CoreBluetooth**: 16 lines of 695 and 24 of 825. The
  radio looks like the most platform-bound part of the app and is almost entirely not. That figure is
  what to expect when porting the radio to `btleplug`: the sequencing is the work and it already exists.
- **The split itself cost 589 access-level edits** and could not be read off one build; the first build
  reported 4,801 error lines naming 94 types, and widening those exposed the next layer, for a dozen
  rounds. **This cost does not recur in Rust.** It was Swift's `internal`-by-default across module
  boundaries, and `pub(crate)` against a workspace has the same shape but the split is being made once,
  up front, rather than retrofitted.
- **Three things had to move out of a platform target into the core**, and each was the same shape: a
  decision written down inside a platform target that the second platform needed too. The BLE trace
  wordings (read back by the scripted suite, so they are interface), the seeded device settings (they are
  the DDL's own seeds), and a menu line that one platform has no window to open for. **Expect this class
  again.** It is not a failure of the model; it is what the second platform is for.

**And one asymmetry turned out to be the argument for a port rather than a cost of one.** The dialogue
port carries *which button is the way out* as a position, because AppKit relocates a button titled
Cancel, which takes Return off the way out. GTK relocates nothing, so the Linux adapter honours the field
in one line. A port whose shape was forced by one platform's difficulty cost the other nothing.

---

## Deep modules

Vocabulary used consistently across these docs, from the September 2026 review of the Swift tree.

A **module** is anything with an interface and an implementation. Its **interface** is everything a caller
has to know: the signature, but also the ordering constraints, the error modes and the invariants. A
module is **deep** when a lot of behaviour sits behind a small interface, and **shallow** when its
interface is nearly as complex as what is inside it.

The rewrite is the moment to make the shallow ones deep, because the interfaces are being written from
scratch anyway. The review's nine candidates are in the reference tree at
`~/harry.git/TimeFlipLinux/docs/architecture-review-2026-09.md`; four were done in Swift and the rest are
open. Read it before
designing a core module that has a Swift counterpart, and read that counterpart's tests in
[behaviour-inventory.md](behaviour-inventory.md) for what it has to do.
