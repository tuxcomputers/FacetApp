# TimeFlip2 firmware observations

Behaviour measured on real hardware that the vendor spec does not describe, and in one case contradicts. Everything here was observed directly, with the debug log rows to prove it; nothing is inferred from the protocol document.

This is the third source in the hierarchy set out in the root `CLAUDE.md`: [`TimeFlip2 BLE Protocol v4.3.md`](TimeFlip2%20BLE%20Protocol%20v4.3.md) is authoritative, [`timeflip.md`](timeflip.md) describes the BLE surface as this app uses it, and **this file records what the hardware actually does where the spec is silent**. Where this file and the spec disagree, the hardware wins, because these are measurements.

**Device under test.** Manufacturer `DI_LABS`, model `2.0`, hardware `TFv4.1`, firmware `FW_v3.64`, read from the Device Information service. Every finding here was measured against that one cube.

**Three host stacks produced these, and which one matters.** Most were taken on macOS over CoreBluetooth, 2026-08-01/02, with findings 4 and 5 re-read on 2026-08-17. Finding 12, the Linux notes inside findings 3, 4, 8 and 10, and the address paragraphs were taken on Linux Mint 22.3 over BlueZ 5.72: the Python probe on 2026-09-06 and Swift over libdbus on 2026-09-07. **Findings 13 and 14 were taken on 2026-09-20 by [`probe/timeflip-btleplug`](../probe/timeflip-btleplug/)**, a Rust program over `btleplug`, which is the stack this app is being rebuilt on. Finding 7 is older traffic from the same cube, 2026-07-28 to 2026-08-14, recorded by an archived predecessor rather than by the app.

**Why the stack is recorded against each one.** A fact measured through two unrelated stacks is a fact about the cube; a fact measured through one may be a fact about the host. Finding 8 is the clearest case and finding 4 the most load-bearing, and both are now confirmed on more than one stack.

## The evidence file

[`timeflip2-firmware-evidence.sqlite`](timeflip2-firmware-evidence.sqlite) sits beside this document and holds the debug log rows every claim below rests on. Every row id quoted here is a row in it.

753 rows in one `debug_log` table, carrying the source database's original `debug_log_id` values and timestamps, so ordering by id is true chronological order (verified: no row's timestamp precedes the row before it). It covers the rename history across the whole test session **plus one complete, unedited BLE trace** of a connect-and-rename from id 6716, so the acknowledgement claims can be checked against a full sequence rather than a flattering selection.

Rows 356 to 379 are finding 4, added on 2026-08-17 from a scripted run: the whole of two login attempts, both PINs and both answers, unedited. They come from the test database, which every run rebuilds from the DDL -- so without copying them here the evidence for that finding would have been destroyed by the next run.

Rows 309 to 319 are finding 5, added the same day from a driven run, and copied for the same reason: the accepted login, the pairing it wrote, and the four Device Information reads that followed, with the raw bytes of each.

Rows 340 to 356 and 384 to 399 are finding 6, from the same day: the `0xFF` write and its acknowledgement, the silence that followed it, and then the login two minutes later that proves the wipe took by being accepted on the vendor PIN. The gap between rows 355 and 356 is the finding — nothing was written in it because nothing happened.

Rows 14520 to 14534 and 19557 to 19580 are finding 7, and they are the odd ones out here: they come from the **archived app's own production database**, not from a scripted run of this one, because the charge is the one thing the rebuild had never read at the time the finding was written. Both stretches are unedited runs of consecutive ids. The first is a connect sequence with the battery read inside it; the second is seven minutes in which nothing was asked for and values arrived anyway.

Rows 20001 to 20051 are finding 12 and the Linux paragraphs of finding 8, added on 2026-09-06. **They are the only rows here that no `debug_log` ever held**: they come from `scripts/linux-ble-probe.py` on a Linux host, which prints rather than writes rows, so they were transcribed from its output. The id range is deliberately clear of every other, so a query can exclude them and get only what this app and the archive recorded. They cover two scans filtered on the service UUID that saw nothing, the unfiltered scan that saw the cube at once, the advertisement's own properties, the whole GATT tree, a login on the vendor default PIN, and sixteen face notifications.

Rows 20101 to 20125 are the Linux notes added on 2026-09-07, and they are the second set here that no
`debug_log` ever held: they come from a Swift probe over **libdbus**, driving `FacetCore`'s own
`SystemBus`, `BlueZRadio` and `BlueZGatt` rather than the Python of rows 20001 to 20051, and were
transcribed from its output for the same reason. Their id range is again clear of every other. They cover
a login on the vendor default and its answer, the `0x10` status read on a cube fresh from a factory reset,
the two events-data strings finding 3's table does not have, and ten face pushes over seven faces from a
passive listen -- no reads while listening, which matters here (see the note on finding 3).

```
sqlite3 docs/timeflip2-firmware-evidence.sqlite \
  "SELECT debug_log_id, logged_at, tag, message FROM debug_log ORDER BY debug_log_id;"
```

| Tag | Meaning |
|---|---|
| `ble-tx` | bytes written to the device |
| `ble-rx` | bytes received from it |
| `face-task` | the app's own rename lifecycle |
| `field` / `click` | the user action that started it |
| `scan` | one scanned advertisement: both names it carried, and the name being searched for |
| `device-name` | the name read on connect, and the name the device later reported |
| `login` | reaching a cube and presenting a PIN: each attempt, and what the cube made of it |
| `battery` | the charge, as the archived app recorded every reading it received |
| `conn-phase` | how long a step of the archived app's connect sequence took |
| `advert` | what the advertisement itself carried, read off the BlueZ `Device1` object |
| `gatt` | the resolved service and characteristic tree, with each characteristic's flags |
| `probe` | a stage of the Linux reachability probe, and what it made of it |
| `hist-*` | its history fetches, which are the only other traffic in the quiet window of finding 7 |

Checked in deliberately, at 60 KB. These measurements cost an evening of device time and several wrong conclusions along the way, and a claim about firmware behaviour is worth little without the trace behind it.

---

## 1. Renaming changes the GAP name but not the advertised name

Command `0x15` changes the GAP Device Name (`0x2A00`). It does **not** change the advertised local name, which stays `TimeFlip v2.0` permanently. Confirmed across seven renames.

That split is the single most consequential fact about renaming, because the two names are what a scan matches on:

- Matching **only the GAP name** loses the device the moment it is renamed off "TimeFlip". On Apple  platforms `CBPeripheral.name` is that value, so it is the obvious thing to filter on, and doing so  is what made a renamed cube undiscoverable here (fixed; see `DeviceNameRules.matchesKnownDevice`).
- Matching **only the advertised name** always finds the hardware but can never show the user the  name they chose.

So both are needed, for different jobs. `DeviceNameRules.matchesKnownDevice` checks both.

### The GAP name is one connection stale

`CBPeripheral.name` is cached by macOS and refreshed only when CoreBluetooth next connects and re-reads GAP. Straight after a rename it still reports the previous name. Polling it 120 times over 30 seconds within the same connection never saw it change.

The next connection does report it: `peripheralDidUpdateName(_:)` fires on every rename tested. That callback is wired up and is what corrects the name a first pairing adopts from the stale cache.

**How long it takes is not two seconds, and nothing should be built on that figure.** This used to read "about two seconds in". A rename measured on 2026-09-10 took **5.3 seconds** from the connection being established to the callback arriving (connected 21:56:58.595, name reported 21:57:03.873, the app's own `debug_log`), on the connection after a rename from Hazza to Cooty. Two seconds was not wrong so much as one end of a range nobody had measured the other end of, so treat it as seconds rather than as a number.

**Consequence for this app:** a name the app has written and the device has confirmed beats a connect-time read, because the read is the stale one. `AppState.shouldAdoptReportedName` implements that, taking the reported name only on a first pairing.

**Unresolved, but no longer unanswerable.** Whether the device applies a rename immediately or defers it cannot be determined from macOS, since the host only re-reads GAP on connect. Answering it needs a second BLE central with no cached record of the device -- and finding 12 is one: a Linux host on BlueZ, which has never held a record of this cube and can be made to forget it on demand with `Adapter1.RemoveDevice`. The experiment is to rename from the Mac and read the GAP name from Linux, and it has not been run yet.

### Forcing the new name to appear

There is no way to make the device advertise the new name, but the reported name can be refreshed on demand rather than waited out, by deliberately spending the connection that refreshes it:

1. Rename the device.
2. **Forget Device**, which drops the connection.
3. **Scan for Devices**.
4. Click the row, which is **still showing the old name** (the list renders `peripheral.name`, the stale GAP value, falling back to the advertised name only when that is absent).
5. Once paired, the Name row shows the new name.

Step 4 is the step that looks wrong and is not: the peripheral identifier is the same cube whatever name is against it, so the connection proceeds and `peripheralDidUpdateName` then delivers the real name a second or two in.

This is written up for users at <https://facet.tux.com.au>. It is a workaround for the device's behaviour, not a fix, and it should stay in the documentation until a firmware release makes it unnecessary.

---

## 2. Only some commands update the command result characteristic

The spec describes the command result characteristic as carrying `0xXX 0xYY`, the command number and an error value, with no suggestion that this is optional. In practice a good number of commands never write to it at all, leaving whatever the previous command left there.

| Command | events data narration | command result |
|---|---|---|
| `0x07` get time | `get system time` | updated (`07 00 …`) |
| `0x08` set time | `set system time` | updated (`02`) |
| `0x10` status | none | updated (`01 02 …`) |
| `0x17` read double-tap | `read acc setting` | updated (`17 3A …`) |
| `0x30` set password | `password set` | updated (`02`) |
| `0x09` LED brightness | `set brightness LEDs` | **never updated** |
| `0x0A` blink period | `set blinking period` | **never updated** |
| `0x11` face colour | `set color` | **never updated** |
| `0x15` set name | `Neme set` | **never updated** |

`0x08` and `0x30` are write-only and *do* answer, so "write-only commands do not answer" is not the rule. Which commands answer looks arbitrary.

For `0x15` this was checked exhaustively rather than assumed: after a rename the characteristic was re-read at +250 ms, +500 ms, +1 s and +2 s, and every read returned the stale `0x17` response, well after the device had announced completion. It is not a timing race.

**Consequence for this app, and it is a real defect.** `TimeFlipBLEDevice.performCommand` writes, reads the command result, and validates it only when the response is one or two bytes long:

```swift
if response.count == 2, response.first == cmd { ...status check... }
else if response.count == 1 { ...status check... }
```

A stale 20-byte response from an unrelated command matches neither branch and is returned as success. So `0x04`, `0x09`, `0x0A`, `0x11` and `0x15` are all effectively write-blind while appearing verified. A single session start does this twelve times over for face colours alone.

---

## 3. Every command is narrated on the events data characteristic

Undocumented, and the only completion signal present for **all** commands, arriving 40-310 ms after the write. The upper end is from a `0x15` measured on 2026-09-10: written at 21:55:56.383, narrated `Neme set` at 21:55:56.694. The range was 40-240 ms before that, so this widened it rather than contradicting it. Plain ASCII on `F1196F51-71A4-11E6-BDF4-0800200C9A66`:

| Bytes | ASCII |
|---|---|
| `73 65 74 20 73 79 73 74 65 6D 20 74 69 6D 65` | `set system time` |
| `67 65 74 20 73 79 73 74 65 6D 20 74 69 6D 65` | `get system time` |
| `73 65 74 20 62 72 69 67 68 74 6E 65 73 73 20 4C 45 44 73` | `set brightness LEDs` |
| `73 65 74 20 62 6C 69 6E 6B 69 6E 67 20 70 65 72 69 6F 64` | `set blinking period` |
| `73 65 74 20 63 6F 6C 6F 72` | `set color` |
| `72 65 61 64 20 61 63 63 20 73 65 74 74 69 6E 67` | `read acc setting` |
| `72 65 61 64 20 68 69 73 74 6F 72 79` | `read history` |
| `67 65 74 20 68 69 73 74 6F 72 79` | `get history` |
| `70 61 73 73 77 6F 72 64 20 73 65 74` | `password set` |
| `4E 65 6D 65 20 73 65 74` | `Neme set` (sic) |

The spec documents this characteristic as carrying event data, not command narration, so the strings are presumably debug output the firmware never stopped emitting. That makes them a fragile thing to depend on: a future firmware could remove them without considering it a breaking change.

**Possible use.** It is the only way to know a `0x15`, `0x09`, `0x0A` or `0x11` actually completed. Worth considering as a confirmation signal, with the caveat above, and worth raising with the vendor so it either becomes supported or is replaced by a proper command result.

---

---

### Two more strings, and one of them is not command narration at all (Linux, 2026-09-07)

Measured over libdbus, rows 20110 to 20124. Neither is in the table above:

| Bytes | ASCII | When |
|---|---|---|
| `70 61 73 73 77 6F 72 64 20 4F 4B` | `password OK` | a correct PIN written to the password characteristic |
| `4E 65 77 20 53 69 64 65 3A 20 30 78 30 30` | `New Side: 0x00` | every face change |

**`password OK` is the login's own narration**, which the table did not have: its `password set` answers
`0x30`, a different thing. So a login now has two independent signals -- this line, and the `0x02` of
finding 4 -- which is worth knowing given how much rests on that byte.

**`New Side` is an event rather than a command**, and it is the first entry here that is. This heading says
"every command is narrated", and that is still true; what this adds is that the characteristic also
narrates something nobody asked for, which is what the spec says it is actually for. **The side number is
always `0x00`**, across seven different faces in one listen (rows 20112 to 20124), so it carries no usable
face and the `faces` characteristic remains the only source of one.

**A caution for anyone counting these on Linux.** A `ReadValue` on BlueZ publishes a `PropertiesChanged`
of its own, so a poller sees its own reads as though the device had pushed them: a probe reading `faces`
once a second produced 39 signals that all looked like notifications. The rows above come from a listen
that read nothing.

## 4. The password check answers `0x02` for a correct PIN, not `0x01`

The spec is explicit and it is wrong. Section 4, on the password characteristic:

> The result of the password check will be written to the command result output characteristic in the first (high) byte of the massive: 0x01 means the password is correct, 0x02 - the password is wrong.

The hardware does the opposite. **`0x02` is acceptance and `0x01` is refusal**, measured on 2026-08-17 across two logins to one cube, seconds apart, one of each outcome (rows 361 to 375):

| Time | Direction | Bytes | Outcome |
|---|---|---|---|
| 05:47:55.098 | `ble-tx` | `30 30 30 30 30 30` (`000000`) | the vendor default |
| 05:47:55.243 | `ble-rx` | `01` | **refused** -- the cube was not on the default |
| 05:47:58.426 | `ble-tx` | `31 32 33 34 35 36` (`123456`) | the PIN the cube was actually on |
| 05:47:58.543 | `ble-rx` | `02` | **accepted** -- every command afterwards worked |

The two are in one trace, from one cube, three seconds apart, so this is not a reading taken under different conditions and compared: the same characteristic answered both PINs and gave different bytes for the one that worked and the one that did not.

The archive reached the same conclusion by logging both outcomes (see `TimeFlipBLEDevice.attemptLogin`, whose comment says "vendor doc v4.3 states 0x01=correct/0x02=wrong, but real hardware observed here does the opposite"). This is that claim measured again on a rebuilt driver, and written down where the other measurements are, because a comment in an archived class is not somewhere anybody would look.

**Consequence for this app, and it is the most load-bearing byte in the feature.** Implemented from the spec, every correct PIN is refused and every wrong one accepted. `DeviceLoginRules.verdict` reads it the measured way round and `Tests/Scripted/51-device-connect.sh` asserts on the raw `commandResult: 02`, so a firmware release that ever moves to match the document fails a check rather than silently letting the wrong cube in.

### The same answer through a second Bluetooth stack (Linux, 2026-09-07)

Rows 20103 to 20106: the vendor default written to the password characteristic, `02` back on the command
result, and every read and command afterwards working -- measured over **BlueZ and libdbus**, where the
above was CoreBluetooth. Two hosts, two stacks, the same inverted byte.

That is worth having beyond redundancy. Finding 8's point is that some of these facts are about the host
rather than about the cube, and this is the byte the whole feature turns on: it being inverted in the
same direction on a completely different stack says it is the firmware's doing and not CoreBluetooth's.

### What the characteristics report about themselves

Also from those rows, and worth having because it settles how the write must be made: the password characteristic's properties are `0x08`, write-with-response only, and the command result's are `0x12`, read plus notify. So `.withResponse` is not a choice about reliability here -- it is the only thing the characteristic supports -- and the answer can be read or subscribed to.

### Timings

For sizing timeouts, from the same trace: connect 1.03s, service and characteristic discovery 0.99s, the write acknowledged 85ms later, the answer read 60ms after that. A refusal, a one-second settle, a reconnect and a second PIN accepted took 5.46s end to end -- consistent with the archive's 5.4s worst case for scan-and-link across 36 connects.

---

## 5. The Device Information strings are exact length, not padded to 20 bytes

Rows 313 to 319. The spec's Tab. 1 gives all four Device Information characteristics a size of **20 bytes**; the cube returns each one at its natural length instead, with no NUL padding and no trailing whitespace:

| Characteristic | UUID | Bytes returned | Value |
|---|---|---|---|
| Manufacturer Name String | `0x2A29` | 7 | `DI_LABS` |
| Model Number String | `0x2A24` | 3 | `2.0` |
| Hardware Revision String | `0x2A27` | 6 | `TFv4.1` |
| Firmware Revision String | `0x2A26` | 8 | `FW_v3.64` |

So the 20 in the spec is the field's **maximum**, not its transfer size. The archive's `readString` did a bare `String(data:encoding:)` with no padding handling and was correct on this firmware for exactly that reason.

**`DeviceInfoRules.reported` strips trailing NULs anyway**, and that is deliberate rather than redundant: what is measured here is one cube on one firmware build, and a build that did pad to the documented width would produce strings that compare unequal to themselves and draw labels wider than their words, with nothing on screen to say why. The strip costs nothing when there is nothing to strip.

### All four answered, and quickly

The whole phase -- discovering the service, then four reads -- took **354ms** (row 313 at `14:33:07.966`, row 318 at `14:33:08.320`), against the 10s deadline `DeviceLogin.infoTimeoutSeconds` allows. The first read cost 171ms including discovery; the remaining three came back at 61, 60 and 60ms. This cube exposes the service and answers every one of the four, so the app's handling of a partial answer is untested on hardware.

### They were read after a login, and only after one

Row 313 follows row 309 (`PIN accepted`) and row 312 (`Paired with`), which is the ordering the feature is built on: the pairing is written before these reads start, so nothing about them can delay or fail it. Standard GATT places no authentication on `0x180A`, so these should answer with no PIN presented at all -- but **that has not been measured**, because the app has never asked for them in any other state. Nothing in the app depends on it either way.

---

## 6. A factory reset does **not** drop the connection

Rows 340 to 356, and 384 to 399. `0xFF` was written and acknowledged at `17:54:22.730`, and the link then **stayed up for the whole 104 seconds** somebody watched it — no disconnect, no notification, nothing at all on any characteristic. The next row in the log is a human closing the window.

This contradicts what the archive assumed. `TimeFlipBLEDevice.factoryReset` describes the cube as rebooting, and `ApplicationDelegate` was built around the drop that reboot causes: it armed a confirmation window, waited for the disconnect, and reconnected from there. On this firmware that disconnect never arrives.

**The wipe itself worked perfectly.** That is the other half of the measurement, and the two together are what make this worth writing down:

| | PIN presented | Result |
|---|---|---|
| Before the reset (`17:54:14`) | `000000` | refused |
| Before the reset (`17:54:17`) | `123456` | **accepted** |
| After the reset (`17:56:28`) | `000000` | **accepted** |

The cube was on the app's PIN, and afterwards it was back on the vendor default — so `0xFF` erased it exactly as documented. What failed was the app noticing, and it failed silently: a reset that had genuinely happened sat unconfirmed until the window timed out.

**Consequence for this app.** `BluetoothRadio.factoryReset` no longer waits for a drop. Once the write is acknowledged it lets go of the link itself and then goes looking for the cube on the vendor PIN, which covers both firmwares — one that severs the connection is still handled by `didDisconnectPeripheral`, and one that does not is disconnected deliberately. Waiting on the device to do it was the whole bug.

### How long the wipe takes, and why one retry is not enough

Measured twice on the fixed code, both times by presenting the vendor PIN every three seconds until it was accepted:

| Run | `0xFF` acknowledged | Attempts refused | Accepted | Wipe took |
|---|---|---|---|---|
| `18:06` | `13.566` | 1 (at `18.955`) | `22.075` | **~8.5s** |
| `18:08` | `31.704` | 2 (at `36.710`, `39.833`) | `42.894` | **~11s** |

So the cube goes on answering the *old* PIN for several seconds after acknowledging the reset — it is not erased when the write returns, and it does not stop answering while it erases. A single confirmation attempt, however well timed, would have failed both runs and reported a wipe that had in fact happened. The retry loop is the feature, not a safety net.

**What is still not known** is whether the cube reboots at all, or merely erases in place. Nothing observable here distinguishes them: no disconnect, and the app was not watching the System State characteristic (`F1196F56`), which is where the archive says a `0x01 0x00` notification would appear if one does.

## 7. The battery level is pushed only when it changes, and it changes constantly

Battery Level (`0x2A19`) is listed in the spec as read and notify, with nothing said about when a notification arrives. Measured across eleven days of the archived app's own BLE trace (2026-08-02 to 2026-08-13, the same cube), it is **on change, and only on change**:

| | | |
|---|---|---|
| Values that answered a read the app made | 13 | 10 of them repeated the level already held |
| Values the cube volunteered | 2,834 | **none** repeated the level already held |

Not one unsolicited value in 2,834 restated something the host already knew, so a notification *is* a change. The ten repeats are all on the other line, and they are what a read is for: a cube asked at the start of a connection usually answers with the same level it had at the end of the last one. The three that did not are the charge having moved while the app was away.

The counts come from replaying the trace rather than from a single query, since a value has to be attributed to a read or to the cube by what preceded it. The rows themselves:

```sql
SELECT COUNT(*) FROM debug_log WHERE tag='ble-tx' AND message = 'read request batteryLevel';
SELECT COUNT(*) FROM debug_log WHERE tag='ble-rx' AND message LIKE 'batteryLevel -> %';
```

**Match the value rows exactly.** `LIKE '%atteryLevel%'` also matches the discovery and subscription rows the trace writes once per connection (`characteristics on batteryService: batteryLevel`, `notify on batteryLevel`), which is 26 rows that are not readings and which inflated this table's first draft.

**So a subscription alone is not enough.** Rows 14520 to 14534 are why: a connect sequence in which the app reads the level (`14528`, answered at `14529` with `63`, which is hex for 99%). Without that read, a freshly connected app has no figure at all until the charge next moves, and the gaps between moves ran to over an hour. Reading once on connecting and subscribing afterwards is what covers both.

### The level dithers across one percent, and each waver is a notification

Rows 19557 to 19580, seven minutes of an ordinary connected session. The only thing the app asked for in it is a history fetch at `17:51:48`; everything after that arrived unasked:

```
17:53:08  batteryLevel -> 63      99%
17:53:10  batteryLevel -> 62      98%
17:56:02  batteryLevel -> 63      99%
17:56:04  batteryLevel -> 62      98%
17:56:16  batteryLevel -> 63      99%
17:56:18  batteryLevel -> 62      98%
17:56:42  batteryLevel -> 63      99%
17:56:44  batteryLevel -> 62      98%
```

Always the same two adjacent values, always about two seconds apart, in bursts with minutes of silence between them. Over the whole trace the gaps fall out as 1,108 under 3s, 891 at 3-10s, 583 at 10-60s, 189 at 1-5m, 60 at 5-60m and 15 over an hour: a median of 4 seconds, and a quarter of them at exactly the 2 seconds a dithering pair takes.

**While the link is actually up, that is a value every 11 seconds.** Taking every gap of two minutes or less as time connected gives 8.3 hours across the trace and 326 values an hour within it, and the figure holds across the three days with enough traffic to measure separately: 329, 278 and 355 an hour.

**The charge itself barely moved.** The app's own `battery` rows, which go back further than the trace, put it at 100% on 2026-07-28 and 98% on 2026-08-14; on 2026-08-13 alone the cube reported 2,168 values while never saying anything other than 98 or 99. The volume is not a battery running down, it is a reading that cannot settle.

**Consequence for this app.** Neither the figure on screen nor the low-battery warning can be driven straight off a reading. `BatteryRules.shown` holds the lower of the two adjacent values and adopts a higher one only once a reading climbs two percent clear of it, so a dithering cube draws one steady figure; `BatteryRules.latched` keeps the archive's five-point recovery margin so the warning does not arm and disarm twice a second at the threshold. `BluetoothRadio` logs a `battery` row only when the answer moves, which is why the app's own log will not reproduce the counts above -- the raw values are all still there under `ble-rx`.

---

---

## 8. A device identifier cannot tell one cube from another

**Neither identifier available to this app is unique to a cube, so nothing may reconnect by one.**

- **The identifier the device itself carries is the same on every TimeFlip.** It is a property of the model, not of
  the unit, so two cubes on one desk are indistinguishable by it.
- **The identifier CoreBluetooth hands out is the Mac's, not the cube's, and it can change.** It is a per-host mapping
  rather than anything the device advertises, so it is not stable across the events that matter: a reset, a re-pair,
  or the system regenerating it.

`device_uuid` in `setting` is therefore a hint and never a gate. It may be used to *prefer* one candidate over another
within a scan, and it must not be used to decide that a candidate is or is not this app's cube.

### A third identifier exists on Linux, and it survived a factory reset (2026-09-07)

**BlueZ hands out the cube's real Bluetooth address where CoreBluetooth hides it behind a per-host
mapping.** `E8:DB:D8:CF:F9:0F`, and unlike either identifier above it *is* specific to this unit.

**It was the same address before and after a factory reset** (row 20102, against row 20006 of the day
before), which is a fact neither of the two identifiers above can offer: this is the first thing here that
distinguishes one cube from another *and* has been seen to survive the event most likely to change it.

**It is still not a promise.** BlueZ reports `AddressType: random` (row 20007), and a random address is
the kind the specification permits a device to change -- so this is a measurement of one reset on one
cube, not a guarantee. What it does mean is that the reasoning above is about CoreBluetooth's identifier
rather than about Bluetooth: a Linux port has a better handle available, and `BlueZAddress` carries it
inside a `UUID` so the rest of the app can keep its own shape.



### What identifies a cube is the PIN

The app sets a PIN of its own on the cube it pairs with (`0x30`), so **the cube that accepts this app's PIN is this
app's cube**, and that is the only answer available. It follows that a reconnect scans by name, collects every
eligible device, and tries each in turn: a refusal is not a failure, it is the answer to "is this one mine?" and the
loop moves on. Only running out of candidates is a failure.

The archive drew exactly this conclusion and wrote down what the alternative cost: connecting to whichever device
answered first "so a colleague's cube advertising a moment sooner was enough to lock this user out of their own
device with a `wrong password` that named nothing" (`ApplicationDelegate.swift`).

### On Linux the address is visible, and it is a random one

BlueZ hands out the device's BLE address, which CoreBluetooth never shows at all. On this cube it is
`E8:DB:D8:CF:F9:0F`, `AddressType: random` (row 20007). The top two bits of `E8` are `11`, which makes it a *static*
random address rather than a rotating private one -- so it is per-unit and stable at least for as long as a session.

**It does not overturn this finding, and the rule above stands unchanged.** The BLE specification permits a static
random address to change on power cycle, and nothing here has tested whether this one survives a battery pull or a
factory reset. So it is a better hint than the per-host UUID CoreBluetooth offers, and it is still a hint. The cube
that accepts this app's PIN is this app's cube.

Worth recording from the same rows, because it settles a question a port has to ask early: `Paired: 0` and
`Bonded: 0` (row 20009), on a cube the probe had just logged into and driven for forty seconds. **The cube needs no
OS-level pairing or bonding.** Authentication is entirely the app-level PIN written to the password characteristic,
which is why nothing in this app ever asks the host to pair, and why a port does not have to reproduce a pairing
agent to get started.

### A second attempt on the same peripheral must let the first one go

Measured twice, a fortnight apart, by two different codebases against the same cube. The archive, 2026-08-09: "a second attempt that
fails instantly is not an answer about the password -- it is the radio still holding the last one." The rebuild,
2026-08-23: a cube refused the PIN, Retry was pressed two seconds later, and the whole attempt took **eight
milliseconds** -- `already known to this session`, `Connecting`, `Disconnected unexpectedly`, `unreachable` -- so the
offer came back saying "nothing answered" about a cube on the desk that had answered moments before.

So a failed attempt forgets its peripheral handle, and Retry clears the whole found list, which is what makes the
next attempt a real scan rather than another go at a connection already being torn down.

## 9. A pause command files a new history event, and the cube says nothing about it

Measured 2026-08-27, on the cube this repository is developed against, driving the rebuilt app through the status
item's right half and reading the `ble-tx`/`ble-rx` trace in order.

**The cube does two things and announces neither.** `0x06 0x01` closes the event that was running and opens a fresh
one on the same face, marked paused, at zero seconds. That record exists in the cube's flash from the moment the
command lands. Nothing is pushed to say so.

The whole window between the write and the app's next request, verbatim:

```
[command]  Sending 06 01
[ble-tx]   command withResponse: 06 01
[ble-rx]   command: write acknowledged      <- the ATT reply to the line above
[command]  Asking whether it took: 10
[ble-tx]   command withResponse: 10
[ble-rx]   command: write acknowledged      <- the ATT reply to the line above
[ble-tx]   commandResult: read requested
[ble-rx]   commandResult: 02 01 00 05 00 00 ...   <- the reply to the read above
[command]  The cube is unlocked and paused
[history]  Fetching history (the cube was paused from the menu bar)   <- the app asks
```

Every `ble-rx` in that window is on the command characteristic and is a reply to something the host sent. Nothing
arrives on the history characteristic at all. When the app does ask, the new event is there waiting:

```
[history]  The cube is on event 12, face 2, 0s, paused
[event]    device_event updated  id=6 ev=11 face=2 dur=17s paused=false
[event]    device_event inserted id=7 ev=12 face=2 dur=0s  paused=true
```

So "the cube records a pause" and "the cube reports a pause" are different claims. The first is true, the second is
not, and a client that waits to be told is waiting for something that never comes.

### This confirms the archive rather than correcting it

The previous app already worked around it, in a comment on the line that does so
(`ApplicationDelegate.swift`): *"Device doesn't send notification after setPause command, so
explicitly fetch history to confirm state change."* This is that claim measured rather than inherited, which is worth
having written down: it was reasonable to wonder whether newer firmware had started volunteering the record, and it
has not.

### What it costs, and where the cost actually falls

**Not on flips.** The faces characteristic *does* notify on change, and this app fetches history on it
(`radio.onFace` -> `refresh(because: "the cube was turned")`). So a cube flipped back and forth is followed at once
whatever `fetch_history_interval_seconds` is set to, and the seeded description of that setting says as much: the
timer runs "in addition to the fetches already triggered by live face/pause events".

**On any pause the app did not make.** A double tap stops the cube's tracking in firmware with no command involved
(`docs/timeflip2-firmware-observations.md` finding 11), and the vendor's own app can pause it too. Neither produces a face change,
neither writes the command result, and `systemState` carries sync and hardware health rather than pause. So there is
no subscription that could carry it: the app finds out on its next fetch and not before. The history timer's interval
is therefore the worst-case latency for **the cube being stopped by someone else**, and for nothing else.

That is the reason a pause the app itself sends is followed immediately by a fetch at every call site that sends one.
It is not tidying up; it is the only way the app finds out what it just did.

## 10. A factory reset does **not** clear the auto-pause delay

`0x05` writes the idle delay after which the cube stops counting on its own, and the cube keeps it in flash. `0xFF`
does not clear it. Measured on run 135 (2026-08-29), where the delay survived two factory resets in one run and went
on being reported afterwards.

Confirmed on a second stack on 2026-09-07 (rows 20107 to 20109): a cube the owner had just factory reset
answered `0x10` with `02 01 00 05`, which `DeviceCommandRules` reads as lock off, pause on, **auto-pause
five minutes**. Nothing in that session had written `0x05`. Measured over BlueZ and libdbus, where the
run below was CoreBluetooth -- so the delay surviving a wipe is the firmware's doing rather than
anything about the host.

**And the spec says the opposite twice over**: `0x05` is documented as *"Auto-pause mode (disabled by
default)"*, so both the default and the survival are contradicted by a single read.

The evidence is the `0x10` answer, whose last two bytes are the delay in minutes. Across that whole run it never
moved:

```
16:02:22  commandResult: 02 02 00 05      before 00-setup's reset
16:02:31  commandResult: 02 02 00 05      after it
16:15:07  commandResult: 02 02 00 05      after 52-device-reset's, which passed all 32 of its checks
16:16:44  commandResult: 02 01 00 05      still five minutes, four scripts later
```

`02 02` is unlocked and running; `00 05` is five minutes. Every one of those readings is a fresh `0x10` after a
login, so none is a stale reply sitting in the characteristic.

**What it does when nobody is expecting it.** The cube stops itself, files the paused stretch as a new event, and
says nothing: there is no notification for a pause the app did not send (finding 9), so the app finds out on its
next history fetch. In run 135 the cube sat untouched for five minutes while `60-device-backlog` waited for a person
to turn Bluetooth off, paused itself, and the check watching the figure carry on counting failed -- about a cube
that had genuinely stopped. Nothing in the app was wrong and the failure named the app.

**Consequence for this app.** The delay is not something a run can inherit safely, so `00-setup` reads it off the
cube and turns it off before anything else runs, and `BluetoothRadio.received(status:)` now says the delay in the
status row whenever it is not zero -- that line is the only place the answer appears, and without it a cube carrying
a delay is indistinguishable from one that is not.

**Worth raising with the vendor**, as a question rather than a correction: the spec says `0xFF` resets the tracker to
factory settings, and a delay written by `0x05` surviving one is either a deliberate exclusion or an omission. Either
way a client cannot assume a reset cube is a cube in its default state.

## 11. No command disables double tap, and the only lever is sensitivity

**The cube pauses itself on any physical double tap, and nothing in the protocol turns that off.** It is
unconditional firmware behaviour: the vendor spec defines no disable command, and none was found on the device. The
only thing a client can change is how hard a knock has to be before the accelerometer calls it one, through the four
`0x16` registers -- `clickThreshold`, `limit`, `latency` and `window`, each a `UInt8` 0-255.

Measured on the real cube by the previous app, whose Device tab exposed those four registers and watched the gesture
against each setting. The registers this app seeds `double_tap_settings` from are that device's actual values, read
off the hardware rather than chosen.

**Consequence for this app.** Turning the gesture off is *faked*, and has to be: `DoubleTapRules` sends `0x16` with
`window` forced to 0, which gives the second knock no time to arrive in and so makes the gesture unrecognisable. The
stored window keeps its real value throughout, that being what turning the gesture back on sends. It is suppression
and not an off switch -- a knock hard enough is still a knock -- which is why `double_tap_settings` is seeded off
rather than on: the gesture stops the clock on any knock hard enough, including one through the desk the cube is
sitting on.

### 11a. The double-tap registers survive a factory reset

**`0xFF` does not put them back.** Measured 2026-09-13 across scripted run 180, which resets the cube in
`52-device-reset` and re-pairs in `53-device-reconnect`: the `0x17` answer was read **20 times over the whole
run**, twice inside the reset script and three times in the reconnect that followed it, and every one of them
reported `Threshold: 90, Limit: 20, Latency: 50, Window: 0`. `0x16` was not sent once in the entire run.

That is worth writing down because the sheet the app shows before a reset says it erases "face colours, task
settings, name, and password", and it is easy to read the accelerometer registers into that list. They are not
in it.

**What follows for anybody testing this.** Once the gesture has been turned off there is nothing in the app or
in `Tests/Scripted` that can turn it back on: the Device tab's control was removed on 2026-09-11, nothing reads
`double_tap_settings`, and a reset does not restore the factory `window`. So a check that wants the app to
*notice a disagreement and correct it* cannot be arranged. `59-double-tap` tried and failed on exactly that in
run 180, and now checks what can be observed: the cube reports the gesture off, and nothing is written to it.

**It also means the app's own send is a one-off in practice.** The registers were last written by a build that
still had the control, and they have held ever since, across resets. `DeviceSettingsSync` still compares on
every connection, which is what would put a cube right if anything ever did move them.

## 12. The advertisement carries no service UUID, and a scan filtered on one finds nothing

**The cube does not put its service UUID in its advertisement**, so a scan filtered on
`f1196f50-71a4-11e6-bdf4-0800200c9a66` never sees it. BlueZ reports the `Device1` object's `UUIDs` property as
**empty** (row 20010) on a cube that plainly has the service, since the same connection goes on to resolve it along
with all eight of its characteristics.

Measured on Linux Mint 22.3, BlueZ 5.72, 2026-09-06, rows 20001 to 20011. Two scans with the filter set saw nothing
in six seconds and then in twenty; with the filter removed and nothing else changed, the same cube appeared in under
one second at RSSI -68.

What the advertisement does carry:

| Field | Value |
|---|---|
| Local name | `TimeFlip v2.0` |
| Service UUIDs | none at all |
| Manufacturer data, company `0xFFFF` | `54 2E 46 6C 69 70 00`, ASCII `T.Flip` |
| Address type | random, and static rather than rotating (finding 8) |

A 128-bit UUID costs 16 of the 31 bytes an advertisement has, so spending them on a name instead is an ordinary
decision. What makes it worth a finding is that **it is invisible from CoreBluetooth**, which is why it went
unrecorded for a year of driving this hardware: `BluetoothRadio` passes `withServices: nil`, that works, and nothing
ever forces the question of whether the narrower filter would have.

**Consequence for this app, and more so for anything reimplementing it.**
`BluetoothRadio.scanForPeripherals(withServices: nil, options: nil)` and `DeviceScanRules.vendorName`, matching the
name as a substring, are not breadth for its own sake and not a stylistic preference: **they are the only thing that
works.** A port that reaches for a service-UUID scan filter -- the obvious, more precise-looking choice on any BLE
stack, and the first thing tried here -- gets a twenty-second dead end on a cube sitting on the desk, with no error
to explain it. That is exactly how this was found.

**The manufacturer data is a second marker, and an untested one.** Company ID `0xFFFF` is the Bluetooth SIG value
reserved for testing rather than an assigned identifier, so `T.Flip` is not an authoritative vendor stamp and should
not be treated as one. It has also not been checked against a *renamed* cube, which is the case that would make it
useful -- finding 1 has the advertised local name staying `TimeFlip v2.0` permanently, so if the manufacturer data is
equally fixed then a cube renamed away from the vendor default still carries two ways to recognise it. Worth
measuring before anything relies on it.
## 13. A factory-reset cube reports a stale clock, not an unset one

**Measured 2026-09-20**, on the same cube, immediately after a factory reset. Asked `0x07`, it answered:

```
07 00 00 00 00 5E B9 20 63
```

`0x5EB92063` is **1589190755**, which is 11 May 2020: about six and a half years behind. Not zero, not
absent, and not implausible on its face.

**Why it matters is the test somebody will write.** A reset clears the clock, and
`DeviceLogin.setTheClock` exists because a cube with no clock has nothing to stamp a history frame with. The
obvious way to ask whether the clock needs setting is to look for zero, or for a value too small to be a real
date. **Both are fooled here.** 1589190755 is a real date and passes any plausibility check that is not
comparing it against the host.

**The only honest test is drift from this machine's clock**, which is what `DeviceCommandRules.readBack`
already does with its tolerance for exactly this command. This finding is written down so nobody replaces it
with a cheaper check.

`0x08` then set it and `0x07` read back `07 00 00 00 00 6A AF 80 14`, which is the value sent, so the write
itself behaves as the spec says.

## 14. On a cube with no history, the trailing bytes of a frame carry its clock

**Measured 2026-09-20**, the same session. A `0x01 FF FF FF FF` single-event request against a cube with
nothing recorded answered:

```
00 00 00 00 00 00 00 00 00 00 00 00 00 5E B9 1F C8     before the clock was set
00 00 00 00 00 00 00 00 00 00 00 00 00 6A AF 80 14     after it was set
```

Bytes 0 to 3 are **event 0**, which `timeflip.md` already records as meaning the cube has no such event.
What was not recorded is bytes 13 to 16, the field the spec calls the duration. **They are not zero, and they
are the cube's own clock**: `5EB91FC8` is 1589190600 and `6AAF8014` is 1789886484, each matching what `0x07`
said at that moment.

**So the all-zero sentinel test does not catch an empty answer.** A parser that checks the documented sentinel
and then reads the duration gets 1,589,190,600 seconds for an event that does not exist. The check has to be
`event == 0` **before** anything else is parsed.

Confirmed in the same session that a real frame is unaffected: after one flip the answer was
`00 00 00 01 02 00 00 00 00 6A AF 80 53 00 00 00 16`, which reads as event 1, face 2, 22 seconds, with the
duration **big-endian** as `timeflip.md` says was seen on 2026-01 firmware.

### The evidence for 13 and 14 is not in the evidence file

**Both were measured by [`probe/timeflip-btleplug`](../probe/timeflip-btleplug/), a Rust program using
`btleplug`, rather than by this app.** It writes no `debug_log` rows, so there is nothing to copy into
`timeflip2-firmware-evidence.sqlite` and no row ids to quote. The raw bytes above are its verbatim stdout,
and re-running it reproduces them.

**Said plainly rather than worked around.** Every other finding here cites rows because the app produced
them; these two cannot, and a reader deciding how much to trust them should know which kind they are. What
does not change is the rule at the top of this file: they are measurements from real hardware, and where they
disagree with the spec the hardware wins.


## Raised with the vendor

Findings 1 to 3 are the subject of an issue against `DI-GROUP/TimeFlip.Docs`. The request is that the spec describe them, not that the behaviour change: a guaranteed-stable advertised name is genuinely useful for scan filtering once documented, rather than merely observed.

**Finding 4 is different in kind and should be raised separately.** The other three are behaviour the spec is silent about; this one is a documented statement that is the wrong way round, and it is the sort of error that costs somebody a day. Either the firmware or the document is wrong, and the vendor is the only one who can say which was intended.

**Finding 6 is worth raising too**, and it is a question rather than a correction: the spec does not say what a client should observe after `0xFF`, and on this firmware the answer is *nothing at all* — the command is acknowledged, the device is genuinely erased, and the connection carries on as though it had not been. Any client that waits for the device to react is waiting for something that never comes. A single documented signal, on the System State characteristic or as a disconnect, would make a reset confirmable without a reconnect.
