// Can btleplug drive this cube?
//
// The Rust evaluation in docs/rust-port.md rests on one untested claim: that btleplug is one API over
// CoreBluetooth, BlueZ and WinRT, and that it can therefore replace three hand-written radios. Everything
// else in that document is replaceable. This is not, so it is the thing to settle first.
//
// **What would settle it, in order.** Each step is harder than the one before, and the last two are the
// ones no generic BLE example would reach:
//
//   1. Scan and find the cube by its vendor service UUID, not by name. Reconnecting is a scan in this app
//      (CLAUDE.md records a rename shipping green and making the cube unreachable), so a library that can
//      only reach already-paired devices is no use. This is also exactly what Qt's Windows backend cannot do.
//   2. Connect and discover the vendor service and its characteristics.
//   3. Present the PIN as ASCII on the password characteristic, with response, and read the verdict back
//      from the command result. A cube refuses every command until a PIN has been accepted.
//   4. Read the standard Device Information strings and the battery level, which are plain GATT reads.
//   5. Write a single-event history request and read a real frame back, then parse it.
//
// **Step 5 is the one that proves it**, because it is the app's own load-bearing exchange and because of a
// measured firmware quirk this probe has to honour: the 0x01 reply arrives as a READ and never as a
// notification. DeviceLogin.readLastEvent records that waiting on a notification here reliably timed out
// against real hardware, and it cost the rebuild a live trace to rediscover.
//
// Nothing is written to the cube that changes it. The PIN is presented, not set; history is read, not
// cleared. Usage:
//
//     cargo run -- <pin>        # the PIN defaults to the vendor 000000

use std::time::Duration;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::Manager;
use uuid::Uuid;

/// The vendor's UUIDs, from Sources/FacetApp/TimeFlipUUIDs.swift, which took them from the v4.3 spec.
const SERVICE: Uuid = Uuid::from_u128(0xF1196F50_71A4_11E6_BDF4_0800200C9A66);
const COMMAND_RESULT: Uuid = Uuid::from_u128(0xF1196F53_71A4_11E6_BDF4_0800200C9A66);
const PASSWORD: Uuid = Uuid::from_u128(0xF1196F57_71A4_11E6_BDF4_0800200C9A66);
const COMMAND: Uuid = Uuid::from_u128(0xF1196F54_71A4_11E6_BDF4_0800200C9A66);
const HISTORY: Uuid = Uuid::from_u128(0xF1196F58_71A4_11E6_BDF4_0800200C9A66);

/// Bluetooth SIG's, which the cube also carries.
const BATTERY_LEVEL: Uuid = Uuid::from_u128(0x00002A19_0000_1000_8000_00805F9B34FB);
const FIRMWARE_REVISION: Uuid = Uuid::from_u128(0x00002A26_0000_1000_8000_00805F9B34FB);
const MODEL_NUMBER: Uuid = Uuid::from_u128(0x00002A24_0000_1000_8000_00805F9B34FB);

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

/// The single-event history frame, per docs/timeflip.md section 5 and the vendor table it defends.
///
/// **The duration is read both ways and the smaller non-zero one taken**, because the firmware disagrees
/// with its own spec about byte order: a plausible duration is small, and 90 seconds read backwards is
/// 1,509,949,440. An earlier version of that document said five bytes little-endian at 13-17 and was wrong
/// on both counts, which made the rebuild reject every answer the cube gave.
fn parse_frame(frame: &[u8]) -> Result<String, String> {
    if frame.len() < 17 {
        return Err(format!("{} bytes, and a single-event answer is 17", frame.len()));
    }
    let event = u32::from_be_bytes([frame[0], frame[1], frame[2], frame[3]]);
    // **Event 0 is checked before the sentinel and before anything is parsed**, which the first run against
    // a freshly reset cube is why. docs/timeflip.md: event 0 means the cube has no such event, so no
    // history at all -- not a segment and not an error. The all-zero sentinel test does not catch it,
    // because bytes 13-16 came back as 5E B9 1F C8 on a cube with nothing recorded, and parsing on gave a
    // duration of 1,589,190,600 seconds for an event that does not exist.
    if event == 0 {
        return Err("event 0, so the cube has no history yet. Turn it onto a face for more than 5s".into());
    }
    if frame[..17].iter().all(|b| *b == 0) {
        return Err("the sentinel, so the cube reports no history at all".into());
    }
    let raw_face = frame[4];
    let (face, paused) = if raw_face > 127 { (raw_face - 128, true) } else { (raw_face, false) };
    let started = u64::from_be_bytes([
        frame[5], frame[6], frame[7], frame[8], frame[9], frame[10], frame[11], frame[12],
    ]);
    let be = u32::from_be_bytes([frame[13], frame[14], frame[15], frame[16]]);
    let le = u32::from_le_bytes([frame[13], frame[14], frame[15], frame[16]]);
    let duration = match (be, le) {
        (0, 0) => 0,
        (0, n) | (n, 0) => n,
        (a, b) => a.min(b),
    };
    Ok(format!(
        "event {event}, face {face}{}, started {started} (unix seconds), {duration}s\n     \
         duration read big-endian {be}, little-endian {le}",
        if paused { ", PAUSED" } else { "" }
    ))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pin = std::env::args().nth(1).unwrap_or_else(|| "000000".into());
    println!("timeflip-probe: presenting PIN {} ({} digits)\n", "*".repeat(pin.len()), pin.len());

    // 1. The adapter.
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = adapters.into_iter().next().ok_or("no Bluetooth adapter")?;
    println!("[1] adapter: {}", adapter.adapter_info().await.unwrap_or_else(|_| "unnamed".into()));

    // 2. Scan unfiltered and match the way the app does. BluetoothRadio scans `withServices: nil` and
    //    DeviceScanRules.isEligible accepts either the vendor service in the advertisement OR a name
    //    carrying the vendor string, because a cube does not reliably advertise its 128-bit UUID. A
    //    service-filtered scan found nothing here on the first run, which is that fact measured.
    //
    //    Unfiltered is also the stronger test of the claim: it is what reconnecting actually does, and it
    //    is precisely what Qt's Windows backend cannot do.
    println!("[2] scanning (unfiltered, matching on service or name, up to 20s)...");
    adapter.start_scan(ScanFilter::default()).await?;
    let mut found = None;
    let mut seen = 0usize;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let peripherals = adapter.peripherals().await?;
        seen = peripherals.len();
        for p in peripherals {
            let props = match p.properties().await? { Some(p) => p, None => continue };
            let named = props.local_name.as_deref().unwrap_or("").to_lowercase().contains("timeflip");
            if props.services.contains(&SERVICE) || named {
                found = Some((p, props));
                break;
            }
        }
        if found.is_some() { break; }
    }
    adapter.stop_scan().await?;
    let (cube, props) = found.ok_or_else(|| format!(
        "no TimeFlip found. {seen} device(s) were seen in total -- if that is 0, the terminal has no \
         Bluetooth permission (System Settings > Privacy & Security > Bluetooth) rather than the cube \
         being absent"
    ))?;
    println!(
        "    saw {seen} device(s); matched {} rssi {:?}\n    advertised the vendor service: {}\n    address {}",
        props.local_name.clone().unwrap_or_else(|| "unnamed".into()),
        props.rssi,
        props.services.contains(&SERVICE),
        cube.address()
    );

    // 3. Connect and discover.
    cube.connect().await?;
    println!("[3] connected: {}", cube.is_connected().await?);
    cube.discover_services().await?;
    let chars = cube.characteristics();
    println!("    {} characteristics across {} services", chars.len(), cube.services().len());
    let find = |u: Uuid| chars.iter().find(|c| c.uuid == u).cloned();

    let password = find(PASSWORD).ok_or("no password characteristic: not a TimeFlip")?;
    let command_result = find(COMMAND_RESULT).ok_or("no command result characteristic")?;
    let history = find(HISTORY).ok_or("no history characteristic")?;

    // 4. Present the PIN, then read the verdict. The read happens strictly after the write is
    //    acknowledged: reading before the cube has processed it is how a stale command result gets
    //    mistaken for an answer (finding 2, docs/timeflip2-firmware-observations.md).
    println!("[4] presenting the PIN...");
    cube.write(&password, pin.as_bytes(), WriteType::WithResponse).await?;
    let verdict = cube.read(&command_result).await?;
    println!("    command result: {}", hex(&verdict));

    // 5. Plain GATT reads, which need no command channel.
    for (name, uuid) in [("model", MODEL_NUMBER), ("firmware", FIRMWARE_REVISION)] {
        if let Some(c) = find(uuid) {
            match cube.read(&c).await {
                Ok(v) => println!("[5] {name}: {}", String::from_utf8_lossy(&v).trim_end_matches('\0')),
                Err(e) => println!("[5] {name}: could not be read ({e})"),
            }
        }
    }
    if let Some(c) = find(BATTERY_LEVEL) {
        match cube.read(&c).await {
            Ok(v) => println!("[5] battery: {}%", v.first().copied().unwrap_or(0)),
            Err(e) => println!("[5] battery: could not be read ({e})"),
        }
    }

    // 6. The clock, which decides what step 7 can possibly mean.
    //
    //    **A cube whose time has never been set files no events at all.** Every history frame carries a
    //    timestamp from this clock, so there is nothing to stamp an interval with, and a factory reset
    //    clears it -- which is why DeviceLogin.setTheClock is the first thing a confirmed connection does.
    //    Reading `event 0` from a freshly reset cube therefore says nothing about flips until this is known.
    //
    //    `0x07` echoes its own command byte, unlike `0x10`, so the answer identifies itself.
    let command = find(COMMAND).ok_or("no command characteristic")?;
    println!("[6] asking what the cube thinks the time is (0x07)...");
    cube.write(&command, &[0x07], WriteType::WithResponse).await?;
    let answer = cube.read(&command_result).await?;
    println!("    raw: {}", hex(&answer));
    let cube_epoch = if answer.len() >= 9 && answer[0] == 0x07 {
        let secs = u64::from_be_bytes([
            answer[1], answer[2], answer[3], answer[4], answer[5], answer[6], answer[7], answer[8],
        ]);
        println!("    the cube says {secs} (unix seconds)");
        Some(secs)
    } else if answer.len() >= 5 && answer[0] == 0x07 {
        let secs = u32::from_be_bytes([answer[1], answer[2], answer[3], answer[4]]) as u64;
        println!("    the cube says {secs} (unix seconds, 32-bit form)");
        Some(secs)
    } else {
        println!("    not a 0x07 answer, so the clock could not be read");
        None
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?.as_secs();
    // **A reset cube does not report zero, it reports a stale but plausible epoch.** Measured 2026-09-20:
    // a freshly factory-reset cube answered 1589190755, which is 2020-05-11, roughly six and a half years
    // behind. So anything testing "is the clock set" by looking for zero, or for an implausible small
    // number, is fooled -- the only honest test is the distance from this machine's clock, which is what
    // DeviceCommandRules.readBack does with its tolerance.
    match cube_epoch {
        Some(secs) => {
            let drift = (secs as i64 - now as i64).abs();
            if drift > 60 {
                println!(
                    "    -> {drift}s from this machine ({:.1} years). A cube whose clock is wrong stamps \
                     its history with that clock.",
                    drift as f64 / 31_557_600.0
                );
            } else {
                println!("    -> set, and within {drift}s of this machine");
            }
        }
        None => {}
    }

    // 7. Set it, but only when asked. This is the one thing here that changes the cube, and it is what
    //    the app does on every connect, so it is safe rather than unusual -- but the default run of this
    //    probe writes nothing that alters state, and that is worth keeping true.
    if std::env::args().any(|a| a == "--set-clock") {
        println!("[7] setting the clock to {now} (0x08)...");
        let mut payload = vec![0x08];
        payload.extend_from_slice(&now.to_be_bytes());
        cube.write(&command, &payload, WriteType::WithResponse).await?;
        println!("    command result: {}", hex(&cube.read(&command_result).await?));
        cube.write(&command, &[0x07], WriteType::WithResponse).await?;
        let back = cube.read(&command_result).await?;
        println!("    read back: {}", hex(&back));
    } else {
        println!("[7] not setting the clock. Pass --set-clock to set it to this machine's time.");
    }

    // 8. The exchange that proves it. 0x01 with 0xFFFFFFFF asks for the latest event, and the answer
    //    comes back as a READ rather than a notification.
    println!("[8] asking for the latest history event (0x01 FF FF FF FF)...");
    cube.write(&history, &[0x01, 0xFF, 0xFF, 0xFF, 0xFF], WriteType::WithResponse).await?;
    let frame = cube.read(&history).await?;
    println!("    raw: {}", hex(&frame));
    match parse_frame(&frame) {
        Ok(read) => println!("    -> {read}"),
        Err(why) => println!("    -> not a frame this probe can read: {why}"),
    }

    cube.disconnect().await?;
    if std::env::args().any(|a| a == "--set-clock") {
        println!("\ndisconnected. The clock was set; nothing else on the cube was changed.");
    } else {
        println!("\ndisconnected. Nothing on the cube was changed.");
    }
    Ok(())
}
