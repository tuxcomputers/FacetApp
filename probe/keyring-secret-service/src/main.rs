// Does `keyring` actually reach this machine's Secret Service, and is it the right crate to reach it with?
//
// **Item 5 of docs/handover-linux.md is a flag rather than a task**: `keyring` 4.2.0 is pinned in the
// workspace, `libsecret-1-dev` is installed, the Secret Service is live and holds a real `gh` token -- and
// *nothing has ever been built against the crate*, on either platform. So the secret store is the one
// capability whose story rests on inference. This is the thing that replaces the inference.
//
// **What it answers, in order:**
//
//   1. Does the crate build here at all, and what does it link? The workspace assumed `libsecret-1-dev`
//      mattered. The default store on Linux is `zbus-secret-service-keyring-store`, which is pure Rust,
//      and there is a separate `dbus-secret-service-keyring-store` that is *not* on. If nothing links
//      `libsecret`, the assumption in system-linux.md is wrong in the same way the btleplug one was right.
//   2. Is a store actually available to this process? `Entry::store_status` asks without writing anything.
//   3. A full round trip: write a password, read it back, write bytes, read those back, delete, and
//      confirm the delete by reading again and requiring a NoEntry error.
//
// **Step 3's last part is the one that matters most to Facet and is easy to leave out.** The app has to
// tell *no PIN has ever been stored* from *the secret store is broken*, because the first is an ordinary
// first run and the second must not be treated as one. That is a distinguishable error or it is not, and
// nothing short of asking finds out.
//
// **Bytes as well as a string, deliberately.** The cube's PIN is six ASCII digits, so `set_password` would
// do; but a PIN is a secret rather than a word, and if the store round-trips arbitrary bytes then nothing
// later has to care whether a future secret is text.
//
// **What this deliberately does not do: lock the keyring.** What a background process gets from a *locked*
// collection -- a prompt, or a D-Bus error with nobody to prompt -- is recorded as untested and it matters,
// but the only collection here is `login`, which holds the `gh` token this repository pushes with. Locking
// it interrupts real work and may throw a dialog at whoever is at the screen. That measurement needs a
// person who has agreed to it, not a probe that surprises them.
//
//     cargo run
//
// It writes only under its own service name, and deletes what it wrote.

use keyring::{Entry, Error};

/// The service name everything here is filed under.
///
/// **Not `facet`.** This probe writes to the same keyring a person's real credentials are in, so it uses a
/// name that says what it is and could not collide with the app's own entries when those exist.
const SERVICE: &str = "facet-keyring-probe";
const ACCOUNT: &str = "probe-account";

/// A PIN-shaped secret, because that is what this store is being considered for.
const PASSWORD: &str = "000000";

/// Bytes that are not valid UTF-8, so `get_secret` is proved to be byte-clean rather than string-clean.
const SECRET: &[u8] = &[0x00, 0x01, 0xFE, 0xFF, 0x42];

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => {
            println!("\nAll checks passed.");
            std::process::ExitCode::SUCCESS
        }
        Err(report) => {
            // Nothing fails silently, and a probe that answered its question with "no" is still a probe
            // that worked. What must not happen is exiting 0 on a store that did nothing.
            eprintln!("\nFAILED: {report}");
            eprintln!("The secret store did not behave as the app would need it to. Nothing was assumed.");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    println!("keyring probe: is the Secret Service usable from Rust on this machine?");
    println!("service={SERVICE} account={ACCOUNT}\n");

    // 2. Is there a store at all? Asked before anything is written, so a machine with no Secret Service
    //    says so rather than failing later inside a write and looking like a permissions problem.
    match Entry::store_status() {
        Ok(()) => println!("  store status      available"),
        Err(error) => {
            return Err(format!(
                "no credential store is available to this process: {error}. \
                 On this platform that means the Secret Service is not running or not reachable"
            ));
        }
    }

    let entry = Entry::new(SERVICE, ACCOUNT)
        .map_err(|error| format!("the entry could not be constructed: {error}"))?;

    // **Before writing, prove the entry is absent.** A stale entry from an interrupted earlier run would
    // otherwise make the read-back below pass without this run having written anything.
    match entry.get_password() {
        Err(Error::NoEntry) => println!("  starting state    absent, as expected"),
        Ok(_) => {
            return Err(
                "a secret is already stored under this probe's own name, left by an interrupted run. \
                 Delete it and run again, or the read-backs below would prove nothing"
                    .to_string(),
            );
        }
        Err(error) => return Err(format!("reading the absent entry failed unexpectedly: {error}")),
    }

    // 3. The round trip.
    entry
        .set_password(PASSWORD)
        .map_err(|error| format!("the password could not be stored: {error}"))?;
    println!("  set_password      wrote {PASSWORD}");

    let read = entry
        .get_password()
        .map_err(|error| format!("the password could not be read back: {error}"))?;
    if read != PASSWORD {
        // **Compared rather than merely fetched.** A store that returns *a* value is not a store that
        // returns *the* value, and the difference is invisible unless something checks.
        return Err(format!("the password came back as {read} rather than {PASSWORD}"));
    }
    println!("  get_password      read back the same value");

    entry
        .set_secret(SECRET)
        .map_err(|error| format!("the byte secret could not be stored: {error}"))?;
    let bytes = entry
        .get_secret()
        .map_err(|error| format!("the byte secret could not be read back: {error}"))?;
    if bytes != SECRET {
        return Err(format!("the secret came back as {bytes:02X?} rather than {SECRET:02X?}"));
    }
    println!("  set/get_secret    round-tripped {} bytes including a non-UTF-8 pair", SECRET.len());

    entry
        .delete_credential()
        .map_err(|error| format!("the credential could not be deleted: {error}"))?;
    println!("  delete_credential removed it");

    // **The check the app depends on.** A first run and a broken store must not look alike.
    match entry.get_password() {
        Err(Error::NoEntry) => {
            println!("  after delete      NoEntry, which is distinguishable from a failure");
        }
        Ok(value) => {
            return Err(format!(
                "the credential survived its own deletion and still reads {value}"
            ));
        }
        Err(error) => {
            return Err(format!(
                "after deleting, the store answered {error} rather than NoEntry. \
                 The app could not tell a first run from a broken store"
            ));
        }
    }

    println!("\nStill unmeasured, and it needs a person rather than a probe:");
    println!("  what a background process gets from a LOCKED collection. The only collection here is");
    println!("  login, which holds the gh token this repository pushes with, so locking it interrupts");
    println!("  real work. See docs/system-linux.md.");
    Ok(())
}
