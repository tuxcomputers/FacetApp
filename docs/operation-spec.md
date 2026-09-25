# Operation Spec: Device Event → Time Entry

[← Back to README](../README.md) · [Workflow](workflow.md) · [Database Design](database-design.md)

This document describes how the app turns a TimeFlip device's raw Bluetooth activity into the rows stored by the schema in [Database Design](database-design.md). It is the pipeline as built, except where a section says otherwise: `device_notification` is the one table below that nothing writes yet. For *why* the schema is shaped this way — how the device owner wants to organize activities and faces — see [Workflow](workflow.md).

## Overview

```
TimeFlip device (BLE)
      │
      ▼
Decode raw notification/history frame
      │
      ├─ Timing segment (face flip / pause) ──► device_event ──► time_entry ──► Google Calendar
      │                                                 ▲
      └─ Point-in-time notification ──────────► device_notification
                                       (double tap, battery, system state, device info, event log)
```

Every device notification is classified into one `event_type` row first. That classification decides which of the two tables below it lands in.

## 1. Classifying a device notification

When the app receives a decoded event from the BLE driver, it is matched to an `event_type` row by name:

- `face_flip`, `pause` → come only from the **history stream** (the `...58` characteristic). These carry a duration and belong to a timing segment → go to `device_event`.
- `double_tap`, `auto_pause_minutes`, `battery_level`, `system_state`, `device_info`, `event_log` → live BLE notifications with no duration → go to `device_notification`.

## 2. Recording a timing segment (`device_event`)

1. The device's history stream reports a frame: event number, face byte, timestamp, duration. The app decodes this into human-readable values (never stores the raw hex) — see [Database Design § decoded, not raw](database-design.md#design-principle-decoded-not-raw).
2. The face byte's high bit determines `event_type_id` (`face_flip` vs `pause`) and the decoded `device_face` number (`1`-`12` from a cube; `13` and `14` are the app's own faces, used in rotation when it is doing the timing).
3. The app inserts a `device_event` row: `event_number`, `event_type_id`, `device_face`, `start_time` / `timezone_id` (captured in the local timezone at the moment the segment started — see [Database Design § local time + timezone](database-design.md#design-principle-local-time--timezone)), `duration_seconds`, `paused`. `DeviceEventRecorder` is the one writer of this table, and the only thing that decides whether a segment opens a row, grows the open one, or closes it out.
4. Identity is **`(event_number, start_epoch)`**, `UN1_device_event`, so re-ingesting a frame already seen (e.g. after a reconnect) brings the existing row up to date rather than adding a duplicate — the device's history buffer can and does replay frames the app has already processed. The pair rather than the number alone, because a factory reset restarts the cube's counter, so an event number is only unique within one counter generation.
5. Per the device's own behavior (see `docs/timeflip.md` §5), the **last frame in every history dump is the current, still-open interval** — its duration keeps growing until the segment ends. The app should treat this last frame as provisional: keep updating the same `device_event` row (matched by `event_number`) rather than creating a new one, until a subsequent flip/pause frame finalizes it.

## 3. Turning a finalized segment into a `time_entry`

A `device_event` row becomes a `time_entry` once its segment is finalized (i.e. it's no longer the device's in-progress last frame — a later event has closed it out). Precisely, a row is eligible when `finalised = 1`, `paused = 0`, `processed = 0`, and its `device_event_id` is not already in `time_entry` — that last one enforced by `UN1_time_entry` rather than merely tested.

`TimeEntryRecorder` is that conversion and is the one writer of `time_entry`. **It is handed an id, not the details**: `DeviceEventRecorder` calls it the moment it finalises a row, and it reads the row back for itself, because the table is what is true about that segment and details passed as arguments are a second copy that can differ from it. The decisions are separated out into `TimeEntryRules`, so they can be asserted against numbers without a database.

(`setting.time_entry_check` was seeded to record when a periodic sweep last ran. Nothing reads it: the conversion is driven by a segment closing rather than by a pass over the table.)

1. Resolve which `category` the segment belongs to: look up `face.category_id` for the `device_event.device_face` value **as it is mapped at the moment the segment is converted**, and write it onto the entry. Reassigning a face afterwards therefore cannot rewrite what was already recorded: existing `time_entry` rows keep the `category_id` they were given.
2. Insert a `time_entry` row: `category_id`, `device_event_id` (the `device_event` row it came from), `started_at`/`start_timezone_id` (copied from the `device_event` row), `ended_at`/`end_timezone_id` (`started_at` + `duration_seconds`, converted back to local time), `duration_seconds`, and `synced_to_google_calendar = 0`.
3. Not every `device_event` row necessarily becomes a `time_entry` — see applying `blip_time` below.

### Applying `blip_time`

While picking the device up and turning it to find the desired face, it can briefly pass over other faces, creating short, unwanted `device_event` segments for them before landing on the intended one. The `blip_time` setting (see [Database Design § `setting`](database-design.md), seeded to `5` seconds, edited on the App tab as "Ignore flips under") filters these out:

- When a segment is finalized (step 3 above), compare its `duration_seconds` to `blip_time`.
- If `duration_seconds < blip_time`, **no `time_entry` is created** and the segment is marked `processed = 1`. Strictly less than, so a segment exactly as long as the threshold is kept. `blip_time = 0` disables the filter, and then even a zero-length segment converts.
- Marking it `processed` is what stops the eligible set growing a permanent tail of rows anything looking at the table has to re-examine.
- **The threshold is read first, before anything about the particular segment.** It is the module's standing rule rather than a property of the record, and reading it first means a decision is never taken against a value read at some other moment.
- The `device_event` row is kept as-is (per the "decoded, not raw" principle nothing there is deleted); only `time_entry` creation is affected. Raising the threshold changes nothing; entries already made stay made. Lowering it does not retroactively convert the segments it skipped either, there being no pass over the table to do so — that is the cost of driving conversion off a segment closing rather than off a sweep.

**The blip is not merged into the following segment**, though an earlier version of this spec called for exactly that: the next segment's entry starting from the blip's `started_at`, so the seconds counted toward the face the user settled on. Dropped deliberately. It depends on a `duration_seconds` this data does not reliably have (see the note below), and it was what forced the awkward "processed with no entry" case. Losing a few seconds per pass-over is the cheaper mistake.

**The device does not do this for us, and it looks as though it does.** The vendor spec's "the history will be sent with all intervals that lasted for at least 5 sec" is a property of the `0x02` stream alone, not of what the cube records: it keeps short intervals and returns them happily to a single-event `0x01` read, which is what the cheap refresh in step 2 of the pipeline relies on. Blips reach this app anyway, by the live path rather than the stream — every flip triggers a history fetch, which writes the now-current segment with the duration it has at that instant, and the next flip closes it out. On 2026-08-02, 13 of 63 finalized unpaused production segments were under 5 seconds and 8 of them read `0.0`. That reasoning removed this section once already; it is written down here so it does not remove it a second time.

Note also that a blip's recorded `duration_seconds` is not trustworthy. It is whatever the last fetch saw, so a segment the following event shows to have lasted 3 seconds can be stored as `0.0` (production `device_event` 28 and 29). Anything built on top of this should treat a sub-`blip_time` duration as "short", not as a measurement.

## 4. Recording a point-in-time notification (`device_notification`)

**Not built.** The table exists and nothing writes it, so a double-tap or a low battery leaves no row behind. What follows is the intended shape.

For any non-timing event type (`double_tap`, `battery_level`, `system_state`, `device_info`, `event_log`, `auto_pause_minutes`): insert a `device_notification` row with `event_type_id`, `start_time`/`timezone_id`/`start_epoch` (now, in local time), and `payload` — the decoded value for that event type (e.g. a battery percentage, a system state name), not the device's raw encoding.

The events themselves do reach the app today — every characteristic that can notify is subscribed to, and every byte is written to `debug_log` by `BLETrace`. What is missing is the durable, decoded record.

## 5. Syncing to Google Calendar

`CalendarSync` selects every `time_entry` row where `synced_to_google_calendar = 0`, oldest first, creates the corresponding Google Calendar event (the entry's `category` name, `started_at`, `ended_at`) and sets the flag. A failed delivery leaves it at `0` so the row is carried by the next pass — there's no separate retry-count or backoff column, since idempotent re-delivery is cheap enough not to need one.

**A sweep of the table, not a hand-off of one row.** Recording an entry is what triggers it, but what runs is every unsynced entry, so time recorded while offline goes across with the next entry recorded. Connecting a calendar triggers the same sweep, which is what delivers a backlog recorded before anyone signed in.

**The event id is derived from the `time_entry` id rather than stored** (`facet4213`). Google refuses a second event with an id it already has, so a crash between writing the event and ticking the row is a no-op rather than a duplicate, and the read-back is a `GET` at a known address rather than a search. No column had to be added for it.

**The tick means the event is right, not that a request succeeded.** Insert, fetch the event back, compare the title, the description and both instants, and only then set the flag — a row marked synced is never looked at again, so the claim has to be earned. A mismatch leaves the row at `0` and says which field differed. A row Google will not take is skipped rather than stopping the pass; only a refused *request* stops one.

**Nothing is remembered between passes.** The calendar id, the entries and their categories are all read at the start of each pass, so a calendar disconnected mid-sweep is noticed on the next one rather than written to anyway.

## 6. Displaying a category's elapsed time

The menu bar (and any other "how long have I spent on X today" display) must show only **today's** accumulated time for a category — never a running total that carries over from a previous day. This was previously observed to be broken (a category showed elapsed time left over from yesterday). It is implemented by `DayTotal` (the figure) over `DayWindow` (the boundary) and `TimeEntryStore` (the rows), and read by `TimingReadout`, which is what both the menu bar and the Faces tab draw.

1. "Today" starts at the `setting` table's `daily_reset_time` (seeded to `03:00`, editable on the App tab), in the timezone the entries were recorded in (see [Database Design § local time + timezone](database-design.md#design-principle-local-time--timezone)). **Not midnight**, which is what this said before the setting existed: a boundary in the small hours means a session running past midnight stays on the day it started, which is the whole reason the setting is configurable. Same boundary the `category.daily_limit` budget is measured against.
2. The displayed total for a category = the sum of `time_entry.duration_seconds` for every `time_entry` row with that `category_id` overlapping the current window, **plus** the elapsed time of a currently in-progress segment if the device is right now on a face mapped to that category. The in-progress segment has no `time_entry` row yet, which is what keeps it from being counted twice.
3. Because faces map to categories many-to-one (see [Workflow § faces map to categories many-to-one](workflow.md#faces-map-to-categories-many-to-one)), this sum must include `time_entry` rows created from *every* face mapped to that category, not just whichever face is currently active. Keying the totals by face instead is exactly how this went wrong: two faces sharing a category each counted alone, so 40 minutes on one and 40 on another left a 60-minute `daily_limit` unreached and the menu bar drew one face's figure beside the shared category's name.
4. `time_entry.category_id` is what the sum groups by, deliberately, rather than re-deriving the category by joining `device_event` to `face`. An entry records the category the face was mapped to **when the segment happened**; the current mapping would move a day's work to whatever the face points at next. An entry is given its category when the segment is converted, and never re-derived afterwards.
5. At the reset boundary, every category's displayed total returns to zero, regardless of whether a segment happens to be in progress at that moment — a live segment spanning the boundary counts only the portion after it toward "today's" total; the portion before belongs to the previous day.

## 7. Reaching a category's daily limit

`category.daily_limit` is a **hard** limit, not a warning: the figure from § 6 reaching it stops the cube. Implemented by `DailyLimitEnforcement` (every rule, and the reasoning) plus `DailyLimitWatch` (the figures, the timer and the writes). `daily_limit = 0` means no limit, so nothing below applies to it.

1. When the category on show reaches its limit, the app pauses the device (`0x06 0x01`) and then **refuses to send the unpause** while that category is still the one on show. The refusal is the enforcement, and it is the app's rather than the device's: nothing in the protocol asks the device to hold a pause against its own user, so `0x06 0x02` is honoured whenever it arrives. The limit is therefore exactly as hard as the set of paths that can send it, which is why they are enumerated in point 3.
2. The pause fires on the second the budget is spent, from a one-shot timer, not on the menu bar's display tick — that tick is a whole minute wide when the seconds preference is off, and how far a hard limit overruns must not depend on a display setting.
3. Three paths can unpause a cube, and the limit answers each differently:
   - The dropdown's **Resume** item and the status item's **right half** both reach `togglePause`, which declines to send while the budget is spent. The dropdown item is also greyed, so the refusal is visible rather than a click that silently does nothing. **Pausing** is never refused, only resuming. Locking is not refused either: unlocking is the one way back to a cube nobody can otherwise operate.
   - A **double tap on the cube** pauses and unpauses it in firmware, telling the app afterwards (see the Double tap characteristic in [the protocol spec](TimeFlip2%20BLE%20Protocol%20v4.3.md)). It cannot be refused, only answered: the history frame reports the cube running, and the pause goes straight back out.
4. Flipping to a face whose category still has budget **resumes** the cube automatically, and flipping back to the spent face pauses it again. Pause is a property of the cube where a limit is a property of a category, so holding the pause across a flip would spend one category's budget and stop the whole day's tracking with it. Only a pause the limit itself asked for is lifted this way — a pause the user asked for is theirs.
5. A category that has reached its limit **stays** reached until the next `daily_reset_time` boundary, or until its `daily_limit` is edited. It deliberately does not follow the total back down, because the total dips on its own: pausing stops the live segment counting, and the segment that ran the budget out is not a `time_entry` until the pause's history fetch ingests it, so for a moment the figure reads under the limit. Re-deriving in that window resumes the cube on the strength of a total the app knows is incomplete. An edit to the limit is answered immediately, being the one signal a stale total cannot imitate.
6. None of this is persisted. The device holds its own pause across a quit, and the totals are re-derived from `time_entry` on launch, so a relaunch onto a paused cube resting on a spent category re-arms the hold from those two facts alone.
7. **It applies while the app is the clock too**, and that is the half `swift test` and `12-daily-limit` can check without a cube: there the stop is the app closing its own open segment rather than `0x06 0x01` going out, and every path that would start it again asks the same question. What is deliberately *not* done in that case is the automatic resume in point 4. With a cube, `.resume` carries on counting a face somebody is resting on; with no cube it would be the app recording time against a category nobody has come back to. Raising the limit lifts the refusal, and starting the clock again stays the user's to do.

## Related documents

- [Workflow](workflow.md) — the intended usage this pipeline serves: recurring vs. short-lived categories, and how faces map to categories.
- [Database Design](database-design.md) — full schema, column-by-column.
- [`crates/facet-core/resources/database/CLAUDE.md`](../crates/facet-core/resources/database/CLAUDE.md) — naming and storage conventions the schema follows.
- [`docs/timeflip.md`](timeflip.md) / [`docs/TimeFlip2 BLE Protocol v4.3.md`](TimeFlip2%20BLE%20Protocol%20v4.3.md) — the device's wire protocol this pipeline decodes (official spec takes priority per the root [`CLAUDE.md`](../CLAUDE.md)).
