//! Module settings over HTTP, and their cache on the phone.
//!
//! The cache holds the catalog, its icons, and the last state of each camera
//! in each language, so the app can show them without a camera and read the
//! catalog again only when its hash changes.

use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use nsh_core::{element::HttpResponse, prelude::data_dir};

use super::camera_http_base;
use crate::domain::{
    LookError,
    camera_settings::{CameraSettings, SettingChange, parse_camera_settings},
    modules::{
        CatalogAnswer, CatalogRead, ModuleCatalog, ModuleDraft, ModuleLayout, ModuleState,
        ModulesState, catalog_answer, module_error, parse_module_catalog, parse_module_patch,
        parse_modules_state,
    },
};

const TIMEOUT: Duration = Duration::from_secs(15);
const ICON_LIMIT: usize = 64 * 1024;

/// What `GET /v1/props` says about module settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleAccess {
    pub read: bool,
    pub write: bool,
    pub boot_id: String,
}

async fn request(
    method: &str,
    path: &str,
    body: Option<serde_json::Value>,
    headers: &[(&str, String)],
) -> Result<HttpResponse, LookError> {
    let mut all = headers.to_vec();
    let body = match body {
        Some(value) => {
            all.push(("Content-Type", "application/json".to_owned()));
            serde_json::to_vec(&value).expect("JSON encodes")
        }
        None => Vec::new(),
    };
    super::camera_request(
        method,
        format!("{}{path}", camera_http_base()),
        body,
        TIMEOUT,
        &all,
    )
    .await
    .map_err(|_| LookError::ConnectionLost)
}

fn failure(response: &HttpResponse) -> LookError {
    module_error(response.status(), &response.text())
}

/// A module ID as one URL path segment.
fn segment(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for byte in id.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Whether this GRKit serves module settings. A GRKit from before them has
/// no such capability.
pub async fn access() -> Result<ModuleAccess, LookError> {
    let response = request("GET", "/v1/props", None, &[]).await?;
    if !response.ok() {
        return Err(failure(&response));
    }
    let value: serde_json::Value =
        serde_json::from_slice(&response.body).map_err(|e| LookError::Service(e.to_string()))?;
    let capability = &value["capabilities"]["moduleSettings"];
    Ok(ModuleAccess {
        read: capability["read"].as_bool().unwrap_or(false),
        write: capability["write"].as_bool().unwrap_or(false),
        boot_id: value["bootId"].as_str().unwrap_or_default().to_owned(),
    })
}

/// Read the catalog, naming the cached one so an unchanged catalog costs no
/// transfer.
pub async fn catalog(
    language: &str,
    cached: Option<ModuleCatalog>,
) -> Result<CatalogAnswer, LookError> {
    let mut headers = Vec::new();
    if let Some(cached) = &cached {
        headers.push(("If-None-Match", format!("\"{}\"", cached.catalog)));
    }
    let path = format!("/v1/modules/catalog?language={language}");
    let response = request("GET", &path, None, &headers).await?;
    let status = response.status();
    catalog_answer(status, response.body, cached)
}

pub async fn state() -> Result<ModulesState, LookError> {
    let response = request("GET", "/v1/params/modules", None, &[]).await?;
    if !response.ok() {
        return Err(failure(&response));
    }
    parse_modules_state(&response.body).map_err(LookError::Service)
}

/// The icon of a module; `None` when it has no tile.
pub async fn icon(id: &str) -> Result<Option<Vec<u8>>, LookError> {
    let path = format!("/v1/modules/{}/icon", segment(id));
    let response = request("GET", &path, None, &[]).await?;
    if response.status() == 404 {
        return Ok(None);
    }
    if !response.ok() {
        return Err(failure(&response));
    }
    Ok((response.body.len() <= ICON_LIMIT).then_some(response.body))
}

/// Change one module. The answer is the module as the camera has it.
pub async fn patch(
    boot_id: &str,
    id: &str,
    body: serde_json::Value,
) -> Result<(ModuleState, u64), LookError> {
    let path = format!("/v1/params/modules/{}", segment(id));
    let headers = [("X-GRKit-Boot-Id", boot_id.to_owned())];
    let response = request("PATCH", &path, Some(body), &headers).await?;
    if !response.ok() {
        return Err(failure(&response));
    }
    parse_module_patch(&response.body).map_err(LookError::Service)
}

/// Replace the ADJ order of the orderable modules.
pub async fn put_order(boot_id: &str, ids: &[String]) -> Result<ModulesState, LookError> {
    let headers = [("X-GRKit-Boot-Id", boot_id.to_owned())];
    let body = serde_json::json!({ "modules": ids });
    let response = request("PUT", "/v1/params/modules/order", Some(body), &headers).await?;
    if !response.ok() {
        return Err(failure(&response));
    }
    parse_modules_state(&response.body).map_err(LookError::Service)
}

/// The GRKit settings of the camera's MENU, or `None` for a GRKit from
/// before the phone could change them.
pub async fn camera_settings() -> Result<Option<CameraSettings>, LookError> {
    let response = request("GET", "/v1/params/settings", None, &[]).await?;
    if matches!(response.status(), 404 | 405) {
        return Ok(None);
    }
    if !response.ok() {
        return Err(failure(&response));
    }
    parse_camera_settings(&response.body)
        .map(Some)
        .map_err(LookError::Service)
}

/// Change one GRKit setting. The answer is every setting as the camera has
/// them.
pub async fn patch_camera_setting(
    boot_id: &str,
    change: SettingChange,
) -> Result<CameraSettings, LookError> {
    let headers = [("X-GRKit-Boot-Id", boot_id.to_owned())];
    let response = request(
        "PATCH",
        "/v1/params/settings",
        Some(change.body()),
        &headers,
    )
    .await?;
    if !response.ok() {
        return Err(failure(&response));
    }
    parse_camera_settings(&response.body).map_err(LookError::Service)
}

/// Replace the layout of ADJ: the toolbar, the GRK root, and the hidden
/// modules.
pub async fn put_layout(boot_id: &str, layout: &ModuleLayout) -> Result<ModulesState, LookError> {
    let headers = [("X-GRKit-Boot-Id", boot_id.to_owned())];
    let response = request(
        "PUT",
        "/v1/params/modules/layout",
        Some(layout.body()),
        &headers,
    )
    .await?;
    if !response.ok() {
        return Err(failure(&response));
    }
    parse_modules_state(&response.body).map_err(LookError::Service)
}

/// Whether this GRKit takes a picture of Shutdown Screen from the phone.
pub async fn shutdown_picture_writable() -> Result<bool, LookError> {
    let response = request("GET", "/v1/props", None, &[]).await?;
    if !response.ok() {
        return Err(failure(&response));
    }
    let value: serde_json::Value =
        serde_json::from_slice(&response.body).map_err(|e| LookError::Service(e.to_string()))?;
    Ok(value["capabilities"]["shutdownPicture"]["write"]
        .as_bool()
        .unwrap_or(false))
}

/// The picture of Shutdown Screen on the card, or `None` without one.
pub async fn shutdown_picture() -> Result<Option<Vec<u8>>, LookError> {
    let response = request("GET", "/v1/shutdown-picture", None, &[]).await?;
    if response.status() == 404 {
        return Ok(None);
    }
    if !response.ok() {
        return Err(failure(&response));
    }
    Ok(Some(response.body))
}

/// Put `jpeg` on the card as the picture of Shutdown Screen, or remove it
/// (`None`). The camera shows it after its next start.
pub async fn put_shutdown_picture(boot_id: &str, jpeg: Option<Vec<u8>>) -> Result<(), LookError> {
    let mut headers = vec![("X-GRKit-Boot-Id", boot_id.to_owned())];
    let (method, body) = match jpeg {
        Some(bytes) => {
            headers.push(("Content-Type", "image/jpeg".to_owned()));
            ("PUT", bytes)
        }
        None => ("DELETE", Vec::new()),
    };
    let response = super::camera_request(
        method,
        format!("{}/v1/shutdown-picture", camera_http_base()),
        body,
        TIMEOUT,
        &headers,
    )
    .await
    .map_err(|_| LookError::ConnectionLost)?;
    if !response.ok() {
        return Err(failure(&response));
    }
    Ok(())
}

/// A name safe for one path component, and distinct for distinct input.
fn file_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn draft_path(camera: &str) -> Option<PathBuf> {
    Some(
        data_dir()?
            .join("module-drafts-v1")
            .join(format!("{}.json", file_name(camera))),
    )
}

/// The module changes that wait for the next update of a camera.
pub fn load_draft(camera: &str) -> ModuleDraft {
    draft_path(camera)
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save_draft(camera: &str, draft: &ModuleDraft) {
    let Some(path) = draft_path(camera) else {
        return;
    };
    if draft.is_empty() {
        let _ = fs::remove_file(path);
        return;
    }
    let save = || -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("tmp");
        fs::write(&temporary, serde_json::to_vec(draft)?)?;
        fs::rename(temporary, &path)
    };
    if let Err(error) = save() {
        log::warn!("Could not save the module changes: {error}");
    }
}

/// What the cache knows about an icon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CachedIcon {
    Image(Vec<u8>),
    /// The camera has no icon for this module.
    Missing,
}

/// The cached catalog, icons, and state of one camera in one language.
#[derive(Clone, Debug)]
pub struct ModuleCache {
    dir: PathBuf,
}

impl ModuleCache {
    pub fn open(camera: &str, language: &str) -> Option<Self> {
        Some(Self::at(
            &data_dir()?
                .join("module-cache-v1")
                .join(file_name(camera))
                .join(file_name(language)),
        ))
    }

    fn at(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
        }
    }

    fn write(&self, path: &Path, bytes: &[u8]) {
        let save = || -> std::io::Result<()> {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let temporary = path.with_extension("tmp");
            fs::write(&temporary, bytes)?;
            fs::rename(temporary, path)
        };
        if let Err(error) = save() {
            log::warn!("Could not cache module data: {error}");
        }
    }

    /// The cached catalog. A catalog this app cannot read counts as none.
    pub fn catalog(&self) -> Option<ModuleCatalog> {
        let bytes = fs::read(self.dir.join("catalog.json")).ok()?;
        match parse_module_catalog(&bytes).ok()? {
            CatalogRead::Ready(catalog) => Some(catalog),
            CatalogRead::TooNew => None,
        }
    }

    /// Keep a new catalog. Its icons may differ, so the old ones go.
    pub fn save_catalog(&self, bytes: &[u8]) {
        let _ = fs::remove_dir_all(self.dir.join("icons"));
        self.write(&self.dir.join("catalog.json"), bytes);
    }

    pub fn state(&self) -> Option<ModulesState> {
        let bytes = fs::read(self.dir.join("state.json")).ok()?;
        parse_modules_state(&bytes).ok()
    }

    pub fn save_state(&self, state: &ModulesState) {
        if let Ok(bytes) = serde_json::to_vec(state) {
            self.write(&self.dir.join("state.json"), &bytes);
        }
    }

    fn icon_path(&self, id: &str) -> PathBuf {
        self.dir
            .join("icons")
            .join(format!("{}.png", file_name(id)))
    }

    /// The cached icon; an empty file records that the camera has none.
    pub fn icon(&self, id: &str) -> Option<CachedIcon> {
        let bytes = fs::read(self.icon_path(id)).ok()?;
        Some(if bytes.is_empty() {
            CachedIcon::Missing
        } else {
            CachedIcon::Image(bytes)
        })
    }

    pub fn save_icon(&self, id: &str, bytes: Option<&[u8]>) {
        self.write(&self.icon_path(id), bytes.unwrap_or_default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_catalog_replaces_the_cached_one_and_its_icons() {
        let dir = std::env::temp_dir().join(format!("grkit-module-cache-{}", std::process::id()));
        let cache = ModuleCache::at(&dir);
        cache.save_catalog(br#"{"schema":1,"catalog":"aaaa","modules":[{"id":"A"}]}"#);
        cache.save_icon("A", Some(b"png"));
        cache.save_icon("B", None);
        assert_eq!(cache.catalog().unwrap().catalog, "aaaa");
        assert_eq!(cache.icon("A"), Some(CachedIcon::Image(b"png".to_vec())));
        assert_eq!(cache.icon("B"), Some(CachedIcon::Missing));

        cache.save_catalog(br#"{"schema":1,"catalog":"bbbb","modules":[]}"#);
        assert_eq!(cache.catalog().unwrap().catalog, "bbbb");
        assert_eq!(cache.icon("A"), None);

        // A catalog from a later format is not shown as data.
        cache.save_catalog(br#"{"schema":99,"catalog":"cccc"}"#);
        assert_eq!(cache.catalog(), None);
        fs::remove_dir_all(&dir).unwrap();
    }
}
