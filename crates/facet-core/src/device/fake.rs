//! A cube in memory, for tests: it holds a PIN, judges what is presented, and answers `0x10`, `0x07`, `0x30` and
//! `0xFF`.

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
    /// How many connections after `0xFF` still find the old PIN, as a wipe still running does.
    wipe_after: usize,
    /// Connections left before a pending wipe finishes; `None` when none is pending.
    wiping: Option<usize>,
    /// The cube's clock, as `0x07` reports it and `0x08` sets it.
    clock: u64,
    /// The face that is up.
    face: u8,
    /// The cube's history, as frames of 17 bytes, oldest first.
    history: Vec<[u8; 17]>,
    /// Where each connection's history notifications go.
    history_feeds: Vec<std::sync::mpsc::Sender<Vec<u8>>>,
}

#[derive(Clone)]
pub struct FakeCube(Arc<Mutex<State>>);

impl FakeCube {
    pub fn new(pin: &str) -> FakeCube {
        FakeCube(Arc::new(Mutex::new(State {
            pin: pin.to_string(),
            clock: 1_789_886_547,
            face: 2,
            ..State::default()
        })))
    }

    pub fn pin(&self) -> String {
        self.0.lock().expect("lock").pin.clone()
    }

    pub fn connections(&self) -> usize {
        self.0.lock().expect("lock").connections
    }

    /// Makes a `0xFF` finish only after `connections` further connections have found the old PIN.
    pub fn wipe_after(&self, connections: usize) {
        self.0.lock().expect("lock").wipe_after = connections;
    }

    /// Adds an event to the cube's history.
    pub fn file(&self, frame: &super::history::Frame) {
        let mut bytes = [0u8; 17];
        bytes[0..4].copy_from_slice(&frame.event_number.to_be_bytes());
        bytes[4] = frame.face + if frame.is_paused { 128 } else { 0 };
        bytes[5..13].copy_from_slice(&frame.start_epoch.to_be_bytes());
        bytes[13..17].copy_from_slice(&frame.duration_seconds.to_be_bytes());
        let mut state = self.0.lock().expect("lock");
        match state.history.iter_mut().find(|held| held[0..4] == bytes[0..4]) {
            Some(held) => *held = bytes,
            None => state.history.push(bytes),
        }
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
        let mut state = self.0.lock().expect("lock");
        state.connections += 1;
        match state.wiping {
            Some(0) => {
                state.pin = "000000".to_string();
                state.wiping = None;
            }
            Some(left) => state.wiping = Some(left - 1),
            None => {}
        }
        drop(state);
        Ok(Box::new(FakeLink {
            cube: self.clone(),
            logged_in: false,
            result: Vec::new(),
            history_answer: Vec::new(),
        }))
    }
}

struct FakeLink {
    cube: FakeCube,
    logged_in: bool,
    result: Vec<u8>,
    history_answer: Vec<u8>,
}

impl Link for FakeLink {
    fn gap_name(&self) -> Option<String> {
        Some("TimeFlip v2.0".into())
    }

    fn has_characteristic(&self, _uuid: u128) -> bool {
        true
    }

    fn characteristics(&self) -> Vec<u128> {
        vec![uuids::PASSWORD, uuids::COMMAND_RESULT, uuids::COMMAND, uuids::BATTERY_LEVEL]
    }

    fn subscribe(&mut self, uuid: u128) -> Result<std::sync::mpsc::Receiver<Vec<u8>>, String> {
        let (sender, receiver) = std::sync::mpsc::channel();
        if uuid == uuids::HISTORY {
            self.cube.0.lock().expect("lock").history_feeds.push(sender);
            return Ok(receiver);
        }
        if sender.send(vec![87]).is_err() {
            return Err("the notification channel closed at once".into());
        }
        Ok(receiver)
    }

    fn read(&mut self, uuid: u128) -> Result<Vec<u8>, String> {
        match uuid {
            uuids::COMMAND_RESULT => Ok(self.result.clone()),
            uuids::BATTERY_LEVEL => Ok(vec![87]),
            uuids::FIRMWARE_REVISION => Ok(b"FW_v3.64".to_vec()),
            uuids::MANUFACTURER_NAME => Ok(b"DI_LABS".to_vec()),
            uuids::FACES => Ok(vec![self.cube.0.lock().expect("lock").face]),
            uuids::HISTORY => Ok(self.history_answer.clone()),
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
            uuids::HISTORY if self.logged_in => match bytes {
                [0x01, 0xFF, 0xFF, 0xFF, 0xFF] => {
                    self.history_answer = state.history.last().map_or(vec![0; 17], |frame| frame.to_vec());
                }
                [0x02, rest @ ..] if rest.len() == 4 => {
                    let from = u32::from_be_bytes(rest.try_into().expect("four bytes"));
                    let frames: Vec<Vec<u8>> = state
                        .history
                        .iter()
                        .filter(|frame| u32::from_be_bytes(frame[0..4].try_into().expect("four")) >= from)
                        .map(|frame| frame.to_vec())
                        .chain(std::iter::once(vec![0; 20]))
                        .collect();
                    for feed in &state.history_feeds {
                        for frame in &frames {
                            let _unheard = feed.send(frame.clone());
                        }
                    }
                }
                _ => {}
            },
            uuids::COMMAND if !self.logged_in => {}
            uuids::COMMAND => match bytes.first() {
                Some(0x10) => self.result = vec![0x02, 0x01, 0x00, 0x05],
                Some(0x07) => {
                    self.result = vec![0x07];
                    self.result.extend_from_slice(&state.clock.to_be_bytes());
                }
                Some(0x08) if bytes.len() == 9 => {
                    state.clock = u64::from_be_bytes(bytes[1..9].try_into().expect("eight bytes"));
                    state.commands.push(bytes.to_vec());
                    self.result = vec![0x02];
                }
                Some(0x30) => {
                    state.pin = String::from_utf8_lossy(&bytes[1..]).into_owned();
                    state.commands.push(bytes.to_vec());
                    self.result = vec![0x02];
                }
                Some(0xFF) => {
                    state.wiping = Some(state.wipe_after);
                    state.commands.push(bytes.to_vec());
                    self.result = vec![0x02];
                }
                _ => state.commands.push(bytes.to_vec()),
            },
            _ => {}
        }
        Ok(())
    }

    fn is_connected(&mut self) -> bool {
        true
    }

    fn disconnect(&mut self) -> Result<(), String> {
        Ok(())
    }
}
