//! Touching the Keychain at launch, and reporting what that cost.
//!
//! **This exists to answer a question rather than to store anything yet.** The question is whether the
//! macOS Keychain prompts for access once and then stops, or prompts again after every rebuild. The
//! answer decides whether a developer mode is needed at all: in the Swift app, a whole flag existed
//! partly because an ad-hoc signed build is a different application as far as the Keychain is
//! concerned, so permission granted to one build matched nothing after the next.
//!
//! **The signature is what settles it, not this code.** An ad-hoc signature's designated requirement is
//! the cdhash of the binary, which changes on every build. A certificate makes the requirement an
//! identifier and an anchor, with no hash in it, so it is the same for every build and one Always Allow
//! holds. `scripts/run.sh` signs with a real identity where the machine has one.
//!
//! **The Facet name is ours**, the Swift app having been renamed to TimeFlip on 2026-09-21. Its live
//! items are `au.com.tux.timeflip.device` and `.google`.
//!
//! **It still avoids `.device` and `.google` under this name.** The Swift app's originals were copied
//! rather than moved, so `au.com.tux.facet.device` still exists as its fallback until that migration is
//! confirmed against the cube. A probe is not worth overwriting somebody's only record of a PIN.

use std::time::{Duration, Instant};

/// Where the probe item lives. Not the Swift app's items, and not where the real PIN will go.
const SERVICE: &str = "au.com.tux.facet.probe";
const ACCOUNT: &str = "keychain-probe";

/// Above this, a human was almost certainly asked something.
///
/// A Keychain read that is allowed returns in single-digit milliseconds. One that prompts blocks until
/// somebody clicks, so the elapsed time is the signal, and it is a far more honest one than trying to
/// detect the dialogue.
const PROMPT_THRESHOLD: Duration = Duration::from_millis(400);

/// What the launch-time Keychain touch did.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The item was there and readable.
    Read,
    /// There was no item, so one was written and read back.
    Created,
    /// The Keychain refused, or there is no store to ask.
    Refused(String),
}

/// Reads the probe item, creating it if it is absent, and says how long the Keychain took.
///
/// **Nothing here panics or exits.** A Keychain that will not answer is a thing to report, not a reason
/// to refuse to start: the app has no secret worth having yet, and the whole point is to observe the
/// behaviour rather than to depend on it.
pub fn touch_at_launch() {
    let started = Instant::now();
    let outcome = read_or_create();
    let elapsed = started.elapsed();

    match &outcome {
        Outcome::Read => println!(
            "[keychain] Read the stored item in {} ms",
            elapsed.as_millis()
        ),
        Outcome::Created => println!(
            "[keychain] No item yet, so one was written and read back, in {} ms",
            elapsed.as_millis()
        ),
        // Nothing fails silently: a refusal is the interesting answer here, not an inconvenience.
        Outcome::Refused(why) => eprintln!(
            "[keychain] The Keychain refused after {} ms: {why}",
            elapsed.as_millis()
        ),
    }

    if elapsed >= PROMPT_THRESHOLD {
        println!(
            "[keychain] That took {} ms, so you were almost certainly asked to allow access.",
            elapsed.as_millis()
        );
        println!("[keychain] Run it again without rebuilding. A second prompt means the signature is");
        println!("[keychain] not stable across launches; no prompt means it is.");
    } else {
        println!("[keychain] No prompt: this build already has permission.");
    }

    report_signature();
}

fn read_or_create() -> Outcome {
    let entry = match keyring::Entry::new(SERVICE, ACCOUNT) {
        Ok(entry) => entry,
        Err(error) => return Outcome::Refused(format!("the store could not be opened: {error}")),
    };

    match entry.get_password() {
        Ok(_) => Outcome::Read,
        Err(keyring::Error::NoEntry) => {
            // **Written and then read back**, which is the same rule a command sent to the cube follows:
            // a write that reports success and did not happen is exactly the disagreement worth catching,
            // and here it would be a PIN the app believes it can recover and cannot.
            if let Err(error) = entry.set_password("probe") {
                return Outcome::Refused(format!("writing failed: {error}"));
            }
            match entry.get_password() {
                Ok(_) => Outcome::Created,
                Err(error) => Outcome::Refused(format!("the write did not stick: {error}")),
            }
        }
        Err(error) => Outcome::Refused(error.to_string()),
    }
}

/// Says how this binary is signed, because that is what decides whether the prompt comes back.
///
/// Read from the running binary rather than assumed from how it was built: a build that was meant to be
/// signed and was not looks identical from the inside otherwise, and the cost of that is a prompt
/// somebody blames on the Keychain.
fn report_signature() {
    let Ok(exe) = std::env::current_exe() else {
        eprintln!("[keychain] Could not find this binary to ask how it is signed");
        return;
    };
    let output = std::process::Command::new("/usr/bin/codesign")
        .args(["-dv", "--verbose=2"])
        .arg(&exe)
        .output();

    match output {
        Ok(output) => {
            // codesign writes its description to stderr, which is not an error here.
            let described = String::from_utf8_lossy(&output.stderr);
            let signature = described
                .lines()
                .find(|line| line.starts_with("Signature="))
                .unwrap_or("Signature=unknown");
            if signature.contains("adhoc") {
                println!("[keychain] {signature}, so this binary's identity is its own hash and the");
                println!("[keychain] next rebuild will be a different application to the Keychain.");
            } else {
                println!("[keychain] {signature}, so the identity is stable across rebuilds.");
            }
        }
        Err(error) => eprintln!("[keychain] Could not run codesign: {error}"),
    }
}
