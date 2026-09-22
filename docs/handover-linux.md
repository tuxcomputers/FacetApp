# Handover: for the Linux box

[← Back to README](../README.md) · [The other direction →](handover-mac.md) · [The Linux box →](system-linux.md) · [The Mac →](system-mac.md)

**This file is for the Linux box to act on. Everything in it is written by the Mac.**

Its mirror is [handover-mac.md](handover-mac.md), which the Linux box writes and the Mac acts on. Neither
machine edits the file addressed to itself except to delete from it, and neither deletes from the file it
wrote.

**Two developers handing work to each other across a desk, and this is the note left on the keyboard.**
Both machines commit to the same branch under the same identity, so the only way one can ask the other
for something is to write it down where the other will look.

## How to work through this

1. **Take the items in whatever order suits.** They are numbered so a commit message can name one, not
   to say which comes first. Where one genuinely blocks another the item says so.
2. **Delete an item the moment it is done, one at a time, and commit that deletion on its own.** The
   commit message is where the answer goes: what you ran, what came back, what you changed because of
   it. Not struck through, not ticked, not moved to a "done" list: removed. One item, one commit, so the
   history reads as a conversation rather than as a bulk edit.
3. **Put facts where facts live, not here.** Something true about this machine goes in
   [system-linux.md](system-linux.md); something the hardware does goes in
   [timeflip2-firmware-observations.md](timeflip2-firmware-observations.md); something the port measured
   goes in [port-findings.md](port-findings.md), the D-Bus mechanics in
   [linux-bluez-port-notes.md](linux-bluez-port-notes.md), and something about the shape of the code in
   [architecture.md](architecture.md). This file is the asking, and it is meant to empty.
4. **An item you cannot do stays put, with a line saying why.** That is an answer too, and a more useful
   one than silence, but say it in the item rather than deleting it, so whoever asked can decide what
   to do instead.
5. **Numbers are labels and are never reused.** A gap means an item was finished. A new item takes the
   next number never used before, so a commit message saying "handover 3" still means the same thing
   years later.
6. **When you have done everything you can, write what you want back.** Add items to
   [handover-mac.md](handover-mac.md) for the other machine. A blank file on both sides is the finished
   state.

## What this is not

**Not the system files.** [system-linux.md](system-linux.md) and [system-mac.md](system-mac.md) hold
*facts about a machine*: what compiler, which filesystem, where the data directory resolves. A question
whose answer is a fact belongs there, written into that machine's own file. This file asks for *work*:
build something, run something, decide something.

**Not the open questions in [rust-port.md](rust-port.md).** Those are what the port has not settled yet,
for whichever machine gets to them. This is what the *other* machine is being asked for, which is a much
shorter list and one that empties.

## The order I would take these in

**A recommendation rather than a rule**, and what it is really saying is which items unblock the most.

1. **2 first**, and it is the only one with nothing in front of it. It needs no window, no radio and no
   decisions, and finishing it means every later item can say what it did in a way this machine can read
   back.
2. **Then 3**, which is the first thing that puts Facet on screen here at all.
3. **4 and 6 are cheap and answer questions that are currently blocking nobody but will block everybody**,
   so take them whenever the machine is in front of you.
4. **1 and 5 are not tasks.** They are what is not ready for you, and what is not proven yet.

---

## 1. Not a task: the Settings window is still inside `facet-mac`, and cannot be shared yet

**Do not start a Slint window here.** Every `.slint` file lives in `crates/facet-mac/ui/` and is compiled by
that crate's own `build.rs`, so today the Settings window is a macOS-only thing. Building a second one here
would be the opposite of what the port is for: requirement 4 in [rust-port.md](rust-port.md) is a single
shared UI, and it is the requirement the whole language choice rests on.

**The Mac will lift it out**, into somewhere both composition roots can compile, and this item goes when
that has happened. Until then the tabs, the pill bar, the metrics and the widgets are all read-only from
here: they are worth reading, since they are what this machine will draw too, but a change to them made
here would be a change to a file the Mac is about to move.

**What is not blocked by this**: everything in item 2, and the whole of item 3. The tray is not the window,
and the composition root does not need a window to bring up a database and start recording.

## 2. The composition root for Linux: the data directory, the database, and the trace

**`crates/facet-linux/src/main.rs` is five lines and prints that it is not implemented.** This is the item
that changes that, and it is deliberately the one with nothing in front of it: no window, no radio, no
decisions left open.

**The core half is already done and is platform-blind.** `facet_core::database::open` takes a path and a DDL
list, `facet_core::setting::debug_trace` reads whether the trace is wanted, and
`facet_core::debug_log::DebugLog` prints and records. None of them knows what platform it is on, and none of
them should learn.

**What this machine has to supply is the paths**, which is the half that is genuinely a platform fact:

| | |
|---|---|
| Data directory | `~/.local/share/Facet`, from `$HOME` rather than from a library call, so it is the same answer `Tests/Scripted/platform.sh` gives |
| The app database | `appdata.sqlite` in it, usually a symlink, opened by name so sqlite resolves which physical file it is |
| The trace | `debug.sqlite`, in the folder the `debug` setting names, or beside the app database when that setting is empty |

`crates/facet-mac/src/main.rs` has the working version of all of this in `data_directory`, `open_databases`
and `expand_home`. **Read it as a worked example, not as something to import**: the only line in it that is
really about macOS is the directory.

**Two things to check on the way through, both cheap and both able to waste an afternoon if they are wrong:**

- **The three `5xx` DDL files are symlinks** (`500_timezone.sql`, `502_timezone_alias.sql`,
  `503_timezone_lookup.sql`), committed as mode `120000`. `include_str!` follows them, so the trace schema
  compiles in correctly, but a clone made with `core.symlinks=false` would embed the string `002_timezone.sql`
  as SQL instead. Confirm `ls -la` shows arrows before assuming anything else is wrong.
- **`rusqlite` is compiled in with its `bundled` feature**, so `cc` runs on every clean build and nothing
  links the system SQLite. That is measured as working on this box (gcc 13.3.0, see
  [system-linux.md](system-linux.md)); it is noted here because it is the first time the workspace has
  actually pulled a C dependency in rather than merely listing one.

**How to know it worked.** `scripts/switch-database.sh test -clean` builds a test database from the same DDL,
then turn the trace on in it and launch:

```sh
sqlite3 ~/.local/share/Facet/test.sqlite \
  "UPDATE setting SET setting_value = json_set(setting_value, '$.enabled', json('true')) \
   WHERE setting_name = 'debug';"
scripts/run.sh
sqlite3 ~/.local/share/Facet/debug.sqlite "SELECT logged_at, tag, message FROM debug_log;"
```

Two rows on the Mac: the trace opening, and the launch. **Leave production alone**, which is the seeded
value and what a fresh install should be.

## 3. The tray, through `ksni` and not `tray-icon`

**`tray-icon` cannot do this on Linux and the reason is not a preference.** Its Linux backend is
libappindicator, which emits no click events at all, and it offers no `ksni` option: the feature list is
`gtk` plus `libappindicator` and nothing else. See
[port-findings.md](port-findings.md#the-tray-icon-is-a-different-kind-of-object-on-each-platform).

`ksni` 0.3.6 is pinned in the workspace and **has never been built against**. It is the richest of the three
platforms' tray APIs: raw pixels through `icon_pixmap`, which carries its own width and height, and
`overlay_icon_pixmap` for a second image drawn on top of the first.

**The shape to build is the one measured on this box on 2026-09-18** and written up in
[rust-port.md](rust-port.md#the-menu-bar-on-mate-measured): left click reaches the app, right click makes the
host show the menu and the app never sees the event. So **nothing may live behind a left click that has no
menu equivalent**, and Pause goes first on the menu because left click is its accelerator rather than its
mechanism.

**The icon itself is `crates/facet-mac/src/status_icon.rs`**, which is worth reading before writing a second
one. It composes Play, Pause and Lock as a variable-width RGBA image, 32 pixels per cell, and its six tests
are about geometry rather than about macOS. **Whether a non-square icon survives on this platform is
untested**: `icon_pixmap` carries its own width and height and the specification permits it, which is not the
same as the applet honouring it. That answer belongs in
[port-findings.md](port-findings.md) when you have it, beside the macOS and Windows ones.

## 4. Say which applet the tray probe ran in

**Cheap, and it decides how far the 2026-09-18 measurement travels.** That probe got exactly the click split
the design wants, but nothing recorded which applet was showing it, and this box has
**`mate-indicator-applet` 1.26.0** installed: the Indicator Applet, whose left and right click bug
[is still open](https://github.com/mate-desktop/mate-indicator-applet/issues/33), rather than the
Notification Area applet whose bugs are closed and which also speaks XEmbed.

**The behaviour observed was the correct one either way**, so this is not a doubt about that machine. It is
that a result from the Notification Area applet generalises to other MATE installs and a result from the
Indicator Applet is a result about an applet that happened not to bite.

Write the answer into [rust-port.md](rust-port.md), in the paragraph that currently says it was not recorded.

## 5. Not a task: `keyring` has never been built against anything

**Flagged so nobody reports the secret store as working on both platforms.** `libsecret-1-dev` 0.21.4 is
installed here and the Secret Service is live, both measured, and `keyring` 4.2.0 is pinned in the
workspace. **No crate in the workspace or either probe pulls it in**, so nothing on this box has exercised
it and nothing on the Mac has either since the launch probe was taken out.

When the secret store port arrives, this is the half with no evidence behind it.

## 6. Does a Slint window appear on the accessibility bus?

**The single most load-bearing unknown for the scripted suite here**, and it needs the owner's screen, so
ask rather than launching.

**What is known**: Slint ships its own AT-SPI bridge and it is in the graph, measured 2026-09-20 from the
probe's build. `accesskit` 0.24.1, `accesskit_unix` 0.22.1, `accesskit_atspi_common` 0.19.1 and `atspi` 0.29.0
all compile as part of `i-slint-backend-winit`. So a Slint window does not need `libatk-adaptor` the way a
GTK app does: it speaks AT-SPI over zbus in pure Rust, which is why the Slint probe links no `libdbus` while
the btleplug one does.

**What is not known is whether any of that reaches the bus.** The probe was built and never run. Until it is,
every accessibility id in the Settings window is a promise rather than a fact, and
`scripts/at-press.py` has nothing proven to press.

**This one is blocked by item 1**, there being no window here to put on the bus, unless you run the Slint
probe in `probe/` instead. That is the cheaper route and answers the same question.
