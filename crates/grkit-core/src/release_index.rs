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
use serde_json::{json, Value};

use crate::{
    camera,
    packaging::{self, VerifiedRuntime},
    release::{self, Statement},
    site, Files,
};

/// The version of the release index format.
pub const FORMAT: u64 = 1;

/// One file that the app may download from the site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    pub sha256: String,
    pub size: u64,
}

/// The signed release of one camera on the site.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// The release that a site holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    /// The GRKit version, which every signed statement must name.
    pub version: String,
    pub cameras: BTreeMap<String, CameraRelease>,
    /// Every downloadable file, by its path from the site root.
    pub files: BTreeMap<String, FileInfo>,
}

/// The files of a site that the index covers. The CLI fills this from disk.
#[derive(Debug, Clone, Default)]
pub struct SiteContents {
    /// `release/<camera>/`: statement and signature bytes, by camera id.
    pub releases: BTreeMap<String, (Vec<u8>, Vec<u8>)>,
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

        for (name, bytes) in &site.modules {
            insert_file(&mut files, &format!("modules/{name}"), bytes)?;
        }

        Ok(Index {
            version: version.expect("checked non-empty releases"),
            cameras,
            files,
        })
    }

    /// The bytes of the index file.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let cameras: BTreeMap<_, _> = self
            .cameras
            .iter()
            .map(|(id, camera)| {
                (
                    id.clone(),
                    json!({
                        "targets": camera.targets,
                        "statement": camera.statement,
                        "signature": camera.signature,
                        "guides": camera.guides,
                    }),
                )
            })
            .collect();
        let files: BTreeMap<_, _> = self
            .files
            .iter()
            .map(|(path, info)| {
                (
                    path.clone(),
                    json!({"sha256": info.sha256, "size": info.size}),
                )
            })
            .collect();
        packaging::json_bytes(&json!({
            "format": FORMAT,
            "version": self.version,
            "cameras": cameras,
            "files": files,
        }))
    }

    /// Reads an index. It does not check the files on a site.
    pub fn parse(bytes: &[u8]) -> Result<Index> {
        let value: Value = serde_json::from_slice(bytes).context("Read the release index")?;
        ensure!(
            value["format"] == FORMAT,
            "The release index has another format"
        );
        let version = value["version"]
            .as_str()
            .context("The release index has no version")?
            .to_owned();
        let cameras = value["cameras"]
            .as_object()
            .context("The release index has no cameras")?
            .iter()
            .map(|(id, entry)| {
                camera::find(id)?;
                let targets = entry["targets"]
                    .as_array()
                    .context("A camera release has no targets")?
                    .iter()
                    .map(|target| {
                        target
                            .as_str()
                            .context("A camera release has a bad target")
                            .map(str::to_owned)
                    })
                    .collect::<Result<Vec<_>>>()?;
                let guides = entry["guides"]
                    .as_object()
                    .context("A camera release has no guides")?
                    .iter()
                    .map(|(language, path)| {
                        let path = path
                            .as_str()
                            .context("A camera release has a bad guide path")?
                            .to_owned();
                        Ok((language.clone(), path))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()?;
                Ok((
                    id.clone(),
                    CameraRelease {
                        targets,
                        statement: entry["statement"]
                            .as_str()
                            .context("A camera release has no statement path")?
                            .to_owned(),
                        signature: entry["signature"]
                            .as_str()
                            .context("A camera release has no signature path")?
                            .to_owned(),
                        guides,
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let files = value["files"]
            .as_object()
            .context("The release index has no files")?
            .iter()
            .map(|(path, info)| {
                let sha256 = info["sha256"]
                    .as_str()
                    .filter(|hash| {
                        hash.len() == 64
                            && hash
                                .bytes()
                                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    })
                    .with_context(|| format!("The release index has no SHA-256 for {path}"))?
                    .to_owned();
                let size = info["size"]
                    .as_u64()
                    .with_context(|| format!("The release index has no size for {path}"))?;
                Ok((path.clone(), FileInfo { sha256, size }))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        Ok(Index {
            version,
            cameras,
            files,
        })
    }
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
}
