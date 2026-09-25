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

---

## 10. Make the tray's Pause drive the Faces tab's clock, as the Mac's now does

**On `feature/faceTab`, which is where this lands.** The Faces tab times by hand against the database
(`facet_ui::faces::Faces`), and on the Mac the menu bar is now part of that clock (`5184cbd`): the status
item's left click and the menu's Pause item both call `Faces::toggle_pause`, and the icon and the item follow
`Faces::menu_bar_timing`. **The Linux tray still flips its own `showing.paused`** in `tray.rs`, so pausing
from the tray there changes the icon and times nothing.

**What is wanted:**

1. **Route `Activated` and the Pause menu item to `faces.toggle_pause()`** on the UI thread, in `drain`.
   The tray thread stops deciding pause itself: it posts the press and nothing else.
2. **Have the tray follow the clock.** `faces.set_on_timing_changed(...)` fires after every re-read (every
   toggle, every click on the tab, every tick while the figure moves). From it, read
   `faces.menu_bar_timing()` and push `is_paused`, `pause_title` and `is_clickable` to the ksni thread, for
   example through `Handle::update`. Pause is disabled when `is_clickable` is false.
3. **Leave Lock as it is**: it is the cube's, and there is no radio yet.
4. **Check it on MATE**: left click pauses and resumes, the icon follows, the menu item reads Resume while
   paused, and a category clicked on the Faces tab changes the icon.

**It also needs building there at all.** `feature/faceTab` wires `Faces` into `facet-linux/src/main.rs` and
the Mac could not compile that crate, so the first thing is `cargo build -p facet-linux` on this branch.
