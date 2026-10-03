//! Judging a PIN, which PINs to present, and the PIN the app puts on a cube.

/// The PIN every cube starts on, and returns to when its batteries come out.
pub const VENDOR_PIN: &str = "000000";

/// What the cube said about a PIN, from the first byte of the command result read straight after the PIN was
/// written. `0x02` is accepted and `0x01` refused, the reverse of the vendor spec (firmware finding 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Accepted,
    Refused,
    /// Neither byte, or nothing: the answer could not be read, which is not a refusal.
    Unreadable,
}

pub fn verdict(result: &[u8]) -> Verdict {
    match result.first() {
        Some(0x02) => Verdict::Accepted,
        Some(0x01) => Verdict::Refused,
        _ => Verdict::Unreadable,
    }
}

/// Whether `pin` is six ASCII digits.
pub fn is_pin(pin: &str) -> bool {
    pin.len() == 6 && pin.bytes().all(|byte| byte.is_ascii_digit())
}

/// The PINs to present when pairing a cube, in order: the vendor PIN, then each of `stored` that is a valid PIN
/// other than it. Each is presented on its own connection. `stored` holds two only while the two places a PIN is kept
/// disagree.
pub fn pairing_candidates(stored: &[String]) -> Vec<String> {
    let mut pins = vec![VENDOR_PIN.to_string()];
    pins.extend(stored.iter().filter(|pin| is_pin(pin) && *pin != VENDOR_PIN).cloned());
    pins
}

/// The PINs to present when reconnecting to the paired cube: each of `stored` that is valid, then the vendor PIN.
pub fn reconnect_candidates(stored: &[String]) -> Vec<String> {
    let mut pins: Vec<String> =
        stored.iter().filter(|pin| is_pin(pin) && *pin != VENDOR_PIN).cloned().collect();
    pins.push(VENDOR_PIN.to_string());
    pins
}

/// Whether a cube that accepted `pin` is moved onto a PIN of the app's own: only the vendor PIN is.
pub fn rotates(pin: &str) -> bool {
    pin == VENDOR_PIN
}

/// A PIN of the app's own from `random` bytes: six digits, never the vendor PIN.
pub fn target_pin(random: [u8; 6]) -> String {
    let pin: String = random.iter().map(|byte| char::from(b'0' + byte % 10)).collect();
    if pin == VENDOR_PIN { "000001".to_string() } else { pin }
}

/// A PIN of the app's own from the system's random source. An error says why there were no random bytes.
pub fn new_pin() -> Result<String, String> {
    let mut random = [0u8; 6];
    getrandom::fill(&mut random).map_err(|error| format!("no random bytes: {error}"))?;
    Ok(target_pin(random))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_is_accepted_and_one_refused() {
        assert_eq!(verdict(&[0x02]), Verdict::Accepted);
        assert_eq!(verdict(&[0x01, 0x00]), Verdict::Refused);
        assert_eq!(verdict(&[0x17, 0x3A]), Verdict::Unreadable);
        assert_eq!(verdict(&[]), Verdict::Unreadable);
    }

    #[test]
    fn the_stored_pins_are_presented_with_the_vendor_pin_and_never_twice() {
        let stored = |pins: &[&str]| pins.iter().map(|pin| pin.to_string()).collect::<Vec<_>>();
        assert_eq!(pairing_candidates(&[]), vec!["000000"]);
        assert_eq!(pairing_candidates(&stored(&["123456"])), vec!["000000", "123456"]);
        assert_eq!(pairing_candidates(&stored(&["000000"])), vec!["000000"]);
        assert_eq!(pairing_candidates(&stored(&["12a456"])), vec!["000000"]);
        assert_eq!(reconnect_candidates(&stored(&["123456"])), vec!["123456", "000000"]);
        assert_eq!(reconnect_candidates(&[]), vec!["000000"]);
        assert_eq!(
            reconnect_candidates(&stored(&["123456", "654321"])),
            vec!["123456", "654321", "000000"],
            "two stored PINs are presented in the order given"
        );
    }

    #[test]
    fn only_the_vendor_pin_rotates_and_the_new_one_never_is_it() {
        assert!(rotates("000000"));
        assert!(!rotates("123456"));
        assert_eq!(target_pin([1, 2, 3, 14, 25, 36]), "123456");
        assert_eq!(target_pin([0, 10, 20, 30, 40, 50]), "000001");
        assert!(is_pin(&target_pin([9; 6])));
    }
}
