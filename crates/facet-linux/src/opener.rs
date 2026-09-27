//! [`Opener`] on Linux, through `xdg-open`. Revealing a file opens the folder holding it: `xdg-open` has no
//! way to select a file within the folder.

use std::path::Path;
use std::process::{Command, Stdio};

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

/// Runs `command` and waits for it to exit, returning an error naming the exit status when it fails.
///
/// **The command gets no pipes.** `xdg-open` starts the browser or file manager when it is not already
/// running, and that program inherits whatever `xdg-open` was given: a pipe it holds would keep this call
/// waiting until the program quits, long after `xdg-open` itself has exited. So what it prints is not read,
/// and a failure is described from `xdg-open`'s documented exit codes instead.
fn run(command: &mut Command) -> Result<(), String> {
    let status = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| format!("xdg-open could not be run: {error}"))?;
    match status.code() {
        Some(0) => Ok(()),
        Some(1) => Err("xdg-open refused its arguments (exit 1)".to_string()),
        Some(2) => Err("xdg-open found no such file (exit 2)".to_string()),
        Some(3) => Err("xdg-open found no program to open it with (exit 3)".to_string()),
        Some(4) => Err("the program xdg-open chose failed to open it (exit 4)".to_string()),
        _ => Err(format!("xdg-open failed ({status})")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn a_program_the_command_leaves_running_does_not_hold_the_call() {
        let started = Instant::now();
        let result = run(Command::new("sh").args(["-c", "sleep 30 & exit 0"]));
        assert_eq!(result, Ok(()));
        assert!(started.elapsed() < Duration::from_secs(5), "took {:?}", started.elapsed());
    }

    #[test]
    fn a_failing_command_says_how_it_failed() {
        assert_eq!(
            run(Command::new("sh").args(["-c", "exit 3"])),
            Err("xdg-open found no program to open it with (exit 3)".to_string())
        );
    }
}
