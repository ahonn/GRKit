//! App-owned JNI bridge. Java owns system callbacks; Rust owns connection/update state.
use crate::domain::connect_policy::NetworkSnapshot;
use crate::domain::nearby_watch::{Frame, advertised_power};
use crate::domain::*;
use async_channel::Sender;
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_channel::oneshot;
use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JClass, JString, JValue},
    sys::jlong,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{
        Mutex, MutexGuard, OnceLock,
        atomic::{AtomicBool, AtomicI64, Ordering},
    },
    time::Duration,
};

struct Bridge {
    vm: JavaVM,
    class: GlobalRef,
}
static BRIDGE: OnceLock<Bridge> = OnceLock::new();
static CONFIG: OnceLock<[String; 4]> = OnceLock::new();
static NEXT: AtomicI64 = AtomicI64::new(1);
type Reply = oneshot::Sender<Result<Value, String>>;
static PENDING: Mutex<BTreeMap<i64, Reply>> = Mutex::new(BTreeMap::new());
static SCANS: Mutex<Option<Sender<Frame>>> = Mutex::new(None);
static DISCONNECTS: Mutex<Option<Sender<String>>> = Mutex::new(None);
static NETWORK_LOSS: Mutex<Option<Sender<()>>> = Mutex::new(None);
static UNAVAILABLE: AtomicBool = AtomicBool::new(false);

pub fn config(name: &str) -> Option<String> {
    let index = match name {
        "GRKIT_CAMERA_URL" => 0,
        "GRKIT_RELEASE_BASE" => 1,
        "GRKIT_DEMO_SCREEN" => 2,
        "GRKIT_RELEASE_PUBLIC_KEY" => 3,
        _ => return None,
    };
    CONFIG
        .get()
        .map(|c| c[index].clone())
        .filter(|v| !v.is_empty())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_studio_techflow_grkit_MainActivity_nativeInitialize(
    mut env: JNIEnv,
    _: JClass,
    camera: JString,
    release: JString,
    demo: JString,
    public_key: JString,
) {
    let result = (|| -> jni::errors::Result<()> {
        let config = [
            env.get_string(&camera)?.into(),
            env.get_string(&release)?.into(),
            env.get_string(&demo)?.into(),
            env.get_string(&public_key)?.into(),
        ];
        let class = env.find_class("studio/techflow/grkit/CameraBridge")?;
        let bridge = Bridge {
            vm: env.get_java_vm()?,
            class: env.new_global_ref(class)?,
        };
        let _ = BRIDGE.set(bridge);
        let _ = CONFIG.set(config);
        Ok(())
    })();
    if let Err(error) = result {
        let _ = env.exception_clear();
        UNAVAILABLE.store(true, Ordering::Relaxed);
        log::error!("Android bridge initialization failed: {error}");
    }
}

// Engine threads can stay attached to the VM for their entire lifetime. Every
// outbound call gets a frame so repeated HTTP polls cannot accumulate local refs.
fn with_env<T>(
    f: impl FnOnce(&mut JNIEnv<'_>, &JClass<'_>) -> jni::errors::Result<T>,
) -> Result<T, String> {
    let bridge = BRIDGE.get().ok_or("Android host is unavailable")?;
    let mut env = bridge
        .vm
        .attach_current_thread()
        .map_err(|e| e.to_string())?;
    let class: &JClass = bridge.class.as_obj().into();
    let result = env.with_local_frame(16, |env| f(env, class));
    if result.is_err() {
        // Never leave a Java exception pending on a reused engine thread.
        let _ = env.exception_clear();
    }
    result.map_err(|e| e.to_string())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn dispatch(id: i64, operation: &str, data: Value) -> Result<(), String> {
    with_env(|env, class| {
        let operation = env.new_string(operation)?;
        let data = env.new_string(data.to_string())?;
        env.call_static_method(
            class,
            "execute",
            "(JLjava/lang/String;Ljava/lang/String;)V",
            &[
                JValue::Long(id),
                JValue::Object(&operation),
                JValue::Object(&data),
            ],
        )
        .map(|_| ())
    })
}
struct RequestGuard(i64);
impl Drop for RequestGuard {
    fn drop(&mut self) {
        let pending = lock(&PENDING).remove(&self.0).is_some();
        if pending {
            let _ = dispatch(0, "cancelRequest", json!({"id": self.0}));
        }
    }
}
async fn call(operation: &str, data: Value, budget: Duration) -> Result<Value, String> {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = oneshot::channel();
    lock(&PENDING).insert(id, tx);
    let _guard = RequestGuard(id);
    dispatch(id, operation, data)?;
    super::with_timeout(rx, budget)
        .await
        .ok_or("ConnectionLost")?
        .map_err(|_| "ConnectionLost".to_owned())?
}
/// An open alert waits for the person, so its budget is not a network one.
const NAME_PROMPT_BUDGET: Duration = Duration::from_secs(24 * 60 * 60);

pub async fn ask_name(prompt: super::NamePrompt) -> Option<String> {
    let data = json!({
        "title": prompt.title,
        "message": prompt.message,
        "value": prompt.value,
        "max": prompt.max,
        "save": prompt.save,
        "cancel": prompt.cancel,
    });
    call("askName", data, NAME_PROMPT_BUDGET)
        .await
        .ok()?
        .as_str()
        .map(str::to_owned)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_studio_techflow_grkit_CameraBridge_complete(
    mut env: JNIEnv,
    _: JClass,
    id: jlong,
    value: JString,
    error: JString,
) {
    let result = if !error.is_null() {
        Err(env
            .get_string(&error)
            .map(String::from)
            .unwrap_or_else(|_| "ConnectionLost".into()))
    } else {
        env.get_string(&value)
            .map(String::from)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
    };
    if result.is_err() {
        let _ = env.exception_clear();
    }
    let pending = lock(&PENDING).remove(&id);
    if let Some(tx) = pending {
        let _ = tx.send(result);
    }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_studio_techflow_grkit_CameraBridge_event(
    mut env: JNIEnv,
    _: JClass,
    kind: JString,
    value: JString,
) {
    let Ok(kind) = env.get_string(&kind).map(String::from) else {
        let _ = env.exception_clear();
        return;
    };
    let Ok(value) = env.get_string(&value).map(String::from) else {
        let _ = env.exception_clear();
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&value) else {
        return;
    };
    match kind.as_str() {
        "unavailable" => UNAVAILABLE.store(value.as_bool().unwrap_or(true), Ordering::Relaxed),
        "disconnected" => {
            if let (Some(tx), Some(id)) = (lock(&DISCONNECTS).as_ref(), value.as_str()) {
                let _ = tx.try_send(id.into());
            }
        }
        "networkLost" => {
            if let Some(tx) = lock(&NETWORK_LOSS).as_ref() {
                let _ = tx.try_send(());
            }
        }
        "scan" => {
            let Some(id) = value["id"].as_str() else {
                return;
            };
            let bytes: Vec<u8> = value["manufacturer"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|b| b.as_u64().and_then(|b| u8::try_from(b).ok()))
                        .collect()
                })
                .unwrap_or_default();
            let services: Vec<String> = value["services"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            let adv = parse_advertisement(Some(&bytes), &services);
            if !is_supported_camera(&adv) {
                return;
            }
            let Some(model) = adv.model else {
                return;
            };
            if let Some(tx) = lock(&SCANS).as_ref() {
                let _ = tx.try_send(Frame {
                    peripheral_id: id.into(),
                    model,
                    power: advertised_power(adv.powered_on, adv.advertises_camera_information),
                    name: value["name"]
                        .as_str()
                        .filter(|n| !n.is_empty())
                        .map(str::to_owned),
                    rssi: value["rssi"].as_i64().and_then(|v| i32::try_from(v).ok()),
                });
            }
        }
        _ => {}
    }
}

pub fn bluetooth_unavailable() -> bool {
    UNAVAILABLE.load(Ordering::Relaxed)
}
pub fn start_scanning(tx: Sender<Frame>, remembered: Option<&str>, ask: bool) {
    *lock(&SCANS) = Some(tx);
    if dispatch(0, "scan", json!({"remembered_id": remembered, "ask": ask})).is_err() {
        UNAVAILABLE.store(true, Ordering::Relaxed);
    }
}
pub fn stop_scanning() {
    *lock(&SCANS) = None;
    let _ = dispatch(0, "stopScan", json!({}));
}
pub fn release_link(_peripheral_id: &str) {
    let _ = dispatch(0, "release", json!({}));
}
pub fn watch_disconnects(tx: Sender<String>) {
    *lock(&DISCONNECTS) = Some(tx);
}
/// Hears each loss of the camera network the app requested.
pub fn watch_network_loss(tx: Sender<()>) {
    *lock(&NETWORK_LOSS) = Some(tx);
}
/// The route to the camera, read from the camera network's link properties,
/// or from the active network when no camera network is up. The interface
/// list cannot tell which `wlan` carries the camera when the phone keeps two
/// Wi-Fi networks at once.
pub async fn network_snapshot() -> NetworkSnapshot {
    match call("network", json!({}), Duration::from_secs(2)).await {
        Ok(value) => NetworkSnapshot {
            wifi: value["wifi"].as_bool().unwrap_or(false),
            address: value["address"].as_str().and_then(|a| a.parse().ok()),
        },
        Err(error) => {
            log::warn!("Network snapshot failed: {error}");
            super::link_support::network_snapshot()
        }
    }
}
/// Close the camera's access point, or turn it off, over Bluetooth.
pub async fn end_session(peripheral_id: &str, power_off: bool) -> bool {
    call(
        "endSession",
        json!({"id": peripheral_id, "power_off": power_off}),
        END_SESSION_BUDGET,
    )
    .await
    .is_ok()
}

/// A reconnect and one write; a power off gets more time, as the camera
/// drops the link while it shuts down.
const END_SESSION_BUDGET: Duration = Duration::from_secs(10);

pub fn load_saved() -> Vec<RememberedCamera> {
    let text = with_env(|env, class| {
        let value = env
            .call_static_method(class, "remembered", "()Ljava/lang/String;", &[])?
            .l()?;
        if value.is_null() {
            return Ok(None);
        }
        let value = JString::from(value);
        env.get_string(&value).map(|s| Some(String::from(s)))
    })
    .ok()
    .flatten();
    text.and_then(|text| decode_saved(&text).ok())
        .unwrap_or_default()
}
pub fn save_saved(cameras: &[RememberedCamera]) -> Result<(), String> {
    let encoded = encode_saved(cameras)?;
    with_env(|env, class| {
        let value = env.new_string(encoded)?;
        env.call_static_method(
            class,
            "saveRemembered",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&value)],
        )
        .map(|_| ())
    })
}
fn link_error(error: String) -> LinkError {
    match error.as_str() {
        "NotNearby" => LinkError::NotNearby,
        "Busy" => LinkError::Busy,
        "Cancelled" => LinkError::Cancelled,
        "BluetoothUnavailable" => LinkError::BluetoothUnavailable,
        "PairingFailed" => LinkError::PairingFailed,
        "NotPaired" => LinkError::NotPaired,
        "Disconnected" => LinkError::BleDisconnected,
        "ConnectFailed" => LinkError::BleConnectFailed,
        "Timeout" => LinkError::BleTimeout,
        "Authentication" => LinkError::BleAuthentication,
        "WifiJoinDenied" => LinkError::WifiJoinDenied,
        "WifiJoinFailed" => LinkError::WifiJoinFailed,
        "WifiJoinTimeout" => LinkError::WifiJoinTimeout,
        "WifiJoinRejected" => LinkError::WifiJoinRejected,
        _ => LinkError::ConnectionLost,
    }
}
/// How a connect attempt ended, told to the Java bridge when it is dropped.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// Dropped before the flow returned: the user cancelled.
    Cancelled,
    Failed,
    Succeeded,
}
struct Attempt(Outcome);
impl Drop for Attempt {
    fn drop(&mut self) {
        let _ = match self.0 {
            // Resume the scan the attempt paused.
            Outcome::Succeeded => dispatch(0, "finish", json!({})),
            // A camera the user cancelled is told to close its access point;
            // an attempt that ran out of time leaves it as it is.
            Outcome::Cancelled if !super::connect_deadline_reached() => {
                dispatch(0, "cancel", json!({"teardown": true}))
            }
            Outcome::Cancelled => dispatch(0, "cancel", json!({})),
            Outcome::Failed => dispatch(0, "cancel", json!({})),
        };
    }
}
type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

struct AndroidTransport {
    model_hint: CameraModel,
    /// A fresh advertisement says the camera is on: skip the wake write.
    powered_on: bool,
    /// Bluetooth connect timeout and the timeouts it retries.
    ble_budget: (Duration, u32),
    package: Option<(CameraStatus, PackageInfo)>,
}

impl ConnectTransport for AndroidTransport {
    fn ble_setup(&mut self, peripheral_id: &str) -> BoxFut<'_, Result<BleSetup, LinkError>> {
        let id = peripheral_id.to_owned();
        let model = self.model_hint;
        let powered_on = self.powered_on;
        let (connect_timeout, timeout_retries) = self.ble_budget;
        Box::pin(async move {
            let setup = call(
                "setup",
                json!({
                    "id": id,
                    "model": model,
                    "powered_on": powered_on,
                    "connect_timeout_ms": connect_timeout.as_millis() as u64,
                    "timeout_retries": timeout_retries,
                }),
                // Includes connection retries and up to two waits for the
                // system pairing prompt.
                Duration::from_secs(240),
            )
            .await
            .map_err(link_error)?;
            let credentials = &setup["credentials"];
            let ssid = credentials["ssid"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or(LinkError::NotPaired)?;
            let passphrase = credentials["passphrase"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or(LinkError::NotPaired)?;
            Ok(BleSetup {
                peripheral_id: id,
                model,
                powered_on_already: setup["powered_on_already"].as_bool().unwrap_or(false),
                credentials: WifiCredentials {
                    ssid: ssid.to_owned(),
                    passphrase: passphrase.to_owned(),
                },
                ap_ready: setup["ap_ready"].as_bool().unwrap_or(false),
            })
        })
    }

    fn wait_foreground(&mut self) -> BoxFut<'_, ()> {
        // Android joins from the background.
        Box::pin(async {})
    }

    fn join_wifi(
        &mut self,
        credentials: &WifiCredentials,
        force: bool,
    ) -> BoxFut<'_, Result<Join, LinkError>> {
        let data =
            json!({"ssid":credentials.ssid,"passphrase":credentials.passphrase,"force":force});
        Box::pin(async move {
            // Permission and network selection each have their own native deadline.
            match call("join", data, Duration::from_secs(130))
                .await
                .map_err(link_error)
            {
                Ok(_) => Ok(Join::Joined),
                Err(error) if error.is_soft_join_failure() => Ok(Join::SoftFailed(error)),
                Err(error) => Err(error),
            }
        })
    }

    fn network(&mut self) -> BoxFut<'_, NetworkSnapshot> {
        Box::pin(network_snapshot())
    }

    fn probe(&mut self, timeout: Duration) -> BoxFut<'_, Probe> {
        Box::pin(super::link_support::probe_props(timeout))
    }

    fn camera_api_answers(&mut self) -> BoxFut<'_, bool> {
        Box::pin(super::http::camera_api_answers())
    }

    fn read_version(&mut self, model: CameraModel) -> BoxFut<'_, Result<(), LinkError>> {
        Box::pin(async move {
            self.package = Some(super::http::read_package(model).await?);
            Ok(())
        })
    }

    fn pause(&mut self, duration: Duration) -> BoxFut<'_, ()> {
        Box::pin(nsh_core::prelude::timer(duration))
    }

    fn clock(&mut self) -> Duration {
        super::link_support::foreground_clock()
    }

    fn local_network_access(&mut self) -> BoxFut<'_, LocalNetworkAccess> {
        Box::pin(async {
            // The native request bounds the system dialog itself; a failed
            // check cannot tell.
            match call("localNetwork", json!({}), Duration::from_secs(65)).await {
                Ok(value) if value == "granted" => LocalNetworkAccess::Granted,
                Ok(value) if value == "denied" => LocalNetworkAccess::Denied,
                _ => LocalNetworkAccess::Unknown,
            }
        })
    }
}

pub async fn connect_camera(
    id: &str,
    model: CameraModel,
    options: ConnectOptions,
    on_step: &mut dyn FnMut(ConnectStep),
) -> Result<super::Connected, LinkError> {
    let mut attempt = Attempt(Outcome::Cancelled);
    let mut transport = AndroidTransport {
        model_hint: model,
        powered_on: options.advertised_powered_on,
        ble_budget: options.ble_connect_budget(),
        package: None,
    };
    let result = run_connect(&mut transport, id, options, on_step)
        .await
        .and_then(|report| {
            let package = transport.package.take().ok_or(LinkError::ConnectionLost)?;
            Ok((report, package))
        });
    attempt.0 = Outcome::Failed;
    let (report, (status, package)) = result?;
    let remembered = RememberedCamera::new(
        report.setup.peripheral_id.clone(),
        status.camera.model,
        report.setup.credentials.ssid.clone(),
    )
    .with_device_id(status.device_id.clone());
    attempt.0 = Outcome::Succeeded;
    Ok(super::Connected {
        status,
        package,
        remembered,
        report: Some(report),
    })
}

/// Open this app's details page in Android settings.
pub async fn open_app_settings() {
    if let Err(error) = call("settings", json!({}), Duration::from_secs(5)).await {
        log::warn!("Could not open app settings: {error}");
    }
}

/// Route the socket `fd` over the camera network that the app requested, as
/// `camera_request` does. Without that network the socket keeps the default
/// route.
pub fn bind_socket(fd: i32) -> Result<(), String> {
    with_env(|env, class| {
        env.call_static_method(class, "bindSocket", "(I)V", &[JValue::Int(fd)])
            .map(|_| ())
    })
}

pub async fn camera_request(
    method: &str,
    url: String,
    body: Vec<u8>,
    budget: Duration,
    headers: &[(&str, String)],
) -> Result<nsh_core::element::HttpResponse, String> {
    let mut header_map = serde_json::Map::new();
    header_map.insert("Cache-Control".into(), json!("no-cache, no-store"));
    for (name, value) in headers {
        header_map.insert((*name).into(), json!(value));
    }
    let value = call(
        "http",
        json!({
            "method": method,
            "url": url,
            "body": STANDARD.encode(body),
            "timeout_ms": budget.as_millis(),
            "headers": header_map,
        }),
        budget,
    )
    .await?;
    let status = value["status"]
        .as_u64()
        .and_then(|v| u16::try_from(v).ok())
        .ok_or("Invalid HTTP status")?;
    let response_headers = value["headers"]
        .as_object()
        .map(|h| {
            h.iter()
                .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_owned())))
                .collect()
        })
        .unwrap_or_default();
    let body = STANDARD
        .decode(value["body"].as_str().ok_or("Invalid HTTP body")?)
        .map_err(|e| e.to_string())?;
    Ok(nsh_core::element::HttpResponse {
        head: nsh_core::platform::HttpHead {
            status,
            status_text: String::new(),
            url,
            redirected: false,
            headers: response_headers,
        },
        body,
    })
}
