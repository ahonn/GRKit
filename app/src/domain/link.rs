//! The camera link the phone uses. Demo uses FakeCamera; iOS uses the real link.

use crate::domain::{Camera, CameraModel, PackageInfo, UpdateError, UpdateKind};

/// Progress while a package is copied to the camera, 0..=1.
pub type SendProgress = f32;

/// What polling the camera reports during an update.
#[derive(Clone, Debug, PartialEq)]
pub enum UpdatePoll {
    Sending { progress: f32 },
    WaitingConfirm,
    Confirmed,
    Installed { version: String },
    Failed(UpdateError),
}

/// Status returned while connecting or when already connected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraStatus {
    pub camera: Camera,
    pub powered_on: bool,
    /// The camera body's stable identity from `/v1/props`, when it gave one.
    pub device_id: Option<String>,
}

/// Errors from the link itself (before an update flow starts). Each connect
/// variant names its error code in its doc comment; the code picks the copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkError {
    NotNearby,
    Busy,
    Cancelled,
    /// The connected camera stopped answering, or a transport failed with no
    /// more specific cause.
    ConnectionLost,
    /// This build or device has no Bluetooth central, or Bluetooth is off.
    BluetoothUnavailable,
    /// The Bluetooth connection failed (`ble-connect-failed`). Retried inside
    /// the Bluetooth retry budget before it is shown.
    BleConnectFailed,
    /// Insufficient authentication or encryption: the system pairing has not
    /// completed yet. Retried while the user answers the pairing prompt.
    BleAuthentication,
    /// The camera dropped Bluetooth after connecting, before it gave the
    /// Wi-Fi credentials: a transient drop, or a pairing it has forgotten.
    BleDisconnected,
    /// The Bluetooth stack timed out: the camera is off, asleep or out of
    /// range (`ble-timeout`).
    BleTimeout,
    /// The camera removed this phone's pairing (`ble-pairing-stale`).
    BlePairingStale,
    /// Pairing did not complete within the retry window
    /// (`ble-authentication-failed`).
    PairingFailed,
    /// Pair on the camera, then try again (`ble-pairing-required`).
    NotPaired,
    /// The user declined to join the camera Wi-Fi (`wifi-join-denied`).
    WifiJoinDenied,
    /// The camera Wi-Fi password was rejected (`wifi-wrong-password`).
    WifiWrongPassword,
    /// The camera network name was rejected (`wifi-invalid-ssid`).
    WifiInvalidSsid,
    /// The system refused the Wi-Fi configuration (`wifi-invalid-config`).
    WifiInvalidConfig,
    /// The join did not complete, but the camera may still be reachable
    /// (`wifi-join-failed`). HTTP verifies before it is shown.
    WifiJoinFailed,
    /// The join did not settle in time (`wifi-join-timeout`).
    WifiJoinTimeout,
    /// A terminal join failure with no more specific reason. Shown as
    /// `wifi-invalid-config`.
    WifiJoinRejected,
    /// The phone joined but still routes elsewhere
    /// (`verify-network-not-ready`).
    VerifyNetworkNotReady,
    /// The phone's own network answers 192.168.0.1 (`verify-subnet-conflict`).
    SubnetConflict,
    /// Something other than a GR camera answered (`verify-not-gr-camera`).
    NotGrCamera,
    /// Local network access is denied (`verify-local-network-denied`).
    LocalNetworkDenied,
    /// Wi-Fi joined but the camera did not answer (`verify-failed`).
    VerifyFailed,
    /// HTTP 503 while the SD card is still mounting.
    CardNotReady,
    /// Camera has no GRKit service or an older version than 1.7.0.
    NeedsGrkit,
    /// The camera's GRKit speaks a newer service protocol than this app.
    AppTooOld,
    /// Unexpected HTTP or transport failure.
    Http(String),
}

/// What the error sheet offers besides closing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorAction {
    /// Try the connection again.
    Retry,
    /// Try again, and offer the app's system settings.
    RetryOrSettings,
    /// Try again once the phone is on another Wi-Fi network.
    RetryAfterSwitch,
    /// The only fix is in the system settings.
    Settings,
    /// Nothing to retry; the user acts elsewhere first.
    None,
}

impl LinkError {
    /// String keys of the title and body (`strings.rs`), and the actions.
    pub fn copy(&self) -> (&'static str, &'static str, ErrorAction) {
        use ErrorAction::*;
        match self {
            LinkError::BleConnectFailed | LinkError::ConnectionLost => {
                ("conn.ble_failed.title", "conn.ble_failed.body", Retry)
            }
            LinkError::BleAuthentication | LinkError::PairingFailed => {
                ("conn.ble_auth.title", "conn.ble_auth.body", Retry)
            }
            LinkError::NotPaired | LinkError::BleDisconnected => {
                ("conn.ble_pair.title", "conn.ble_pair.body", Retry)
            }
            LinkError::BlePairingStale => ("conn.ble_stale.title", "conn.ble_stale.body", Retry),
            LinkError::BleTimeout | LinkError::NotNearby => {
                ("conn.ble_timeout.title", "conn.ble_timeout.body", Retry)
            }
            LinkError::BluetoothUnavailable => ("error.bt_title", "error.bt_body", Settings),
            LinkError::WifiJoinDenied => (
                "conn.wifi_denied.title",
                "conn.wifi_denied.body",
                RetryOrSettings,
            ),
            // A retry reads the credentials from the camera again, so it
            // succeeds once the camera's Wi-Fi settings have changed.
            LinkError::WifiWrongPassword => {
                ("conn.wifi_password.title", "conn.wifi_password.body", Retry)
            }
            LinkError::WifiInvalidSsid => ("conn.wifi_ssid.title", "conn.wifi_ssid.body", Retry),
            LinkError::WifiInvalidConfig => {
                ("conn.wifi_config.title", "conn.wifi_config.body", Retry)
            }
            // Other terminal join failures read as the access point not ready.
            LinkError::WifiJoinFailed | LinkError::WifiJoinRejected => {
                ("conn.wifi_failed.title", "conn.wifi_failed.body", Retry)
            }
            LinkError::WifiJoinTimeout => {
                ("conn.wifi_timeout.title", "conn.wifi_timeout.body", Retry)
            }
            LinkError::VerifyNetworkNotReady => (
                "conn.network_not_ready.title",
                "conn.network_not_ready.body",
                RetryOrSettings,
            ),
            LinkError::SubnetConflict => {
                ("conn.subnet.title", "conn.subnet.body", RetryAfterSwitch)
            }
            LinkError::NotGrCamera => ("conn.not_gr.title", "conn.not_gr.body", None),
            LinkError::LocalNetworkDenied => (
                "conn.local_network.title",
                "conn.local_network.body",
                Settings,
            ),
            LinkError::VerifyFailed => ("conn.verify.title", "conn.verify.body", Retry),
            LinkError::CardNotReady => ("error.card_ready_title", "error.card_ready_body", Retry),
            LinkError::NeedsGrkit => ("cam.no_grkit_title", "cam.no_grkit_body", Retry),
            LinkError::AppTooOld => ("conn.app_old.title", "conn.app_old.body", None),
            LinkError::Http(_) => ("conn.unsupported.title", "conn.unsupported.body", None),
            LinkError::Busy | LinkError::Cancelled => {
                ("conn.unknown.title", "conn.unknown.body", Retry)
            }
        }
    }

    /// The connect step that failed (0 Bluetooth, 1 Wi-Fi, 2 the camera),
    /// or `None` for an error that comes before any step.
    pub fn failed_step(&self) -> Option<u8> {
        match self {
            LinkError::BluetoothUnavailable | LinkError::LocalNetworkDenied => None,
            LinkError::BleConnectFailed
            | LinkError::BleAuthentication
            | LinkError::PairingFailed
            | LinkError::NotPaired
            | LinkError::BleDisconnected
            | LinkError::BlePairingStale
            | LinkError::BleTimeout
            | LinkError::NotNearby => Some(0),
            LinkError::WifiJoinDenied
            | LinkError::WifiWrongPassword
            | LinkError::WifiInvalidSsid
            | LinkError::WifiInvalidConfig
            | LinkError::WifiJoinFailed
            | LinkError::WifiJoinTimeout
            | LinkError::WifiJoinRejected => Some(1),
            _ => Some(2),
        }
    }

    /// The camera must be paired on its own menu first.
    pub fn needs_pairing(&self) -> bool {
        matches!(self, LinkError::NotPaired | LinkError::BleDisconnected)
    }

    /// Codes that suggest the join worked but the first verify lost a race:
    /// worth one silent verify pass before the error shows.
    pub fn is_transient_verify(&self) -> bool {
        matches!(
            self,
            LinkError::VerifyFailed
                | LinkError::WifiJoinTimeout
                | LinkError::WifiJoinFailed
                | LinkError::ConnectionLost
        )
    }

    /// Errors that the camera network appearing can disprove: an automatic
    /// connect started by that network may run over their sheet. Errors only
    /// the user can fix (pairing, password, permission) stay.
    pub fn is_auto_retryable(&self) -> bool {
        self.is_transient_verify()
            || matches!(
                self,
                LinkError::Busy | LinkError::VerifyNetworkNotReady | LinkError::SubnetConflict
            )
    }

    /// Join errors after which an HTTP probe may still find the camera.
    pub fn is_soft_join_failure(&self) -> bool {
        matches!(self, LinkError::WifiJoinFailed | LinkError::WifiJoinTimeout)
    }
}

/// A camera seen while scanning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NearbyCamera {
    pub peripheral_id: String,
    pub model: CameraModel,
    pub powered_on: bool,
    /// The Bluetooth name (`GR_92910A`), once a frame carried it.
    pub name: Option<String>,
    /// Signal strength in dBm; absent when the peripheral was retrieved without an advertisement.
    pub rssi: Option<i32>,
}

/// Talks to a camera: connect, read package state, send an update.
pub trait CameraLink {
    fn nearby(&self) -> Option<CameraModel>;
    fn connect(&mut self) -> impl std::future::Future<Output = Result<CameraStatus, LinkError>>;
    fn read_status(&self) -> impl std::future::Future<Output = Result<CameraStatus, LinkError>>;
    fn read_package(&self) -> impl std::future::Future<Output = Result<PackageInfo, LinkError>>;
    fn send_update(
        &mut self,
        kind: UpdateKind,
    ) -> impl std::future::Future<Output = Result<(), LinkError>>;
    fn poll_update(&mut self) -> impl std::future::Future<Output = Result<UpdatePoll, LinkError>>;
    fn cancel(&mut self) -> impl std::future::Future<Output = Result<(), LinkError>>;
}

/// Poll a future that is known to be ready (demo / unit tests).
pub fn block_on_ready<T>(fut: impl std::future::Future<Output = T>) -> T {
    use std::task::{Context, Poll, Waker};
    let mut fut = std::pin::pin!(fut);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(v) => v,
        Poll::Pending => panic!("future was not ready"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_sheets_offer_what_can_fix_them() {
        let action = |error: LinkError| error.copy().2;
        assert_eq!(action(LinkError::NeedsGrkit), ErrorAction::Retry);
        assert_eq!(
            action(LinkError::SubnetConflict),
            ErrorAction::RetryAfterSwitch
        );
        assert_eq!(
            action(LinkError::WifiJoinDenied),
            ErrorAction::RetryOrSettings
        );
        assert_eq!(
            action(LinkError::VerifyNetworkNotReady),
            ErrorAction::RetryOrSettings
        );
        assert_eq!(action(LinkError::LocalNetworkDenied), ErrorAction::Settings);
        // A retry reads the camera's Wi-Fi settings again.
        for error in [
            LinkError::WifiWrongPassword,
            LinkError::WifiInvalidSsid,
            LinkError::WifiInvalidConfig,
        ] {
            assert_eq!(action(error), ErrorAction::Retry);
        }
        assert_eq!(action(LinkError::NotGrCamera), ErrorAction::None);
    }

    #[test]
    fn only_network_shaped_errors_give_way_to_an_automatic_connect() {
        for error in [
            LinkError::VerifyFailed,
            LinkError::WifiJoinTimeout,
            LinkError::WifiJoinFailed,
            LinkError::ConnectionLost,
            LinkError::VerifyNetworkNotReady,
            LinkError::SubnetConflict,
        ] {
            assert!(error.is_auto_retryable(), "{error:?}");
        }
        for error in [
            LinkError::NotPaired,
            LinkError::PairingFailed,
            LinkError::WifiWrongPassword,
            LinkError::WifiJoinDenied,
            LinkError::LocalNetworkDenied,
            LinkError::BleTimeout,
            LinkError::NeedsGrkit,
        ] {
            assert!(!error.is_auto_retryable(), "{error:?}");
        }
    }
}
