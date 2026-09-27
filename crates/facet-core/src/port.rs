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

/// Keeps one secret: the Google refresh token.
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
