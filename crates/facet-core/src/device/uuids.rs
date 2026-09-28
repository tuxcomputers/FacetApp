//! The cube's services and characteristics, as 128-bit UUIDs.

/// A characteristic of the vendor service, `F1196F5x-71A4-11E6-BDF4-0800200C9A66`, by its last hex digit.
const fn vendor(last: u128) -> u128 {
    0xF119_6F50_71A4_11E6_BDF4_0800_200C_9A66 + (last << 96)
}

/// A standard 16-bit UUID in its 128-bit form, `0000xxxx-0000-1000-8000-00805F9B34FB`.
const fn standard(short: u128) -> u128 {
    0x0000_0000_0000_1000_8000_0080_5F9B_34FB + (short << 96)
}

pub const TIMEFLIP_SERVICE: u128 = vendor(0x0);
/// ASCII narration of each command the cube carries out. Read and notify.
pub const EVENTS_DATA: u128 = vendor(0x1);
/// The face that is up, 1 to 12; 0 when undefined or when no PIN has been accepted.
pub const FACES: u128 = vendor(0x2);
/// The answer to the last command that updates it. Read, and notify.
pub const COMMAND_RESULT: u128 = vendor(0x3);
pub const COMMAND: u128 = vendor(0x4);
pub const DOUBLE_TAP: u128 = vendor(0x5);
pub const SYSTEM_STATE: u128 = vendor(0x6);
/// Takes the six ASCII digits of a PIN. Write with response only.
pub const PASSWORD: u128 = vendor(0x7);
pub const HISTORY: u128 = vendor(0x8);

pub const BATTERY_LEVEL: u128 = standard(0x2A19);
pub const MANUFACTURER_NAME: u128 = standard(0x2A29);
pub const MODEL_NUMBER: u128 = standard(0x2A24);
pub const HARDWARE_REVISION: u128 = standard(0x2A27);
pub const FIRMWARE_REVISION: u128 = standard(0x2A26);

/// The name a trace uses for `uuid`, or its hex form when it is not one of the cube's.
pub fn name(uuid: u128) -> String {
    match uuid {
        TIMEFLIP_SERVICE => "timeFlipService".into(),
        EVENTS_DATA => "eventsData".into(),
        FACES => "faces".into(),
        COMMAND_RESULT => "commandResult".into(),
        COMMAND => "command".into(),
        DOUBLE_TAP => "doubleTap".into(),
        SYSTEM_STATE => "systemState".into(),
        PASSWORD => "password".into(),
        HISTORY => "history".into(),
        BATTERY_LEVEL => "batteryLevel".into(),
        MANUFACTURER_NAME => "manufacturerName".into(),
        MODEL_NUMBER => "modelNumber".into(),
        HARDWARE_REVISION => "hardwareRevision".into(),
        FIRMWARE_REVISION => "firmwareRevision".into(),
        other => format!("{other:032x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_uuids_are_the_vendor_and_standard_ones() {
        assert_eq!(TIMEFLIP_SERVICE, 0xf1196f50_71a4_11e6_bdf4_0800200c9a66);
        assert_eq!(PASSWORD, 0xf1196f57_71a4_11e6_bdf4_0800200c9a66);
        assert_eq!(HISTORY, 0xf1196f58_71a4_11e6_bdf4_0800200c9a66);
        assert_eq!(BATTERY_LEVEL, 0x00002a19_0000_1000_8000_00805f9b34fb);
        assert_eq!(name(COMMAND_RESULT), "commandResult");
        assert_eq!(name(1), "00000000000000000000000000000001");
    }
}
