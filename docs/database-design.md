# Database Design

[← Back to README](../README.md)

This document describes the schema used to persist TimeFlip data locally. DDL files for each table live in [`crates/facet-core/resources/database/`](../crates/facet-core/resources/database), numbered in the order they should be applied. See [`its CLAUDE.md`](../crates/facet-core/resources/database/CLAUDE.md) for the storage conventions referenced below.

## Rule: sections follow the DDL numbering

The `## Tables` sections below appear in the **same order as the tables are created** — `001`, then `002`, and so on — not grouped by topic or importance. When a table is added or renumbered, its section moves to match.

This isn't housekeeping. Foreign keys are enforced, so the numbering already guarantees a table is created after everything it references (see the design principle below). Mirroring that order here means **every foreign key a section mentions points at a table already described above it** — the document reads top to bottom without forward references, and each `Foreign keys` list can say "described above" and be reliably true.

## Design principle: decoded, not raw

The TimeFlip device reports events over Bluetooth as raw hex payloads (e.g. a face byte, a big-endian duration, a status flag). This database never stores those raw bytes directly — every column holds the *decoded, human-readable* value instead. For example, the device's raw face byte is converted to a plain face number (`1`-`12`) before it's written to the `face` column, so the table can be read and reasoned about directly (in a SQLite browser, in `sqlite3`, etc.) without needing to know the device's wire format.

## Design principle: local time + timezone

Date/time columns store **local time**, not UTC. The timestamp itself omits any UTC offset/`Z` suffix (e.g. `2026-07-16T09:30:00`) so it always reads the same as the wall-clock time at the moment it was recorded, regardless of the reader's timezone. The IANA zone that local time was captured in — needed to recover the offset — is held in a **foreign key to the `timezone` table**, not repeated as inline text on every row: a single `timezone_id` column when the table has one timestamp (e.g. `device_event.timezone_id`), or short per-timestamp `<prefix>_timezone_id` columns when it has several (e.g. `time_entry.start_timezone_id` / `end_timezone_id` for `started_at` / `ended_at`). The app resolves the current zone's id by get-or-create (`TimezoneStore.currentID()`) and binds it into each row. These columns are `NOT NULL DEFAULT 0`, and `timezone` is seeded with an id-`0` `Unknown` row, so the FK is always satisfiable — a lookup that fails falls back to that sentinel rather than a null or a dangling id.

## Design principle: foreign keys enforced

The app opens every database connection with `PRAGMA foreign_keys = ON` (SQLite defaults this OFF, and it's a per-connection setting, not stored in the file — see `DatabaseConnection`). So the `REFERENCES` clauses in the schema are real, enforced constraints, not just documentation: an insert or update with a dangling foreign key fails. This keeps local behaviour aligned with the eventual remote server (which enforces them) and lets the same DDL be reused there.

Because enforcement is on, the DDL files are numbered so a **parent table is created and seeded before any table that references it** — e.g. `004_icon`, `005_colour`, and `006_project` all precede `007_category`. Inserting a new table therefore follows the renumber rule in [`its CLAUDE.md`](../crates/facet-core/resources/database/CLAUDE.md).

## Tables

### `event_type` (`database/001_event_type.sql`)

Reference table of the different event types the TimeFlip device can trigger. Most of these (`double_tap`, `battery_level`, `system_state`, `device_info`, `event_log`) are live BLE notifications the device sends outside the history stream, not timing segments, and so never appear in `device_event` — only `face_flip` and `pause` come from the history stream that populates `device_event` (see `TimeFlipEvent.swift` and `docs/timeflip.md` §4-5 for the full notification/history breakdown).

| Column           | Type    | Description                                              |
|-------------------|---------|-------------------------------------------------------------|
| `event_type_id`   | INTEGER | Primary key. Not autoincrementing — seeded with fixed IDs.  |
| `event_name`      | TEXT    | Event type name, e.g. `face_flip`, `pause`, `double_tap`.   |

Constraints:
- `event_name` is `UNIQUE` — each event type is only represented by one row.
- Seeded with all known device event types, with the ids **grouped by which table an event of that type lands in** and a blank line between the groups in the DDL:

  | IDs   | Group                                                | Types                                                                            |
  |-------|------------------------------------------------------|----------------------------------------------------------------------------------|
  | `1`-`2` | → `device_event` — timing segments, carry a duration | `face_flip`, `pause`                                                            |
  | `3`-`8` | → `device_notification` — point-in-time, no duration | `double_tap`, `auto_pause_minutes`, `battery_level`, `system_state`, `device_info`, `event_log` |

  The grouping is a convention of the seed order, not something the schema enforces — nothing stops a row in either table referencing an id from the other group. It exists so the id alone tells you where an event of that type belongs, which is why a **new event type is appended within its matching group rather than interleaved** (see `database/CLAUDE.md` § Seed inserts). Which table a given event actually lands in is decided by the classification step in [Operation Spec § 1](operation-spec.md).

### `timezone` (`database/002_timezone.sql`)

Reference table of IANA time zones. Every date/time table references it by id (see the "local time + timezone" design principle) instead of repeating the identifier string on every row. The app resolves the current zone's id through the `timezone_lookup` view (`TimezoneStore.currentID()`). It is numbered `002` so it precedes every table that references it (foreign keys are enforced).

**Seeded with every zone, at ids that are the same in every Facet database.** 447 zones from tzdb **2026c**, in alphabetical order at ids `1`-`447`, plus the `Unknown` sentinel at `0`. So `timezone_id 116` is `America/Havana` here, on the next machine, and in the `debug.sqlite` beside it. Before this the rows were created in visit order, which made an id mean whatever the file it sat in had happened to see first: measured 2026-09-07 on the Mac, `timezone_id 1` was `Australia/Brisbane` in `production.sqlite` and `AEST` in `test.sqlite`.

**The ids are fixed and are never renumbered.** A zone added by a later tzdb release is appended above the block rather than slotted into alphabetical order, because renumbering would silently change what every stored `timezone_id` means. Alphabetical order is how the first 447 were assigned, not a property the table maintains.

Legacy names (`Cuba`, `US/Pacific`) are **not** in this table: they live in [`timezone_alias`](#timezone_alias-database012_timezone_aliassql) and are resolved through [`timezone_lookup`](#timezone_lookup-database013_timezone_lookupsql-a-view), both documented at their numbered position below.

**Editing this file edits the debug database's schema too.** `database/500_timezone.sql` is a symlink to it, so `debug.sqlite` is created from these same statements — see [the debug database's `timezone`](#timezone-database500_timezonesql-a-symlink-to-002_timezonesql) for why it needs its own table and why the two sets of rows may never be joined. A column added here appears in both, which is the point; anything that should reach only one of them cannot go in this file at all, and would mean giving `500` a real file back.

| Column          | Type    | Description                                                        |
|-----------------|---------|--------------------------------------------------------------------|
| `timezone_id`   | INTEGER | Row identifier, primary key. **Set explicitly by every seed row**; `AUTOINCREMENT` remains so that a zone the seed does not know lands *above* the seeded block (448 and up) rather than in a gap a later release would claim. |
| `timezone_name` | TEXT    | IANA time zone identifier (e.g. `Australia/Sydney`). `NOT NULL`, `UNIQUE`. |
| `display_name`  | TEXT    | Optional human-friendly label for a picker (e.g. `Sydney`). Nullable. |
| `active`     | INTEGER | `1` if the zone should be offered in a picker, `0` to hide it. `NOT NULL`, defaults to `1`. **Every seeded zone is `1`**, including the non-geographic ones (`Etc/*`, `Factory`, `EST5EDT`); whether a picker should offer those is a decision for whoever builds the picker, and setting them `0` costs nothing then because the ids stay either way. Legacy names are not hidden here -- they are not in this table at all, see `timezone_alias`. |

Constraints:
- `timezone_name` is `NOT NULL` and `UNIQUE` (`UN1_timezone`), so a zone can be looked up by identifier and never stored twice. **This index is also what makes the static seeding safe against a database that predates it**: applying the seeds to a file whose ids were assigned at runtime fails on the first zone already present by name, loudly, rather than skipping a guarded insert and leaving that file's id meaning something no other file agrees with.
- `active` is constrained to `0`/`1` (SQLite has no native boolean type) and defaults to `1`.

The `Unknown` sentinel at `timezone_id 0` is the value every referencing `timezone_id` column defaults to, so a row can satisfy its foreign key before a real zone has been resolved, and `TimezoneStore` falls back to it when the lookup answers nothing. `display_name` is left null for the seeded zones: the identifier is what a reader can trust, and a label per zone would be 447 pieces of curation nobody has asked for yet.

The app still writes this table in one case, so it is **not** a reference table under the `CLAUDE.md` exception: a name the seed has never heard of, which means a tzdb release newer than the seed. `TimezoneStore.id(for:)` records it rather than filing the row under `Unknown`, and a row above `447` is the signal to add that zone to this file.

Deliberately has **no** UTC-offset column: an offset varies with DST within the same zone, so storing a fixed one would be misleading — the offset is derived from the IANA identifier at read time instead.

### `device_event` (`database/003_device_event.sql`)

One row per device-reported timing segment — created whenever the device is flipped to a new face or paused/resumed, marking the end of the previous segment.

| Column             | Type    | Description                                                                 |
|---------------------|---------|-------------------------------------------------------------------------------|
| `device_event_id`  | INTEGER | Row identifier, primary key, autoincrementing (`PK_device_event`).          |
| `event_number`      | INTEGER | The device's own sequence number for this event. Part of the composite matching key with `start_epoch` — see below — but not unique on its own, and not used for ordering. |
| `event_type_id`     | INTEGER | References `event_type.event_type_id` — always `face_flip` or `pause` for rows in this table. |
| `device_face`       | INTEGER | Decoded face number, `1`-`14`. Decoded from the device's raw face byte, not stored as hex. `13` and `14` are not faces of any cube: they are the two the app owns and uses **in rotation** when it is doing the timing, so a hand-timed segment resolves its category through the same join as every other segment (see `face` below). Two rather than one, because a face's category is what says whose time a segment was, so consecutive segments sharing a face would let a reassignment change the answer under a finished segment still awaiting its `time_entry`. A cube never has this problem: flipping from one face to another leaves the first one's mapping alone. |
| `start_time`        | TEXT    | When the segment started, as a local-time ISO 8601 timestamp with no UTC offset (e.g. `2026-07-16T09:30:00`). Decoded from the device's raw timestamp encoding. Display only — see `start_epoch` for ordering/comparisons. |
| `timezone_id`       | INTEGER | References `timezone.timezone_id` — the IANA zone (e.g. `America/New_York`) `start_time` was recorded in. |
| `start_epoch`       | INTEGER | The same moment as `start_time`, as Unix epoch seconds. This — not `event_number` — is what `DeviceEventRules` compares to decide ordering and the `finalised` flag; also half of the composite matching key (see below). Indexed. |
| `duration_seconds`  | REAL    | How long the segment lasted, in seconds.                                    |
| `paused`         | INTEGER | `1` if this segment was a paused interval, `0` otherwise.                   |
| `finalised`         | INTEGER | `1` once the segment is closed out, `0` while it's still the device's in-progress interval. |
| `processed`         | INTEGER | `1` once the conversion has dealt with this segment: either it became a `time_entry`, or it was skipped as shorter than `blip_time`. `0` otherwise, which includes every paused segment, since a pause is never something the conversion deals with. |

Foreign keys:
- The `event_type_id` column references the PK of the table `event_type` described above. `NOT NULL`. Always `face_flip` or `pause` for rows in this table.
- The `timezone_id` column references the PK of the table `timezone` described above. `NOT NULL DEFAULT 0` — id `0` is the seeded `Unknown` sentinel.

Constraints:
- `(event_number, start_epoch)` has a composite `UNIQUE` index (`UN1_device_event`) — see below for why it's the pair, not `event_number` alone, that's unique.
- `device_face` is constrained to `1`-`14`: the cube's twelve, plus the app's own `13` and `14` (`ManualFace.all`). Nothing above 12 ever comes from a device, so a BLE path that decodes a face byte still treats anything past 12 as a corrupt frame; the wider bound guards the app's own side only.
- `duration_seconds` is constrained to be non-negative.
- `paused` is constrained to `0`/`1` (SQLite has no native boolean type).
- `finalised` is constrained to `0`/`1` (SQLite has no native boolean type) and defaults to `0`.
- `processed` is constrained to `0`/`1` (SQLite has no native boolean type) and defaults to `0`.
- `start_epoch` also has its own non-unique index (`IN1_device_event`) for ordering queries that don't also filter on `event_number`.

Why matching and ordering are both keyed off `start_epoch`, and neither trusts `event_number` alone:
- **Ordering** ("is this new segment newer than anything recorded so far?") compares `start_epoch` against the newest row the table holds (`DeviceEventMark`, read from `device_event` rather than accumulated in memory). `event_number` is a counter maintained on the device itself — a device-side reset (a battery pull, or a reset from the official app; confirmed happening in practice: a real device's `event_number` sequence jumped from `139` straight back to `1` after an official-app reset) can make it restart from a low number while this table already holds higher `event_number` values from before the reset. Comparing `event_number` magnitudes directly would treat that brand-new event as *older* than history it's actually superseding. `start_epoch` is derived from the device's own timestamp and doesn't reset, so it's safe to compare directly.
- **Matching** ("have I already recorded this exact segment?", used to decide update-in-place vs. insert) uses the composite `(event_number, start_epoch)` pair, not `event_number` alone: after a reset, `event_number` gets reused for a completely different real-world segment, so a bare `UNIQUE` on `event_number` would either block that new segment from ever being inserted, or (if matched on `event_number` alone in the `UPDATE`) silently overwrite the unrelated old row. `start_epoch` alone isn't unique enough to use by itself either — the device only reports whole-second timestamps (`docs/TimeFlip2 BLE Protocol v4.3.md`'s `0x07`/`0x08` commands and the history frame's flip-timestamp field are both "number of seconds", no finer resolution), so two genuinely different segments (e.g. a quick flip across a face while searching for the right one — see the `blip_time` setting) can legitimately share the same `start_epoch` second. The combination of both is what's actually unique: the only way two different real segments collide on `(event_number, start_epoch)` is an exact coincidence of a device reset landing the reused `event_number` in the very same wall-clock second as the old segment it collides with — vanishingly unlikely in practice.

`finalised` vs. `processed`: the device's history stream always reports its still-open, in-progress segment as the last frame in every dump (see `docs/timeflip.md` §5). That frame is inserted with `finalised = 0` and its row is updated in place (matched by `(event_number, start_epoch)`) as the duration keeps growing on each refresh, until a subsequent flip/pause closes it out and a later write sets `finalised = 1`. `processed` is a separate, independent flag — it tracks whether a (finalised) segment has been turned into a `time_entry` yet, and is only ever meaningful once `finalised = 1`; the `finalised` update path never touches it, so an already-`processed` row can't be silently un-flagged by the live segment's growth.

### `icon` (`database/004_icon.sql`)

Reference table of activity icons that can be assigned to a face.

| Column     | Type    | Description                                                                                   |
|------------|---------|-------------------------------------------------------------------------------------------------|
| `icon_id`  | INTEGER | Row identifier, primary key, autoincrementing.                                                  |
| `icon_name`| TEXT    | Identifier the app uses to locate the icon asset (see `ActivityIconLoader`), e.g. `"briefcase"`. |

Constraints:
- `icon_name` is `UNIQUE` — each icon asset is only represented by one row.
- Seeded with a `None` row (`icon_id = 0`) representing "no icon assigned", alongside the real icon assets (`icon_id` 1-42) — so `category.icon_id` can stay a `NOT NULL` foreign key instead of allowing `NULL`.

**This table is the only say in which icons exist.** Adding one is a row here plus its SVG under `Resources/Icons/Activities`, and no code change: `ActivityLibrary.iconOptions(from:)` offers every row but the `None` sentinel, in this table's order. A hardcoded 42-name Swift array used to filter it, so a row the array didn't list vanished from the grid with nothing said; it is gone. What replaced it is a complaint rather than a filter — `ActivityLibrary.reportUnresolvableIcons` logs any row whose SVG will not load, at launch, under the `icons` tag, and the row is still offered so the failure shows up as a placeholder glyph in the grid rather than as a row that silently does not exist. `IconPaletteTests` asserts every row this DDL seeds resolves to bundled artwork.

### `colour` (`database/005_colour.sql`)

Reference table of the colours available to assign to a category.

| Column       | Type    | Description                                              |
|--------------|---------|--------------------------------------------------------------|
| `colour_id`   | INTEGER | Primary key. Not autoincrementing — seeded with fixed IDs.   |
| `colour_name` | TEXT    | Colour name, e.g. `Red`, `Teal`, `Cyan`.                      |
| `device_hex`  | TEXT    | The RGB value shown on the device for this colour, as an `#rrggbb` hex string (`NULL` for `None`). This is the value sent to the tracker's face-colour command (`0x11`, which takes 16-bit R/G/B — see the BLE protocol doc); the app scales each 8-bit channel up when sending. Stored here — rather than derived from an AppKit system colour — so each named colour maps to a fixed, predictable value on the LED. |
| `white_lines` | INTEGER | `1` when the device drawn in this colour should take white inner lines and a white icon, `0` for black ones. `NOT NULL`, defaults to `0`. |

Constraints:
- `colour_name` is `UNIQUE` — each colour is only represented by one row.
- `white_lines` is constrained to `0`/`1` (SQLite has no native boolean type) and defaults to `0`. It is seeded `1` for `Maroon`, `Brown`, `Green`, `Teal`, `Blue`, `Navy` and `Purple`: the seven seeded colours dark enough that black lines disappear into them, by WCAG relative luminance (those below the ~0.179 crossover where white contrasts better than black). Held as data rather than computed from `device_hex` so the choice can be retuned by editing the row. Only the drawn device's **inner** lines and centre icon follow it; the outer outline stays black whatever the face is lit in, so the shape still reads against the window behind it.
- Seeded with a `None` row (`colour_id = 0`, `device_hex` `NULL`) representing "no colour assigned" — so `category.colour_id` can stay a `NOT NULL` foreign key instead of allowing `NULL` — alongside 20 named colours (`colour_id` 1-20).
- The 20 named colours are the categories listed on [html-color.codes](https://html-color.codes), each seeded with the **first** (canonical) colour of its category as `device_hex`: `Red` (`#ff0000`), `Maroon`, `Brown`, `Tan`, `Orange`, `Peach`, `Gold`, `Yellow`, `Lime`, `Olive`, `Green`, `Teal`, `Cyan`, `Blue`, `Navy`, `Purple`, `Magenta`, `Pink` (`#ffc0cb`), `Grey`, `Silver`. `White` and `Black` are deliberately excluded — the device's LED can't render either (black is just off; white isn't reproducible). Nothing reads `device_hex` yet — it's groundwork for matching calendar-entry colours to the device LED.

### `project` (`database/006_project.sql`)

A named project. Id and name only for now — groundwork for a planned projects feature. Numbered `006_*`, before `category`, so it's created and seeded before the tables that reference it — which matters now that foreign keys are enforced (see the design principle above).

| Column         | Type    | Description                                              |
|----------------|---------|----------------------------------------------------------|
| `project_id`   | INTEGER | Row identifier, primary key, autoincrementing.           |
| `project_name` | TEXT    | Project name. `NOT NULL`.                                |

Constraints:
- `project_name` is `NOT NULL`.
- Seeded with a `None` row pinned to `project_id = 0` — a fixed sentinel for "no project assigned", the same id-0 convention used by `category` (`Unassigned`) and `colour` (`None`), so `category.project_id` can stay `NOT NULL` and default to `0` instead of allowing `NULL`.

### `category` (`database/007_category.sql`)

Named activity category, linked to the icon and colour assigned to it.

| Column       | Type    | Description                                              |
|--------------|---------|------------------------------------------------------------|
| `category_id`  | INTEGER | Row identifier, primary key, autoincrementing.             |
| `category_name`| TEXT    | Category name (e.g. an activity mapped to a face). At most **35 characters**, by `CHECK`. Long enough for anything a person calls an activity, short enough that the Faces tab can draw it at a readable size: the name is the largest text in that column, and an unbounded one made the Settings window as wide as whatever somebody typed. Both fields that take a name hold to the same limit as it is typed (`CategoryCreateRules.maximumLength`), so the constraint is never what a person meets. |
| `icon_id`    | INTEGER | References `icon.icon_id` — the icon assigned to this category. Use `0` (the seeded `None` icon) if no real icon is assigned. |
| `colour_id`  | INTEGER | References `colour.colour_id` — the colour assigned to this category. Use `0` (the seeded `None` colour) if no real colour is assigned. |
| `project_id` | INTEGER | References `project.project_id` — the project this category belongs to. Use `0` (the seeded `None` project) if no project is assigned. |
| `daily_limit`| INTEGER | Whole minutes of tracked time allowed against this category per day (`0` = no limit). Minutes rather than the seconds convention used by `duration_seconds` elsewhere (e.g. `time_entry`): a limit is a coarse user-set budget, and nobody sets one to the nearest 30 seconds. The day boundary is the `setting` table's `daily_reset_time`, not midnight. `NOT NULL`, defaults to `0`. |
| `cost`       | INTEGER | What an hour of this category costs, as a whole number of **cents** (e.g. `250` = \$2.50 per hour) to avoid floating-point money; the UI formats it for display as `$x.xx`. **A rate per hour, not a flat charge per entry** — so a `time_entry`'s `total_cost` is a function of its duration. `NOT NULL`, defaults to `0`. Nothing reads or sets it yet — groundwork for Cost time entry (reasoning in `~/harry.git/TimeFlipLinux/docs/TODO-features-under-development.md`). |
| `active`     | INTEGER | `1` while the category is in use, `0` once it has been retired. Retiring is an `UPDATE`, never a `DELETE`, so historical `time_entry` rows keep resolving; the category simply stops being offered for new assignments. It is also taken **off every face holding it** (`CategoryStore.setActive`), since a face left on a retired category would go on showing one nothing can pick -- and is **refused outright while a `locked` face holds it**, that face having been told to keep what it has. `NOT NULL`, defaults to `1`, constrained to `0`/`1`. |

Foreign keys — all three parents are seeded with an id-`0` sentinel row, which is what lets these columns be `NOT NULL` and still allow "nothing assigned":
- The `icon_id` column references the PK of the table `icon` described above. `NOT NULL DEFAULT 0` (the `None` icon), so a new category can be inserted without specifying one.
- The `colour_id` column references the PK of the table `colour` described above. `NOT NULL DEFAULT 0` (the `None` colour), for the same reason.
- The `project_id` column references the PK of the table `project` described above. `NOT NULL DEFAULT 0` (the `None` project), for the same reason.

Constraints:
- `category_name` is `CHECK`ed at **35 characters**, which is the table's own answer rather than a restatement of the app's: a name arriving from anywhere else -- a script writing straight to the file, a paste that never passed through a field -- is refused here. Applying it to a database that predates it means rebuilding the table (sqlite cannot `ALTER` a `CHECK`), and any name already over the limit has to be cut to it first or the rebuild fails on its own constraint.
- Seeded with an `Unassigned` row (linked to the `None` icon and the `None` colour), a `Break` row (linked to the `ic_break` icon), and a `Meeting` row (linked to the `ic_meeting` icon) -- both seeded with the `None` colour, since none was specified.
- `UN1_category` is a **partial** unique index: `category_name COLLATE NOCASE` where `active = 1`. Only one *active* category may hold a name, while any number of retired ones may share it. That is what makes "create a new category with the same name" a real choice rather than a way to end up with two live categories nobody can tell apart, and it is why the index cannot simply be `UNIQUE (category_name)` the way `colour_name` and `icon_name` are.
- `COLLATE NOCASE` deliberately matches `CategoryStore.matching(name:)`, which is also case-insensitive. An index without it would let `email` and `Email` both be active while the app's own collision check considered them the same name, so the two would disagree about what a duplicate is.
- Because a retired category's name can be taken by an active one while it is away, **reinstating can fail**. `CategoryStore.setActive` returns whether the write took, and the tab is redrawn from the table afterwards rather than from what was asked for, so a refused reinstatement never shows as a ticked box over a row that is still retired.
- The same partiality means **which row is being renamed decides whether a name collides**, not only which row is in the way. An active category cannot take a name another active one holds, the index refusing the write; a *retired* one can, because the index does not cover it. The app follows the index rather than being stricter than it (`CategoryRenameRules.decision` reads `isActive` on the row being renamed), so renaming a retired row onto an active name is confirmed rather than refused, and confirmed because of what it costs: while the two share a name, reinstating the retired one is exactly what the index then refuses. Renaming either of them frees it again.
- `CategoryStore.matching(name:)` orders `active DESC, category_id ASC` for the same reason: answering with a retired namesake would have the tab offer to reinstate a category whose name is already taken, which this index then refuses. It returns *every* match rather than the first, because how many there are changes the answer -- one retired namesake can be reactivated, several cannot, there being no way to say which was meant.

### `face` (`database/008_face.sql`)

The 12 physical faces of the TimeFlip device, each linked to the category currently assigned to it, plus a 13th the app keeps for itself.

Face `13` is manual mode's. It is not a face of any cube; it exists so that a segment timed from the app resolves its category through the same `device_event.device_face -> face -> category` join as a segment flipped on hardware, rather than needing a second way to record what was being worked on. One face is enough because manual mode times one thing at a time: the category being timed is whatever face 13 currently holds, and picking a new category in the Faces tab reassigns it. Keeping it off the cube's own 1-12 means a manual session never disturbs what the physical faces are assigned to.

| Column        | Type    | Description                                                        |
|---------------|---------|-----------------------------------------------------------------------|
| `face_id`     | INTEGER | Primary key, `1`-`12` (matches the device's face numbering).         |
| `category_id` | INTEGER | References `category.category_id` — the category currently assigned to this face. |
| `locked`      | INTEGER | `1` to pin this face's category so it can't be reassigned by accident (a face the user wants permanent, e.g. Break or Meeting), `0` if it can be reassigned freely. Also blocks the *category* being retired while this face holds it, since retiring would take it off (see `active` above). `NOT NULL`, defaults to `0`. |

Foreign keys:
- The `category_id` column references the PK of the table `category` described above. `NOT NULL`.

Constraints:
- Seeded with all 12 faces pointing at the `Unassigned` category, except face `2` (`Meeting`) and face `8` (`Break`).
- Those same two faces are the only ones seeded **locked**. They are the two the physical test cube carries stickers for, and the two the `locked` column exists for: a face whose meaning is printed on the cube cannot be reassigned without the sticker becoming a lie. Every other face seeds unlocked, since `Unassigned` is exactly the face you would want to reassign.
- `locked` is constrained to `0`/`1` (SQLite has no native boolean type) and defaults to `0`.

### `time_entry` (`database/009_time_entry.sql`)

A single tracked time span, linked to the category it was logged against.

| Column                      | Type    | Description                                                        |
|------------------------------|---------|-----------------------------------------------------------------------|
| `time_entry_id`              | INTEGER | Row identifier, primary key, autoincrementing.                        |
| `category_id`                | INTEGER | References `category.category_id` — the category this entry was logged against. |
| `device_event_id`           | INTEGER | References `device_event.device_event_id` — the device event this entry was created from. Every time entry has exactly one device event, but not every device event becomes a time entry. |
| `started_at`                 | TEXT    | When the entry started, as a local-time ISO 8601 timestamp with no UTC offset. |
| `start_timezone_id`     | INTEGER | References `timezone.timezone_id` — the IANA zone `started_at` was recorded in.        |
| `ended_at`                   | TEXT    | When the entry ended, as a local-time ISO 8601 timestamp with no UTC offset. |
| `end_timezone_id`       | INTEGER | References `timezone.timezone_id` — the IANA zone `ended_at` was recorded in.          |
| `duration_seconds`           | REAL    | How long the entry lasted, in seconds.                                 |
| `total_cost`                 | INTEGER | What this entry cost, as a whole number of **cents** (e.g. `250` = \$2.50) to avoid floating-point money; the UI formats it for display as `$x.xx`. Its category's hourly `cost` applied to this entry's `duration_seconds`, captured when the row is created and never recalculated, so re-rating a category leaves history priced as it was actually logged. `NOT NULL`, defaults to `0`. Nothing computes it yet, so every existing row reads `0` — see Cost time entry (reasoning in `~/harry.git/TimeFlipLinux/docs/TODO-features-under-development.md`). |
| `synced_to_google_calendar`  | INTEGER | `1` if this entry has been synced to Google Calendar, `0` otherwise.  |

Foreign keys:
- The `category_id` column references the PK of the table `category` described above. `NOT NULL`.
- The `device_event_id` column references the PK of the table `device_event` described above. `NOT NULL`.
- The `start_timezone_id` and `end_timezone_id` columns both reference the PK of the table `timezone` described above — two separate FKs because `started_at` and `ended_at` are each stamped in whatever zone was current at the time, and a span can cross a zone change. Both `NOT NULL DEFAULT 0` — id `0` is the seeded `Unknown` sentinel.

Constraints:
- `duration_seconds` is constrained to be non-negative.
- `synced_to_google_calendar` is constrained to `0`/`1` (SQLite has no native boolean type) and defaults to `0`.
- `device_event_id` has a `UNIQUE` index (`UN1_time_entry`), so a device event can produce at most one time entry. The creation rule (see `docs/operation-spec.md` § 3) already tests that a segment is not in `time_entry` before converting it, but that test races itself: two sweeps overlapping, or a sweep restarted after a crash between the insert and the `processed` update, would both find the same segment eligible. The index is what makes the rule true rather than merely intended, and it is why `time_entry_check` can be re-run at any time without care.

  It also means a time entry can point at only **one** device event, which matters because of `blip_time`: a segment shorter than that setting is merged into the following one, so a single entry can cover several segments of wall-clock time while naming just the one the user actually settled on. The merged-away segments keep their `device_event` rows and never appear here.

### `device_notification` (`database/010_device_notification.sql`)

Point-in-time device notifications that aren't timing segments — `double_tap`, `battery_level`, `system_state`, `device_info`, `event_log` (see `event_type`). Unlike `device_event`, these don't have a duration or a face; each row is a single moment with a decoded value.

**Nothing writes this table yet.** The events reach the app -- every characteristic that can notify is subscribed to, and every byte is traced into `debug_log` -- but the durable, decoded record described here is not built.

| Column                  | Type    | Description                                                              |
|--------------------------|---------|------------------------------------------------------------------------------|
| `device_notification_id`| INTEGER | Row identifier, primary key, autoincrementing.                              |
| `event_type_id`          | INTEGER | References `event_type.event_type_id` — which kind of notification this is. |
| `start_time`             | TEXT    | When the notification was received, as a local-time ISO 8601 timestamp with no UTC offset. Named to match `device_event` rather than e.g. `occurred_at`, so both device tables can be queried/ordered the same way. |
| `timezone_id`          | INTEGER | References `timezone.timezone_id` — the IANA zone `start_time` was recorded in.              |
| `start_epoch`            | INTEGER | The same moment as `start_time`, as Unix epoch seconds. Indexed.            |
| `payload`                | TEXT    | The decoded value this event type carries (e.g. a battery percentage, a system state name), not the device's raw encoding. |

Foreign keys:
- The `event_type_id` column references the PK of the table `event_type` described above. `NOT NULL`. Always one of the point-in-time types for rows in this table.
- The `timezone_id` column references the PK of the table `timezone` described above. `NOT NULL DEFAULT 0` — id `0` is the seeded `Unknown` sentinel.

Constraints:
- `start_epoch` has a non-unique index (`IN1_device_notification`).

### `setting` (`database/011_setting.sql`)

Generic key/value store for device/app settings — one row per setting, rather than a dedicated column per setting.

| Column                 | Type    | Description                                    |
|-------------------------|---------|---------------------------------------------------|
| `setting_id`            | INTEGER | Row identifier, primary key, autoincrementing.     |
| `setting_name`          | TEXT    | The setting's name, e.g. `led_settings`.           |
| `setting_value`         | TEXT    | The setting's value, always a JSON object (even single-value settings) so reading this table never needs to branch on which row it is. |
| `setting_description`   | TEXT    | Human-readable explanation of what this setting controls. |

Constraints:
- `setting_name` is `UNIQUE` — each setting is only represented by one row.
- `setting_value` is `NOT NULL`.

Seeded rows:
- `db_type` = `{"type":"production"}` — which physical database file this row lives in, `production` or `test`. **Set once, when the file is first created, and never changed afterwards**: `production.sqlite` seeds as `production` from this default, and `scripts/switch-database.sh test` overrides a freshly-created `test.sqlite` to `test` immediately after seeding it. Read by `DatabaseEnvironment` and used as a pre-testing safety check: if this reads `production` during what is supposed to be a testing session, the `appdata.sqlite` symlink was never repointed and testing must not proceed. `DatabaseEnvironment` deliberately answers `nil` rather than a default for anything unexpected — no row, an unrecognised value, a database it cannot read — because defaulting would answer "which database am I writing to" with the reassuring option at exactly the moment the answer is unknown. **This is one of the two documented exceptions to reading a value at the point of use** (see the root `CLAUDE.md`): the row is written when the file is created and never again, so it cannot go stale within a launch.
- `double_tap_settings` = `{"enabled":false,"clickThreshold":90,"limit":20,"latency":50,"window":50}` — `enabled` is what the app wants the gesture to be. **Seeded off**: the gesture pauses the cube on any knock hard enough to register, which includes one through the desk it is sitting on, so a cube nobody has asked for it should not be stopping the clock. Turning it off is faked: no BLE command disables double tap, so `false` means `0x16` is sent with `window` forced to `0`, which makes the gesture unrecognisable (measured on a real cube, `docs/timeflip2-firmware-observations.md` finding 11). The stored `window` keeps its real value throughout -- that is what turning the gesture back on sends, so zeroing it would leave nothing to come back to. `DoubleTapRules.asSent` is the one place that decides. `clickThreshold`/`limit`/`latency`/`window` are the accelerometer parameters, seeded from `DoubleTapParameters.default` in `TimeFlipDoubleTapParameters.swift` -- captured from a real device's actual registers (see `Tests/Bench/device_register_snapshot.json`), not an arbitrary guess.
- `led_settings` = `{"brightness":50,"blink_interval":5}` — a single record for the only two LED properties the vendor protocol exposes (device cmd `0x09`/`0x0A`; see [`docs/TimeFlip2 BLE Protocol v4.3.md`](TimeFlip2%20BLE%20Protocol%20v4.3.md)):
  - `brightness` (%) and `blink_interval` (seconds — the gap from the end of one blink to the start of the next) are seeded from the archive's own defaults (`AppState.swift`, `ledBrightnessPercent`/`blinkIntervalSeconds`). Bounded 1-100 and 5-60 on the Device tab (`DeviceCommandRules.brightnessRange`/`blinkRange`). **Neither has a read-back defined in the vendor protocol**, so unlike every other command the Device tab sends, these two are written and believed.
- `auto_pause_minutes` = `{"minutes":0}` — delay after which the device pauses itself if the face hasn't changed (device cmd `0x05`; `0` disables, matching the vendor protocol's own disabled-by-default behavior; the device itself only supports whole-minute granularity, so this can't be made finer). The timer resets every time the face changes.
- `blip_time` = `{"seconds":5}` — while picking up and turning the device to find the desired face, it can briefly pass over other faces, creating unwanted `device_event` segments for them. Any segment shorter than `seconds` gets no `time_entry` and is marked `processed`; it is **not** merged into the following segment. `0` disables the filter. Edited on the App tab as "Ignore flips under", bounded 0-30. See [Operation Spec § applying `blip_time`](operation-spec.md) for why the device does not do this for us and why lowering the value is reversible.
- `firmware_check` = `{"last_alert":"<today>","interval_months":2}` — a single record for the firmware-update reminder:
  - `last_alert` is a local date (`YYYY-MM-DD`, no companion timezone column since only calendar-date granularity matters here), seeded to the date this row was first inserted (`date('now', 'localtime')`, string-concatenated into the JSON literal at seed time, same style as the rest of this file's hand-written JSON — `ON CONFLICT DO NOTHING` means it's never reseeded on later launches).
  - `interval_months` (seeded to `2`) is how many calendar months after `last_alert` before the user is prompted again to connect the device to the official TimeFlip app and check for a firmware update. **This app does read the cube's current version** — Firmware Revision String `0x2A26`, stored in `device_info` above — so the reminder is not there for want of knowing what is on the cube. What it cannot do is *change* it: flashing new firmware is only possible from the vendor's own app, and sending the user there is the whole job of this alert. (An earlier version of this line said there was no documented way to check the version at all, which was wrong: reading the version and being able to replace it are different things, and only the second is missing.)
  - The next-due date is `last_alert + interval_months` (calendar months, not a fixed day count). The Settings button that dismisses the alert resets `last_alert` to the current date regardless of whether the user actually performed the check, pushing the next alert out by `interval_months` either way.
- `time_entry_check` = `{"last_check":"…"}` — when the app last swept `device_event` for finalized segments to turn into `time_entry` rows. A local date-time (`YYYY-MM-DDTHH:MM:SS`), seeded to when the row was created (`strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime')`, string-concatenated into the JSON literal at seed time, same style as `connection.last_connection`) so the first sweep has a starting point rather than a null to special-case.
  - A segment is eligible when `finalised = 1`, `paused = 0`, `processed = 0`, and its `device_event_id` is not already in `time_entry`. The last of those is enforced by `UN1_time_entry`, not merely tested, so a sweep that overlaps another or restarts after a crash cannot double-count.
  - Eligible is not the same as producing a row: a segment shorter than `blip_time` gets no entry at all. It is **not** merged into the following one, which an earlier version of this line claimed and `blip_time`'s own entry above already contradicted — merging needs a `duration_seconds` this data does not reliably carry (a segment the next event proves ran three seconds can be stored as `0.0`; see production `device_event` 28), and losing a few seconds per pass-over is the cheaper mistake. Those segments still need marking `processed` when the sweep passes over them, or they stay eligible forever, since no `time_entry` will ever reference them.
- `pause_on_lock` = `{"enabled":true}` — when `enabled`, locking the device from the app (command `0x04`) pauses it first (command `0x06`), so no time is recorded against a category while the device is locked and unattended. The lock is what drives the pause, not the other way round. Applies to both ways the app locks: the status item's double-click gesture, and quitting with this setting on (`applicationShouldTerminate` pauses, then locks). Unlocking does **not** auto-resume — that's a deliberate second decision by the user. Nothing here applies to a pause triggered by double-tapping the device itself; that never locks.
- `fetch_history_interval_seconds` = `{"seconds":10}` — how often `HistoryTimer` asks for a history fetch, independent of the fetches already triggered by live face/pause events, so anything the device hasn't pushed a live notification for still gets picked up. The timer is one-shot and re-armed after every timeout, **re-reading this row each time**, so a change takes effect on the next tick rather than at the next launch. Stored in seconds; the App tab edits it in whole minutes (1 to 60) and converts before saving, so the seeded 10 seconds is below anything the UI can set.
- `display_seconds` = `{"enabled":true}` — when `enabled`, the menu bar duration display includes a seconds component (`H:MM:SS`) and refreshes every second; when disabled, it shows `H:MM` and refreshes every minute. Hours are unpadded below 10 (`1:23:45`) and two digits from 10 up (`12:23:45`).
- `daily_reset_time` = `{"hour":3,"minute":0}` — the local time each category's daily accounting rolls over to a new day. 3 AM rather than midnight so a session spanning midnight isn't split in two. `DayWindow` turns it into the window `DayTotal` sums over, which makes it the window `category.daily_limit` is spent against and therefore the one the menu bar's over-limit colouring is judged by. The App tab edits it in whole hours plus AM/PM; the minute is stored as well so a finer time can be set when testing the rollover firing.
- `low_battery_level` = `{"percent":10}` — the battery percentage (from the Battery Level characteristic `0x2A19`) at or below which the menu bar's category name starts flashing red (`LowBatteryWatch`, deciding by `BatteryRules`). To avoid flickering the warning on and off when a reading wobbles right around this value, it only clears again once the battery climbs 5 percentage points above the threshold (a fixed hysteresis margin, not stored in this setting).
- `debug` = `{"enabled":false,"directory":"~/Library/Application Support/Facet"}` — the trace: whether it is gathered, and where it is kept. Both are shown on the App tab's Debug section (`AppSettingsPane`, `DebugTraceRules`).
  - `enabled` — **the one gate on debug logging, and it is live.** Read at launch and then told by the App tab's Debug section (`DebugLog.setRecording`), so logging starts and stops as the box is pressed rather than at the next launch. What makes leaving it off free is the message being an `@autoclosure`: a call site that is not being recorded builds no string at all, which matters because every BLE packet is a call site. The trace database is brought up by the first message actually recorded, so a launch that records nothing leaves no `debug.sqlite` behind. A row edited behind the app's back is not noticed until it restarts, which is why `Tests/Scripted/00-setup.sh` writes it with the app shut. Seeded off, so a fresh install is quiet until somebody turns it on. The compile-time `DeveloperMode` flag this used to sit behind was removed on 2026-09-04.
  - `directory` — which folder `debug.sqlite` is kept in. **Not how a user finds it**: the Debug section's Trace file row reveals the file in the Finder and saves a copy to send in, so nobody has to navigate to this path. It is seeded to the folder the app already keeps its databases in, deliberately away from `~/Documents` and `~/Desktop`, which are iCloud sync roots when Desktop and Documents syncing is on: a database written continuously inside one of those is how conflicted copies and corruption happen. **Stored with a leading `~` and expanded where the file is opened**, never stored expanded: an absolute path names one machine's home directory, and this database is copied between machines and rebuilt from the DDL by the test suite. Read at launch (`main.swift`), so a folder chosen on the App tab applies from the next launch — the trace database is open from launch to quit — and one that cannot be written falls back to the seeded folder with a line on stderr. What the file is *called* is not stored here: `DatabaseBootstrap.debugDatabaseURL(in:)` names it, so the folder is the only part anybody chooses.
  - Revealing it, copying it and emptying it are `DebugTraceFile`, which opens its own connection **whether or not this launch has a logger** — the file outlives logging being on, and that is most of a support conversation. A copy is `VACUUM INTO` rather than a byte copy, since the app may be writing the trace while somebody is sending it: what is wanted is one consistent file that opens on its own, with no `-wal` or `-shm` beside it. Emptying deletes the rows and `VACUUM`s rather than deleting the file, so a running logger's open connection and prepared insert are untouched; `timezone` is left alone, every `debug_log` row carrying its id as a real foreign key.
  - **`to_file` is gone** (2026-09-03). It was a placeholder for writing the same messages to a plain log file a non-technical user could send back, which is what `debug.sqlite` already is. An existing database may still hold the key, which nothing reads.
- `google_account` = `{}` — everything Google, in one row:
  - `name` / `email` — the connected account's identity, from the OpenID Connect userinfo endpoint. Fetched once after sign-in and reused so the endpoint isn't called on every launch or Settings open. These two are reset on sign-out; the keys below are not, since they are configuration rather than identity.
  - `calendar_id` / `calendar_name` — the calendar time entries sync into. Here rather than in their own row because a calendar selection is meaningless without an account.
  - `client_id` — the OAuth client id. Not a secret (it appears in every OAuth URL), which is why it is not in the Keychain alongside the client secret. Developer mode's `config.json` overrides it at launch.
- `connection` = `{"connected":false,"last_connection":"…","connection_lost":"","quit_request":""}` — **connection**: whether the app can reach its paired device right now, and when that last changed. Every field is transient, moving on each connect and drop.
  - `connected` — true between a successful login and the next drop or quit. This is the flag to read for "is the device reachable now?".
  - `last_connection` — the most recent successful login (a new pairing, or any app-start/reconnect login). Seeded to when the row was created.
  - `connection_lost` — when a drop was last detected. Cleared to `""` on a clean quit, so an intentional shutdown isn't later misread as the device having gone away.
  - `quit_request` — when the app was last asked to quit.
  - The three timestamps exist so an observer can tell *connected now* from *lost the device* from *quit deliberately*, rather than inferring it from a single boolean.
- `paired` = `{"paired":false}`, `device_uuid` = `{}` and `device_name` = `{}` — **pairing**: whether the app knows which device to talk to, which one that is, and what that one is called. All three are durable (see [Pairing vs connection](#pairing-vs-connection) below).
  - `paired` answers *whether*. True from a successful first pairing until the user forgets the device — Forget Device, or the end of a confirmed factory reset. Nothing else clears it. Restored at launch, where it decides whether a connection is worth attempting at all.
  - `device_uuid` answers *which*: the CoreBluetooth peripheral identifier used to reconnect to that same device rather than rediscovering it. Absent until a first pairing, and cleared on forget. The identifier is assigned by this Mac's CoreBluetooth stack rather than by the device, so it means nothing on another machine and is not the device's BLE address.
  - `device_name` answers *what it is called*: the name the cube itself is carrying, its GAP Device Name `0x2A00`, which is what device command `0x15` writes and what a scan sees. Read from the peripheral on every connect and mirrored here, so it follows the device rather than leading it. Absent until the first connection, since the name is read rather than guessed.
  - **The name and the uuid have deliberately different lifetimes, which is why they are two rows rather than one.** Forget Device clears `device_uuid` and **keeps** `device_name`: forgetting does not un-rename the cube, and once a device has been renamed off "TimeFlip" that string is the only thing the filtered scan can match it on, so discarding it would discard the way back to the device. A confirmed factory reset (`0xFF`) clears **both**, the cube having reverted to the vendor name. `0xFE` (reset task info) leaves the name alone and so touches neither.
  - The Device tab's "Not paired" placeholder is a rendering of `paired` being false, not a stored value (`DeviceInfoRules.detail`), so a forgotten-but-remembered name is held in `device_name` without the tab claiming a pairing that is gone.
- `device_info` = `{}` — what the paired cube says it *is*, read from the standard Device Information service `0x180A` after every successful login and mirrored here, so it follows the device rather than leading it. Four keys, one per characteristic the vendor protocol lists in Tab. 1 (each 20 bytes, read-only): `manufacturer` (`0x2A29`), `model` (`0x2A24`), `hardware` (`0x2A27`), `firmware` (`0x2A26`). On the measured cube these read `DI_LABS` / `2.0` / `TFv4.1` / `FW_v3.64` (see [firmware observations](timeflip2-firmware-observations.md)). System ID (`0x2A23`) sits in the same service and is deliberately **not** read: it is raw binary rather than text, and nothing displays it.
  - **This is the one device reading that is stored, and the Battery Level beside it on the same tab is the one that is not.** The difference is not importance, it is whether the value can go stale while nobody is watching: a manufacturer string is a fact about the hardware, and a firmware string changes only when someone updates the firmware — at which point the next login re-reads it. A battery percentage is true for about a minute, so a remembered one would be a number that was true at a moment nobody can name, and it is left `nil` on purpose.
  - **Each key is absent until a connection has answered for it, and a read that fails leaves the stored value alone rather than blanking it.** The four reads are independent and a cube may answer three of them; `DeviceInfo`'s fields are optional the whole way down (`nil` = did not answer, never `""`) precisely so `DevicePairingRecorder.recordInfo` can tell "the cube declined to say" from "the cube says nothing", and write only what arrived. A cube has not stopped being a `TFv4.1` because one read timed out.
  - Durable alongside `paired`/`device_uuid`, and **cleared by Forget Device with them** (`DevicePairingRecorder.recordForget`). This is the one row on the pairing side that a forget empties — `device_name` deliberately survives one — and the reason is the bullet above: because only what a cube *answers* is written, a second cube that exposes no Device Information service would otherwise be shown wearing the first one's manufacturer and firmware. The row describes *the paired device*, so when there is not one it says nothing. Emptied to `""` rather than removed, the key having to survive for the write's read-back to confirm it.
  - Display is gated independently of that, and the two are not the same guard: `DeviceInfoRules.detail` shows **"Not paired"** over whatever is stored whenever `paired` is false, the same gate the Name and Battery rows use, so the tab stops claiming a device the instant the pairing goes rather than depending on the clear having succeeded. While paired but not connected the stored strings do show, greyed — `DeviceInfoRules.isLive` — which is the same "this was true when it was read" distinction drawn in colour rather than in words.

There is deliberately **no** `manual_mode` row. There was one, holding a threshold of failed connect attempts before the app offered manual mode, and it is gone with the threshold: one scan either finds the user's device or it does not, and a second and third scan of the same airspace seconds later find the same nothing. Whether manual mode is currently *on* was never stored and still is not — it is per-launch, in-memory state, and one of the two ways out of the mode is quitting and restarting, which a persisted flag would outlive, stranding a user who restarted specifically to get their cube back. (The other way out is pairing a device, which ends the session there and then; neither exit is anything the app does on its own.) With no row at all there is no longer an obvious place to put one by mistake. The reasoning is under Manual mode in `~/harry.git/TimeFlipLinux/docs/TODO-features-under-development.md`.

#### Pairing vs connection

Two distinct things, and the schema keeps them apart because conflating them is what made an out-of-band device reset look like nothing had happened:

| | Row | Lifetime | Changed by |
|---|---|---|---|
| **Pairing** | `paired`, `device_uuid` | Durable | Pairing a device; Forget Device |
| **Device name** | `device_name` | Durable, and outlives Forget Device | Connecting (mirrored from the cube); factory reset clears it |
| **Device identity** | `device_info` | Durable | Connecting (mirrored from the cube's `0x180A` reads) |
| **Connection** | `connection` | Transient | Every connect, drop, retry, quit |

Connection is **gated by pairing**: an app that isn't paired has no device to be connected to, so `connection.connected` cannot meaningfully be true while `paired` is false. The reverse is routine — a paired device that is switched off, out of range or simply not reached yet is paired and disconnected, and the app keeps retrying because it still knows which device it wants.

Practically, that means going out of range does **not** write to `paired`. To ask "does this need pairing?" read `paired`; to ask "can the app reach it right now?" read `connection.connected`.

### `timezone_alias` (`database/012_timezone_alias.sql`)

**The 151 legacy IANA names, each against the `timezone` row that replaced it.** `Cuba` → `America/Havana`, `US/Pacific` → `America/Los_Angeles`, `Asia/Calcutta` → `Asia/Kolkata`, and the rest of tzdb's `backward` links, from the same 2026c release the zones came from.

It exists because **a machine can genuinely be set to one of these**, and Foundation hands the name straight through without canonicalising it: measured on Linux 2026-09-07, `TZ=Cuba` makes `TimeZone.current.identifier` answer `Cuba`, and `TimeZone.knownTimeZoneIdentifiers` does not contain it. Without this table that machine's rows would be filed under a second id meaning Havana-by-another-name, which is the two-answers problem the first rule in `CLAUDE.md` is about.

| Column                | Type    | Description                                                        |
|-----------------------|---------|--------------------------------------------------------------------|
| `timezone_alias_id`   | INTEGER | Row identifier, primary key. Set explicitly by every seed row; no `AUTOINCREMENT`, nothing writes this table at runtime. |
| `timezone_alias_name` | TEXT    | The legacy IANA name (e.g. `Cuba`). `NOT NULL`, `UNIQUE` (`UN1_timezone_alias`). |
| `timezone_id`         | INTEGER | References `timezone(timezone_id)` — the zone this name resolves to. `NOT NULL DEFAULT 0`, per the convention for every `timezone_id` column, though every seeded row supplies a real id. |

**A separate table rather than a column on `timezone`, deliberately.** An alias row in `timezone` would own a `timezone_id` of its own, and `device_event.timezone_id` could point at it and satisfy the foreign key — two ids for one zone, and nothing would fail. Here the legacy names have no ids at all, so the only thing a foreign key can name is a real zone.

Numbered `012` rather than `003` so it follows `timezone` without renumbering the nine files between them; the numbering rule only requires a table to come after everything it references.

### `timezone_lookup` (`database/013_timezone_lookup.sql`, a view)

**Every name a zone answers to, canonical and legacy alike, against the one id it resolves to.** `timezone` unioned with `timezone_alias`, 599 rows over the 448 zones.

```sql
CREATE VIEW IF NOT EXISTS timezone_lookup AS
SELECT timezone_name, timezone_id FROM timezone
UNION ALL
SELECT timezone_alias_name AS timezone_name, timezone_id FROM timezone_alias;
```

**This is what the app reads, and the table is not.** `TimezoneStore.id(for:)` queries the view, so `Cuba` and `America/Havana` both answer `116` and neither the store nor any caller has to know which kind of name it was handed. The resolution rule lives in the database, which means a scripted check can ask the question with the same SQL the app uses.

The other direction — id to name, for display — reads `timezone` directly, because the view has one row per *name* and would answer a canonical row and each of its aliases. Resolving a name and naming an id are different questions and take different objects.

Being a view it is read-only, which is only viable because the tables under it are seeded: there is nothing for the app to insert through it. The one write that remains (a zone newer than the seed) goes to `timezone` directly.

## The debug database (`debug.sqlite`)

**A separate file, in the same directory as `production.sqlite` and `test.sqlite`.** It is what somebody sends in when they turn the `debug` setting on, so it holds the trace and nothing else: no `time_entry`, no `category`, no Google account. It also keeps the log out of the file the app is writing — anything reading `debug_log` locked `appdata.sqlite` against the app, and a confirmed pairing was lost to exactly that on 2026-08-22 (see `DatabaseConnection`'s busy timeout, which is the other half of the answer).

**Its DDL lives in the same `database/` directory, numbered from 500.** Below 500 is the app's schema; 500 and above is this one's. One directory holds both because `Package.swift` processes `Resources` and SwiftPM flattens what it processes, so a real subdirectory would be folded back in and applied to whichever database asked first. `DatabaseBootstrap.firstDebugDDLNumber` is where the line is drawn, and the gap between `011` and `500` is room for both schemas to grow without either renumbering the other.

### `timezone` (`database/500_timezone.sql`, a symlink to `002_timezone.sql`)

The same table as the app's, deliberately created a second time in the debug database. `debug_log.logged_at` is local time and needs its zone, and a foreign key cannot cross database files — so this file carries its own.

**It is one file, not two copies.** `500_timezone.sql` is a relative symlink to `002_timezone.sql`, so the definition cannot drift between the databases: there is nowhere for a column added to one to be missing from the other. It used to be a duplicate, and the only thing that differed was the second comment line. The numbering rule still decides which database the file belongs to, and it decides it from the *name* — `DatabaseBootstrap` reads `500` off the symlink and applies it to `debug.sqlite`, having never looked at what it points at.

**What may never be joined is the data, and that is unchanged by the link.** The two tables fill independently, so the same zone can hold a different id in each; nothing may join across the files, and nothing needs to. The point of the second table is that a submitted `debug.sqlite` is readable on its own, without the app's database beside it. Sharing the definition and sharing the rows are different things, and only the first is being done here.

**The link is relative and its target sits in the same directory, which is what makes it survive the build.** SwiftPM copies it into the resource bundle as a link rather than following it, and `.process` flattens everything to the bundle root, so `002_timezone.sql` lands beside it and the link resolves. Measured on Linux 2026-09-07, from both the source directory and the built bundle: 13 `.sql` files present, the debug database coming up with `debug_log` and `timezone` and the seeded `Unknown` row at id 0.

### `timezone_alias` and `timezone_lookup` (`502`/`503`, symlinks to `012`/`013`)

The same two objects as the app's, symlinked in exactly as `500_timezone.sql` is, because `DebugLog` resolves its zone through the view like everything else. `debug.sqlite` therefore holds `timezone`, `timezone_alias` and the `timezone_lookup` view alongside `debug_log`, and a submitted trace resolves `Cuba` on its own without the app's database beside it.

They are numbered after `501_debug_log.sql` rather than before it, which reads oddly and is correct: the numbering rule only requires a table to follow what it *references*, and both follow `500`. Putting them at `501`/`502` would have meant renumbering `debug_log` and every reference to it for no gain.

### `debug_log` (`database/501_debug_log.sql`)

Every `DebugLog.record` message, in addition to being printed to the terminal — lets a failed run be reconstructed from the database afterwards rather than depending on a terminal transcript that was never captured. It is what every scripted check polls: press by name, then wait for the row.

`DebugLog` holds **its own connection** to this file, deliberately. Not because sharing one would be hard, but because a diagnostic record that rides inside somebody else's transaction is rolled back with it -- the log of what the app was doing when it went wrong would disappear along with the work that went wrong.

| Column                | Type    | Description                                                        |
|------------------------|---------|--------------------------------------------------------------------|
| `debug_log_id`         | INTEGER | Row identifier, primary key, autoincrementing. **Clearing the trace does not send it back to 1**: `AUTOINCREMENT` keeps the highest id ever issued in `sqlite_sequence`, and neither the delete nor the `VACUUM` touches it. Every scripted check baselines `MAX(debug_log_id)` and polls for rows above it, so ids that started again would put later rows under a baseline taken before the clear. |
| `logged_at`            | TEXT    | When the message was printed, as a local-time ISO 8601 timestamp with no UTC offset. |
| `timezone_id`          | INTEGER | References `timezone.timezone_id` **in this file** — the IANA zone `logged_at` was recorded in. |
| `tag`                  | TEXT    | The `DebugLog.Tag` raw value (e.g. `history`, `entry`) identifying which subsystem logged this message — matches the bracketed tag in the terminal output, which is padded to the width of the longest case so lines stay aligned. |
| `message`              | TEXT    | The debug message text, exactly as printed (without the timestamp/tag prefix, which are separate columns here). |

Foreign keys:
- The `timezone_id` column references the PK of `timezone` in this same file, described above. `NOT NULL DEFAULT 0` — id `0` is the seeded `Unknown` sentinel.

Constraints:
- `logged_at`, `timezone_id`, `tag`, `message` are all `NOT NULL`.

No retention/cleanup is implemented yet — this file grows for as long as the `debug` setting is on. `Tests/Scripted/run.sh` deletes it on a clean rebuild, so a scripted run starts from an empty trace; the app recreates it on its next launch.
