//! Checks the Bluetooth adapter against a real cube without changing anything on it: scans, connects, reads the
//! battery and Device Information, presents the vendor PIN and reports the verdict, then disconnects. No new PIN
//! is set and no command that changes the cube is sent.
//!
//!     cargo run -p facet-adapters --example radio-check

#[cfg(target_os = "macos")]
fn main() {
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    use facet_adapters::radio::BtleplugRadio;
    use facet_core::debug_log::Trace;
    use facet_core::device::{login, scan, session};
    use facet_core::port::Radio;

    let radio = match BtleplugRadio::new() {
        Ok(radio) => radio,
        Err(reason) => {
            eprintln!("no radio: {reason}");
            std::process::exit(1);
        }
    };
    println!("radio state: {:?}", radio.state());

    let mut cube = None;
    let mut seen = 0;
    let stop = AtomicBool::new(false);
    let scanned = radio.scan(Duration::from_secs(12), &stop, &mut |advert| {
        seen += 1;
        if scan::is_eligible(advert, &[], false) && cube.is_none() {
            println!("found {} ({}) rssi {:?}", scan::label(advert), advert.handle, advert.rssi);
            cube = Some(advert.handle.clone());
        }
    });
    println!("scan: {scanned:?}, {seen} advertisement(s) heard");
    let Some(handle) = cube else {
        eprintln!("no TimeFlip heard");
        std::process::exit(1);
    };

    let log = Trace::none();
    match radio.connect(&handle, session::CONNECT_TIMEOUT) {
        Ok(mut link) => {
            println!("connected; GAP name {:?}", link.gap_name());
            for uuid in [
                facet_core::device::uuids::PASSWORD,
                facet_core::device::uuids::COMMAND_RESULT,
                facet_core::device::uuids::COMMAND,
            ] {
                println!(
                    "  has {}: {}",
                    facet_core::device::uuids::name(uuid),
                    link.has_characteristic(uuid)
                );
            }
            println!("  battery before login: {:?}", session::battery(&mut *link, &log));
            println!("  device info before login: {:?}", session::device_info(&mut *link, &log));
            session::disconnect(&mut *link, &log);
            println!("disconnected");
        }
        Err(reason) => {
            eprintln!("connect failed: {reason}");
            std::process::exit(1);
        }
    }

    std::thread::sleep(session::SETTLE_BETWEEN_CANDIDATES);
    let pins = vec![login::VENDOR_PIN.to_string()];
    match session::log_in(&radio, &handle, &pins, None, &log) {
        session::LoginOutcome::LoggedIn { mut link, .. } => {
            println!("the vendor PIN was accepted; nothing was rotated");
            session::disconnect(&mut *link, &log);
        }
        outcome => println!("login with the vendor PIN: {}", outcome.describe("the cube")),
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("this build has no radio adapter");
    std::process::exit(1);
}
