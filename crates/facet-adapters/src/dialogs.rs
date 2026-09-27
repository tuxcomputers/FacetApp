//! [`FileChooser`] through `rfd`: the native open and save panels on macOS and Windows, and the XDG desktop
//! portal on Linux. The dialogs are modal and run on the calling thread, which must be the UI thread.

use std::path::{Path, PathBuf};

use facet_core::port::FileChooser;

/// The platform's own file dialogs.
pub struct NativeFileChooser;

impl FileChooser for NativeFileChooser {
    fn choose_folder(&self, start: &Path, message: &str) -> Option<PathBuf> {
        rfd::FileDialog::new().set_title(message).set_directory(start).pick_folder()
    }

    fn choose_save_file(&self, name: &str, message: &str) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::new().set_title(message).set_file_name(name);
        if let Some(downloads) = dirs::download_dir() {
            dialog = dialog.set_directory(downloads);
        }
        dialog.save_file()
    }
}
