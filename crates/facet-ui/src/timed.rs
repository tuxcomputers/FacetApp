//! Secret store calls with a deadline. A locked store blocks until somebody answers its prompt
//! (`docs/port-findings.md`), so every call here gives up after [`STORE_TIMEOUT`]. Call from a background thread.

use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::Duration;

use facet_core::port::{SecretLookup, SecretStore};

/// How long a secret store call may take before it is treated as unanswered.
pub const STORE_TIMEOUT: Duration = Duration::from_secs(30);

/// Runs `work` on its own thread and waits up to [`STORE_TIMEOUT`] for it. `None` when it did not answer in
/// time; the thread is left to finish on its own.
fn within<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (sender, receiver) = channel();
    std::thread::spawn(move || {
        // A closed channel means the caller stopped waiting, and the answer has nobody to go to.
        if sender.send(work()).is_err() {
            eprintln!("facet: the secret store answered after its timeout, and the answer was dropped");
        }
    });
    receiver.recv_timeout(STORE_TIMEOUT).ok()
}

fn unanswered() -> String {
    format!("it did not answer within {} seconds", STORE_TIMEOUT.as_secs())
}

/// The stored secret. An answer that does not come in time is `Unavailable`.
pub fn look_up(store: &Arc<dyn SecretStore>) -> SecretLookup {
    let store = Arc::clone(store);
    within(move || store.look_up()).unwrap_or_else(|| SecretLookup::Unavailable(unanswered()))
}

/// Stores `secret`, as [`SecretStore::store`].
pub fn store(store: &Arc<dyn SecretStore>, secret: &str) -> Result<bool, String> {
    let store = Arc::clone(store);
    let secret = secret.to_string();
    within(move || store.store(&secret)).unwrap_or_else(|| Err(unanswered()))
}

/// Removes the secret, as [`SecretStore::clear`].
pub fn clear(store: &Arc<dyn SecretStore>) -> Result<(), String> {
    let store = Arc::clone(store);
    within(move || store.clear()).unwrap_or_else(|| Err(unanswered()))
}
