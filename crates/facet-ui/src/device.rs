//! The Device tab: what the table holds about the paired cube, scanning for one, pairing with it, renaming it,
//! forgetting it, factory resetting it, and the five device settings.
//!
//! Radio work runs on background threads and comes back through a channel a timer drains. A background job
//! cannot write to the trace, which belongs to the UI thread, so it collects its log lines and they are
//! recorded, in order, when its outcome is acted on.
//!
//! **What is held here rather than read at the point of use**, each because it is not in the table: the live
//! link to the cube, the battery level it last reported, whether that level has the low-battery warning on, and
//! the devices the current scan has heard. Opening the tab rebuilds the list from a new scan, the battery is
//! replaced by every reading the cube sends, the warning is decided again from the table's warning level whenever
//! the charge or that level changes, and the link is dropped when it goes.

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
use facet_core::device::forced_pause::{self, Decision, PauseClaim, Resting};
use facet_core::device::name::{self, NameDecision, NameProblem};
use facet_core::device::pin_source::{self, AfterLogin, AtRead};
use facet_core::device::rows::{self, DeviceInfo, DeviceSetting};
use facet_core::device::session::{Fetched, ResetOutcome};
use facet_core::device::system_state::{self, CubeHardwareState, CubeSyncState};
use facet_core::device::trace::TracedRadio;
use facet_core::device::{colour, face, history, info, login, scan, session, uuids};
use facet_core::port::{Advert, Link, Radio, RadioState, SecretLookup, SecretStore, Zone};
use facet_core::{app_settings, cube_history};
use rusqlite::Connection;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::notice::Notice;
use crate::timed;
use crate::{DeviceData, FoundDevice, SettingsWindow};

/// How often a held link is asked whether it is still up.
const LIVENESS_EVERY: Duration = Duration::from_secs(5);
/// The longest a pairing waits for a scan it stopped to end before connecting anyway.
const SCAN_END_WAIT: Duration = Duration::from_secs(5);
/// The first wait before looking for a cube that dropped out, doubled after each miss up to [`REACH_AGAIN_AT_MOST`].
const REACH_AGAIN_FIRST: Duration = Duration::from_secs(2);
const REACH_AGAIN_AT_MOST: Duration = Duration::from_secs(30);
/// How long each half of the low-battery blink lasts.
const BLINK_EVERY: Duration = Duration::from_millis(500);
/// How long a stepper's value has to stand still before it is sent to the cube.
pub const EDIT_QUIET_FOR: Duration = Duration::from_millis(500);
/// How long a quit waits for the cube to be paused, locked and let go of before quitting without it.
pub const QUIT_DEADLINE: Duration = Duration::from_secs(5);

/// One stepper's edits on their way to the cube.
#[derive(Default)]
struct EditedSetting {
    /// Restarted by every edit, and sends the newest value when it runs out.
    timer: slint::Timer,
    /// The newest value edited and not yet sent.
    unsent: Cell<Option<i64>>,
    /// Whether a send of this setting is out with the cube.
    is_sending: Cell<bool>,
}

impl EditedSetting {
    /// Whether the person's value is still on its way to the table, and so not the table's to overwrite.
    fn is_open(&self) -> bool {
        self.unsent.get().is_some() || self.is_sending.get()
    }
}

/// The three settings whose edits wait for the value to stop moving: the ones that go to the cube.
#[derive(Default)]
struct EditedSettings {
    auto_pause: EditedSetting,
    led_brightness: EditedSetting,
    led_blink: EditedSetting,
}

impl EditedSettings {
    /// `None` for a setting no stepper edits.
    fn of(&self, setting: DeviceSetting) -> Option<&EditedSetting> {
        match setting {
            DeviceSetting::AutoPause => Some(&self.auto_pause),
            DeviceSetting::LedBrightness => Some(&self.led_brightness),
            DeviceSetting::LedBlink => Some(&self.led_blink),
            DeviceSetting::PauseOnLock | DeviceSetting::BatteryWarning => None,
        }
    }
}

/// Where a background job logs: each line goes to the UI thread as it happens, through the same channel as the
/// outcomes, so the trace keeps the order things happened in.
#[derive(Clone)]
struct Lines(Sender<Outcome>);

impl Record for Lines {
    fn record(&self, tag: Tag, message: impl FnOnce() -> String) {
        let message = message();
        if let Err(error) = self.0.send(Outcome::Log(tag, message, false)) {
            eprintln!("facet: a device log line had nowhere to go: {:?}", describe_lost(&error.0));
        }
    }

    fn record_failure(&self, tag: Tag, message: impl FnOnce() -> String) {
        let message = message();
        if let Err(error) = self.0.send(Outcome::Log(tag, message, true)) {
            eprintln!("facet: a device failure had nowhere to go: {:?}", describe_lost(&error.0));
        }
    }
}

/// What a log line that could not be delivered said, for stderr.
fn describe_lost(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Log(tag, message, _) => format!("[{}] {message}", tag.word()),
        _ => "an outcome".to_string(),
    }
}

/// What a background job came back with.
enum Outcome {
    /// A log line, recorded on the UI thread in the order it was made.
    Log(Tag, String, bool),
    RadioState(RadioState),
    Heard(Advert),
    ScanEnded(Result<(), String>),
    Paired(Box<Paired>),
    PairingFailed {
        label: String,
        message: String,
    },
    ReconnectFailed {
        message: String,
    },
    Sent {
        setting: DeviceSetting,
        value: i64,
        result: Result<(), String>,
    },
    /// A charge the cube sent while the link is up. Not a finished job.
    Battery(u8),
    /// The face the cube says is up, sent while the link is up. Not a finished job.
    Face(u8),
    /// A system state value the cube sent while the link is up. Not a finished job.
    SystemState(Vec<u8>),
    /// The settings sync after a login ended: the status the cube read back when auto-pause was sent, and its system
    /// state.
    Synced {
        status: Option<CubeStatus>,
        system_state: Option<Vec<u8>>,
    },
    HistoryFetched {
        reason: String,
        result: Result<Fetched, String>,
    },
    /// A pause or lock exchange ended, with the last status the cube read back, and why it was asked for.
    CubeCommanded {
        reason: String,
        status: Result<CubeStatus, String>,
    },
    ResetEnded {
        label: String,
        outcome: ResetOutcome,
    },
    Renamed {
        name: String,
        result: Result<(), String>,
    },
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
    face: Option<u8>,
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
    /// Where the PIN goes when `pins` will not take it, and is read from when `pins` will not answer.
    pin_fallback: RefCell<Option<Arc<dyn SecretStore>>>,
    /// Names the zone each cube event is filed under when it is first recorded.
    zone: Arc<dyn Zone>,
    edited: EditedSettings,
    link: Arc<Mutex<Option<Box<dyn Link>>>>,
    /// The history characteristic's notifications for the held link, subscribed once at login.
    history_feed: Arc<Mutex<Option<Receiver<Vec<u8>>>>>,
    /// The face the cube last said was up, while the link is held.
    cube_face: Cell<Option<u8>>,
    /// The cube's lock, pause and auto-pause as its last `0x10` answer said, while the link is held. Held rather than
    /// read because the lock is in no table: it is the cube's, and every pause or lock exchange reads it again.
    cube_status: Cell<Option<CubeStatus>>,
    on_cube_not_found: RefCell<Vec<Box<dyn Fn()>>>,
    on_blink: RefCell<Vec<Box<dyn Fn()>>>,
    /// While the cube that dropped out is being looked for again: the wait before the next look. `None` otherwise,
    /// and while it is `Some` a failed look tries again rather than offering Rescan.
    reach_again_after: Cell<Option<Duration>>,
    reach_again: slint::Timer,
    /// When all twelve colours last went because the cube asked, so a cube that keeps asking is answered at most
    /// every 30 seconds.
    colours_asked_at: Cell<Option<std::time::Instant>>,
    /// Whether the cube's wish for task parameters has been said on this link; it is said once.
    has_said_task_parameters: Cell<bool>,
    /// When the settings last went, so the cube stepping through its sync codes as they land is not answered twice.
    settings_sent_at: Cell<Option<std::time::Instant>>,
    /// Why the app last paused the cube itself, while that pause stands.
    pause_claim: Cell<Option<PauseClaim>>,
    /// A pause or resume the app decided on itself is on its way to the cube, or its history not yet filed.
    is_forced_pause_sending: Cell<bool>,
    is_history_fetching: Cell<bool>,
    /// Why another fetch was asked for while one ran; it runs once that one ends.
    is_another_fetch_wanted: RefCell<Option<String>>,
    history_timer: slint::Timer,
    on_history_changed: RefCell<Vec<Box<dyn Fn()>>>,
    battery: Cell<Option<u8>>,
    /// A charge the cube sent before its login was recorded, applied once it is.
    early_charge: Cell<Option<u8>>,
    heard: RefCell<Vec<Advert>>,
    stop: Arc<AtomicBool>,
    /// True from a scan job starting until the radio's scan has returned, which is later than `is_scanning` going
    /// false only by the time the outcome takes to reach the UI thread.
    is_radio_scanning: Arc<AtomicBool>,
    is_scanning: Cell<bool>,
    is_reaching_for_cube: Cell<bool>,
    is_factory_reset_running: Cell<bool>,
    is_battery_low: Cell<bool>,
    is_blink_on: Cell<bool>,
    blink: slint::Timer,
    status: RefCell<String>,
    sender: Sender<Outcome>,
    receiver: Receiver<Outcome>,
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
        zone: Arc<dyn Zone>,
    ) -> Rc<Device> {
        let (sender, receiver) = channel();
        // Every connection the radio makes is traced, into the same channel the jobs log through.
        let radio =
            radio.map(|radio| Arc::new(TracedRadio::new(radio, Lines(sender.clone()))) as Arc<dyn Radio>);
        let device = Rc::new(Device {
            ui: ui.as_weak(),
            database,
            log,
            notice,
            radio,
            pins,
            pin_fallback: RefCell::new(None),
            zone,
            edited: EditedSettings::default(),
            link: Arc::new(Mutex::new(None)),
            history_feed: Arc::new(Mutex::new(None)),
            cube_face: Cell::new(None),
            cube_status: Cell::new(None),
            on_cube_not_found: RefCell::new(Vec::new()),
            on_blink: RefCell::new(Vec::new()),
            reach_again_after: Cell::new(None),
            reach_again: slint::Timer::default(),
            colours_asked_at: Cell::new(None),
            has_said_task_parameters: Cell::new(false),
            settings_sent_at: Cell::new(None),
            pause_claim: Cell::new(None),
            is_forced_pause_sending: Cell::new(false),
            is_history_fetching: Cell::new(false),
            is_another_fetch_wanted: RefCell::new(None),
            history_timer: slint::Timer::default(),
            on_history_changed: RefCell::new(Vec::new()),
            battery: Cell::new(None),
            early_charge: Cell::new(None),
            heard: RefCell::new(Vec::new()),
            stop: Arc::new(AtomicBool::new(false)),
            is_radio_scanning: Arc::new(AtomicBool::new(false)),
            is_scanning: Cell::new(false),
            is_reaching_for_cube: Cell::new(false),
            is_factory_reset_running: Cell::new(false),
            is_battery_low: Cell::new(false),
            is_blink_on: Cell::new(false),
            blink: slint::Timer::default(),
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
        data.on_rename_opened(action(Device::rename_opened));
        let weak = Rc::downgrade(&device);
        data.on_rename_edited(move |text| {
            if let Some(device) = weak.upgrade() {
                device.limit_rename(&text);
            }
        });
        let weak = Rc::downgrade(&device);
        data.on_rename_committed(move |text| {
            if let Some(device) = weak.upgrade() {
                device.rename_committed(&text);
            }
        });
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
                    device.edited(setting, i64::from(value));
                }
            }
        };
        data.on_auto_pause_edited(sends(DeviceSetting::AutoPause));
        data.on_led_brightness_edited(sends(DeviceSetting::LedBrightness));
        data.on_led_blink_edited(sends(DeviceSetting::LedBlink));
        device
    }

    /// Gives the PIN a second home, `fallback`, which is written when the secret store will not take a PIN and read
    /// when it will not answer. Call at launch, before a cube is paired or reconnected.
    pub fn set_pin_fallback(&self, fallback: Arc<dyn SecretStore>) {
        *self.pin_fallback.borrow_mut() = Some(fallback);
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
        let refusal = name::rename_refusal(paired, connected, pairing.name.as_deref())
            .or_else(|| self.is_factory_reset_running.get().then_some(name::RenameRefusal::NotConnected));
        data.set_can_rename(refusal.is_none());
        data.set_rename_help(refusal.map_or("", |refusal| refusal.help()).into());
        data.set_battery_alert(self.is_battery_low.get() && self.is_blink_on.get());
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
        // A value being edited is not read back underneath the person editing it, until its send has ended.
        if !self.edited.auto_pause.is_open() {
            data.set_auto_pause_minutes(settings.auto_pause_minutes as i32);
        }
        if !self.edited.led_brightness.is_open() {
            data.set_led_brightness_percent(settings.led_brightness_percent as i32);
        }
        if !self.edited.led_blink.is_open() {
            data.set_led_blink_seconds(settings.led_blink_seconds as i32);
        }
        data.set_can_scan(self.radio.is_some());
        data.set_is_scanning(self.is_scanning.get());
        data.set_is_reaching_for_cube(self.is_reaching_for_cube.get());
        data.set_is_factory_reset_running(self.is_factory_reset_running.get());
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
        let lines = Lines(self.sender.clone());
        self.outstanding.set(self.outstanding.get() + 1);
        std::thread::spawn(move || {
            let outcome = work(&lines);
            // A closed channel means the window has gone, and the outcome has nobody to go to.
            if lines.0.send(outcome).is_err() {
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
        while let Ok(outcome) = self.receiver.try_recv() {
            match outcome {
                Outcome::Log(tag, message, failure) => {
                    if failure {
                        self.log.record_failure(tag, || message);
                    } else {
                        self.log.record(tag, || message);
                    }
                }
                // A scan reports every device it hears before it ends, and only its end is a finished job; a charge
                // the cube sends belongs to no job at all.
                Outcome::Heard(advert) => self.finish(Outcome::Heard(advert)),
                Outcome::Battery(percent) => self.finish(Outcome::Battery(percent)),
                Outcome::Face(face) => self.finish(Outcome::Face(face)),
                Outcome::SystemState(bytes) => self.finish(Outcome::SystemState(bytes)),
                outcome => {
                    self.outstanding.set(self.outstanding.get().saturating_sub(1));
                    self.finish(outcome);
                }
            }
        }
        // A held link sends notifications no job is waiting for, so the pump keeps running while one is held.
        let is_link_held = self.link.try_lock().map_or(true, |slot| slot.is_some());
        if self.outstanding.get() == 0 && !is_link_held {
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
                self.notify_history_changed();
                if self.reach_again_after.get().is_some() {
                    self.schedule_reach_again();
                } else {
                    for callback in self.on_cube_not_found.borrow().iter() {
                        callback();
                    }
                }
            }
            Outcome::CubeCommanded { reason, status } => {
                match status {
                    Ok(status) => self.cube_status.set(Some(status)),
                    Err(error) => self.log.record(Tag::Command, || {
                        format!("The cube did not answer about its state: {}", plain(&error))
                    }),
                }
                self.fetch_history(&reason);
                self.notify_history_changed();
            }
            Outcome::Sent { setting, value, result } => self.sent(setting, value, result),
            Outcome::LinkLost => {
                self.link_gone();
                if let Some(connection) = self.connect() {
                    self.report(rows::record_connection_lost(&connection, &*self.log));
                }
                self.check_battery_warning();
                self.reach_again_after.set(Some(REACH_AGAIN_FIRST));
                self.schedule_reach_again();
            }
            Outcome::Face(face) => self.face_arrived(face),
            Outcome::SystemState(bytes) => self.system_state_arrived(&bytes),
            Outcome::Synced { status, system_state } => {
                if let Some(status) = status {
                    self.cube_status.set(Some(status));
                }
                if let Some(bytes) = system_state {
                    self.system_state_arrived(&bytes);
                }
                self.notify_history_changed();
            }
            Outcome::HistoryFetched { reason, result } => self.history_fetched(&reason, result),
            Outcome::Battery(reading) => self.charge_arrived(reading),
            Outcome::ResetEnded { label, outcome } => self.reset_ended(&label, outcome),
            Outcome::Renamed { name, result } => self.renamed(&name, result),
            Outcome::Released | Outcome::Log(..) => {}
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

    /// Stops a scan that is running, saying `reason`: leaving the Device tab or closing the window. Does nothing with
    /// no scan running.
    pub fn stop_scan(&self, reason: &str) {
        if self.is_scanning.get() && !self.stop.load(Ordering::Relaxed) {
            self.log.record(Tag::Radio, || format!("Stopping the scan: {}", plain(reason)));
            self.stop.store(true, Ordering::Relaxed);
        }
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
        let is_radio_scanning = Arc::clone(&self.is_radio_scanning);
        is_radio_scanning.store(true, Ordering::Relaxed);
        self.run(move |lines| {
            lines.record(Tag::Radio, || "Scanning, unfiltered".to_string());
            let result = radio.scan(Duration::from_secs(scan::SCAN_SECONDS), &stop, &mut |advert| {
                if sender.send(Outcome::Heard(advert.clone())).is_err() {
                    stop.store(true, Ordering::Relaxed);
                }
            });
            is_radio_scanning.store(false, Ordering::Relaxed);
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
        let fallback = self.pin_fallback.borrow().clone();
        let held = Arc::clone(&self.link);
        let feed = Arc::clone(&self.history_feed);
        let is_radio_scanning = Arc::clone(&self.is_radio_scanning);
        self.run(move |lines| {
            // **BlueZ aborts a connection made while it is still discovering** (`le-connection-abort-by-local`,
            // measured on the laptop 2026-09-28: the connect went out 200ms before the stopped scan had ended). So the
            // scan this press stopped is waited out before connecting.
            let waited = std::time::Instant::now();
            while is_radio_scanning.load(Ordering::Relaxed) && waited.elapsed() < SCAN_END_WAIT {
                std::thread::sleep(Duration::from_millis(50));
            }
            if is_radio_scanning.load(Ordering::Relaxed) {
                lines.record(Tag::Radio, || {
                    "The scan had not ended after 5s, so connecting anyway".to_string()
                });
            }
            let stored = match read_pins(&pins, fallback.as_ref(), lines) {
                Ok(stored) => stored,
                Err(message) => return Outcome::PairingFailed { label, message },
            };
            let candidates = login::pairing_candidates(&stored.order());
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
                session::LoginOutcome::LoggedIn { link, pin, rotated } => settle(
                    link,
                    &pin,
                    rotated,
                    &stored,
                    &pins,
                    fallback.as_ref(),
                    &held,
                    &feed,
                    lines,
                    handle,
                    label,
                ),
                outcome => Outcome::PairingFailed { message: outcome.describe(&label), label },
            }
        });
    }

    /// Finds the paired cube again and logs in to it: scans for TimeFlips and names this app gave the cube,
    /// tries the handle last recorded first, and presents the stored PIN and then the vendor PIN to each, on
    /// connections of their own. The one that accepts is this app's cube. Does nothing when no cube is paired or
    /// there is no radio. Call once at launch.
    /// Looks for the cube that dropped out again after the current wait, doubling the wait for the next miss. Stops
    /// once the cube is reached, forgotten or reset.
    fn schedule_reach_again(&self) {
        let Some(after) = self.reach_again_after.get() else { return };
        self.log.record(Tag::Pair, || {
            format!("The cube went away; looking for it again in {}s", after.as_secs())
        });
        self.reach_again_after.set(Some((after * 2).min(REACH_AGAIN_AT_MOST)));
        let weak = self.this.borrow().clone();
        self.reach_again.start(slint::TimerMode::SingleShot, after, move || {
            let Some(device) = weak.upgrade() else { return };
            if device.reach_again_after.get().is_none() || device.is_cube_connected() {
                return;
            }
            device.reconnect();
        });
    }

    /// Stops looking for a cube that dropped out.
    fn stop_reaching_again(&self) {
        self.reach_again_after.set(None);
        self.reach_again.stop();
    }

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
        let fallback = self.pin_fallback.borrow().clone();
        let held = Arc::clone(&self.link);
        let feed = Arc::clone(&self.history_feed);
        self.run(move |lines| {
            lines.record(Tag::Pair, || "Looking for the paired cube".to_string());
            let stored = match read_pins(&pins, fallback.as_ref(), lines) {
                Ok(stored) => stored,
                Err(message) => return Outcome::ReconnectFailed { message },
            };
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
            let candidates = login::reconnect_candidates(&stored.order());
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
                            &stored,
                            &pins,
                            fallback.as_ref(),
                            &held,
                            &feed,
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
        self.stop_reaching_again();
        self.heard.borrow_mut().clear();
        self.battery.set(paired.battery);
        self.cube_face.set(paired.face);
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
        match &paired.status {
            Ok(status) => self.cube_status.set(Some(*status)),
            Err(reason) => self.log.record(Tag::Command, || {
                format!("The status of the cube could not be read: {}", plain(reason))
            }),
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
        if let Some(reading) = self.early_charge.take() {
            self.charge_arrived(reading);
        }
        self.check_battery_warning();
        self.fetch_history("the link came up");
        self.arm_history_timer(true);
        // After the login's own questions, never before: all twelve faces in their categories' colours.
        let all: Vec<i64> = (1..=12).collect();
        self.send_face_colours(&all, "the cube connected");
        self.has_said_task_parameters.set(false);
        self.sync_settings(true);
    }

    /// Sends the table's device settings to the connected cube, read from the table now: LED brightness and blink
    /// interval always, having no read-back, and auto-pause when the cube's last `0x10` answer differs from the table.
    /// With `ask_state`, the system state is read after.
    fn sync_settings(&self, ask_state: bool) {
        let Some(connection) = self.connect() else { return };
        let Some(settings) = self.report(rows::settings(&connection)) else { return };
        let cube_minutes = self.cube_status.get().map(|status| i64::from(status.auto_pause_minutes));
        self.settings_sent_at.set(Some(std::time::Instant::now()));
        let held = Arc::clone(&self.link);
        self.run(move |lines| {
            let result = with_link(&held, |link| {
                lines.record(Tag::Command, || {
                    format!("Telling the cube LED brightness {}%", settings.led_brightness_percent)
                });
                link.write(uuids::COMMAND, &command::set_led_brightness(settings.led_brightness_percent))?;
                lines.record(Tag::Command, || format!("Telling the cube blink period {}s", settings.led_blink_seconds));
                link.write(uuids::COMMAND, &command::set_led_blink(settings.led_blink_seconds))?;
                if let Err(error) = session::turn_double_tap_off(link, lines) {
                    lines.record(Tag::Command, || format!("The double tap could not be checked: {}", plain(&error)));
                }
                let mut status = None;
                if cube_minutes != Some(settings.auto_pause_minutes) {
                    lines.record(Tag::Command, || {
                        format!(
                            "Telling the cube auto-pause {}m (the cube says its auto-pause is {} and the table says {}m)",
                            settings.auto_pause_minutes,
                            cube_minutes.map_or("unknown".to_string(), |minutes| format!("{minutes}m")),
                            settings.auto_pause_minutes
                        )
                    });
                    link.write(uuids::COMMAND, &command::set_auto_pause(settings.auto_pause_minutes))?;
                    status = Some(session::status(link, lines)?);
                }
                let system_state = if ask_state {
                    lines.record(Tag::Device, || "Asking the cube what it needs".to_string());
                    Some(link.read(uuids::SYSTEM_STATE)?)
                } else {
                    None
                };
                Ok((status, system_state))
            });
            match result {
                Ok((status, system_state)) => Outcome::Synced { status, system_state },
                Err(error) => {
                    lines.record(Tag::Command, || format!("The settings did not all reach the cube: {}", plain(&error)));
                    Outcome::Synced { status: None, system_state: None }
                }
            }
        });
    }

    /// Sets the connected cube's clock to now again, on a background thread.
    fn resend_clock(&self) {
        let held = Arc::clone(&self.link);
        self.run(move |lines| {
            if let Err(error) = with_link(&held, |link| {
                set_the_clock(link, lines);
                Ok(())
            }) {
                lines
                    .record(Tag::Command, || format!("The clock could not be sent again: {}", plain(&error)));
            }
            Outcome::Released
        });
    }

    /// Answers what the cube says it needs, and says what it reports about its hardware when that is not all well.
    fn system_state_arrived(&self, bytes: &[u8]) {
        let Some((sync, hardware)) = system_state::parse(bytes) else {
            self.log.record(Tag::Device, || {
                format!("The system state could not be read ({})", command::hex(bytes))
            });
            return;
        };
        if hardware != CubeHardwareState::Ok {
            self.log
                .record_failure(Tag::Device, || format!("The cube reports a hardware fault: {hardware:?}"));
        }
        let all: Vec<i64> = (1..=12).collect();
        match sync {
            CubeSyncState::Ok => self.log.record(Tag::Device, || "The cube needs nothing sent".to_string()),
            CubeSyncState::FactoryReset => {
                self.log.record(Tag::Device, || {
                    "The cube says it was put back to the factory, so everything is sent again".to_string()
                });
                self.resend_clock();
                self.send_face_colours(&all, "the cube says it was put back to the factory");
                self.sync_settings(false);
                self.fetch_history("the cube says it was put back to the factory");
            }
            CubeSyncState::TimeRequired => {
                self.log.record(Tag::Device, || "The cube wants the time".to_string());
                self.resend_clock();
            }
            CubeSyncState::FaceColoursRequired => {
                let is_recent =
                    self.colours_asked_at.get().is_some_and(|at| at.elapsed() < Duration::from_secs(30));
                if is_recent {
                    self.log.record(Tag::Device, || {
                        "The cube wants its face colours again, and they went less than 30s ago".to_string()
                    });
                } else {
                    self.colours_asked_at.set(Some(std::time::Instant::now()));
                    self.log.record(Tag::Device, || "The cube wants its face colours".to_string());
                    self.send_face_colours(&all, "the cube asked for its face colours");
                }
            }
            CubeSyncState::LedBrightnessRequired
            | CubeSyncState::BlinkIntervalRequired
            | CubeSyncState::AutoPauseRequired => {
                let is_recent =
                    self.settings_sent_at.get().is_some_and(|at| at.elapsed() < Duration::from_secs(10));
                if is_recent {
                    self.log.record(Tag::Device, || {
                        format!("The cube wants its settings ({sync:?}), and they went less than 10s ago")
                    });
                } else {
                    self.log.record(Tag::Device, || format!("The cube wants its settings: {sync:?}"));
                    if sync == CubeSyncState::AutoPauseRequired {
                        self.cube_status.set(None);
                    }
                    self.sync_settings(false);
                }
            }
            CubeSyncState::TaskParametersRequired => {
                if !self.has_said_task_parameters.replace(true) {
                    self.log.record(Tag::Device, || {
                        "The cube wants its task parameters, which this app has never set, so it goes on asking"
                            .to_string()
                    });
                }
            }
            CubeSyncState::Unknown => self.log.record(Tag::Device, || {
                format!("The cube reports a state this app does not know ({})", command::hex(bytes))
            }),
        }
    }

    /// Lights each of `faces` on the connected cube in its category's colour, read from the table now, on a background
    /// thread. `0x11` has no read-back, so the cube taking the write is all there is. Said and skipped with no cube
    /// connected.
    pub fn send_face_colours(&self, faces: &[i64], reason: &str) {
        if !self.is_cube_connected() {
            self.log
                .record(Tag::Colour, || format!("No cube connected, so {} lights nothing", plain(reason)));
            return;
        }
        let Some(connection) = self.connect() else { return };
        let mut lit = Vec::new();
        for face in faces.iter().copied().filter(|face| (1..=12).contains(face)) {
            let Some((name, hex)) = self.report(colour::of_face(&connection, face)) else { return };
            lit.push((face as u8, name, hex));
        }
        let held = Arc::clone(&self.link);
        let reason = plain(reason);
        self.run(move |lines| {
            let result = with_link(&held, |link| {
                for (face, name, hex) in &lit {
                    let bytes = colour::command(*face, hex.as_deref());
                    link.write(uuids::COMMAND, &bytes)?;
                    let [red, green, blue] = colour::rgb16(hex.as_deref());
                    lines.record(Tag::Colour, || {
                        format!(
                            "The cube took face {face} {} {} as rgb16 {red:04x},{green:04x},{blue:04x} ({reason}), with no \
                             read-back to confirm it",
                            name.as_deref().map_or("no category".to_string(), plain),
                            hex.as_deref().unwrap_or("off")
                        )
                    });
                }
                Ok(())
            });
            if let Err(error) = result {
                lines.record(Tag::Colour, || format!("The face colours did not all go ({reason}): {}", plain(&error)));
            }
            Outcome::Released
        });
    }

    /// Clears what the app held about a link that has gone: liveness, the history timer and feed, the face and the
    /// charge.
    fn link_gone(&self) {
        self.liveness.stop();
        if self.history_timer.running() {
            self.history_timer.stop();
            self.log.record(Tag::History, || "History timer stopped, the cube is not connected".to_string());
        }
        if let Ok(mut feed) = self.history_feed.lock() {
            *feed = None;
        }
        if self.cube_face.take().is_some() {
            self.log.record(Tag::Face, || "The face goes with the link".to_string());
        }
        self.cube_status.set(None);
        self.pause_claim.set(None);
        self.is_forced_pause_sending.set(false);
        self.battery.set(None);
        self.notify_history_changed();
    }

    /// Registers `callback` to run when the reconnect at launch does not find the paired cube.
    pub fn set_on_cube_not_found(&self, callback: impl Fn() + 'static) {
        self.on_cube_not_found.borrow_mut().push(Box::new(callback));
    }

    /// The face the connected cube last said was up. `None` with no cube connected or no face named yet.
    pub fn cube_face(&self) -> Option<u8> {
        self.cube_face.get().filter(|_| self.is_cube_connected())
    }

    /// Whether a paired launch is still reaching for its cube, with no link held yet. False while looking for the cube
    /// again after a link dropped, which is the cube being unreachable rather than a launch connecting.
    pub fn is_connecting(&self) -> bool {
        self.is_reaching_for_cube.get() && !self.is_cube_connected() && self.reach_again_after.get().is_none()
    }

    /// Whether the low battery warning is on, and whether its blink is on the lit half.
    pub fn battery_warning(&self) -> (bool, bool) {
        (self.is_battery_low.get(), self.is_blink_on.get())
    }

    /// Registers `callback` to run on each half of the low battery blink, and when the warning goes on or off.
    pub fn set_on_blink(&self, callback: impl Fn() + 'static) {
        self.on_blink.borrow_mut().push(Box::new(callback));
    }

    fn notify_blink(&self) {
        for callback in self.on_blink.borrow().iter() {
            callback();
        }
    }

    /// Whether a link to the cube is held now.
    pub fn is_cube_connected(&self) -> bool {
        self.link.try_lock().map_or(true, |slot| slot.is_some()) && !self.is_factory_reset_running.get()
    }

    /// Whether the cube said it was locked when last asked. `None` with no cube connected or no answer yet.
    pub fn is_cube_locked(&self) -> Option<bool> {
        self.cube_status.get().map(|status| status.is_locked)
    }

    /// Pauses the cube if it is running and resumes it if it is paused, on a background thread. Which way is read
    /// from the open cube segment, or from the cube's last `0x10` answer while there is none. Refused, and said, with
    /// no cube connected or with the cube locked.
    pub fn toggle_cube_pause(&self) {
        if !self.is_cube_connected() {
            self.log.record(Tag::Command, || {
                "No cube connected, so there is nothing to pause or resume".to_string()
            });
            return;
        }
        if self.is_cube_locked() == Some(true) {
            self.log.record(Tag::Command, || {
                "The cube is locked, so pausing it means nothing; unlock it first".to_string()
            });
            return;
        }
        let Some(connection) = self.connect() else { return };
        let Some(reading) = self.report(facet_core::timing::read_cube(&connection, now_seconds())) else {
            return;
        };
        // The open cube segment says which way, and with none yet the cube's own last answer does.
        let is_paused = match &reading {
            Some(reading) => reading.is_paused,
            None => self.cube_status.get().is_some_and(|status| status.is_paused),
        };
        let pause = !is_paused;
        if !pause && reading.as_ref().is_some_and(|reading| reading.is_limit_reached) {
            self.log.record(Tag::Limit, || {
                "The cube is left stopped: the category on show has spent its daily limit".to_string()
            });
            return;
        }
        self.pause_claim.set(None);
        let held = Arc::clone(&self.link);
        self.run(move |lines| {
            let status =
                with_link(&held, |link| session::set_pause(link, pause, lines).map(|(status, _)| status));
            Outcome::CubeCommanded {
                reason: format!(
                    "the cube was {} from the menu bar",
                    if pause { "paused" } else { "resumed" }
                ),
                status,
            }
        });
    }

    /// Locks the cube if it is unlocked, pausing it first when pause_on_lock is on, and unlocks it if it is locked.
    /// **Unlocking never changes whether the cube is paused**: a paused cube stays paused and a running one stays
    /// running. Refused, and said, with no cube connected.
    pub fn toggle_cube_lock(&self) {
        if !self.is_cube_connected() {
            self.log.record(Tag::Command, || {
                "No cube connected, so there is nothing to lock or unlock".to_string()
            });
            return;
        }
        let unlock = self.is_cube_locked() == Some(true);
        self.pause_claim.set(None);
        let pause_on_lock = self
            .connect()
            .and_then(|connection| self.report(rows::settings(&connection)))
            .is_none_or(|settings| settings.pause_on_lock);
        let held = Arc::clone(&self.link);
        self.run(move |lines| {
            let status = with_link(&held, |link| {
                if unlock { session::unlock(link, lines) } else { lock_the_cube(link, pause_on_lock, lines) }
            });
            Outcome::CubeCommanded {
                reason: format!(
                    "the cube was {} from the menu bar",
                    if unlock { "unlocked" } else { "locked" }
                ),
                status,
            }
        });
    }

    /// Pauses the cube itself when it is counting on a face with no category, or on a category that has spent its
    /// daily limit, and starts it again once a face it paused for having no category is given one. Read from the table
    /// now; does nothing while locked, with no cube connected, or while its own last decision is still on its way.
    pub fn enforce_cube_rules(&self) {
        if !self.is_cube_connected()
            || self.is_forced_pause_sending.get()
            || self.is_cube_locked() != Some(false)
        {
            return;
        }
        let Some(connection) = self.connect() else { return };
        let Some(Some(reading)) = self.report(facet_core::timing::read_cube(&connection, now_seconds()))
        else {
            return;
        };
        let cube = Resting {
            face: reading.face,
            is_paused: reading.is_paused,
            has_category: reading.category.is_some(),
            is_limit_reached: reading.is_limit_reached,
        };
        let (pause, claim) = match forced_pause::decide(cube, self.pause_claim.get()) {
            Decision::Leave => return,
            Decision::Release => {
                self.log.record(Tag::Forced, || {
                    "Forced pause released: the cube is no longer stopped on the face the app stopped it on"
                        .to_string()
                });
                self.pause_claim.set(None);
                return;
            }
            Decision::Pause(claim) => (true, claim),
            Decision::Resume(claim) => (false, claim),
        };
        let face = reading.face;
        match (pause, claim) {
            (true, PauseClaim::NoCategory { .. }) => self.log.record(Tag::Forced, || {
                format!("Forced pause: face {face} has no category, so the cube is being stopped")
            }),
            (true, PauseClaim::DailyLimit) => {
                let name = reading.category.as_ref().map_or(String::new(), |category| plain(&category.name));
                let minutes = reading.seconds / 60;
                self.log.record(Tag::Limit, || {
                    format!("Daily limit reached: {name} has spent {minutes}m, stopping the clock")
                });
            }
            (false, _) => self.log.record(Tag::Forced, || {
                "Forced pause lifted: the face has a category now, so the cube is being started".to_string()
            }),
        }
        self.pause_claim.set(pause.then_some(claim));
        self.is_forced_pause_sending.set(true);
        let held = Arc::clone(&self.link);
        self.run(move |lines| {
            let status =
                with_link(&held, |link| session::set_pause(link, pause, lines).map(|(status, _)| status));
            Outcome::CubeCommanded {
                reason: match (pause, claim) {
                    (true, PauseClaim::NoCategory { .. }) => format!("face {face} has no category"),
                    (true, PauseClaim::DailyLimit) => "a category spent its daily limit".to_string(),
                    (false, _) => format!("face {face} has a category now"),
                },
                status,
            }
        });
    }

    /// Registers `callback` to run whenever a history fetch has written to `device_event`, when the link goes, and when
    /// a look for the paired cube ends without it.
    pub fn set_on_history_changed(&self, callback: impl Fn() + 'static) {
        self.on_history_changed.borrow_mut().push(Box::new(callback));
    }

    fn notify_history_changed(&self) {
        for callback in self.on_history_changed.borrow().iter() {
            callback();
        }
    }

    /// Takes the face the cube says is up. A turn onto another face is logged and fetches history.
    fn face_arrived(&self, face: u8) {
        // A face named while a login is still under way is the login's to read.
        if self.cube_face.get() == Some(face)
            || self.is_factory_reset_running.get()
            || self.is_reaching_for_cube.get()
        {
            return;
        }
        self.cube_face.set(Some(face));
        self.log.record(Tag::Face, || format!("Face {face} is up"));
        self.fetch_history("the cube was turned");
    }

    /// Fetches the cube's history on a background thread and files it when it comes back. A fetch asked for while one
    /// runs is run once that one ends, however many are asked for. Does nothing with no link held.
    pub fn fetch_history(&self, reason: &str) {
        let is_link_held = self.link.try_lock().map_or(true, |slot| slot.is_some());
        if !is_link_held || self.is_factory_reset_running.get() {
            return;
        }
        if self.is_history_fetching.get() {
            self.log.record(Tag::History, || {
                format!("Already fetching history ({reason}): asking again when this one is done")
            });
            self.is_another_fetch_wanted.borrow_mut().get_or_insert_with(|| reason.to_string());
            return;
        }
        let Some(connection) = self.connect() else { return };
        let Some(recorded) = self.report(cube_history::resume_point(&connection)) else { return };
        self.is_history_fetching.set(true);
        self.log.record(Tag::History, || {
            format!(
                "Fetching history ({reason}); on record: {}",
                recorded
                    .map_or("nothing".to_string(), |(number, start)| format!("event {number} at {start}"))
            )
        });
        let held = Arc::clone(&self.link);
        let feed = Arc::clone(&self.history_feed);
        let reason = reason.to_string();
        self.run(move |lines| {
            let result = (|| {
                let mut slot = held.lock().map_err(|_| "the link is poisoned".to_string())?;
                let link = slot.as_mut().ok_or_else(|| "there is no cube connected".to_string())?;
                let feed = feed.lock().map_err(|_| "the history feed is poisoned".to_string())?;
                let feed = feed.as_ref().ok_or_else(|| "the history is not being followed".to_string())?;
                session::fetch_history(&mut **link, feed, recorded, lines)
            })();
            Outcome::HistoryFetched { reason, result }
        });
    }

    fn history_fetched(&self, reason: &str, result: Result<Fetched, String>) {
        self.is_history_fetching.set(false);
        let outcome = match self.connect() {
            None => "the database would not open".to_string(),
            Some(connection) => match result {
                Ok(Fetched::Unchanged(frame)) => {
                    match self.report(cube_history::record(
                        &connection,
                        &*self.zone,
                        &frame,
                        true,
                        &*self.log,
                    )) {
                        Some(_) => format!("event {} again, {}s", frame.event_number, frame.duration_seconds),
                        None => "the table refused the write".to_string(),
                    }
                }
                Ok(Fetched::Frames { frames, latest }) => match history::plan(&frames, latest) {
                    Some(batch) => {
                        match self.report(cube_history::record_batch(
                            &connection,
                            &*self.zone,
                            &batch,
                            &*self.log,
                        )) {
                            Some(count) => format!("{count} frame(s) written"),
                            None => "the table refused a write".to_string(),
                        }
                    }
                    None => {
                        let last = frames.iter().map(|frame| frame.event_number).max().unwrap_or(0);
                        format!(
                            "the stream stopped at event {last}, short of the latest the cube reports, {}, so none of it is \
                             written",
                            latest.unwrap_or(0)
                        )
                    }
                },
                Err(reason) => format!("failed, {}", plain(&reason)),
            },
        };
        self.log.record(Tag::History, || format!("History fetch done ({reason}): {}", plain(&outcome)));
        // A pause the app sent itself stands decided until a fetch has filed what the cube did with it.
        self.is_forced_pause_sending.set(false);
        self.notify_history_changed();
        self.enforce_cube_rules();
        if let Some(again) = self.is_another_fetch_wanted.borrow_mut().take() {
            self.fetch_history(&again);
        }
    }

    /// Arms the one-shot history timer on the interval the table holds now, read again at every firing. `announce`
    /// logs the start.
    fn arm_history_timer(&self, announce: bool) {
        let seconds = self
            .connect()
            .and_then(|connection| self.report(app_settings::fetch_interval_seconds(&connection)))
            .unwrap_or(10)
            .clamp(1, 3600);
        if announce {
            self.log.record(Tag::History, || format!("History timer started, asking every {seconds}s"));
        }
        let weak = self.this.borrow().clone();
        self.history_timer.start(
            slint::TimerMode::SingleShot,
            Duration::from_secs(seconds as u64),
            move || {
                let Some(device) = weak.upgrade() else { return };
                let is_link_held = device.link.try_lock().map_or(true, |slot| slot.is_some());
                if !is_link_held {
                    device.log.record(Tag::History, || {
                        "History timer stopped, it fired with no link to the cube held".to_string()
                    });
                    return;
                }
                device
                    .log
                    .record(Tag::History, || format!("History timer fired, asking on a {seconds}s interval"));
                device.fetch_history("the timer asked");
                device.arm_history_timer(false);
            },
        );
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
        self.stop_reaching_again();
        self.link_gone();
        self.check_battery_warning();
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

    /// Leaves the cube paused and locked, lets go of it and records the quit. Blocks until the cube is let go of or
    /// [`QUIT_DEADLINE`] passes, whichever is first; call once, as the app quits.
    pub fn quit(&self) {
        self.stop_reaching_again();
        self.liveness.stop();
        self.history_timer.stop();
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
        // The cube is left locked, so nothing counts while the app is not watching; paused first when pause_on_lock
        // is on, read at this step. A cube that would not answer is let go of anyway.
        let pause_on_lock = self
            .connect()
            .and_then(|connection| self.report(rows::settings(&connection)))
            .is_none_or(|settings| settings.pause_on_lock);
        let (lines_sender, lines) = channel();
        let (done_sender, done) = channel();
        let worker_lines = Lines(lines_sender);
        std::thread::spawn(move || {
            let is_locked =
                lock_the_cube(&mut *link, pause_on_lock, &worker_lines).is_ok_and(|status| status.is_locked);
            session::disconnect(&mut *link, &worker_lines);
            if done_sender.send(is_locked).is_err() {
                eprintln!("facet: the quit had stopped waiting for the cube before it was let go of");
            }
        });
        let deadline = std::time::Instant::now() + QUIT_DEADLINE;
        let is_locked = loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            match lines.recv_timeout(left) {
                Ok(Outcome::Log(tag, message, true)) => self.log.record_failure(tag, || message),
                Ok(Outcome::Log(tag, message, false)) => self.log.record(tag, || message),
                Ok(other) => self.log.record(Tag::Quit, || {
                    format!("Quit: {} arrived while locking the cube, and was dropped", describe_lost(&other))
                }),
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => match done.try_recv() {
                    Ok(is_locked) => break Some(is_locked),
                    Err(error) => {
                        self.log.record_failure(Tag::Quit, || {
                            format!("Quit: locking the cube ended without an answer: {error}")
                        });
                        break Some(false);
                    }
                },
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break None,
            }
        };
        self.log.record(Tag::Quit, || match (is_locked, pause_on_lock) {
            (Some(true), true) => "Quit: the cube is paused and locked".to_string(),
            (Some(true), false) => "Quit: the cube is locked".to_string(),
            (Some(false), _) => "Quit: the cube was not left locked".to_string(),
            (None, _) => format!(
                "Quit: the cube did not answer within {}s, so the app quits without it",
                QUIT_DEADLINE.as_secs()
            ),
        });
        if let Some(connection) = self.connect() {
            self.report(rows::record_quit(&connection, &*self.log));
        }
    }

    /// Asks whether to factory reset the connected cube, and resets it on a yes. Cancel is the first choice, so a
    /// stray Return does not reset.
    fn reset_pressed(&self) {
        self.log.record(Tag::Click, || "Button clicked: Reset Device".to_string());
        let weak = self.this.borrow().clone();
        self.notice.ask_with_way_out(
            "Reset this TimeFlip to factory settings?",
            "This erases everything stored on the device -- face colours, task settings, name, and password -- back \
             to factory defaults. This cannot be undone.",
            &["Cancel", "Reset Device"],
            Some(0),
            move |choice| {
                let Some(device) = weak.upgrade() else { return };
                if choice == 1 {
                    device.reset();
                } else {
                    device.log.record(Tag::Pair, || "The reset was called off".to_string());
                }
            },
        );
    }

    /// Sends `0xFF` over the held link and proves the wipe on the factory PIN, on a background thread. Forget,
    /// Reset and the Name row stand down until it ends.
    fn reset(&self) {
        let Some(radio) = self.radio.clone() else { return };
        let Some(connection) = self.connect() else { return };
        let Some(pairing) = self.report(rows::pairing(&connection)) else { return };
        let Some(handle) = pairing.handle.clone().filter(|_| pairing.is_cube_connected) else {
            self.log.record(Tag::Pair, || "Asked to reset with no cube connected".to_string());
            return;
        };
        let label = pairing.name.clone().unwrap_or_else(|| "this TimeFlip".to_string());
        self.is_factory_reset_running.set(true);
        self.stop_reaching_again();
        self.link_gone();
        self.set_status(format!("Resetting {label}..."));
        self.draw();
        let held = Arc::clone(&self.link);
        self.run(move |lines| {
            let link = match held.lock() {
                Ok(mut slot) => slot.take(),
                Err(_) => None,
            };
            let Some(mut link) = link else {
                return Outcome::ResetEnded {
                    label,
                    outcome: ResetOutcome::Failed("there is no cube connected".to_string()),
                };
            };
            let outcome = session::factory_reset(
                &*radio,
                &handle,
                &mut *link,
                session::RESET_PROOF_EVERY,
                session::RESET_PROOF_ATTEMPTS,
                lines,
            );
            // A reset that could not be sent leaves the link as it was, so it is held again.
            if matches!(outcome, ResetOutcome::Failed(_))
                && let Ok(mut slot) = held.lock()
            {
                *slot = Some(link);
            }
            Outcome::ResetEnded { label, outcome }
        });
    }

    fn reset_ended(&self, label: &str, outcome: ResetOutcome) {
        self.is_factory_reset_running.set(false);
        self.log.record(Tag::Pair, || {
            format!(
                "Reset: {}",
                match &outcome {
                    ResetOutcome::Confirmed => "confirmed",
                    ResetOutcome::Unconfirmed => "not confirmed",
                    ResetOutcome::Failed(_) => "not sent",
                }
            )
        });
        let Some(connection) = self.connect() else { return };
        match outcome {
            ResetOutcome::Confirmed => {
                self.battery.set(None);
                if self.report(rows::record_factory_reset(&connection, &*self.log)) != Some(true) {
                    self.notice.tell(
                        "The reset was not saved",
                        "The cube is back to factory settings, but the database would not record that it is no \
                         longer paired.",
                    );
                }
                self.set_status(format!("{label} was reset and is back to factory settings."));
            }
            ResetOutcome::Unconfirmed => {
                self.battery.set(None);
                self.report(rows::record_connection_lost(&connection, &*self.log));
                self.set_status(format!(
                    "{label} did not come back after the reset, so nothing has been changed. Flip it to wake it, \
                     then try again."
                ));
            }
            ResetOutcome::Failed(reason) => {
                self.log.record(Tag::Command, || {
                    format!("The cube would not take the reset: {}", plain(&reason))
                });
                self.set_status(format!("Could not send the reset to {label}: {reason}"));
                self.start_liveness();
            }
        }
        self.check_battery_warning();
    }

    /// Opens the Name row for editing, with the name on record in it.
    fn rename_opened(&self) {
        let Some(connection) = self.connect() else { return };
        let Some(pairing) = self.report(rows::pairing(&connection)) else { return };
        if name::rename_refusal(pairing.is_cube_paired, pairing.is_cube_connected, pairing.name.as_deref())
            .is_some()
        {
            return;
        }
        if let Some(ui) = self.ui.upgrade() {
            let data = ui.global::<DeviceData>();
            data.set_editing_device_name(pairing.name.unwrap_or_default().into());
            data.set_is_editing_device_name(true);
        }
    }

    /// Cuts the name being typed to the most the cube stores.
    fn limit_rename(&self, text: &str) {
        if text.chars().count() > name::MAXIMUM_LENGTH
            && let Some(ui) = self.ui.upgrade()
        {
            let cut: String = text.chars().take(name::MAXIMUM_LENGTH).collect();
            ui.global::<DeviceData>().set_editing_device_name(cut.into());
        }
    }

    /// Sends a name typed into the Name row to the cube, and records it once the cube has taken it.
    fn rename_committed(&self, typed: &str) {
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<DeviceData>().set_is_editing_device_name(false);
        }
        let Some(connection) = self.connect() else { return };
        let Some(pairing) = self.report(rows::pairing(&connection)) else { return };
        let name = match name::decide(typed, pairing.name.as_deref()) {
            NameDecision::Ignore => {
                self.log.record(Tag::Settings, || "The device name was left as it was".to_string());
                self.draw();
                return;
            }
            NameDecision::Refuse(problem) => {
                self.log.record(Tag::Settings, || {
                    format!("The device cannot be called {}: {}", plain(typed), problem.title())
                });
                self.notice.tell(problem.title(), &problem.message());
                self.draw();
                return;
            }
            NameDecision::Write(name) => name,
        };
        let Some(bytes) = command::set_name(&name) else {
            self.renamed(&name, Err("the name would not encode".to_string()));
            return;
        };
        self.log.record(Tag::Settings, || format!("Renaming the cube to {}", plain(&name)));
        let held = Arc::clone(&self.link);
        self.run(move |lines| {
            let result = (|| {
                let mut slot = held.lock().map_err(|_| "the link is poisoned".to_string())?;
                let link = slot.as_mut().ok_or_else(|| "there is no cube connected".to_string())?;
                link.write(uuids::COMMAND, &bytes)?;
                // 0x15 has no read-back: the cube reports the name as its GAP name on the next connection.
                lines.record(Tag::Command, || {
                    "The cube took the write; nothing can read this command back".to_string()
                });
                Ok(())
            })();
            Outcome::Renamed { name, result }
        });
    }

    fn renamed(&self, name: &str, result: Result<(), String>) {
        match result {
            Ok(()) => {
                let Some(connection) = self.connect() else { return };
                let previous = self.report(rows::pairing(&connection)).and_then(|pairing| pairing.name);
                match self.report(rows::record_name(
                    &connection,
                    name,
                    "renamed from the Device tab",
                    &*self.log,
                )) {
                    Some(true) => {
                        self.log.record(Tag::Settings, || {
                            format!(
                                "The cube is now called {}, and will go on advertising its old name",
                                plain(name)
                            )
                        });
                        self.notice.tell(
                            "The TimeFlip has been renamed",
                            &name::rename_lag_notice(name, previous.as_deref()),
                        );
                    }
                    _ => self.refused(),
                }
            }
            Err(reason) => {
                self.log.record(Tag::Settings, || {
                    format!("The device name: the cube did not take {}, so the row goes back", plain(name))
                });
                let problem = NameProblem::WriteFailed(reason);
                self.notice.tell(problem.title(), &problem.message());
            }
        }
        self.draw();
    }

    /// Takes a charge the cube sent, as [`info::charge_to_show`] decides, while a cube is connected.
    fn charge_arrived(&self, reading: u8) {
        let connected = self
            .connect()
            .and_then(|connection| self.report(rows::pairing(&connection)))
            .is_some_and(|pairing| pairing.is_cube_connected);
        if self.is_factory_reset_running.get() {
            return;
        }
        if !connected {
            // The login that subscribed has not been recorded yet; the charge is its to take.
            if self.is_reaching_for_cube.get() {
                self.early_charge.set(Some(reading));
            }
            return;
        }
        let shown = info::charge_to_show(self.battery.get(), reading);
        if shown != self.battery.get() {
            self.battery.set(shown);
            if let Some(percent) = shown {
                self.log.record(Tag::Device, || {
                    if percent == reading {
                        format!("Charge {percent}%")
                    } else {
                        format!("Charge {percent}% (the cube said {reading}%)")
                    }
                });
            }
            self.check_battery_warning();
        }
    }

    /// Decides the low-battery warning from the charge shown and the warning level in the table, read now, and
    /// starts or stops the Battery row blinking. Logs the change, the colour being unreadable from outside.
    fn check_battery_warning(&self) {
        let warning = self
            .connect()
            .and_then(|connection| self.report(rows::settings(&connection)))
            .map_or(10, |settings| settings.battery_warning_percent.clamp(0, 100) as u8);
        let was = self.is_battery_low.get();
        let is_low = info::is_battery_low(self.battery.get(), warning, was);
        if is_low != was {
            self.is_battery_low.set(is_low);
            let percent = self.battery.get().map_or("no".to_string(), |percent| format!("{percent}%"));
            self.log.record(Tag::Device, || {
                format!(
                    "Battery warning {} at {percent}, the warning level being {warning}%",
                    if is_low { "on" } else { "off" }
                )
            });
            if is_low {
                let weak = self.this.borrow().clone();
                self.blink.start(slint::TimerMode::Repeated, BLINK_EVERY, move || {
                    if let Some(device) = weak.upgrade() {
                        device.is_blink_on.set(!device.is_blink_on.get());
                        if let Some(ui) = device.ui.upgrade() {
                            ui.global::<DeviceData>()
                                .set_battery_alert(device.is_battery_low.get() && device.is_blink_on.get());
                        }
                        device.notify_blink();
                    }
                });
            } else {
                self.blink.stop();
                self.is_blink_on.set(false);
            }
            self.notify_blink();
        }
        self.draw();
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
        if setting == DeviceSetting::BatteryWarning {
            self.check_battery_warning();
        }
        self.draw();
    }

    /// A stepper changed `setting` to `value`. Only the value it stops on is sent, once, [`EDIT_QUIET_FOR`] after the
    /// last change: a held arrow is a burst of changes, and the cube refuses a second command while the first is
    /// out. The field keeps what was edited until the table holds it or the cube refuses it.
    fn edited(&self, setting: DeviceSetting, value: i64) {
        let Some(edit) = self.edited.of(setting) else { return };
        edit.unsent.set(Some(value));
        self.log.record(Tag::Settings, || format!("Device setting {setting:?} edited to {value}"));
        let weak = self.this.borrow().clone();
        edit.timer.start(slint::TimerMode::SingleShot, EDIT_QUIET_FOR, move || {
            if let Some(device) = weak.upgrade() {
                device.send_edited(setting);
            }
        });
    }

    /// Sends the value `setting` was last edited to, if it has not gone already.
    fn send_edited(&self, setting: DeviceSetting) {
        let Some(value) = self.edited.of(setting).and_then(|edit| edit.unsent.take()) else { return };
        self.send_setting(setting, value);
    }

    /// Sends a setting to the connected cube and stores it once the cube has it. Auto-pause is confirmed by
    /// reading it back with `0x10`; LED brightness and blink interval have no read-back, so the cube's
    /// acknowledgement is all there is.
    fn send_setting(&self, setting: DeviceSetting, value: i64) {
        if let Some(edit) = self.edited.of(setting) {
            edit.is_sending.set(true);
        }
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
        if let Some(edit) = self.edited.of(setting) {
            edit.is_sending.set(false);
        }
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

/// What the two homes of the cube's PIN hold.
struct StoredPins {
    /// The PIN in the config file, which only holds one because the secret store refused it.
    file: Option<String>,
    /// The PIN in the secret store.
    store: Option<String>,
}

impl StoredPins {
    /// The PINs to present, in order.
    fn order(&self) -> Vec<String> {
        pin_source::read_order(self.file.as_deref(), self.store.as_deref())
    }
}

/// The PINs the two homes hold, read now. An error, already logged, is a secret store that would not answer with
/// nothing in the config file to go on, and says so in words fit for the Device tab; the caller stops there rather than
/// presenting a PIN.
///
/// A config file copy that the secret store already holds is removed here, being a live PIN in a plain file for no
/// reason. Two that differ are left for the next login to settle, since only the cube can say which it took.
fn read_pins(
    pins: &Arc<dyn SecretStore>,
    fallback: Option<&Arc<dyn SecretStore>>,
    lines: &Lines,
) -> Result<StoredPins, String> {
    let (store, unreadable) = match timed::look_up(pins) {
        SecretLookup::Found(pin) => (Some(pin), None),
        SecretLookup::Missing => (None, None),
        SecretLookup::Unavailable(reason) => (None, Some(reason)),
    };
    let file = fallback.and_then(|fallback| match timed::look_up(fallback) {
        SecretLookup::Found(pin) => Some(pin),
        SecretLookup::Missing => None,
        SecretLookup::Unavailable(reason) => {
            lines.record_failure(Tag::Pin, || {
                format!("The config file could not be read: {}", plain(&reason))
            });
            None
        }
    });
    if let Some(reason) = unreadable {
        lines.record_failure(Tag::Pin, || format!("The stored PIN could not be read: {}", plain(&reason)));
        if file.is_none() {
            return Err(format!("The stored PIN could not be read, so no cube was contacted: {reason}"));
        }
        lines.record(Tag::Pin, || {
            "The secret store would not say whether it holds a PIN, so the one in the config file is used"
                .to_string()
        });
    }
    match (pin_source::at_read(file.as_deref(), store.as_deref()), fallback) {
        (AtRead::ClearTheFile, Some(fallback)) => clear_config_copy(
            fallback,
            lines,
            "The secret store already holds the PIN the config file names, so the file no longer needs to",
        ),
        (AtRead::AwaitTheCube, _) => lines.record(Tag::Pin, || {
            "The secret store and the config file name different PINs, so the next login settles it"
                .to_string()
        }),
        _ => {}
    }
    Ok(StoredPins { file, store })
}

/// Removes the config file's copy of the PIN, and says whether it went.
fn clear_config_copy(fallback: &Arc<dyn SecretStore>, lines: &Lines, said: &str) {
    match timed::clear(fallback) {
        Ok(()) => lines.record(Tag::Pin, || said.to_string()),
        Err(reason) => lines.record_failure(Tag::Pin, || {
            format!("The config file would not give up its copy of the PIN: {}", plain(&reason))
        }),
    }
}

/// Writes down the PIN a cube has just proved it is on, which is called only after the cube has taken it. The secret
/// store is where it belongs. When that refuses, the config file holds it instead, so the cube is never left on a PIN
/// nothing can name. An error says why nowhere would hold it.
fn record_pin(
    pin: &str,
    rotated: bool,
    stored: &StoredPins,
    pins: &Arc<dyn SecretStore>,
    fallback: Option<&Arc<dyn SecretStore>>,
    lines: &Lines,
) -> Result<(), String> {
    match pin_source::after_login(pin, rotated, stored.file.as_deref(), stored.store.as_deref()) {
        AfterLogin::Nothing => Ok(()),
        AfterLogin::ClearTheFile => {
            if let Some(fallback) = fallback {
                clear_config_copy(
                    fallback,
                    lines,
                    "The secret store holds the PIN the cube answered to, so the config file no longer needs to hold one",
                );
            }
            Ok(())
        }
        AfterLogin::WriteIt => {
            let refused = match timed::store(pins, pin) {
                Ok(true) => None,
                Ok(false) => Some("it did not read back".to_string()),
                Err(reason) => Some(reason),
            };
            let Some(reason) = refused else {
                if let (Some(fallback), Some(_)) = (fallback, stored.file.as_deref()) {
                    clear_config_copy(
                        fallback,
                        lines,
                        "The PIN the cube answered to is in the secret store, so the config file no longer needs to hold one",
                    );
                }
                return Ok(());
            };
            let Some(fallback) = fallback else { return Err(reason) };
            match timed::store(fallback, pin) {
                Ok(true) => {
                    lines.record(Tag::Pin, || {
                        format!(
                            "The secret store would not take the PIN ({}), so it is in the config file instead",
                            plain(&reason)
                        )
                    });
                    Ok(())
                }
                Ok(false) => Err(format!("{reason}; and the config file did not read back")),
                Err(file_reason) => Err(format!("{reason}; and the config file: {file_reason}")),
            }
        }
    }
}

/// Runs `exchange` on the held link, or says there is none.
fn with_link<T>(
    held: &Arc<Mutex<Option<Box<dyn Link>>>>,
    exchange: impl FnOnce(&mut dyn Link) -> Result<T, String>,
) -> Result<T, String> {
    let mut slot = held.lock().map_err(|_| "the link is poisoned".to_string())?;
    let link = slot.as_mut().ok_or_else(|| "there is no cube connected".to_string())?;
    exchange(&mut **link)
}

/// Locks the cube, pausing it first when `pause_first`, and returns the status read back after the lock. The lock is
/// sent even when the pause was not confirmed. The pause goes first because a locked cube reports itself paused.
fn lock_the_cube(link: &mut dyn Link, pause_first: bool, log: &impl Record) -> Result<CubeStatus, String> {
    if pause_first {
        session::set_pause(link, true, log)?;
    } else {
        log.record(Tag::Command, || {
            "pause_on_lock is off, so the cube is locked without pausing it".to_string()
        });
    }
    session::set_lock(link, true, log).map(|(status, _)| status)
}

/// Now, in whole seconds since 1970.
fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64)
}

/// Sets the cube's clock to now, as the first thing after a login, so its history carries usable timestamps. A clock
/// that could not be set is said and the login goes on.
fn set_the_clock(link: &mut dyn Link, lines: &Lines) {
    let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(since) => since.as_secs(),
        Err(error) => {
            lines.record_failure(Tag::Command, || {
                format!("The clock on this machine is before 1970, so the cube is not set: {error}")
            });
            return;
        }
    };
    if let Err(reason) = session::set_clock(link, now, lines) {
        lines.record(Tag::Command, || format!("The clock on the cube could not be set: {}", plain(&reason)));
    }
}

/// Subscribes to the cube's history and keeps the notifications in `feed` for [`session::fetch_history`].
fn follow_history(link: &mut dyn Link, feed: &Arc<Mutex<Option<Receiver<Vec<u8>>>>>, lines: &Lines) {
    match link.subscribe(uuids::HISTORY) {
        Ok(frames) => match feed.lock() {
            Ok(mut slot) => {
                *slot = Some(frames);
                lines.record(Tag::History, || "Following the history".to_string());
            }
            Err(_) => lines.record_failure(Tag::History, || "The history feed is poisoned".to_string()),
        },
        Err(reason) => {
            lines.record(Tag::History, || format!("The history cannot be followed: {}", plain(&reason)))
        }
    }
}

/// Subscribes to the face that is up and forwards each face the cube names to the UI thread, until the link's
/// notifications end or the window has gone.
fn follow_faces(link: &mut dyn Link, lines: &Lines) {
    match link.subscribe(uuids::FACES) {
        Ok(faces) => {
            lines.record(Tag::Face, || "Following the face".to_string());
            let forward = lines.clone();
            std::thread::spawn(move || {
                for bytes in faces {
                    if let Some(face) = face::cube_face(&bytes)
                        && forward.0.send(Outcome::Face(face)).is_err()
                    {
                        break;
                    }
                }
            });
        }
        Err(reason) => lines.record(Tag::Face, || format!("The face cannot be followed: {}", plain(&reason))),
    }
}

/// Subscribes to the cube's system state and forwards each value it sends to the UI thread.
fn follow_system_state(link: &mut dyn Link, lines: &Lines) {
    match link.subscribe(uuids::SYSTEM_STATE) {
        Ok(states) => {
            let forward = lines.clone();
            std::thread::spawn(move || {
                for bytes in states {
                    if forward.0.send(Outcome::SystemState(bytes)).is_err() {
                        break;
                    }
                }
            });
        }
        Err(reason) => {
            lines.record(Tag::Device, || format!("The system state cannot be followed: {}", plain(&reason)))
        }
    }
}

/// Subscribes to the cube's battery level and forwards each charge it sends to the UI thread, until the link's
/// notifications end or the window has gone. A subscription the cube refuses is said and nothing more.
fn follow_battery(link: &mut dyn Link, lines: &Lines) {
    match link.subscribe(uuids::BATTERY_LEVEL) {
        Ok(charges) => {
            lines.record(Tag::Device, || "Following the battery".to_string());
            let forward = lines.clone();
            std::thread::spawn(move || {
                for bytes in charges {
                    if let Some(percent) = info::battery_percent(&bytes)
                        && forward.0.send(Outcome::Battery(percent)).is_err()
                    {
                        break;
                    }
                }
            });
        }
        Err(reason) => {
            lines.record(Tag::Device, || format!("The battery cannot be followed: {}", plain(&reason)))
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
    stored: &StoredPins,
    pins: &Arc<dyn SecretStore>,
    fallback: Option<&Arc<dyn SecretStore>>,
    held: &Arc<Mutex<Option<Box<dyn Link>>>>,
    feed: &Arc<Mutex<Option<Receiver<Vec<u8>>>>>,
    lines: &Lines,
    handle: String,
    label: String,
) -> Outcome {
    let pin_stored = record_pin(pin, rotated, stored, pins, fallback, lines);
    set_the_clock(&mut *link, lines);
    let gap_name = link.gap_name();
    let info = session::device_info(&mut *link, lines);
    let battery = session::battery(&mut *link, lines);
    follow_battery(&mut *link, lines);
    follow_history(&mut *link, feed, lines);
    follow_faces(&mut *link, lines);
    follow_system_state(&mut *link, lines);
    let face = session::face(&mut *link, lines);
    let status = session::status(&mut *link, lines);
    match held.lock() {
        Ok(mut slot) => *slot = Some(link),
        Err(_) => {
            return Outcome::PairingFailed { label, message: "The link could not be kept.".to_string() };
        }
    }
    Outcome::Paired(Box::new(Paired { handle, label, gap_name, info, battery, face, status, pin_stored }))
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
        fn characteristics(&self) -> Vec<u128> {
            vec![uuids::PASSWORD, uuids::COMMAND_RESULT, uuids::COMMAND]
        }
        fn subscribe(&mut self, _uuid: u128) -> Result<std::sync::mpsc::Receiver<Vec<u8>>, String> {
            let (sender, receiver) = std::sync::mpsc::channel();
            if sender.send(vec![86]).is_err() {
                return Err("closed".into());
            }
            Ok(receiver)
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
                        [0xFF] => *pin = "000000".to_string(),
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

    /// A store that holds nothing and will not take anything, as a Keychain that refuses a write.
    struct RefusingStore;
    impl SecretStore for RefusingStore {
        fn store(&self, _secret: &str) -> Result<bool, String> {
            Err("refused".into())
        }
        fn look_up(&self) -> SecretLookup {
            SecretLookup::Missing
        }
        fn clear(&self) -> Result<(), String> {
            Ok(())
        }
    }

    /// A store that never answers with a secret, as a Keychain waiting on a permission prompt.
    struct UnreadableStore;
    impl SecretStore for UnreadableStore {
        fn store(&self, _secret: &str) -> Result<bool, String> {
            Err("locked".into())
        }
        fn look_up(&self) -> SecretLookup {
            SecretLookup::Unavailable("locked".into())
        }
        fn clear(&self) -> Result<(), String> {
            Err("locked".into())
        }
    }

    /// Drains until no job is outstanding, then a little longer for the notifications a held link forwards on threads
    /// of their own.
    fn settle(device: &Device) {
        for _ in 0..200 {
            std::thread::sleep(Duration::from_millis(25));
            device.drain();
            if device.outstanding.get() == 0 {
                for _ in 0..6 {
                    std::thread::sleep(Duration::from_millis(25));
                    device.drain();
                }
                if device.outstanding.get() == 0 {
                    return;
                }
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
        let notice = Notice::attach(&ui, Rc::new(Trace::none()));
        let data = ui.global::<DeviceData>();

        let without = Device::attach(
            &ui,
            path.clone(),
            Rc::new(Trace::none()),
            Rc::clone(&notice),
            None,
            Arc::new(MemoryStore(Mutex::new(None))),
            Arc::new(facet_core::timezone::SYDNEY),
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
            Arc::new(facet_core::timezone::SYDNEY),
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
        // The charge read at login is replaced by the one the cube sends once the battery is followed.
        assert_eq!(data.get_battery(), "86%");
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

        // A held arrow is a burst of edits: nothing goes while the value moves, and then only where it stopped.
        let before = sent.lock().expect("lock").len();
        for minutes in [6, 7, 8] {
            data.set_auto_pause_minutes(minutes);
            device.edited(DeviceSetting::AutoPause, i64::from(minutes));
        }
        assert_eq!(sent.lock().expect("lock").len(), before, "nothing is sent while the value is moving");
        // The field holds what was edited, whatever else makes the tab redraw, while the table has the old value.
        device.draw();
        assert_eq!(data.get_auto_pause_minutes(), 8);
        assert_eq!(rows::settings(&connection).expect("read").auto_pause_minutes, 5);
        std::thread::sleep(EDIT_QUIET_FOR + Duration::from_millis(200));
        slint::platform::update_timers_and_animations();
        settle(&device);
        let writes: Vec<Vec<u8>> = sent.lock().expect("lock")[before..].to_vec();
        assert_eq!(writes, vec![vec![0x05, 0x00, 0x08]], "one write, at the number the arrow stopped on");
        assert_eq!(rows::settings(&connection).expect("read").auto_pause_minutes, 8);
        assert_eq!(data.get_auto_pause_minutes(), 8);
        // With nothing left unsent the table is the answer again.
        data.set_auto_pause_minutes(3);
        device.draw();
        assert_eq!(data.get_auto_pause_minutes(), 8);

        // A name the cube cannot store is refused before anything is sent; one it can is sent and then recorded.
        device.rename_opened();
        assert!(data.get_is_editing_device_name());
        assert_eq!(data.get_editing_device_name(), "TimeFlip v2.0");
        let before = sent.lock().expect("lock").len();
        device.rename_committed("Cube \u{1F3B2}");
        assert_eq!(notice.title(), "The TimeFlip cannot store that name");
        assert_eq!(sent.lock().expect("lock").len(), before);
        notice.choose(0);
        device.rename_committed("Facet cube");
        settle(&device);
        assert_eq!(sent.lock().expect("lock").last(), Some(&[&[0x15, 10][..], b"Facet cube"].concat()));
        let held = rows::pairing(&connection).expect("read");
        assert_eq!(held.name.as_deref(), Some("Facet cube"));
        assert_eq!(held.previous_name.as_deref(), Some("TimeFlip v2.0"));
        assert_eq!(notice.title(), "The TimeFlip has been renamed");
        notice.choose(0);

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
            Arc::new(facet_core::timezone::SYDNEY),
        );
        assert!(!rows::pairing(&connection).expect("read").is_cube_connected);
        device.open();
        settle(&device);
        assert_eq!(data.get_connection(), "Disconnected");
        device.reconnect();
        settle(&device);
        assert_eq!(data.get_connection(), "Connected");
        assert_eq!(pin.lock().expect("lock").as_str(), before);
        // The fake still reports the name the rename replaced, which does not undo it.
        assert_eq!(rows::pairing(&connection).expect("read").name.as_deref(), Some("Facet cube"));
        drop(device);

        // A store that will not answer stops the reconnect before any PIN is presented.
        let radio: Arc<dyn Radio> = Arc::new(FakeRadio(Arc::clone(&pin), Arc::clone(&sent)));
        let locked = Device::attach(
            &ui,
            path.clone(),
            Rc::new(Trace::none()),
            Rc::clone(&notice),
            Some(radio),
            Arc::new(UnreadableStore),
            Arc::new(facet_core::timezone::SYDNEY),
        );
        locked.open();
        settle(&locked);
        locked.reconnect();
        settle(&locked);
        assert!(data.get_scan_status().contains("stored PIN could not be read"));
        assert_eq!(data.get_connection(), "Disconnected");
        assert!(!rows::pairing(&connection).expect("read").is_cube_connected);
        drop(locked);

        let radio: Arc<dyn Radio> = Arc::new(FakeRadio(Arc::clone(&pin), Arc::clone(&sent)));
        let device = Device::attach(
            &ui,
            path.clone(),
            Rc::new(Trace::none()),
            Rc::clone(&notice),
            Some(radio),
            Arc::clone(&store) as Arc<dyn SecretStore>,
            Arc::new(facet_core::timezone::SYDNEY),
        );
        device.open();
        settle(&device);
        device.reconnect();
        settle(&device);

        // Reset asks first, and Cancel sends nothing; confirmed, the cube is wiped, proved on the factory PIN, and
        // forgotten.
        let before = sent.lock().expect("lock").len();
        device.reset_pressed();
        assert_eq!(notice.title(), "Reset this TimeFlip to factory settings?");
        notice.choose(0);
        assert_eq!(sent.lock().expect("lock").len(), before);
        device.reset_pressed();
        notice.choose(1);
        for _ in 0..300 {
            std::thread::sleep(Duration::from_millis(25));
            device.drain();
            if device.outstanding.get() == 0 {
                break;
            }
        }
        assert_eq!(sent.lock().expect("lock").last(), Some(&vec![0xFF]));
        assert_eq!(pin.lock().expect("lock").as_str(), "000000");
        let held = rows::pairing(&connection).expect("read");
        assert!(!held.is_cube_paired && held.handle.is_none() && held.name.is_none());
        assert_eq!(held.previous_name.as_deref(), Some("Facet cube"));
        assert!(data.get_scan_status().contains("back to factory settings"));
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
            Arc::new(facet_core::timezone::SYDNEY),
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
        drop(device);

        // Nor does pairing: a cube on the vendor PIN is left on it.
        *pin.lock().expect("lock") = "000000".to_string();
        let radio: Arc<dyn Radio> = Arc::new(FakeRadio(Arc::clone(&pin), Arc::clone(&sent)));
        let locked = Device::attach(
            &ui,
            path.clone(),
            Rc::new(Trace::none()),
            Rc::clone(&notice),
            Some(radio),
            Arc::new(UnreadableStore),
            Arc::new(facet_core::timezone::SYDNEY),
        );
        locked.open();
        locked.scan_pressed();
        settle(&locked);
        locked.pair("cube");
        settle(&locked);
        assert!(data.get_scan_status().contains("stored PIN could not be read"));
        assert!(!rows::pairing(&connection).expect("read").is_cube_paired);
        assert_eq!(pin.lock().expect("lock").as_str(), "000000");
        drop(locked);

        // A secret store that refuses the new PIN leaves it in the config file, and the cube is paired all the same.
        let file = Arc::new(MemoryStore(Mutex::new(None)));
        let radio: Arc<dyn Radio> = Arc::new(FakeRadio(Arc::clone(&pin), Arc::clone(&sent)));
        let refused = Device::attach(
            &ui,
            path.clone(),
            Rc::new(Trace::none()),
            Rc::clone(&notice),
            Some(radio),
            Arc::new(RefusingStore),
            Arc::new(facet_core::timezone::SYDNEY),
        );
        refused.set_pin_fallback(Arc::clone(&file) as Arc<dyn SecretStore>);
        refused.open();
        refused.scan_pressed();
        settle(&refused);
        refused.pair("cube");
        settle(&refused);
        assert_eq!(notice.title(), "", "a PIN the config file holds is a PIN saved");
        assert!(rows::pairing(&connection).expect("read").is_cube_paired);
        let rotated = pin.lock().expect("lock").clone();
        assert_ne!(rotated, "000000");
        assert_eq!(file.look_up(), SecretLookup::Found(rotated.clone()), "the config file holds the new PIN");
        drop(refused);

        // A secret store that will not answer no longer stops the reconnect when the config file has the PIN.
        let radio: Arc<dyn Radio> = Arc::new(FakeRadio(Arc::clone(&pin), Arc::clone(&sent)));
        let from_file = Device::attach(
            &ui,
            path.clone(),
            Rc::new(Trace::none()),
            Rc::clone(&notice),
            Some(radio),
            Arc::new(UnreadableStore),
            Arc::new(facet_core::timezone::SYDNEY),
        );
        from_file.set_pin_fallback(Arc::clone(&file) as Arc<dyn SecretStore>);
        from_file.open();
        settle(&from_file);
        from_file.reconnect();
        settle(&from_file);
        assert_eq!(data.get_connection(), "Connected");
        assert_eq!(pin.lock().expect("lock").as_str(), rotated, "nothing was rotated");
        drop(from_file);

        // Once the secret store takes the PIN again, the next login moves it there and the file lets go of it.
        let store = Arc::new(MemoryStore(Mutex::new(None)));
        let radio: Arc<dyn Radio> = Arc::new(FakeRadio(Arc::clone(&pin), Arc::clone(&sent)));
        let healed = Device::attach(
            &ui,
            path.clone(),
            Rc::new(Trace::none()),
            Rc::clone(&notice),
            Some(radio),
            Arc::clone(&store) as Arc<dyn SecretStore>,
            Arc::new(facet_core::timezone::SYDNEY),
        );
        healed.set_pin_fallback(Arc::clone(&file) as Arc<dyn SecretStore>);
        healed.open();
        settle(&healed);
        healed.reconnect();
        settle(&healed);
        assert_eq!(store.look_up(), SecretLookup::Found(rotated.clone()), "the secret store has it now");
        assert_eq!(file.look_up(), SecretLookup::Missing, "and the config file does not");
        drop(healed);

        std::fs::remove_file(&path).expect("the test database should be removable");
    }
}
