//! Native Bluetooth / Wi-Fi bridges for iOS and Android.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(not(any(target_os = "ios", target_os = "android")))]
mod stub;

/// Registers the app's native views on an iOS surface (`lib.rs`).
#[cfg(target_os = "ios")]
pub(crate) fn register_ios_components(components: &nsh_platform_ios::Components) {
    ios::activity::register(components);
}

/// A text alert that asks for a name.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) struct NamePrompt {
    pub title: String,
    pub message: String,
    pub value: String,
    pub max: usize,
    pub save: String,
    pub cancel: String,
}

/// Ask for a name in the platform's own alert: the text on Save, `None` on
/// Cancel.
pub(crate) async fn ask_name(prompt: NamePrompt) -> Option<String> {
    #[cfg(target_os = "ios")]
    {
        ios::name_prompt::ask(prompt).await
    }
    #[cfg(target_os = "android")]
    {
        android::ask_name(prompt).await
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        stub::ask_name(prompt)
    }
}

pub(crate) mod changes;
pub(crate) mod http;
pub(crate) mod link_support;
pub(crate) mod looks;
pub(crate) mod recipes;
pub(crate) mod release;
pub(crate) mod update;

use crate::domain::{
    CAMERA_HTTP_BASE, CameraModel, CameraStatus, ConnectOptions, ConnectReport, ConnectStep,
    LinkError, PackageInfo, RememberedCamera,
};

/// A connected camera, and what the attempt taught the warm-up learner
/// (none for an HTTP-only development connect).
pub struct Connected {
    pub status: CameraStatus,
    pub package: PackageInfo,
    pub remembered: RememberedCamera,
    pub report: Option<ConnectReport>,
}

/// Base URL for camera HTTP. `GRKIT_CAMERA_URL` overrides the on-camera default when set.
pub fn camera_http_base() -> String {
    config_value("GRKIT_CAMERA_URL")
        .map(|s| s.trim().trim_end_matches('/').to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| CAMERA_HTTP_BASE.to_owned())
}

/// True when `GRKIT_CAMERA_URL` is set (HTTP-only connect for simulator / local server).
pub fn has_dev_camera_url() -> bool {
    config_value("GRKIT_CAMERA_URL")
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

/// True when this build can drive a Bluetooth central in principle (iOS / Android).
#[allow(dead_code)]
pub fn bluetooth_available() -> bool {
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        true
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        false
    }
}

/// True when the central has reported that Bluetooth cannot be used (e.g. simulator Unsupported).
pub fn bluetooth_unavailable() -> bool {
    #[cfg(target_os = "ios")]
    {
        ios::ble::bluetooth_unavailable()
    }
    #[cfg(target_os = "android")]
    {
        android::bluetooth_unavailable()
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        true
    }
}

/// Load the saved cameras, the most recent first.
pub fn load_saved() -> Vec<RememberedCamera> {
    #[cfg(target_os = "ios")]
    {
        ios::remember::load()
    }
    #[cfg(target_os = "android")]
    {
        android::load_saved()
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        stub::load_saved()
    }
}

/// Persist the saved cameras.
pub fn save_saved(cameras: &[RememberedCamera]) -> Result<(), String> {
    #[cfg(target_os = "ios")]
    {
        ios::remember::save(cameras)
    }
    #[cfg(target_os = "android")]
    {
        android::save_saved(cameras)
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        stub::save_saved(cameras)
    }
}

/// End the session over Bluetooth, when the camera's API did not: close the
/// camera's access point, or turn the camera off. Best effort and bounded.
pub async fn end_session_over_bluetooth(peripheral_id: &str, power_off: bool) -> bool {
    #[cfg(target_os = "ios")]
    {
        ios::ble::end_session(peripheral_id, power_off).await
    }
    #[cfg(target_os = "android")]
    {
        android::end_session(peripheral_id, power_off).await
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        let _ = (peripheral_id, power_off);
        false
    }
}

/// Start scanning for nearby cameras. Each advertisement frame is pushed on `tx`.
/// When `remembered_id` is set, retrieve that peripheral so reconnect works before a fresh advertisement.
/// Only `ask` may show the system's Bluetooth permission request: a scan
/// without it waits until the user has allowed Bluetooth.
pub fn start_scanning(
    tx: async_channel::Sender<crate::domain::nearby_watch::Frame>,
    remembered_id: Option<&str>,
    ask: bool,
) {
    #[cfg(target_os = "ios")]
    {
        ios::ble::start_scanning(tx, remembered_id, ask);
    }
    #[cfg(target_os = "android")]
    {
        android::start_scanning(tx, remembered_id, ask);
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        stub::start_scanning(tx, remembered_id, ask);
    }
}

/// Sends the id of each camera that disconnects on `tx`.
pub fn watch_disconnects(tx: async_channel::Sender<String>) {
    #[cfg(target_os = "ios")]
    {
        ios::ble::watch_disconnects(tx);
    }
    #[cfg(target_os = "android")]
    {
        android::watch_disconnects(tx);
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        stub::watch_disconnects(tx);
    }
}

/// Sends a value on `tx` each time the phone drops the camera network the
/// app requested. Only Android reports this; elsewhere the probes notice.
pub fn watch_network_loss(tx: async_channel::Sender<()>) {
    #[cfg(target_os = "android")]
    {
        android::watch_network_loss(tx);
    }
    #[cfg(not(target_os = "android"))]
    {
        drop(tx);
    }
}

/// The route the phone's camera traffic takes now.
pub async fn network_snapshot() -> crate::domain::connect_policy::NetworkSnapshot {
    #[cfg(target_os = "android")]
    {
        android::network_snapshot().await
    }
    #[cfg(not(target_os = "android"))]
    {
        link_support::network_snapshot()
    }
}

/// Stop an active scan.
#[allow(dead_code)]
pub fn stop_scanning() {
    #[cfg(target_os = "ios")]
    {
        ios::ble::stop_scanning();
    }
    #[cfg(target_os = "android")]
    {
        android::stop_scanning();
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        stub::stop_scanning();
    }
}

/// The camera session ended: drop the Bluetooth link the connect kept, so the
/// camera advertises again and the nearby watch sees it power off and on.
/// The camera's access point and the phone's Wi-Fi are left as they are.
pub fn release_link(peripheral_id: &str) {
    #[cfg(target_os = "ios")]
    {
        ios::ble::release_link(peripheral_id);
    }
    #[cfg(target_os = "android")]
    {
        android::release_link(peripheral_id);
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        let _ = peripheral_id;
    }
}

/// Full connect: BLE wake + Wi-Fi join + status/package, or HTTP-only when `GRKIT_CAMERA_URL` is set.
pub async fn connect_camera(
    peripheral_id: &str,
    model_hint: CameraModel,
    options: ConnectOptions,
    mut on_step: impl FnMut(ConnectStep),
) -> Result<Connected, LinkError> {
    if has_dev_camera_url() {
        return connect_http_only(model_hint, &mut on_step).await;
    }
    #[cfg(target_os = "ios")]
    {
        ios::link::connect(peripheral_id, model_hint, options, &mut on_step).await
    }
    #[cfg(target_os = "android")]
    {
        android::connect_camera(peripheral_id, model_hint, options, &mut on_step).await
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        let _ = (peripheral_id, model_hint, options, &mut on_step);
        stub::connect_camera().await
    }
}

/// Skip Bluetooth and Wi-Fi; poll status then package against [`camera_http_base`].
async fn connect_http_only(
    model_hint: CameraModel,
    on_step: &mut dyn FnMut(ConnectStep),
) -> Result<Connected, LinkError> {
    app_info!("connect HTTP-only against {}", camera_http_base());
    on_step(ConnectStep::Bluetooth);
    on_step(ConnectStep::Wifi);
    http::wait_until_ready().await?;
    on_step(ConnectStep::Version);
    let (status, package) = http::read_package(model_hint).await?;
    let remembered = RememberedCamera::new("dev-url", status.camera.model, "dev".to_owned());
    Ok(Connected {
        status,
        package,
        remembered,
        report: None,
    })
}

static CONNECT_DEADLINE_REACHED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// A connect attempt starts: it has not run out of time.
pub fn begin_connect_attempt() {
    CONNECT_DEADLINE_REACHED.store(false, std::sync::atomic::Ordering::SeqCst);
}

/// The attempt ran out of time and is about to be dropped. Unlike a user
/// cancel, this leaves the camera's access point on.
pub fn mark_connect_deadline() {
    CONNECT_DEADLINE_REACHED.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Whether the attempt being dropped ran out of time.
#[cfg_attr(not(any(target_os = "ios", target_os = "android")), allow(dead_code))]
pub(crate) fn connect_deadline_reached() -> bool {
    CONNECT_DEADLINE_REACHED.load(std::sync::atomic::Ordering::SeqCst)
}

/// Open this app's page in the system settings (permissions, Local Network).
pub async fn open_app_settings() {
    #[cfg(target_os = "android")]
    {
        android::open_app_settings().await;
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = nsh_core::prelude::open_url("app-settings:").await;
    }
}

/// Bound a platform operation even when its native completion never arrives.
pub(crate) async fn with_timeout<T>(
    future: impl std::future::Future<Output = T>,
    duration: std::time::Duration,
) -> Option<T> {
    use futures_util::future::{Either, select};
    match select(
        Box::pin(future),
        Box::pin(nsh_core::prelude::timer(duration)),
    )
    .await
    {
        Either::Left((value, _)) => Some(value),
        Either::Right(_) => None,
    }
}

/// Android debug host injects build settings without mutating process environment.
pub(crate) fn config_value(name: &str) -> Option<String> {
    #[cfg(target_os = "android")]
    if let Some(value) = android::config(name) {
        return Some(value);
    }
    std::env::var(name).ok()
}

pub(crate) async fn camera_request(
    method: &str,
    url: String,
    body: Vec<u8>,
    budget: std::time::Duration,
    headers: &[(&str, String)],
) -> Result<nsh_core::element::HttpResponse, String> {
    #[cfg(target_os = "android")]
    {
        android::camera_request(method, url, body, budget, headers).await
    }
    #[cfg(not(target_os = "android"))]
    {
        let mut request = nsh_core::platform::HttpRequest::new(method, url)
            .header("Cache-Control", "no-cache, no-store");
        for (name, value) in headers {
            request = request.header(*name, value.clone());
        }
        let mut request = request.body(body);
        request.follow_redirects = false;
        with_timeout(nsh_core::prelude::fetch(request), budget)
            .await
            .ok_or_else(|| "ConnectionLost".to_owned())?
    }
}
