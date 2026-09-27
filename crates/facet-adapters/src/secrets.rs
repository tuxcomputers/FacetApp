//! [`SecretStore`] through `keyring`: the Keychain on macOS, the Secret Service on Linux, the Credential
//! Manager on Windows.

use facet_core::port::{SecretLookup, SecretStore};
use keyring::{Entry, Error};

/// One secret, under a service and an account name.
pub struct KeyringSecretStore {
    service: String,
    account: String,
}

impl KeyringSecretStore {
    pub fn new(service: &str, account: &str) -> KeyringSecretStore {
        KeyringSecretStore { service: service.to_string(), account: account.to_string() }
    }

    fn entry(&self) -> Result<Entry, String> {
        Entry::new(&self.service, &self.account)
            .map_err(|error| format!("the entry could not be made: {error}"))
    }
}

impl SecretStore for KeyringSecretStore {
    fn store(&self, secret: &str) -> Result<bool, String> {
        let entry = self.entry()?;
        entry.set_password(secret).map_err(|error| format!("it could not be stored: {error}"))?;
        match entry.get_password() {
            Ok(read) => Ok(read == secret),
            Err(error) => Err(format!("it could not be read back: {error}")),
        }
    }

    fn look_up(&self) -> SecretLookup {
        match self.entry().map(|entry| entry.get_password()) {
            Ok(Ok(secret)) => SecretLookup::Found(secret),
            Ok(Err(Error::NoEntry)) => SecretLookup::Missing,
            Ok(Err(error)) => SecretLookup::Unavailable(error.to_string()),
            Err(error) => SecretLookup::Unavailable(error),
        }
    }

    fn clear(&self) -> Result<(), String> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(error) => Err(format!("it could not be removed: {error}")),
        }
    }
}
