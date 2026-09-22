# Handover: for the Mac

[← Back to README](../README.md) · [The other direction →](handover-linux.md) · [The Mac →](system-mac.md) · [The Linux box →](system-linux.md)

**This file is for the Mac to act on. Everything in it was written by the Linux box.**

Its mirror is [handover-linux.md](handover-linux.md), which the Mac writes and the Linux box acts on.
Neither machine edits the file addressed to itself except to delete from it, and neither deletes from the
file it wrote.

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
   [system-mac.md](system-mac.md); something the hardware does goes in
   [timeflip2-firmware-observations.md](timeflip2-firmware-observations.md); something the port measured
   goes in [port-findings.md](port-findings.md), and something about the shape of the code goes in
   [architecture.md](architecture.md). This file is the asking, and it is meant to empty.
4. **An item you cannot do stays put, with a line saying why.** That is an answer too, and a more useful
   one than silence, but say it in the item rather than deleting it, so whoever asked can decide what
   to do instead.
5. **Numbers are labels and are never reused.** A gap means an item was finished. A new item takes the
   next number never used before, so a commit message saying "handover 3" still means the same thing
   years later.
6. **When you have done everything you can, write what you want back.** Add items to
   [handover-linux.md](handover-linux.md) for the other machine. A blank file on both sides is the
   finished state.

---

## 2. Not a task: `cargo fmt` is not a gate, and this box cannot make it one alone

**Flagged rather than asked, because it is a decision and not work.** `cargo fmt --check` fails on this
tree. Measured on a clean checkout on 2026-09-22, so **it predates the Linux work and is not something
that arrived with it**.

**It is not a formatting lapse.** There is no `rustfmt.toml`, and the house style is wider than rustfmt's
defaults: compact struct literals such as `Showing { paused: false, locked: false }` are on one line
throughout, and rustfmt's `struct_lit_width` of 18 would explode every one. Running `cargo fmt` would
rewrite most of the codebase into a style nobody chose, which is why nobody has run it here.

**The Mac cannot check this at all**, rustfmt not being installed there: `docs/system-mac.md` records it,
and it is why no commit from that side has ever been formatted-checked. **So the honest position is that
the project has no formatting standard it enforces**, and making one means agreeing a `rustfmt.toml` first
and reformatting once, deliberately.

`cargo clippy` is the gate that is actually ready: installed here, exit 0 on `facet-core`, `facet-ui` and
`facet-linux`. It is **not** installed on the Mac either, and CI gates on neither.
