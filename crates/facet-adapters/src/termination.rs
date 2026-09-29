//! The operating system asking the app to quit: SIGTERM, SIGHUP and SIGINT.

use std::sync::mpsc::{Receiver, channel};

use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

/// A receiver that gets the signal's name, `SIGTERM`, `SIGHUP` or `SIGINT`, each time one arrives. Once this is
/// called those signals no longer end the process by themselves: whoever holds the receiver must quit.
pub fn requests() -> Result<Receiver<&'static str>, String> {
    let mut signals = Signals::new([SIGTERM, SIGHUP, SIGINT])
        .map_err(|error| format!("the signals could not be caught: {error}"))?;
    let (sender, receiver) = channel();
    std::thread::Builder::new()
        .name("termination".to_string())
        .spawn(move || {
            for signal in signals.forever() {
                let name = match signal {
                    SIGTERM => "SIGTERM",
                    SIGHUP => "SIGHUP",
                    _ => "SIGINT",
                };
                if sender.send(name).is_err() {
                    eprintln!("facet: {name} arrived with nobody left to quit");
                    return;
                }
            }
        })
        .map_err(|error| format!("the thread watching for signals could not start: {error}"))?;
    Ok(receiver)
}
