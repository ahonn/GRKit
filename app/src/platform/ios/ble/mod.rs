//! CoreBluetooth central: the shared state, scanning, and the link. The
//! central's delegate is in `delegate`; the GATT operations of a setup and
//! of the teardown are in `gatt`.

mod delegate;
mod gatt;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use dispatch2::DispatchQueue;
use objc2::AnyThread;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2_core_bluetooth::{
    CBCentralManager, CBCentralManagerScanOptionAllowDuplicatesKey, CBManager,
    CBManagerAuthorization, CBManagerState, CBPeripheral, CBUUID,
};
use objc2_foundation::{NSArray, NSDictionary, NSError, NSNumber, NSString, NSUUID};

use crate::domain::nearby_watch::Frame;
use crate::domain::uuids::{BLUETOOTH_CONTROL_SERVICE, CAMERA_INFORMATION_SERVICE};
use crate::domain::{ApReadyLatch, LinkError, NearbyCamera, is_ble_pairing_error};

use delegate::{BleDelegate, uuid_string};
use gatt::with_peripheral;
pub use gatt::{ble_setup, close_access_point, end_session};

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

/// Sends the id of each peripheral that disconnects on `tx`, for example a
/// camera that is turned off after it connected.
pub fn watch_disconnects(tx: async_channel::Sender<String>) {
    *shared().disconnect_tx.lock().unwrap() = Some(tx);
}
