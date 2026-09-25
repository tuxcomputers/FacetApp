# The Settings window, drawn on each machine

**One folder per machine, the same six tabs, from the same command**, so the shared window can be
compared rather than asserted. This answers item 7 of
[`handover-linux.md`](../handover-linux.md): `facet-ui` makes a single shared UI *structurally* true, and
structurally true is not the same as true.

```sh
scripts/switch-database.sh test -clean
FACET_DATABASE="<data directory>/appdata.sqlite" cargo run -p facet-ui --example draw-settings-tabs
```

Then copy the six files into the folder for the machine you are on.

**Each file is numbered for its place in the tab bar**, so every folder here lists in the order the window
reads rather than alphabetically:

| | | | | | |
|---|---|---|---|---|---|
| `1-faces.png` | `2-categories.png` | `3-report.png` | `4-app.png` | `5-device.png` | `6-about.png` |

**Keep it that way.** The number is the example's own tab index plus one, so it comes out of
`draw-settings-tabs` already prefixed and a copy needs no renaming. A tab that is added, removed or moved is
renumbered there, in `TABS`, and all three folders here are renamed to match in the same change, so the
same number means the same tab on both machines and in `compare/`. The example empties
`target/settings-tabs/` before it draws, so a renumbering cannot leave a file under its old name to be
copied in beside the new one. The data directory is
`~/Library/Application Support/Facet` on the Mac and `~/.local/share/Facet` on Linux.

**The Faces and Categories tabs are drawn from a freshly built test database**, so both machines render the same rows: the
seeded Break and Meeting, with their icons, and nothing being timed. Without `FACET_DATABASE` the tab draws
no categories at all, and a database with anything else in it draws something the other machine cannot
reproduce.

| | |
|---|---|
| [`linux/`](linux/) | Linux Mint 22.3, MATE 1.26.1, X11, `x86_64`. Rendered 2026-09-25 on `feature/catergoryTab` at `4f7690c`, against a clean test database. `2-categories.png` is the Categories tab read from the database; the other five came out byte-identical to the set before |
| [`mac/`](mac/) | macOS 26.6.2, Apple silicon, `arm64`. Rendered 2026-09-25 on `feature/catergoryTab` at `e5992ef`, against a clean test database. `2-categories.png` is the Categories tab read from the database; the other five are unchanged |
| [`compare/`](compare/) | One image per tab: Mac, Linux, and the pixels that differ between them. Rebuilt 2026-09-25 from Mac `e5992ef` and Linux `4f7690c`; every diff panel is empty, and all six PNGs are byte-identical across the two machines, `2-categories.png` with its icons, disabled checkbox and stepper included |

**These are Slint's software renderer, not screenshots of the running app.** It draws into a buffer with
no window server involved, so window chrome and compositing are out of the comparison.

**Fonts were not, until Inter was packaged.** The renderer rasterises the glyphs itself but took the
typeface from the system's font stack, which is what the first comparison caught. `facet-ui` now compiles
Inter into the binary, so both machines draw the same face and a difference between two folders can only
be a difference in layout.

**What they cannot answer**: how the real window looks. Window chrome is the platform's, so running the
real app on each is worth doing as well, and it needs somebody at the screen on both.

**They are a dated snapshot, not a golden file.** Nothing diffs them automatically and nothing should:
the point is a comparison made once, written up in
[`port-findings.md`](../port-findings.md), and then these stop being load-bearing. **Do not treat a stale
image here as evidence about a window that has changed since.** The date above is the date the answer is
about; re-render rather than trusting them if the `.slint` sources have moved on.

## The answer: identical, byte for byte

**Measured 2026-09-25, Mac `26b9f40` against Linux `759684b`, both with Inter packaged.** The two sets are
not merely alike:

| | |
|---|---|
| Pixels differing by any amount | **0**, on all six tabs |
| Greatest single-channel delta | **0** |
| MD5 of each PNG | **identical**, Mac and Linux |
| SHA-256 of the decoded RGB buffers | **identical** |

**Not one pixel of anti-aliasing separates them**, which is a stronger result than the comparison was set
up to detect: the `compare/` threshold exists to ignore edge noise and there is none to ignore. An
`arm64` Mac and an `x86_64` Linux box produced the same file.

**What that shows is that the shared UI is genuinely shared.** One set of `.slint` sources, one packaged
typeface and one software renderer produce the same drawing on both platforms, so requirement 4 in
[`rust-port.md`](../rust-port.md) is met in fact rather than only structurally. It also means a rendered
tab can be treated as a reference: a future change that moves the layout on one machine and not the other
shows up as a non-empty diff panel.

**Two things it still does not cover.** Window chrome, and the real window's font rasterisation, which is
the platform's rather than the renderer's. Both want the app open on each machine with somebody there.

## The comparison images

Each file in [`compare/`](compare/) is three panels side by side: the Mac render, the Linux render, and a
black image with every pixel where any channel differs by more than 32 drawn white. Built from the two
folders with Pillow:

```python
from PIL import Image, ImageChops
d = ImageChops.difference(mac, linux).point(lambda v: 255 if v > 32 else 0)
```

**Rebuild them whenever either folder is re-rendered**, since they are only as current as the older of
the two.

## What the Linux set showed

Measured off the pixels rather than eyeballed, 2026-09-25:

| | |
|---|---|
| Every tab | **640 x 680**, all six, so the width pins under MATE |
| Tab bar rule | y = **44**, all six |
| Panel box, App tab | x **22** to **618** |
| Stepper arrows, all three App rows | x **537-568**, identical to the pixel |
| Checkbox, Show seconds row | x **596-609** |

Nothing elided on any tab. The Report tab's two calendars came out identical to each other, so the
derivation from the day cell holds. The About tab carries the `AboutSlint` widget, which is a licence
condition and is visible here as well as in the accessibility tree.

## What the comparison showed

Measured off the pixels, Mac `2bc9721` against Linux `cf58ebd`, 2026-09-25.

**Every horizontal measurement matches to the pixel.** All six tabs are 640 x 680, the tab rule is at
y 44, the App panel runs x 22 to 618, the stepper arrows sit at x 537-568 on all three rows and the
checkbox at x 596-609, on both machines. Nothing is elided on either.

**The text does not match.** The Mac draws a Helvetica-style face; Linux draws a wider one that is also
taller. The height difference accumulates down a tab, so panel boxes finish lower on Linux:

| Tab | Last panel ends, Mac | Linux | Drift |
|---|---|---|---|
| Report | y 394 | y 412 | 18px |
| Device | y 506 | y 519 | 13px |
| Categories | y 380 | y 389 | 9px |
| App | y 445 | y 450 | 5px |

**So the layout is shared and the font is not.** The fix is packaging a font with `facet-ui`, after which
the two folders should come out identical.

**Inter is now packaged** (`26b9f40`) and `mac/` is re-rendered with it. Horizontal positions are
unchanged; the last panel on each tab now ends at Report y 405, Device y 514, Categories y 386 and App
y 448, between the two old sets. The measurements above describe the pre-Inter renders and stay as the
record of why the font was packaged.
