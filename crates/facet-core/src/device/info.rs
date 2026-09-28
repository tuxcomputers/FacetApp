//! Battery and Device Information, as the cube reports them.

/// A Device Information string: the bytes as UTF-8, trailing NULs and whitespace removed. `None` when that
/// leaves nothing.
pub fn text(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_end_matches('\0').trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// The battery level in percent from a Battery Level read. `None` for an empty read or a value over 100.
pub fn battery_percent(bytes: &[u8]) -> Option<u8> {
    bytes.first().copied().filter(|percent| *percent <= 100)
}

/// How much higher a reading must be than the charge shown before the shown charge rises to it. A lower reading is
/// taken at once.
pub const RISE_TO_ADOPT: u8 = 2;
/// How far above the warning level the charge must climb before the warning goes off again.
pub const WARNING_CLEARS_ABOVE: u8 = 5;

/// The charge to show after `reading` arrives, given the charge `shown` now. A reading outside 1 to 100 is ignored.
pub fn charge_to_show(shown: Option<u8>, reading: u8) -> Option<u8> {
    if !(1..=100).contains(&reading) {
        return shown;
    }
    match shown {
        Some(shown) if reading > shown && reading - shown < RISE_TO_ADOPT => Some(shown),
        _ => Some(reading),
    }
}

/// Whether the low-battery warning is on for a charge of `percent` against a warning level of `warning`, given
/// whether it `was_low`: on at or below the level, and off only once the charge is more than
/// [`WARNING_CLEARS_ABOVE`] above it. No charge is not low.
pub fn is_battery_low(percent: Option<u8>, warning: u8, was_low: bool) -> bool {
    match percent {
        None => false,
        Some(percent) if percent <= warning => true,
        Some(percent) => was_low && percent <= warning.saturating_add(WARNING_CLEARS_ABOVE),
    }
}

/// What a row on the Device tab shows for a value: `Not paired` when there is no paired cube, `Unknown` when the
/// value has not been read, otherwise the value.
pub fn shown(is_cube_paired: bool, value: Option<&str>) -> String {
    match (is_cube_paired, value.map(str::trim)) {
        (false, _) => "Not paired".to_string(),
        (true, Some(value)) if !value.is_empty() => value.to_string(),
        (true, _) => "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_lose_their_padding_and_battery_is_a_percentage() {
        assert_eq!(text(b"DI_LABS\0\0"), Some("DI_LABS".to_string()));
        assert_eq!(text(b"\0\0"), None);
        assert_eq!(battery_percent(&[87]), Some(87));
        assert_eq!(battery_percent(&[101]), None);
        assert_eq!(battery_percent(&[]), None);
    }

    #[test]
    fn a_small_rise_is_held_and_a_fall_is_taken_at_once() {
        assert_eq!(charge_to_show(None, 87), Some(87));
        assert_eq!(charge_to_show(Some(87), 88), Some(87));
        assert_eq!(charge_to_show(Some(87), 89), Some(89));
        assert_eq!(charge_to_show(Some(87), 86), Some(86));
        assert_eq!(charge_to_show(Some(87), 0), Some(87));
    }

    #[test]
    fn the_warning_comes_on_at_the_level_and_goes_off_five_above_it() {
        assert!(is_battery_low(Some(10), 10, false));
        assert!(!is_battery_low(Some(11), 10, false));
        assert!(is_battery_low(Some(15), 10, true));
        assert!(!is_battery_low(Some(16), 10, true));
        assert!(!is_battery_low(None, 10, true));
    }

    #[test]
    fn rows_say_not_paired_unknown_or_the_value() {
        assert_eq!(shown(false, Some("FW_v3.64")), "Not paired");
        assert_eq!(shown(true, None), "Unknown");
        assert_eq!(shown(true, Some(" ")), "Unknown");
        assert_eq!(shown(true, Some("FW_v3.64")), "FW_v3.64");
    }
}
