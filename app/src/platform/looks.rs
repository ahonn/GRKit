//! Camera Looks over HTTP (`/v1/looks`) and the official Look catalog.
//!
//! Each mutation names the camera start that the phone read the Looks in
//! (`X-GRKit-Boot-Id`): after a restart the Looks that wait for a start
//! differ, and the camera refuses the change. On or off and the order apply
//! at once. A new, renamed, replaced, or removed Look, and a new cover, is a
//! file change, which the camera reads when it starts again. A mutation is never replayed
//! automatically: after an unknown result the store reads the Looks again.

use std::{fs, path::PathBuf, time::Duration};

use grkit_core::{
    camera,
    cover::{Cover, Scheme},
    packaging,
};
use nsh_core::{
    element::HttpResponse,
    platform::HttpRequest,
    prelude::{data_dir, fetch},
};

use super::camera_http_base;
use crate::domain::{
    CameraModel, Catalog, CatalogLook, CoverGroup, LookError, LooksSnapshot, look_error,
    parse_catalog, parse_looks,
};

/// A Looks read: the camera service reads each Look file from the card.
const READ_TIMEOUT: Duration = Duration::from_secs(15);
const WRITE_TIMEOUT: Duration = Duration::from_secs(15);
const CATALOG_LIMIT: usize = 4 * 1024 * 1024;
const GRF_LIMIT: usize = 16 * 1024;

/// The camera start of the last Looks read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraContext {
    pub boot_id: String,
}

impl From<&LooksSnapshot> for CameraContext {
    fn from(snapshot: &LooksSnapshot) -> Self {
        Self {
            boot_id: snapshot.boot_id.clone(),
        }
    }
}

async fn request(
    method: &str,
    path: &str,
    body: Vec<u8>,
    budget: Duration,
    headers: &[(&str, String)],
) -> Result<HttpResponse, LookError> {
    super::camera_request(
        method,
        format!("{}{path}", camera_http_base()),
        body,
        budget,
        headers,
    )
    .await
    .map_err(|_| LookError::ConnectionLost)
}

fn failure(response: &HttpResponse) -> LookError {
    look_error(response.status(), &response.text())
}

/// The camera's Looks in menu order.
pub async fn list() -> Result<LooksSnapshot, LookError> {
    let response = request("GET", "/v1/looks", Vec::new(), READ_TIMEOUT, &[]).await?;
    if !response.ok() {
        let body = response.text();
        log::warn!(
            "GET /v1/looks: HTTP {}: {}",
            response.status(),
            body.chars().take(400).collect::<String>()
        );
        return Err(failure(&response));
    }
    parse_looks(&response.body).map_err(LookError::Service)
}

fn cache_dir() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("look-cache-v1"))
}

fn checked_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A Look file from the cache, when one with this content hash is there.
fn cached(hash: &str) -> Option<Vec<u8>> {
    if !checked_hash(hash) {
        return None;
    }
    let bytes = fs::read(cache_dir()?.join(format!("{hash}.grf"))).ok()?;
    (packaging::sha256(&bytes) == hash).then_some(bytes)
}

fn store(hash: &str, bytes: &[u8]) {
    let Some(dir) = cache_dir() else { return };
    if !checked_hash(hash) {
        return;
    }
    let save = || -> std::io::Result<()> {
        fs::create_dir_all(&dir)?;
        let temporary = dir.join(format!("{hash}.tmp"));
        fs::write(&temporary, bytes)?;
        fs::rename(temporary, dir.join(format!("{hash}.grf")))
    };
    if let Err(error) = save() {
        log::warn!("Could not cache a Look: {error}");
    }
}

/// The GRF bytes of a Look, from the cache when its hash matches.
pub async fn content(id: &str, content_hash: &str) -> Result<Vec<u8>, LookError> {
    if let Some(bytes) = cached(content_hash) {
        return Ok(bytes);
    }
    let response = request(
        "GET",
        &format!("/v1/looks/{id}/content"),
        Vec::new(),
        READ_TIMEOUT,
        &[],
    )
    .await?;
    if !response.ok() {
        return Err(failure(&response));
    }
    let bytes = response.body;
    if bytes.len() > GRF_LIMIT {
        return Err(LookError::Service("The Look file is too large.".into()));
    }
    let hash = packaging::sha256(&bytes);
    if hash != content_hash {
        // The Look changed after the list was read; show the new content.
        log::info!("Look {id} content changed since the list was read");
    }
    store(&hash, &bytes);
    Ok(bytes)
}

/// Send one mutation. The camera answers when the change ends.
async fn mutate(
    method: &str,
    path: &str,
    body: Vec<u8>,
    mut headers: Vec<(&str, String)>,
    context: &CameraContext,
) -> Result<(), LookError> {
    headers.push(("X-GRKit-Boot-Id", context.boot_id.clone()));
    let response = request(method, path, body, WRITE_TIMEOUT, &headers).await?;
    if response.ok() {
        Ok(())
    } else {
        Err(failure(&response))
    }
}

fn json_body(value: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&value).expect("JSON encodes")
}

fn json_type() -> Vec<(&'static str, String)> {
    vec![("Content-Type", "application/json".into())]
}

/// Show or hide a Look in the camera menu.
pub async fn set_enabled(
    context: &CameraContext,
    id: &str,
    enabled: bool,
) -> Result<(), LookError> {
    mutate(
        "PATCH",
        &format!("/v1/looks/{id}"),
        json_body(serde_json::json!({ "enabled": enabled })),
        json_type(),
        context,
    )
    .await
}

/// Rename an imported Look. The camera redraws a default cover for the new
/// name.
pub async fn rename(context: &CameraContext, id: &str, name: &str) -> Result<(), LookError> {
    mutate(
        "PATCH",
        &format!("/v1/looks/{id}"),
        json_body(serde_json::json!({ "name": name })),
        json_type(),
        context,
    )
    .await
}

/// Remove a Look. `active_off` switches the camera to Off when it is in use.
pub async fn delete(context: &CameraContext, id: &str, active_off: bool) -> Result<(), LookError> {
    let query = if active_off { "?active=off" } else { "" };
    mutate(
        "DELETE",
        &format!("/v1/looks/{id}{query}"),
        Vec::new(),
        Vec::new(),
        context,
    )
    .await
}

/// Replace the camera menu order with the complete list of IDs.
pub async fn set_order(context: &CameraContext, ids: &[String]) -> Result<(), LookError> {
    mutate(
        "PUT",
        "/v1/params/looks/order",
        json_body(serde_json::json!({ "ids": ids })),
        json_type(),
        context,
    )
    .await
}

/// Give the built-in Looks without an individual cover the covers of
/// `scheme`. The camera rewrites their files.
pub async fn set_cover_scheme(context: &CameraContext, scheme: Scheme) -> Result<(), LookError> {
    mutate(
        "PUT",
        "/v1/looks/covers/scheme",
        json_body(serde_json::json!({ "scheme": scheme.id() })),
        json_type(),
        context,
    )
    .await
}

/// Put the individual covers of the built-in Looks of `group` back to the
/// scheme of the card.
pub async fn reset_covers(context: &CameraContext, group: CoverGroup) -> Result<(), LookError> {
    mutate(
        "POST",
        "/v1/looks/covers/reset",
        json_body(serde_json::json!({ "group": group.id() })),
        json_type(),
        context,
    )
    .await
}

/// Add a Look, replace the content of an imported one with this ID, or give
/// a Look another cover. `cover` must draw the icon of `grf`; the camera
/// keeps it for the Look.
pub async fn put_content(
    context: &CameraContext,
    id: &str,
    grf: Vec<u8>,
    cover: Option<&Cover>,
) -> Result<(), LookError> {
    let mut headers = vec![("Content-Type", "application/octet-stream".to_owned())];
    if let Some(cover) = cover {
        // Compact JSON: cover texts are printable ASCII.
        headers.push(("X-GRKit-Cover", cover.to_json().to_string()));
    }
    mutate(
        "PUT",
        &format!("/v1/looks/{id}/content"),
        grf,
        headers,
        context,
    )
    .await
}

fn site_base() -> String {
    super::config_value("GRKIT_RELEASE_BASE")
        .unwrap_or_else(|| "https://grkit.app".into())
        .trim_end_matches('/')
        .to_owned()
}

async fn site_get(path: &str, limit: usize) -> Result<Vec<u8>, String> {
    let mut request =
        HttpRequest::get(format!("{}/{path}", site_base())).header("Cache-Control", "no-cache");
    request.follow_redirects = false;
    let response = super::with_timeout(fetch(request), Duration::from_secs(15))
        .await
        .ok_or_else(|| format!("Timed out downloading {path}"))??;
    if !response.ok() {
        return Err(format!("HTTP {}: {path}", response.status()));
    }
    if response.body.len() > limit {
        return Err(format!("{path} exceeds its size limit"));
    }
    Ok(response.body)
}

fn camera_for(model: CameraModel) -> &'static camera::Camera {
    match model {
        CameraModel::Gr3 => &camera::GR3,
        CameraModel::Gr3x => &camera::GR3X,
        CameraModel::Gr4 => &camera::GR4,
    }
}

/// The catalog target of a camera firmware.
pub fn target(model: CameraModel, firmware: &str) -> Option<&'static str> {
    let camera = camera_for(model);
    camera
        .releases
        .iter()
        .find(|release| release.firmware == firmware)
        .map(|release| release.target)
        .or_else(|| camera.targets().first().copied())
}

/// The official Look catalog of a firmware target. A cached copy serves
/// when the phone is offline.
pub async fn catalog(target: &str) -> Result<Catalog, String> {
    let name = format!("catalog-{target}.json");
    let cache = cache_dir();
    match site_get(&format!("effects/{target}/catalog.json"), CATALOG_LIMIT).await {
        Ok(bytes) => {
            let catalog = parse_catalog(&bytes)?;
            if let Some(dir) = cache {
                let _ = fs::create_dir_all(&dir).and_then(|_| fs::write(dir.join(&name), &bytes));
            }
            Ok(catalog)
        }
        Err(error) => {
            let bytes = cache
                .and_then(|dir| fs::read(dir.join(&name)).ok())
                .ok_or(error)?;
            parse_catalog(&bytes)
        }
    }
}

/// The GRF of a catalog Look, checked against the catalog digest.
pub async fn catalog_file(target: &str, look: &CatalogLook) -> Result<Vec<u8>, String> {
    if let Some(bytes) = cached(&look.sha256) {
        return Ok(bytes);
    }
    let bytes = site_get(&format!("effects/{target}/{}", look.file), GRF_LIMIT).await?;
    if packaging::sha256(&bytes) != look.sha256 {
        return Err(format!("{} failed its integrity check", look.name));
    }
    store(&look.sha256, &bytes);
    Ok(bytes)
}
