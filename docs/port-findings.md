# Port findings: what was measured before the rewrite started

[← Back to README](../README.md) · [The Rust port →](rust-port.md) · [Architecture →](architecture.md) · [BlueZ notes →](linux-bluez-port-notes.md)

**Facts established by building a second platform in Swift, kept because they cost real time to find and
none of them is about Swift.** This file exists so the Rust port does not pay for them twice.

**Everything here is measured, with the date and the machine.** Where something was reasoned rather than
run, it says so. The full originals, including the Swift-specific material and the task lists that are now
spent, are `docs/linux-port.md` and `docs/architecture-review-2026-09.md` in the reference tree at
`~/harry.git/TimeFlipLinux/docs/`.

**What is deliberately *not* here:** anything about the cube's own behaviour, which is
[`timeflip2-firmware-observations.md`](timeflip2-firmware-observations.md); anything about BlueZ as a
stack, which is [`linux-bluez-port-notes.md`](linux-bluez-port-notes.md); and the Rust evaluation itself,
which is [`rust-port.md`](rust-port.md).

---

## The device pipeline works end to end on the second platform

**Measured on the Linux box against the real cube, 2026-09-13.** Pair, rotate the PIN into the login
keyring, reconnect on a later launch, bring in the cube's own history, file a time entry. All of it, on
BlueZ, with no change to a single core module.

**And it turned up three faults that the hermetic suite could not see.** That ratio is the finding: a
suite of 1,718 tests was green throughout, and putting the thing in front of real hardware for the first
time found three real faults in an afternoon. **Budget a hardware session per platform slot**, and do not
treat a green suite as evidence that a slot is done.

---

## The Linux tray is a D-Bus object, and that changes how checks address it

**Measured 2026-09-08, corrected 2026-09-13.** Three corrections, all load-bearing.

- **No identifier crosses to the tray.** The menu port carries an identifier per item; it reaches
  `AXIdentifier` on macOS and reaches **nothing** on Linux. `com.canonical.dbusmenu`'s `GetLayout` answers
  the label plus whether the item is enabled, and that is all. **So a Linux check addresses a tray item by
  its label.** Design the checks around that rather than discovering it in one.
- **The numeric ids are libdbusmenu's own and are reassigned on every rebuild**, so an id read in one step
  is meaningless in the next. The first spike recorded ids 2, 3 and 4 as though they were stable.
- **The `Event` signature is `isvu`, not `issu`.** The data argument is a variant, so python-dbus needs
  `dbus.String("", variant_level=1)` and an explicit signature, or the call is refused.

**The tray label reads back.** `XAyatanaLabel` on `org.kde.StatusNotifierItem` answers what the app put
there, so a check can assert the menu-bar clock directly. **Colour cannot be read**, an indicator label
being plain text, which is why the readout writes its colour decisions to `debug_log`: that is the only
way a check sees them on **either** platform.

`scripts/tray-menu.py` is the working implementation of all of the above and carries over unchanged.

**This finding applies to `tray-icon` in Rust as much as it did to the Swift adapter**, because it is a
fact about the StatusNotifierItem protocol rather than about the binding. It is also the answer to open
question 4 in [rust-port.md](rust-port.md): macOS loses its status-item identifier in the move to
`tray-icon`, and addressing by label is the pattern that already works.

---

## A toolkit app is drivable by accessibility, without a mouse

**Measured on the Linux box, 2026-09-08**, against a GTK3 stand-in shaped like the parts a check has to
reach. Every claim confirmed twice: the action returned successfully **and** the app recorded that it
happened.

| | |
|---|---|
| AT-SPI sees the app | Yes. GTK3 loads the atk-bridge on its own |
| `toolkit-accessibility` does not gate it | The gsetting read `false` and the app was visible anyway. **Worth knowing, because the obvious first move is to turn it on**, and doing so would have credited the wrong thing |
| The identifier equivalent | The accessible **name** is the locator; a lookup is one tree walk comparing it |
| Pressing | `queryAction().doAction(0)`. **No mouse event, no coordinates** |
| Typing | `queryEditableText().setTextContents(...)`, and the app's change handler fired |
| Reading for assertions | `queryText().getText(0, -1)`. The name stays the identifier and the text is the value, the same split as `AXIdentifier` against `AXValue` |
| **While unfocused and covered** | **Yes.** Another window was raised over it and made active, and the press still landed. This is the difference that matters most for a suite |

**What this does not say.** Nothing was run headless: `xvfb` is not installed on that box and it has no
passwordless sudo, so whether the suite can run without a screen is open. The screen was not locked
either.

**The Slint equivalent is measured and separate**, on macOS, 2026-09-20: `accessible-id` arrives as
`AXIdentifier`, per-row identifiers built by string concatenation inside a `for` come through intact, and
`accessible-id` is refused unless `accessible-role` is set alongside it. See
[rust-port.md](rust-port.md) and [scripted-suite.md](scripted-suite.md).

---

## Measure the thing the app does, not a convenient stand-in for it

**The most expensive lesson of the whole evaluation, and it is about probing rather than about any
toolkit.**

Three of the four problems first reported from the Slint harness on 2026-09-20 were artifacts of driving
it with synthetic clicks: click-to-edit "not working", the caret opening at position 0 rather than where
the click landed, and a full-window touch area "swallowing" row clicks, which led to that area being
deleted on a theory that was simply wrong. **A human click settled all three in one minute.**

The conclusion drawn from them, that the whole driver layer would have to be replaced, was also wrong, and
it was wrong for the same reason: the probing used System Events `click at`, which **is not how the suite
drives anything**. `scripts/ax-press.py` performs an accessibility action and `scripts/ax-set.py` writes a
property. Neither goes near a coordinate, and both work against Slint unchanged.

**The residue is real**: synthetic clicks do not reach a bare touch area. That is [open question 3's
answer](rust-port.md) and it produces a design rule, below.

---

## An empty listing is not a success, and one platform made that a silent failure

**Measured 2026-09-06. Two faults that compounded, and the repository was already arranged to trigger
them.**

Directory enumeration through a symlink returned **zero** entries on Linux where macOS returned fifteen.
The database bootstrap filtered that listing and applied what survived, so **an empty listing applied
nothing and reported success**: database created, files applied none, no error. The result was a database
with no tables, described as a database that was created. It was found because every setting read came
back null and nothing anywhere said why.

**Two rules come out of it, and both outlive the language:**

- **A DDL directory yielding no files is never a correct outcome.** Bootstrap must fail loudly on an empty
  listing, on every platform. This was a fix on macOS as much as on Linux.
- **No runtime path should traverse a symlink to reach its resources.** In this repository the DDL is a
  real directory at `crates/facet-core/resources/database/` and nothing links to it.

**The narrower fact, for anyone who hits the symptom:** it is specific to a symlinked *directory*. A
symlinked file inside a real directory reads straight through.

---

## A binary that works on the build machine is not a binary that ships

**Measured 2026-09-08, by hiding the build directory for the length of one run.** Swift's generated
resource accessor tried the path beside the executable and then fell back to **the build directory of the
machine that compiled it**. So on the build machine every binary worked wherever it was run, and the same
binary on any other machine did not degrade, it aborted. Worse, it aborted through a panic, so the
carefully written "the DDL is not where it should be" error never got the chance to report it.

**This exact mechanism does not exist in Rust**, where `include_str!` and `include_bytes!` put the DDL
inside the binary at compile time and there is nothing to find at runtime. **The lesson that does carry is
the test that was missing**: nothing ever ran the binary somewhere other than where it was built. Whatever
the packaging story becomes, something has to check it, because the build machine is the one place nobody
would catch it.

---

## Run every test on every platform that can run it

**Decided 2026-09-09, and the reasoning is the part worth keeping.** The tempting plan is to run the full
suite on the primary platform and only the platform-specific parts elsewhere. That plan was rejected,
because **the overlap is the product rather than the waste.**

Both platform divergences found up to that point lived in *shared* code that passed on one platform and
failed on the other: a timer scheduled on a run loop the second platform never ran, and the symlinked
directory above. Neither is a platform-specific test. **Running one test against two platforms asks two
different questions**, so coverage is a property of test × platform rather than of the test list.

**The corollary that bit once:** a test excluded from a platform stays excluded after the reason
disappears, because nothing asks it to leave. The Swift suite grew a test whose only job was to fail when
an exclusion had stopped earning its place. Some equivalent is wanted here.

---

## The date and timezone handling was the expected risk and cost nothing

**Measured 2026-09-07.** Porting the core to a second platform's standard library produced **not one**
date or timezone error, against an expectation that this would be the worst of it. The reason is a single
discipline applied throughout: every formatter is pinned to a fixed POSIX locale rather than the
machine's.

**Neither box's system timezone is canonical**, and both were measured saying so. Anything that compares
timezone identifiers has to normalise rather than assume; there are alias and lookup tables in the schema
(`012`, `013`, `502`, `503`) for exactly this.

---

## The tray icon is a different kind of object on each platform

**Measured 2026-09-21, from the three backends rather than from documentation.** The icon can be changed
at runtime everywhere, but what it is allowed to be differs, and the difference decides a design.

| | What `set_icon` does | Can it be wider than it is tall? |
|---|---|---|
| **macOS** | Sets the `NSStatusItem` button's image and re-measures the item | **Yes.** tray-icon asks for an 18 point tall image and derives the width from the aspect, so the item widens to fit |
| **Windows** | `Shell_NotifyIcon` with `NIF_ICON` | **No.** The icon is an `HICON` drawn into the shell's own square slot, so a 2:1 image is squashed rather than given room |
| **Linux** | Depends entirely on the backend, below | **Untested.** ksni's `icon_pixmap` carries its own width and height and the specification permits non-square |

**So a row of same-size glyphs is a macOS and Linux shape, and Windows needs another answer.** Facet shows
Play or Pause, with a padlock beside it at the same size when the cube is locked. On Windows both have to
fit inside one square, which halves them, or the second fact becomes a badge in a corner. **That is a real
decision for the Windows adapter and not a detail**, and it is here so it is not discovered by looking at
a squashed icon.

**tray-icon cannot be used for this on Linux at all.** Its Linux backend is libappindicator, whose
`set_icon` writes a fresh temporary PNG with an incrementing counter and repoints the icon theme path,
because AppIndicator takes an icon *name* rather than pixels. tray-icon offers no ksni backend: the
feature list is `gtk` plus `libappindicator` and nothing else. [`rust-port.md`](rust-port.md) had already
ruled AppIndicator out because it emits no click events, so Linux uses ksni directly, and ksni is the
richest of the three: raw pixels through `icon_pixmap`, and `overlay_icon_pixmap` for a second image drawn
on top of the first.

**A template image throws the colours away, and that is not a bug.** macOS draws a template in the menu
bar's own ink, black on a light bar and white on a dark one, so an icon built with colours in it comes out
black. `with_icon_as_template(false)` is what lets colours through, and the cost is that they stop
adapting: a white glyph is invisible on a light menu bar. Facet accepts that, having chosen its colours
deliberately.

---

## Three Slint layout facts, measured while drawing the Settings tabs

**Measured 2026-09-21 against Slint 1.18 and the cupertino style**, by rendering the tabs rather than by
reading source. All three were invisible from the `.slint` files and each produced a layout that looked
plausible until it was drawn.

**A wrapping `Text` in a layout is measured at one line.** Slint does not do height-for-width for text: a
`Text` with `wrap: word-wrap` reports the height of the sentence on one line, is given that height, and then
wraps to two inside it. The second line of the App tab's Debug footnote sat outside the panel it belongs to.
The fix is to set the width first and take the height from the result, which is what `Footnote` in
`crates/facet-ui/ui/widgets.slint` is a box around a text for:

```slint
Rectangle {
    height: label.preferred-height;
    label := Text { width: 100%; wrap: word-wrap; }
}
```

**A widget in a layout stretches unless it is told not to.** `SpinBox` and the rest fill the space a layout
gives them, so every stepped field on the App and Device tabs ran to the window edge with its number stranded
a long way from the words naming it. A control with a size of its own needs both `horizontal-stretch: 0` and
an explicit width, and the slack goes to a spacer.

**A disclosure triangle typed as a character renders as a speck.** The size and baseline of `▾` belong to the
font rather than to the layout, and at a heading's size it came out as a dot. Drawn as a `Path` with a
viewbox, it is the size it is asked to be.

**The renderer is the way to look at any of this.** `cargo run -p facet-mac --example draw-settings-tabs`
draws each tab through Slint's software renderer into `target/settings-tabs/<tab>.png`, with no window and no
menu bar, so a layout question does not cost a launch on the owner's screen. It is evidence about arrangement
rather than about appearance: the fonts are rasterised by Slint and not by the platform.

---

## Design rules that follow from all of the above

Short list, all of them enforceable from the first commit.

1. **Anything a check must press is a real button, or carries an accessibility action of its own.** A bare
   touch area is invisible to an accessibility press, and the driver script reports success and exits
   zero against one. That is a silent pass, which is the worst failure shape this project has a rule
   about. A guard belongs in `scripts/ax-press.py`.
2. **Every control a check must find carries both an accessibility id and an accessibility role**, because
   Slint refuses the id without the role.
3. **Tray items are addressed by label, on every platform.** No identifier survives the trip on Linux.
4. **Nothing lives behind a left click that has no menu equivalent.** Left click is an accelerator; on
   some Linux desktops the app never receives it.
5. **Anything a check needs to assert that the UI cannot report gets written to `debug_log`.** Colour is
   the standing example: it is unreadable from the tray on both platforms.
6. **Bootstrap fails loudly on an empty or missing DDL set.** Never a success with nothing applied.
