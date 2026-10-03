# The TimeFlip2 BLE surface, as this app uses it

[← Back to README](../README.md) · [Firmware observations →](timeflip2-firmware-observations.md) · [BlueZ notes →](linux-bluez-port-notes.md) · [Operation spec →](operation-spec.md)

**How the cube exposes itself over BLE, and what this app does with each part of it.** This is the
reference for changing the radio half or a test double against it.

**Where it sits in the hierarchy set out in `CLAUDE.md`:**
[`TimeFlip2 BLE Protocol v4.3.md`](TimeFlip2%20BLE%20Protocol%20v4.3.md) is authoritative,
**this file is how the app reads it**, and
[`timeflip2-firmware-observations.md`](timeflip2-firmware-observations.md) records what the hardware
actually does where the two disagree. **The hardware wins**, and where it does, this file says so and
cites the finding.

**Written from the Rust implementation and naming mechanisms rather than types.** Where a behaviour needs
its implementation read, it is in `crates/facet-core/src/device/` (the rules, against the `Link` port),
`crates/facet-ui/src/device.rs` (the Device tab and the held link) and `crates/facet-adapters/src/radio.rs`
(btleplug); `crates/facet-core/src/device/fake.rs` is the test double. The tests that pin it down are in
[behaviour-inventory.md](behaviour-inventory.md).

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

Write the command to `...54`, then read the answer from the command result characteristic (`...53`).
`[cmd, 0x02]` is success in the vendor format, and some firmware builds send a lone `0x02`, but this app does
not judge either: the write's acknowledgement and the answer are traced, and only a read-back, where one
exists, is believed.

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
| `0x16` / `0x17` | Write / read accelerometer double-tap registers |
| `0x30` | Set new password, 6 bytes |
| `0xFE` / `0xFF` | Reset task info / factory reset |

The app issues `0x04`, `0x05`, `0x06`, `0x07`, `0x08`, `0x09`, `0x0A`, `0x10`, `0x11`, `0x15`, `0x16`, `0x17`,
`0x30` and `0xFF`. The rest (`0x13`, `0x14`, `0xFE`) are understood and unexercised.

### Confirming a command took effect

**The write-ack says the cube accepted the command. It says nothing about the resulting state**, and the
protocol never pushes an unsolicited notification when a command changes something. There is no single
confirmation mechanism, so there are three cases.

- **A dedicated read-back exists**: `0x10` (lock, pause, auto-pause), `0x17` (double-tap registers), `0x07`
  (the clock). **Write, read back, compare, and only then believe it.**
  This is a standing rule in `CLAUDE.md`, not a per-command choice.
- **`0xFF` is confirmed by logging in**: the cube keeps the link up through the wipe and goes on taking its old PIN
  for several seconds (finding 6), so the app lets go and presents `000000` every 3 seconds, for up to 120, on
  connections of its own. Only an accepted vendor PIN counts as the reset having happened.
- **No read-back is defined**: `0x09`, `0x0A` (LED), `0x11` (face colour), `0x15` (name). For the first
  three the table is the account: LED brightness and blink are sent on every connect and again when the
  cube asks for them through the system-state sync-required codes, and face colours are sent on every
  connect (see Face colours).
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

Auto-pause, LED brightness and blink interval are edited live, through steppers whose arrows can be clicked
several times in quick succession, firing several intermediate values. Every change does two things:

1. **Logs the edit and restarts a debounce of 0.5 s for that setting.** Only the value still current when it
   runs out reaches the cube, once. 0.5 s is `EDIT_QUIET_FOR` in `facet-ui`'s `device.rs`. Each
   setting has its own debounce, so editing one does not cancel another's pending write.
2. **Holds the field at the edited value** until the send has ended. The tab redraws after every outcome, and
   it does not read these three fields back from the table while an edit is unsent or out with the cube. When
   the cube has taken the value the table is written, and the field then shows the table again. A refusal
   puts the table's value back and says so in a notice.

The table is written only once the cube has the value, which is what the read-back rule asks of a device
setting.

**Writes that are not a settling value are not debounced, and must not be**: lock and pause (a click that
must act at once, including the pause-and-lock-before-quit sequence), the clock, the password, the name,
and the colours pushed on connect or in answer to the cube's own resync request. Delaying any of those
either makes the UI feel broken or races a teardown.

**Booleans are in that group**, a checkbox having no intermediate values to wait out: Pause on lock is
written to the table as it is pressed. There is no double-tap control. At every login the app reads the
cube's registers with `0x17` and, unless the window is already 0, sends `0x16` with the cube's own
threshold, limit and latency and a window of 0, then reads them back (`session::turn_double_tap_off`).
Disabling is faked this way because no command disables the gesture (finding 11).

### Face colours

**Colours are not edited as a value of their own**: they follow the faces' categories, so what changes
them is assigning a face a different category or recolouring a category. All twelve faces resolve through
the category's colour row to a device RGB. **A face with no category, or a category with no colour,
resolves to black, which is the protocol's only way to say "off".**

Everything is logged under the `colour` tag: one line per face written, naming the face, its category, the
colour (`off` for black) and the 16-bit values `0x11` carries, because hex is 8 bits per channel and the
command takes 16, so a scaling problem shows up rather than having to be inferred. Each line says why it
went and that there is no read-back to confirm it.

**Because `0x11` has no read-back, the app keeps no record of what it last sent.** The faces and their
colours are read from the table at the moment of sending (`Device::send_face_colours`), and which faces go
depends on what asked:

- **A face given a category, or a category recoloured, retired or reassigned** sends only the faces it
  touched.
- **Every connection** sends all twelve, after the login's own questions, so a cube that was factory reset,
  re-paired or coloured by another app is corrected on its next connect.
- **A cube request** (system state `0x02 0x02`) and **a factory reset report** (`0x01 0x00`) send all
  twelve.

**Cube requests are answered once and then held off for 30 seconds.** The cube repeats itself freely, once
per notification, again per post-reconcile re-read, and again on every reconnect while it is unhappy.
**Answering each one measurably made things worse**: each answer is twelve flash writes that also light the
LED, and with flat batteries the cube was rebooting, so 8 requests in one second became 96 colour writes
that helped brown it out further. A request inside the 30 seconds is logged and dropped.

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

A fetch first reads the cube's latest event (`0x01 FFFFFFFF`). When that is the row on record, it writes
that one event again for its duration and asks for nothing more. Otherwise it writes `0x02 <event#>` once and
reads the frames the cube sends as notifications, stopping at the sentinel, at a frame it cannot read, after
6 seconds with no frame, or at 2048 frames.

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
SELECT event_number, start_epoch FROM device_event WHERE device_face BETWEEN 1 AND 12 ORDER BY start_epoch DESC, event_number DESC, device_event_id DESC LIMIT 1;
```

Rows on the app's own faces (13 and 14) are not the cube's and are left out.

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

**What the app does, in order.**

1. **Wait for the radio.**
2. **Scan unfiltered and match on service or name.** A service-filtered scan finds nothing (finding 12).
3. **Connect, discover services, discover every characteristic in §2.**
4. **Write the password.** **Pairing presents the vendor PIN first and then each stored PIN; reconnecting
   presents each stored PIN first and then the vendor PIN** (`login::pairing_candidates`,
   `login::reconnect_candidates`), **each on a connection of its own** and no guess beyond them. A cube
   accepted on the vendor PIN is moved onto a PIN of the app's own with `0x30` and proved by a login on it.
   A second attempt on the same peripheral must let the first one go.

   **The stored PIN has two homes.** The secret store is where it belongs. `config.json` beside the databases,
   under the key `PIN`, holds it only when the store refused a write, so that a cube moved onto a PIN of the app's
   own is never left on one nothing can name (only taking its batteries out puts it back on the vendor PIN). Both
   are read at the moment they are wanted. The file's PIN is presented first, being the newer, and a PIN both hold
   is presented once. The file is cleared as soon as it is redundant: when the store holds the same PIN, or once a
   login has proved the PIN and the store has taken it. A file that is not a JSON object is never overwritten, and
   the file's other keys are left alone. A secret store that will not
   answer, with nothing in the file, still stops the login before any PIN is presented.
5. **Subscribe by property, not by list.** Subscribe to every characteristic whose properties say it can
   notify, so nothing the cube can push is silently unsubscribed and therefore never sent. The archived
   driver named five characteristics and that is how a notification goes missing.
6. **Initialise**: set the clock (`0x08`, read back with `0x07`), read the GAP name, the Device Information
   strings and the battery, subscribe, read the face and the status (`0x10`). Then record the login, fetch
   history, arm the history timer, send all twelve face colours, send LED brightness and blink, check the
   double-tap registers, set auto-pause only when the cube's `0x10` answer differs from the table, and read
   the system state.
7. **Steady state**: translate characteristic updates into events. What is durable is a row (see
   [database-design.md](database-design.md), `device_info`). The Device holds only what is true of the link
   and is the cube's to say, and drops it when the link goes: the face last reported, the last `0x10` answer
   (the lock is in no table), the charge, and the pause claim.
8. **Every command with a read-back defined is read back before it is believed.**

**The Device Information strings are read after a login and only after one**, they are exact length rather
than padded to 20 bytes, and all four answer quickly (finding 5).

## 7. Operational rules

- **Write the password immediately after connecting.** Many commands silently fail otherwise, and face
  notifications return `0`.
- **After a factory reset (system state `0x0100`), run the full sync**: set the clock, push all twelve face
  colours, LED brightness and blink, check the double-tap registers, set auto-pause when the cube's differs
  from the table, and fetch history. Task parameters are not part of it: the app never sets them. **A factory reset does not clear the auto-pause
  delay** (finding 10) and **does not drop the connection** (finding 6): the command is acknowledged, the
  cube is genuinely erased, and the link carries on as though nothing happened. Anything waiting for the
  cube to react is waiting for something that never comes.
- **Treat face `66` as a hard fault** and surface it.
- **Enforce a frame cap on history dumps** (2048) and stop on the sentinel, to avoid a hang.
- **The double-tap registers survive a factory reset** (finding 11a), and **no command disables double tap**:
  a window of 0 is the only lever, and the app sets it at every login (finding 11).

## 8. Where the spec is wrong

Four places, each measured. [`timeflip2-firmware-observations.md`](timeflip2-firmware-observations.md) is
the evidence.

| Spec says | Hardware does |
|---|---|
| Password OK is `0x01` | **`0x02`**, on two unrelated Bluetooth stacks (finding 4). **Implemented from the spec, every correct PIN is refused and every wrong one accepted.** The most load-bearing byte in the app |
| Duration is five bytes little-endian | **Four bytes big-endian** |
| An empty history answer is all zeros | **Bytes 13–16 carry the cube's clock** (finding 14) |
| Nothing about the advertisement | **It carries no service UUID** (finding 12) |

One more where the spec is merely silent and the app has to choose: `0x11` takes 16 bits per channel in
practice where the examples imply 8. `0x14`'s elapsed-seconds endianness varies too, so the smaller non-zero
reading of the two is the safe one, but the app never sends `0x14`.
