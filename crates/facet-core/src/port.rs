//! The platform capabilities the app needs, stated as traits. The composition root hands over something
//! that does each; nothing in the core chooses or knows which.

use std::path::{Path, PathBuf};

/// Shows files and opens links with whatever this machine uses for them.
pub trait Opener {
    /// Shows `file` in the file manager, selected where the platform can. An error says why it did not.
    fn reveal(&self, file: &Path) -> Result<(), String>;

    /// Opens `url` in the default browser. An error says why it did not.
    fn open_url(&self, url: &str) -> Result<(), String>;
}

/// Asks the person for a folder or a file to write, with the platform's own dialogs.
pub trait FileChooser {
    /// A folder picked from a dialog starting at `start`, which may create folders. `None` when cancelled.
    fn choose_folder(&self, start: &Path, message: &str) -> Option<PathBuf>;

    /// A file to write, picked from a save dialog starting in the downloads folder with `name` filled in.
    /// `None` when cancelled.
    fn choose_save_file(&self, name: &str, message: &str) -> Option<PathBuf>;
}

/// What looking a secret up found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretLookup {
    Found(String),
    Missing,
    /// The store would not answer, which is not the same as holding nothing.
    Unavailable(String),
}

/// Keeps one secret, such as the Google refresh token or the cube's PIN; each is its own store.
pub trait SecretStore: Send + Sync {
    /// Stores `secret`. `Ok(true)` only when it reads back as stored.
    fn store(&self, secret: &str) -> Result<bool, String>;

    fn look_up(&self) -> SecretLookup;

    /// Removes the secret. Succeeds when there was nothing to remove.
    fn clear(&self) -> Result<(), String>;
}

/// One HTTP reply, whatever its status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// Sends HTTPS requests.
pub trait Http: Send + Sync {
    /// Sends `method` to `url`, with a bearer token and a `(content type, text)` body when given. Every status
    /// is a reply; an error is a request that got no reply at all.
    fn send(
        &self,
        method: &str,
        url: &str,
        bearer: Option<&str>,
        body: Option<(&str, &str)>,
    ) -> Result<HttpResponse, String>;
}

/// Receives the browser's redirect on a loopback address.
pub trait LoopbackListener: Send + Sync {
    /// Listens on `127.0.0.1` on a port the system assigns.
    fn bind(&self) -> Result<Box<dyn LoopbackSession>, String>;
}

/// One listening loopback socket.
pub trait LoopbackSession: Send {
    fn port(&self) -> u16;

    /// Waits up to `timeout` for the next request, answers it with `respond`'s reply to its request line, and
    /// returns that line. `Ok(None)` when the time ran out.
    fn next(
        &mut self,
        timeout: std::time::Duration,
        respond: &dyn Fn(&str) -> String,
    ) -> Result<Option<String>, String>;
}

/// Whether the Bluetooth radio can be used at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadioState {
    Ready,
    /// Bluetooth is switched off.
    Off,
    /// This app has not been allowed to use Bluetooth.
    Unauthorised,
    /// There is no radio to use, with the reason.
    Unavailable(String),
}

/// One device heard advertising during a scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advert {
    /// The platform's handle for the device, which [`Radio::connect`] takes. Meaningless on another machine.
    pub handle: String,
    /// The name in the advertisement, when it carries one.
    pub name: Option<String>,
    /// The 128-bit UUIDs of the services the advertisement lists.
    pub services: Vec<u128>,
    pub rssi: Option<i16>,
}

/// Scans for and connects to Bluetooth Low Energy devices. Every call blocks, so call from a background thread.
pub trait Radio: Send + Sync {
    fn state(&self) -> RadioState;

    /// Scans for up to `duration`, unfiltered, calling `heard` for each device the first time it is heard and
    /// again when what it advertises changes. Returns early once `stop` is set. An error says why the scan could
    /// not run.
    fn scan(
        &self,
        duration: std::time::Duration,
        stop: &std::sync::atomic::AtomicBool,
        heard: &mut dyn FnMut(&Advert),
    ) -> Result<(), String>;

    /// Connects to the device `handle` names and discovers its services, within `timeout`.
    fn connect(&self, handle: &str, timeout: std::time::Duration) -> Result<Box<dyn Link>, String>;
}

/// One connection to a device. Dropping it disconnects.
pub trait Link: Send {
    /// The device's GAP name, once connected. `None` when it has not said.
    fn gap_name(&self) -> Option<String>;

    /// Whether the device has characteristic `uuid`.
    fn has_characteristic(&self, uuid: u128) -> bool;

    fn read(&mut self, uuid: u128) -> Result<Vec<u8>, String>;

    /// Writes `bytes` to `uuid` with response, returning once the device has acknowledged the write. An
    /// acknowledgement says the bytes arrived, not that the device acted on them.
    fn write(&mut self, uuid: u128, bytes: &[u8]) -> Result<(), String>;

    /// Whether the connection is still up. `false` once the device has gone, whatever the reason.
    fn is_connected(&mut self) -> bool;

    fn disconnect(&mut self) -> Result<(), String>;
}
