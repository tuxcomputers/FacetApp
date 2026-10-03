# Operation Spec: Device Event → Time Entry

[← Back to README](../README.md) · [Workflow](workflow.md) · [Database Design](database-design.md)

This document describes how the app turns a TimeFlip device's raw Bluetooth activity into the rows stored by the schema in [Database Design](database-design.md). It is the pipeline as built. `device_notification` is the one table below that nothing writes, and no version of this app has ever written it. For *why* the schema is shaped this way (how the device owner wants to organize activities and faces), see [Workflow](workflow.md).

## Overview

```
TimeFlip device (BLE)
      │
      ▼
Decode history frame
      │
      ▼
device_event ──► time_entry ──► Google Calendar
```

## 1. Classifying a timing segment

Only timing segments are classified. A history frame whose face byte has the high bit set is a `pause` and any other is a `face_flip` (`cube_history::record`); the app's own segments on faces 13 and 14 are `face_flip` (`segment`). Both go to `device_event`. The other `event_type` rows (`double_tap`, `auto_pause_minutes`, `battery_level`, `system_state`, `device_info`, `event_log`) are seeded for point-in-time notifications and nothing selects them.

## 2. Recording a timing segment (`device_event`)

1. The device's history stream reports a frame: event number, face byte, timestamp, duration. The app decodes this into human-readable values (never stores the raw hex); see [Database Design § decoded, not raw](database-design.md#design-principle-decoded-not-raw).
2. The face byte's high bit determines `event_type_id` (`face_flip` vs `pause`) and the decoded `device_face` number (`1`-`12` from a cube; `13` and `14` are the app's own faces, used in rotation when it is doing the timing).
3. The app inserts a `device_event` row: `event_number`, `event_type_id`, `device_face`, `start_time` / `timezone_id` (filed under the machine's zone when the row is first recorded, `facet_core::timezone::current_id`; see [Database Design § local time + timezone](database-design.md#design-principle-local-time--timezone)), `duration_seconds`, `paused`. Two writers share this table, one per kind of face: `facet_core::cube_history::record` for the cube's segments (faces 1 to 12) and `facet_core::segment` for the app's own (13 and 14). Each decides whether a segment opens a row, grows the open one, or closes it out.
4. Identity is **`(event_number, start_epoch)`**, `UN1_device_event`, so re-ingesting a frame already seen (e.g. after a reconnect) brings the existing row up to date rather than adding a duplicate: the device's history buffer can and does replay frames the app has already processed. The pair rather than the number alone, because a factory reset restarts the cube's counter, so an event number is only unique within one counter generation.
5. Per the device's own behavior (see `docs/timeflip.md` §5), the **last frame in every history dump is the current, still-open interval**: its duration keeps growing until the segment ends. The app treats this last frame as provisional: `cube_history::record` writes it with `finalised = 0` and keeps updating the same `device_event` row, matched by `(event_number, start_epoch)`, rather than creating a new one, until a subsequent flip/pause frame finalizes it.

## 3. Turning a finalized segment into a `time_entry`

A `device_event` row becomes a `time_entry` once its segment is finalized (i.e. it's no longer the device's in-progress last frame: a later event has closed it out). Precisely, `time_entry::consider` makes an entry when `finalised = 1`, `paused = 0`, the duration is at least `blip_time`, and the `device_event_id` is not already in `time_entry`, that last one enforced by `UN1_time_entry` rather than merely tested; it sets `processed = 1` either way.

`facet_core::time_entry::consider` is that conversion and the one writer of `time_entry`. **It is handed an id, not the details**: `cube_history::record` and `segment` call it the moment they finalise a row, and it reads the row back for itself, because the table is what is true about that segment and details passed as arguments are a second copy that can differ from it.

(`setting.time_entry_check` was seeded to record when a periodic sweep last ran. Nothing reads it: the conversion is driven by a segment closing rather than by a pass over the table.)

1. Resolve which `category` the segment belongs to: look up `face.category_id` for the `device_event.device_face` value **as it is mapped at the moment the segment is converted**, and write it onto the entry. Reassigning a face afterwards therefore cannot rewrite what was already recorded: existing `time_entry` rows keep the `category_id` they were given.
2. Insert a `time_entry` row: `category_id`, `device_event_id` (the `device_event` row it came from), `started_at`/`start_timezone_id` (copied from the `device_event` row), `ended_at`/`end_timezone_id` (`started_at` + `duration_seconds`, converted back to local time), `duration_seconds`, and `synced_to_google_calendar = 0`.
3. Not every `device_event` row necessarily becomes a `time_entry`; see applying `blip_time` below.

### Applying `blip_time`

While picking the device up and turning it to find the desired face, it can briefly pass over other faces, creating short, unwanted `device_event` segments for them before landing on the intended one. The `blip_time` setting (see [Database Design § `setting`](database-design.md), seeded to `5` seconds, edited on the App tab as "Ignore flips under") filters these out:

- When a segment is finalized (step 3 above), compare its `duration_seconds` to `blip_time`.
- If `duration_seconds < blip_time`, **no `time_entry` is created** and the segment is marked `processed = 1`. Strictly less than, so a segment exactly as long as the threshold is kept. `blip_time = 0` disables the filter, and then even a zero-length segment converts.
- Marking it `processed` is what stops the eligible set growing a permanent tail of rows anything looking at the table has to re-examine.
- **The threshold is read first, before anything about the particular segment.** It is the module's standing rule rather than a property of the record, and reading it first means a decision is never taken against a value read at some other moment.
- The `device_event` row is kept as-is (per the "decoded, not raw" principle nothing there is deleted); only `time_entry` creation is affected. Raising the threshold changes nothing; entries already made stay made. Lowering it does not retroactively convert the segments it skipped either, there being no pass over the table to do so; that is the cost of driving conversion off a segment closing rather than off a sweep.

**The blip is not merged into the following segment**, though an earlier version of this spec called for exactly that: the next segment's entry starting from the blip's `started_at`, so the seconds counted toward the face the user settled on. Dropped deliberately. It depends on a `duration_seconds` this data does not reliably have (see the note below), and it was what forced the awkward "processed with no entry" case. Losing a few seconds per pass-over is the cheaper mistake.

**The device does not do this for us, and it looks as though it does.** The vendor spec's "the history will be sent with all intervals that lasted for at least 5 sec" is a property of the `0x02` stream alone, not of what the cube records: it keeps short intervals and returns them happily to a single-event `0x01` read, which is what the cheap refresh in step 2 of the pipeline relies on. Blips reach this app anyway, by the live path rather than the stream: every flip triggers a history fetch, which writes the now-current segment with the duration it has at that instant, and the next flip closes it out. On 2026-08-02, 13 of 63 finalized unpaused production segments were under 5 seconds and 8 of them read `0.0`. That reasoning removed this section once already; it is written down here so it does not remove it a second time.

Note also that a blip's recorded `duration_seconds` is not trustworthy. It is whatever the last fetch saw, so a segment the following event shows to have lasted 3 seconds can be stored as `0.0` (production `device_event` 28 and 29). Anything built on top of this should treat a sub-`blip_time` duration as "short", not as a measurement.

## 4. Point-in-time notifications (`device_notification`)

**Nothing writes this table.** A double tap, a battery reading or a system state reaches the app but no decoded record of it is kept. The app follows four characteristics (history, face, system state, battery level), and `TracedRadio` and `TracedLink` write every write, read and notification on them to `debug_log` under the `ble-tx` and `ble-rx` tags, which is the only record. A row would hold `event_type_id`, `start_time`/`timezone_id`/`start_epoch` and a decoded `payload`, as documented in [Database Design](database-design.md).

## 5. Syncing to Google Calendar

`facet_core::google_events`, driven by `Google::sync_calendar` in `facet-ui`, selects every `time_entry` row where `synced_to_google_calendar = 0`, oldest first, at most 50 a pass, and creates the corresponding Google Calendar event: the entry's category name as the title, `Facet time entry <id>` and `Device event <id>` on two lines as the description, and the start and end as UTC instants from the device event's `start_epoch` and the entry's `duration_seconds`. Each instant is sent beside the IANA zone the entry was filed under (`start_timezone_id`), or with no zone named when it was filed under Unknown; Google shows the event in the calendar's own zone. A failed delivery leaves the flag at `0` so the row is carried by the next pass; there's no separate retry-count or backoff column, since idempotent re-delivery is cheap enough not to need one.

**A sweep of the table, not a hand-off of one row.** A sweep runs at launch, whenever cube history is filed or the by-hand clock changes, when a calendar is made or confirmed, and again at once when a full batch of 50 went across. What runs is every unsynced entry, so time recorded while offline goes across with the next sweep, and a backlog recorded before anyone signed in goes across on connecting a calendar. A pass that stops on a failed request is not tried again for 60 seconds.

**The event id is derived from the `time_entry` id rather than stored** (`facet4213`). Google refuses a second event with an id it already has, so a crash between writing the event and ticking the row is a no-op rather than a duplicate, and the read-back is a `GET` at a known address rather than a search. No column had to be added for it.

**The tick means the event is right, not that a request succeeded.** Insert, fetch the event back, compare the title, the description and both instants, and only then set the flag: a row marked synced is never looked at again, so the claim has to be earned. A mismatch leaves the row at `0` and says which field differed. A row Google will not take is skipped rather than stopping the pass; only a refused *request* stops one.

**No data is remembered between passes.** The calendar id, the entries and their categories are all read at the start of each pass, so a calendar disconnected mid-sweep is noticed on the next one rather than written to anyway. The only thing held is when the last pass failed, for the 60 second back-off.

## 6. Displaying a category's elapsed time

The menu bar (and any other "how long have I spent on X today" display) must show only **today's** accumulated time for a category: never a running total that carries over from a previous day. This was previously observed to be broken (a category showed elapsed time left over from yesterday). It is implemented by `facet_core::timing::day_seconds` (the figure) over `timing::window_start` (the boundary) and `time_entry::seconds_in_window` (the rows), and read through `timing::read` and `timing::read_cube`, which is what the status item's line and the Faces tab's Timing column draw.

1. "Today" starts at the most recent `daily_reset_time` (the `setting` row, seeded to `03:00`, editable on the App tab) at or before now, in the machine's local time. **Not midnight**, which is what this said before the setting existed: a boundary in the small hours means a session running past midnight stays on the day it started, which is the whole reason the setting is configurable. Same boundary the `category.daily_limit` budget is measured against.
2. The displayed total for a category = the sum of `time_entry.duration_seconds` for every `time_entry` row with that `category_id` overlapping the current window, **plus** the elapsed time of a currently in-progress segment if the device is right now on a face mapped to that category. The in-progress segment has no `time_entry` row yet, which is what keeps it from being counted twice.
3. Because faces map to categories many-to-one (see [Workflow § faces map to categories many-to-one](workflow.md#faces-map-to-categories-many-to-one)), this sum must include `time_entry` rows created from *every* face mapped to that category, not just whichever face is currently active. Keying the totals by face instead is exactly how this went wrong: two faces sharing a category each counted alone, so 40 minutes on one and 40 on another left a 60-minute `daily_limit` unreached and the menu bar drew one face's figure beside the shared category's name.
4. `time_entry.category_id` is what the sum groups by, deliberately, rather than re-deriving the category by joining `device_event` to `face`. An entry records the category the face was mapped to **when the segment happened**; the current mapping would move a day's work to whatever the face points at next. An entry is given its category when the segment is converted, and never re-derived afterwards.
5. At the reset boundary, every category's displayed total returns to zero, regardless of whether a segment happens to be in progress at that moment: a live segment spanning the boundary counts only the portion after it toward "today's" total; the portion before belongs to the previous day.

## 7. Reaching a category's daily limit

`category.daily_limit` is a **hard** limit, not a warning: the figure from § 6 reaching it stops the clock. The rules are `facet_core::timing::is_limit_reached`, `is_resume_refused` and `is_clickable`; for the app's own clock `timing::enforce_daily_limit` stops it, and for a cube `facet_core::device::forced_pause::decide` decides and `Device::enforce_cube_rules` carries it out. `daily_limit = 0` means no limit, so nothing below applies to it.

1. When the category on show reaches its limit, the app stops the clock and then **refuses to start it again** while that category is still the one on show. With a cube the stop is `0x06 0x01`, confirmed with `0x10`, and the refusal is `Device::toggle_cube_pause` declining to send `0x06 0x02`. The refusal is the enforcement, and it is the app's rather than the cube's: nothing in the protocol asks the cube to hold a pause against its own user, so `0x06 0x02` is honoured whenever it arrives. The limit is therefore exactly as hard as the set of paths that can send it, which is why they are enumerated in point 3.
2. The stop fires within a second of the budget being spent. A one-second timer runs while a figure is counting, whatever the Show seconds setting says, and each tick re-reads the table and asks whether the limit is spent.
3. Three paths can start a clock again, and the limit answers each differently:
   - The menu's **Pause and Resume** item and a left click on the status item, which is its accelerator, reach the same toggle, which declines to resume while the budget is spent. The menu item is also greyed (`is_clickable`), so the refusal is visible rather than a click that silently does nothing. **Pausing** is never refused, only resuming. Locking is not refused either: unlocking is the one way back to a cube nobody can otherwise operate.
   - A **double tap on the cube** pauses and unpauses it in firmware, telling the app afterwards (see the Double tap characteristic in [the protocol spec](TimeFlip2%20BLE%20Protocol%20v4.3.md)). It cannot be refused, only answered: the next history fetch shows the cube running on a spent category and `decide` sends the pause again.
   - **Turning the cube** to another face starts it in firmware. The app neither sends that resume nor prevents it.
4. Turning to a face whose category still has budget therefore leaves the cube running, and the app drops its claim on the pause (`Decision::Release`); turning back to the spent face pauses it again. Pause is a property of the cube where a limit is a property of a category, so holding the pause across a turn would spend one category's budget and stop the whole day's tracking with it. The app never resumes a cube for a spent limit. A face that holds **no category** is handled alike but is lifted by giving that face a category (`PauseClaim::NoCategory { face }`, trace tag `forced`), and only while the cube is still stopped on that face; the claim ends as soon as the cube is seen running.
5. A category that has reached its limit **stays** reached until the next `daily_reset_time` boundary, or until its `daily_limit` is edited. It deliberately does not follow the total back down, because the total dips on its own: pausing stops the live segment counting, and the segment that ran the budget out is not a `time_entry` until the pause's history fetch ingests it, so for a moment the figure reads under the limit. Re-deriving in that window resumes the cube on the strength of a total the app knows is incomplete. An edit to the limit is answered immediately, being the one signal a stale total cannot imitate.
6. None of this is persisted. The cube holds its own pause across a quit, the claim is in memory only, and the totals are re-derived from `time_entry` on launch, so a relaunch onto a paused cube resting on a spent category still refuses the resume from those facts alone.
7. **It applies while the app is the clock too**, and that is the half `cargo test` and `12-daily-limit` can check without a cube: there the stop is the app closing its own open segment rather than `0x06 0x01` going out, and every path that would start it again asks the same question. There is no automatic resume in that case: with no cube it would be the app recording time against a category nobody has come back to. Raising the limit lifts the refusal, and starting the clock again stays the user's to do.

## Related documents

- [Workflow](workflow.md): the intended usage this pipeline serves: recurring vs. short-lived categories, and how faces map to categories.
- [Database Design](database-design.md): full schema, column-by-column.
- [`crates/facet-core/resources/database/CLAUDE.md`](../crates/facet-core/resources/database/CLAUDE.md): naming and storage conventions the schema follows.
- [`docs/timeflip.md`](timeflip.md) / [`docs/TimeFlip2 BLE Protocol v4.3.md`](TimeFlip2%20BLE%20Protocol%20v4.3.md): the device's wire protocol this pipeline decodes (official spec takes priority per the root [`CLAUDE.md`](../CLAUDE.md)).
