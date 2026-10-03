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

/// One secret kept as a string value in a JSON file, beside whatever else the file holds.
///
/// A fallback for a secret the keychain will not take. Every other key in the file is left as it is. The file is
/// written in place and never replaced, so a symlink to it survives. A file that exists but is not a JSON object is
/// never overwritten: reading it answers nothing, and writing it is refused.
pub struct FileSecretStore {
    path: std::path::PathBuf,
    key: String,
}

impl FileSecretStore {
    pub fn new(path: std::path::PathBuf, key: &str) -> FileSecretStore {
        FileSecretStore { path, key: key.to_string() }
    }

    /// The file's object, empty when there is no file. An error says why it could not be had.
    fn object(&self) -> Result<serde_json::Map<String, serde_json::Value>, String> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(serde_json::Map::new()),
            Err(error) => return Err(format!("{} could not be read: {error}", self.path.display())),
        };
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(serde_json::Value::Object(object)) => Ok(object),
            Ok(_) => Err(format!("{} is not a JSON object", self.path.display())),
            Err(error) => Err(format!("{} is not JSON: {error}", self.path.display())),
        }
    }

    fn write(&self, object: serde_json::Map<String, serde_json::Value>) -> Result<(), String> {
        if let Some(folder) = self.path.parent() {
            std::fs::create_dir_all(folder)
                .map_err(|error| format!("{} could not be made: {error}", folder.display()))?;
        }
        let text = serde_json::to_string_pretty(&serde_json::Value::Object(object))
            .map_err(|error| format!("the file could not be written out: {error}"))?;
        std::fs::write(&self.path, text)
            .map_err(|error| format!("{} could not be written: {error}", self.path.display()))
    }
}

impl SecretStore for FileSecretStore {
    fn store(&self, secret: &str) -> Result<bool, String> {
        let mut object = self.object()?;
        object.insert(self.key.clone(), serde_json::Value::String(secret.to_string()));
        self.write(object)?;
        Ok(matches!(self.look_up(), SecretLookup::Found(read) if read == secret))
    }

    /// A file that is missing, empty of the key, or not readable as a JSON object holds nothing, so that a broken
    /// file never stops a login that has the vendor PIN to fall back on.
    fn look_up(&self) -> SecretLookup {
        match self.object() {
            Ok(object) => match object.get(&self.key) {
                Some(serde_json::Value::String(secret)) => SecretLookup::Found(secret.clone()),
                _ => SecretLookup::Missing,
            },
            Err(_) => SecretLookup::Missing,
        }
    }

    fn clear(&self) -> Result<(), String> {
        let mut object = self.object()?;
        if object.remove(&self.key).is_none() {
            return Ok(());
        }
        self.write(object)?;
        match self.look_up() {
            SecretLookup::Found(_) => Err(format!("{} still holds it", self.path.display())),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("facet-secrets-test-{}-{name}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn a_missing_file_holds_nothing_and_a_store_makes_it() {
        let path = file("missing");
        let store = FileSecretStore::new(path.clone(), "PIN");
        assert_eq!(store.look_up(), SecretLookup::Missing);
        assert_eq!(store.store("123456"), Ok(true));
        assert_eq!(store.look_up(), SecretLookup::Found("123456".to_string()));
        assert!(path.is_file());
    }

    #[test]
    fn the_other_keys_in_the_file_are_left_alone() {
        let path = file("others");
        std::fs::write(&path, r#"{"client_id":"abc","client_secret":"shh"}"#).expect("write");
        let store = FileSecretStore::new(path.clone(), "PIN");
        assert_eq!(store.store("123456"), Ok(true));
        assert_eq!(store.clear(), Ok(()));
        let kept: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("json");
        assert_eq!(kept, serde_json::json!({"client_id": "abc", "client_secret": "shh"}));
        assert_eq!(store.look_up(), SecretLookup::Missing);
    }

    #[test]
    fn clearing_what_is_not_there_succeeds() {
        let store = FileSecretStore::new(file("nothing"), "PIN");
        assert_eq!(store.clear(), Ok(()));
    }

    #[test]
    fn a_file_that_is_not_json_holds_nothing_and_is_never_overwritten() {
        let path = file("broken");
        std::fs::write(&path, "client_id = abc").expect("write");
        let store = FileSecretStore::new(path.clone(), "PIN");
        assert_eq!(store.look_up(), SecretLookup::Missing);
        assert!(store.store("123456").is_err());
        assert!(store.clear().is_err());
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "client_id = abc");
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_to_the_file_survives_a_write() {
        let target = file("target");
        std::fs::write(&target, "{}").expect("write");
        let link = file("link");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
        let store = FileSecretStore::new(link.clone(), "PIN");
        assert_eq!(store.store("123456"), Ok(true));
        assert!(std::fs::symlink_metadata(&link).expect("metadata").file_type().is_symlink());
        assert!(std::fs::read_to_string(&target).expect("read").contains("123456"));
    }
}
