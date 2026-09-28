//! Which devices a scan offers, and what each is called in the list.

use super::uuids;
use crate::port::Advert;

/// The substring, lower case, a TimeFlip's advertised or GAP name carries.
pub const VENDOR_NAME: &str = "timeflip";

/// How long a scan runs before it stops by itself.
pub const SCAN_SECONDS: u64 = 15;

/// Whether `advert` belongs in the list. With `all_devices` every device does. Otherwise one that lists the
/// TimeFlip service, or whose name contains `timeflip` (any case), or equals one of `known_names` exactly: the
/// names `device_name` holds for a cube this app has renamed. The cube advertises no service UUID, so the name
/// is what usually decides.
pub fn is_eligible(advert: &Advert, known_names: &[String], all_devices: bool) -> bool {
    if all_devices || advert.services.contains(&uuids::TIMEFLIP_SERVICE) {
        return true;
    }
    let Some(name) = advert.name.as_deref() else { return false };
    name.to_lowercase().contains(VENDOR_NAME)
        || known_names.iter().any(|known| !known.is_empty() && known == name)
}

/// What the list shows for `advert`: its name, or `Unnamed device`.
pub fn label(advert: &Advert) -> String {
    match advert.name.as_deref().map(str::trim) {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => "Unnamed device".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advert(name: Option<&str>, services: Vec<u128>) -> Advert {
        Advert { handle: "h".into(), name: name.map(str::to_string), services, rssi: None }
    }

    #[test]
    fn a_timeflip_is_found_by_name_service_or_a_known_name() {
        assert!(is_eligible(&advert(Some("TimeFlip v2.0"), vec![]), &[], false));
        assert!(is_eligible(&advert(None, vec![uuids::TIMEFLIP_SERVICE]), &[], false));
        assert!(is_eligible(&advert(Some("Desk"), vec![]), &["Desk".into()], false));
        assert!(!is_eligible(&advert(Some("Desk lamp"), vec![]), &["Desk".into()], false));
        assert!(!is_eligible(&advert(Some("Smart Monitor"), vec![]), &[String::new()], false));
        assert!(!is_eligible(&advert(None, vec![]), &[], false));
        assert!(is_eligible(&advert(None, vec![]), &[], true));
    }

    #[test]
    fn an_unnamed_device_is_labelled_so() {
        assert_eq!(label(&advert(Some(" TimeFlip v2.0 "), vec![])), "TimeFlip v2.0");
        assert_eq!(label(&advert(Some("  "), vec![])), "Unnamed device");
        assert_eq!(label(&advert(None, vec![])), "Unnamed device");
    }
}
