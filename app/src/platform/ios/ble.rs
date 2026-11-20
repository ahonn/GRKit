//! CoreBluetooth central: scan, connect, GATT queue for wake and Wi-Fi credentials.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use dispatch2::DispatchQueue;
use futures_util::future::{Either, select};
use nsh_core::prelude::timer;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{AnyThread, Message, define_class, msg_send};
use objc2_core_bluetooth::{
    CBAdvertisementDataManufacturerDataKey, CBAdvertisementDataServiceUUIDsKey, CBCentralManager,
    CBCentralManagerDelegate, CBCentralManagerScanOptionAllowDuplicatesKey, CBCharacteristic,
    CBCharacteristicWriteType, CBManager, CBManagerAuthorization, CBManagerState, CBPeripheral,
    CBPeripheralDelegate, CBPeripheralState, CBService, CBUUID,
};
use objc2_foundation::{
    NSArray, NSData, NSDictionary, NSError, NSNumber, NSObject, NSObjectProtocol, NSString, NSUUID,
};

use crate::domain::nearby_watch::{Frame, advertised_power};
use crate::domain::uuids::{
    BLUETOOTH_CONTROL_SERVICE, CAMERA_INFORMATION_SERVICE, CAMERA_POWER, CAMERA_SERVICE,
    NETWORK_TYPE, PASSPHRASE, SSID, WLAN_CONTROL_SERVICE,
};
use crate::domain::{
    ApReadyLatch, BleSetup, CameraModel, LinkError, NearbyCamera, WifiCredentials, decode_cred,
    is_ble_pairing_error, is_supported_camera, matches_operation, parse_advertisement, timing,
};

/// ObjC objects touched only on the main queue; marked Send for storage.
struct MainObj<T>(Retained<T>);
// SAFETY: all uses hop to the main queue before touching the object.
unsafe impl<T> Send for MainObj<T> {}
unsafe impl<T> Sync for MainObj<T> {}

#[derive(Clone, Debug)]
enum BleEvent {
    State(CBManagerState),
    Connected(String),
    ConnectFailed {
        peripheral_id: String,
        fault: BleFault,
    },
    Disconnected(String),
    ServicesReady(String),
    CharacteristicsReady {
        peripheral_id: String,
        service: String,
    },
    Value {
        peripheral_id: String,
        characteristic: String,
        data: Vec<u8>,
    },
    Written {
        peripheral_id: String,
        characteristic: String,
    },
    NotifyReady {
        peripheral_id: String,
        characteristic: String,
    },
    Error {
        peripheral_id: String,
        characteristic: Option<String>,
        fault: BleFault,
    },
}

/// What a native Bluetooth failure means for the connect flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BleFault {
    /// Insufficient authentication or encryption: the system pairing has
    /// not completed yet.
    Authentication,
    /// The camera removed this phone's pairing (CoreBluetooth error 14).
    PairingStale,
    /// A required service or characteristic is missing: the camera does not
    /// expose them before it is paired.
    Missing,
    /// Any other native failure.
    Failed,
}

impl BleFault {
    fn of(error: &NSError) -> Self {
        // NSError descriptions are localized; the domains keep ATT and CB codes distinct.
        let domain = error.domain().to_string();
        if !is_ble_pairing_error(&domain, error.code()) {
            BleFault::Failed
        } else if domain == "CBErrorDomain" {
            BleFault::PairingStale
        } else {
            BleFault::Authentication
        }
    }

    fn link_error(self) -> LinkError {
        match self {
            BleFault::Authentication => LinkError::BleAuthentication,
            BleFault::PairingStale => LinkError::BlePairingStale,
            BleFault::Missing => LinkError::NotPaired,
            BleFault::Failed => LinkError::BleConnectFailed,
        }
    }
}

struct Shared {
    peripherals: Mutex<HashMap<String, MainObj<CBPeripheral>>>,
    nearby: Mutex<HashMap<String, NearbyCamera>>,
    nearby_tx: Mutex<Option<async_channel::Sender<Frame>>>,
    /// Hears the id of each peripheral that disconnects (`watch_disconnects`).
    disconnect_tx: Mutex<Option<async_channel::Sender<String>>>,
}

static SHARED: OnceLock<Shared> = OnceLock::new();
static CENTRAL: Mutex<Option<MainObj<CBCentralManager>>> = Mutex::new(None);
static DELEGATE: Mutex<Option<MainObj<BleDelegate>>> = Mutex::new(None);
/// Sender for the in-flight connect operation only. Scanning never touches this.
static EVENT_TX: Mutex<Option<(u64, async_channel::Sender<BleEvent>)>> = Mutex::new(None);
static NEXT_RECEIVER: AtomicU64 = AtomicU64::new(1);
static SCAN_WANTED: AtomicBool = AtomicBool::new(false);
/// Set when the central reports Unsupported, Unauthorized, or PoweredOff.
static BT_UNAVAILABLE: AtomicBool = AtomicBool::new(false);

fn shared() -> &'static Shared {
    SHARED.get_or_init(|| Shared {
        peripherals: Mutex::new(HashMap::new()),
        nearby: Mutex::new(HashMap::new()),
        nearby_tx: Mutex::new(None),
        disconnect_tx: Mutex::new(None),
    })
}

fn short_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

fn record_central_state(state: CBManagerState) {
    let unavailable = matches!(
        state,
        CBManagerState::Unsupported | CBManagerState::Unauthorized | CBManagerState::PoweredOff
    );
    BT_UNAVAILABLE.store(unavailable, Ordering::SeqCst);
    if unavailable {
        log::warn!("BLE central unavailable (state={state:?})");
    }
}

/// True after the central reports Unsupported, Unauthorized, or PoweredOff.
pub fn bluetooth_unavailable() -> bool {
    BT_UNAVAILABLE.load(Ordering::SeqCst)
}

struct BleReceiver {
    events: async_channel::Receiver<BleEvent>,
    ap_ready: ApReadyLatch,
    owner: u64,
    peripheral_id: String,
    succeeded: bool,
}

impl Drop for BleReceiver {
    fn drop(&mut self) {
        let mut sender = EVENT_TX.lock().unwrap();
        if sender
            .as_ref()
            .is_some_and(|(owner, _)| *owner == self.owner)
        {
            *sender = None;
            if !self.succeeded {
                cancel_connection(&self.peripheral_id);
            }
        }
    }
}

fn install_event_receiver(peripheral_id: &str) -> BleReceiver {
    let (tx, events) = async_channel::unbounded();
    let owner = NEXT_RECEIVER.fetch_add(1, Ordering::SeqCst);
    *EVENT_TX.lock().unwrap() = Some((owner, tx));
    BleReceiver {
        events,
        ap_ready: ApReadyLatch::new(peripheral_id),
        owner,
        peripheral_id: peripheral_id.to_owned(),
        succeeded: false,
    }
}

fn emit(event: BleEvent) {
    if let Some((_, tx)) = EVENT_TX.lock().unwrap().as_ref() {
        let _ = tx.try_send(event);
    }
}

/// Cancel both an established BLE link and a pending CoreBluetooth connection.
pub fn cancel_connection(peripheral_id: &str) {
    let id = peripheral_id.to_owned();
    DispatchQueue::main().exec_async(move || {
        with_peripheral(&id, |peripheral| {
            if let Some(MainObj(central)) = CENTRAL.lock().unwrap().as_ref() {
                unsafe { central.cancelPeripheralConnection(peripheral) };
            }
        });
    });
}

struct BleIvars;

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = BleIvars]
    struct BleDelegate;

    unsafe impl NSObjectProtocol for BleDelegate {}

    unsafe impl CBCentralManagerDelegate for BleDelegate {
        #[unsafe(method(centralManagerDidUpdateState:))]
        unsafe fn central_manager_did_update_state(&self, central: &CBCentralManager) {
            let state = unsafe { central.state() };
            app_info!("BLE central state {state:?}");
            record_central_state(state);
            emit(BleEvent::State(state));
            if state == CBManagerState::PoweredOn && SCAN_WANTED.load(Ordering::SeqCst) {
                app_info!("BLE powered on; starting scan");
                begin_scan(central);
            }
        }

        #[unsafe(method(centralManager:didDiscoverPeripheral:advertisementData:RSSI:))]
        unsafe fn did_discover(
            &self,
            _central: &CBCentralManager,
            peripheral: &CBPeripheral,
            advertisement_data: &NSDictionary<NSString, AnyObject>,
            rssi: &NSNumber,
        ) {
            let id = uuid_string(peripheral);
            let (manufacturer, services) = parse_adv_dict(advertisement_data);
            let adv = parse_advertisement(manufacturer.as_deref(), &services);
            if !is_supported_camera(&adv) {
                return;
            }
            let Some(model) = adv.model else {
                return;
            };
            let rssi = (rssi.intValue() != 127).then(|| rssi.intValue());
            let name = unsafe { peripheral.name() }
                .map(|n| n.to_string())
                .filter(|n| !n.is_empty());
            let nearby = NearbyCamera {
                peripheral_id: id.clone(),
                model,
                powered_on: adv.shows_powered_on(),
                name: name.clone(),
                rssi,
            };
            shared()
                .peripherals
                .lock()
                .unwrap()
                .insert(id.clone(), MainObj(peripheral.retain()));
            let first = shared()
                .nearby
                .lock()
                .unwrap()
                .insert(id.clone(), nearby)
                .is_none();
            if first {
                app_info!("BLE discovered camera {}", short_id(&id));
            }
            // Every frame goes to the nearby watch, which owns freshness.
            if let Some(tx) = shared().nearby_tx.lock().unwrap().as_ref() {
                let _ = tx.try_send(Frame {
                    peripheral_id: id,
                    model,
                    power: advertised_power(adv.powered_on, adv.advertises_camera_information),
                    name,
                    rssi,
                });
            }
        }

        #[unsafe(method(centralManager:didConnectPeripheral:))]
        unsafe fn did_connect(&self, _central: &CBCentralManager, peripheral: &CBPeripheral) {
            let id = uuid_string(peripheral);
            unsafe {
                peripheral.setDelegate(Some(ProtocolObject::from_ref(self)));
            }
            app_info!("BLE connected {}", short_id(&id));
            emit(BleEvent::Connected(id));
        }

        #[unsafe(method(centralManager:didFailToConnectPeripheral:error:))]
        unsafe fn did_fail_connect(
            &self,
            _central: &CBCentralManager,
            peripheral: &CBPeripheral,
            error: Option<&NSError>,
        ) {
            let id = uuid_string(peripheral);
            let message = error
                .map(|e| e.localizedDescription().to_string())
                .unwrap_or_else(|| "connect failed".into());
            log::warn!("BLE connect failed {}: {message}", short_id(&id));
            emit(BleEvent::ConnectFailed {
                peripheral_id: id,
                fault: error.map_or(BleFault::Failed, BleFault::of),
            });
        }

        #[unsafe(method(centralManager:didDisconnectPeripheral:error:))]
        unsafe fn did_disconnect(
            &self,
            _central: &CBCentralManager,
            peripheral: &CBPeripheral,
            _error: Option<&NSError>,
        ) {
            let id = uuid_string(peripheral);
            log::warn!("BLE disconnected {}", short_id(&id));
            if let Some(tx) = shared().disconnect_tx.lock().unwrap().as_ref() {
                let _ = tx.try_send(id.clone());
            }
            emit(BleEvent::Disconnected(id));
        }
    }

    unsafe impl CBPeripheralDelegate for BleDelegate {
        #[unsafe(method(peripheral:didDiscoverServices:))]
        unsafe fn did_discover_services(&self, peripheral: &CBPeripheral, error: Option<&NSError>) {
            let id = uuid_string(peripheral);
            if let Some(err) = error {
                emit_native_gatt_error(&id, None, err);
                return;
            }
            emit(BleEvent::ServicesReady(id));
        }

        #[unsafe(method(peripheral:didDiscoverCharacteristicsForService:error:))]
        unsafe fn did_discover_characteristics(
            &self,
            peripheral: &CBPeripheral,
            service: &CBService,
            error: Option<&NSError>,
        ) {
            let id = uuid_string(peripheral);
            if let Some(err) = error {
                emit_native_gatt_error(&id, None, err);
                return;
            }
            emit(BleEvent::CharacteristicsReady {
                peripheral_id: id,
                service: unsafe { service.UUID().UUIDString().to_string() },
            });
        }

        #[unsafe(method(peripheral:didUpdateValueForCharacteristic:error:))]
        unsafe fn did_update_value(
            &self,
            peripheral: &CBPeripheral,
            characteristic: &CBCharacteristic,
            error: Option<&NSError>,
        ) {
            let id = uuid_string(peripheral);
            let char_uuid = unsafe { characteristic.UUID().UUIDString().to_string() };
            if let Some(err) = error {
                emit_native_gatt_error(&id, Some(&char_uuid), err);
                return;
            }
            let data = unsafe { characteristic.value() }
                .map(|d| d.to_vec())
                .unwrap_or_default();
            emit(BleEvent::Value {
                peripheral_id: id,
                characteristic: char_uuid,
                data,
            });
        }

        #[unsafe(method(peripheral:didWriteValueForCharacteristic:error:))]
        unsafe fn did_write_value(
            &self,
            peripheral: &CBPeripheral,
            characteristic: &CBCharacteristic,
            error: Option<&NSError>,
        ) {
            let id = uuid_string(peripheral);
            let char_uuid = unsafe { characteristic.UUID().UUIDString().to_string() };
            if let Some(err) = error {
                emit_native_gatt_error(&id, Some(&char_uuid), err);
                return;
            }
            emit(BleEvent::Written {
                peripheral_id: id,
                characteristic: char_uuid,
            });
        }

        #[unsafe(method(peripheral:didUpdateNotificationStateForCharacteristic:error:))]
        unsafe fn did_update_notify(
            &self,
            peripheral: &CBPeripheral,
            characteristic: &CBCharacteristic,
            error: Option<&NSError>,
        ) {
            let id = uuid_string(peripheral);
            let char_uuid = unsafe { characteristic.UUID().UUIDString().to_string() };
            if let Some(err) = error {
                emit_native_gatt_error(&id, Some(&char_uuid), err);
                return;
            }
            emit(BleEvent::NotifyReady {
                peripheral_id: id,
                characteristic: char_uuid,
            });
        }
    }
);

fn emit_native_gatt_error(peripheral_id: &str, characteristic: Option<&str>, error: &NSError) {
    emit_ble_error(
        peripheral_id,
        characteristic,
        &error.localizedDescription().to_string(),
        BleFault::of(error),
    );
}

fn emit_missing(peripheral_id: &str, characteristic: Option<&str>, message: &str) {
    emit_ble_error(peripheral_id, characteristic, message, BleFault::Missing);
}

fn emit_ble_error(
    peripheral_id: &str,
    characteristic: Option<&str>,
    message: &str,
    fault: BleFault,
) {
    log::warn!(
        "BLE GATT error {}{}: {message}",
        short_id(peripheral_id),
        characteristic.map(|c| format!("/{c}")).unwrap_or_default()
    );
    emit(BleEvent::Error {
        peripheral_id: peripheral_id.to_owned(),
        characteristic: characteristic.map(str::to_owned),
        fault,
    });
}

impl BleDelegate {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(BleIvars);
        unsafe { msg_send![super(this), init] }
    }
}

fn uuid_string(peripheral: &CBPeripheral) -> String {
    unsafe { peripheral.identifier().UUIDString().to_string() }
}

fn parse_adv_dict(
    advertisement_data: &NSDictionary<NSString, AnyObject>,
) -> (Option<Vec<u8>>, Vec<String>) {
    let manufacturer = unsafe {
        advertisement_data
            .objectForKey(CBAdvertisementDataManufacturerDataKey)
            .and_then(|obj| obj.downcast_ref::<NSData>().map(|d| d.to_vec()))
    };
    let services = unsafe {
        advertisement_data
            .objectForKey(CBAdvertisementDataServiceUUIDsKey)
            .and_then(|obj| obj.downcast_ref::<NSArray>().map(|arr| arr.retain()))
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        item.downcast_ref::<CBUUID>()
                            .map(|u| u.UUIDString().to_string())
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    (manufacturer, services)
}

fn ensure_central() {
    let mut slot = CENTRAL.lock().unwrap();
    if slot.is_some() {
        return;
    }
    let delegate = BleDelegate::new();
    let central = unsafe {
        CBCentralManager::initWithDelegate_queue(
            CBCentralManager::alloc(),
            Some(ProtocolObject::from_ref(&*delegate)),
            Some(DispatchQueue::main()),
        )
    };
    *DELEGATE.lock().unwrap() = Some(MainObj(delegate));
    *slot = Some(MainObj(central));
}

fn begin_scan(central: &CBCentralManager) {
    app_info!("BLE scan start");
    let s1 = unsafe { CBUUID::UUIDWithString(&NSString::from_str(CAMERA_INFORMATION_SERVICE)) };
    let s2 = unsafe { CBUUID::UUIDWithString(&NSString::from_str(BLUETOOTH_CONTROL_SERVICE)) };
    let services = NSArray::from_retained_slice(&[s1, s2]);
    unsafe {
        let allow_duplicates = NSNumber::new_bool(true);
        let options = NSDictionary::from_slices(
            &[CBCentralManagerScanOptionAllowDuplicatesKey],
            &[&*allow_duplicates as &AnyObject],
        );
        central.scanForPeripheralsWithServices_options(Some(&services), Some(&options));
    }
}

fn stop_radio_scan() {
    DispatchQueue::main().exec_async(|| {
        if let Some(MainObj(central)) = CENTRAL.lock().unwrap().as_ref() {
            unsafe { central.stopScan() };
        }
    });
}

/// Start scanning and publish nearby camera snapshots on `tx`.
/// Optionally primes a remembered peripheral via retrieve (not “nearby” until it advertises).
/// Creating the central shows the permission request, so a scan without
/// `ask` waits until the user has answered it.
pub fn start_scanning(tx: async_channel::Sender<Frame>, remembered_id: Option<&str>, ask: bool) {
    *shared().nearby_tx.lock().unwrap() = Some(tx);
    SCAN_WANTED.store(true, Ordering::SeqCst);
    let remembered = remembered_id.map(str::to_owned);
    DispatchQueue::main().exec_async(move || {
        let undecided =
            unsafe { CBManager::authorization_class() } == CBManagerAuthorization::NotDetermined;
        if !ask && undecided && CENTRAL.lock().unwrap().is_none() {
            app_info!("BLE scan waits for the user to allow Bluetooth");
            return;
        }
        ensure_central();
        if let Some(id) = remembered.as_deref() {
            retrieve_remembered(id);
        }
        let guard = CENTRAL.lock().unwrap();
        let Some(MainObj(central)) = guard.as_ref() else {
            return;
        };
        if unsafe { central.state() } == CBManagerState::PoweredOn {
            begin_scan(central);
        } else {
            app_info!("BLE scan wanted; waiting for PoweredOn");
        }
    });
}

fn retrieve_remembered(peripheral_id: &str) {
    let central_guard = CENTRAL.lock().unwrap();
    let Some(MainObj(central)) = central_guard.as_ref() else {
        return;
    };
    let Some(uuid) =
        NSUUID::initWithUUIDString(NSUUID::alloc(), &NSString::from_str(peripheral_id))
    else {
        log::warn!(
            "BLE remembered id is not a UUID: {}",
            short_id(peripheral_id)
        );
        return;
    };
    let ids = NSArray::from_retained_slice(&[uuid]);
    let peripherals = unsafe { central.retrievePeripheralsWithIdentifiers(&ids) };
    drop(central_guard);
    for peripheral in peripherals {
        let id = uuid_string(&peripheral);
        app_info!("BLE retrieved remembered peripheral {}", short_id(&id));
        shared()
            .peripherals
            .lock()
            .unwrap()
            .insert(id, MainObj(peripheral));
    }
}

/// Pause the radio scan during a connect without dropping the nearby publisher.
pub fn pause_scanning() {
    SCAN_WANTED.store(false, Ordering::SeqCst);
    stop_radio_scan();
}

/// Resume scanning with the same nearby publisher after a connect finishes.
pub fn resume_scanning() {
    if shared().nearby_tx.lock().unwrap().is_none() {
        return;
    }
    SCAN_WANTED.store(true, Ordering::SeqCst);
    DispatchQueue::main().exec_async(|| {
        ensure_central();
        let guard = CENTRAL.lock().unwrap();
        let Some(MainObj(central)) = guard.as_ref() else {
            return;
        };
        if unsafe { central.state() } == CBManagerState::PoweredOn {
            app_info!("BLE resuming scan");
            begin_scan(central);
        }
    });
}

/// The camera session ended: drop the link the connect kept, so the camera
/// advertises again and the nearby watch sees it power off and on.
pub fn release_link(peripheral_id: &str) {
    cancel_connection(peripheral_id);
}

pub fn stop_scanning() {
    SCAN_WANTED.store(false, Ordering::SeqCst);
    *shared().nearby_tx.lock().unwrap() = None;
    stop_radio_scan();
}

/// Connect, wake, start AP, and read Wi-Fi credentials. `connected` is set
/// once the Bluetooth link is up, so a cancelled attempt knows to close the
/// access point.
/// `powered_on` comes from a fresh advertisement: only then is the wake
/// write skipped. `budget` is the connect timeout and how many timeouts
/// are retried.
pub async fn ble_setup(
    peripheral_id: &str,
    model_hint: CameraModel,
    powered_on: bool,
    budget: (Duration, u32),
    connected: &Cell<bool>,
) -> Result<BleSetup, LinkError> {
    app_info!("BLE setup start {}", short_id(peripheral_id));
    let mut rx = install_event_receiver(peripheral_id);
    let result = ble_setup_inner(
        peripheral_id,
        model_hint,
        powered_on,
        budget,
        connected,
        &mut rx,
    )
    .await;
    rx.succeeded = result.is_ok();
    match &result {
        Ok(_) => app_info!("BLE setup ok {}", short_id(peripheral_id)),
        Err(e) => log::warn!("BLE setup failed {}: {e:?}", short_id(peripheral_id)),
    }
    result
}

async fn ble_setup_inner(
    peripheral_id: &str,
    model_hint: CameraModel,
    powered_on_already: bool,
    (connect_timeout, timeout_retries): (Duration, u32),
    connected: &Cell<bool>,
    rx: &mut BleReceiver,
) -> Result<BleSetup, LinkError> {
    ensure_central_async(rx).await?;

    pause_scanning();
    timer(timing::SCAN_CONNECT_SETTLE).await;

    let model = shared()
        .nearby
        .lock()
        .unwrap()
        .get(peripheral_id)
        .map(|n| n.model)
        .unwrap_or(model_hint);

    connect_with_retries(peripheral_id, connect_timeout, timeout_retries, rx).await?;
    connected.set(true);

    app_info!("BLE reading Wi-Fi credentials {}", short_id(peripheral_id));
    let ssid = decode_cred(&gatt_read(peripheral_id, SSID, rx).await?);
    let passphrase = decode_cred(&gatt_read(peripheral_id, PASSPHRASE, rx).await?);
    if ssid.is_empty() || passphrase.is_empty() {
        // The camera hides its network until this phone is paired.
        log::warn!("BLE Wi-Fi credentials empty {}", short_id(peripheral_id));
        return Err(LinkError::NotPaired);
    }

    if !powered_on_already {
        app_info!("BLE waking camera {}", short_id(peripheral_id));
        let _ = gatt_write(peripheral_id, CAMERA_POWER, &[0x01], rx).await;
    }

    app_info!("BLE starting access point {}", short_id(peripheral_id));
    let ap_start = async {
        gatt_subscribe(peripheral_id, NETWORK_TYPE, rx).await?;
        let mut ap_ready = false;
        for attempt in 0..timing::NETWORK_TYPE_NOTIFY_ATTEMPTS {
            rx.ap_ready.reset();
            gatt_write(peripheral_id, NETWORK_TYPE, &[0x01], rx).await?;
            if wait_notify_value(
                peripheral_id,
                NETWORK_TYPE,
                0x01,
                timing::NETWORK_TYPE_NOTIFY_TIMEOUT,
                rx,
            )
            .await?
            {
                ap_ready = true;
                break;
            }
            if attempt + 1 < timing::NETWORK_TYPE_NOTIFY_ATTEMPTS {
                timer(timing::NETWORK_TYPE_RETRY_DELAY).await;
            }
        }
        Ok::<bool, LinkError>(ap_ready)
    }
    .await;
    let ap_ready = match ap_start {
        Ok(ready) => ready,
        Err(error) => {
            // Some camera firmware drops BLE while bringing up its AP. The
            // credentials are already available; HTTP will verify the link.
            log::warn!("BLE AP start failed; continuing to Wi-Fi: {error:?}");
            false
        }
    };
    let id = peripheral_id.to_owned();
    DispatchQueue::main().exec_async(move || {
        with_peripheral(&id, |peripheral| {
            if let Some(characteristic) = find_characteristic(peripheral, NETWORK_TYPE) {
                unsafe { peripheral.setNotifyValue_forCharacteristic(false, &characteristic) };
            }
        });
    });
    // The shared flow waits out the learned access point warm-up.
    Ok(BleSetup {
        peripheral_id: peripheral_id.to_owned(),
        model,
        powered_on_already,
        credentials: WifiCredentials { ssid, passphrase },
        ap_ready,
    })
}

async fn ensure_central_async(rx: &mut BleReceiver) -> Result<(), LinkError> {
    DispatchQueue::main().exec_async(|| {
        ensure_central();
        if let Some(MainObj(central)) = CENTRAL.lock().unwrap().as_ref() {
            emit(BleEvent::State(unsafe { central.state() }));
        }
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        if std::time::Instant::now() > deadline {
            return Err(LinkError::BluetoothUnavailable);
        }
        match recv_timeout(rx, Duration::from_millis(200)).await {
            Some(BleEvent::State(CBManagerState::PoweredOn)) => return Ok(()),
            Some(BleEvent::State(CBManagerState::Unauthorized))
            | Some(BleEvent::State(CBManagerState::Unsupported))
            | Some(BleEvent::State(CBManagerState::PoweredOff)) => {
                return Err(LinkError::BluetoothUnavailable);
            }
            _ => {}
        }
    }
}

/// Connect and discover services. A failed connect or an incomplete
/// authentication is retried up to `BLE_CONNECT_ATTEMPTS` times; a connect
/// that times out after `timeout` only `timeout_retries` times. Other errors
/// end at once.
async fn connect_with_retries(
    peripheral_id: &str,
    timeout: Duration,
    timeout_retries: u32,
    rx: &mut BleReceiver,
) -> Result<(), LinkError> {
    let mut timeouts = 0;
    let mut attempt = 1;
    loop {
        let result = match connect_once(peripheral_id, timeout, rx).await {
            Ok(()) => discover_all(peripheral_id, rx).await,
            Err(error) => Err(error),
        };
        let error = match result {
            Ok(()) => return Ok(()),
            Err(error) => error,
        };
        let timeout_retry = error == LinkError::BleTimeout && timeouts < timeout_retries;
        let retryable = matches!(
            error,
            LinkError::BleAuthentication | LinkError::BleConnectFailed
        );
        if !(retryable || timeout_retry) || attempt == timing::BLE_CONNECT_ATTEMPTS {
            return Err(error);
        }
        if timeout_retry {
            timeouts += 1;
        }
        log::warn!(
            "BLE connect attempt {attempt} failed {}: {error:?}",
            short_id(peripheral_id)
        );
        // A pending connection left in CoreBluetooth would race the retry.
        cancel_connection(peripheral_id);
        timer(timing::BLE_CONNECT_RETRY_DELAY).await;
        while rx.events.try_recv().is_ok() {}
        attempt += 1;
    }
}

async fn connect_once(
    peripheral_id: &str,
    timeout: Duration,
    rx: &mut BleReceiver,
) -> Result<(), LinkError> {
    if !shared()
        .peripherals
        .lock()
        .unwrap()
        .contains_key(peripheral_id)
    {
        return Err(LinkError::NotNearby);
    }
    // Drop callbacks from a connection cancelled during the scan handoff.
    while rx.events.try_recv().is_ok() {}
    let id = peripheral_id.to_owned();
    let id_for_main = id.clone();
    DispatchQueue::main().exec_async(move || {
        let peripherals = shared().peripherals.lock().unwrap();
        let Some(MainObj(peripheral)) = peripherals.get(&id_for_main) else {
            return;
        };
        let central = CENTRAL.lock().unwrap();
        let Some(MainObj(central)) = central.as_ref() else {
            return;
        };
        if unsafe { peripheral.state() } == CBPeripheralState::Connected {
            if let Some(MainObj(delegate)) = DELEGATE.lock().unwrap().as_ref() {
                unsafe { peripheral.setDelegate(Some(ProtocolObject::from_ref(&**delegate))) };
            }
            emit(BleEvent::Connected(id_for_main.clone()));
        } else if unsafe { peripheral.state() } == CBPeripheralState::Disconnecting {
            emit(BleEvent::ConnectFailed {
                peripheral_id: id_for_main.clone(),
                fault: BleFault::Failed,
            });
        } else {
            unsafe { central.connectPeripheral_options(peripheral, None) };
        }
    });
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        match recv_timeout(rx, Duration::from_millis(100)).await {
            Some(BleEvent::Connected(cid)) if cid.eq_ignore_ascii_case(&id) => return Ok(()),
            Some(BleEvent::ConnectFailed {
                peripheral_id: cid,
                fault,
            }) if cid.eq_ignore_ascii_case(&id) => return Err(fault.link_error()),
            // Not connected yet: a drop here is a failed connect.
            Some(BleEvent::Disconnected(cid)) if cid.eq_ignore_ascii_case(&id) => {
                return Err(LinkError::BleConnectFailed);
            }
            Some(BleEvent::Error {
                peripheral_id: cid,
                fault,
                ..
            }) if cid.eq_ignore_ascii_case(&id) => return Err(fault.link_error()),
            _ => {}
        }
    }
    cancel_connection(peripheral_id);
    Err(LinkError::BleTimeout)
}

async fn discover_all(peripheral_id: &str, rx: &mut BleReceiver) -> Result<(), LinkError> {
    let id = peripheral_id.to_owned();
    DispatchQueue::main().exec_async(move || {
        with_peripheral(&id, |peripheral| unsafe {
            peripheral.discoverServices(None);
        });
    });
    wait_for(
        rx,
        peripheral_id,
        None,
        timing::GATT_TIMEOUT,
        |e| matches!(e, BleEvent::ServicesReady(sid) if sid.eq_ignore_ascii_case(peripheral_id)),
    )
    .await?;

    for service_uuid in [CAMERA_SERVICE, WLAN_CONTROL_SERVICE] {
        let id = peripheral_id.to_owned();
        let service_uuid_owned = service_uuid.to_owned();
        DispatchQueue::main().exec_async(move || {
            with_peripheral(&id, |peripheral| {
                let services = unsafe { peripheral.services() }
                    .unwrap_or_else(|| NSArray::from_retained_slice(&[]));
                for service in services {
                    let uuid = unsafe { service.UUID().UUIDString().to_string() };
                    if uuid.eq_ignore_ascii_case(&service_uuid_owned) {
                        unsafe { peripheral.discoverCharacteristics_forService(None, &service) };
                        return;
                    }
                }
                emit_missing(&id, None, "missing required service");
            });
        });
        wait_for(rx, peripheral_id, None, timing::GATT_TIMEOUT, |e| {
            matches!(
                e,
                BleEvent::CharacteristicsReady {
                    peripheral_id: pid,
                    service
                } if pid.eq_ignore_ascii_case(peripheral_id)
                    && service.eq_ignore_ascii_case(service_uuid)
            )
        })
        .await?;
    }
    let id = peripheral_id.to_owned();
    DispatchQueue::main().exec_async(move || {
        with_peripheral(&id, |peripheral| {
            let services = unsafe { peripheral.services() }
                .unwrap_or_else(|| NSArray::from_retained_slice(&[]));
            for service in services {
                let uuid = unsafe { service.UUID().UUIDString().to_string() };
                let chars: Vec<String> = unsafe { service.characteristics() }
                    .map(|list| {
                        list.iter()
                            .map(|c| unsafe { c.UUID().UUIDString().to_string() })
                            .collect()
                    })
                    .unwrap_or_default();
                app_info!("BLE service {uuid} characteristics={chars:?}");
            }
        });
    });
    Ok(())
}

async fn gatt_read(
    peripheral_id: &str,
    char_uuid: &str,
    rx: &mut BleReceiver,
) -> Result<Vec<u8>, LinkError> {
    let id = peripheral_id.to_owned();
    let char_uuid_owned = char_uuid.to_owned();
    DispatchQueue::main().exec_async(move || {
        with_peripheral(&id, |peripheral| {
            if let Some(ch) = find_characteristic(peripheral, &char_uuid_owned) {
                unsafe { peripheral.readValueForCharacteristic(&ch) };
            } else {
                emit_missing(&id, Some(&char_uuid_owned), "missing characteristic");
            }
        });
    });
    let char_uuid = char_uuid.to_owned();
    let id = peripheral_id.to_owned();
    let deadline = std::time::Instant::now() + timing::GATT_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(LinkError::BleTimeout);
        }
        match recv_timeout(rx, remaining).await {
            Some(BleEvent::Value {
                peripheral_id: pid,
                characteristic,
                data,
            }) if matches_operation(&pid, Some(&characteristic), &id, Some(&char_uuid)) => {
                return Ok(data);
            }
            Some(BleEvent::Disconnected(cid)) if cid.eq_ignore_ascii_case(&id) => {
                return Err(LinkError::BleDisconnected);
            }
            Some(BleEvent::Error {
                peripheral_id: pid,
                characteristic,
                fault,
            }) if matches_operation(&pid, characteristic.as_deref(), &id, Some(&char_uuid)) => {
                return Err(fault.link_error());
            }
            None => return Err(LinkError::ConnectionLost),
            _ => {}
        }
    }
}

async fn gatt_write(
    peripheral_id: &str,
    char_uuid: &str,
    value: &[u8],
    rx: &mut BleReceiver,
) -> Result<(), LinkError> {
    let id = peripheral_id.to_owned();
    let char_uuid_owned = char_uuid.to_owned();
    let bytes = value.to_vec();
    DispatchQueue::main().exec_async(move || {
        with_peripheral(&id, |peripheral| {
            if let Some(ch) = find_characteristic(peripheral, &char_uuid_owned) {
                let data = NSData::with_bytes(&bytes);
                unsafe {
                    peripheral.writeValue_forCharacteristic_type(
                        &data,
                        &ch,
                        CBCharacteristicWriteType::WithResponse,
                    );
                }
            } else {
                emit_missing(&id, Some(&char_uuid_owned), "missing characteristic");
            }
        });
    });
    let char_uuid = char_uuid.to_owned();
    let id = peripheral_id.to_owned();
    let deadline = std::time::Instant::now() + timing::GATT_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(LinkError::BleTimeout);
        }
        match recv_timeout(rx, remaining).await {
            Some(BleEvent::Written {
                peripheral_id: pid,
                characteristic,
            }) if matches_operation(&pid, Some(&characteristic), &id, Some(&char_uuid)) => {
                return Ok(());
            }
            Some(BleEvent::Disconnected(cid)) if cid.eq_ignore_ascii_case(&id) => {
                return Err(LinkError::BleDisconnected);
            }
            Some(BleEvent::Error {
                peripheral_id: pid,
                characteristic,
                fault,
            }) if matches_operation(&pid, characteristic.as_deref(), &id, Some(&char_uuid)) => {
                return Err(fault.link_error());
            }
            None => return Err(LinkError::ConnectionLost),
            _ => {}
        }
    }
}

async fn gatt_subscribe(
    peripheral_id: &str,
    char_uuid: &str,
    rx: &mut BleReceiver,
) -> Result<(), LinkError> {
    let id = peripheral_id.to_owned();
    let char_uuid_owned = char_uuid.to_owned();
    DispatchQueue::main().exec_async(move || {
        with_peripheral(&id, |peripheral| {
            if let Some(ch) = find_characteristic(peripheral, &char_uuid_owned) {
                unsafe { peripheral.setNotifyValue_forCharacteristic(true, &ch) };
            } else {
                emit_missing(&id, Some(&char_uuid_owned), "missing characteristic");
            }
        });
    });
    let char_uuid = char_uuid.to_owned();
    let id = peripheral_id.to_owned();
    let deadline = std::time::Instant::now() + timing::GATT_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(LinkError::BleTimeout);
        }
        match recv_timeout(rx, remaining).await {
            Some(BleEvent::NotifyReady {
                peripheral_id: pid,
                characteristic,
            }) if matches_operation(&pid, Some(&characteristic), &id, Some(&char_uuid)) => {
                return Ok(());
            }
            Some(BleEvent::Disconnected(cid)) if cid.eq_ignore_ascii_case(&id) => {
                return Err(LinkError::BleDisconnected);
            }
            Some(BleEvent::Error {
                peripheral_id: pid,
                characteristic,
                fault,
            }) if matches_operation(&pid, characteristic.as_deref(), &id, Some(&char_uuid)) => {
                return Err(fault.link_error());
            }
            None => return Err(LinkError::ConnectionLost),
            _ => {}
        }
    }
}

async fn wait_notify_value(
    peripheral_id: &str,
    char_uuid: &str,
    want: u8,
    timeout: Duration,
    rx: &mut BleReceiver,
) -> Result<bool, LinkError> {
    let id = peripheral_id.to_owned();
    let char_uuid = char_uuid.to_owned();
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if rx.ap_ready.is_ready() {
            return Ok(true);
        }
        match recv_timeout(rx, Duration::from_millis(100)).await {
            Some(BleEvent::Value {
                peripheral_id: pid,
                characteristic,
                data,
            }) if matches_operation(&pid, Some(&characteristic), &id, Some(&char_uuid))
                && data.first() == Some(&want) =>
            {
                return Ok(true);
            }
            Some(BleEvent::Disconnected(cid)) if cid.eq_ignore_ascii_case(&id) => {
                return Err(LinkError::BleDisconnected);
            }
            _ => {}
        }
    }
    Ok(false)
}

fn with_peripheral(id: &str, f: impl FnOnce(&CBPeripheral)) {
    let peripherals = shared().peripherals.lock().unwrap();
    if let Some(MainObj(peripheral)) = peripherals.get(id) {
        f(peripheral);
    }
}

fn find_characteristic(
    peripheral: &CBPeripheral,
    uuid: &str,
) -> Option<Retained<CBCharacteristic>> {
    let services = unsafe { peripheral.services()? };
    for service in services {
        // A service whose characteristics were not discovered has none:
        // skip it, the characteristic may be in a later service.
        let Some(chars) = (unsafe { service.characteristics() }) else {
            continue;
        };
        for ch in chars {
            let id = unsafe { ch.UUID().UUIDString().to_string() };
            if id.eq_ignore_ascii_case(uuid) {
                return Some(ch);
            }
        }
    }
    None
}

async fn wait_for(
    rx: &mut BleReceiver,
    peripheral_id: &str,
    characteristic: Option<&str>,
    timeout: Duration,
    pred: impl Fn(&BleEvent) -> bool,
) -> Result<(), LinkError> {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        match recv_timeout(rx, Duration::from_millis(100)).await {
            Some(e) if pred(&e) => return Ok(()),
            Some(BleEvent::Disconnected(cid)) if cid.eq_ignore_ascii_case(peripheral_id) => {
                return Err(LinkError::BleDisconnected);
            }
            Some(BleEvent::Error {
                peripheral_id: pid,
                characteristic: ch,
                fault,
            }) if matches_operation(&pid, ch.as_deref(), peripheral_id, characteristic) => {
                return Err(fault.link_error());
            }
            _ => {}
        }
    }
    Err(LinkError::BleTimeout)
}

async fn recv_timeout(rx: &mut BleReceiver, timeout: Duration) -> Option<BleEvent> {
    match select(
        std::pin::pin!(rx.events.recv()),
        std::pin::pin!(timer(timeout)),
    )
    .await
    {
        Either::Left((Ok(ev), _)) => {
            if let BleEvent::Value {
                peripheral_id,
                characteristic,
                data,
            } = &ev
            {
                rx.ap_ready.observe(peripheral_id, characteristic, data);
            }
            Some(ev)
        }
        _ => None,
    }
}

/// Taking the link back for one write on a camera that is already on.
const TEARDOWN_RECONNECT_TIMEOUT: Duration = Duration::from_secs(6);
/// One teardown command; a wedged CoreBluetooth call must not hold it.
const TEARDOWN_COMMAND_TIMEOUT: Duration = Duration::from_secs(2);
/// A power off write; the camera may answer late while it shuts down.
const POWER_OFF_COMMAND_TIMEOUT: Duration = Duration::from_secs(8);

/// After the user cancels a connect: turn the camera access point off
/// (Network Type 0x00), then drop the Bluetooth link. The link is taken
/// back when it already dropped. Each step is bounded, and the teardown
/// stops as soon as `owns` reports that a newer attempt uses the camera.
pub async fn close_access_point(peripheral_id: &str, owns: impl Fn() -> bool) {
    if !owns() {
        return;
    }
    let mut rx = install_event_receiver(peripheral_id);
    let stopped = async {
        crate::platform::with_timeout(
            ensure_link(peripheral_id, &mut rx),
            TEARDOWN_RECONNECT_TIMEOUT,
        )
        .await
        .unwrap_or(Err(LinkError::BleTimeout))?;
        if !owns() {
            return Err(LinkError::Cancelled);
        }
        crate::platform::with_timeout(
            gatt_write(peripheral_id, NETWORK_TYPE, &[0x00], &mut rx),
            TEARDOWN_COMMAND_TIMEOUT,
        )
        .await
        .unwrap_or(Err(LinkError::BleTimeout))
    }
    .await;
    match stopped {
        Ok(()) => app_info!("BLE access point closed {}", short_id(peripheral_id)),
        Err(error) => app_info!(
            "BLE access point close skipped {}: {error:?}",
            short_id(peripheral_id)
        ),
    }
    // Dropping the receiver cancels the link, unless a newer attempt owns it.
    rx.succeeded = !owns();
}

/// The user disconnects or turns the camera off: write NETWORK_TYPE 0 so it
/// closes its access point, or CAMERA_POWER 0 so it turns off. A power off
/// may drop the link before the write is answered; that counts as done.
pub async fn end_session(peripheral_id: &str, power_off: bool) -> bool {
    let mut rx = install_event_receiver(peripheral_id);
    let (characteristic, budget) = if power_off {
        (CAMERA_POWER, POWER_OFF_COMMAND_TIMEOUT)
    } else {
        (NETWORK_TYPE, TEARDOWN_COMMAND_TIMEOUT)
    };
    let result = async {
        crate::platform::with_timeout(
            ensure_link(peripheral_id, &mut rx),
            TEARDOWN_RECONNECT_TIMEOUT,
        )
        .await
        .unwrap_or(Err(LinkError::BleTimeout))?;
        crate::platform::with_timeout(
            gatt_write(peripheral_id, characteristic, &[0x00], &mut rx),
            budget,
        )
        .await
        .unwrap_or(Err(LinkError::BleTimeout))
    }
    .await;
    match &result {
        Ok(()) => app_info!("BLE session end written {}", short_id(peripheral_id)),
        Err(error) => app_info!(
            "BLE session end skipped {}: {error:?}",
            short_id(peripheral_id)
        ),
    }
    // Dropping the receiver cancels the link.
    rx.succeeded = false;
    result.is_ok() || (power_off && !matches!(result, Err(LinkError::BleTimeout)))
}

/// Use the current link when it is up with its characteristics, otherwise
/// connect once more (no connect timeout retry).
async fn ensure_link(peripheral_id: &str, rx: &mut BleReceiver) -> Result<(), LinkError> {
    let (tx, linked) = futures_channel::oneshot::channel();
    let id = peripheral_id.to_owned();
    DispatchQueue::main().exec_async(move || {
        let mut up = false;
        with_peripheral(&id, |peripheral| {
            up = unsafe { peripheral.state() } == CBPeripheralState::Connected
                && find_characteristic(peripheral, NETWORK_TYPE).is_some();
        });
        let _ = tx.send(up);
    });
    if linked.await.unwrap_or(false) {
        return Ok(());
    }
    connect_with_retries(peripheral_id, timing::BLE_CONNECT_TIMEOUT, 0, rx).await
}

/// Sends the id of each peripheral that disconnects on `tx`, for example a
/// camera that is turned off after it connected.
pub fn watch_disconnects(tx: async_channel::Sender<String>) {
    *shared().disconnect_tx.lock().unwrap() = Some(tx);
}
