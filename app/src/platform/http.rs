//! Poll the camera GRKit HTTP service after Wi-Fi join (or against a dev URL).

use std::time::{Duration, Instant};

use nsh_core::prelude::*;

use crate::domain::{
    CameraModel, CameraStatus, LinkError, PackageInfo, PackageMapError, STATUS_POLL_BUDGET_MS,
    StatusAttempt, classify_props_response, in_ble_startup, installed_to_models, parse_props,
    status_backoff_ms, status_poll_timeout_error,
};

use super::camera_http_base;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

async fn camera_get(url: String) -> Result<nsh_core::element::HttpResponse, LinkError> {
    camera_get_within(url, REQUEST_TIMEOUT).await
}

async fn camera_get_within(
    url: String,
    timeout: Duration,
) -> Result<nsh_core::element::HttpResponse, LinkError> {
    super::camera_request("GET", url, Vec::new(), timeout, &[])
        .await
        .map_err(|_| LinkError::ConnectionLost)
}

/// Wait until `/v1/props` answers successfully.
///
/// Retries transport failures and HTTP 503 within the poll budget. A body that
/// does not parse ends the connect at once. On timeout: all unreachable →
/// [`LinkError::NeedsGrkit`] only if the camera API is reachable; otherwise
/// [`LinkError::ConnectionLost`]. A last 503 answer means [`LinkError::CardNotReady`].
pub async fn wait_until_ready() -> Result<(), LinkError> {
    let base = camera_http_base();
    app_info!("HTTP wait props at {base}");
    let deadline = Instant::now() + Duration::from_millis(STATUS_POLL_BUDGET_MS);
    let mut attempt = 0u32;
    loop {
        let last_was_503 = match get_props_once(&base).await {
            StatusAttempt::Ready => {
                app_info!("HTTP props ready");
                return Ok(());
            }
            StatusAttempt::Fatal(err) => {
                log::warn!("HTTP props fatal: {err:?}");
                return Err(if service_needs_newer_app(&base).await {
                    LinkError::AppTooOld
                } else {
                    err
                });
            }
            StatusAttempt::CardNotReady => true,
            StatusAttempt::Unreachable => false,
        };
        if Instant::now() >= deadline {
            log::warn!("HTTP props timed out (last_503={last_was_503})");
            return Err(if last_was_503 {
                status_poll_timeout_error(true)
            } else if probe_camera_api(&base).await {
                LinkError::NeedsGrkit
            } else {
                LinkError::ConnectionLost
            });
        }
        let delay = status_backoff_ms(attempt);
        attempt = attempt.saturating_add(1);
        timer(Duration::from_millis(delay)).await;
    }
}

/// A valid GRKit props response is stronger evidence than iOS association bookkeeping.
#[cfg(target_os = "ios")]
pub(crate) async fn service_ready() -> bool {
    get_props_once(&camera_http_base()).await == StatusAttempt::Ready
}

async fn get_props_once(base: &str) -> StatusAttempt {
    let url = format!("{base}/v1/props");
    let response = match camera_get(url).await {
        Ok(response) => response,
        Err(e) => {
            log::warn!("HTTP props unreachable: {e:?}");
            return StatusAttempt::Unreachable;
        }
    };
    let body = response.text();
    let attempt = classify_props_response(response.status(), &body);
    if let StatusAttempt::Fatal(LinkError::Http(ref msg)) = attempt
        && msg.contains("props body")
    {
        log::warn!("HTTP props parse failed len={} detail={msg}", body.len());
    }
    attempt
}

/// GET `/v1/props` and map installed GRKit into UI models.
pub async fn read_package(
    fallback_model: CameraModel,
) -> Result<(CameraStatus, PackageInfo), LinkError> {
    read_package_within(fallback_model, REQUEST_TIMEOUT).await
}

/// [`read_package`] with one request bounded by `timeout`.
pub async fn read_package_within(
    fallback_model: CameraModel,
    timeout: Duration,
) -> Result<(CameraStatus, PackageInfo), LinkError> {
    let base = camera_http_base();
    let url = format!("{base}/v1/props");
    app_info!("HTTP read props at {base}");
    let response = camera_get_within(url, timeout).await?;
    if crate::domain::should_retry_status(response.status()) {
        return Err(LinkError::CardNotReady);
    }
    if !response.ok() {
        return Err(LinkError::Http(format!("props HTTP {}", response.status())));
    }
    if service_needs_newer_app(&base).await {
        return Err(LinkError::AppTooOld);
    }
    let text = response.text();
    let props = parse_props(&text).map_err(|e| {
        log::warn!("HTTP props parse failed len={} err={e}", text.len());
        LinkError::Http(format!("props body ({} bytes): {e}", text.len()))
    })?;
    let Some(version) = props.grkit.version.as_deref().filter(|v| !v.is_empty()) else {
        log::warn!(
            "HTTP props without a GRKit version: {}",
            text.chars().take(600).collect::<String>()
        );
        return Err(LinkError::NeedsGrkit);
    };
    let (camera, info) = installed_to_models(version, props.camera.as_ref(), fallback_model)
        .map_err(|e| match e {
            PackageMapError::NeedsGrkit => LinkError::NeedsGrkit,
        })?;
    Ok((
        CameraStatus {
            camera,
            powered_on: true,
            device_id: props.device_id,
        },
        info,
    ))
}

/// The service speaks a newer protocol than this app (`GET /v1/ping`). A
/// service that does not answer the ping is not newer.
async fn service_needs_newer_app(base: &str) -> bool {
    match camera_get(format!("{base}/v1/ping")).await {
        Ok(response) if response.ok() => crate::domain::needs_newer_app(&response.text()),
        _ => false,
    }
}

/// Whether the camera's own HTTP API (port 80) answers.
#[cfg_attr(not(any(target_os = "ios", target_os = "android")), allow(dead_code))]
pub(crate) async fn camera_api_answers() -> bool {
    probe_camera_api(&camera_http_base()).await
}

/// The host of the camera in a GRKit service base URL.
fn camera_host(base: &str) -> Option<&str> {
    base.strip_prefix("http://")
        .and_then(|rest| rest.split('/').next())
        .and_then(|authority| authority.split(':').next())
}

/// Ask the camera's own API (port 80) to end the session: `wlan/finish`
/// closes its access point, `finish` turns it off. Returns whether the camera
/// accepted. GR III and IIIx may reject the power off.
pub(crate) async fn camera_finish(power_off: bool) -> bool {
    let Some(host) = camera_host(&camera_http_base()).map(str::to_owned) else {
        return false;
    };
    let (path, budget) = if power_off {
        ("device/finish", POWER_OFF_TIMEOUT)
    } else {
        ("device/wlan/finish", FINISH_WLAN_TIMEOUT)
    };
    let url = format!("http://{host}/v1/{path}");
    match super::camera_request("POST", url, Vec::new(), budget, &[]).await {
        Ok(response) if response.ok() => {
            serde_json::from_str::<serde_json::Value>(&response.text())
                .map(|body| {
                    body.get("errCode")
                        .and_then(|c| c.as_i64())
                        .is_none_or(|c| c == 200)
                })
                .unwrap_or(true)
        }
        Ok(response) => {
            app_info!("camera {path} answered {}", response.status());
            false
        }
        Err(error) => {
            app_info!("camera {path} failed: {error}");
            false
        }
    }
}

/// The time the camera gets to answer a request to close its access point.
const FINISH_WLAN_TIMEOUT: Duration = Duration::from_secs(2);
/// The time the camera gets to answer a power off.
const POWER_OFF_TIMEOUT: Duration = Duration::from_secs(3);

/// Logs whether the camera's own HTTP API (port 80) answers, to tell a
/// missing GRKit service from a network the phone cannot reach.
/// A camera that a Bluetooth wake started stays in `bleStartup`, with its
/// screen off. Switch it to playback through its own API, as the camera
/// app does after a wake; the camera accepts the switch only from
/// `bleStartup`.
pub(crate) async fn leave_ble_startup() {
    let base = camera_http_base();
    let Some(host) = camera_host(&base) else {
        return;
    };
    let in_startup = match camera_get(format!("http://{host}/v1/props")).await {
        Ok(response) if response.ok() => in_ble_startup(&response.text()),
        _ => false,
    };
    app_info!("camera in bleStartup: {in_startup}");
    if !in_startup {
        return;
    }
    let url = format!("http://{host}/v1/params/device");
    let headers = [(
        "Content-Type",
        "application/x-www-form-urlencoded".to_owned(),
    )];
    for attempt in 1..=PLAYBACK_SWITCH_ATTEMPTS {
        match super::camera_request(
            "PUT",
            url.clone(),
            b"operationMode=playback".to_vec(),
            REQUEST_TIMEOUT,
            &headers,
        )
        .await
        {
            // The camera answers a refusal with HTTP 200 and its errCode.
            Ok(response) if response.ok() && accepted(&response.text()) => {
                app_info!("camera switched to playback (attempt {attempt})");
                return;
            }
            Ok(response) => log::warn!("playback switch HTTP {}", response.status()),
            Err(e) => log::warn!("playback switch failed: {e}"),
        }
        timer(PLAYBACK_SWITCH_RETRY).await;
    }
}

fn accepted(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|b| b.get("errCode").and_then(|c| c.as_i64()))
        .is_none_or(|code| code == 200)
}

const PLAYBACK_SWITCH_ATTEMPTS: u32 = 3;
const PLAYBACK_SWITCH_RETRY: Duration = Duration::from_millis(500);

async fn probe_camera_api(base: &str) -> bool {
    let Some(host) = camera_host(base) else {
        return false;
    };
    match camera_get(format!("http://{host}/v1/props")).await {
        Ok(response) if response.ok() => {
            serde_json::from_str::<serde_json::Value>(&response.text())
                .is_ok_and(|body| body.get("model").is_some() || body.get("errCode").is_some())
        }
        _ => false,
    }
}
