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

## 4. Render the Settings tabs here and compare them with these numbers

**The other half of [handover-linux.md](handover-linux.md) item 7, which cannot be answered from one
machine.** The renderer has moved to `facet-ui`, so both boxes run the same command against the same
sources:

```sh
cargo run -p facet-ui --example draw-settings-tabs
```

**The move is done and needs nothing from you.** `crates/facet-mac/examples/draw-settings-tabs.rs` is now
`crates/facet-ui/examples/draw-settings-tabs.rs`, unchanged apart from the two usage lines naming the new
crate. Nothing in it was ever about macOS. `facet-ui` gained `png` as a dev-dependency and needed no
`renderer-software` feature, that being a slint default all along -- which also means the explicit one on
`facet-mac`'s `slint` line was always redundant. **It is left on that line**, this box not being able to
compile the crate to prove that taking it off changes nothing; simplifying it to `slint.workspace = true`
is safe to do from there.

**What Linux produced, 2026-09-25**, measured off the PNGs rather than eyeballed:

| | Linux |
|---|---|
| Every tab | **640 x 680**, all six |
| Tab bar rule | y = **44**, all six |
| Panel box, App tab | x **22** to **618** |
| Stepper arrows, all three App rows | x **537-568** |
| Those three rows | y 148-161, 180-193, 212-225, so **32px apart** |
| Checkbox, Show seconds row | x **596-609** |

**The stepper alignment is the one item 7 said would break quietly**, a different default `SpinBox` font
size moving the 110px box and the 34px unit slot without breaking a build. Here all three sit on the same
x to the pixel, and `AM`, `min` and `secs` line up. If the Mac's three do not agree with each other, that
is the fault; if they agree with each other but not with 537-568, that is a font metric difference and
still worth knowing.

**Nothing was elided on any tab**, which was the other named risk. The Report tab's two calendars came out
identical to each other, so the derivation from the day cell holds on this box.

**The Linux images are committed now**, at
[`docs/settings-tabs/linux/`](settings-tabs/linux/), six PNGs and a README saying what they are. `target/`
is ignored, so putting them in the tree is what makes the comparison possible at all from two machines
that never see each other's disks.

**So: run the command, then copy `target/settings-tabs/*.png` into `docs/settings-tabs/mac/` and commit
them.** That folder is the one thing this item actually needs back; the numbers above are for checking
without opening an image viewer.

**Then compare, and write the answer down either way.** If they match, that is one line in
[port-findings.md](port-findings.md) saying the shared window is measured identical, because *measured
identical* is a fact somebody would otherwise pay to establish twice. If they do not, the difference is
the finding and the two images are the evidence beside it.

**They are a dated snapshot rather than a golden file**, and the README says so: nothing diffs them
automatically and nothing should. Once the comparison is written up they stop being load-bearing, and a
stale image is not evidence about a window that has moved on since.

**Worth opening the real window afterwards as well, and it is a separate exercise.** The software renderer
rasterises its own fonts, so it takes the window server out of the comparison deliberately -- which is the
point, and also means it cannot answer what fontconfig against Core Text does to the running app. Fonts
are the likeliest difference between the two machines and the one thing these images are guaranteed not
to show. That needs somebody at the screen on both boxes.
