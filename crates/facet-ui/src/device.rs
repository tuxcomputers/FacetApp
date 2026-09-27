//! The Device tab: what the table holds about the paired cube, scanning for one, pairing with it, forgetting it,
//! and the five device settings.
//!
//! Radio work runs on background threads and comes back through a channel a timer drains. A background job
//! cannot write to the trace, which belongs to the UI thread, so it collects its log lines and they are
//! recorded, in order, when its outcome is acted on.
//!
//! **What is held here rather than read at the point of use**, each because it is not in the table: the live
//! link to the cube, the battery level it last reported, and the devices the current scan has heard. Opening the
//! tab rebuilds the list from a new scan, the battery is replaced by every read, and the link is dropped when
//! it goes.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use facet_core::app_settings::Value;
use facet_core::database;
use facet_core::debug_log::{Record, Tag, Trace, plain};
use facet_core::device::command::{self, CubeStatus};
use facet_core::device::rows::{self, DeviceInfo, DeviceSetting};
use facet_core::device::{info, login, scan, session, uuids};
use facet_core::port::{Advert, Link, Radio, RadioState, SecretLookup, SecretStore};
use rusqlite::Connection;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::notice::Notice;
use crate::timed;
use crate::{DeviceData, FoundDevice, SettingsWindow};

/// How often a held link is asked whether it is still up.
const LIVENESS_EVERY: Duration = Duration::from_secs(5);

/// Log lines a background job produced, recorded on the UI thread afterwards.
#[derive(Default)]
struct Lines(Mutex<Vec<(Tag, String, bool)>>);

impl Record for Lines {
    fn record(&self, tag: Tag, message: impl FnOnce() -> String) {
        if let Ok(mut lines) = self.0.lock() {
            lines.push((tag, message(), false));
        }
    }

    fn record_failure(&self, tag: Tag, message: impl FnOnce() -> String) {
        let message = message();
        match self.0.lock() {
            Ok(mut lines) => lines.push((tag, message, true)),
            Err(_) => eprintln!("facet: [{}] {message}", tag.word()),
        }
    }
}

impl Lines {
    fn take(&self) -> Logged {
        self.0.lock().map(|mut lines| std::mem::take(&mut *lines)).unwrap_or_default()
    }
}

/// Log lines a job produced, with whether each is a failure.
type Logged = Vec<(Tag, String, bool)>;

/// What a background job came back with.
enum Outcome {
    RadioState(RadioState),
    Heard(Advert),
    ScanEnded(Result<(), String>),
    Paired(Box<Paired>),
    PairingFailed { label: String, message: String },
    ReconnectFailed { message: String },
    Sent { setting: DeviceSetting, value: i64, result: Result<(), String> },
    LinkLost,
    Released,
}

/// A successful pairing: the cube is logged in, on `pin`, and the link is held.
struct Paired {
    handle: String,
    label: String,
    gap_name: Option<String>,
    info: DeviceInfo,
    battery: Option<u8>,
    status: Result<CubeStatus, String>,
    pin_stored: Result<(), String>,
}

/// The Device tab, attached to one Settings window.
pub struct Device {
    ui: slint::Weak<SettingsWindow>,
    database: PathBuf,
    log: Rc<Trace>,
    notice: Rc<Notice>,
    radio: Option<Arc<dyn Radio>>,
    pins: Arc<dyn SecretStore>,
    link: Arc<Mutex<Option<Box<dyn Link>>>>,
    battery: Cell<Option<u8>>,
    heard: RefCell<Vec<Advert>>,
    stop: Arc<AtomicBool>,
    is_scanning: Cell<bool>,
    is_reaching_for_cube: Cell<bool>,
    status: RefCell<String>,
    sender: Sender<(Outcome, Logged)>,
    receiver: Receiver<(Outcome, Logged)>,
    pump: slint::Timer,
    outstanding: Cell<usize>,
    liveness: slint::Timer,
    this: RefCell<Weak<Device>>,
}

impl Device {
    /// Wires the tab's callbacks on `ui`. `radio` is `None` in a build with no Bluetooth; `pins` keeps the PIN
    /// the app puts on the cube. A `connection.connected` left set by the last run is cleared here, since no link
    /// survives a relaunch.
    pub fn attach(
        ui: &SettingsWindow,
        database: PathBuf,
        log: Rc<Trace>,
        notice: Rc<Notice>,
        radio: Option<Arc<dyn Radio>>,
        pins: Arc<dyn SecretStore>,
    ) -> Rc<Device> {
        let (sender, receiver) = channel();
        let device = Rc::new(Device {
            ui: ui.as_weak(),
            database,
            log,
            notice,
            radio,
            pins,
            link: Arc::new(Mutex::new(None)),
            battery: Cell::new(None),
            heard: RefCell::new(Vec::new()),
            stop: Arc::new(AtomicBool::new(false)),
            is_scanning: Cell::new(false),
            is_reaching_for_cube: Cell::new(false),
            status: RefCell::new(String::new()),
            sender,
            receiver,
            pump: slint::Timer::default(),
            outstanding: Cell::new(0),
            liveness: slint::Timer::default(),
            this: RefCell::new(Weak::new()),
        });
        *device.this.borrow_mut() = Rc::downgrade(&device);

        if let Some(connection) = device.connect()
            && let Err(error) = rows::record_no_link_at_launch(&connection, &*device.log)
        {
            device.log.record_failure(Tag::Database, || {
                format!("Device: the connection row could not be cleared: {error}")
            });
        }

        let data = ui.global::<DeviceData>();
        let action = |run: fn(&Device)| {
            let weak = Rc::downgrade(&device);
            move || {
                if let Some(device) = weak.upgrade() {
                    run(&device);
                }
            }
        };
        data.on_scan_pressed(action(Device::scan_pressed));
        data.on_forget_pressed(action(Device::forget));
        data.on_reset_pressed(action(Device::reset_pressed));
        let weak = Rc::downgrade(&device);
        data.on_found_pressed(move |handle| {
            if let Some(device) = weak.upgrade() {
                device.pair(&handle);
            }
        });
        let weak = Rc::downgrade(&device);
        data.on_section_toggled(move |id, open| {
            if let Some(device) = weak.upgrade() {
                device.log.record(Tag::Settings, || {
                    format!("Device section {id} {}", if open { "opened" } else { "folded" })
                });
            }
        });
        let weak = Rc::downgrade(&device);
        data.on_pause_on_lock_toggled(move |enabled| {
            if let Some(device) = weak.upgrade() {
                device.store_setting(DeviceSetting::PauseOnLock, &Value::Flag(enabled));
            }
        });
        let weak = Rc::downgrade(&device);
        data.on_battery_warning_edited(move |percent| {
            if let Some(device) = weak.upgrade() {
                device.store_setting(DeviceSetting::BatteryWarning, &Value::Number(i64::from(percent)));
            }
        });
        let sends = |setting: DeviceSetting| {
            let weak = Rc::downgrade(&device);
            move |value: i32| {
                if let Some(device) = weak.upgrade() {
                    device.send_setting(setting, i64::from(value));
                }
            }
        };
        data.on_auto_pause_edited(sends(DeviceSetting::AutoPause));
        data.on_led_brightness_edited(sends(DeviceSetting::LedBrightness));
        data.on_led_blink_edited(sends(DeviceSetting::LedBlink));
        device
    }

    /// Reads the tab from the table, folds the sections as a fresh window has them, and asks the radio whether
    /// it can scan. Call when the Settings window opens.
    pub fn open(&self) {
        if let Some(ui) = self.ui.upgrade() {
            let data = ui.global::<DeviceData>();
            data.set_timeflip_expanded(true);
            data.set_settings_expanded(true);
            data.set_more_expanded(false);
            data.set_led_expanded(false);
        }
        self.draw();
        if let Some(radio) = self.radio.clone() {
            self.run(move |_| Outcome::RadioState(radio.state()));
        }
    }

    /// Draws the tab from the table, the held link, and the current scan.
    fn draw(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let Some(connection) = self.connect() else { return };
        let Some(pairing) = self.report(rows::pairing(&connection)) else { return };
        let Some(settings) = self.report(rows::settings(&connection)) else { return };
        let known = self.report(rows::known_names(&connection)).unwrap_or_default();
        let data = ui.global::<DeviceData>();
        let paired = pairing.is_cube_paired;
        let connected = paired && pairing.is_cube_connected;
        data.set_paired(paired);
        data.set_cube_connected(connected);
        data.set_device_name(info::shown(paired, pairing.name.as_deref()).into());
        data.set_connection(
            if !paired {
                "Manual mode, no device"
            } else if connected {
                "Connected"
            } else {
                "Disconnected"
            }
            .into(),
        );
        data.set_battery(
            info::shown(
                paired,
                self.battery.get().filter(|_| connected).map(|percent| format!("{percent}%")).as_deref(),
            )
            .into(),
        );
        data.set_manufacturer(info::shown(paired, pairing.info.manufacturer.as_deref()).into());
        data.set_model(info::shown(paired, pairing.info.model.as_deref()).into());
        data.set_hardware(info::shown(paired, pairing.info.hardware.as_deref()).into());
        data.set_firmware(info::shown(paired, pairing.info.firmware.as_deref()).into());
        data.set_pause_on_lock(settings.pause_on_lock);
        data.set_battery_warning_percent(settings.battery_warning_percent as i32);
        data.set_auto_pause_minutes(settings.auto_pause_minutes as i32);
        data.set_led_brightness_percent(settings.led_brightness_percent as i32);
        data.set_led_blink_seconds(settings.led_blink_seconds as i32);
        data.set_can_scan(self.radio.is_some());
        data.set_is_scanning(self.is_scanning.get());
        data.set_is_reaching_for_cube(self.is_reaching_for_cube.get());
        let all = data.get_scan_all();
        let found: Vec<FoundDevice> = self
            .heard
            .borrow()
            .iter()
            .filter(|advert| scan::is_eligible(advert, &known, all))
            .map(|advert| FoundDevice {
                handle: advert.handle.as_str().into(),
                label: scan::label(advert).into(),
            })
            .collect();
        let current = data.get_found();
        let changed = current.row_count() != found.len()
            || found.iter().enumerate().any(|(index, row)| current.row_data(index).as_ref() != Some(row));
        if changed {
            data.set_found(ModelRc::new(VecModel::from(found)));
        }
        let status = if self.radio.is_none() {
            "This build has no Bluetooth.".to_string()
        } else {
            self.status.borrow().clone()
        };
        data.set_scan_status(SharedString::from(status));
    }

    fn set_status(&self, status: impl Into<String>) {
        *self.status.borrow_mut() = status.into();
    }

    /// Runs `work` on a background thread with a [`Lines`] to log into, and acts on its outcome on the UI thread.
    fn run(&self, work: impl FnOnce(&Lines) -> Outcome + Send + 'static) {
        let sender = self.sender.clone();
        self.outstanding.set(self.outstanding.get() + 1);
        std::thread::spawn(move || {
            let lines = Lines::default();
            let outcome = work(&lines);
            // A closed channel means the window has gone, and the outcome has nobody to go to.
            if sender.send((outcome, lines.take())).is_err() {
                eprintln!("facet: a device outcome arrived after the Settings window had gone");
            }
        });
        self.start_pump();
    }

    fn start_pump(&self) {
        if !self.pump.running() {
            let weak = self.this.borrow().clone();
            self.pump.start(slint::TimerMode::Repeated, Duration::from_millis(200), move || {
                if let Some(device) = weak.upgrade() {
                    device.drain();
                }
            });
        }
    }

    fn drain(&self) {
        while let Ok((outcome, lines)) = self.receiver.try_recv() {
            for (tag, message, failure) in lines {
                if failure {
                    self.log.record_failure(tag, || message);
                } else {
                    self.log.record(tag, || message);
                }
            }
            // A scan reports every device it hears before it ends, and only its end is a finished job.
            if !matches!(outcome, Outcome::Heard(_)) {
                self.outstanding.set(self.outstanding.get().saturating_sub(1));
            }
            self.finish(outcome);
        }
        if self.outstanding.get() == 0 {
            self.pump.stop();
        }
    }

    fn finish(&self, outcome: Outcome) {
        match outcome {
            Outcome::RadioState(state) => {
                let status = match state {
                    RadioState::Ready => String::new(),
                    RadioState::Off => "Bluetooth is off.".to_string(),
                    RadioState::Unauthorised => {
                        "Facet has not been allowed to use Bluetooth, or it is not ready yet.".to_string()
                    }
                    RadioState::Unavailable(reason) => format!("Bluetooth is not available: {reason}"),
                };
                if !status.is_empty() {
                    self.log.record(Tag::Radio, || status.clone());
                }
                if !self.is_scanning.get() && !self.is_reaching_for_cube.get() {
                    self.set_status(status);
                }
            }
            Outcome::Heard(advert) => {
                let mut heard = self.heard.borrow_mut();
                match heard.iter_mut().find(|known| known.handle == advert.handle) {
                    Some(known) => *known = advert,
                    None => heard.push(advert),
                }
            }
            Outcome::ScanEnded(result) => {
                self.is_scanning.set(false);
                match result {
                    Ok(()) => {
                        let count = self.eligible_count();
                        self.log.record(Tag::Radio, || format!("The scan ended, {count} device(s) found"));
                        self.set_status(match count {
                            0 => "No devices found.".to_string(),
                            count => format!("Found {count} device(s)."),
                        });
                    }
                    Err(reason) => {
                        self.log.record_failure(Tag::Radio, || format!("The scan failed: {reason}"));
                        self.set_status(format!("The scan failed: {reason}"));
                    }
                }
            }
            Outcome::Paired(paired) => self.paired(*paired),
            Outcome::PairingFailed { label, message } => {
                self.is_reaching_for_cube.set(false);
                self.log.record(Tag::Pair, || format!("Pairing with {} did not complete", plain(&label)));
                self.set_status(message);
            }
            Outcome::ReconnectFailed { message } => {
                self.is_reaching_for_cube.set(false);
                self.log.record(Tag::Pair, || {
                    format!("The paired cube was not reconnected: {}", plain(&message))
                });
                self.set_status(message);
            }
            Outcome::Sent { setting, value, result } => self.sent(setting, value, result),
            Outcome::LinkLost => {
                self.liveness.stop();
                self.battery.set(None);
                if let Some(connection) = self.connect() {
                    self.report(rows::record_connection_lost(&connection, &*self.log));
                }
            }
            Outcome::Released => {}
        }
        self.draw();
    }

    fn eligible_count(&self) -> usize {
        let known = self
            .connect()
            .and_then(|connection| self.report(rows::known_names(&connection)))
            .unwrap_or_default();
        let all = self.ui.upgrade().is_some_and(|ui| ui.global::<DeviceData>().get_scan_all());
        self.heard.borrow().iter().filter(|advert| scan::is_eligible(advert, &known, all)).count()
    }

    /// Starts a scan, or stops the one that is running.
    fn scan_pressed(&self) {
        let Some(radio) = self.radio.clone() else { return };
        if self.is_scanning.get() {
            self.log.record(Tag::Click, || "Button clicked: Stop Scan".to_string());
            self.stop.store(true, Ordering::Relaxed);
            return;
        }
        let all = self.ui.upgrade().is_some_and(|ui| ui.global::<DeviceData>().get_scan_all());
        self.log.record(Tag::Click, || format!("Button clicked: Scan for Devices (allDevices={all})"));
        self.heard.borrow_mut().clear();
        self.stop.store(false, Ordering::Relaxed);
        self.is_scanning.set(true);
        self.set_status("Looking for devices...");
        self.draw();
        let stop = Arc::clone(&self.stop);
        let sender = self.sender.clone();
        self.run(move |lines| {
            lines.record(Tag::Radio, || "Scanning, unfiltered".to_string());
            let result = radio.scan(Duration::from_secs(scan::SCAN_SECONDS), &stop, &mut |advert| {
                if sender.send((Outcome::Heard(advert.clone()), Vec::new())).is_err() {
                    stop.store(true, Ordering::Relaxed);
                }
            });
            Outcome::ScanEnded(result)
        });
    }

    /// Pairs with the device `handle` names: presents the vendor PIN and then any stored one, each on its own
    /// connection; a cube on the vendor PIN is moved onto a PIN of the app's own, which is stored. On success
    /// the cube's Device Information, battery and status are read and the link is held.
    fn pair(&self, handle: &str) {
        let Some(radio) = self.radio.clone() else { return };
        if self.is_reaching_for_cube.get() {
            return;
        }
        let label = self
            .heard
            .borrow()
            .iter()
            .find(|advert| advert.handle == handle)
            .map(scan::label)
            .unwrap_or_else(|| "the device".to_string());
        self.log.record(Tag::Click, || format!("Device clicked: {}", plain(&label)));
        self.stop.store(true, Ordering::Relaxed);
        self.is_reaching_for_cube.set(true);
        self.set_status(format!("Connecting to {label}..."));
        self.draw();
        let handle = handle.to_string();
        let pins = Arc::clone(&self.pins);
        let held = Arc::clone(&self.link);
        self.run(move |lines| {
            let stored = stored_pin(&pins, lines);
            let candidates = login::pairing_candidates(stored.as_deref());
            let new_pin = match login::new_pin() {
                Ok(pin) => pin,
                Err(reason) => {
                    return Outcome::PairingFailed {
                        label,
                        message: format!("No new PIN could be made: {reason}"),
                    };
                }
            };
            match session::log_in(&*radio, &handle, &candidates, Some(&new_pin), lines) {
                session::LoginOutcome::LoggedIn { link, pin, rotated } => {
                    settle(link, &pin, rotated, stored.as_deref(), &pins, &held, lines, handle, label)
                }
                outcome => Outcome::PairingFailed { message: outcome.describe(&label), label },
            }
        });
    }

    /// Finds the paired cube again and logs in to it: scans for TimeFlips and names this app gave the cube,
    /// tries the handle last recorded first, and presents the stored PIN and then the vendor PIN to each, on
    /// connections of their own. The one that accepts is this app's cube. Does nothing when no cube is paired or
    /// there is no radio. Call once at launch.
    pub fn reconnect(&self) {
        let Some(radio) = self.radio.clone() else { return };
        let Some(connection) = self.connect() else { return };
        let Some(pairing) = self.report(rows::pairing(&connection)) else { return };
        if !pairing.is_cube_paired || self.is_reaching_for_cube.get() {
            return;
        }
        let known = self.report(rows::known_names(&connection)).unwrap_or_default();
        let recorded = pairing.handle.clone();
        self.is_reaching_for_cube.set(true);
        self.set_status("Looking for the paired cube...");
        self.draw();
        let pins = Arc::clone(&self.pins);
        let held = Arc::clone(&self.link);
        self.run(move |lines| {
            lines.record(Tag::Pair, || "Looking for the paired cube".to_string());
            let stored = stored_pin(&pins, lines);
            let stop = AtomicBool::new(false);
            let mut found: Vec<Advert> = Vec::new();
            let scanned = radio.scan(Duration::from_secs(scan::SCAN_SECONDS), &stop, &mut |advert| {
                if scan::is_eligible(advert, &known, false)
                    && !found.iter().any(|seen| seen.handle == advert.handle)
                {
                    found.push(advert.clone());
                    if recorded.as_deref() == Some(advert.handle.as_str()) {
                        stop.store(true, Ordering::Relaxed);
                    }
                }
            });
            if let Err(reason) = scanned {
                return Outcome::ReconnectFailed { message: format!("The scan failed: {reason}") };
            }
            found.sort_by_key(|advert| recorded.as_deref() != Some(advert.handle.as_str()));
            let candidates = login::reconnect_candidates(stored.as_deref());
            let new_pin = match login::new_pin() {
                Ok(pin) => pin,
                Err(reason) => {
                    return Outcome::ReconnectFailed {
                        message: format!("No new PIN could be made: {reason }"),
                    };
                }
            };
            for advert in found {
                let label = scan::label(&advert);
                match session::log_in(&*radio, &advert.handle, &candidates, Some(&new_pin), lines) {
                    session::LoginOutcome::LoggedIn { link, pin, rotated } => {
                        return settle(
                            link,
                            &pin,
                            rotated,
                            stored.as_deref(),
                            &pins,
                            &held,
                            lines,
                            advert.handle,
                            label,
                        );
                    }
                    session::LoginOutcome::NewPinRefused => {
                        return Outcome::ReconnectFailed {
                            message: session::LoginOutcome::NewPinRefused.describe(&label),
                        };
                    }
                    outcome => {
                        let why = outcome.describe(&label);
                        lines.record(Tag::Pair, || format!("Not the paired cube: {}", plain(&why)));
                    }
                }
            }
            Outcome::ReconnectFailed { message: "The paired cube was not found.".into() }
        });
    }

    fn paired(&self, paired: Paired) {
        self.is_reaching_for_cube.set(false);
        self.heard.borrow_mut().clear();
        self.battery.set(paired.battery);
        if let Err(reason) = &paired.pin_stored {
            self.log
                .record_failure(Tag::Pin, || format!("The PIN on the cube could not be stored: {reason}"));
            self.notice.tell(
                "The cube's PIN was not saved",
                &format!(
                    "Facet paired with the cube but could not save the PIN it set on it ({reason}). Until it is saved, the \
                     next connection will be refused; taking the cube's batteries out puts it back on the factory PIN."
                ),
            );
        }
        if let Err(reason) = &paired.status {
            self.log
                .record(Tag::Command, || format!("The cube's status could not be read: {}", plain(reason)));
        }
        let Some(connection) = self.connect() else { return };
        let recorded = self.report(rows::record_login(
            &connection,
            &paired.handle,
            paired.gap_name.as_deref(),
            &paired.info,
            &*self.log,
        ));
        if recorded != Some(true) {
            self.notice.tell(
                "The pairing was not saved",
                "The cube accepted Facet's PIN, but the database would not record the pairing.",
            );
        }
        self.set_status(format!("Connected to {}.", paired.label));
        self.start_liveness();
    }

    /// Asks the held link, every [`LIVENESS_EVERY`], whether it is still up.
    fn start_liveness(&self) {
        let weak = self.this.borrow().clone();
        self.liveness.start(slint::TimerMode::Repeated, LIVENESS_EVERY, move || {
            let Some(device) = weak.upgrade() else { return };
            let held = Arc::clone(&device.link);
            device.run(move |lines| {
                let Ok(mut slot) = held.lock() else { return Outcome::Released };
                let is_up = match slot.as_mut() {
                    Some(link) => link.is_connected(),
                    None => return Outcome::Released,
                };
                if is_up {
                    return Outcome::Released;
                }
                lines.record(Tag::Radio, || "The cube is no longer connected".to_string());
                *slot = None;
                Outcome::LinkLost
            });
        });
    }

    /// Forgets the paired cube and drops the link. The PIN is kept, since it is what identifies this app's cube.
    fn forget(&self) {
        self.log.record(Tag::Click, || "Button clicked: Forget Device".to_string());
        self.liveness.stop();
        self.battery.set(None);
        let held = Arc::clone(&self.link);
        self.run(move |lines| {
            if let Ok(mut slot) = held.lock()
                && let Some(mut link) = slot.take()
            {
                session::disconnect(&mut *link, lines);
            }
            Outcome::Released
        });
        if let Some(connection) = self.connect() {
            self.report(rows::record_forget(&connection, &*self.log));
        }
        self.set_status("");
        self.draw();
    }

    /// Closes the link to the cube and records the quit. Blocks until the cube is disconnected; call once, as the
    /// app quits.
    pub fn quit(&self) {
        self.liveness.stop();
        let link = match self.link.lock() {
            Ok(mut slot) => slot.take(),
            Err(_) => {
                self.log.record_failure(Tag::Quit, || {
                    "Quit: the link to the cube could not be reached".to_string()
                });
                None
            }
        };
        let Some(mut link) = link else { return };
        session::disconnect(&mut *link, &*self.log);
        if let Some(connection) = self.connect() {
            self.report(rows::record_quit(&connection, &*self.log));
        }
    }

    fn reset_pressed(&self) {
        self.log.record(Tag::Click, || "Button clicked: Reset Device".to_string());
        self.notice.tell(
            "Resetting the cube is not built yet",
            "Facet cannot factory reset a cube yet. Taking its batteries out puts it back on the factory PIN, which is \
             the part of a reset most often wanted.",
        );
    }

    /// Stores a setting no command carries, with a read-back.
    fn store_setting(&self, setting: DeviceSetting, value: &Value) {
        let stored = self
            .connect()
            .and_then(|connection| self.report(rows::write_setting(&connection, setting, value, &*self.log)))
            .unwrap_or(false);
        if !stored {
            self.refused();
        }
        self.draw();
    }

    /// Sends a setting to the connected cube and stores it once the cube has it. Auto-pause is confirmed by
    /// reading it back with `0x10`; LED brightness and blink interval have no read-back, so the cube's
    /// acknowledgement is all there is.
    fn send_setting(&self, setting: DeviceSetting, value: i64) {
        let held = Arc::clone(&self.link);
        self.run(move |lines| {
            let result = (|| {
                let mut slot = held.lock().map_err(|_| "the link is poisoned".to_string())?;
                let link = slot.as_mut().ok_or_else(|| "there is no cube connected".to_string())?;
                let bytes = match setting {
                    DeviceSetting::AutoPause => command::set_auto_pause(value),
                    DeviceSetting::LedBrightness => command::set_led_brightness(value),
                    DeviceSetting::LedBlink => command::set_led_blink(value),
                    DeviceSetting::PauseOnLock | DeviceSetting::BatteryWarning => return Ok(()),
                };
                lines.record(Tag::Command, || format!("Sending {}", command::hex(&bytes)));
                link.write(uuids::COMMAND, &bytes)?;
                if setting == DeviceSetting::AutoPause {
                    let status = session::status(&mut **link, lines)?;
                    if i64::from(status.auto_pause_minutes) != value {
                        return Err(format!("the cube reads back {}m", status.auto_pause_minutes));
                    }
                    lines.record(Tag::Command, || "The cube confirms it took".to_string());
                } else {
                    lines.record(Tag::Command, || {
                        "The cube took the write; nothing can read this command back".to_string()
                    });
                }
                Ok(())
            })();
            Outcome::Sent { setting, value, result }
        });
    }

    fn sent(&self, setting: DeviceSetting, value: i64, result: Result<(), String>) {
        match result {
            Ok(()) => self.store_setting(setting, &Value::Number(value)),
            Err(reason) => {
                self.log
                    .record(Tag::Command, || format!("The cube did not take {value}: {}", plain(&reason)));
                self.notice.tell(
                    "The TimeFlip did not accept that",
                    &format!("The setting has gone back ({reason})."),
                );
            }
        }
    }

    fn refused(&self) {
        self.notice.tell(
            "That setting was not saved",
            "The database would not take the new value, so the setting is unchanged and the row has gone back to what \
             is stored.",
        );
    }

    fn connect(&self) -> Option<Connection> {
        match database::connect(&self.database) {
            Ok(connection) => Some(connection),
            Err(error) => {
                self.log.record_failure(Tag::Database, || {
                    format!("Device: the database would not open: {error}")
                });
                None
            }
        }
    }

    fn report<T>(&self, result: Result<T, rusqlite::Error>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.log.record_failure(Tag::Database, || format!("Device: a database call failed: {error}"));
                None
            }
        }
    }
}

/// The PIN the store holds, or `None` when it holds none or cannot be read, which is said.
fn stored_pin(pins: &Arc<dyn SecretStore>, lines: &Lines) -> Option<String> {
    match timed::look_up(pins) {
        SecretLookup::Found(pin) => Some(pin),
        SecretLookup::Missing => None,
        SecretLookup::Unavailable(reason) => {
            lines.record_failure(Tag::Pin, || format!("The stored PIN could not be read: {reason}"));
            None
        }
    }
}

/// After a login to `handle` on `pin`: stores the PIN when it is new, reads Device Information, battery and
/// status, and holds the link.
#[allow(clippy::too_many_arguments)]
fn settle(
    mut link: Box<dyn Link>,
    pin: &str,
    rotated: bool,
    stored: Option<&str>,
    pins: &Arc<dyn SecretStore>,
    held: &Arc<Mutex<Option<Box<dyn Link>>>>,
    lines: &Lines,
    handle: String,
    label: String,
) -> Outcome {
    let pin_stored = if rotated || stored != Some(pin) {
        match timed::store(pins, pin) {
            Ok(true) => Ok(()),
            Ok(false) => Err("it did not read back".to_string()),
            Err(reason) => Err(reason),
        }
    } else {
        Ok(())
    };
    let gap_name = link.gap_name();
    let info = session::device_info(&mut *link, lines);
    let battery = session::battery(&mut *link, lines);
    let status = session::status(&mut *link, lines);
    match held.lock() {
        Ok(mut slot) => *slot = Some(link),
        Err(_) => {
            return Outcome::PairingFailed { label, message: "The link could not be kept.".to_string() };
        }
    }
    Outcome::Paired(Box::new(Paired { handle, label, gap_name, info, battery, status, pin_stored }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::platform::software_renderer::MinimalSoftwareWindow;
    use slint::platform::{Platform, WindowAdapter};

    struct Headless(Rc<MinimalSoftwareWindow>);
    impl Platform for Headless {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
            Ok(self.0.clone())
        }
    }

    /// A radio that hears a TimeFlip on `pin` and a monitor, and connects to the TimeFlip.
    struct FakeRadio(Arc<Mutex<String>>, Arc<Mutex<Vec<Vec<u8>>>>);
    impl Radio for FakeRadio {
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
                rssi: None,
            });
            heard(&Advert {
                handle: "tv".into(),
                name: Some("Smart Monitor".into()),
                services: vec![],
                rssi: None,
            });
            Ok(())
        }
        fn connect(&self, _handle: &str, _timeout: Duration) -> Result<Box<dyn Link>, String> {
            Ok(Box::new(FakeLink {
                pin: Arc::clone(&self.0),
                sent: Arc::clone(&self.1),
                ok: false,
                result: vec![],
                minutes: 0,
            }))
        }
    }

    struct FakeLink {
        pin: Arc<Mutex<String>>,
        sent: Arc<Mutex<Vec<Vec<u8>>>>,
        ok: bool,
        result: Vec<u8>,
        minutes: u16,
    }
    impl Link for FakeLink {
        fn gap_name(&self) -> Option<String> {
            Some("TimeFlip v2.0".into())
        }
        fn has_characteristic(&self, _uuid: u128) -> bool {
            true
        }
        fn read(&mut self, uuid: u128) -> Result<Vec<u8>, String> {
            Ok(match uuid {
                uuids::COMMAND_RESULT => self.result.clone(),
                uuids::BATTERY_LEVEL => vec![87],
                uuids::FIRMWARE_REVISION => b"FW_v3.64".to_vec(),
                _ => Vec::new(),
            })
        }
        fn write(&mut self, uuid: u128, bytes: &[u8]) -> Result<(), String> {
            let mut pin = self.pin.lock().expect("lock");
            match uuid {
                uuids::PASSWORD => {
                    self.ok = bytes == pin.as_bytes();
                    self.result = vec![if self.ok { 0x02 } else { 0x01 }];
                }
                uuids::COMMAND if self.ok => {
                    match bytes {
                        [0x30, rest @ ..] => *pin = String::from_utf8_lossy(rest).into_owned(),
                        [0x05, high, low] => self.minutes = u16::from_be_bytes([*high, *low]),
                        _ => {}
                    }
                    if bytes != [0x10] {
                        self.sent.lock().expect("lock").push(bytes.to_vec());
                    }
                    let [high, low] = self.minutes.to_be_bytes();
                    self.result = if bytes == [0x10] { vec![0x02, 0x02, high, low] } else { vec![0x02] };
                }
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

    struct MemoryStore(Mutex<Option<String>>);
    impl SecretStore for MemoryStore {
        fn store(&self, secret: &str) -> Result<bool, String> {
            *self.0.lock().expect("lock") = Some(secret.to_string());
            Ok(true)
        }
        fn look_up(&self) -> SecretLookup {
            match self.0.lock().expect("lock").clone() {
                Some(secret) => SecretLookup::Found(secret),
                None => SecretLookup::Missing,
            }
        }
        fn clear(&self) -> Result<(), String> {
            *self.0.lock().expect("lock") = None;
            Ok(())
        }
    }

    fn settle(device: &Device) {
        for _ in 0..200 {
            std::thread::sleep(Duration::from_millis(25));
            device.drain();
            if device.outstanding.get() == 0 {
                return;
            }
        }
        panic!("the device job did not finish");
    }

    #[test]
    fn scanning_pairing_sending_and_forgetting() {
        slint::platform::set_platform(Box::new(Headless(MinimalSoftwareWindow::new(Default::default()))))
            .expect("the headless platform should install");
        let path = std::env::temp_dir().join(format!("facet-ui-device-{}.sqlite", std::process::id()));
        if path.exists() {
            std::fs::remove_file(&path).expect("a stale test database should be removable");
        }
        let connection = database::open(&path, database::APPDATA_DDL).expect("the app DDL should apply");
        let ui = SettingsWindow::new().expect("the window should build");
        let notice = Notice::attach(&ui);
        let data = ui.global::<DeviceData>();

        let without = Device::attach(
            &ui,
            path.clone(),
            Rc::new(Trace::none()),
            Rc::clone(&notice),
            None,
            Arc::new(MemoryStore(Mutex::new(None))),
        );
        without.open();
        assert_eq!(data.get_scan_status(), "This build has no Bluetooth.");
        assert!(!data.get_can_scan());
        drop(without);

        // A cube on the vendor PIN pairs, is moved onto a PIN of the app's own, and that PIN is stored.
        let pin = Arc::new(Mutex::new("000000".to_string()));
        let sent = Arc::new(Mutex::new(Vec::new()));
        let store = Arc::new(MemoryStore(Mutex::new(None)));
        let radio: Arc<dyn Radio> = Arc::new(FakeRadio(Arc::clone(&pin), Arc::clone(&sent)));
        let device = Device::attach(
            &ui,
            path.clone(),
            Rc::new(Trace::none()),
            Rc::clone(&notice),
            Some(radio),
            Arc::clone(&store) as Arc<dyn SecretStore>,
        );
        device.open();
        settle(&device);
        assert_eq!(data.get_connection(), "Manual mode, no device");
        assert!(!data.get_cube_connected());

        device.scan_pressed();
        settle(&device);
        assert_eq!(data.get_found().row_count(), 1);
        assert_eq!(data.get_scan_status(), "Found 1 device(s).");
        data.set_scan_all(true);
        device.draw();
        assert_eq!(data.get_found().row_count(), 2);
        data.set_scan_all(false);

        device.pair("cube");
        settle(&device);
        assert_eq!(notice.title(), "");
        let rotated = pin.lock().expect("lock").clone();
        assert_ne!(rotated, "000000");
        assert_eq!(store.look_up(), SecretLookup::Found(rotated));
        let held = rows::pairing(&connection).expect("read");
        assert!(held.is_cube_paired && held.is_cube_connected);
        assert_eq!(data.get_connection(), "Connected");
        assert_eq!(data.get_battery(), "87%");
        assert_eq!(data.get_firmware(), "FW_v3.64");
        assert!(data.get_cube_connected());

        // Auto-pause goes to the cube, is read back, and is stored.
        device.send_setting(DeviceSetting::AutoPause, 5);
        settle(&device);
        assert_eq!(rows::settings(&connection).expect("read").auto_pause_minutes, 5);
        assert_eq!(sent.lock().expect("lock").last(), Some(&vec![0x05, 0x00, 0x05]));
        device.send_setting(DeviceSetting::LedBrightness, 70);
        settle(&device);
        assert_eq!(rows::settings(&connection).expect("read").led_brightness_percent, 70);

        // A relaunch: the new controller finds the table saying connected, clears it, and reconnects on the
        // stored PIN, rotating nothing.
        drop(device);
        let before = pin.lock().expect("lock").clone();
        let radio: Arc<dyn Radio> = Arc::new(FakeRadio(Arc::clone(&pin), Arc::clone(&sent)));
        let device = Device::attach(
            &ui,
            path.clone(),
            Rc::new(Trace::none()),
            Rc::clone(&notice),
            Some(radio),
            Arc::clone(&store) as Arc<dyn SecretStore>,
        );
        assert!(!rows::pairing(&connection).expect("read").is_cube_connected);
        device.open();
        settle(&device);
        assert_eq!(data.get_connection(), "Disconnected");
        device.reconnect();
        settle(&device);
        assert_eq!(data.get_connection(), "Connected");
        assert_eq!(pin.lock().expect("lock").as_str(), before);

        device.forget();
        settle(&device);
        let held = rows::pairing(&connection).expect("read");
        assert!(!held.is_cube_paired && !held.is_cube_connected);
        assert_eq!(data.get_connection(), "Manual mode, no device");
        drop(device);

        // A cube on some other PIN refuses both, and nothing is stored or paired.
        *pin.lock().expect("lock") = "654321".to_string();
        let empty = Arc::new(MemoryStore(Mutex::new(None)));
        let radio: Arc<dyn Radio> = Arc::new(FakeRadio(Arc::clone(&pin), Arc::clone(&sent)));
        let device = Device::attach(
            &ui,
            path.clone(),
            Rc::new(Trace::none()),
            Rc::clone(&notice),
            Some(radio),
            Arc::clone(&empty) as Arc<dyn SecretStore>,
        );
        device.open();
        device.scan_pressed();
        settle(&device);
        device.pair("cube");
        settle(&device);
        assert!(data.get_scan_status().contains("refused the PIN"));
        assert_eq!(empty.look_up(), SecretLookup::Missing);
        assert!(!rows::pairing(&connection).expect("read").is_cube_paired);
        assert_eq!(pin.lock().expect("lock").as_str(), "654321");

        std::fs::remove_file(&path).expect("the test database should be removable");
    }
}
