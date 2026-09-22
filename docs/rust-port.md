# The Rust port

[← Back to README](../README.md) · [Port findings →](port-findings.md) · [BlueZ notes →](linux-bluez-port-notes.md) · [Architecture →](architecture.md) · [The Mac →](system-mac.md) · [The Linux box →](system-linux.md) · [TimeFlip BLE →](timeflip.md)

**The evaluation that decided Facet will be rewritten in Rust, and the requirements it was judged against.**
Written 2026-09-18. Nothing here has been built into the app: this is the record of a decision and of what was
established while making it, so that the work is not re-done when somebody picks it up.

**Every claim below is marked measured or untested.** Measured means it was run on this machine on the date
given. Untested means it comes from a vendor document, a crate's own platform notes or reasoning, and is
therefore a thing to confirm rather than a thing to build on. The distinction is the whole value of this file.

**The sequence is settled and the rewrite is not in flight.** The Linux port finishes in Swift first. The
conversion to Rust begins after it. That is a deliberate ordering, not a default: the Linux port is what
establishes the second platform's behaviour, and that knowledge is what the rewrite is then written against.

---

## The requirements

Stated by the owner, 2026-09-17 and 2026-09-18, and the thing every finding below is measured against.

1. **Three platforms**: macOS, Linux, and Windows. Windows is not yet started and is described as probable
   rather than certain, but it is a requirement of the language choice.
2. **The shipped app is completely self-contained.** The user installs no external programs, no runtime, no
   toolkit. **Build-time dependencies are unconstrained**: what it takes to develop the app does not matter.
3. **It looks, feels and operates the same on all three platforms, including the menu bar.**
4. **It does not need to look like a native Mac app.** This was stated explicitly after seeing the prototype,
   and it is what makes a single shared UI viable at all.
5. **The Linux target is MATE.** Not GNOME, not KDE. This narrows several answers below and improves one of
   them.
6. **The effort of changing language is not a constraint.**

Requirements 3 and 4 together are the unusual pair: uniformity matters, fidelity to any one platform does not.
That is the opposite of the priority the macOS app was built with, and it is why the conclusion differs from
the one the existing code implies.

---

## The decision, and the one fact behind it

**Rust, because of Bluetooth, not because of the UI.**

The expensive, error-prone, per-platform work in this app is the radio. macOS is CoreBluetooth, Linux is BlueZ
over D-Bus, and Windows would be a third implementation against WinRT. Every other concern (a window, a
database, an OAuth flow) has many cross-platform answers. The radio has almost none.

[`btleplug`](https://github.com/deviceplug/btleplug) is one async API over CoreBluetooth, BlueZ and WinRT,
host/central role only, no Bluetooth Classic, which is exactly this app's use. **Measured against the cube on
2026-09-20 and it does all of it**, which is the section below. It was the deciding factor and the single
largest unverified claim in this document, and it is neither of those any more.

The rest of the stack follows without difficulty, all **untested**:

- `rusqlite` with the `bundled` feature compiles SQLite into the binary, so the schema and DDL carry over
  unchanged and nothing is installed on the user's machine.
- `tray-icon` gives a real `NSStatusItem`, `Shell_NotifyIcon` and StatusNotifierItem respectively.
- `keyring` covers Keychain, Windows Credential Manager and Secret Service.
- One self-contained binary per platform.

### What was rejected

**Qt, on Bluetooth.** Qt's own documentation records that the Windows Bluetooth LE backend can only find
devices **already paired through Windows Settings**, and does not supply RSSI or manufacturer-specific data.
Facet reconnects by scanning, so that limitation lands directly on the app's central mechanism. Rejected on
that alone, without reaching the UI question. **Untested**, from
[Qt Bluetooth](https://doc.qt.io/qt-6/qtbluetooth-index.html).

**Staying in Swift.** More viable than it was: a [Swift Windows workgroup formed in January
2026](https://www.swift.org/blog/announcing-windows-workgroup/) and Swift 6.3 expanded Windows support. But
there is no cross-platform BLE library and no cross-platform UI in that ecosystem, so Windows costs a third
radio and a third UI. It is the largest total effort of the options considered, and it is the path the project
is currently on.

**Python and Go.** `bleak` is a genuinely good cross-platform BLE library, but Python's self-contained
packaging conflicts with requirement 2 and its desktop UI story is weak. Go's BLE libraries are markedly less
mature than `btleplug` for the central role. Neither was pursued.

---

## The radio, measured

**2026-09-20, against the cube, on macOS.** [`probe/timeflip-btleplug`](../probe/timeflip-btleplug/) is the
program, about 230 lines, and its README carries the transcript. Every mechanism this app's macOS driver
depends on, from one crate:

| Step | Result on FW_v3.64 |
|---|---|
| Unfiltered scan, matched as `DeviceScanRules.isEligible` does | found, rssi -65 |
| Connect and discover | 15 characteristics, 4 services |
| PIN on the password characteristic, with response | accepted (`02`) |
| Device Information and Battery, plain reads | model 2.0, FW_v3.64, 100% |
| Command channel: `0x07` read, `0x08` write, `0x07` read back | set and confirmed exactly |
| History `0x01 FF FF FF FF`, **read rather than notify** | `00 00 00 01 02 00 00 00 00 6A AF 80 53 00 00 00 16` |

That frame parses as event 1, face 2, started 1789886547, 22 seconds, duration **big-endian**.

**The scan is the part worth dwelling on**, because it is where Qt was rejected. A service-filtered scan found
nothing: this cube does not advertise its 128-bit UUID at all, which is why `BluetoothRadio` scans
`withServices: nil` and matches on service **or** name. `btleplug` does unfiltered scanning and hands over the
advertised name and services, so the app's own eligibility rule ports across unchanged. A backend that can
only reach already-paired devices, which is Qt on Windows, could not do this at all.

**Two firmware facts came out of the run** and are findings 12 and 13 of
[`timeflip2-firmware-observations.md`](timeflip2-firmware-observations.md): a factory-reset cube reports a
stale clock rather than an unset one, and the trailing bytes of an empty history frame carry that clock rather
than a duration. Both would have cost a session to rediscover, and the second was caught by this probe
misreading it first.

**What this does not say.** It was run on macOS only, so CoreBluetooth is the backend that has been exercised
and BlueZ and WinRT have not. That is a much smaller question than the one just closed -- the API is the same
and the Linux port has already proved the protocol works over BlueZ from Swift -- but it is not nothing, and
the honest statement is that one of three platforms is measured.

---

## The UI, measured

**A Slint prototype of the Settings window was built and run on this machine, 2026-09-18.** Five tabs, 640pt
wide, with every metric read out of `SettingsMetrics` rather than chosen, so that what was being compared was
the toolkit and not two people's taste in padding. Cupertino style, which is Slint's macOS imitation, chosen
so that any remaining difference is a real limit rather than a theme that was never trying.

**What transferred intact.** The layout rules survived: panels span the full tab width, the row rhythm reads
correctly, headings sit on their panel as the first row, and folds nest with their own defaults. Roughly 450
lines of markup reproduced the structure of the window faithfully. The `SettingsMetrics` work is portable.

**What did not.**

- **The tab bar.** Slint's `TabWidget` draws a full-width segmented strip welded to the top edge, where the
  app uses a centred pill group. That is the widget's structure rather than a theme setting, so matching it
  means reimplementing the tab bar.
- **Glyph fallback.** Category glyphs rendered as empty boxes. Slint performs its own font fallback and did
  not find them where AppKit would have. The real app uses SVG icons so this specific case does not arise, but
  it establishes that glyph fallback is now the toolkit's job and is weaker than the system's.
- **Controls are imitations.** The stepper is two chevron buttons rather than an `NSStepper`; pop-ups are
  bordered full-width fields rather than macOS pop-up buttons; primary buttons are a flat blue rather than the
  system accent treatment.
- **Type is heavier and larger** than AppKit at the same nominal size.
- **The window chrome is genuinely native** while its contents are not, so the seam sits at the window edge.

**Verdict against requirement 4: acceptable.** A Mac user would identify it as not a Mac app within seconds,
and that has been accepted.

### Styling is chosen at build time

**Untested**, from [Slint's documentation](https://docs.slint.dev/latest/docs/slint/reference/std-widgets/style/).
The widget style is fixed at compile time, not runtime. One build therefore looks identical on all three
platforms, which serves requirement 3 directly.

The `native` alias resolves per platform: `cupertino` on macOS, `fluent` on Windows, and on Linux `qt` **if Qt
is installed on the user's machine**, otherwise `fluent`. That Linux branch conflicts with requirement 2, so
Linux must pin an explicit style (`fluent` or `cosmic`) rather than letting `native` decide.

**The consequence worth holding on to:** choosing one style everywhere satisfies requirement 3 exactly.
Choosing `native` gives three appearances and partially abandons it. Given requirement 4, one style everywhere
is the right answer, and it is also the cheaper one.

---

## The tables, measured

**2026-09-20, on macOS.** [`probe/slint-editable-table`](../probe/slint-editable-table/) is a six-row list
whose clock column is rewritten by `set_row_data` **once a second on every row, including the row being
edited**, which is the Faces tab's Timing column against the Categories tab's rename. It reports every event to
stdout, so the transcript is the evidence.

**The question that could have ended this: does a model update take the text out from under whoever is
typing?** `CLAUDE.md` requires that it must not, because that "clamps 1 on the way to 15". It does not:

```
15.65s  row 3: edit opened
17.83s  row 3: field now holds "XAdmin"
19.99s  row 3: field now holds "XYAdmin"
22.16s  row 3: field now holds "XYZAdmin"
```

Roughly two ticks passed between each keystroke, every one of them calling `set_row_data` on that row. The text
accumulated and was never reset to the stored name.

**The commit may be asynchronous, which is the database rule expressed in UI.** Return does not change the
name on screen; the name changes when the table has it:

```
5832.96s  row 3: Return pressed with "Well there"; the row still shows the stored name
5833.76s  row 3: the table now holds "Well there", so the row adopts it
```

**Everything `EditableNameCell` does, Slint does**, confirmed by the owner driving it by hand: click the name
to open it, Return commits, Escape abandons, a click anywhere else abandons, and the stored name is untouched
by either. Two of those need writing rather than coming free, and neither is exotic: a `LineEdit` does not
focus itself, so an edit opens with `init => { field.focus(); }`, and Escape is claimed by a `FocusScope`
around the field, AppKit having needed its own workaround at the same spot for a different reason.

**Accessibility works, and that was not a given.** `accessible-id` arrives as `AXIdentifier` on macOS:
`scripts/ax-dump.py` shows `id=category-name-3`, `id=category-edit-3`, `id=probe-status`, and the open field as
`AXTextField id=category-name-field-3`. Per-row identifiers built by string concatenation inside a `for` come
through intact, so the locator model the scripted suite is written around converts rather than being reinvented.
One constraint found by hitting it: `accessible-id` is refused unless `accessible-role` is set alongside it.

**What this cost to find, and it is a warning about probing rather than about Slint.** Three of the four
problems first reported from this harness were artifacts of driving it with synthetic clicks: click-to-edit
"not working", the caret opening at position 0 rather than where the click landed, and a full-window
`TouchArea` "swallowing" row clicks, which led to it being deleted on a theory that was simply wrong. A human
click settled all three in one minute. **The residue is real and is now open question 3**: synthetic clicks do
not reach a Slint `TouchArea`, which is how `Tests/Scripted` drives everything.

**What was not tested**: sorting, a row leaving the list while it is being edited, the icon grid, and any of
this on Linux or Windows.

---

## Driving it from a script, measured

**2026-09-20, against the same harness, using `Tests/Scripted`'s own mechanisms rather than anything new.**

| Mechanism | The script that uses it | Result |
|---|---|---|
| `AXPress` by `AXIdentifier` | `scripts/ax-press.py` | **Works.** `pressed category-edit-3` and the edit opened |
| Writing `AXValue` | `scripts/ax-set.py` | **Works**, and fires Slint's own `edited` callback |
| A real `CGEvent` keystroke | `scripts/ax-key.py` | **Works.** Return committed, and the asynchronous write landed 800ms later |
| `AXPress` on a bare `TouchArea` | -- | **Does nothing, and reports success** |

**The correction this section exists to record.** It was concluded first that the driver layer would have to be
replaced, on the evidence that synthetic clicks did not reach a `TouchArea`. That conclusion was wrong, and it
was wrong because the probing was done with System Events `click at`, which **is not how this suite drives
anything**. `ax-press.py` performs an accessibility action and `ax-set.py` writes a property; neither goes near
a coordinate. Both work on Slint unchanged. The lesson is about measuring the thing the app actually does
rather than a convenient stand-in for it.

**The one design rule that follows**: anything a check must press has to be a `Button`, or carry an
accessibility action of its own. A bare `TouchArea` is invisible to `AXPress` -- and `ax-press.py` prints
`pressed` and exits 0 against one, which is a silent pass and exactly the shape `CLAUDE.md` has a section
about. A guard belongs in that script when the conversion happens.

**Two tooling changes, both one-liners.** `ax-set.py` hardcodes `pgrep -x Facet` and `ax-key.py` refuses to run
unless Facet is running, so neither takes `--app` the way `ax-press.py` does. Their mechanisms were reproduced
inline to measure this.

**What was not tested**: the status item, which is open question 4 and a different tree, and any of this on
Linux or Windows.

---

## The menu bar

**This is the only part of the app where requirement 3 cannot be fully met, and the reason is the operating
systems rather than the language.** All figures below are **untested**, from
[`tray-icon`'s platform notes](https://docs.rs/tray-icon/latest/tray_icon/struct.TrayIcon.html) and
[its event documentation](https://docs.rs/tray-icon/latest/tray_icon/enum.TrayIconEvent.html).

The status item itself is native everywhere, because the OS owns that strip and nothing can draw its own. So
unlike the Settings window, the menu bar is the real thing on each platform rather than an imitation.

| | Left click | Right click | Text beside the icon |
|---|---|---|---|
| **macOS** | App's event | App's event | Yes |
| **Windows** | App's event | App's event | **Never** |
| **Linux, KSNI backend** | App's event | Host shows the menu, app never sees it | Yes, if an icon is set too |
| **Linux, AppIndicator backend** | **No event at all** | Host shows the menu | Yes, **measured** |

**The Linux title column is not a guess.** `linux-port.md` on `feature/linuxPort` records that `XAyatanaLabel` on
`org.kde.StatusNotifierItem` answers what `StatusItemReadout` produced, measured on the Linux box 2026-09-13,
so the menu bar clock already works there and a scripted check can read it back. What it cannot carry is
colour, an AppIndicator label being plain text. See
[the tray is a D-Bus object](port-findings.md#the-linux-tray-is-a-d-bus-object-and-that-changes-how-checks-address-it).

Three consequences:

- **Windows can never show text in the tray.** `set_title` is documented "Windows: Unsupported", because
  `Shell_NotifyIcon` provides an icon and a hover tooltip and nothing else. The running category and elapsed
  clock cannot appear beside the icon there in any language. **The recommended answer is to put the elapsed
  time in the tooltip on Windows**, which is the nearest thing the platform offers.
- **Use the `ksni` backend on Linux, not `libappindicator`.** AppIndicator emits no click events whatsoever,
  which forecloses the design outright. `ksni` also drops GTK, `libxdo` and `libappindicator` as runtime
  dependencies, which serves requirement 2 at the same time.
- **On `ksni` the wanted shape falls out naturally.** Left click reaches the app, so pause and resume work;
  right click makes the host display the menu that was registered. The app does not need the right-click
  event, only the menu, and the host supplies it.

### MATE specifically

**Requirement 5 improves this answer, and MATE is the best Linux desktop for it.**

MATE's **Notification Area** applet implements *both* StatusNotifierItem over D-Bus *and* the legacy XEmbed
tray protocol. Most desktops dropped XEmbed, which is how they lost per-click control; MATE kept it. SNI
support is switchable through `org.mate.panel.enable-sni-support`.

The historical click bugs are largely closed: `mate-panel`
[#838](https://github.com/mate-desktop/mate-panel/issues/838) and
[#976](https://github.com/mate-desktop/mate-panel/issues/976) were closed in 2018 and 2019. The one still open,
[`mate-indicator-applet` #33](https://github.com/mate-desktop/mate-indicator-applet/issues/33), is a
**different applet**: the Indicator Applet, not the Notification Area.

### The menu bar on MATE, measured

**2026-09-18, on the Linux box, with the `ksni` backend.** A probe put an icon, a title and a three-item menu
in the tray and printed every event with its button and state. What came back:

```
23.53s  tray  Click { ... button: Left,   button_state: Up }
32.00s  tray  Click { ... button: Middle, button_state: Up }
53.25s  menu  Quit
59.27s  menu  Pause
64.67s  menu  Settings...
```

- **Left click reaches the app and opens no menu.** Confirmed by the owner watching the screen, which is the
  half the log cannot show: *the left click only produced that message, no menu appeared*. So the click is
  free, and pause and resume on left click is available on MATE.
- **Middle click reaches the app too**, which is a spare gesture if one is ever wanted.
- **Right click produced no event, and the menu opened.** The three `menu` lines are the owner choosing items
  from it. That is the documented KSNI split working exactly as wanted: the host owns right click and shows
  the menu the app registered, so the app never needs the event.
- **`rect` came back as zeros**, as the crate documents: the StatusNotifier protocol does not expose the
  icon rectangle.

**So the shape asked for is available on all three platforms**: left click for pause and resume, right click
for the menu. macOS and Windows give the app both clicks; MATE gives it the left one and handles the right
one itself, which amounts to the same behaviour by a different route.

**Which applet it ran in is now answered, and it is neither of the two this section had been weighing.**
Measured on the Linux box 2026-09-22: **`xapp-sn-watcher`**, from `libxapp1`, forwarding to the
**`mate-xapp-status-applet`** on the panel. Not the Notification Area applet and not the Indicator Applet.

**The route is forced rather than chosen, which is what makes the answer solid.** `ksni` publishes a
StatusNotifierItem on the session bus, so the host is whoever owns `org.kde.StatusNotifierWatcher`, and on
that box the sole owner is `xapp-sn-watcher`. The two candidates are both excluded for reasons that are
structural rather than circumstantial:

- **The Notification Area applet is not doing SNI at all**: `org.mate.panel enable-sni-support` reads
  **false** there. So it is the XEmbed half only, and an SNI item cannot reach it. The paragraph above is
  right that the applet can do both; the switch is simply off.
- **The Indicator Applet has never been on that panel.** It is installed, which is what the earlier reading
  of `linux-port.md` picked up, but installed is not running: it is absent from
  `/org/mate/panel/general/object-id-list`, absent from every object under it, and `dconf` holds no stale key
  mentioning it anywhere. **Installed and running are different questions, and the earlier note confused
  them.**

**So the open `mate-indicator-applet` #33 bug never applied to that measurement**, and the worry that the
result came from an applet that happened not to bite is retired.

**How far it travels is a different question, and the honest answer is narrower than "MATE".** The result is
about the XApp SNI watcher, which Linux Mint ships and plain MATE does not, so it generalises to Mint and to
anything else running `xapp-sn-watcher` rather than to every MATE install. A box with
`enable-sni-support` true, or with the Indicator Applet actually on the panel, is a different host and an
untested one.

**One caveat, stated rather than glossed.** This was measured four days after the probe, and the desktop
session began 2026-09-20, so these are not the same processes that hosted it. What carries across is the
panel configuration, which is persistent `dconf` state and holds no indicator applet and no trace of one,
and the structural point that an SNI item has nowhere else on that box to go.

**Measured 2026-09-18 and it works**, which the section above sets out. This paragraph used to say it was the
open question that mattered most, and it was; the probe that settled it is described there.

**The XEmbed route is closed from Rust.** It would guarantee full click control, but it needs `GtkStatusIcon`,
and [the GTK3 Rust bindings are unmaintained with RUSTSEC
advisories](https://fedoraproject.org/wiki/Changes/Retire_gtk3-rs,_gtk-rs-core_v0.18,_and_gtk4-rs_v0.7) while
GTK4 removed `StatusIcon` entirely. **Untested**, from those advisories.

### What requirement 3 actually gets

- **The Settings window**: identical on all three. Achievable.
- **The menu's contents and behaviour**: identical on all three. Achievable, provided the menu is the primary
  route to everything.
- **Left click as a shortcut**: macOS, Windows and MATE, all yes, the last measured 2026-09-18. Where it is
  unavailable on some other desktop the user loses a shortcut, not a capability.
- **Text beside the icon**: macOS and MATE yes, Windows never.

**The design rule that follows:** put Pause and Resume as the first item of the menu on every platform, and
treat left click as an accelerator for it rather than as the mechanism. Nothing may live behind left click
that has no menu equivalent, or Linux users lose a feature rather than a convenience.

---

## A finding that affects the current Swift Linux port

**This one matters before the rewrite, not after it**, which is why it is called out separately.

The Ayatana AppIndicator model is menu-only: it takes a menu and provides no click callbacks. **So the Linux
port as currently designed has the same left-click limitation described above**, and the divergence from the
macOS menu bar is already present rather than being something Rust would introduce. `linux-port.md` on `feature/linuxPort` records
the tray as a D-Bus object driven through `com.canonical.dbusmenu`, which is the menu, not the icon: **no
click behaviour has been measured on Linux at all**, and nothing in that file claims otherwise.

**MATE supports XEmbed, and the Swift port already calls GTK3 through a modulemap**, so `gtk_status_icon_new`
and its `activate` and `popup-menu` signals are reachable from the existing code today. That route needs no
new dependency: the toolkit decision already made is
one process, Swift calling GTK3 through a modulemap,
and it sidesteps the reason Rust cannot take the same route, which is that the *bindings* are unmaintained
rather than the C API being gone. `GtkStatusIcon` is deprecated in GTK3 and absent from GTK4, but MATE 1.26.1
is a GTK3 desktop.

**This is reasoning, not a measurement: it has not been tried, and the deprecation makes it a route with a
known end date.** It is recorded here because the option is open now and closes when the app leaves Swift.

---

## Open questions

Each of these is a thing to run, not a thing to think about further.

1. ~~**What MATE actually does with a left click.**~~ **Answered 2026-09-18: the design works**, and
   **2026-09-22 for the applet it ran in**: `xapp-sn-watcher` feeding `mate-xapp-status-applet`, which is
   neither of the two that question was between. See *The menu bar on MATE, measured* above. Nothing is left
   open here.
2. ~~**Whether `btleplug` can drive this cube.**~~ **Answered 2026-09-20: it can.** See *The radio, measured*
   below. This was the one that mattered and it is no longer open.
3. ~~**Driving a Slint window from a script.**~~ **Answered 2026-09-20: the suite's own mechanisms work.**
   See *Driving it from a script, measured* below. This entry read, for about an hour, that the driver layer
   needed replacing; that was wrong and the section says why.

4. ~~**The scripted suite's hold on the status item.**~~ **Answered 2026-09-21: the identifier can be
   set, and `scripts/status-item-click.py` works unchanged.** `tray-icon` exposes no API for it, which
   is what this entry said, but it does hand over the `NSStatusItem` through `ns_status_item()`. From
   there `button(mtm).setAccessibilityIdentifier(...)` sets `status-item`, which is the identifier that
   script already looks for, so the locator model converted rather than needing the Linux
   address-by-label pattern on macOS after all. `crates/facet-mac/src/main.rs` is the four lines.
   Measured by running the script against the built app: it found the item and clicked it.

   **Two things that came out of testing it.** `status-item-click.py --right` clicks the *right half*
   of a wide status item rather than posting a right button, which is a distinction the Swift app's
   two-halves status item needed and an icon-only item does not; a real right click needs a
   `kCGEventRightMouseDown`. And with `with_menu_on_left_click(false)` a left click reaches the app as
   an event while right click opens the menu, which is the split the design rule wants, confirmed on
   the real menu bar.

5. ~~**Editable tables in Slint.**~~ **Answered 2026-09-20: they work.** See *The tables, measured* below.
   What is still untested is sorting, a row leaving the list mid-edit, and the icon grid.

---

## The scratch work

**Two are committed**, being the ones that answered something:

- [`probe/timeflip-btleplug`](../probe/timeflip-btleplug/) -- the radio, open question 2.
- [`probe/slint-editable-table`](../probe/slint-editable-table/) -- the tables and the scripting, open
  questions 3 and 5.

**Two are not**, and were built on 2026-09-18 outside this repository:

- `~/temp/facet-ui-prototype` -- the Slint Settings window, all five tabs, working tab bar and folds, nothing
  behind any of it. It answered what the UI looks like, which the screenshots and *The UI, measured* record.
- `~/temp/tray-probe` -- the MATE click probe of open question 1. It answered its question on the Linux box
  and the transcript is in *The menu bar on MATE, measured*.

**The two uncommitted ones are throwaways and what they established is recorded above**, which is the part
that has to survive them. The two committed ones are kept because a measurement is worth more with the thing
that produced it beside it, and because both can be re-run.
