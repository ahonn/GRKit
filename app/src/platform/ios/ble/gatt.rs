//! The GATT operations of one setup (connect, discover, read, write,
//! subscribe) and of the teardown after it.

use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use dispatch2::DispatchQueue;
use futures_util::future::{Either, select};
use nsh_core::prelude::timer;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_core_bluetooth::{
    CBCharacteristic, CBCharacteristicWriteType, CBManagerState, CBPeripheral, CBPeripheralState,
};
use objc2_foundation::{NSArray, NSData};

use crate::domain::uuids::{CAMERA_POWER, CAMERA_SERVICE, NETWORK_TYPE, WLAN_CONTROL_SERVICE};
use crate::domain::{
    BleSetup, CameraModel, GattPort, LinkError, matches_operation, run_ble_setup, timing,
};

use super::delegate::emit_missing;
use super::*;

/// Connect, read Wi-Fi credentials, wake and start the AP
/// ([`run_ble_setup`]). `connected` is set once the Bluetooth link is up, so
/// a cancelled attempt knows to close the access point.
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
    let model = shared()
        .nearby
        .lock()
        .unwrap()
        .get(peripheral_id)
        .map(|n| n.model)
        .unwrap_or(model_hint);
    let mut port = IosGatt {
        peripheral_id,
        budget,
        connected,
        rx: &mut rx,
    };
    let result = run_ble_setup(&mut port, peripheral_id, model, powered_on).await;
    rx.succeeded = result.is_ok();
    match &result {
        Ok(_) => app_info!("BLE setup ok {}", short_id(peripheral_id)),
        Err(e) => log::warn!("BLE setup failed {}: {e:?}", short_id(peripheral_id)),
    }
    result
}

type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// The GATT port of one setup over CoreBluetooth.
struct IosGatt<'a> {
    peripheral_id: &'a str,
    budget: (Duration, u32),
    connected: &'a Cell<bool>,
    rx: &'a mut BleReceiver,
}

impl GattPort for IosGatt<'_> {
    fn connect(&mut self) -> BoxFut<'_, Result<(), LinkError>> {
        Box::pin(async move {
            ensure_central_async(self.rx).await?;
            pause_scanning();
            timer(timing::SCAN_CONNECT_SETTLE).await;
            let (timeout, timeout_retries) = self.budget;
            connect_with_retries(self.peripheral_id, timeout, timeout_retries, self.rx).await?;
            self.connected.set(true);
            Ok(())
        })
    }

    fn read(&mut self, characteristic: &'static str) -> BoxFut<'_, Result<Vec<u8>, LinkError>> {
        Box::pin(gatt_read(self.peripheral_id, characteristic, self.rx))
    }

    fn write(
        &mut self,
        characteristic: &'static str,
        value: &'static [u8],
    ) -> BoxFut<'_, Result<(), LinkError>> {
        // The latch keeps a notify that arrives before the write response.
        if characteristic == NETWORK_TYPE {
            self.rx.ap_ready.reset();
        }
        Box::pin(gatt_write(
            self.peripheral_id,
            characteristic,
            value,
            self.rx,
        ))
    }

    fn subscribe(&mut self, characteristic: &'static str) -> BoxFut<'_, Result<(), LinkError>> {
        Box::pin(gatt_subscribe(self.peripheral_id, characteristic, self.rx))
    }

    fn wait_notify(
        &mut self,
        characteristic: &'static str,
        value: u8,
        timeout: Duration,
    ) -> BoxFut<'_, Result<bool, LinkError>> {
        Box::pin(wait_notify_value(
            self.peripheral_id,
            characteristic,
            value,
            timeout,
            self.rx,
        ))
    }

    fn unsubscribe(&mut self, characteristic: &'static str) {
        let id = self.peripheral_id.to_owned();
        DispatchQueue::main().exec_async(move || {
            with_peripheral(&id, |peripheral| {
                if let Some(ch) = find_characteristic(peripheral, characteristic) {
                    unsafe { peripheral.setNotifyValue_forCharacteristic(false, &ch) };
                }
            });
        });
    }

    fn pause(&mut self, duration: Duration) -> BoxFut<'_, ()> {
        Box::pin(timer(duration))
    }
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

pub(super) fn with_peripheral(id: &str, f: impl FnOnce(&CBPeripheral)) {
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
