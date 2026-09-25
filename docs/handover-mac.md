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

## 6. Run the suite on `feature/catergoryTab`, with `04-categories` in it

**On `feature/catergoryTab`.** `04-categories` is converted and passes on Linux, and the whole run is 169 of 169
at `06f9f11` (`last-run-linux.md`). CI's stamp check now fails only on the Mac side: `crates/` and
`Tests/Scripted/` have changed since the Mac's run.

**What is wanted:**

1. **Run `Tests/Scripted/run.sh` and commit `last-run-mac.md` straight away.**
2. **Watch the notice helpers, which now serve macOS too.** The Rust app has no native alerts, so
   `platform_alert_buttons`, `platform_alert_message`, `platform_press_title` and `platform_press_sheet` read
   the `notice-choice-<n>` buttons from the tree on both platforms (`platform_notice_choices` in
   `platform.sh`). They parse `value=` or `title=` for the label. Linux prints `value=`, and it is a guess what
   `ax-dump.py` prints for a Slint Button. If `04` fails on "the notice offers three buttons", that is where
   to look.
3. **`set_field_cut`** is the ordinary `ax-set.py` write on macOS, on the belief that writing `AXValue` bypasses
   the field's own 35-character cut. Say if the cut happens anyway: the check reads the table, so it passes
   either way.
4. **Say if your window clips the Categories lists.** On Linux the window opens 400 points tall, and rows the
   pane clips are absent from the tree. `04` now folds whichever section it is not working in. A taller Mac
   window should not care.
