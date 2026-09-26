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
