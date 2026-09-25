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

1. **8 first**: it is two commands and a doc fix, and the one-off reformat waits on its answer.
2. **7** is waiting on the Mac to run the renderer and compare, so there is nothing to do on it here.

---

## 7. Does the shared Settings window actually look the same on MATE?

**Nobody has looked, and it is the requirement the language choice rests on.** Requirement 4 in
[rust-port.md](rust-port.md#the-requirements) is a single shared UI, and requirement 3 is that it looks and
operates the same on all three platforms. `facet-ui` now makes that structurally true: one set of `.slint`
sources, one `build.rs` choosing cupertino, compiled into both composition roots. **Structurally true is
not the same as true**, and the window has been on screen on both machines without the two ever being put
side by side.

**What is likely to differ, and none of it would fail a build:**

- **Fonts.** Cupertino is a Slint style rather than a native toolkit, so the widgets should match, but the
  text is rendered by whatever fontconfig serves on that box against whatever Core Text serves here. Every
  width in the Settings window is fixed except the labels, so a wider font wraps or elides rather than
  resizing anything, and elided labels are the thing to look for.
- **The 640 width.** `min-width` and `max-width` pin it here because the Mac windowing layer treated a
  plain `width` as a hint and gave an 800pt window. Whether the same pinning holds under MATE is unknown.
- **The stepped fields.** They were the one control that had to be told not to stretch, and they carry a
  hard 110px box and a 34px unit slot. A different default font size inside a `SpinBox` would break that
  alignment without breaking anything else.
- **The calendars on the Report tab.** Every size there is derived from the day cell, which is derived
  from the tab width rounded down. It should therefore be identical, and if it is not then the derivation
  has an assumption in it that nobody has found.

**The Mac owes you the tool first, and this item is blocked until then.** The headless renderer is
`crates/facet-mac/examples/draw-settings-tabs.rs`, which draws all six tabs through Slint's software
renderer into `target/settings-tabs/` with no window and no tray. **It is in the wrong crate**, for exactly
the reason the status icon was: nothing in it is about macOS, it only lives there because that is where the
UI used to live. It moves to `facet-ui` and then both machines run the same command and produce comparable
images. That is the Mac's to do and this item says so rather than asking you to work around it.

**Software rendering is the point, not a limitation.** It takes the platform's window server out of the
comparison entirely, so a difference in the output is a difference in the layout rather than in how a
compositor drew it. Worth running the real window afterwards as well, since fonts are the likeliest
difference and the renderer rasterises those itself.

**Where the answer goes**: a real difference is a port finding and belongs in
[port-findings.md](port-findings.md); no difference is worth one line in the same place, because
*measured identical* is a fact somebody will otherwise pay to establish twice.

---

**The Linux half is done, and the tool was moved here rather than waited for** (2026-09-25). Nothing in
`draw-settings-tabs.rs` was about macOS, so this box moved it to `facet-ui`, verified it by running it,
and left `facet-mac` compiling as it was. **That was the Mac's to do and it had emptied its handover
without doing it**, so waiting would have blocked this on something nobody was tracking. `facet-ui`
needed only a `png` dev-dependency: `renderer-software` is a slint default, so the explicit one on
`facet-mac` was always redundant.

**What this box produced**, measured off the images rather than eyeballed:

| | Linux |
|---|---|
| Every tab | **640 x 680**, all six, so the width pins under MATE as it does on the Mac |
| Tab bar rule | y = **44**, all six |
| Panel box, App tab | x **22** to **618** |
| Stepper arrows, all three App rows | x **537-568**, identical to the pixel |
| Checkbox, Show seconds row | x **596-609** |

**Nothing elided on any tab**, and the Report tab's two calendars came out identical to each other, so
the derivation from the day cell holds here. **The stepper alignment, which this item named as the thing
that would break quietly, is intact**: `AM`, `min` and `secs` all line up.

**The six images are committed**, at [`settings-tabs/linux/`](settings-tabs/linux/) with a README
saying what they are and how to regenerate them. `target/` is ignored, so putting them in the tree is
what makes a comparison possible between two machines that never see each other's disks. They are a
dated snapshot rather than a golden file: nothing diffs them automatically, and once the comparison is
written up they stop being load-bearing.

**It stays open because one machine cannot answer it.** The question is whether the two *match*, and
that needs the Mac to run the same command now that it can, and to commit its own six into
`settings-tabs/mac/`. [handover-mac.md](handover-mac.md) item 4 asks for exactly that and carries these
numbers to check against. Nothing goes in [port-findings.md](port-findings.md) until there is a
comparison to record, in either direction.

**These cannot answer the font question**, which is the likeliest difference of the four this item
names: the software renderer rasterises its own. That wants the real window open on both machines, with
somebody at each screen, and is a separate exercise from this one.

---
