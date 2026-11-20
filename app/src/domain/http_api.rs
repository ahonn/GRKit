//! Parse camera HTTP props/package JSON and decide readiness retries.
//!
//! Shapes match `grkit-serve` (`GET /v1/props`, `GET /v1/update` inventory).

use serde::Deserialize;

use super::PackageInfo;
use super::camera::{Camera, CameraModel, ConnectionState};
use super::link::LinkError;

/// Base URL of the GRKit service on the camera access point.
pub const CAMERA_HTTP_BASE: &str = "http://192.168.0.1:8080";

/// How long to keep polling `/v1/props` after joining Wi-Fi.
pub const STATUS_POLL_BUDGET_MS: u64 = 30_000;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropsCamera {
    pub model: Option<String>,
    #[serde(alias = "firmware")]
    pub firmware_version: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct PropsGrkit {
    pub version: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct PropsStorage {
    pub state: Option<String>,
    /// Free bytes on the card, when the service could read them.
    #[serde(default, rename = "freeBytes")]
    pub free_bytes: Option<u64>,
}

/// `/v1/props` body from the camera service.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropsResponse {
    /// A stable identity of the camera body (a hash of its serial number);
    /// absent until the camera's own API has answered the service.
    #[serde(default)]
    pub device_id: Option<String>,
    pub boot_id: String,
    pub card_id: Option<String>,
    pub camera: Option<PropsCamera>,
    pub grkit: PropsGrkit,
    #[serde(default)]
    pub storage: Option<PropsStorage>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PackageCamera {
    pub model: Option<String>,
    pub firmware: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct PackageResponse {
    pub version: Option<String>,
    pub modules: Option<serde_json::Value>,
    pub effects: Option<serde_json::Value>,
    pub left_out: Option<serde_json::Value>,
    /// Null when the camera's own API has not answered yet (still a valid package).
    pub camera: Option<PackageCamera>,
}

/// Parse a `/v1/props` body.
pub fn parse_props(body: &str) -> Result<PropsResponse, String> {
    serde_json::from_str(body).map_err(|e| e.to_string())
}

/// Parse an inventory package object (from `GET /v1/update`).
pub fn parse_package(body: &str) -> Result<PackageResponse, String> {
    serde_json::from_str(body).map_err(|e| e.to_string())
}

/// 503 means the card or runtime is not ready or busy, and 409 that the
/// card identity became known during the request; keep retrying within the
/// poll budget.
pub fn should_retry_status(status_code: u16) -> bool {
    matches!(status_code, 409 | 503)
}

/// Outcome of one `/v1/props` attempt (for the poll loop).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatusAttempt {
    /// Parsed a successful props body.
    Ready,
    /// HTTP 503 or 409: the card is not ready yet; retry.
    CardNotReady,
    /// Could not reach the service (connection refused, no route, etc.); retry.
    Unreachable,
    /// Non-retryable failure (bad JSON, unexpected HTTP status).
    Fatal(LinkError),
}

/// Classify an HTTP response to `/v1/props` (not transport failures).
pub fn classify_props_response(status_code: u16, body: &str) -> StatusAttempt {
    if should_retry_status(status_code) {
        return StatusAttempt::CardNotReady;
    }
    if !(200..300).contains(&status_code) {
        return StatusAttempt::Fatal(LinkError::Http(format!("props HTTP {status_code}")));
    }
    match parse_props(body) {
        Ok(_) => StatusAttempt::Ready,
        Err(err) => StatusAttempt::Fatal(LinkError::Http(format!(
            "props body ({} bytes): {err}",
            body.len()
        ))),
    }
}

/// Whether the camera's own `/v1/props` says it is in `bleStartup`, the mode
/// a Bluetooth wake starts it in. GRKit does not load in that mode; the
/// camera loads it once switched to playback.
pub fn in_ble_startup(native_props: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(native_props)
        .ok()
        .and_then(|body| {
            body.get("operationMode")
                .and_then(|m| m.as_str())
                .map(|m| m == "bleStartup")
        })
        .unwrap_or(false)
}

/// Which error to surface when the props poll budget runs out.
pub fn status_poll_timeout_error(last_was_503: bool) -> LinkError {
    if last_was_503 {
        LinkError::CardNotReady
    } else {
        LinkError::NeedsGrkit
    }
}

/// Compare dotted numeric versions (`1.7.0`). Non-numeric parts sort as 0.
pub fn version_at_least(version: &str, minimum: &str) -> bool {
    let a = parse_version_parts(version);
    let b = parse_version_parts(minimum);
    a >= b
}

fn parse_version_parts(v: &str) -> [u32; 3] {
    let mut parts = [0u32; 3];
    for (i, piece) in v.split('.').take(3).enumerate() {
        let digits: String = piece.chars().take_while(|c| c.is_ascii_digit()).collect();
        parts[i] = digits.parse().unwrap_or(0);
    }
    parts
}

/// Map installed version + camera metadata into UI models.
pub fn installed_to_models(
    version: &str,
    camera: Option<&PropsCamera>,
    fallback_model: CameraModel,
) -> Result<(Camera, PackageInfo), PackageMapError> {
    if version.trim().is_empty() {
        return Err(PackageMapError::NeedsGrkit);
    }
    let installed = version.to_owned();
    // Only GRKit 1.7.0 and later run this service, so an answer is enough:
    // the reported version is not checked again (development builds report
    // the version of their branch).
    let model = camera
        .and_then(|c| c.model.as_deref())
        .and_then(model_from_name)
        .unwrap_or(fallback_model);
    let firmware = camera
        .and_then(|c| c.firmware_version.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "—".into());
    let camera = Camera {
        model,
        firmware: firmware.clone(),
        installed_version: installed.clone(),
        connection: ConnectionState::Connected,
    };
    let info = PackageInfo {
        installed,
        available: None,
        firmware,
        release_notes: String::new(),
        package_bytes: 0,
    };
    Ok((camera, info))
}

/// Map a package response into UI models. `fallback_model` is used when the
/// package omits camera info or a recognizable model string.
pub fn package_to_models(
    package: &PackageResponse,
    fallback_model: CameraModel,
) -> Result<(Camera, PackageInfo), PackageMapError> {
    let installed = package
        .version
        .clone()
        .filter(|v| !v.is_empty())
        .ok_or(PackageMapError::NeedsGrkit)?;
    let props_camera = package.camera.as_ref().map(|c| PropsCamera {
        model: c.model.clone(),
        firmware_version: c.firmware.clone(),
    });
    installed_to_models(&installed, props_camera.as_ref(), fallback_model)
}

/// Why package JSON cannot become a usable camera session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageMapError {
    NeedsGrkit,
}

pub(crate) fn model_from_name(name: &str) -> Option<CameraModel> {
    let upper = name.to_ascii_uppercase();
    if upper.contains("IIIX") || upper.contains("GR3X") {
        Some(CameraModel::Gr3x)
    } else if upper.contains("IV") || upper.contains("GR4") {
        Some(CameraModel::Gr4)
    } else if upper.contains("III") || upper.contains("GR3") {
        Some(CameraModel::Gr3)
    } else {
        None
    }
}

/// Next delay before another props poll (ms), capped for the 30s budget.
pub fn status_backoff_ms(attempt: u32) -> u64 {
    let base = 600u64;
    let delay = base.saturating_mul(1u64 << attempt.min(4));
    delay.min(3_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_ble_startup_needs_the_playback_switch() {
        assert!(in_ble_startup(
            r#"{"model":"RICOH GR III","operationMode":"bleStartup"}"#
        ));
        assert!(!in_ble_startup(r#"{"operationMode":"playback"}"#));
        assert!(!in_ble_startup(r#"{"model":"RICOH GR III"}"#));
        assert!(!in_ble_startup("not json"));
    }
    use serde_json::json;

    const PROPS_READY: &str = r#"{"bootId":"boot-1","cardId":"card-1","camera":{"model":"RICOH GR III","firmwareVersion":"2.21"},"grkit":{"version":"1.7.0"},"storage":{"state":"ready"}}"#;
    const PACKAGE_WITH_CAMERA: &str = r#"{"version":"1.7.0","modules":[],"effects":[],"left_out":[],"camera":{"model":"RICOH GR III","firmware":"2.21"}}"#;
    const PACKAGE_CAMERA_NULL: &str = r#"{"version":"1.7.0","modules":[],"effects":[],"left_out":["LEFT_OUT.GRF"],"camera":null}"#;

    #[test]
    fn parses_props_like_service() {
        let props = parse_props(PROPS_READY).unwrap();
        assert_eq!(props.boot_id, "boot-1");
        assert_eq!(props.card_id.as_deref(), Some("card-1"));
        assert_eq!(props.grkit.version.as_deref(), Some("1.7.0"));
        assert_eq!(
            props.camera.as_ref().and_then(|c| c.model.as_deref()),
            Some("RICOH GR III")
        );
    }

    #[test]
    fn rejects_wrong_props_types_as_parse_error() {
        let err = parse_props(r#"{"bootId":1,"grkit":{"version":"1.7.0"}}"#).unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn parse_failure_is_fatal_not_retry() {
        let body = r#"{"bootId":1}"#;
        match classify_props_response(200, body) {
            StatusAttempt::Fatal(LinkError::Http(msg)) => {
                assert!(msg.contains("props body"));
                assert!(msg.contains(&format!("{} bytes", body.len())));
            }
            other => panic!("expected Fatal, got {other:?}"),
        }
    }

    #[test]
    fn classifies_503_as_retry_card() {
        assert_eq!(
            classify_props_response(
                503,
                r#"{"type":"about:blank","title":"Service Unavailable","status":503,"code":"card_unavailable","detail":"The card is not ready."}"#
            ),
            StatusAttempt::CardNotReady
        );
    }

    #[test]
    fn classifies_a_changed_card_context_as_retry() {
        assert_eq!(
            classify_props_response(
                409,
                r#"{"type":"about:blank","title":"Conflict","status":409,"code":"context_changed","detail":"The card changed while reading properties."}"#
            ),
            StatusAttempt::CardNotReady
        );
    }

    #[test]
    fn classifies_ok_props_as_ready() {
        assert_eq!(
            classify_props_response(200, PROPS_READY),
            StatusAttempt::Ready
        );
    }

    #[test]
    fn timeout_all_unreachable_needs_grkit() {
        assert_eq!(status_poll_timeout_error(false), LinkError::NeedsGrkit);
    }

    #[test]
    fn timeout_last_503_is_card_not_ready() {
        assert_eq!(status_poll_timeout_error(true), LinkError::CardNotReady);
    }

    #[test]
    fn parses_package_with_camera() {
        let p = parse_package(PACKAGE_WITH_CAMERA).unwrap();
        let (cam, pkg) = package_to_models(&p, CameraModel::Gr3).unwrap();
        assert_eq!(cam.model, CameraModel::Gr3);
        assert_eq!(pkg.installed, "1.7.0");
        assert_eq!(cam.firmware, "2.21");
    }

    #[test]
    fn null_camera_uses_fallback_model() {
        let p = parse_package(PACKAGE_CAMERA_NULL).unwrap();
        let (cam, pkg) = package_to_models(&p, CameraModel::Gr4).unwrap();
        assert_eq!(cam.model, CameraModel::Gr4);
        assert_eq!(pkg.installed, "1.7.0");
        assert_eq!(cam.firmware, "—");
        assert_eq!(pkg.firmware, "—");
    }

    #[test]
    fn any_answering_service_is_supported() {
        let p =
            parse_package(r#"{"version":"1.6.1","camera":{"model":"GR III","firmware":"2.00"}}"#)
                .unwrap();
        let (cam, pkg) = package_to_models(&p, CameraModel::Gr3).unwrap();
        assert_eq!(cam.installed_version, "1.6.1");
        assert_eq!(pkg.installed, "1.6.1");
    }

    #[test]
    fn null_version_needs_grkit() {
        let p = parse_package(r#"{"version":null,"camera":null}"#).unwrap();
        assert_eq!(
            package_to_models(&p, CameraModel::Gr3),
            Err(PackageMapError::NeedsGrkit)
        );
    }

    #[test]
    fn props_to_models() {
        let props = parse_props(PROPS_READY).unwrap();
        let (cam, pkg) = installed_to_models(
            props.grkit.version.as_deref().unwrap(),
            props.camera.as_ref(),
            CameraModel::Gr3,
        )
        .unwrap();
        assert_eq!(pkg.installed, "1.7.0");
        assert_eq!(cam.firmware, "2.21");
        assert_eq!(json!(props.storage.as_ref().unwrap().state), json!("ready"));
    }

    #[test]
    fn retries_only_503() {
        assert!(should_retry_status(503));
        assert!(should_retry_status(409));
        assert!(!should_retry_status(200));
        assert!(!should_retry_status(404));
    }

    #[test]
    fn version_compare() {
        assert!(version_at_least("1.7.0", "1.7.0"));
        assert!(version_at_least("1.7.1", "1.7.0"));
        assert!(!version_at_least("1.6.9", "1.7.0"));
    }

    #[test]
    fn backoff_grows() {
        assert_eq!(status_backoff_ms(0), 600);
        assert!(status_backoff_ms(3) >= status_backoff_ms(1));
        assert_eq!(status_backoff_ms(10), 3_000);
    }
}
