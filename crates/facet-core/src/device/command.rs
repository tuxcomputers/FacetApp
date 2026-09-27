//! The cube's commands, written to the command characteristic, and the answers read from the command result.
//!
//! Which commands can be confirmed is a matrix, written out in `docs/timeflip.md`: `0x10` reads back lock,
//! pause and auto-pause; `0x07` reads back the clock; LED brightness, blink interval and face colour have no
//! read-back at all.

/// Ask for the cube's clock. Answered by `[0x07]` then a big-endian u64 of seconds.
pub const READ_TIME: u8 = 0x07;
/// Ask for lock, pause and auto-pause. Answered by four bare bytes with no echoed command byte.
pub const READ_STATUS: u8 = 0x10;

/// The auto-pause control's range, in whole minutes. 0 turns auto-pause off.
pub const AUTO_PAUSE_MINUTES_RANGE: (i64, i64) = (0, 240);
/// The LED brightness control's range, in percent.
pub const LED_BRIGHTNESS_RANGE: (i64, i64) = (1, 100);
/// The LED blink interval control's range, in seconds.
pub const LED_BLINK_SECONDS_RANGE: (i64, i64) = (5, 60);
/// The battery warning control's range, in percent.
pub const BATTERY_WARNING_RANGE: (i64, i64) = (1, 20);

/// `0x05`: auto-pause after `minutes`, clamped to [`AUTO_PAUSE_MINUTES_RANGE`], as a big-endian u16.
pub fn set_auto_pause(minutes: i64) -> Vec<u8> {
    let minutes = minutes.clamp(AUTO_PAUSE_MINUTES_RANGE.0, AUTO_PAUSE_MINUTES_RANGE.1) as u16;
    let [high, low] = minutes.to_be_bytes();
    vec![0x05, high, low]
}

/// `0x09`: LED brightness, clamped to [`LED_BRIGHTNESS_RANGE`]. There is no read-back for it.
pub fn set_led_brightness(percent: i64) -> Vec<u8> {
    vec![0x09, percent.clamp(LED_BRIGHTNESS_RANGE.0, LED_BRIGHTNESS_RANGE.1) as u8]
}

/// `0x0A`: LED blink interval, clamped to [`LED_BLINK_SECONDS_RANGE`]. There is no read-back for it.
pub fn set_led_blink(seconds: i64) -> Vec<u8> {
    vec![0x0A, seconds.clamp(LED_BLINK_SECONDS_RANGE.0, LED_BLINK_SECONDS_RANGE.1) as u8]
}

/// What `0x10` says about the cube.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CubeStatus {
    pub is_locked: bool,
    /// **Reported as paused whenever the cube is locked**, whatever its pause byte says, so a pause is
    /// confirmed before a lock is sent, never after.
    pub is_paused: bool,
    pub auto_pause_minutes: u16,
}

/// Reads a `0x10` answer: lock and pause each `01` on or `02` off, then auto-pause minutes as a big-endian u16.
/// `None` for anything else, including a stale answer to another command, which a `0x10` answer cannot be told
/// apart from except by these values.
pub fn status(answer: &[u8]) -> Option<CubeStatus> {
    let on_off = |byte: u8| match byte {
        0x01 => Some(true),
        0x02 => Some(false),
        _ => None,
    };
    let [lock, pause, high, low, ..] = answer else { return None };
    let is_locked = on_off(*lock)?;
    let is_paused = on_off(*pause)?;
    Some(CubeStatus {
        is_locked,
        is_paused: is_locked || is_paused,
        auto_pause_minutes: u16::from_be_bytes([*high, *low]),
    })
}

/// Reads a `0x07` answer: the cube's clock in seconds since 1970. `None` when the answer is not a `0x07` one.
pub fn clock(answer: &[u8]) -> Option<u64> {
    let (&first, rest) = answer.split_first()?;
    if first != READ_TIME || rest.len() < 8 {
        return None;
    }
    let seconds = u64::from_be_bytes(rest[..8].try_into().ok()?);
    Some(seconds)
}

/// Bytes as the trace shows them: lower-case hex pairs separated by spaces.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_commands_clamp_and_encode() {
        assert_eq!(set_auto_pause(5), vec![0x05, 0x00, 0x05]);
        assert_eq!(set_auto_pause(300), vec![0x05, 0x00, 240]);
        assert_eq!(set_auto_pause(-1), vec![0x05, 0x00, 0x00]);
        assert_eq!(set_led_brightness(0), vec![0x09, 1]);
        assert_eq!(set_led_blink(90), vec![0x0A, 60]);
    }

    #[test]
    fn a_locked_cube_reads_as_paused_and_other_answers_are_refused() {
        assert_eq!(
            status(&[0x02, 0x01, 0x00, 0x05]),
            Some(CubeStatus { is_locked: false, is_paused: true, auto_pause_minutes: 5 })
        );
        assert_eq!(
            status(&[0x01, 0x02, 0x00, 0x00]),
            Some(CubeStatus { is_locked: true, is_paused: true, auto_pause_minutes: 0 })
        );
        assert_eq!(status(&[0x17, 0x3A, 0x5A, 0x3B]), None);
        assert_eq!(status(&[0x02]), None);
    }

    #[test]
    fn the_clock_is_a_big_endian_u64_after_its_echo() {
        let mut answer = vec![0x07];
        answer.extend_from_slice(&1_789_886_547u64.to_be_bytes());
        assert_eq!(clock(&answer), Some(1_789_886_547));
        assert_eq!(clock(&[0x02]), None);
        assert_eq!(hex(&[0x02, 0xab]), "02 ab");
    }
}
