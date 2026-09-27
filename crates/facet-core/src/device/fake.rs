//! A cube in memory, for tests: it holds a PIN, judges what is presented, and answers `0x10`, `0x07` and `0x30`.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::uuids;
use crate::port::{Advert, Link, Radio, RadioState};

#[derive(Default)]
struct State {
    pin: String,
    connections: usize,
    /// Every write to the command characteristic other than a question.
    commands: Vec<Vec<u8>>,
}

#[derive(Clone)]
pub struct FakeCube(Arc<Mutex<State>>);

impl FakeCube {
    pub fn new(pin: &str) -> FakeCube {
        FakeCube(Arc::new(Mutex::new(State { pin: pin.to_string(), ..State::default() })))
    }

    pub fn pin(&self) -> String {
        self.0.lock().expect("lock").pin.clone()
    }

    pub fn connections(&self) -> usize {
        self.0.lock().expect("lock").connections
    }

    /// The writes to the command characteristic that change something.
    pub fn commands(&self) -> Vec<Vec<u8>> {
        self.0.lock().expect("lock").commands.clone()
    }
}

impl Radio for FakeCube {
    fn state(&self) -> RadioState {
        RadioState::Ready
    }

    fn scan(
        &self,
        _duration: Duration,
        _stop: &AtomicBool,
        heard: &mut dyn FnMut(&Advert),
    ) -> Result<(), String> {
        heard(&Advert {
            handle: "cube".into(),
            name: Some("TimeFlip v2.0".into()),
            services: vec![],
            rssi: Some(-60),
        });
        Ok(())
    }

    fn connect(&self, _handle: &str, _timeout: Duration) -> Result<Box<dyn Link>, String> {
        self.0.lock().expect("lock").connections += 1;
        Ok(Box::new(FakeLink { cube: self.clone(), logged_in: false, result: Vec::new() }))
    }
}

struct FakeLink {
    cube: FakeCube,
    logged_in: bool,
    result: Vec<u8>,
}

impl Link for FakeLink {
    fn gap_name(&self) -> Option<String> {
        Some("TimeFlip v2.0".into())
    }

    fn has_characteristic(&self, _uuid: u128) -> bool {
        true
    }

    fn read(&mut self, uuid: u128) -> Result<Vec<u8>, String> {
        match uuid {
            uuids::COMMAND_RESULT => Ok(self.result.clone()),
            uuids::BATTERY_LEVEL => Ok(vec![87]),
            uuids::FIRMWARE_REVISION => Ok(b"FW_v3.64".to_vec()),
            uuids::MANUFACTURER_NAME => Ok(b"DI_LABS".to_vec()),
            _ => Ok(Vec::new()),
        }
    }

    fn write(&mut self, uuid: u128, bytes: &[u8]) -> Result<(), String> {
        let mut state = self.cube.0.lock().expect("lock");
        match uuid {
            uuids::PASSWORD => {
                self.logged_in = bytes == state.pin.as_bytes();
                self.result = vec![if self.logged_in { 0x02 } else { 0x01 }];
            }
            uuids::COMMAND if !self.logged_in => {}
            uuids::COMMAND => match bytes.first() {
                Some(0x10) => self.result = vec![0x02, 0x01, 0x00, 0x05],
                Some(0x07) => {
                    self.result = vec![0x07];
                    self.result.extend_from_slice(&1_789_886_547u64.to_be_bytes());
                }
                Some(0x30) => {
                    state.pin = String::from_utf8_lossy(&bytes[1..]).into_owned();
                    state.commands.push(bytes.to_vec());
                    self.result = vec![0x02];
                }
                _ => state.commands.push(bytes.to_vec()),
            },
            _ => {}
        }
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), String> {
        Ok(())
    }
}
