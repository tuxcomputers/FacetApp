# The Settings window, drawn on each machine

**One folder per machine, the same six tabs, from the same command**, so the shared window can be
compared rather than asserted. This answers item 7 of
[`handover-linux.md`](../handover-linux.md): `facet-ui` makes a single shared UI *structurally* true, and
structurally true is not the same as true.

```sh
cargo run -p facet-ui --example draw-settings-tabs   # writes target/settings-tabs/
```

Then copy the six files into the folder for the machine you are on.

| | |
|---|---|
| [`linux/`](linux/) | Linux Mint 22.3, MATE 1.26.1, X11. Rendered 2026-09-25 at `cf58ebd` |
| [`mac/`](mac/) | macOS 26.6.2, Apple silicon. Rendered 2026-09-25 at `26b9f40`, **with Inter packaged** |
| [`compare/`](compare/) | One image per tab: Mac, Linux, and the pixels that differ between them. **Stale**: both halves predate Inter, and it is rebuilt once `linux/` is re-rendered |

**These are Slint's software renderer, not screenshots of the running app.** It draws into a buffer with
no window server involved, so window chrome and compositing are out of the comparison.

**Fonts are not.** The renderer rasterises the glyphs itself but takes the typeface from the system's font
stack, unless the UI packages its own, which it now does: `facet-ui` compiles Inter into the binary. So a difference between two folders is a difference in layout *or*
in the font each machine supplied, and `compare/` shows which.

**What they cannot answer**: how the real window looks. Window chrome is the platform's, so running the
real app on each is worth doing as well, and it needs somebody at the screen on both.

**They are a dated snapshot, not a golden file.** Nothing diffs them automatically and nothing should:
the point is a comparison made once, written up in
[`port-findings.md`](../port-findings.md), and then these stop being load-bearing. **Do not treat a stale
image here as evidence about a window that has changed since.** The date above is the date the answer is
about; re-render rather than trusting them if the `.slint` sources have moved on.

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
