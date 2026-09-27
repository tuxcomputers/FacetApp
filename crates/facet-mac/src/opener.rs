//! [`Opener`] on macOS, through the `open` command.

use std::path::Path;
use std::process::Command;

use facet_core::port::Opener;

pub struct MacOpener;

impl Opener for MacOpener {
    fn reveal(&self, file: &Path) -> Result<(), String> {
        run(Command::new("open").arg("-R").arg(file))
    }

    fn open_url(&self, url: &str) -> Result<(), String> {
        run(Command::new("open").arg(url))
    }
}

/// Runs `command` and returns an error naming its exit status and what it printed when it fails.
fn run(command: &mut Command) -> Result<(), String> {
    let output = command.output().map_err(|error| format!("open could not be run: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("open failed ({}): {}", output.status, String::from_utf8_lossy(&output.stderr).trim()))
    }
}
