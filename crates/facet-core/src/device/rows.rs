//! The `setting` rows the app keeps about the cube: the five device settings, and what it knows about the
//! paired cube. Read at the point of use, written with a read-back.

use rusqlite::{Connection, OptionalExtension, params};

use crate::app_settings::{self, Value};
use crate::debug_log::{Record, Tag, plain};

/// The Device tab's settings, as the table holds them. An absent row or field reads as its seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceSettings {
    /// `pause_on_lock.enabled`: locking the cube pauses it first.
    pub pause_on_lock: bool,
    /// `low_battery_level.percent`.
    pub battery_warning_percent: i64,
    /// `auto_pause_minutes.minutes`; 0 is off.
    pub auto_pause_minutes: i64,
    /// `led_settings.brightness`.
    pub led_brightness_percent: i64,
    /// `led_settings.blink_interval`.
    pub led_blink_seconds: i64,
}

pub fn settings(connection: &Connection) -> Result<DeviceSettings, rusqlite::Error> {
    let field = |name: &str, path: &str| -> Result<Option<i64>, rusqlite::Error> {
        Ok(connection
            .query_row(
                "SELECT json_extract(setting_value, ?2) FROM setting WHERE setting_name = ?1",
                params![name, path],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten())
    };
    Ok(DeviceSettings {
        pause_on_lock: field("pause_on_lock", "$.enabled")?.map(|value| value != 0).unwrap_or(true),
        battery_warning_percent: field("low_battery_level", "$.percent")?.unwrap_or(10),
        auto_pause_minutes: field("auto_pause_minutes", "$.minutes")?.unwrap_or(0),
        led_brightness_percent: field("led_settings", "$.brightness")?.unwrap_or(50),
        led_blink_seconds: field("led_settings", "$.blink_interval")?.unwrap_or(15),
    })
}

/// One of the five device settings, naming the row and field it lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSetting {
    PauseOnLock,
    BatteryWarning,
    AutoPause,
    LedBrightness,
    LedBlink,
}

impl DeviceSetting {
    fn row(self) -> (&'static str, &'static str) {
        match self {
            DeviceSetting::PauseOnLock => ("pause_on_lock", "enabled"),
            DeviceSetting::BatteryWarning => ("low_battery_level", "percent"),
            DeviceSetting::AutoPause => ("auto_pause_minutes", "minutes"),
            DeviceSetting::LedBrightness => ("led_settings", "brightness"),
            DeviceSetting::LedBlink => ("led_settings", "blink_interval"),
        }
    }
}

/// Writes `value` into `setting`'s row and reads it back. Returns whether the table holds it.
pub fn write_setting(
    connection: &Connection,
    setting: DeviceSetting,
    value: &Value,
    log: &impl Record,
) -> Result<bool, rusqlite::Error> {
    let (name, field) = setting.row();
    app_settings::write(connection, name, field, value, log)
}

/// The four Device Information strings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceInfo {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub hardware: Option<String>,
    pub firmware: Option<String>,
}

/// What the table holds about the paired cube.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pairing {
    /// `paired.paired`.
    pub is_cube_paired: bool,
    /// `connection.connected`.
    pub is_cube_connected: bool,
    /// `device_uuid.uuid`: the platform's handle for the cube, a hint rather than an identity.
    pub handle: Option<String>,
    /// `device_name.name`: the cube's GAP name as last read.
    pub name: Option<String>,
    /// `device_name.previous_name`.
    pub previous_name: Option<String>,
    pub info: DeviceInfo,
}

pub fn pairing(connection: &Connection) -> Result<Pairing, rusqlite::Error> {
    let text = |name: &str, path: &str| -> Result<Option<String>, rusqlite::Error> {
        Ok(connection
            .query_row(
                "SELECT json_extract(setting_value, ?2) FROM setting WHERE setting_name = ?1",
                params![name, path],
                |row| row.get::<_, Option<rusqlite::types::Value>>(0),
            )
            .optional()?
            .flatten()
            .and_then(|value| match value {
                rusqlite::types::Value::Text(text) if !text.trim().is_empty() => Some(text),
                _ => None,
            }))
    };
    let flag = |name: &str, path: &str| -> Result<bool, rusqlite::Error> {
        Ok(connection
            .query_row(
                "SELECT json_extract(setting_value, ?2) FROM setting WHERE setting_name = ?1",
                params![name, path],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten()
            .is_some_and(|value| value != 0))
    };
    Ok(Pairing {
        is_cube_paired: flag("paired", "$.paired")?,
        is_cube_connected: flag("connection", "$.connected")?,
        handle: text("device_uuid", "$.uuid")?,
        name: text("device_name", "$.name")?,
        previous_name: text("device_name", "$.previous_name")?,
        info: DeviceInfo {
            manufacturer: text("device_info", "$.manufacturer")?,
            model: text("device_info", "$.model")?,
            hardware: text("device_info", "$.hardware")?,
            firmware: text("device_info", "$.firmware")?,
        },
    })
}

/// The names a scan also accepts as a TimeFlip: those `device_name` holds for a cube this app has renamed.
pub fn known_names(connection: &Connection) -> Result<Vec<String>, rusqlite::Error> {
    let pairing = pairing(connection)?;
    Ok([pairing.name, pairing.previous_name].into_iter().flatten().collect())
}

/// Sets one JSON field of a row, keeping its other fields. `value` is SQL that yields the value.
fn set(
    connection: &Connection,
    name: &str,
    path: &str,
    value: &str,
    bound: Option<&str>,
) -> Result<(), rusqlite::Error> {
    let sql = format!(
        "UPDATE setting SET setting_value = json_set(setting_value, ?2, {value}) WHERE setting_name = ?1"
    );
    match bound {
        Some(bound) => connection.execute(&sql, params![name, path, bound])?,
        None => connection.execute(&sql, params![name, path])?,
    };
    Ok(())
}

/// Records a successful login to the cube `handle` names: paired, connected now, its handle and GAP name, and
/// the Device Information strings that were read. A string not read keeps what the table held. Returns whether
/// the table holds all of it, and logs `Paired with <label> (<handle>)` or `Reconnected to <label> (<handle>)`.
pub fn record_login(
    connection: &Connection,
    handle: &str,
    gap_name: Option<&str>,
    info: &DeviceInfo,
    log: &impl Record,
) -> Result<bool, rusqlite::Error> {
    let was_paired = pairing(connection)?.is_cube_paired;
    set(connection, "paired", "$.paired", "json('true')", None)?;
    set(connection, "connection", "$.connected", "json('true')", None)?;
    set(
        connection,
        "connection",
        "$.last_connection",
        "strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime')",
        None,
    )?;
    set(connection, "connection", "$.connection_lost", "''", None)?;
    set(connection, "device_uuid", "$.uuid", "?3", Some(handle))?;
    let before = pairing(connection)?;
    let adopted = match gap_name.map(str::trim).filter(|name| !name.is_empty()) {
        Some(name) => {
            log.record(Tag::Pair, || format!("The cube now reports its name as {}", plain(name)));
            match name_adoption(&before, name) {
                NameAdoption::Adopt => record_name(connection, name, "the cube said so on connecting", log)?,
                NameAdoption::Unchanged => true,
                NameAdoption::Stale => {
                    log.record(Tag::Pair, || {
                        "The cube reports the name it had before the rename, which the system is a connection \
                         behind on, so the record stands"
                            .to_string()
                    });
                    true
                }
            }
        }
        None => true,
    };
    for (path, value) in [
        ("$.manufacturer", &info.manufacturer),
        ("$.model", &info.model),
        ("$.hardware", &info.hardware),
        ("$.firmware", &info.firmware),
    ] {
        if let Some(value) = value {
            set(connection, "device_info", path, "?3", Some(value))?;
        }
    }
    let held = pairing(connection)?;
    let stored =
        held.is_cube_paired && held.is_cube_connected && held.handle.as_deref() == Some(handle) && adopted;
    let label = plain(gap_name.unwrap_or("the cube"));
    log.record(Tag::Pair, || {
        format!(
            "{} {label} ({}){}",
            if was_paired { "Reconnected to" } else { "Paired with" },
            plain(handle),
            if stored { "" } else { " REFUSED, the table does not hold it" }
        )
    });
    Ok(stored)
}

/// Records that the link to the cube dropped: not connected, and when. Pairing is kept.
pub fn record_connection_lost(connection: &Connection, log: &impl Record) -> Result<bool, rusqlite::Error> {
    set(connection, "connection", "$.connected", "json('false')", None)?;
    set(
        connection,
        "connection",
        "$.connection_lost",
        "strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime')",
        None,
    )?;
    let stored = !pairing(connection)?.is_cube_connected;
    log.record(Tag::Pair, || {
        format!(
            "The link to the cube dropped{}",
            if stored { "" } else { " REFUSED, the table still says connected" }
        )
    });
    Ok(stored)
}

/// Records a clean quit: not connected, and `quit_request` set to now. Pairing is kept. Returns whether the table
/// holds it.
pub fn record_quit(connection: &Connection, log: &impl Record) -> Result<bool, rusqlite::Error> {
    set(connection, "connection", "$.connected", "json('false')", None)?;
    set(
        connection,
        "connection",
        "$.quit_request",
        "strftime('%Y-%m-%dT%H:%M:%S', 'now', 'localtime')",
        None,
    )?;
    let stored = !pairing(connection)?.is_cube_connected;
    log.record(Tag::Quit, || {
        format!(
            "Quit: the link to the cube is closed{}",
            if stored { "" } else { " REFUSED, the table still says connected" }
        )
    });
    Ok(stored)
}

/// Clears a `connection.connected` the last run left set: no link survives a relaunch. Returns whether the table
/// holds it; does nothing when the table already says not connected.
pub fn record_no_link_at_launch(connection: &Connection, log: &impl Record) -> Result<bool, rusqlite::Error> {
    if !pairing(connection)?.is_cube_connected {
        return Ok(true);
    }
    set(connection, "connection", "$.connected", "json('false')", None)?;
    let stored = !pairing(connection)?.is_cube_connected;
    log.record(Tag::Pair, || {
        format!(
            "The last run left the cube marked connected, and no link survives a relaunch{}",
            if stored { "" } else { " REFUSED, the table still says connected" }
        )
    });
    Ok(stored)
}

/// What a GAP name reported on connecting does to the name on record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameAdoption {
    /// The name on record already.
    Unchanged,
    /// The name a rename replaced, which the platform reports for one connection after a rename: the record stands.
    Stale,
    /// Anything else becomes the name on record.
    Adopt,
}

/// What `reported` does to the name `held` has on record.
pub fn name_adoption(held: &Pairing, reported: &str) -> NameAdoption {
    if held.name.as_deref() == Some(reported) {
        NameAdoption::Unchanged
    } else if held.name.is_some() && held.previous_name.as_deref() == Some(reported) {
        NameAdoption::Stale
    } else {
        NameAdoption::Adopt
    }
}

/// Records `name` as what the cube is called, `because` saying why. A different name already on record moves to
/// `previous_name`, which the scan filter keeps. Returns whether the table holds it; an empty name changes
/// nothing and is refused.
pub fn record_name(
    connection: &Connection,
    name: &str,
    because: &str,
    log: &impl Record,
) -> Result<bool, rusqlite::Error> {
    let name = name.trim();
    if name.is_empty() {
        log.record(Tag::Pair, || {
            "Asked to record an empty name for the cube, which is not a name, so nothing changed".to_string()
        });
        return Ok(false);
    }
    let before = pairing(connection)?;
    if let Some(previous) = before.name.as_deref().filter(|previous| *previous != name) {
        set(connection, "device_name", "$.previous_name", "?3", Some(previous))?;
        log.record(Tag::Pair, || {
            format!("The cube was called {}, which the scan filter keeps", plain(previous))
        });
    }
    set(connection, "device_name", "$.name", "?3", Some(name))?;
    let stored = pairing(connection)?.name.as_deref() == Some(name);
    log.record(Tag::Pair, || {
        if stored {
            format!("The cube is called {}: {because}", plain(name))
        } else {
            format!("THE NAME {} WAS NOT RECORDED, the table refused a write", plain(name))
        }
    });
    Ok(stored)
}

/// Records a confirmed factory reset: the cube is forgotten as [`record_forget`] does, and its name moves to
/// `previous_name`, so a scan still knows it in case the wipe did not take. The stored PIN is not the table's and is
/// left alone. Returns whether the table holds it.
pub fn record_factory_reset(connection: &Connection, log: &impl Record) -> Result<bool, rusqlite::Error> {
    let forgotten = record_forget(connection, log)?;
    let before = pairing(connection)?;
    if let Some(name) = before.name.as_deref() {
        set(connection, "device_name", "$.previous_name", "?3", Some(name))?;
        set(connection, "device_name", "$.name", "''", None)?;
        log.record(Tag::Pair, || {
            format!(
                "The cube was called {}; keeping it in the scan filter in case the wipe did not take",
                plain(name)
            )
        });
    }
    let held = pairing(connection)?;
    let stored =
        forgotten && held.name.is_none() && held.previous_name == before.name.or(before.previous_name);
    log.record(Tag::Pair, || {
        if stored {
            "Reset the cube and forgot it".to_string()
        } else {
            "RESET NOT FULLY RECORDED, the table refused a write".to_string()
        }
    });
    Ok(stored)
}

/// Forgets the paired cube: not paired, not connected, no handle, Device Information emptied. The name is kept,
/// so a scan can still recognise the cube by a name this app gave it. Returns whether the table holds it.
pub fn record_forget(connection: &Connection, log: &impl Record) -> Result<bool, rusqlite::Error> {
    set(connection, "paired", "$.paired", "json('false')", None)?;
    set(connection, "connection", "$.connected", "json('false')", None)?;
    connection.execute("UPDATE setting SET setting_value = json_remove(setting_value, '$.uuid') WHERE setting_name = 'device_uuid'", [])?;
    for path in ["$.manufacturer", "$.model", "$.hardware", "$.firmware"] {
        set(connection, "device_info", path, "''", None)?;
    }
    let held = pairing(connection)?;
    let stored = !held.is_cube_paired
        && !held.is_cube_connected
        && held.handle.is_none()
        && held.info == DeviceInfo::default();
    log.record(Tag::Pair, || {
        format!(
            "Forgot the device; the name it is carrying is kept so a scan can still find it{}",
            if stored { "" } else { " REFUSED, the table still holds it" }
        )
    });
    Ok(stored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_log::Trace;
    use crate::testing::seeded;

    #[test]
    fn a_fresh_database_holds_the_seeds_and_no_cube() {
        let connection = seeded();
        assert_eq!(
            settings(&connection).expect("read"),
            DeviceSettings {
                pause_on_lock: true,
                battery_warning_percent: 10,
                auto_pause_minutes: 0,
                led_brightness_percent: 50,
                led_blink_seconds: 15
            }
        );
        let held = pairing(&connection).expect("read");
        assert!(
            !held.is_cube_paired && !held.is_cube_connected && held.handle.is_none() && held.name.is_none()
        );
    }

    #[test]
    fn each_setting_writes_its_own_field() {
        let connection = seeded();
        let log = Trace::none();
        assert!(
            write_setting(&connection, DeviceSetting::LedBlink, &Value::Number(30), &log).expect("write")
        );
        assert!(
            write_setting(&connection, DeviceSetting::PauseOnLock, &Value::Flag(false), &log).expect("write")
        );
        let held = settings(&connection).expect("read");
        assert_eq!(
            (held.led_blink_seconds, held.led_brightness_percent, held.pause_on_lock),
            (30, 50, false)
        );
    }

    #[test]
    fn a_login_pairs_a_drop_disconnects_and_forget_keeps_the_name() {
        let connection = seeded();
        let log = Trace::none();
        let info = DeviceInfo { firmware: Some("FW_v3.64".into()), ..DeviceInfo::default() };
        assert!(
            record_login(&connection, "E8:DB:D8:CF:F9:0F", Some("TimeFlip v2.0"), &info, &log)
                .expect("login")
        );
        let held = pairing(&connection).expect("read");
        assert!(held.is_cube_paired && held.is_cube_connected);
        assert_eq!(held.handle.as_deref(), Some("E8:DB:D8:CF:F9:0F"));
        assert_eq!(held.info.firmware.as_deref(), Some("FW_v3.64"));
        assert_eq!(known_names(&connection).expect("read"), vec!["TimeFlip v2.0"]);

        assert!(record_connection_lost(&connection, &log).expect("drop"));
        assert!(pairing(&connection).expect("read").is_cube_paired);

        assert!(record_forget(&connection, &log).expect("forget"));
        let held = pairing(&connection).expect("read");
        assert!(!held.is_cube_paired && held.handle.is_none() && held.info.firmware.is_none());
        assert_eq!(held.name.as_deref(), Some("TimeFlip v2.0"));
    }

    #[test]
    fn a_rename_keeps_the_old_name_and_the_stale_report_after_it_is_ignored() {
        let connection = seeded();
        let log = Trace::none();
        let info = DeviceInfo::default();
        assert!(record_login(&connection, "cube", Some("TimeFlip v2.0"), &info, &log).expect("login"));
        assert!(record_name(&connection, "Facet cube", "renamed from the Device tab", &log).expect("rename"));
        let held = pairing(&connection).expect("read");
        assert_eq!(held.name.as_deref(), Some("Facet cube"));
        assert_eq!(held.previous_name.as_deref(), Some("TimeFlip v2.0"));

        // The next connection can still report the old name, which does not undo the rename.
        assert!(record_login(&connection, "cube", Some("TimeFlip v2.0"), &info, &log).expect("login"));
        assert_eq!(pairing(&connection).expect("read").name.as_deref(), Some("Facet cube"));
        assert!(record_login(&connection, "cube", Some("Facet cube"), &info, &log).expect("login"));
        assert_eq!(known_names(&connection).expect("read"), vec!["Facet cube", "TimeFlip v2.0"]);

        // A name from somewhere else is adopted, and the one it replaces is kept.
        assert!(record_login(&connection, "cube", Some("Desk"), &info, &log).expect("login"));
        let held = pairing(&connection).expect("read");
        assert_eq!(held.name.as_deref(), Some("Desk"));
        assert_eq!(held.previous_name.as_deref(), Some("Facet cube"));
        assert!(!record_name(&connection, "  ", "typed", &log).expect("empty"));
    }

    #[test]
    fn a_confirmed_reset_forgets_the_cube_and_keeps_its_name_for_the_scan() {
        let connection = seeded();
        let log = Trace::none();
        let info = DeviceInfo { firmware: Some("FW_v3.64".into()), ..DeviceInfo::default() };
        assert!(record_login(&connection, "cube", Some("Facet cube"), &info, &log).expect("login"));
        assert!(record_factory_reset(&connection, &log).expect("reset"));
        let held = pairing(&connection).expect("read");
        assert!(!held.is_cube_paired && !held.is_cube_connected && held.handle.is_none());
        assert!(held.info.firmware.is_none() && held.name.is_none());
        assert_eq!(held.previous_name.as_deref(), Some("Facet cube"));
        assert_eq!(known_names(&connection).expect("read"), vec!["Facet cube"]);
    }
}
