//! The cube's PIN has two homes: the secret store it belongs in, and a config file the app falls back to when that
//! store will not take a write. Without the file, a refused write would leave the cube on a PIN nothing can name, and
//! only taking its batteries out would put it back on the vendor PIN.
//!
//! These are the rules for the two together. They hold no store: what each holds is read at the moment it is needed.

use super::login::is_pin;

/// The stored PINs in the order to present them. The file's comes first: it only holds a PIN because the store
/// refused one, so it is the newer of the two. Only valid PINs, and each once.
pub fn read_order(file: Option<&str>, store: Option<&str>) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    for pin in [file, store].into_iter().flatten() {
        if is_pin(pin) && !order.iter().any(|seen| seen == pin) {
            order.push(pin.to_string());
        }
    }
    order
}

/// What reading the two places settles before any cube has answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtRead {
    /// Nothing to settle: the file holds no PIN.
    Nothing,
    /// They agree, so the file's copy is redundant and is removed rather than left as a live PIN in a plain file.
    ClearTheFile,
    /// They differ. Only the cube can say which it took, so nothing moves until a login proves one.
    AwaitTheCube,
}

pub fn at_read(file: Option<&str>, store: Option<&str>) -> AtRead {
    match file {
        None => AtRead::Nothing,
        Some(file) if Some(file) == store => AtRead::ClearTheFile,
        Some(_) => AtRead::AwaitTheCube,
    }
}

/// What to do about the two places once a login has been accepted on `accepted`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterLogin {
    /// The store holds `accepted` and the file holds no PIN: nothing to do.
    Nothing,
    /// The store holds `accepted`, so any copy in the file is redundant or stale and is removed.
    ClearTheFile,
    /// The store does not hold `accepted`. Write it there, and once it is held there remove the file's copy. When the
    /// store still refuses, the file is written instead.
    WriteIt,
}

/// `rotated` is whether the cube was just moved onto `accepted`, which no stored PIN can already be.
pub fn after_login(accepted: &str, rotated: bool, file: Option<&str>, store: Option<&str>) -> AfterLogin {
    if !rotated && store == Some(accepted) {
        if file.is_some() { AfterLogin::ClearTheFile } else { AfterLogin::Nothing }
    } else {
        AfterLogin::WriteIt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_is_presented_first_and_a_pin_both_hold_once() {
        assert_eq!(read_order(Some("111111"), Some("222222")), ["111111", "222222"]);
        assert_eq!(read_order(Some("111111"), Some("111111")), ["111111"]);
        assert_eq!(read_order(None, Some("222222")), ["222222"]);
        assert_eq!(read_order(Some("111111"), None), ["111111"]);
        assert!(read_order(None, None).is_empty());
    }

    #[test]
    fn a_stored_value_that_is_not_a_pin_is_not_presented() {
        assert_eq!(read_order(Some("12a456"), Some("222222")), ["222222"]);
        assert!(read_order(Some(""), Some("1234567")).is_empty());
    }

    #[test]
    fn reading_settles_what_it_can_without_the_cube() {
        assert_eq!(at_read(None, Some("222222")), AtRead::Nothing);
        assert_eq!(at_read(None, None), AtRead::Nothing);
        assert_eq!(at_read(Some("111111"), Some("111111")), AtRead::ClearTheFile);
        assert_eq!(at_read(Some("111111"), Some("222222")), AtRead::AwaitTheCube);
        assert_eq!(at_read(Some("111111"), None), AtRead::AwaitTheCube);
    }

    #[test]
    fn a_login_on_the_stores_own_pin_needs_no_write() {
        assert_eq!(after_login("222222", false, None, Some("222222")), AfterLogin::Nothing);
    }

    #[test]
    fn a_login_on_the_stores_pin_drops_a_stale_or_redundant_file_copy() {
        assert_eq!(after_login("222222", false, Some("111111"), Some("222222")), AfterLogin::ClearTheFile);
        assert_eq!(after_login("222222", false, Some("222222"), Some("222222")), AfterLogin::ClearTheFile);
    }

    #[test]
    fn a_login_on_the_files_pin_moves_it_into_the_store() {
        assert_eq!(after_login("111111", false, Some("111111"), Some("222222")), AfterLogin::WriteIt);
        assert_eq!(after_login("111111", false, Some("111111"), None), AfterLogin::WriteIt);
    }

    #[test]
    fn a_new_pin_is_always_written() {
        assert_eq!(after_login("333333", true, Some("111111"), Some("222222")), AfterLogin::WriteIt);
        assert_eq!(after_login("222222", true, None, Some("222222")), AfterLogin::WriteIt);
    }
}
