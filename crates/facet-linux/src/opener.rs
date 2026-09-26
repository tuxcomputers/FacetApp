//! [`Opener`] on Linux, through `xdg-open`. Revealing a file opens the folder holding it: `xdg-open` has no
//! way to select a file within the folder.

use std::path::Path;
use std::process::Command;

use facet_core::port::Opener;

pub struct LinuxOpener;

impl Opener for LinuxOpener {
    fn reveal(&self, file: &Path) -> Result<(), String> {
        let folder = file.parent().ok_or_else(|| format!("{} has no folder", file.display()))?;
        run(Command::new("xdg-open").arg(folder))
    }

    fn open_url(&self, url: &str) -> Result<(), String> {
        run(Command::new("xdg-open").arg(url))
    }
}

/// Runs `command` and returns an error naming its exit status and what it printed when it fails.
fn run(command: &mut Command) -> Result<(), String> {
    let output = command.output().map_err(|error| format!("xdg-open could not be run: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "xdg-open failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
