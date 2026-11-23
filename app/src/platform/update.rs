//! Camera software requests over HTTP (`/v1/update`). The store runs one at
//! a time. A mutation names the camera start that the phone read
//! (`X-GRKit-Boot-Id`) and is never replayed automatically: after an unknown
//! result the phone reads the resource again.
use super::camera_http_base;
use crate::domain::{
    JournalState, PackageResponse, SoftwareRequestSummary, UpdateError, UpdateIntent,
    UpdateResource, UpdateSnapshot, problem_code, response_error,
};
use nsh_core::prelude::data_dir;
use std::{fs, time::Duration};

const READ_TIMEOUT: Duration = Duration::from_secs(5);

async fn raw_request(
    method: &str,
    path: &str,
    body: Vec<u8>,
    budget: Duration,
    headers: &[(&str, String)],
) -> Result<nsh_core::element::HttpResponse, UpdateError> {
    super::camera_request(
        method,
        format!("{}{path}", camera_http_base()),
        body,
        budget,
        headers,
    )
    .await
    .map_err(|_| UpdateError::ConnectionLost)
}

fn decode_json<T: serde::de::DeserializeOwned>(
    response: &nsh_core::element::HttpResponse,
) -> Result<T, UpdateError> {
    if !response.ok() {
        return Err(response_error(response.status(), &response.text()));
    }
    serde_json::from_slice(&response.body)
        .map_err(|_| UpdateError::Service("The camera returned an invalid response.".into()))
}

async fn read(path: &str) -> Result<nsh_core::element::HttpResponse, UpdateError> {
    let response = raw_request("GET", path, Vec::new(), READ_TIMEOUT, &[]).await?;
    if response.status() == 503 {
        return Err(UpdateError::Service(
            "The card is not ready. Try again.".into(),
        ));
    }
    Ok(response)
}

pub async fn snapshot() -> Result<UpdateSnapshot, UpdateError> {
    let resource: UpdateResource = decode_json(&read("/v1/update").await?)?;
    let package = PackageResponse {
        version: resource.inventory.package.version,
        modules: resource.inventory.package.modules,
        effects: resource.inventory.package.effects,
        left_out: resource.inventory.package.left_out,
        camera: None,
    };
    Ok(UpdateSnapshot {
        state: resource.inventory.state,
        package,
        request: resource.request,
    })
}

pub async fn package() -> Result<PackageResponse, UpdateError> {
    let mut package = snapshot().await?.package;
    if let Ok(response) = raw_request("GET", "/v1/props", Vec::new(), READ_TIMEOUT, &[]).await
        && response.ok()
        && let Ok(props) = serde_json::from_slice::<crate::domain::PropsResponse>(&response.body)
    {
        package.camera = props.camera.map(|camera| crate::domain::PackageCamera {
            model: camera.model,
            firmware: camera.firmware_version,
        });
        if package.version.is_none() {
            package.version = props.grkit.version;
        }
    }
    Ok(package)
}

/// The camera start that the phone reads now. A software request names it,
/// so a request after a restart of the camera is refused.
pub async fn boot_id() -> Result<String, UpdateError> {
    let props: crate::domain::PropsResponse = decode_json(&read("/v1/props").await?)?;
    Ok(props.boot_id)
}

fn boot_header(boot_id: &str) -> (&'static str, String) {
    ("X-GRKit-Boot-Id", boot_id.to_owned())
}

/// Stage an update with the package `body`. The camera checks and writes the
/// whole package before it answers. When the answer does not arrive, or a
/// request already waits, the resource tells whether an update waits now.
pub async fn stage(body: Vec<u8>, boot_id: &str) -> Result<(), UpdateError> {
    let headers = [
        ("Content-Type", "application/zip".to_owned()),
        boot_header(boot_id),
    ];
    let response = raw_request(
        "PUT",
        "/v1/update",
        body,
        Duration::from_secs(180),
        &headers,
    )
    .await;
    let error = match response {
        Ok(response) if response.ok() => return Ok(()),
        Ok(response) => {
            let body = response.text();
            let error = response_error(response.status(), &body);
            if problem_code(&body).as_deref() != Some("software_pending") {
                return Err(error);
            }
            error
        }
        Err(error) => error,
    };
    let waiting = snapshot().await.ok().and_then(|s| s.request);
    if waiting.is_some_and(|request| request.waits()) {
        Ok(())
    } else {
        Err(error)
    }
}

/// Cancel the update that waits on the card, then check that the camera
/// journal no longer has one.
pub async fn cancel(boot_id: &str) -> Result<(), UpdateError> {
    let response = raw_request(
        "POST",
        "/v1/update/cancel",
        Vec::new(),
        Duration::from_secs(15),
        &[boot_header(boot_id)],
    )
    .await?;
    if response.ok() {
        let cancelled: SoftwareRequestSummary = decode_json(&response)?;
        log::info!(
            "Cancelled {} request {}: {:?}",
            cancelled.domain,
            cancelled.request_id,
            cancelled.status
        );
    } else {
        let body = response.text();
        // Nothing waits, as after a cancel whose answer was lost: the journal
        // decides.
        if problem_code(&body).as_deref() != Some("no_software_request") {
            return Err(response_error(response.status(), &body));
        }
    }
    if snapshot().await?.state != JournalState::None {
        return Err(UpdateError::Service(
            "The camera still has a pending update.".into(),
        ));
    }
    Ok(())
}

/// Free bytes on the card, when the camera reports them.
pub async fn free_bytes() -> Option<u64> {
    let props: crate::domain::PropsResponse = decode_json(&read("/v1/props").await.ok()?).ok()?;
    props.storage?.free_bytes
}

fn seen_path() -> Option<std::path::PathBuf> {
    Some(data_dir()?.join("seen_versions.json"))
}

fn seen_versions() -> std::collections::BTreeMap<String, String> {
    seen_path()
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// The GRKit version this phone last saw on the camera `camera_id`.
pub fn seen_version(camera_id: &str) -> Option<String> {
    seen_versions().remove(camera_id)
}

pub fn save_seen_version(camera_id: &str, version: &str) {
    let mut versions = seen_versions();
    if versions.get(camera_id).map(String::as_str) == Some(version) {
        return;
    }
    versions.insert(camera_id.to_owned(), version.to_owned());
    if let (Some(path), Ok(bytes)) = (seen_path(), serde_json::to_vec(&versions)) {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(path, bytes);
    }
}

pub fn load_intent() -> Option<UpdateIntent> {
    serde_json::from_slice(&fs::read(data_dir()?.join("pending_update.json")).ok()?).ok()
}

pub fn save_intent(intent: &UpdateIntent) -> Result<(), UpdateError> {
    let save = || -> Result<(), Box<dyn std::error::Error>> {
        let dir = data_dir().ok_or("App storage is unavailable")?;
        fs::create_dir_all(&dir)?;
        let temporary = dir.join("pending_update.tmp");
        fs::write(&temporary, serde_json::to_vec(intent)?)?;
        fs::rename(temporary, dir.join("pending_update.json"))?;
        Ok(())
    };
    save().map_err(|_| {
        UpdateError::Service("Could not save update recovery information. Try again.".into())
    })
}

pub fn clear_intent() {
    if let Some(dir) = data_dir() {
        let _ = fs::remove_file(dir.join("pending_update.json"));
    }
}
