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
| **Linux** | Depends entirely on the backend, below | **Yes**, measured 2026-09-22. ksni's `icon_pixmap` carries its own width and height, and the host took 32x32 unlocked and 70x32 locked from the running app |

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

**The Linux half is no longer untested.** Facet was put on a real MATE panel on 2026-09-22 and the host
read back `IconPixmap` as 32 by 32 with one glyph and **70 by 32** with the padlock beside it, the icon
widening rather than being squashed or clipped. So the row-of-glyphs shape holds on two of the three
platforms and Windows remains the one that needs another answer.

**One conversion, and it is the kind that cannot fail loudly.** `facet_ui::status_icon::render` produces
RGBA, which macOS and Windows take unchanged. The StatusNotifier specification wants **ARGB32 in network
byte order**, so the alpha moves to the front of every pixel. Get it wrong and the icon still draws, in
the wrong colours, which reads as a drawing fault rather than a byte order one. `facet-linux` has three
tests on that conversion for exactly that reason.

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
draws each tab through Slint's software renderer into `target/settings-tabs/<n>-<tab>.png`, with no window and no
menu bar, so a layout question does not cost a launch on the owner's screen. It is evidence about arrangement
rather than about appearance: the fonts are rasterised by Slint and not by the platform.

---

## Slint already depends on ksni, and the two must agree on an async backend

**Measured 2026-09-22 on the Linux box, building `facet-linux`'s tray.** Taking `ksni` at its default and
adding it beside `slint` does not compile:

```
error: Features "tokio" and "async-io" cannot be enabled at the same time.
 --> ksni-0.3.6/src/compat.rs:5:1
```

**Nothing in this workspace asked for `async-io`. Slint did.** `slint` 1.18 has `system-tray` in its
**default** features, which enables `i-slint-core/system-tray`, which on
`cfg(unix, not(apple), not(android))` pulls this same `ksni` with `ksni/async-io` and `ksni/blocking`.
Cargo unifies features across the graph, so a second dependant asking for `ksni`'s default `tokio` turns
both async backends on at once and `ksni` refuses outright.

**It fails at compile time, which is the good case and worth saying.** The two backends are mutually
exclusive and the crate says so with a `compile_error!` rather than picking one, so this cannot ship as a
runtime fault. What it can do is look like a broken lockfile or a bad version pin, because the crate named
in the error is one the manifest does name and the feature in the error is one the manifest does not.

**Three things follow, and the third is the one that generalises:**

- **`facet-linux` takes `default-features = false` with `async-io` and `blocking`**, matching what Slint
  has already forced. `blocking` is the reason there is no async runtime and no tokio in the crate at all:
  `ksni::blocking::TrayMethods::spawn` is an ordinary call returning a handle.
- **The `default-features = false` lives in the workspace manifest, not the member**, because cargo refuses
  to let a member turn a workspace dependency's defaults off. The member names the two features it wants.
- **A dependency's default features are part of what a UI toolkit brings in**, and Slint brings in more
  than a renderer. Before adding any crate that a desktop shell might also provide -- a tray, a
  notification, a portal, a clipboard -- check whether Slint is already carrying it, because the collision
  arrives as a feature error naming a crate nobody thought they had two of.

**Not yet answered: whether Slint means to expose a tray of its own.** The feature is on by default and
`i-slint-core` holds the dependency, but nothing tray-shaped is re-exported from `slint`'s public API at
1.18. If a later version does expose one, it is worth a look: a tray that came from the same crate as the
window would serve requirement 4 better than one crate per platform. That is a thing to check, not a plan.

## A Slint window reaches the accessibility bus, but only once an AT is enabled

**Measured 2026-09-22 on the Linux box, with the real app on screen.** This was the most load-bearing
unknown for the scripted suite here, and the answer is yes with a condition that is the whole finding.

**With `toolkit-accessibility` false, Facet is not on the bus at all.** Twenty-seven applications were
listed and none of them was it. **Setting it true put the window on the bus immediately**, no restart, and
the tree is complete:

```
[application] 'facet-linux'
  [frame] 'Facet Settings'
    [page tab] 'Faces' ... 'Categories' ... 'Report' ... 'App' ... 'Device' ... 'About'
    [scroll pane] 'Faces'
      [push button] 'Play'   [list box] 'Categories'   [push button] 'Meeting' ...
```

**This is the opposite of the GTK3 result and both are measured.** A GTK3 app loaded the bridge anyway
with the setting false, 2026-09-08, which is why this file previously said the setting was not the gate it
looks like. **For Slint it is the gate.** AccessKit's Unix adapter stays dormant until an assistive
technology is active, which it learns from `org.a11y.Status`: `IsEnabled` and `ScreenReaderEnabled` were
both false, and `toolkit-accessibility` is what turns the first of them on.

**So the scripted suite must enable it before it drives anything**, and must put it back afterwards. A run
that forgets finds no application, and every check that presses a control fails identically to a window
that never opened, which is the wrong diagnosis for a missing gsetting.

**It is a real press, not just a visible tree.** `scripts/at-press.py --app facet-linux About` reported
`pressed 'About' (page tab)` and the app recorded `Settings tab selected: About` in `debug_log`. The
accessibility path is proven end to end rather than inferred from the tree being present.

**The About tab's Slint attribution is readable from the bus too**, which matters because its presence is
a licence condition rather than a design choice: `Made with Slint`, `#MadeWithSlint`, the version and the
licence sentence all come back as labels. A scripted check can assert the condition instead of it quietly
lapsing.

## Three driver faults, found the first time the suite met the Rust app

**All three on 2026-09-22, and none of them in the app.** Worth recording together because they share a
shape: each was written against something the Swift app did, and each failed in a way that pointed at the
app rather than at the driver.

1. **`tray-menu.py` looked for the wrong process.** It ran `pgrep -f FacetLinux`, which is the Swift
   binary; the Rust one is `facet-linux`, so it matched nothing and reported **facet is not running**
   against a running app with a visible tray icon. That is indistinguishable from the app having failed to
   start. Fixed to `pgrep -x facet-linux`, the spelling `Tests/Scripted/platform.sh` already uses.
2. **It used libappindicator's object paths.** `/org/ayatana/NotificationItem/facet` was the Swift app's
   AppIndicator layout; ksni publishes the item at `/StatusNotifierItem` and its menu at `/MenuBar`, and
   the old path answered `UnknownObject`. **Fixed by reading the `Menu` property off the item** rather
   than hardcoding a second guess, which is what the specification puts that property there for and what
   will survive the next backend.
3. **`at-press.py --tab` finds no notebook.** It answers *nothing in the tree matches a notebook*, because
   Slint presents `page tab` children directly under the frame where GTK presents a `page tab list`.
   **Not fixed**, because it is a choice rather than a repair: either `--tab` grows a Slint path, or checks
   press tabs by name, which already works and is what proved the item above.

**The first two failed loudly and exited non-zero**, which is the rule in CLAUDE.md doing its job: a
driver that had returned success against a tray it never found would have made the suite green on nothing.

## A locked secret store blocks rather than failing, and that is a design constraint

**Measured 2026-09-22 on the Linux box with the owner present**, by `probe/keyring-secret-service`. The
question was recorded as *a prompt to the user, or a D-Bus error if there is nobody to prompt*. **It is
neither.** The read simply does not return.

| | |
|---|---|
| `get_password` against a locked collection | **blocked indefinitely.** Killed at a 25s cap, exit 124 |
| What happened instead | a GNOME **Unlock Login Keyring** dialog appeared |
| When the caller was killed | **the dialog stayed on screen**, orphaned from the dead process |
| After unlocking | the secret read back intact; the lock cycle destroyed nothing |

**The code path explains it and would not have predicted the severity.** Every operation in
`zbus-secret-service-keyring-store` calls `ensure_unlocked` first; `secret-service`'s `ensure_unlocked`
runs `exec_prompt` and awaits it. There *is* a `ServiceError::Locked => no_access` mapping, which is what
makes reading the source reassuring, **but it is only reached once a prompt has been refused.** While one
is pending there is no error to map, and nothing times out on the app's behalf.

**Three things follow for the secret store port, and the third is the one that is easy to miss:**

- **Reading a stored secret needs its own timeout.** The store will not supply one. A read that can take
  unbounded time is not a read a launch can depend on.
- **It must not sit on the launch path.** A background Facet on a locked keyring hangs before it has a
  window or a tray to say so from, which is indistinguishable from a crash. The PIN is wanted when
  connecting to the cube, not when starting up, so this is a natural shape rather than a compromise.
- **A headless machine is the worst case and is the one CI runs on.** With no prompter there is nothing to
  dismiss and nothing on screen to explain the wait. Anything that reads a secret in a scripted run needs
  the collection unlocked first, the same way the accessibility checks need `toolkit-accessibility`.

**This is not a fault in `keyring` and would be the same through `keyring-core`**, the blocking being the
Secret Service's own prompt mechanism rather than a wrapper's choice. It is a property of the platform the
port has to hold rather than something a different crate avoids.

### macOS does the same thing, so these are rules for the port

**Measured 2026-09-22 on the Mac with the owner present**, by the same probe, and the answer is that the
platforms agree. This was written down as the question that decided whether the three rules above were a
Linux workaround or a constraint of the design. **They are a constraint of the design.**

| | macOS | Linux |
|---|---|---|
| `get_password` against a locked store | **blocked.** Killed at a 25s cap | **blocked.** Killed at a 25s cap |
| What appeared instead | a `SecurityAgent` password dialog | a GNOME **Unlock Login Keyring** dialog |
| When the caller was killed | **the dialog stayed on screen**, orphaned | **the dialog stayed on screen**, orphaned |
| Unlocked round trip | every check passes | every check passes |

**The item really was in the keychain that was locked**, which is worth stating because it is the
explanation that would have made this measurement worthless. `keyring` 4.2.0 on macOS could reasonably have
used the data protection keychain, which `security lock-keychain` does not touch;
`security find-generic-password -s facet-keyring-probe` found it in `login.keychain-db` by name, so the
lock and the read were about the same store.

**The first attempt at this measured nothing and looked like a clean pass**, which is the part worth
carrying. A `security show-keychain-info` run to confirm the lock had itself raised a password dialog, and
answering that dialog unlocked the keychain before the probe ever read from it. The read then returned in
one second with the right value. **Two commands that both look like inspection are not**: `show-keychain-info`
against a locked keychain prompts and can unlock it, so it cannot be used to check the state a test depends
on. What settled it instead was watching for the `SecurityAgent` process during the read, which needs
nobody to report what they saw.

## The shared Settings window renders identically on both platforms, once a font is packaged

**Measured 2026-09-25**, Mac `arm64` against Linux `x86_64`, both through Slint's software renderer via
`cargo run -p facet-ui --example draw-settings-tabs`. Images and method in
[`settings-tabs/`](settings-tabs/).

**The first comparison did not match, and the difference was entirely the font.** Every horizontal
measurement agreed to the pixel -- 640 x 680 tabs, the tab rule at y 44, the App panel at x 22 to 618,
the stepper arrows at x 537-568 on all three rows. What differed was the typeface: the software renderer
rasterises glyphs itself but **took the family from the host's font stack**, so the Mac drew a
Helvetica-style face and Linux drew Noto Sans, which is wider and taller. The height difference
accumulated down each tab and pushed the last panel up to **18px** lower on Linux.

**So the layout was shared and the font was not.** Packaging one fixed it: `facet-ui` compiles Inter 4.1
into the binary as four static weights, with `default-font-family: "Inter"` in `settings.slint`.

**After that the two sets are byte-identical:**

| | |
|---|---|
| Pixels differing by any amount | **0**, on all six tabs |
| MD5 of each PNG, Mac against Linux | **identical** |

**Not one pixel of anti-aliasing separates them**, which is stronger than the comparison was built to
detect: its diff threshold exists to ignore edge noise and there was none to ignore. Two different
instruction sets produced the same file.

**Three things follow:**

- **Requirement 4 is met in fact rather than structurally.** One set of `.slint` sources compiled into
  two composition roots really does draw the same window, which is the claim the choice of Rust and Slint
  rested on and had not been checked.
- **A UI toolkit that draws its own widgets still does not draw its own text.** Slint's software renderer
  is deterministic across architectures, and the one thing it delegated was the one thing that differed.
  **Anything wanting identical output has to package its typeface**, and this is the measurement that
  says so rather than an assumption about fonts.
- **A rendered tab is now usable as a reference.** A change that moves the layout on one machine and not
  the other shows up as a non-empty diff panel, which is a cheap check nobody had before.

**What it does not cover**, and neither is a small caveat: **window chrome**, which is the platform's,
and **the real window's font rasterisation**, which is fontconfig against Core Text rather than the
software renderer. Both need the app open on each machine with somebody at the screen. The packaged font
removes the *family* as a variable there too, but not the rasteriser.

## Four facts about driving a Slint window over AT-SPI, found converting the Faces checks

**Measured 2026-09-25 on the Linux box**, with the real app on screen, while `05`, `06` and `12` were brought
across from the Swift suite. Each one made a correct app look broken until the driver was taught about it.

1. **`accessible-id` arrives as AT-SPI `AccessibleId`, and the accessible name is the label.** AccessKit maps
   Slint's id to `author_id`, which `accesskit_unix` publishes as `AccessibleId`. GTK was the other way round:
   the Swift app overwrote the *name* with its identifier. `atspi_tree.by_name` now matches the id first and the
   name second, and `at-dump.py` prints `id=<AccessibleId>  value=<name>`, the same line shape as the macOS dump.
2. **A Slint text field cannot be written through the bus.** `accesskit_unix` 0.22 implements no
   EditableText, so `setTextContents` has nothing to call. `at-set.py` now activates the window with
   `wmctrl`, gives the field focus through its Component interface, types the string with XTEST and **reads the
   field back**, refusing if it does not hold what was typed. These are real keystrokes, which is the sharp case
   in CLAUDE.md, so a run needs the owner's hands off.
3. **A hidden Slint window stays on the accessibility bus whole.** Its frame reports `showing` and `visible`, its
   panes are all there, and its controls can be pressed: a run pressed Create on a Settings window nobody had
   opened, and the field behind it was then never found. `settings_is_open` asks the window manager instead
   (`wmctrl -l` lists only mapped windows). Only `active` differs, and that is focus, not presence.
4. **`accessible-enabled: false` does not reach AT-SPI.** The Faces tab's play/pause glyph draws greyed while a
   daily limit is spent, and the tray computes the same answer and shows its item insensitive, but AT-SPI reports
   the glyph `enabled` and `sensitive`. Slint 1.18's bridge does call `set_disabled` for the property, so where it
   is lost is not yet known. `12` asserts the refusal by pressing the glyph instead, which is the stronger check
   anyway: the tab ignores the press, so no click row and no segment appear.

**An element revealed by a press is absent for a moment**, because AccessKit publishes a tree change after the
frame that made it. `require` in `atspi_tree.py` now waits up to five seconds rather than asking once.

## The same window over macOS accessibility, found running the Faces checks on the Mac

**Measured 2026-09-25 on the Mac**, driving `facet-mac` while `00`, `05`, `06` and `12` were run there for the
first time. The suite passed 71 of 71 once the harness was taught these.

1. **A hidden Slint window leaves the macOS accessibility tree entirely**, the opposite of Linux fact 3 above.
   With Settings closed the app reports no windows at all, so on macOS the tree is the answer: `settings_is_open`
   asks for the Faces tab button, and `close_settings` presses the title bar's close button through AX
   (`ax-press.py --close-window`).
2. **`accessible-enabled: false` does reach macOS accessibility.** With the daily limit spent, `ax-dump.py`
   prints the Faces tab's play/pause glyph as `disabled`. Slint's side therefore works, and the loss in Linux
   fact 4 is between AccessKit's tree and AT-SPI, in `accesskit_unix` or the bridge after it.
3. **Every tray-icon menu item carries the same `AXIdentifier`, `fireMenuItemAction:`.** The ids the app gives
   its items (`pause`, `settings`, `quit`) do not reach AX, so the suite presses menu items by title on macOS as
   it does on Linux. The items take an AX press with the menu closed, so reaching the menu needs no mouse event.
4. **Straight after launch the menu is not in the tree yet.** A press in the first moment finds no item; a few
   hundred milliseconds later it does. The macOS menu press asks again for up to five seconds.
5. **A real left click on the status item is one toggle.** macOS delivers a press and a release, the app logs
   both, and only the release acts: one segment closed per click.
6. **With no bundle, macOS names the app after the binary**, `facet-mac`, so every script that found the app
   as `Facet` found nothing. The scripts read `FACET_APP_NAME`, which `platform.sh` sets.

**The Linux tray hang below has not been seen on the Mac** in about eight launches that day, which is too few to
rule out a rate of one in fifteen but is a different tray stack (tray-icon, not ksni) in any case.

## The Linux tray stops answering D-Bus on about one launch in fifteen

**Measured 2026-09-25, and not yet understood.** On a launch that goes wrong, every call to the item or its menu
(`Introspect`, `GetLayout`, `Activate`) gets `NoReply` after the 25 second timeout, for as long as the process
lives. The event loop is still running (the main thread is in `ep_poll`) and the trace shows the launch completing
normally, including the tray's first push of the clock.

| | |
|---|---|
| This branch, accessibility on | 1 of 10 launches |
| This branch, accessibility off | 1 of 15 |
| `87e7d2e`, before the tray followed the clock | 1 of 20 |

**So it predates the tray following the clock and is not AccessKit.** It lives in the ksni service or its zbus
connection. For the suite it means a run fails at its first tray press about once in fifteen, with the tray
reported unreachable. That is the right diagnosis, but it is the app's fault, not the driver's.

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
