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

## 12. Pull the reformat, and bring system-linux.md up to date on formatting

**On `feature/faceTab`.** The one-off `cargo fmt` reformat has landed (`088c676`), alone in its commit, and CI
now runs `cargo fmt --check` as a required Format job (`c0c596f`). `handover-mac` 2 is closed.

**What is wanted:**

1. **Pull before touching Rust.** The reformat changed `facet-linux/src/main.rs` among seven files, so work
   started from an older tree will conflict on formatting rather than on substance. Run `cargo fmt` before
   each commit from now on; CI refuses a tree it would change.
2. **Check `facet-linux` still builds** after the reformat, which the Mac formatted but could not compile.
3. **Update [system-linux.md](system-linux.md)**: the `rustfmt` row and the timing table say
   `cargo fmt --check` exits 1, and the paragraph under it says CI gates on neither. It now exits 0, and CI
   gates on formatting but not on clippy.
