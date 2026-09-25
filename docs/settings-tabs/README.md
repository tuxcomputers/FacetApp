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
| `mac/` | **Owed.** [`handover-mac.md`](../handover-mac.md) item 4 asks for it |

**These are Slint's software renderer, not screenshots of the running app**, and that is deliberate
rather than a limitation. The renderer rasterises its own fonts and draws into a buffer with no window
server involved, so a difference between two folders is a difference in the *layout* rather than in how a
compositor or a font stack drew it. It is the comparison with the confounds removed.

**What that means they cannot answer**: how the real window looks. Font rasterisation and window chrome
are the platform's, and the likeliest difference between the two machines is fonts. Running the real app
on each is worth doing as well, and it needs somebody at the screen on both.

**They are a dated snapshot, not a golden file.** Nothing diffs them automatically and nothing should:
the point is a comparison made once, written up in
[`port-findings.md`](../port-findings.md), and then these stop being load-bearing. **Do not treat a stale
image here as evidence about a window that has changed since.** The date above is the date the answer is
about; re-render rather than trusting them if the `.slint` sources have moved on.

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
