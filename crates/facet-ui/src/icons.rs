//! The activity icons, compiled in from `ui/icons/` by `build.rs`.

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

/// The SVG bytes of the icon called `name` (as stored in `icon.icon_name`, such as `ic_meeting`). `None` for
/// a name with no file.
pub fn svg(name: &str) -> Option<&'static [u8]> {
    ICONS.binary_search_by(|(candidate, _)| (*candidate).cmp(name)).ok().map(|index| ICONS[index].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_the_database_seeds_has_a_file() {
        let seeded = include_str!("../../facet-core/resources/database/004_icon.sql");
        let names: Vec<&str> =
            seeded.split('\'').skip(1).step_by(2).filter(|name| name.starts_with("ic_")).collect();
        assert!(names.len() >= 40, "expected the seeded icon names, found {}", names.len());
        for name in names {
            assert!(svg(name).is_some(), "no icon file for {name}");
        }
    }

    #[test]
    fn an_unknown_name_has_no_icon() {
        assert!(svg("ic_nothing_by_this_name").is_none());
        assert!(svg("None").is_none());
    }
}
