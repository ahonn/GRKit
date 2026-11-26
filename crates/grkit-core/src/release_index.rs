//! The release index of a site: one JSON file that lists every downloadable
//! file of a release, with its SHA-256 and size, so a phone can build a
//! package for a camera over Wi-Fi from the site alone.
//!
//! The index covers the signed statement of each camera, its runtime builds,
//! the official modules, the effect catalogs, and the guides. It fails when a
//! statement's runtime hashes do not match the package runtime that the site
//! would build for that camera (`release::Statement::of_release`).

use std::collections::BTreeMap;

use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    camera,
    packaging::{self, VerifiedRuntime},
    release::{self, Statement},
    site, Files,
};

/// The version of the release index format.
pub const FORMAT: u64 = 1;

/// The release contract that this build reads. Raise it only for a release
/// that an older app cannot install: an index names it as `minAppProtocol`.
pub const APP_PROTOCOL: u64 = 1;

/// The path on the site of a file of the package runtime of a camera.
pub fn package_path(camera: &str, path: &str) -> String {
    format!("release/{camera}/{}/{path}", release::PACKAGE_FOLDER)
}

/// One file that the app may download from the site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileInfo {
    pub sha256: String,
    pub size: u64,
}

/// The signed release of one camera on the site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraRelease {
    /// The runtime targets of the camera, in release order.
    pub targets: Vec<String>,
    /// Site path of `RELEASE.JSON`.
    pub statement: String,
    /// Site path of `RELEASE.SIG`.
    pub signature: String,
    /// Site path of each guide PDF, by language code.
    pub guides: BTreeMap<String, String>,
}

/// The release notes of one version: plain-text items, in English.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VersionNotes {
    pub version: String,
    pub items: Vec<String>,
}

/// The most versions an index gives notes for.
pub const NOTES_VERSIONS: usize = 10;
/// The most items of one version.
pub const NOTES_ITEMS: usize = 30;
/// The longest item, in characters.
pub const NOTE_CHARS: usize = 500;

/// A recipe preset of the site, as the phone adds it: its recipe for each
/// camera family, the official effect it needs, its author, and a short
/// description by language code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IndexPreset {
    pub id: String,
    pub effect: Option<String>,
    pub author: Option<String>,
    pub text: BTreeMap<String, String>,
    pub recipes: Vec<crate::recipes::Recipe>,
}

/// The most presets an index gives.
pub const PRESETS_MAX: usize = 64;
/// The longest description of a preset, in characters.
pub const PRESET_TEXT_CHARS: usize = 300;

impl IndexPreset {
    /// The presets of `grkit_core::recipes`, with the descriptions of
    /// `text`: language code, then preset ID, then the text.
    pub fn all(text: &BTreeMap<String, BTreeMap<String, String>>) -> Vec<IndexPreset> {
        crate::recipes::presets()
            .into_iter()
            .map(|preset| IndexPreset {
                text: text
                    .iter()
                    .filter_map(|(language, texts)| {
                        texts
                            .get(preset.id)
                            .filter(|t| plain(t) && t.chars().count() <= PRESET_TEXT_CHARS)
                            .map(|t| (language.clone(), t.clone()))
                    })
                    .collect(),
                id: preset.id.to_owned(),
                effect: preset.effect.map(str::to_owned),
                author: preset.author.map(str::to_owned),
                recipes: preset.recipes,
            })
            .collect()
    }
}

/// What a phone shows of an official module that a camera does not have.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleInfo {
    /// The title by language code.
    pub titles: BTreeMap<String, String>,
    pub color_only: bool,
}

/// The release that a site holds. Its JSON has these fields and `format`
/// (`FORMAT`); a reader leaves out the parts of the optional fields that do
/// not fit (`Index::parse`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    /// The GRKit version, which every signed statement must name.
    pub version: String,
    /// The oldest `APP_PROTOCOL` that can install this release.
    pub min_app_protocol: u64,
    /// The official modules of the release, by ID.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub modules: BTreeMap<String, ModuleInfo>,
    pub cameras: BTreeMap<String, CameraRelease>,
    /// Every downloadable file, by its path from the site root.
    pub files: BTreeMap<String, FileInfo>,
    /// The release notes of recent versions, newest first. They are for
    /// display only: no signature covers them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<VersionNotes>,
    /// The recipe presets of the site, for the phone to add. Display and
    /// settings only: the camera checks a recipe like any other.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub presets: Vec<IndexPreset>,
}

/// The index file as a reader takes it: the optional fields stay JSON, so
/// that an entry that does not fit is left out instead of failing the index.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexFile {
    format: u64,
    version: String,
    #[serde(default = "app_protocol")]
    min_app_protocol: u64,
    cameras: BTreeMap<String, Value>,
    files: BTreeMap<String, FileInfo>,
    #[serde(default)]
    modules: Value,
    #[serde(default)]
    notes: Value,
    #[serde(default)]
    presets: Value,
}

fn app_protocol() -> u64 {
    APP_PROTOCOL
}

/// The files of a site that the index covers. The CLI fills this from disk.
#[derive(Debug, Clone, Default)]
pub struct SiteContents {
    /// `release/<camera>/`: statement and signature bytes, by camera id.
    pub releases: BTreeMap<String, (Vec<u8>, Vec<u8>)>,
    /// `release/<camera>/sd/`: the package runtime, by camera id.
    pub packages: BTreeMap<String, Files>,
    /// `runtime/<target>/build.json`.
    pub manifests: BTreeMap<String, Vec<u8>>,
    /// `runtime/<target>/`: verified runtime of each build.
    pub runtimes: BTreeMap<String, VerifiedRuntime>,
    /// `modules/<ID>.GRM`.
    pub modules: BTreeMap<String, Vec<u8>>,
    /// `effects/<target>/`: files relative to that directory.
    pub effects: BTreeMap<String, Files>,
    /// `guides/<camera>/<language>.pdf`, by camera id.
    pub guides: BTreeMap<String, BTreeMap<String, Vec<u8>>>,
}

impl Index {
    /// Builds the index of `site`. Checks that each statement matches the
    /// package runtime of its camera on the site.
    pub fn of_site(site: &SiteContents) -> Result<Index> {
        ensure!(
            !site.releases.is_empty(),
            "The site has no signed release in release/"
        );
        let mut version = None;
        let mut cameras = BTreeMap::new();
        let mut files = BTreeMap::new();
        let mut targets = BTreeMap::<String, VerifiedRuntime>::new();

        for (camera_id, (statement_bytes, signature_bytes)) in &site.releases {
            let camera = camera::find(camera_id)?;
            let statement = Statement::parse(statement_bytes)?;
            ensure!(
                statement.camera == camera,
                "release/{}/{} is for the {}, not the {}",
                camera_id,
                release::STATEMENT_ENTRY,
                statement.camera.name,
                camera.name
            );
            match &version {
                None => version = Some(statement.version.clone()),
                Some(expected) => ensure!(
                    statement.version == *expected,
                    "The signed statements name different GRKit versions"
                ),
            }

            let runtimes = camera
                .targets()
                .into_iter()
                .map(|target| {
                    site.runtimes.get(target).cloned().with_context(|| {
                        format!(
                            "The site has no runtime/{target} for the signed {}",
                            camera.name
                        )
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let modules = statement
                .modules
                .keys()
                .map(|name| {
                    site.modules
                        .get(name)
                        .cloned()
                        .with_context(|| format!("The site has no modules/{name}"))
                })
                .collect::<Result<Vec<_>>>()?;
            let expected = Statement::of_release(&runtimes, &modules, camera)?;
            ensure!(
                expected.runtime == statement.runtime,
                "The runtime files of {} on the site do not match release/{}/{}",
                camera.name,
                camera.id,
                release::STATEMENT_ENTRY
            );
            ensure!(
                expected.modules == statement.modules,
                "The modules of {} on the site do not match release/{}/{}",
                camera.name,
                camera.id,
                release::STATEMENT_ENTRY
            );
            ensure!(
                expected.version == statement.version,
                "The GRKit version of {} on the site does not match its statement",
                camera.name
            );

            let statement_path = format!("release/{}/{}", camera.id, release::STATEMENT_ENTRY);
            let signature_path = format!("release/{}/{}", camera.id, release::SIGNATURE_ENTRY);
            insert_file(&mut files, &statement_path, statement_bytes)?;
            insert_file(&mut files, &signature_path, signature_bytes)?;
            let package = site
                .packages
                .get(camera_id)
                .with_context(|| format!("The site has no package runtime for {}", camera.name))?;
            ensure!(
                package.len() == statement.runtime.len()
                    && statement.runtime.iter().all(|(path, hash)| package
                        .get(path)
                        .is_some_and(|bytes| packaging::sha256(bytes) == *hash)),
                "The package runtime of {} on the site does not match its statement",
                camera.name
            );
            for (path, bytes) in package {
                insert_file(&mut files, &package_path(camera.id, path), bytes)?;
            }

            let mut guide_paths = BTreeMap::new();
            let guides = site.guides.get(camera_id).with_context(|| {
                format!(
                    "The site has no guides/{} for the signed {}",
                    camera.id, camera.name
                )
            })?;
            ensure!(
                !guides.is_empty(),
                "The site has no guide PDF for the signed {}",
                camera.name
            );
            for (language, pdf) in guides {
                let path = format!("guides/{}/{language}.pdf", camera.id);
                insert_file(&mut files, &path, pdf)?;
                guide_paths.insert(language.clone(), path);
            }

            for runtime in &runtimes {
                targets.insert(runtime.target().to_owned(), runtime.clone());
            }

            cameras.insert(
                camera.id.to_owned(),
                CameraRelease {
                    targets: camera.targets().into_iter().map(str::to_owned).collect(),
                    statement: statement_path,
                    signature: signature_path,
                    guides: guide_paths,
                },
            );
        }

        for (target, runtime) in &targets {
            let manifest = site
                .manifests
                .get(target)
                .cloned()
                .with_context(|| format!("The site has no runtime/{target}/build.json"))?;
            let manifest_path = format!("runtime/{target}/build.json");
            insert_file(&mut files, &manifest_path, &manifest)?;
            for path in packaging::manifest_paths(&manifest)? {
                let file = runtime.files().get(&path).with_context(|| {
                    format!("runtime/{target}/build.json lists missing sd/{path}")
                })?;
                insert_file(&mut files, &format!("runtime/{target}/sd/{path}"), file)?;
            }

            let effects = site.effects.get(target).with_context(|| {
                format!("The site has no effects/{target} for a signed camera that uses it")
            })?;
            let catalog = effects
                .get("catalog.json")
                .with_context(|| format!("effects/{target} has no catalog.json"))?;
            for relative in effect_files(catalog)? {
                let bytes = effects.get(&relative).with_context(|| {
                    format!("effects/{target}/catalog.json lists missing {relative}")
                })?;
                insert_file(&mut files, &format!("effects/{target}/{relative}"), bytes)?;
            }
        }

        let mut module_info = BTreeMap::new();
        for (name, bytes) in &site.modules {
            insert_file(&mut files, &format!("modules/{name}"), bytes)?;
            let module = crate::grm::Module::decode(bytes)?;
            module_info.insert(
                module.id.clone(),
                ModuleInfo {
                    titles: BTreeMap::new(),
                    color_only: module.color_only()?,
                },
            );
        }

        Ok(Index {
            version: version.expect("checked non-empty releases"),
            min_app_protocol: APP_PROTOCOL,
            modules: module_info,
            cameras,
            files,
            notes: Vec::new(),
            presets: Vec::new(),
        })
    }

    /// The bytes of the index file.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut index = serde_json::to_value(self)?;
        index["format"] = FORMAT.into();
        packaging::json_bytes(&index)
    }

    /// The release is for a later app than this build. A file that does not
    /// parse says nothing about the app.
    pub fn needs_newer_app(bytes: &[u8]) -> bool {
        let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
            return false;
        };
        value["format"]
            .as_u64()
            .is_some_and(|format| format > FORMAT)
            || value["minAppProtocol"]
                .as_u64()
                .is_some_and(|protocol| protocol > APP_PROTOCOL)
    }

    /// Reads an index. It leaves out the cameras that this build does not
    /// know, and does not check the files on a site.
    pub fn parse(bytes: &[u8]) -> Result<Index> {
        let file: IndexFile = serde_json::from_slice(bytes).context("Read the release index")?;
        ensure!(
            file.format == FORMAT,
            "The release index has another format"
        );
        let cameras = file
            .cameras
            .into_iter()
            .filter(|(id, _)| camera::find(id).is_ok())
            .map(|(id, entry)| {
                let release = serde_json::from_value(entry)
                    .with_context(|| format!("Read the release of the camera {id}"))?;
                Ok((id, release))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        if let Some(path) = file.files.iter().find_map(|(path, info)| {
            let valid = info.sha256.len() == 64
                && info
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
            (!valid).then_some(path)
        }) {
            anyhow::bail!("The release index has no SHA-256 for {path}");
        }
        Ok(Index {
            version: file.version,
            min_app_protocol: file.min_app_protocol,
            modules: read_modules(&file.modules),
            cameras,
            files: file.files,
            notes: read_notes(&file.notes),
            presets: read_presets(&file.presets),
        })
    }
}

/// Each entry of `value` that reads as a `T`; the others are left out, so
/// that a later or damaged entry never stops an update.
fn entries<T: serde::de::DeserializeOwned>(value: &Value) -> impl Iterator<Item = T> + '_ {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| serde_json::from_value(entry.clone()).ok())
}

/// The modules of an index as a reader takes them: a title that is not a
/// text is left out.
fn read_modules(value: &Value) -> BTreeMap<String, ModuleInfo> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Entry {
        #[serde(default)]
        titles: BTreeMap<String, Value>,
        #[serde(default)]
        color_only: Option<bool>,
    }
    value
        .as_object()
        .into_iter()
        .flatten()
        .map(|(id, entry)| {
            let entry: Option<Entry> = serde_json::from_value(entry.clone()).ok();
            let info = entry.map_or_else(ModuleInfo::default, |entry| ModuleInfo {
                titles: entry
                    .titles
                    .into_iter()
                    .filter_map(|(language, title)| Some((language, title.as_str()?.to_owned())))
                    .collect(),
                color_only: entry.color_only.unwrap_or(false),
            });
            (id.clone(), info)
        })
        .collect()
}

/// The notes of an index as a reader takes them: the versions and items
/// that do not fit are left out.
fn read_notes(value: &Value) -> Vec<VersionNotes> {
    #[derive(Deserialize)]
    struct Entry {
        version: String,
        items: Vec<Value>,
    }
    entries::<Entry>(value)
        .filter(|entry| semver(&entry.version).is_some())
        .filter_map(|entry| {
            let items: Vec<String> = entry
                .items
                .iter()
                .filter_map(Value::as_str)
                .filter(|item| plain(item) && item.chars().count() <= NOTE_CHARS)
                .take(NOTES_ITEMS)
                .map(str::to_owned)
                .collect();
            (!items.is_empty()).then_some(VersionNotes {
                version: entry.version,
                items,
            })
        })
        .take(NOTES_VERSIONS)
        .collect()
}

/// The presets of an index as a reader takes them: a preset whose recipes
/// do not pass the checks of the camera is left out, and so is a text that
/// does not fit.
fn read_presets(value: &Value) -> Vec<IndexPreset> {
    #[derive(Deserialize)]
    struct Entry {
        id: String,
        #[serde(default)]
        effect: Option<String>,
        #[serde(default)]
        author: Option<String>,
        #[serde(default)]
        text: BTreeMap<String, Value>,
        recipes: Vec<crate::recipes::Recipe>,
    }
    entries::<Entry>(value)
        .filter(|entry| {
            !entry.id.is_empty()
                && entry.id.len() <= 64
                && !entry.recipes.is_empty()
                && entry.recipes.iter().all(|recipe| recipe.check().is_ok())
        })
        .map(|entry| IndexPreset {
            id: entry.id,
            effect: entry.effect,
            author: entry.author,
            text: entry
                .text
                .into_iter()
                .filter_map(|(language, text)| {
                    let text = text.as_str()?;
                    (plain(text) && text.chars().count() <= PRESET_TEXT_CHARS)
                        .then(|| (language, text.to_owned()))
                })
                .collect(),
            recipes: entry.recipes,
        })
        .take(PRESETS_MAX)
        .collect()
}

/// The parts of a version `x.y.z`.
pub fn semver(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.');
    let mut next = || {
        parts
            .next()
            .filter(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|p| p.parse().ok())
    };
    let version = (next()?, next()?, next()?);
    parts.next().is_none().then_some(version)
}

fn plain(item: &str) -> bool {
    !item.trim().is_empty() && !item.chars().any(char::is_control)
}

/// The release notes for an index from release bodies in Markdown, by
/// version: the bullet items of each body as plain text, newest version
/// first. Headings and other lines are left out.
pub fn notes_from_markdown(releases: &[(String, String)]) -> Result<Vec<VersionNotes>> {
    let mut notes: Vec<VersionNotes> = Vec::new();
    for (version, body) in releases {
        let version = version.strip_prefix('v').unwrap_or(version);
        ensure!(
            semver(version).is_some(),
            "Release notes name a bad version: {version}"
        );
        ensure!(
            notes.iter().all(|n| n.version != version),
            "Release notes name version {version} twice"
        );
        let items: Vec<String> = body.lines().filter_map(plain_item).collect();
        ensure!(
            items.len() <= NOTES_ITEMS,
            "The notes of {version} have more than {NOTES_ITEMS} items"
        );
        if let Some(long) = items.iter().find(|i| i.chars().count() > NOTE_CHARS) {
            anyhow::bail!("A note of {version} is longer than {NOTE_CHARS} characters: {long}");
        }
        if !items.is_empty() {
            notes.push(VersionNotes {
                version: version.to_owned(),
                items,
            });
        }
    }
    notes.sort_by_key(|n| std::cmp::Reverse(semver(&n.version)));
    notes.truncate(NOTES_VERSIONS);
    Ok(notes)
}

/// One Markdown bullet line as plain text: emphasis, code marks, and links
/// become their text, images and generated author credits go.
fn plain_item(line: &str) -> Option<String> {
    let line = line.trim();
    let text = ["- ", "* ", "+ "]
        .iter()
        .find_map(|marker| line.strip_prefix(marker))?;
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(['[', '!']) {
        let image = rest[start..].starts_with("![");
        let open = if image { start + 1 } else { start };
        if !rest[open..].starts_with('[') {
            out.push_str(&rest[..=start]);
            rest = &rest[start + 1..];
            continue;
        }
        let Some(close) = rest[open..].find("](").map(|i| open + i) else {
            out.push_str(&rest[..=start]);
            rest = &rest[start + 1..];
            continue;
        };
        let Some(end) = rest[close..].find(')').map(|i| close + i) else {
            break;
        };
        out.push_str(&rest[..start]);
        if !image {
            out.push_str(&rest[open + 1..close]);
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    // A generated note ends with " by @someone in https://...".
    if let Some(at) = out.find(" by @") {
        if out[at..].contains(" in http") {
            out.truncate(at);
        }
    }
    let out: String = out
        .replace("**", "")
        .replace("__", "")
        .chars()
        .filter(|c| !matches!(c, '`' | '*') && !c.is_control())
        .collect();
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    (!out.is_empty()).then_some(out)
}

fn insert_file(files: &mut BTreeMap<String, FileInfo>, path: &str, bytes: &[u8]) -> Result<()> {
    let info = FileInfo {
        sha256: packaging::sha256(bytes),
        size: bytes.len() as u64,
    };
    if let Some(earlier) = files.insert(path.to_owned(), info.clone()) {
        ensure!(
            earlier == info,
            "The site lists {path} twice with different contents"
        );
    }
    Ok(())
}

/// The relative paths that an effects catalog names, including `catalog.json`.
fn effect_files(catalog: &[u8]) -> Result<Vec<String>> {
    let value: Value = serde_json::from_slice(catalog).context("Read the effects catalog")?;
    ensure!(
        value["schema"] == site::CATALOG_SCHEMA,
        "The effects catalog has another schema"
    );
    let mut paths = vec!["catalog.json".to_owned()];
    let baseline = value["preview"]["baseline"]
        .as_str()
        .context("The effects catalog has no preview baseline")?;
    paths.push(baseline.to_owned());
    for effect in value["effects"]
        .as_array()
        .context("The effects catalog has no effects")?
    {
        paths.push(
            effect["file"]
                .as_str()
                .context("An effect catalog entry has no file")?
                .to_owned(),
        );
        paths.push(
            effect["preview"]
                .as_str()
                .context("An effect catalog entry has no preview")?
                .to_owned(),
        );
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;
    use crate::testing::{self, RELEASE_SEED};
    use serde_json::json;
    use std::collections::BTreeMap;

    fn effect_tree(target: &str) -> Files {
        let mut files = Files::new();
        files.insert("preview/standard.lut".into(), vec![0; 12]);
        files.insert("effect.grf".into(), vec![1; 8]);
        files.insert("preview/test.lut".into(), vec![2; 8]);
        let catalog = packaging::json_bytes(&json!({
            "schema": site::CATALOG_SCHEMA,
            "target": target,
            "base_gamma": vec![0u16; 256],
            "preview": {
                "size": 33,
                "order": "rgb",
                "input": "linear-srgb16",
                "shaper": "srgb",
                "baseline": "preview/standard.lut",
            },
            "effects": [{
                "id": "test",
                "name": "Test",
                "icon": "",
                "monochrome": false,
                "look": "Standard",
                "origin": "test",
                "license": "test",
                "file": "effect.grf",
                "sha256": packaging::sha256(&[1; 8]),
                "preview": "preview/test.lut",
            }],
        }))
        .unwrap();
        files.insert("catalog.json".into(), catalog);
        files
    }

    /// A site tree for `camera` at `version`, as the release workflow publishes it.
    fn signed_site(camera: &'static Camera, version: &str) -> (SiteContents, Vec<u8>, Vec<u8>) {
        let runtimes = testing::release_runtimes(camera, version);
        let modules = testing::official_modules();
        let statement = Statement::of_release(&runtimes, &modules, camera)
            .unwrap()
            .to_bytes()
            .unwrap();
        let signature = release::SigningKey::from_hex(RELEASE_SEED)
            .unwrap()
            .sign(&statement);
        let mut site = SiteContents::default();
        site.releases
            .insert(camera.id.to_owned(), (statement.clone(), signature.clone()));
        site.packages.insert(
            camera.id.to_owned(),
            release::package_runtime_files(&runtimes, camera).unwrap(),
        );
        for runtime in runtimes {
            let target = runtime.target().to_owned();
            let manifest = packaging::json_bytes(&Value::Object(packaging::manifest(
                &target,
                runtime.files(),
            )))
            .unwrap();
            site.manifests.insert(target.clone(), manifest);
            site.runtimes.insert(target.clone(), runtime);
            site.effects.insert(target.clone(), effect_tree(&target));
        }
        for bytes in modules {
            let id = crate::grm::Module::decode(&bytes).unwrap().id;
            site.modules.insert(format!("{id}.GRM"), bytes);
        }
        site.guides.insert(
            camera.id.to_owned(),
            BTreeMap::from([("en".to_owned(), b"%PDF-1.7\n% test\n".to_vec())]),
        );
        (site, statement, signature)
    }

    #[test]
    fn the_index_round_trips() {
        let (site, _, _) = signed_site(&camera::GR3, "1.7.0");
        let index = Index::of_site(&site).unwrap();
        assert_eq!(index.version, "1.7.0");
        assert_eq!(
            index.cameras["gr3"].targets,
            camera::GR3
                .targets()
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        );
        assert!(index.files.contains_key("release/gr3/RELEASE.JSON"));
        assert!(index.files.contains_key("release/gr3/RELEASE.SIG"));
        assert!(index.files.contains_key("runtime/gr3-2.10/build.json"));
        assert!(index
            .files
            .keys()
            .any(|path| path.starts_with("runtime/gr3-2.10/sd/")));
        assert!(index.files.contains_key("effects/gr3-2.10/catalog.json"));
        assert!(index.files.contains_key("guides/gr3/en.pdf"));
        assert!(index.files.keys().any(|path| path.starts_with("modules/")));
        let bytes = index.to_bytes().unwrap();
        assert_eq!(Index::parse(&bytes).unwrap(), index);
    }

    #[test]
    fn an_older_reader_skips_a_new_camera_and_obeys_the_app_protocol() {
        let (site, _, _) = signed_site(&camera::GR3, "1.7.0");
        let mut value: Value =
            serde_json::from_slice(&Index::of_site(&site).unwrap().to_bytes().unwrap()).unwrap();
        value["cameras"]["gr9"] = json!({"future": true});
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(!Index::needs_newer_app(&bytes));
        assert_eq!(Index::parse(&bytes).unwrap().cameras.len(), 1);
        value["minAppProtocol"] = json!(APP_PROTOCOL + 1);
        assert!(Index::needs_newer_app(&serde_json::to_vec(&value).unwrap()));
        assert!(!Index::needs_newer_app(b"not json"));
    }

    #[test]
    fn an_update_of_signed_files_verifies_and_a_changed_file_does_not() {
        let (site, statement, signature) = signed_site(&camera::GR3, "1.7.0");
        let mut runtime = site.packages["gr3"].clone();
        let zip =
            release::assemble_update(&statement, &signature, &runtime, &site.modules).unwrap();
        let (update, _) = release::verify_update(&zip, &[testing::release_key()]).unwrap();
        assert_eq!(update.modules.len(), site.modules.len());
        runtime.values_mut().next().unwrap().push(0);
        assert!(release::assemble_update(&statement, &signature, &runtime, &site.modules).is_err());
    }

    #[test]
    fn the_presets_round_trip_and_a_bad_one_is_left_out() {
        let (site, _, _) = signed_site(&camera::GR3, "1.7.0");
        let mut index = Index::of_site(&site).unwrap();
        let text = BTreeMap::from([(
            "en".to_owned(),
            BTreeMap::from([("bennet-800".to_owned(), "Warm negative.".to_owned())]),
        )]);
        index.presets = IndexPreset::all(&text);
        assert!(!index.presets.is_empty());
        let bennet = index.presets.iter().find(|p| p.id == "bennet-800").unwrap();
        assert_eq!(bennet.text["en"], "Warm negative.");
        let bytes = index.to_bytes().unwrap();
        assert_eq!(Index::parse(&bytes).unwrap(), index);
        let mut value: Value = serde_json::from_slice(&bytes).unwrap();
        value["presets"][0]["recipes"] = json!([{"family": "gr3"}]);
        let read = Index::parse(&packaging::json_bytes(&value).unwrap()).unwrap();
        assert_eq!(read.presets.len(), index.presets.len() - 1);
    }

    #[test]
    fn the_index_requires_its_fields() {
        let (site, _, _) = signed_site(&camera::GR3, "1.7.0");
        let bytes = Index::of_site(&site).unwrap().to_bytes().unwrap();
        let mut value: Value = serde_json::from_slice(&bytes).unwrap();
        value.as_object_mut().unwrap().remove("format");
        assert!(Index::parse(&packaging::json_bytes(&value).unwrap()).is_err());
        value = serde_json::from_slice(&bytes).unwrap();
        value.as_object_mut().unwrap().remove("files");
        assert!(Index::parse(&packaging::json_bytes(&value).unwrap()).is_err());
        value = serde_json::from_slice(&bytes).unwrap();
        value["format"] = json!(2);
        assert!(Index::parse(&packaging::json_bytes(&value).unwrap()).is_err());
    }

    #[test]
    fn a_runtime_hash_mismatch_is_an_error() {
        let (mut site, _, signature) = signed_site(&camera::GR3, "1.7.0");
        let (bytes, _) = site.releases["gr3"].clone();
        let mut statement = Statement::parse(&bytes).unwrap();
        let path = statement.runtime.keys().next().unwrap().clone();
        statement.runtime.insert(path, "ab".repeat(32));
        site.releases
            .insert("gr3".into(), (statement.to_bytes().unwrap(), signature));
        let error = format!("{:#}", Index::of_site(&site).unwrap_err());
        assert!(error.contains("do not match"), "{error}");
    }

    #[test]
    fn a_package_from_the_index_verifies() {
        let camera = &camera::GR3;
        let (site, statement, signature) = signed_site(camera, "1.7.0");
        let index = Index::of_site(&site).unwrap();
        let runtimes: Vec<_> = index.cameras["gr3"]
            .targets
            .iter()
            .map(|target| site.runtimes[target].clone())
            .collect();
        let modules: Vec<_> = site.modules.values().cloned().collect();
        let package = packaging::assemble_with(
            &runtimes,
            &testing::effect_set(&["test-effect"]),
            &modules,
            camera,
            &packaging::Guide {
                language: crate::language::Language::English,
                pdf: b"%PDF-1.7\n% test\n",
            },
            packaging::Extras {
                release: Some((&statement, &signature)),
                ..packaging::Extras::default()
            },
        )
        .unwrap();
        let (update, _) =
            release::verify_update(&package.bytes, &[testing::release_key()]).unwrap();
        assert_eq!(update.statement.version, index.version);
        assert_eq!(update.statement.camera, camera);
        assert_eq!(
            index.files["release/gr3/RELEASE.JSON"].sha256,
            packaging::sha256(&statement)
        );
        assert_eq!(
            index.files["release/gr3/RELEASE.SIG"].sha256,
            packaging::sha256(&signature)
        );
    }

    #[test]
    fn a_missing_effect_file_is_an_error() {
        let (mut site, _, _) = signed_site(&camera::GR3, "1.7.0");
        site.effects
            .get_mut("gr3-2.10")
            .unwrap()
            .remove("effect.grf");
        let error = format!("{:#}", Index::of_site(&site).unwrap_err());
        assert!(error.contains("missing"), "{error}");
    }

    #[test]
    fn a_tempdir_site_tree_builds_the_index() {
        let camera = &camera::GR3;
        let (site, _, _) = signed_site(camera, "1.7.0");
        let root = tempfile::tempdir().unwrap();
        for (camera_id, (statement, signature)) in &site.releases {
            let dir = root.path().join("release").join(camera_id);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(release::STATEMENT_ENTRY), statement).unwrap();
            std::fs::write(dir.join(release::SIGNATURE_ENTRY), signature).unwrap();
        }
        for (target, manifest) in &site.manifests {
            let dir = root.path().join("runtime").join(target);
            std::fs::create_dir_all(dir.join("sd")).unwrap();
            std::fs::write(dir.join("build.json"), manifest).unwrap();
            for (path, bytes) in site.runtimes[target].files() {
                let file = dir.join("sd").join(path);
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(file, bytes).unwrap();
            }
        }
        for (name, bytes) in &site.modules {
            let dir = root.path().join("modules");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(name), bytes).unwrap();
        }
        for (target, files) in &site.effects {
            for (path, bytes) in files {
                let file = root.path().join("effects").join(target).join(path);
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(file, bytes).unwrap();
            }
        }
        for (camera_id, guides) in &site.guides {
            for (language, pdf) in guides {
                let file = root
                    .path()
                    .join("guides")
                    .join(camera_id)
                    .join(format!("{language}.pdf"));
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(file, pdf).unwrap();
            }
        }
        assert!(root.path().join("release/gr3/RELEASE.JSON").is_file());
        assert_eq!(
            Index::of_site(&site).unwrap().version,
            Index::parse(&Index::of_site(&site).unwrap().to_bytes().unwrap())
                .unwrap()
                .version
        );
    }

    #[test]
    fn release_bodies_become_plain_notes_newest_first() {
        let notes = notes_from_markdown(&[
            (
                "v1.6.3".into(),
                "### Fixes\n\n- **Color covers** show the `box` colors.\n- See [the guide](https://x).\n\nOther text".into(),
            ),
            (
                "1.7.0".into(),
                "* Wi-Fi updates by @someone in https://github.com/x/pull/1\n![shot](a.png)".into(),
            ),
            ("1.6.2".into(), "## Nothing to list".into()),
        ])
        .unwrap();
        assert_eq!(
            notes,
            vec![
                VersionNotes {
                    version: "1.7.0".into(),
                    items: vec!["Wi-Fi updates".into()],
                },
                VersionNotes {
                    version: "1.6.3".into(),
                    items: vec![
                        "Color covers show the box colors.".into(),
                        "See the guide.".into()
                    ],
                },
            ]
        );
    }

    #[test]
    fn release_notes_reject_bad_input() {
        let long = format!("- {}", "a".repeat(NOTE_CHARS + 1));
        assert!(notes_from_markdown(&[("1.7".into(), "- a".into())]).is_err());
        assert!(notes_from_markdown(&[("1.7.0".into(), long)]).is_err());
        let many = "- a\n".repeat(NOTES_ITEMS + 1);
        assert!(notes_from_markdown(&[("1.7.0".into(), many)]).is_err());
        let versions: Vec<_> = (0..12)
            .map(|minor| (format!("1.{minor}.0"), "- a".to_owned()))
            .collect();
        let notes = notes_from_markdown(&versions).unwrap();
        assert_eq!(notes.len(), NOTES_VERSIONS);
        assert_eq!(notes[0].version, "1.11.0");
    }

    #[test]
    fn notes_round_trip_and_bad_entries_are_left_out() {
        let (site, _, _) = signed_site(&camera::GR3, "1.7.0");
        let mut index = Index::of_site(&site).unwrap();
        index.notes = notes_from_markdown(&[("1.7.0".into(), "- New".into())]).unwrap();
        assert_eq!(Index::parse(&index.to_bytes().unwrap()).unwrap(), index);

        let mut value: Value = serde_json::from_slice(&index.to_bytes().unwrap()).unwrap();
        value["notes"] = json!([
            {"version": "1.7.0", "items": ["Ok", "bad\u{7}", 3]},
            {"version": "next", "items": ["x"]},
            {"version": "1.6.0", "items": []},
        ]);
        let read = Index::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(
            read.notes,
            vec![VersionNotes {
                version: "1.7.0".into(),
                items: vec!["Ok".into()]
            }]
        );
        value["notes"] = json!("nonsense");
        assert!(Index::parse(&serde_json::to_vec(&value).unwrap())
            .unwrap()
            .notes
            .is_empty());
    }
}
