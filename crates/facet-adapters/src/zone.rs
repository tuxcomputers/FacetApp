//! The machine's time zone, as the operating system reports it.

use facet_core::port::Zone;

/// Names the zone from the operating system's own setting.
pub struct SystemZone;

impl Zone for SystemZone {
    fn name(&self) -> Result<String, String> {
        iana_time_zone::get_timezone().map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whatever zone this machine is in, the answer is a name the `timezone` table can file: not empty, and a
    /// Region/City or a plain name such as `UTC`.
    #[test]
    fn this_machine_can_name_its_zone() {
        let name = SystemZone.name().expect("the machine should name its zone");
        assert!(!name.trim().is_empty());
        assert!(name.chars().all(|c| c.is_ascii_alphanumeric() || "/_+-".contains(c)), "{name}");
    }
}
