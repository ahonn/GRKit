//! The phone's own services: the Bluetooth and Wi-Fi link to a camera,
//! the saved cameras, system alerts and settings, and the HTTP and socket
//! transport. Each OS module (`ios`, `android`, and `stub` for the desktop)
//! has the same functions; this module forwards to the one it builds for.
//! The camera service's clients over that transport are in `camera_api`.

#[cfg(target_os = "android")]
mod android;
#[cfg(not(target_os = "android"))]
mod generic;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(not(any(target_os = "ios", target_os = "android")))]
mod stub;

pub(crate) mod link_support;

#[cfg(target_os = "android")]
use android as imp;
#[cfg(target_os = "ios")]
use ios as imp;
#[cfg(not(any(target_os = "ios", target_os = "android")))]
use stub as imp;

use std::io;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

#[cfg(demo)]
use crate::camera_api::service;
use crate::domain::connect_policy::NetworkSnapshot;
use crate::domain::nearby_watch::Frame;
use crate::domain::{
    CameraModel, CameraStatus, ConnectOptions, ConnectReport, ConnectStep, LinkError, PackageInfo,
    RememberedCamera,
};

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
    /// An empty name is a valid answer (a recipe then takes its automatic
    /// name).
    pub allow_empty: bool,
    pub save: String,
    pub cancel: String,
}

/// Ask for a name in the platform's own alert: the text on Save, `None` on
/// Cancel.
pub(crate) async fn ask_name(prompt: NamePrompt) -> Option<String> {
    imp::ask_name(prompt).await
}

/// A connected camera, and what the attempt taught the warm-up learner
/// (none for an HTTP-only development connect).
pub struct Connected {
    pub status: CameraStatus,
    pub package: PackageInfo,
    pub remembered: RememberedCamera,
    pub report: Option<ConnectReport>,
}

/// True when the central has reported that Bluetooth cannot be used (e.g. simulator Unsupported).
pub fn bluetooth_unavailable() -> bool {
    imp::bluetooth_unavailable()
}

/// Load the saved cameras, the most recent first.
pub fn load_saved() -> Vec<RememberedCamera> {
    imp::load_saved()
}

/// Persist the saved cameras.
pub fn save_saved(cameras: &[RememberedCamera]) -> Result<(), String> {
    imp::save_saved(cameras)
}

/// End the session over Bluetooth, when the camera's API did not: close the
/// camera's access point, or turn the camera off. Best effort and bounded.
pub async fn end_session_over_bluetooth(peripheral_id: &str, power_off: bool) -> bool {
    imp::end_session(peripheral_id, power_off).await
}

/// Start scanning for nearby cameras. Each advertisement frame is pushed on `tx`.
/// When `remembered_id` is set, retrieve that peripheral so reconnect works before a fresh advertisement.
/// Only `ask` may show the system's Bluetooth permission request: a scan
/// without it waits until the user has allowed Bluetooth.
pub fn start_scanning(tx: async_channel::Sender<Frame>, remembered_id: Option<&str>, ask: bool) {
    imp::start_scanning(tx, remembered_id, ask);
}

/// Sends the id of each camera that disconnects on `tx`.
pub fn watch_disconnects(tx: async_channel::Sender<String>) {
    imp::watch_disconnects(tx);
}

/// Sends a value on `tx` each time the phone drops the camera network the
/// app requested. Only Android reports this; elsewhere the probes notice.
pub fn watch_network_loss(tx: async_channel::Sender<()>) {
    imp::watch_network_loss(tx);
}

/// The route the phone's camera traffic takes now.
pub async fn network_snapshot() -> NetworkSnapshot {
    imp::network_snapshot().await
}

/// Stop an active scan.
pub fn stop_scanning() {
    imp::stop_scanning();
}

/// The camera session ended: drop the Bluetooth link the connect kept, so the
/// camera advertises again and the nearby watch sees it power off and on.
/// The camera's access point and the phone's Wi-Fi are left as they are.
pub fn release_link(peripheral_id: &str) {
    imp::release_link(peripheral_id);
}

/// Full connect: BLE wake + Wi-Fi join + status/package. A development
/// build with `GRKIT_CAMERA_URL` set connects over HTTP alone.
pub async fn connect_camera(
    peripheral_id: &str,
    model_hint: CameraModel,
    options: ConnectOptions,
    mut on_step: impl FnMut(ConnectStep),
) -> Result<Connected, LinkError> {
    #[cfg(demo)]
    if crate::camera_api::has_dev_camera_url() {
        return connect_http_only(model_hint, &mut on_step).await;
    }
    imp::connect_camera(peripheral_id, model_hint, options, &mut on_step).await
}

/// Skip Bluetooth and Wi-Fi; poll status then package against
/// `camera_api::camera_http_base`.
#[cfg(demo)]
async fn connect_http_only(
    model_hint: CameraModel,
    on_step: &mut dyn FnMut(ConnectStep),
) -> Result<Connected, LinkError> {
    app_info!(
        "connect HTTP-only against {}",
        crate::camera_api::camera_http_base()
    );
    on_step(ConnectStep::Bluetooth);
    on_step(ConnectStep::Wifi);
    service::wait_until_ready().await?;
    on_step(ConnectStep::Version);
    let (status, package) = service::read_package(model_hint).await?;
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
    imp::open_app_settings().await;
}

/// Bound a platform operation even when its native completion never arrives.
pub(crate) async fn with_timeout<T>(
    future: impl std::future::Future<Output = T>,
    duration: Duration,
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

/// A build setting that the host passes in: the debuggable Android host
/// injects them without changing the process environment. `None` elsewhere.
pub(crate) fn host_config(name: &str) -> Option<String> {
    imp::config(name)
}

/// A build setting: from the host ([`host_config`]), else from the process
/// environment.
pub(crate) fn config_value(name: &str) -> Option<String> {
    host_config(name).or_else(|| std::env::var(name).ok())
}

/// A request to the camera over the phone's own HTTP stack, without waiting
/// for a turn at the service (`camera_api::camera_request` waits): the link
/// check, which must not wait behind a slow read. On Android it goes over
/// the network that the app requested for the camera.
pub(crate) async fn camera_request_now(
    method: &str,
    url: String,
    body: Vec<u8>,
    budget: Duration,
    headers: &[(&str, String)],
) -> Result<nsh_core::element::HttpResponse, String> {
    imp::camera_request(method, url, body, budget, headers).await
}

/// A TCP connection to the camera within `timeout`: on Android over the
/// camera network that the app requested, which is not the default route.
pub(crate) fn connect_socket(address: SocketAddr, timeout: Duration) -> io::Result<TcpStream> {
    imp::connect_socket(address, timeout)
}
