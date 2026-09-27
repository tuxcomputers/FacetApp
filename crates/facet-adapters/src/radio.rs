//! [`Radio`] and [`Link`] on `btleplug`, which is CoreBluetooth on macOS. It runs its own Tokio runtime and
//! blocks the calling thread on it, so every call belongs on a background thread.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use btleplug::api::{
    Central, CentralState, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral, PeripheralId};
use facet_core::port::{Advert, Link, Radio, RadioState};
use tokio::runtime::Runtime;

/// How long a single read or write may take.
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(10);
/// How often a scan asks what has been heard.
const SCAN_POLL: Duration = Duration::from_millis(250);

pub struct BtleplugRadio {
    runtime: Arc<Runtime>,
    adapter: Mutex<Option<Adapter>>,
    /// Every device heard, by the handle [`Advert::handle`] gave it, so [`Radio::connect`] can find it again.
    heard: Mutex<HashMap<String, PeripheralId>>,
}

impl BtleplugRadio {
    /// Starts the runtime. The adapter is found on first use. An error says why the runtime would not start.
    pub fn new() -> Result<BtleplugRadio, String> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|error| format!("the Bluetooth runtime would not start: {error}"))?;
        Ok(BtleplugRadio {
            runtime: Arc::new(runtime),
            adapter: Mutex::new(None),
            heard: Mutex::new(HashMap::new()),
        })
    }

    fn adapter(&self) -> Result<Adapter, String> {
        let mut held =
            self.adapter.lock().map_err(|_| "the Bluetooth adapter lock is poisoned".to_string())?;
        if let Some(adapter) = held.as_ref() {
            return Ok(adapter.clone());
        }
        let adapter = self.runtime.block_on(async {
            let manager =
                Manager::new().await.map_err(|error| format!("Bluetooth would not start: {error}"))?;
            let adapters =
                manager.adapters().await.map_err(|error| format!("no Bluetooth adapter: {error}"))?;
            adapters.into_iter().next().ok_or_else(|| "this machine has no Bluetooth adapter".to_string())
        })?;
        *held = Some(adapter.clone());
        Ok(adapter)
    }

    fn remember(&self, handle: String, id: PeripheralId) {
        match self.heard.lock() {
            Ok(mut heard) => {
                heard.insert(handle, id);
            }
            Err(_) => {
                eprintln!("facet: the list of heard devices is poisoned, so {handle} cannot be reached")
            }
        }
    }

    fn find(&self, handle: &str) -> Option<PeripheralId> {
        self.heard.lock().ok()?.get(handle).cloned()
    }
}

fn advert_of(id: &PeripheralId, properties: btleplug::api::PeripheralProperties) -> Advert {
    Advert {
        handle: id.to_string(),
        name: properties.local_name.or(properties.advertisement_name),
        services: properties.services.iter().map(|uuid| uuid.as_u128()).collect(),
        rssi: properties.rssi,
    }
}

impl Radio for BtleplugRadio {
    fn state(&self) -> RadioState {
        let adapter = match self.adapter() {
            Ok(adapter) => adapter,
            Err(reason) => return RadioState::Unavailable(reason),
        };
        match self.runtime.block_on(adapter.adapter_state()) {
            Ok(CentralState::PoweredOn) => RadioState::Ready,
            Ok(CentralState::PoweredOff) => RadioState::Off,
            Ok(CentralState::Unknown) => RadioState::Unauthorised,
            Err(error) => RadioState::Unavailable(error.to_string()),
        }
    }

    fn scan(
        &self,
        duration: Duration,
        stop: &AtomicBool,
        heard: &mut dyn FnMut(&Advert),
    ) -> Result<(), String> {
        let adapter = self.adapter()?;
        let runtime = Arc::clone(&self.runtime);
        runtime.block_on(async {
            adapter
                .start_scan(ScanFilter::default())
                .await
                .map_err(|error| format!("the scan would not start: {error}"))?;
            let deadline = Instant::now() + duration;
            let mut last: HashMap<String, Advert> = HashMap::new();
            let outcome: Result<(), String> = async {
                while Instant::now() < deadline && !stop.load(Ordering::Relaxed) {
                    let peripherals = adapter
                        .peripherals()
                        .await
                        .map_err(|error| format!("the scan could not be read: {error}"))?;
                    for peripheral in peripherals {
                        let Ok(Some(properties)) = peripheral.properties().await else { continue };
                        let advert = advert_of(&peripheral.id(), properties);
                        if last.get(&advert.handle) != Some(&advert) {
                            self.remember(advert.handle.clone(), peripheral.id());
                            heard(&advert);
                            last.insert(advert.handle.clone(), advert);
                        }
                    }
                    tokio::time::sleep(SCAN_POLL).await;
                }
                Ok(())
            }
            .await;
            let stopped =
                adapter.stop_scan().await.map_err(|error| format!("the scan would not stop: {error}"));
            outcome.and(stopped)
        })
    }

    fn connect(&self, handle: &str, timeout: Duration) -> Result<Box<dyn Link>, String> {
        let adapter = self.adapter()?;
        let id = match self.find(handle) {
            Some(id) => id,
            None => {
                let stop = AtomicBool::new(false);
                self.scan(timeout, &stop, &mut |_| {})?;
                self.find(handle).ok_or_else(|| format!("{handle} was not heard"))?
            }
        };
        // **A handle goes stale once its link drops**: CoreBluetooth forgets the peripheral, and connecting again
        // takes a fresh scan to hear it (firmware finding 8). So an unknown peripheral is scanned for once.
        let known = self.runtime.block_on(adapter.peripheral(&id)).is_ok();
        let id = if known {
            id
        } else {
            let stop = AtomicBool::new(false);
            let mut again = None;
            self.scan(timeout, &stop, &mut |advert| {
                if advert.handle == handle {
                    stop.store(true, Ordering::Relaxed);
                    again = Some(());
                }
            })?;
            again.ok_or_else(|| format!("{handle} was not heard again"))?;
            self.find(handle).ok_or_else(|| format!("{handle} was not heard again"))?
        };
        let runtime = Arc::clone(&self.runtime);
        let peripheral = runtime.block_on(async {
            let peripheral = adapter
                .peripheral(&id)
                .await
                .map_err(|error| format!("{handle} is not known to Bluetooth: {error}"))?;
            tokio::time::timeout(timeout, peripheral.connect())
                .await
                .map_err(|_| "the connection timed out".to_string())?
                .map_err(|error| format!("the connection failed: {error}"))?;
            tokio::time::timeout(timeout, peripheral.discover_services())
                .await
                .map_err(|_| "discovering its services timed out".to_string())?
                .map_err(|error| format!("its services could not be discovered: {error}"))?;
            Ok::<Peripheral, String>(peripheral)
        })?;
        let characteristics =
            peripheral.characteristics().into_iter().map(|c| (c.uuid.as_u128(), c)).collect();
        Ok(Box::new(BtleplugLink { runtime, peripheral, characteristics, connected: true }))
    }
}

struct BtleplugLink {
    runtime: Arc<Runtime>,
    peripheral: Peripheral,
    characteristics: HashMap<u128, Characteristic>,
    connected: bool,
}

impl BtleplugLink {
    fn characteristic(&self, uuid: u128) -> Result<&Characteristic, String> {
        self.characteristics.get(&uuid).ok_or_else(|| {
            format!("the device has no characteristic {}", facet_core::device::uuids::name(uuid))
        })
    }
}

impl Link for BtleplugLink {
    fn gap_name(&self) -> Option<String> {
        let properties = self.runtime.block_on(self.peripheral.properties()).ok()??;
        properties.local_name
    }

    fn has_characteristic(&self, uuid: u128) -> bool {
        self.characteristics.contains_key(&uuid)
    }

    fn read(&mut self, uuid: u128) -> Result<Vec<u8>, String> {
        let characteristic = self.characteristic(uuid)?.clone();
        self.runtime.block_on(async {
            tokio::time::timeout(EXCHANGE_TIMEOUT, self.peripheral.read(&characteristic))
                .await
                .map_err(|_| "the read timed out".to_string())?
                .map_err(|error| format!("the read failed: {error}"))
        })
    }

    fn write(&mut self, uuid: u128, bytes: &[u8]) -> Result<(), String> {
        let characteristic = self.characteristic(uuid)?.clone();
        self.runtime.block_on(async {
            tokio::time::timeout(
                EXCHANGE_TIMEOUT,
                self.peripheral.write(&characteristic, bytes, WriteType::WithResponse),
            )
            .await
            .map_err(|_| "the write timed out".to_string())?
            .map_err(|error| format!("the write failed: {error}"))
        })
    }

    fn disconnect(&mut self) -> Result<(), String> {
        if !self.connected {
            return Ok(());
        }
        self.connected = false;
        self.runtime
            .block_on(self.peripheral.disconnect())
            .map_err(|error| format!("the disconnect failed: {error}"))
    }
}

impl Drop for BtleplugLink {
    fn drop(&mut self) {
        if let Err(reason) = self.disconnect() {
            eprintln!("facet: a dropped Bluetooth link would not disconnect: {reason}");
        }
    }
}
