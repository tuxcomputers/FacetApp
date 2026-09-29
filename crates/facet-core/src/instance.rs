//! One copy of the app at a time: the first to start holds a lock on `singleinstance.lock` in the data folder for as
//! long as it runs, and a second finds it held.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;

/// The file the lock is held on, in the data folder.
pub const LOCK_FILE: &str = "singleinstance.lock";

/// Claims the lock in `directory`. `Ok(Some(file))` is this copy's, held until the file is dropped; `Ok(None)` means
/// another copy holds it. An error is a lock file that could not be opened or asked.
pub fn claim(directory: &Path) -> Result<Option<File>, String> {
    let path = directory.join(LOCK_FILE);
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|error| format!("{} would not open: {error}", path.display()))?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(format!("{} would not lock: {error}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_claim_finds_the_first_holding_it_until_it_lets_go() {
        let directory = std::env::temp_dir().join(format!("facet-instance-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a scratch folder");
        let first = claim(&directory).expect("claim").expect("the first claim holds it");
        assert!(claim(&directory).expect("claim").is_none(), "a second claim must find it held");
        drop(first);
        assert!(claim(&directory).expect("claim").is_some(), "once let go it can be claimed again");
        std::fs::remove_dir_all(&directory).expect("the scratch folder goes");
    }
}
