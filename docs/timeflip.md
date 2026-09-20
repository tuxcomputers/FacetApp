# The TimeFlip2 BLE surface, as this app uses it

[← Back to README](../README.md) · [Firmware observations →](timeflip2-firmware-observations.md) · [BlueZ notes →](linux-bluez-port-notes.md) · [Operation spec →](operation-spec.md)

**How the cube exposes itself over BLE, and what this app does with each part of it.** This is the
architectural source for writing the radio half or a test double against it.

**Where it sits in the hierarchy set out in `CLAUDE.md`:**
[`TimeFlip2 BLE Protocol v4.3.md`](TimeFlip2%20BLE%20Protocol%20v4.3.md) is authoritative,
**this file is how the app reads it**, and
[`timeflip2-firmware-observations.md`](timeflip2-firmware-observations.md) records what the hardware
actually does where the two disagree. **The hardware wins**, and where it does, this file says so and
cites the finding.

**Carried over from the Swift implementation and rewritten to name mechanisms rather than types.** Where
a behaviour needs its implementation read, the citation is to the frozen TimeFlipApp repository; the
corresponding tests are in [behaviour-inventory.md](behaviour-inventory.md).

---

## 1. The model

- A 12-face accelerometer timer. **All state is in volatile RAM and resets when the coin cell is
  removed** (vendor doc v4.3, rev 20.02.2022).
- The host is the GATT client. The cube exposes one vendor service plus Battery (`0x180F`) and Device
  Information (`0x180A`).
- **Authentication is a six-byte ASCII password written to the password characteristic on every
  connection.** It resets on every disconnect. The vendor default is `000000`.
- A session is an ordered sequence: radio ready → scan and connect → discover → write the password →
  subscribe → host-led initialisation (set the clock, read the status) → steady event stream.

## 2. The GATT surface

Vendor service `F1196F50-71A4-11E6-BDF4-0800200C9A66`:

| UUID | Name | Properties |
|---|---|---|
| `...51` | Events data | R/N, ASCII |
| `...52` | Faces | R/N, current face 1–12, `0` if undefined or the password was rejected |
| `...53` | Command result | R, 20 B |
| `...54` | Command | R/W, 20 B |
| `...55` | Double tap | N, 1 B |
| `...56` | System state | R/N, 4 B |
| `...57` | Password | **W only**, 6 B ASCII |
| `...58` | History | R/W/N, 20 B |

Standard: Battery Level `2A19` (1 B percent, R/N), Device Information `2A29/2A24/2A27/2A26/2A23` (R).

**The characteristics report their own properties and they settle one design question.** The password
characteristic is `0x08`, write-with-response **only**, so a write with response is not a reliability
choice, it is the only thing the characteristic supports. The command result is `0x12`, read plus notify,
so the answer can be read or subscribed to (finding 4).

**The cube advertises no service UUID at all.** A service-filtered scan finds nothing. Scan unfiltered and
match on the advertised service **or** the name (finding 12). This is also why the Rust port could not use
a backend that only reaches already-paired devices; see [rust-port.md](rust-port.md).

## 3. The command channel (`...54`)

Write, then read the same characteristic back. `[cmd, 0x02]` is success in the vendor format, and a lone
`0x02` is tolerated because some firmware builds send it.

| Op | Meaning |
|---|---|
| `0x04` | Lock mode on/off |
| `0x05` | Auto-pause minutes (u16, 0 disables) |
| `0x06` | Pause mode on/off |
| `0x07` | Read device time → `[0x07][unix seconds, u64 big-endian]` |
| `0x08` | Set device time, 8 bytes big-endian |
| `0x09` | LED brightness, 1–100 % |
| `0x0A` | LED blink interval, 5–60 s |
| `0x10` | Status → lock (`0x01`/`0x02`), pause (same, unless locked), auto-pause minutes (u16) |
| `0x11` | Set face colour: face id, then 16-bit R, G, B |
| `0x13` | Set task parameters: face, mode (0 simple, 1 pomodoro), pomodoro seconds (u32) |
| `0x14` | Read task parameters: face, mode, limit, elapsed seconds |
| `0x15` | Set device name: length, then ASCII |
| `0x16` / `0x17` | Read / write accelerometer double-tap registers |
| `0x30` | Set new password, 6 bytes |
| `0xFE` / `0xFF` | Reset task info / factory reset |

The app issues `0x05`, `0x06`, `0x08`, `0x10`, `0x11`, `0x14`, `0x15`, `0x17`, `0x30`, `0xFF`. The rest
are understood and unexercised.

### Confirming a command took effect

**The write-ack says the cube accepted the command. It says nothing about the resulting state**, and the
protocol never pushes an unsolicited notification when a command changes something. There is no single
confirmation mechanism, so there are three cases.

- **A dedicated read-back exists**: `0x10` (lock, pause, auto-pause), `0x14` (task parameters), `0x17`
  (double-tap registers), `0x07` (the clock). **Write, read back, compare, and only then believe it.**
  This is a standing rule in `CLAUDE.md`, not a per-command choice.
- **No read-back is defined**: `0x09`, `0x0A` (LED), `0x11` (face colour), `0x15` (name). For the first
  three the app is the system of record: what it last sent is the only account of what the cube holds,
  and the cube asks for a value back through the system-state sync-required codes when it has lost one.
  **`0x15` is the measured one and it is worse than absent from the spec**: the cube never updates the
  command result characteristic for it at all (finding 2), so even `[cmd, 0x02]` never arrives. A rename
  is confirmed by the *next connection* reporting the GAP name, which is a different mechanism and cannot
  be asked for. The name is therefore the exception among the four: the cube does report it, just never in
  answer to the write, so there the app follows the cube rather than standing in for it.
- **A read is impossible by nature**: `0x30`. Confirmation is functional, attempt a real login with the
  new password and treat the rotation as successful only if that login succeeds.

**Reading the clock back needs drift, not plausibility.** A factory-reset cube reports a **stale** clock,
not an unset one: 11 May 2020 on the cube under test, which is a real date and passes any sanity check
that is not comparing it against the host (finding 13). The only honest test is drift from this machine's
clock.

### Debouncing live-edited settings

Auto-pause, LED brightness, blink interval, the double-tap registers and a face's assigned category are
all edited live, through press-and-hold steppers and click-through lists, which fire many intermediate
values in quick succession. Every change does two things:

1. **Persists to the database and logs immediately**, so the database and the debug log always reflect the
   live value, even mid-hold.
2. **Reschedules a debounce of 2 s.** Only the value still current 2 s after the last change reaches the
   cube. Every debounced write shares that one constant so the whole UI settles at the same rate, and each
   setting has its own debouncer so editing one does not cancel another's pending write.

**Writes that are not a settling value are not debounced, and must not be**: lock and pause (a click that
must act at once, including the pause-and-lock-before-quit sequence), the clock, the password, the name,
and the colours pushed on connect or in answer to the cube's own resync request. Delaying any of those
either makes the UI feel broken or races a teardown.

**Booleans are in that group**, a checkbox having no intermediate values to wait out. The double-tap
*disable* control is the one that reaches the cube and it shares its path with the register values
(disabling is faked by sending a window of 0), so that path takes an *immediately* flag: the checkbox
passes true, the steppers pass false. **An immediate write cancels any pending register write first**,
because that one carries parameters worked out before the flag flipped and letting it land afterwards
would undo the toggle.

### Face colours

**Colours are not edited as a value of their own**: they follow the faces' categories, so what changes
them is assigning a face a different category or recolouring a category. All twelve faces resolve through
the category's colour row to a device RGB. **A face with no category, or a category with no colour,
resolves to black, which is the protocol's only way to say "off".**

Everything is logged under the `sync-colour` tag: one line per face actually written, naming the face, its
category, the colour, the hex **and** the 16-bit values `0x11` carries, because hex is 8 bits per channel
and the command takes 16, so a scaling problem shows up rather than having to be inferred. Then a closing
line with the count and the reason.

**Because `0x11` has no read-back, the app's record of what it last sent decides what goes out:**

- **A category edit** writes only the faces whose colour changed.
- **The first connect of a run, and any fresh pairing, write all twelve.** Until something has actually
  been sent, the record is an assumption seeded from the database and is no evidence at all. A cube that
  was factory reset, re-paired or coloured by another app would otherwise be left wrong indefinitely.
- **Later reconnects in the same run** write only what drifted, which in practice means a face reassigned
  while the cube was away. By then the record is real, and **the cube keeps its colours across a dropped
  link**, which is why the record deliberately survives a disconnect.
- **A cube request** (system state `0x02 0x02`) writes all twelve regardless, because the cube is saying
  it no longer has them.

**Those requests are collapsed, not answered one for one.** The first schedules a resync one debounce
delay later; anything arriving while one is pending, running, or inside a **30 s cooldown** is counted and
dropped, with the count logged. The cube repeats itself freely, once per notification, again per
post-reconcile re-read, and again on every reconnect while it is unhappy. **Answering each one measurably
made things worse**: each answer is twelve flash writes that also light the LED, and with flat batteries
the cube was rebooting, so 8 requests in one second became 96 colour writes that helped brown it out
further. A new connection clears the cooldown, so a cube that really has lost its colours is still
answered promptly.

## 4. Notification semantics

- **Faces (`...52`)**: 1 byte, face 1–12. `0` means undefined **or** the password was rejected.
- **Double tap (`...55`)**: 1 byte. `<128` is a face with pause off; `>=128` is pause on with face
  `value − 128`.
- **System state (`...56`)**: 4 bytes. Bytes 0–1 are sync state: `0000` ok, `0100` factory reset, `0201`
  time sync required, `0202` face colour, `0203` LED brightness, `0204` blink interval, `0205` task
  parameters, `0206` auto-pause. Bytes 2–3 are hardware status: `0000` ok, `0201` accelerometer error,
  `0202` flash error, `0203` both.
- **Events data (`...51`)**: ASCII. **The spec calls this diagnostics and it is more than that**, it is
  the only completion signal present for *all* commands, arriving 40–310 ms after the write (finding 3).
  It also narrates `password OK` on a correct PIN and `New Side: 0x00` on every face change, the latter
  always `0x00` across seven different faces, so it carries no usable face.
- **Battery (`2A19`)**: 1 byte percent. **Pushed only when it changes, and it changes constantly**: the
  level dithers across one percent and each waver is a notification (finding 7).

## 5. The history stream (`...58`)

**Commands:** `0x01 <event#>` for a single entry, `0x02 <event#>` for a sequential stream that stops at a
sentinel.

**Frame layout** (spec v4.3 and observed firmware):

| Bytes | Field |
|---|---|
| 0–3 | Event id, u32 big-endian. `0xFFFFFFFF` asks for the last. **`0` means the cube has no such event** |
| 4 | Face. `>127` is a pause event for `value − 128`; `66` signals an accelerometer error; `0` is invalid |
| 5–12 | Flip timestamp, seconds since epoch, u64 big-endian |
| 13–16 | Duration, **four** bytes |
| 17–19 | Streamed (`0x02`) form only: a previous-event pointer. **A single-event (`0x01`) answer is 17 bytes** |

**Check `event == 0` before parsing anything else.** The documented all-zero sentinel does **not** catch
an empty answer: on a cube with no history, bytes 13–16 carry the cube's own clock rather than a zero
duration, so a parser that checks the sentinel and then reads the duration gets 1,589,190,600 seconds for
an event that does not exist (finding 14).

**The duration is big-endian**, observed on firmware shipping 2026-01 and confirmed 2026-09-20. Firmware
has disagreed with the spec about byte order, so the tolerant reading is to take both and prefer the
smaller non-zero value: a plausible duration is small, and 90 seconds read backwards is 1,509,949,440.

**An earlier version of this file said the duration was five bytes little-endian at 13–17, and it was
wrong on both counts.** A rebuild inherited that and rejected every single-event answer as too short,
logging "a frame this app cannot read" while the cube answered correctly (2026-08-21). The vendor table is
the authority here.

**Intervals shorter than 5 seconds are never in history.** The spec says `0x02` returns intervals lasting
at least 5 s, so a quick flip, or a flip a few seconds after an unlock, leaves nothing to fetch. **An
empty answer is an ordinary state and not evidence of a fault.**

A fetch writes `0x02`, increments the event number per frame, caps at 2048 frames, and stops on the
sentinel or on a parse failure.

### What the live record does

- **The last frame in every dump is the current interval snapshot**, even when paused; its face byte is
  `>=128` when paused.
- **The cube reuses the same event number for the current interval** and refreshes its duration roughly
  every 5 s, so duration updates arrive on the same event number.
- **So the host must not advance its cursor past the last frame**, or refreshed durations for the
  in-progress interval are missed.

### Ingestion rules

**There is no cursor, stored or in memory.** The resume position is a query, re-read on every refresh:

```sql
SELECT event_number, start_epoch FROM device_event ORDER BY start_epoch DESC, device_event_id DESC LIMIT 1;
```

**Start *at* that number, not past it.** The newest row is normally the cube's still-open segment, and
asking for it again is how its finished duration comes back.

**The newest row, not the highest number.** Those are different questions and only the first is useful.
**Event numbers restart at 1 after a factory reset**, so `MAX(event_number)` returns a stranded value from
a counter generation the cube has abandoned: on the production database it returned 38, from a dead
generation, while the newest segment was event 10.

**A position the cube cannot reach is not used.** Take the stored row and the cube's own last event (a
live single-frame read) and use the stored position only if the cube's event is at or after it in **both**
the counter and the clock. Failing either, start the stream from 0. Two ways it fails:

- **A lower number.** The counter restarted at 1, which is what a factory reset does.
- **An earlier `start_epoch`.** Within one generation a later event never begins before an earlier one, so
  a "newer" event that started before the row on file proves the generation changed. **This is what
  catches a reset that has already counted back up to the stored number**, where the numbers match and
  nothing about them looks wrong. Measured against a copy of the production database: comparing numbers
  alone took the "nothing changed" path and wrote one row where ten were due, losing events 1–9 of the new
  generation silently.

**Known gap.** A cube reset while the app is not running, which then counts *past* the stored position
before the next launch, reports both a higher number and a later start, so both tests pass. The two
generations merge and the new generation's first events are never ingested. Telling that case apart needs
a second read: asking for the cube's own event 10 and seeing that it began at a different second from the
row on file. **Not currently handled.**

**One fetch at a time, and a request arriving during one is run afterwards rather than dropped.** Two
fetches at once would be two conversations on one characteristic with nothing in the answers to say which
is which. Dropping a mid-fetch request was tried and was wrong for six of the seven callers: only the
timer can afford to wait, while the others ask *because* something has just changed, the cube was turned,
locked, unlocked, paused, reset, or a link came up. **However many arrive during one fetch, they collapse
into a single re-run.**

**Every frame that lands is written, in ascending order, with no "have I seen this number before?" filter
in front of it.** Rows match on `(event_number, start_epoch)`, so re-writing a segment already on record
updates it in place, and the database is the only thing that answers correctly across a reset, a filter
keyed on the number alone discards a whole post-reset generation as already seen. All but the last frame
are written closed; the last is the open segment, whose duration grows on each refresh until a later event
closes it out.

**The one case where the last frame is not written is an ambiguous one.** A stream cut short by a dropped
connection can end on a frame that is really already closed, with unfetched history beyond it. Trust the
last frame as current only when its event number is at least the cube's own reported last event **and**
every frame ahead of it committed. Failing either, **withhold the whole batch**, not just the live frame:
open-versus-closed is decided from the newest visible row, so a partial batch would leave a stretch that
finished long ago drawn as what is happening now. The next refresh resumes from the same point.

**The day's totals are per category and derived, not accumulated.** Re-sum the entries overlapping the
current window every time, and add the open segment's elapsed time on top. Nothing is kept between calls,
so there is no seeding step and no tally to keep in step. **A running tally would double-count the moment
a segment was re-written**, which this app does deliberately. Per category, because that is what a daily
limit is set on. See [Operation Spec § 6](operation-spec.md).

## 6. The connection sequence

**What the app does, in order.** The archived predecessor's version of this sequence, and the rebuild's,
are both in the frozen repository; the differences below are the ones that were paid for.

1. **Wait for the radio.**
2. **Scan unfiltered and match on service or name.** A service-filtered scan finds nothing (finding 12).
3. **Connect, discover services, discover every characteristic in §2.**
4. **Write the password.** **Two candidates and no third guess**: the vendor default, then the stored one,
   **each on a connection of its own**. There is no "fall back to the default if the user's fails",
   because there is no user-supplied PIN. A second attempt on the same peripheral must let the first one
   go.
5. **Subscribe by property, not by list.** Subscribe to every characteristic whose properties say it can
   notify, so nothing the cube can push is silently unsubscribed and therefore never sent. The archived
   driver named five characteristics and that is how a notification goes missing.
6. **Initialise**: set the clock (`0x08`), read the status (`0x10`) and the system state, normalise
   auto-pause to the preference, read the Device Information strings.
7. **Steady state**: translate characteristic updates into events. **Nothing accumulates a picture of the
   cube in memory.** Each question is asked when its answer is wanted, and what is durable is a row (see
   [database-design.md](database-design.md), `device_info`). The archived driver kept a snapshot object
   and the rebuild deliberately does not.
8. **Every command with a read-back defined is read back before it is believed.**

**The Device Information strings are read after a login and only after one**, they are exact length rather
than padded to 20 bytes, and all four answer quickly (finding 5).

## 7. Operational rules

- **Write the password immediately after connecting.** Many commands silently fail otherwise, and face
  notifications return `0`.
- **After a factory reset (system state `0x0100`), run the full sync**: set the clock, push the face
  colours and LED settings, task parameters, auto-pause. **A factory reset does not clear the auto-pause
  delay** (finding 10) and **does not drop the connection** (finding 6): the command is acknowledged, the
  cube is genuinely erased, and the link carries on as though nothing happened. Anything waiting for the
  cube to react is waiting for something that never comes.
- **Treat face `66` as a hard fault** and surface it.
- **Enforce a frame cap on history dumps** (2048) and stop on the sentinel, to avoid a hang.
- **The double-tap registers survive a factory reset** (finding 11a), and **no command disables double tap**
 , sensitivity is the only lever (finding 11).

## 8. Where the spec is wrong

Four places, each measured. [`timeflip2-firmware-observations.md`](timeflip2-firmware-observations.md) is
the evidence.

| Spec says | Hardware does |
|---|---|
| Password OK is `0x01` | **`0x02`**, on two unrelated Bluetooth stacks (finding 4). **Implemented from the spec, every correct PIN is refused and every wrong one accepted.** The most load-bearing byte in the app |
| Duration is five bytes little-endian | **Four bytes big-endian** |
| An empty history answer is all zeros | **Bytes 13–16 carry the cube's clock** (finding 14) |
| Nothing about the advertisement | **It carries no service UUID** (finding 12) |

Two more where the spec is merely silent and the app has to choose: `0x11` takes 16 bits per channel in
practice where the examples imply 8, and `0x14`'s elapsed-seconds endianness varies, so take the smaller
non-zero reading of the two.
