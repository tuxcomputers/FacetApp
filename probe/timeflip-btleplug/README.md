# Can btleplug drive the cube?

**The experiment behind open question 2 of [`docs/rust-port.md`](../../docs/rust-port.md), and it is answered:
yes, measured 2026-09-20 against the real cube.** The whole Rust recommendation rested on this and on nothing
else, because Bluetooth is the only part of this app with no good cross-platform answer.

```sh
cargo run                  # read-only: scan, connect, PIN, reads, history
cargo run -- 000000        # the PIN, if it is not the vendor default
cargo run -- --set-clock   # also sets the cube's clock to this machine's
```

**Read-only unless `--set-clock` is passed.** The PIN is presented, not set; history is read, not cleared.
Setting the clock is the one thing here that changes the cube, and it is what the app does on every connect
anyway (`DeviceLogin.setTheClock`).

**Quit Facet first.** A cube talks to one central at a time.

## What it proved

Every mechanism the macOS driver depends on, from one crate:

| Step | Result on FW_v3.64 |
|---|---|
| Unfiltered scan, matched as `DeviceScanRules.isEligible` does | found, rssi -65 |
| Connect and discover | 15 characteristics, 4 services |
| PIN on the password characteristic, with response | accepted (`02`) |
| Device Information and Battery, plain reads | model 2.0, FW_v3.64, 100% |
| Command channel `0x07` read, `0x08` write, read back | set and confirmed exactly |
| History `0x01 FF FF FF FF`, read rather than notify | `00 00 00 01 02 00 00 00 00 6A AF 80 53 00 00 00 16` |

That last frame parses as event 1, face 2, started 1789886547, 22s.

## What it is not

**Not a driver and not the start of one.** It is a question asked once and answered, kept because the answer
is worth more with the thing that produced it beside it. Two firmware facts came out of the run that are now
findings 12 and 13 of [`docs/timeflip2-firmware-observations.md`](../../docs/timeflip2-firmware-observations.md).

**It writes no `debug_log` rows**, not being the app, so its evidence is its own stdout rather than rows in
`timeflip2-firmware-evidence.sqlite`. The two findings say so at the point they claim it.
