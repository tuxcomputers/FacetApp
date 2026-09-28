//! The system state characteristic (`...56`): what the cube says it needs sent again, and what it says about its own
//! hardware. Four bytes: the sync state in the first two, the hardware state in the last two.

/// What the cube says it needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CubeSyncState {
    Ok,
    /// `01 00`: it has been put back to the factory, so everything is to be sent again.
    FactoryReset,
    TimeRequired,
    FaceColoursRequired,
    LedBrightnessRequired,
    BlinkIntervalRequired,
    /// `02 05`: task parameters, which this app has never set.
    TaskParametersRequired,
    AutoPauseRequired,
    Unknown,
}

/// What the cube says about its hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CubeHardwareState {
    Ok,
    Accelerometer,
    Flash,
    AccelerometerAndFlash,
    Unknown,
}

/// Reads a system state value. `None` for one shorter than four bytes.
pub fn parse(bytes: &[u8]) -> Option<(CubeSyncState, CubeHardwareState)> {
    let [sync_high, sync_low, hardware_high, hardware_low] = *bytes.get(..4)? else { return None };
    let sync = match (sync_high, sync_low) {
        (0x00, 0x00) => CubeSyncState::Ok,
        (0x01, 0x00) => CubeSyncState::FactoryReset,
        (0x02, 0x01) => CubeSyncState::TimeRequired,
        (0x02, 0x02) => CubeSyncState::FaceColoursRequired,
        (0x02, 0x03) => CubeSyncState::LedBrightnessRequired,
        (0x02, 0x04) => CubeSyncState::BlinkIntervalRequired,
        (0x02, 0x05) => CubeSyncState::TaskParametersRequired,
        (0x02, 0x06) => CubeSyncState::AutoPauseRequired,
        _ => CubeSyncState::Unknown,
    };
    let hardware = match (hardware_high, hardware_low) {
        (0x00, 0x00) => CubeHardwareState::Ok,
        (0x02, 0x01) => CubeHardwareState::Accelerometer,
        (0x02, 0x02) => CubeHardwareState::Flash,
        (0x02, 0x03) => CubeHardwareState::AccelerometerAndFlash,
        _ => CubeHardwareState::Unknown,
    };
    Some((sync, hardware))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_codes_read_as_the_spec_lists_them() {
        assert_eq!(parse(&[0, 0, 0, 0]), Some((CubeSyncState::Ok, CubeHardwareState::Ok)));
        assert_eq!(parse(&[1, 0, 0, 0]), Some((CubeSyncState::FactoryReset, CubeHardwareState::Ok)));
        assert_eq!(
            parse(&[2, 5, 0, 0]),
            Some((CubeSyncState::TaskParametersRequired, CubeHardwareState::Ok))
        );
        assert_eq!(
            parse(&[2, 2, 2, 1]),
            Some((CubeSyncState::FaceColoursRequired, CubeHardwareState::Accelerometer))
        );
        assert_eq!(parse(&[9, 9, 9, 9]), Some((CubeSyncState::Unknown, CubeHardwareState::Unknown)));
        assert_eq!(parse(&[2, 1]), None);
    }
}
