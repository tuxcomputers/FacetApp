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
    fn rows_say_not_paired_unknown_or_the_value() {
        assert_eq!(shown(false, Some("FW_v3.64")), "Not paired");
        assert_eq!(shown(true, None), "Unknown");
        assert_eq!(shown(true, Some(" ")), "Unknown");
        assert_eq!(shown(true, Some("FW_v3.64")), "FW_v3.64");
    }
}
