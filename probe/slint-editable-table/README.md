# Does an editable Slint table hold up?

**The experiment behind open questions 3 and 5 of [`docs/rust-port.md`](../../docs/rust-port.md), and both are
answered: yes.** Measured 2026-09-20 on macOS.

```sh
cargo run                          # with an Edit button per row
FACET_PROBE_NO_BUTTON=1 cargo run  # clicking the name is the only way in
```

Six rows whose clock column is rewritten by `set_row_data` **once a second on every row, including the row
being edited**. Every event is printed with the seconds since launch, so the transcript is the evidence rather
than whatever somebody remembers seeing.

## What it proved

**The one that could have ended the port**: a model update does not take the text out from under whoever is
typing. `CLAUDE.md` requires that, because re-reading a field mid-edit "clamps 1 on the way to 15".

```
15.65s  row 3: edit opened
17.83s  row 3: field now holds "XAdmin"
19.99s  row 3: field now holds "XYAdmin"
22.16s  row 3: field now holds "XYZAdmin"
```

Roughly two ticks between each keystroke, every one of them writing that row.

**The commit may be asynchronous**, which is the database rule on screen: Return does not change the name, the
table does.

```
5832.96s  row 3: Return pressed with "Well there"; the row still shows the stored name
5833.76s  row 3: the table now holds "Well there", so the row adopts it
```

**Everything `EditableNameCell` does**: click the name to open, Return commits, Escape abandons, a click
elsewhere abandons. Two need writing rather than coming free, and both are marked in `ui/table.slint`: a
`LineEdit` does not focus itself, and Escape is claimed by a `FocusScope`.

**It is drivable by `Tests/Scripted`'s own mechanisms.** `AXPress` by identifier opens the edit, writing
`AXValue` fills the field and fires Slint's `edited`, and a real `CGEvent` Return commits it. A bare
`TouchArea` is the exception: `AXPress` does nothing to one and reports success.

## What it cost to learn, which is the warning

**Three of the four problems first reported from this harness were artifacts of driving it with synthetic
clicks**: click-to-edit "not working", the caret opening at position 0 rather than where the click landed, and
a full-window `TouchArea` "swallowing" row clicks -- which led to that element being deleted on a theory that
was wrong. One click by hand settled all three in a minute.

**Measure the thing the app actually does**, not a convenient stand-in for it. `Tests/Scripted` does not use
coordinate clicks, so the mechanism that failed here was never the one that mattered.

## What it does not test

Sorting, a row leaving the list mid-edit, the icon grid, and any of this on Linux or Windows.
