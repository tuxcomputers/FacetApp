//! Driving the cube over a [`Link`]: presenting a PIN, moving a cube onto the app's own PIN, asking it the
//! questions that change nothing, and reading what it reports about itself.
//!
//! **Each PIN is presented on a connection of its own.** A refused PIN leaves the link unusable for another,
//! so [`log_in`] disconnects and connects again between candidates, and never tries a third.

use std::time::Duration;

use super::command::{self, CubeStatus};
use super::info;
use super::login::{self, Verdict};
use super::rows::DeviceInfo;
use super::uuids;
use crate::debug_log::{Record, Tag};
use crate::port::{Link, Radio};

/// How long a connection, discovery included, may take.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// How long to wait after a refused PIN before connecting again for the next one.
pub const SETTLE_BETWEEN_CANDIDATES: Duration = Duration::from_secs(1);

/// How a login ended.
pub enum LoginOutcome {
    /// A PIN was accepted, and the cube is now on `pin`. `link` is the live connection.
    LoggedIn { link: Box<dyn Link>, pin: String, rotated: bool },
    /// Every PIN presented was refused.
    Refused,
    /// The device has no password or command result characteristic, so it is not a TimeFlip.
    NotATimeFlip,
    /// The cube took the vendor PIN, was sent a new one, and then refused it. Pulling its batteries puts it
    /// back on the vendor PIN.
    NewPinRefused,
    /// The connection or an exchange failed, with the reason.
    Failed(String),
}

impl LoginOutcome {
    /// What the Device tab's status line says about the outcome, for the cube called `label`.
    pub fn describe(&self, label: &str) -> String {
        match self {
            LoginOutcome::LoggedIn { .. } => format!("Connected to {label}."),
            LoginOutcome::Refused => format!(
                "{label} refused the PIN. It may be paired with another app; taking its batteries out puts it back on the \
                 factory PIN."
            ),
            LoginOutcome::NotATimeFlip => format!("{label} is not a TimeFlip."),
            LoginOutcome::NewPinRefused => format!(
                "{label} would not take the new PIN. Take its batteries out to put it back on the factory PIN, then try \
                 again."
            ),
            LoginOutcome::Failed(reason) => format!("Could not connect to {label}: {reason}"),
        }
    }
}

/// Writes `pin` to the password characteristic and reads the verdict from the command result.
pub fn present_pin(link: &mut dyn Link, pin: &str, log: &impl Record) -> Result<Verdict, String> {
    log.record(Tag::Login, || "Presenting a PIN".to_string());
    link.write(uuids::PASSWORD, pin.as_bytes())?;
    let answer = link.read(uuids::COMMAND_RESULT)?;
    let verdict = login::verdict(&answer);
    log.record(Tag::Login, || match verdict {
        Verdict::Accepted => "PIN accepted".to_string(),
        Verdict::Refused => "PIN refused".to_string(),
        Verdict::Unreadable => format!("The answer to the PIN could not be read ({})", command::hex(&answer)),
    });
    Ok(verdict)
}

/// Moves a logged-in cube onto `new_pin` with `0x30`, then proves it by presenting `new_pin`. The `0x30` answer
/// is not judged: the command result may still hold an earlier reply. Returns whether the cube accepted
/// `new_pin`.
pub fn rotate_pin(link: &mut dyn Link, new_pin: &str, log: &impl Record) -> Result<bool, String> {
    log.record(Tag::Pin, || "Setting the PIN on the cube to one of its own".to_string());
    let mut bytes = vec![0x30];
    bytes.extend_from_slice(new_pin.as_bytes());
    link.write(uuids::COMMAND, &bytes)?;
    let _unjudged = link.read(uuids::COMMAND_RESULT)?;
    let accepted = present_pin(link, new_pin, log)? == Verdict::Accepted;
    log.record(Tag::Pin, || {
        if accepted {
            "The cube is now on its new PIN".to_string()
        } else {
            "The cube would not log in with the new PIN, so it is not being recorded".to_string()
        }
    });
    Ok(accepted)
}

/// Logs in to the device `handle` names, presenting each of `pins` on its own connection until one is accepted.
/// A cube that accepts the vendor PIN is moved onto `new_pin` when one is given; `None` leaves it where it is.
pub fn log_in(
    radio: &dyn Radio,
    handle: &str,
    pins: &[String],
    new_pin: Option<&str>,
    log: &impl Record,
) -> LoginOutcome {
    for (index, pin) in pins.iter().enumerate() {
        if index > 0 {
            std::thread::sleep(SETTLE_BETWEEN_CANDIDATES);
        }
        let mut link = match radio.connect(handle, CONNECT_TIMEOUT) {
            Ok(link) => link,
            Err(reason) => return LoginOutcome::Failed(reason),
        };
        if !link.has_characteristic(uuids::PASSWORD) || !link.has_characteristic(uuids::COMMAND_RESULT) {
            disconnect(&mut *link, log);
            return LoginOutcome::NotATimeFlip;
        }
        match present_pin(&mut *link, pin, log) {
            Ok(Verdict::Accepted) => {
                let Some(new_pin) = new_pin.filter(|_| login::rotates(pin)) else {
                    return LoginOutcome::LoggedIn { link, pin: pin.clone(), rotated: false };
                };
                return match rotate_pin(&mut *link, new_pin, log) {
                    Ok(true) => LoginOutcome::LoggedIn { link, pin: new_pin.to_string(), rotated: true },
                    Ok(false) => {
                        disconnect(&mut *link, log);
                        LoginOutcome::NewPinRefused
                    }
                    Err(reason) => {
                        disconnect(&mut *link, log);
                        LoginOutcome::Failed(reason)
                    }
                };
            }
            Ok(Verdict::Refused) => {
                disconnect(&mut *link, log);
                if index + 1 < pins.len() {
                    log.record(Tag::Login, || "Refused, and there is another PIN to try".to_string());
                }
            }
            Ok(Verdict::Unreadable) => {
                disconnect(&mut *link, log);
                return LoginOutcome::Failed("the answer to the PIN could not be read".to_string());
            }
            Err(reason) => {
                disconnect(&mut *link, log);
                return LoginOutcome::Failed(reason);
            }
        }
    }
    LoginOutcome::Refused
}

/// Disconnects, saying so when the disconnect itself fails.
pub fn disconnect(link: &mut dyn Link, log: &impl Record) {
    if let Err(reason) = link.disconnect() {
        log.record_failure(Tag::Radio, || format!("The disconnect failed: {reason}"));
    }
}

/// Sends a command that changes nothing on the cube and returns its answer from the command result.
fn ask(link: &mut dyn Link, question: u8, log: &impl Record) -> Result<Vec<u8>, String> {
    log.record(Tag::Command, || format!("Sending {}", command::hex(&[question])));
    link.write(uuids::COMMAND, &[question])?;
    link.read(uuids::COMMAND_RESULT)
}

/// Asks a logged-in cube for its lock, pause and auto-pause with `0x10`.
pub fn status(link: &mut dyn Link, log: &impl Record) -> Result<CubeStatus, String> {
    let answer = ask(link, command::READ_STATUS, log)?;
    let status = command::status(&answer)
        .ok_or_else(|| format!("the answer to 0x10 was not a status ({})", command::hex(&answer)))?;
    log.record(Tag::Command, || {
        format!(
            "The cube is {} and {}{}",
            if status.is_locked { "locked" } else { "unlocked" },
            if status.is_paused { "paused" } else { "running" },
            if status.auto_pause_minutes > 0 {
                format!(", pausing itself after {}m", status.auto_pause_minutes)
            } else {
                String::new()
            }
        )
    });
    Ok(status)
}

/// Asks a logged-in cube for its clock with `0x07`, in seconds since 1970.
pub fn clock(link: &mut dyn Link, log: &impl Record) -> Result<u64, String> {
    let answer = ask(link, command::READ_TIME, log)?;
    command::clock(&answer)
        .ok_or_else(|| format!("the answer to 0x07 was not a clock ({})", command::hex(&answer)))
}

/// Reads the battery level in percent. `None` when the cube did not say.
pub fn battery(link: &mut dyn Link, log: &impl Record) -> Option<u8> {
    match link.read(uuids::BATTERY_LEVEL) {
        Ok(bytes) => info::battery_percent(&bytes),
        Err(reason) => {
            log.record(Tag::Device, || format!("The battery could not be read: {reason}"));
            None
        }
    }
}

/// Reads the four Device Information strings. One that cannot be read is `None`.
pub fn device_info(link: &mut dyn Link, log: &impl Record) -> DeviceInfo {
    let mut read = |uuid: u128| match link.read(uuid) {
        Ok(bytes) => info::text(&bytes),
        Err(reason) => {
            log.record(Tag::Device, || format!("{} could not be read: {reason}", uuids::name(uuid)));
            None
        }
    };
    DeviceInfo {
        manufacturer: read(uuids::MANUFACTURER_NAME),
        model: read(uuids::MODEL_NUMBER),
        hardware: read(uuids::HARDWARE_REVISION),
        firmware: read(uuids::FIRMWARE_REVISION),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_log::Trace;
    use crate::device::fake::FakeCube;

    fn pins(list: &[&str]) -> Vec<String> {
        list.iter().map(|pin| pin.to_string()).collect()
    }

    #[test]
    fn the_vendor_pin_logs_in_and_rotates_when_asked() {
        let cube = FakeCube::new("000000");
        match log_in(&cube, "cube", &pins(&["000000"]), Some("123456"), &Trace::none()) {
            LoginOutcome::LoggedIn { pin, rotated, .. } => {
                assert_eq!((pin.as_str(), rotated), ("123456", true))
            }
            _ => panic!("expected a login"),
        }
        assert_eq!(cube.pin(), "123456");
    }

    #[test]
    fn without_a_new_pin_nothing_on_the_cube_changes() {
        let cube = FakeCube::new("000000");
        match log_in(&cube, "cube", &pins(&["000000"]), None, &Trace::none()) {
            LoginOutcome::LoggedIn { rotated, .. } => assert!(!rotated),
            _ => panic!("expected a login"),
        }
        assert_eq!(cube.pin(), "000000");
        assert!(cube.commands().is_empty());
    }

    #[test]
    fn a_cube_on_another_pin_refuses_both_on_two_connections() {
        let cube = FakeCube::new("654321");
        assert!(matches!(
            log_in(&cube, "cube", &pins(&["000000", "123456"]), Some("111111"), &Trace::none()),
            LoginOutcome::Refused
        ));
        assert_eq!(cube.connections(), 2);
        assert_eq!(cube.pin(), "654321");
    }

    #[test]
    fn the_stored_pin_logs_in_without_rotating() {
        let cube = FakeCube::new("123456");
        match log_in(&cube, "cube", &pins(&["000000", "123456"]), Some("999999"), &Trace::none()) {
            LoginOutcome::LoggedIn { pin, rotated, .. } => {
                assert_eq!((pin.as_str(), rotated), ("123456", false))
            }
            _ => panic!("expected a login"),
        }
    }

    #[test]
    fn a_logged_in_cube_answers_status_clock_battery_and_info() {
        let cube = FakeCube::new("000000");
        let LoginOutcome::LoggedIn { mut link, .. } =
            log_in(&cube, "cube", &pins(&["000000"]), None, &Trace::none())
        else {
            panic!("expected a login")
        };
        let log = Trace::none();
        assert_eq!(
            status(&mut *link, &log),
            Ok(CubeStatus { is_locked: false, is_paused: true, auto_pause_minutes: 5 })
        );
        assert_eq!(clock(&mut *link, &log), Ok(1_789_886_547));
        assert_eq!(battery(&mut *link, &log), Some(87));
        assert_eq!(device_info(&mut *link, &log).firmware.as_deref(), Some("FW_v3.64"));
    }
}
