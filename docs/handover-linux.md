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

1. **5 is not a task.** It is what is not proven yet, and it is all that is left.

---

## 5. Not a task: `keyring` has never been built against anything

**Flagged so nobody reports the secret store as working on both platforms.** `libsecret-1-dev` 0.21.4 is
installed here and the Secret Service is live, both measured, and `keyring` 4.2.0 is pinned in the
workspace. **No crate in the workspace or either probe pulls it in**, so nothing on this box has exercised
it and nothing on the Mac has either since the launch probe was taken out.

When the secret store port arrives, this is the half with no evidence behind it.

---

**The Linux half now has evidence, so half of this is retired** (2026-09-22, by the box this file is
addressed to). `probe/keyring-secret-service` pulls `keyring` 4.2.0 in and round-trips a secret through
the live Secret Service: written, read back and compared, five bytes including a non-UTF-8 pair, deleted,
and a read afterwards answering `NoEntry` rather than an error -- which is the part the app depends on,
needing to tell a first run from a broken store. **`libsecret-1-dev` turned out not to be load-bearing**:
the binary links `libc` and `libgcc_s` and nothing else, the path being pure Rust over zbus.

**It stays open because the Mac half is still inference**, which is what this item exists to prevent, and
because two things came out of the probe that are decisions rather than measurements:

- **The locked keyring is still untested** and the probe deliberately does not force it: the only
  collection here is `login`, which holds the `gh` token, so it needs a person who has agreed to it.
- **`keyring` may be the wrong crate.** Its own docs say an application choosing its store per platform
  *"should not be linking to this library at all"* and should take `keyring-core` plus a specific store.
  That describes this app. A question for the port, not a change to make now.

[handover-mac.md](handover-mac.md) item 3 asks the Mac for its half.
