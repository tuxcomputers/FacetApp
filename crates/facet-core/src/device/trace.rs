//! The Bluetooth trace: every connection, write, acknowledgement, read and subscription, recorded in words the
//! scripted suite reads. The wording is interface, the same as the Swift app's, and the tags are `ble-tx` for what
//! goes out and `ble-rx` for what comes back.
//!
//! [`TracedRadio`] wraps a [`Radio`] so every [`Link`] it hands out is a [`TracedLink`].

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use super::uuids;
use crate::debug_log::{Record, Tag, plain};
use crate::port::{Advert, Link, Radio, RadioState};

/// Bytes as the trace shows them: upper-case hex pairs separated by spaces, followed by the ASCII in brackets
/// when every byte is printable. Apostrophes and quotation marks are left out of the ASCII.
pub fn describe(bytes: &[u8]) -> String {
    let hex = bytes.iter().map(|byte| format!("{byte:02X}")).collect::<Vec<_>>().join(" ");
    let printable = !bytes.is_empty() && bytes.iter().all(|byte| (0x20..0x7F).contains(byte));
    if printable { format!("{hex} ({})", plain(&String::from_utf8_lossy(bytes))) } else { hex }
}

/// What was read from `uuid`, as [`describe`] shows it, except a one-byte battery level, which is shown as hex
/// followed by the percentage it is.
pub fn describe_read(uuid: u128, bytes: &[u8]) -> String {
    match bytes {
        [percent] if uuid == uuids::BATTERY_LEVEL => format!("{percent:02X} ({percent}%)"),
        _ => describe(bytes),
    }
}

/// A [`Radio`] whose connections are traced into `log`.
pub struct TracedRadio<R> {
    inner: Arc<dyn Radio>,
    log: R,
}

impl<R: Record + Clone + Send + Sync + 'static> TracedRadio<R> {
    pub fn new(inner: Arc<dyn Radio>, log: R) -> TracedRadio<R> {
        TracedRadio { inner, log }
    }
}

impl<R: Record + Clone + Send + Sync + 'static> Radio for TracedRadio<R> {
    fn state(&self) -> RadioState {
        self.inner.state()
    }

    fn scan(
        &self,
        duration: Duration,
        stop: &AtomicBool,
        heard: &mut dyn FnMut(&Advert),
    ) -> Result<(), String> {
        self.inner.scan(duration, stop, heard)
    }

    /// Logs `Connecting to <handle>`, then `Connected to <handle>` and a `Found characteristic <name>` for each of
    /// the cube's characteristics, or why the connection failed.
    fn connect(&self, handle: &str, timeout: Duration) -> Result<Box<dyn Link>, String> {
        self.log.record(Tag::Radio, || format!("Connecting to {}", plain(handle)));
        match self.inner.connect(handle, timeout) {
            Ok(link) => {
                self.log
                    .record(Tag::Radio, || format!("Connected to {}, presenting a PIN next", plain(handle)));
                for uuid in link.characteristics() {
                    if !uuids::name(uuid).chars().all(|c| c.is_ascii_hexdigit()) {
                        self.log.record(Tag::Login, || format!("Found characteristic {}", uuids::name(uuid)));
                    }
                }
                Ok(Box::new(TracedLink { inner: link, log: self.log.clone() }))
            }
            Err(reason) => {
                self.log.record(Tag::Radio, || {
                    format!("The connection to {} failed: {}", plain(handle), plain(&reason))
                });
                Err(reason)
            }
        }
    }
}

/// A [`Link`] whose every exchange is traced into `log`.
pub struct TracedLink<R> {
    inner: Box<dyn Link>,
    log: R,
}

impl<R: Record + Send> Link for TracedLink<R> {
    fn gap_name(&self) -> Option<String> {
        self.inner.gap_name()
    }

    fn has_characteristic(&self, uuid: u128) -> bool {
        self.inner.has_characteristic(uuid)
    }

    fn characteristics(&self) -> Vec<u128> {
        self.inner.characteristics()
    }

    fn subscribe(&mut self, uuid: u128) -> Result<Receiver<Vec<u8>>, String> {
        let name = uuids::name(uuid);
        self.log.record(Tag::BleTx, || format!("{name}: notify on requested"));
        let result = self.inner.subscribe(uuid);
        self.log.record(Tag::BleRx, || match &result {
            Ok(_) => format!("{name}: notifying"),
            Err(reason) => format!("{name}: notify refused, {}", plain(reason)),
        });
        result
    }

    fn read(&mut self, uuid: u128) -> Result<Vec<u8>, String> {
        let name = uuids::name(uuid);
        self.log.record(Tag::BleTx, || format!("{name}: read requested"));
        let result = self.inner.read(uuid);
        self.log.record(Tag::BleRx, || match &result {
            Ok(bytes) => format!("{name}: {}", describe_read(uuid, bytes)),
            Err(reason) => format!("{name}: failed, {}", plain(reason)),
        });
        result
    }

    fn write(&mut self, uuid: u128, bytes: &[u8]) -> Result<(), String> {
        let name = uuids::name(uuid);
        self.log.record(Tag::BleTx, || format!("{name} withResponse: {}", describe(bytes)));
        let result = self.inner.write(uuid, bytes);
        self.log.record(Tag::BleRx, || match &result {
            Ok(()) => format!("{name}: write acknowledged"),
            Err(reason) => format!("{name}: write refused, {}", plain(reason)),
        });
        result
    }

    fn is_connected(&mut self) -> bool {
        self.inner.is_connected()
    }

    fn disconnect(&mut self) -> Result<(), String> {
        self.inner.disconnect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_read_as_hex_with_their_ascii_when_printable() {
        assert_eq!(describe(&[0x02]), "02");
        assert_eq!(describe(&[0xFF]), "FF");
        assert_eq!(describe(b"000000"), "30 30 30 30 30 30 (000000)");
        assert_eq!(describe(b"it's"), "69 74 27 73 (its)");
        assert_eq!(describe(&[]), "");
    }

    #[test]
    fn a_battery_level_reads_as_a_percentage() {
        assert_eq!(describe_read(uuids::BATTERY_LEVEL, &[0x64]), "64 (100%)");
        assert_eq!(describe_read(uuids::BATTERY_LEVEL, &[0x07]), "07 (7%)");
        assert_eq!(describe_read(uuids::FIRMWARE_REVISION, &[0x64]), "64 (d)");
    }
}
