//! Browser entry points that assemble a GRKit SD package.
//!
//! The site serves a runtime build as CI publishes it: `build.json` and the files under `sd/`.
//! The page fetches those files and the selected effects, then builds the ZIP locally.
//! `Builder` holds the logic so that native tests cover it. `Package` only adapts it for JS.

use anyhow::{ensure, Context, Result};
use grkit_core::{effects, packaging, runtime, Files};
use wasm_bindgen::prelude::*;

/// Collects one runtime build and a set of compiled effects, then assembles the package.
pub struct Builder {
    manifest: Vec<u8>,
    paths: Vec<String>,
    runtime: Files,
    effects: Files,
}

impl Builder {
    /// Starts a package from the runtime's `build.json`.
    pub fn new(manifest: &[u8]) -> Result<Self> {
        let parsed = packaging::read_manifest(manifest)?;
        let paths = parsed["files"]
            .as_array()
            .context("Expected a manifest file list")?
            .iter()
            .map(|item| {
                item["path"]
                    .as_str()
                    .map(str::to_owned)
                    .context("Expected a runtime path")
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            manifest: manifest.to_vec(),
            paths,
            runtime: Files::new(),
            effects: Files::new(),
        })
    }

    /// The runtime files to fetch, as paths under the build's `sd/` directory.
    pub fn runtime_paths(&self) -> &[String] {
        &self.paths
    }

    /// Adds one runtime file. `build` checks its size and SHA-256 against the manifest.
    pub fn add_runtime_file(&mut self, path: &str, bytes: &[u8]) -> Result<()> {
        ensure!(
            self.paths.iter().any(|known| known == path),
            "Not a runtime file of this build: {path}"
        );
        ensure!(
            !self.runtime.contains_key(path),
            "Duplicate runtime file: {path}"
        );
        self.runtime.insert(path.to_owned(), bytes.to_vec());
        Ok(())
    }

    /// Adds one compiled effect and returns its ID. The package filename comes from the ID.
    pub fn add_effect(&mut self, blob: &[u8]) -> Result<String> {
        let (effect, _) = effects::decode(blob)?;
        let id = effect["effect_id"]
            .as_str()
            .context("Expected an effect ID")?
            .to_owned();
        let name = effects::filename(&id)?;
        ensure!(
            !self.effects.contains_key(&name),
            "Duplicate effect ID or filename hash collision: {id}"
        );
        ensure!(
            self.effects.len() < runtime::MAX_COUNT,
            "This target supports at most {} effects",
            runtime::MAX_COUNT
        );
        self.effects.insert(name, blob.to_vec());
        Ok(id)
    }

    /// Verifies the runtime against its manifest and returns the verified package ZIP.
    pub fn build(&self) -> Result<Vec<u8>> {
        let (runtime, manifest) = packaging::load_runtime(&self.manifest, &self.runtime)?;
        packaging::assemble(&runtime, &manifest, &self.effects)
    }
}

/// Builds a GRKit SD package from a runtime build and compiled effects.
///
/// Each failing call throws an `Error` whose message has the full cause chain.
#[wasm_bindgen]
pub struct Package(Builder);

#[wasm_bindgen]
impl Package {
    /// Starts a package from the runtime's `build.json` bytes.
    #[wasm_bindgen(constructor)]
    pub fn new(manifest: &[u8]) -> Result<Package, JsError> {
        Builder::new(manifest).map(Package).map_err(js_error)
    }

    /// The runtime files to fetch, as paths under the build's `sd/` directory.
    #[wasm_bindgen(js_name = runtimePaths)]
    pub fn runtime_paths(&self) -> Vec<String> {
        self.0.runtime_paths().to_vec()
    }

    /// Adds one runtime file. `build()` checks its size and SHA-256.
    #[wasm_bindgen(js_name = addRuntimeFile)]
    pub fn add_runtime_file(&mut self, path: &str, bytes: &[u8]) -> Result<(), JsError> {
        self.0.add_runtime_file(path, bytes).map_err(js_error)
    }

    /// Adds one compiled `.GRF` effect and returns its effect ID.
    #[wasm_bindgen(js_name = addEffect)]
    pub fn add_effect(&mut self, blob: &[u8]) -> Result<String, JsError> {
        self.0.add_effect(blob).map_err(js_error)
    }

    /// Returns the verified package as ZIP bytes.
    pub fn build(&self) -> Result<Vec<u8>, JsError> {
        self.0.build().map_err(js_error)
    }
}

fn js_error(error: anyhow::Error) -> JsError {
    JsError::new(&format!("{error:#}"))
}
