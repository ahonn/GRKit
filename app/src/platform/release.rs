//! Fetch an official release and rebuild the camera's selected package contents.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use grkit_core::{
    Files, camera,
    language::Language,
    packaging,
    release::{self, Statement},
    release_index::{FileInfo, Index},
};
use nsh_core::{
    platform::HttpRequest,
    prelude::{data_dir, fetch},
};

use crate::domain::{CameraModel, PackageResponse, ReleaseProblem, firmware_problem};

const INDEX_LIMIT: usize = 2 * 1024 * 1024;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(45);
const CACHE_LIMIT: u64 = 128 * 1024 * 1024;

/// Development keys come only from the debuggable Android host. Normal builds,
/// including desktop and iOS, retain the compiled-in official trust roots.
fn trusted_keys(base: &str) -> Result<Vec<[u8; 32]>, String> {
    #[cfg(target_os = "android")]
    let development_key = super::android::config("GRKIT_RELEASE_PUBLIC_KEY");
    #[cfg(not(target_os = "android"))]
    let development_key: Option<String> = None;
    trusted_keys_for(base, development_key.as_deref())
}

fn trusted_keys_for(base: &str, development_key: Option<&str>) -> Result<Vec<[u8; 32]>, String> {
    let mut keys = release::OFFICIAL_KEYS.to_vec();
    let Some(encoded) = development_key else {
        return Ok(keys);
    };
    if !loopback_release_base(base) {
        return Err("Development release keys require a localhost or 127.0.0.1 release URL".into());
    }
    let bytes = encoded.as_bytes();
    if bytes.len() != 64 {
        return Err(
            "Development release public key must contain exactly 64 hexadecimal characters".into(),
        );
    }
    fn nibble(byte: u8) -> Result<u8, String> {
        match byte {
            b'0'..=b'9' => Ok(byte - b'0'),
            b'a'..=b'f' => Ok(byte - b'a' + 10),
            b'A'..=b'F' => Ok(byte - b'A' + 10),
            _ => Err("Development release public key contains a non-hexadecimal character".into()),
        }
    }
    let mut key = [0; 32];
    for (target, pair) in key.iter_mut().zip(bytes.as_chunks::<2>().0) {
        *target = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    keys.push(key);
    Ok(keys)
}

fn loopback_release_base(base: &str) -> bool {
    let Some((scheme, rest)) = base.split_once("://") else {
        return false;
    };
    if !matches!(scheme, "http" | "https") || rest.contains(['?', '#', '\\']) {
        return false;
    }
    let authority = rest.split('/').next().unwrap_or_default();
    let (host, port) = authority
        .split_once(':')
        .map_or((authority, None), |(host, port)| (host, Some(port)));
    matches!(host, "localhost" | "127.0.0.1")
        && port.is_none_or(|port| {
            !port.is_empty()
                && port.bytes().all(|b| b.is_ascii_digit())
                && port.parse::<u16>().is_ok_and(|port| port != 0)
        })
}

fn cache_dir() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("release-cache-v1"))
}

fn read_bounded(path: &Path, limit: usize) -> Option<Vec<u8>> {
    let meta = fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.len() > limit as u64 {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    (bytes.len() <= limit).then_some(bytes)
}

fn cached_file(dir: &Path, info: &FileInfo) -> Option<Vec<u8>> {
    let bytes = read_bounded(
        &dir.join(format!("{}.bin", info.sha256)),
        packaging::MAX_FILE_SIZE,
    )?;
    valid_file(&bytes, info).then_some(bytes)
}

fn valid_file(bytes: &[u8], info: &FileInfo) -> bool {
    bytes.len() as u64 == info.size && packaging::sha256(bytes) == info.sha256
}

/// Delete only regular files in this app-owned cache, oldest first.
fn prune_cache(dir: &Path, incoming: u64, limit: u64) -> std::io::Result<()> {
    let mut files = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            if !entry.file_type().ok()?.is_file() {
                return None;
            }
            let meta = entry.metadata().ok()?;
            Some((meta.modified().ok(), entry.path(), meta.len()))
        })
        .collect::<Vec<_>>();
    let mut total = files.iter().map(|(_, _, size)| *size).sum::<u64>();
    files.sort_by_key(|(modified, _, _)| *modified);
    for (_, path, size) in files {
        if total.saturating_add(incoming) <= limit {
            break;
        }
        fs::remove_file(path)?;
        total = total.saturating_sub(size);
    }
    Ok(())
}

fn cache_write(dir: &Path, name: &str, bytes: &[u8]) {
    let save = || -> std::io::Result<()> {
        fs::create_dir_all(dir)?;
        // Include temporary-write space in the cache budget.
        prune_cache(dir, bytes.len() as u64, CACHE_LIMIT)?;
        let temporary = dir.join(format!("{name}.tmp"));
        if let Err(error) = fs::write(&temporary, bytes) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        if let Err(error) = fs::rename(&temporary, dir.join(name)) {
            let _ = fs::remove_file(temporary);
            return Err(error);
        }
        Ok(())
    };
    if let Err(error) = save() {
        log::warn!("Could not cache release file: {error}");
    }
}

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    /// The notes of recent releases, from the index: display only.
    pub notes: Vec<grkit_core::release_index::VersionNotes>,
    /// The recipe presets of the site: display and settings only.
    pub presets: Vec<grkit_core::release_index::IndexPreset>,
    /// Upper estimate of the release files, before applying camera selections.
    pub bytes: u64,
    base: String,
    index: Index,
    camera: &'static camera::Camera,
    statement: Statement,
    statement_bytes: Vec<u8>,
    signature: Vec<u8>,
}

fn camera_for(model: CameraModel) -> &'static camera::Camera {
    match model {
        CameraModel::Gr3 => &camera::GR3,
        CameraModel::Gr3x => &camera::GR3X,
        CameraModel::Gr4 => &camera::GR4,
    }
}

/// Only site-relative paths are accepted, without URL escaping or dot segments.
fn checked_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || !path
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/-_.".contains(&c))
    {
        return Err(format!("Invalid release file path: {path}"));
    }
    Ok(())
}

async fn get(base: &str, path: &str, limit: usize) -> Result<Vec<u8>, String> {
    checked_path(path)?;
    let mut request =
        HttpRequest::get(format!("{base}/{path}")).header("Cache-Control", "no-cache, no-store");
    request.follow_redirects = false;
    let response = super::with_timeout(fetch(request), DOWNLOAD_TIMEOUT)
        .await
        .ok_or_else(|| format!("Timed out downloading {path}"))??;
    if !response.ok() {
        return Err(format!(
            "Release download HTTP {}: {path}",
            response.status()
        ));
    }
    if response.body.len() > limit {
        return Err(format!("Release file exceeds its size limit: {path}"));
    }
    Ok(response.body)
}

async fn indexed_file(base: &str, index: &Index, path: &str) -> Result<Vec<u8>, String> {
    let info = index
        .files
        .get(path)
        .ok_or_else(|| format!("Release has no file: {path}"))?;
    if info.size > packaging::MAX_FILE_SIZE as u64 {
        return Err(format!("Release file is too large: {path}"));
    }
    let cache = cache_dir();
    if let Some(bytes) = cache.as_ref().and_then(|dir| cached_file(dir, info)) {
        return Ok(bytes);
    }
    let bytes = get(base, path, info.size as usize).await?;
    if !valid_file(&bytes, info) {
        return Err(format!(
            "Release file failed integrity verification: {path}"
        ));
    }
    if let Some(dir) = cache {
        cache_write(&dir, &format!("{}.bin", info.sha256), &bytes);
    }
    Ok(bytes)
}

/// Read and authenticate release metadata before offering an update. The
/// error says what the Software Update page tells the person; the details
/// go to the log.
pub async fn latest(model: CameraModel, firmware: &str) -> Result<Release, ReleaseProblem> {
    let camera = camera_for(model);
    let supported: Vec<&str> = camera.releases.iter().map(|r| r.firmware).collect();
    if let Some(problem) = firmware_problem(&supported, firmware) {
        log::warn!("Unsupported {} firmware: {firmware}", camera.name);
        return Err(problem);
    }
    latest_for(camera).await.map_err(|(problem, detail)| {
        log::warn!("Release check failed: {detail}");
        problem
    })
}

type Problem = (ReleaseProblem, String);

fn network(detail: String) -> Problem {
    (ReleaseProblem::Network, detail)
}

fn newer_app(detail: String) -> Problem {
    (ReleaseProblem::AppTooOld, detail)
}

async fn latest_for(camera: &'static camera::Camera) -> Result<Release, Problem> {
    let base = super::config_value("GRKIT_RELEASE_BASE")
        .unwrap_or_else(|| "https://grkit.app".into())
        .trim_end_matches('/')
        .to_owned();
    let keys = trusted_keys(&base).map_err(network)?;
    let cache = cache_dir();
    let index_name = format!(
        "index-{}-{}.json",
        packaging::sha256(base.as_bytes()),
        camera.id
    );
    // A stale authenticated release is usable offline; checking for a newer
    // release must not stall every attempt on the camera's isolated network.
    let online = super::with_timeout(
        get(&base, "release/index.json", INDEX_LIMIT),
        Duration::from_secs(8),
    )
    .await
    .unwrap_or_else(|| Err("Release index request timed out".into()));
    let index_bytes = match online {
        Ok(bytes) => bytes,
        Err(error) => {
            let cached = cache
                .as_ref()
                .and_then(|dir| read_bounded(&dir.join(&index_name), INDEX_LIMIT));
            match cached {
                Some(bytes) => {
                    log::info!("Using cached release metadata: {error}");
                    bytes
                }
                None => return Err(network(error)),
            }
        }
    };
    let index = Index::parse(&index_bytes).map_err(|error| newer_app(error.to_string()))?;
    for path in index.files.keys() {
        checked_path(path).map_err(newer_app)?;
    }
    let entry = index
        .cameras
        .get(camera.id)
        .ok_or_else(|| newer_app(format!("No release for {}", camera.name)))?;
    let expected: Vec<_> = camera.targets().into_iter().map(str::to_owned).collect();
    if entry.targets != expected {
        return Err(newer_app(
            "Release runtime targets do not match this camera".into(),
        ));
    }
    let statement_bytes = indexed_file(&base, &index, &entry.statement)
        .await
        .map_err(network)?;
    let signature = indexed_file(&base, &index, &entry.signature)
        .await
        .map_err(network)?;
    let statement = release::verify(&statement_bytes, &signature, &keys)
        .map_err(|error| network(error.to_string()))?;
    if statement.camera != camera || statement.version != index.version {
        return Err(network(
            "Signed release does not match the camera or index version".into(),
        ));
    }
    // Persist only after the official statement authenticates this camera and
    // version. Cache reads repeat all signature, path and payload checks.
    if let Some(dir) = cache {
        cache_write(&dir, &index_name, &index_bytes);
    }
    let bytes = index
        .files
        .iter()
        .filter(|(path, _)| {
            entry
                .targets
                .iter()
                .any(|target| path.starts_with(&format!("runtime/{target}/")))
                || path.starts_with("modules/")
                || entry.guides.get("en") == Some(path)
                || **path == entry.statement
                || **path == entry.signature
        })
        .try_fold(0u64, |sum, (_, info)| sum.checked_add(info.size))
        .ok_or_else(|| network("Release size overflow".to_owned()))?;
    Ok(Release {
        version: index.version.clone(),
        notes: index.notes.clone(),
        presets: index.presets.clone(),
        bytes,
        base,
        index,
        camera,
        statement,
        statement_bytes,
        signature,
    })
}

fn names(value: &Option<serde_json::Value>, field: &str) -> Result<Vec<String>, String> {
    let list = value
        .as_ref()
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("Camera package has no {field} list"))?;
    let mut seen = BTreeSet::new();
    list.iter()
        .map(|value| {
            let name = value
                .as_str()
                .ok_or_else(|| format!("Invalid camera {field} entry"))?;
            checked_path(name)?;
            if name.contains('/') || !seen.insert(name.to_owned()) {
                return Err(format!("Invalid or duplicate camera {field} entry: {name}"));
            }
            Ok(name.to_owned())
        })
        .collect()
}

/// Why a release could not become an update package.
#[derive(Debug, PartialEq, Eq)]
pub enum DownloadError {
    /// A file of the release could not be fetched or did not match.
    Network(String),
    /// The camera has a module that the release does not have.
    ModuleNotKept(String),
    /// The release files do not make a valid package.
    Package(String),
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::Network(message) | DownloadError::Package(message) => {
                f.write_str(message)
            }
            DownloadError::ModuleNotKept(id) => write!(f, "The release has no module {id}"),
        }
    }
}

/// The module files of the release that keep the camera's modules. An
/// update carries no Looks: the camera keeps its own, and the phone
/// manages them on their own. A module that the release lacks blocks the
/// update, as the camera removes a package module that an update omits.
fn selected_modules(
    release: &Release,
    snapshot: &PackageResponse,
) -> Result<Vec<String>, DownloadError> {
    names(&snapshot.modules, "modules")
        .map_err(DownloadError::Package)?
        .into_iter()
        .map(|id| {
            let filename = format!("{id}.GRM");
            let path = format!("modules/{filename}");
            if !release.statement.modules.contains_key(&filename)
                || !release.index.files.contains_key(&path)
            {
                return Err(DownloadError::ModuleNotKept(id));
            }
            Ok(path)
        })
        .collect()
}

/// Download, hash-check and assemble a signed update that keeps the camera's
/// modules and carries no Looks.
/// Progress reports completed file bytes; NSH fetch does not expose byte streaming.
pub async fn download(
    release: &Release,
    snapshot: &PackageResponse,
    on_progress: impl Fn(f32),
) -> Result<Vec<u8>, DownloadError> {
    let entry = &release.index.cameras[release.camera.id];
    let module_paths = selected_modules(release, snapshot)?;
    let package = DownloadError::Package;
    let guide_path = entry
        .guides
        .get("en")
        .ok_or_else(|| package("Release has no English guide".to_owned()))?;
    let mut paths = BTreeSet::new();
    let mut manifests = Vec::new();
    for target in &entry.targets {
        let path = format!("runtime/{target}/build.json");
        let manifest = indexed_file(&release.base, &release.index, &path)
            .await
            .map_err(DownloadError::Network)?;
        for runtime_path in
            packaging::manifest_paths(&manifest).map_err(|error| package(error.to_string()))?
        {
            checked_path(&runtime_path).map_err(package)?;
            paths.insert(format!("runtime/{target}/sd/{runtime_path}"));
        }
        manifests.push((target, manifest));
    }
    paths.extend(module_paths.iter().cloned());
    paths.insert(guide_path.clone());
    let total = paths.iter().try_fold(0u64, |sum, path| {
        let info = release
            .index
            .files
            .get(path)
            .ok_or_else(|| package(format!("Release has no file: {path}")))?;
        sum.checked_add(info.size)
            .ok_or_else(|| package("Release size overflow".to_owned()))
    })?;
    if total > packaging::MAX_TOTAL_SIZE as u64 {
        return Err(package("Release exceeds the package size limit".into()));
    }
    on_progress(0.0);
    let mut received = 0u64;
    let mut files = Files::new();
    for path in paths {
        let bytes = indexed_file(&release.base, &release.index, &path)
            .await
            .map_err(DownloadError::Network)?;
        received += bytes.len() as u64;
        files.insert(path, bytes);
        on_progress(received as f32 / total.max(1) as f32);
    }
    let mut runtimes = Vec::new();
    for (target, manifest) in manifests {
        let prefix = format!("runtime/{target}/sd/");
        let runtime_files = files
            .iter()
            .filter_map(|(path, bytes)| {
                path.strip_prefix(&prefix)
                    .map(|path| (path.to_owned(), bytes.clone()))
            })
            .collect();
        let runtime = packaging::load_runtime(&manifest, &runtime_files)
            .map_err(|error| package(error.to_string()))?;
        if runtime.target() != target {
            return Err(package(
                "Runtime manifest target differs from the release index".into(),
            ));
        }
        runtimes.push(runtime);
    }
    let modules: Vec<_> = module_paths
        .iter()
        .map(|path| files[path].clone())
        .collect();
    let effects = packaging::EffectSet::default();
    let assembled = packaging::assemble_with(
        &runtimes,
        &effects,
        &modules,
        release.camera,
        &packaging::Guide {
            language: Language::English,
            pdf: &files[guide_path],
        },
        packaging::Extras {
            release: Some((&release.statement_bytes, &release.signature)),
            ..Default::default()
        },
    )
    .map_err(|error| package(error.to_string()))?;
    release::verify_update(
        &assembled.bytes,
        &trusted_keys(&release.base).map_err(package)?,
    )
    .map_err(|error| package(error.to_string()))?;
    Ok(assembled.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_trust_contains_only_official_keys() {
        for base in ["https://grkit.app", "http://127.0.0.1:18081"] {
            assert_eq!(
                trusted_keys_for(base, None).unwrap(),
                release::OFFICIAL_KEYS
            );
        }
    }

    #[test]
    fn development_key_is_decoded_only_for_loopback_releases() {
        let key = "aB".repeat(32);
        for base in [
            "http://127.0.0.1:18081",
            "http://localhost",
            "https://localhost:443/fixture",
        ] {
            let keys = trusted_keys_for(base, Some(&key)).unwrap();
            assert_eq!(
                &keys[..release::OFFICIAL_KEYS.len()],
                release::OFFICIAL_KEYS
            );
            assert_eq!(keys.last(), Some(&[0xab; 32]));
        }
        for base in [
            "https://grkit.app",
            "http://10.0.2.2",
            "http://127.0.0.1.example",
            "http://localhost@evil.test",
            "http://localhost:80@evil.test",
            "http://localhost:0",
            "http://localhost:65536",
            "http://localhost:abc",
            "http://localhost?evil",
            "ftp://localhost",
            "http://localhost\\evil.test",
        ] {
            assert!(trusted_keys_for(base, Some(&key)).is_err(), "{base}");
        }
    }

    #[test]
    fn development_key_rejects_malformed_hex() {
        for key in [
            String::new(),
            "a".repeat(63),
            "a".repeat(65),
            "g".repeat(64),
            "é".repeat(32),
        ] {
            assert!(trusted_keys_for("http://127.0.0.1", Some(&key)).is_err());
        }
    }

    #[test]
    fn paths_cannot_escape_the_release_origin() {
        for path in [
            "../secret",
            "/absolute",
            "https://other/file",
            "runtime/%2e%2e/x",
            "a//b",
            "a/./b",
            "a?x=1",
        ] {
            assert!(checked_path(path).is_err(), "{path}");
        }
        assert!(checked_path("runtime/gr3-2.21/sd/SCRIPT/GRKIT/CORE.BIN").is_ok());
    }

    #[test]
    fn missing_or_malformed_camera_selection_is_not_an_empty_package() {
        assert!(names(&None, "effects").is_err());
        assert!(names(&Some(serde_json::json!(["A.GRF", "A.GRF"])), "effects").is_err());
        assert!(names(&Some(serde_json::json!(["../A.GRF"])), "effects").is_err());
        assert_eq!(
            names(&Some(serde_json::json!([])), "effects").unwrap(),
            Vec::<String>::new()
        );
    }

    fn selection_fixture() -> (Release, PackageResponse) {
        let index = Index::parse(&serde_json::to_vec(&serde_json::json!({
            "format": 1, "version": "1.7.0",
            "cameras": {"gr3": {"targets": ["gr3-2.10"], "guides": {},
                "statement": "release/gr3/RELEASE.JSON", "signature": "release/gr3/RELEASE.SIG"}},
            "files": {
                "modules/FILMLOOK.GRM": {"sha256": "0".repeat(64), "size": 1},
                "effects/gr3-2.10/KNOWN.GRF": {"sha256": "0".repeat(64), "size": 1}
            }
        })).unwrap()).unwrap();
        let release = Release {
            version: index.version.clone(),
            notes: Vec::new(),
            presets: Vec::new(),
            bytes: 2,
            base: "https://example.test".into(),
            index,
            camera: &camera::GR3,
            statement: Statement {
                version: "1.7.0".into(),
                camera: &camera::GR3,
                runtime: Default::default(),
                modules: [("FILMLOOK.GRM".into(), "0".repeat(64))].into(),
                official_looks: Default::default(),
                official_look_bodies: Default::default(),
                official_covers: Default::default(),
            },
            statement_bytes: vec![],
            signature: vec![],
        };
        let snapshot: PackageResponse = serde_json::from_value(serde_json::json!({
            "version": "1.6.1", "modules": ["FILMLOOK"], "effects": ["KNOWN.GRF"],
            "left_out": ["OMITTED.GRF"], "camera": null
        }))
        .unwrap();
        (release, snapshot)
    }

    #[test]
    fn an_update_keeps_the_modules_and_carries_no_looks() {
        let (release, mut snapshot) = selection_fixture();
        assert_eq!(
            selected_modules(&release, &snapshot).unwrap(),
            ["modules/FILMLOOK.GRM"]
        );
        // Looks that the release lacks do not block the update.
        snapshot.effects = Some(serde_json::json!(["CUSTOM.GRF"]));
        assert!(selected_modules(&release, &snapshot).is_ok());
        snapshot.modules = Some(serde_json::json!(["CUSTOM"]));
        assert_eq!(
            selected_modules(&release, &snapshot),
            Err(DownloadError::ModuleNotKept("CUSTOM".into()))
        );
    }

    #[test]
    fn cached_payload_requires_both_digest_and_size() {
        let bytes = b"verified release payload";
        let info = FileInfo {
            sha256: packaging::sha256(bytes),
            size: bytes.len() as u64,
        };
        assert!(valid_file(bytes, &info));
        assert!(!valid_file(b"modified release payload", &info));
        assert!(!valid_file(
            bytes,
            &FileInfo {
                size: info.size + 1,
                ..info
            }
        ));
    }

    #[test]
    fn cache_budget_prunes_only_owned_regular_files() {
        struct Temp(PathBuf);
        impl Drop for Temp {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let dir = Temp(std::env::temp_dir().join(format!(
                "grkit-release-cache-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )));
        fs::create_dir_all(dir.0.join("untouched-directory")).unwrap();
        fs::write(dir.0.join("old.bin"), [1; 12]).unwrap();
        fs::write(dir.0.join("other.bin"), [2; 12]).unwrap();
        prune_cache(&dir.0, 8, 20).unwrap();
        let total = fs::read_dir(&dir.0)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().unwrap().is_file())
            .map(|entry| entry.metadata().unwrap().len())
            .sum::<u64>();
        assert!(total + 8 <= 20);
        assert!(dir.0.join("untouched-directory").is_dir());
    }
}
