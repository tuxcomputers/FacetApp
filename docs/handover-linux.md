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

## 13. Run the scripted suite on this branch as it stands, and commit the stamp

**CI now needs a passing run from both machines** (`scripts/check-scripted-stamps.sh`, the
`Scripted suite run on both machines` job, required by `All tests pass`). The Mac's stamp passes. The Linux
stamp is refused: it records `cf9f664` on a dirty tree, and `crates/` and `Tests/Scripted/` have changed since,
including the reformat (`088c676`) and the Mac's harness fixes.

**What is wanted:**

1. **Pull, commit anything outstanding, then run `Tests/Scripted/run.sh`** from a clean tree.
2. **Commit `last-run-linux.md` straight away**, before anything else and before running again.
3. **Check it clears the gate:** `scripts/check-scripted-stamps.sh --branch feature/faceTab` should say both
   machines passed. Any change to `crates/` or `Tests/Scripted/` after that needs a fresh run on both.

The harness changes the Mac made (`a5f92eb` to `2fa801f`) touch only macOS paths in `lib.sh` and
`platform.sh`, but say if a Linux check moves.

## 14. The Categories tab: build it, render it, and convert `04-categories`

**On `feature/catergoryTab`** (spelled as the branch is). The Categories tab now reads and writes the
database (`997d50d`, wired in `e5992ef`), and the Faces tab's notice and Create moved to shared code on the way
(`65a2314`). The Mac could not compile `facet-linux`, whose `main.rs` changed in both.

**What is wanted:**

1. **`cargo build -p facet-linux`**, then run it and open the Categories tab once.
2. **Re-render against a clean test database and rebuild `compare/`**, as the settings-tabs README says. Only
   `2-categories.png` changed on the Mac; the other five should still be byte-identical to both committed
   sets. The Categories render draws icons and a disabled checkbox and stepper, which have not been compared
   across the machines before.
3. **Convert `Tests/Scripted/04-categories.sh`** from the Swift suite, as `05`, `06` and `12` were. What has
   changed under it:
   - **The dialogues are in-window notices now**, not native alerts: `notice`, `notice-title`,
     `notice-message` and `notice-choice-<n>` in the tree, pressed like any button. `alert_buttons` and
     `press_sheet` need to read those on both platforms.
   - **The icon grid and colour list are overlays in the pane**, ids unchanged (`icon-grid`,
     `icon-cell-<file>`, `colour-list`, `colour-option-<Name>`).
   - **The stepper cannot do the hold test.** Slint's SpinBox has no `-up`/`-down` ids and repeats at its own
     rate, so the Swift acceleration (1 per 0.1s to the second multiple of 5, then 5 per 0.3s) is not built.
     Keep the typed-value checks and drop the hold ones, saying so in the script.
   - **A refused write now raises a notice** ("The change was not saved"), where the Swift app only logged
     `REFUSED`.
   - **The log wording is the Swift app's**, category names passed through `debug_log::plain`, so the
     patterns should carry over.
4. **Run the suite and commit the stamp.** CI needs both machines' stamps on this branch, and `crates/`
   has changed since either was taken.
