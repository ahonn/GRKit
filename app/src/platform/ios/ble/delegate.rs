//! The delegate of the central and its peripherals: it turns each
//! CoreBluetooth callback into a [`BleEvent`].

use std::sync::atomic::Ordering;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{AnyThread, Message, define_class, msg_send};
use objc2_core_bluetooth::{
    CBAdvertisementDataManufacturerDataKey, CBAdvertisementDataServiceUUIDsKey, CBCentralManager,
    CBCentralManagerDelegate, CBCharacteristic, CBManagerState, CBPeripheral, CBPeripheralDelegate,
    CBService, CBUUID,
};
use objc2_foundation::{
    NSArray, NSData, NSDictionary, NSError, NSNumber, NSObject, NSObjectProtocol, NSString,
};

use crate::domain::nearby_watch::{Frame, advertised_power};
use crate::domain::{NearbyCamera, is_supported_camera, parse_advertisement};

use super::*;

pub(super) struct BleIvars;

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = BleIvars]
    pub(super) struct BleDelegate;

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

pub(super) fn emit_native_gatt_error(
    peripheral_id: &str,
    characteristic: Option<&str>,
    error: &NSError,
) {
    emit_ble_error(
        peripheral_id,
        characteristic,
        &error.localizedDescription().to_string(),
        BleFault::of(error),
    );
}

pub(super) fn emit_missing(peripheral_id: &str, characteristic: Option<&str>, message: &str) {
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
    pub(super) fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(BleIvars);
        unsafe { msg_send![super(this), init] }
    }
}

pub(super) fn uuid_string(peripheral: &CBPeripheral) -> String {
    unsafe { peripheral.identifier().UUIDString().to_string() }
}

pub(super) fn parse_adv_dict(
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
