//! Browser entry points that assemble a GRKit SD package and import LUTs as effects.
//!
//! The site serves a runtime build as CI publishes it: `build.json` and the files under `sd/`.
//! It also serves each official module file. The page fetches those files, the selected
//! modules, and the selected effects, then builds the ZIP locally.
//! `Builder` holds the logic so that native tests cover it. `Package` only adapts it for JS.
//! `import_lut` fits a LUT as `grkit effects import` does, over the base gamma of the catalog.

use anyhow::{ensure, Context, Result};
use grkit_core::{
    camera::{self, Camera},
    cube::Cube,
    effects, fit,
    language::Language,
    lut_image,
    packaging::{self, EffectSet},
    site, Files,
};
use serde_json::Value;
use wasm_bindgen::prelude::*;

/// The largest LUT file that the site imports. A 16-bit TIFF HALD image of level 16 has
/// about 100 MB. The CLI accepts more, because it does not share the memory of a page.
pub const MAX_LUT_SIZE: usize = 128 * 1024 * 1024;

/// An effect that `import_lut` made from a LUT.
pub struct Imported {
    /// The effect.json text, as `grkit effects import` writes it.
    pub definition: String,
    /// The compiled `.GRF` effect for the package.
    pub blob: Vec<u8>,
    /// The RAW preview table, as `grkit effects publish` writes it for a catalog effect.
    pub preview: Vec<u8>,
    /// The ADJ icon: 40 x 40 RGBA pixels.
    pub icon: Vec<u8>,
    pub monochrome: bool,
    /// A to D: how closely the effect follows the LUT (`fit::Grade`). The page shows a
    /// description of it, not the letter.
    pub grade: &'static str,
    /// Conditions that the user should check before using the effect. The page describes the
    /// grade itself.
    pub warnings: Vec<String>,
}

/// What the user gives for an import. The fields are the `grkit effects import` options.
pub struct ImportRequest<'a> {
    pub effect_id: &'a str,
    pub name: &'a str,
    /// The `icon` of effect.json: a film cover (`template` box or cine) or a label tile.
    pub icon: &'a Value,
    /// The file name, recorded in the attribution.
    pub source: &'a str,
    /// The input encoding name: `standard`, `srgb`, `linear`, a log encoding, or `auto` for
    /// the one that the LUT data suggests.
    pub input: &'a str,
}

/// Rejects a LUT file over `MAX_LUT_SIZE` and reads the site catalog's base gamma.
fn lut_inputs(bytes: &[u8], base_gamma: &[u16]) -> Result<[u16; 256]> {
    ensure!(
        bytes.len() <= MAX_LUT_SIZE,
        "The LUT file is larger than {} MB",
        MAX_LUT_SIZE >> 20
    );
    site::base_gamma(base_gamma)
}

/// Fits a `.cube` file or a LUT image over `base_gamma`, the table of the site catalog.
pub fn import_lut(bytes: &[u8], base_gamma: &[u16], request: &ImportRequest) -> Result<Imported> {
    import_lut_with_progress(bytes, base_gamma, request, &|_| {})
}

/// `import_lut`, which reports the progress of the fit to `progress`.
pub fn import_lut_with_progress(
    bytes: &[u8],
    base_gamma: &[u16],
    request: &ImportRequest,
    progress: fit::Progress,
) -> Result<Imported> {
    let base_gamma = lut_inputs(bytes, base_gamma)?;
    let imported = fit::import_with_progress(
        bytes,
        &base_gamma,
        &fit::Import {
            effect_id: request.effect_id,
            name: request.name,
            icon: request.icon,
            source: request.source,
            input: match request.input {
                "auto" => None,
                name => Some(name.parse()?),
            },
            // The site has no `--exposure` or `--no-post` control; it fits as the CLI
            // does without them.
            exposure: 0.0,
            options: fit::Options::default(),
        },
        progress,
    )?;
    let blob = effects::compile_definition(&effects::parse_json(imported.text.as_bytes())?)?;
    let (effect, icon) = effects::decode(&blob)?;
    Ok(Imported {
        preview: site::linear_preview_lut(&base_gamma, &effect)?,
        icon,
        blob,
        monochrome: imported.fit.monochrome,
        grade: fit::Grade::of(&imported.fit).letter(),
        warnings: imported.warnings,
        definition: imported.text,
    })
}

/// A look that `native_looks` read from a GR IV firmware file of the user.
pub struct NativeEffect {
    /// The GR IV version of the file.
    pub version: &'static str,
    pub id: String,
    pub name: String,
    /// The effect.json text, as `grkit effects native` writes it.
    pub definition: String,
    /// The compiled `.GRF` effect for the package.
    pub blob: Vec<u8>,
    /// The RAW preview table with the look's default tone steps.
    pub preview: Vec<u8>,
    /// The tone curves of the preview (`site::ToneCurves::to_values`), or none for a look
    /// without default tone steps. `effect_preview` draws the preview again from them.
    pub tone: Vec<u16>,
    /// The ADJ icon: 40 x 40 RGBA pixels.
    pub icon: Vec<u8>,
    pub monochrome: bool,
}

/// Reads the GR IV looks (`gr4_looks::LOOKS`) from an official GR IV zip or its firmware
/// file. The page keeps the file in the browser. `base_gamma` is the table of the
/// site catalog, for the previews.
pub fn native_looks(bytes: &[u8], base_gamma: &[u16]) -> Result<Vec<NativeEffect>> {
    let base_gamma = site::base_gamma(base_gamma)?;
    let image = grkit_core::gr4_looks::Image::read(bytes)?;
    let tables = image.tone_tables()?;
    image
        .definitions()?
        .into_iter()
        .map(|definition| {
            let blob = effects::compile_definition(&definition)?;
            let (effect, icon) = effects::decode(&blob)?;
            let curves = site::native_tone_curves(&effect, &tables);
            Ok(NativeEffect {
                version: image.source.version,
                preview: site::native_preview_lut(&base_gamma, &effect, curves.as_ref())?,
                tone: curves.map(|curves| curves.to_values()).unwrap_or_default(),
                monochrome: effect.monochrome(),
                id: effect.id,
                name: effect.name,
                definition: serde_json::to_string_pretty(&definition)?,
                blob,
                icon,
            })
        })
        .collect()
}

/// The input that a LUT most likely expects, from its data alone: see `fit::guess_input`.
pub fn guess_input(bytes: &[u8], base_gamma: &[u16]) -> Result<&'static str> {
    let base_gamma = lut_inputs(bytes, base_gamma)?;
    Ok(fit::guess_input(&base_gamma, &Cube::read(bytes)?).name())
}

/// One runtime build of a package: its `build.json`, the paths that it lists,
/// and the files that arrived.
struct Build {
    manifest: Vec<u8>,
    paths: Vec<String>,
    files: Files,
}

impl Build {
    fn new(manifest: &[u8]) -> Result<Self> {
        Ok(Self {
            manifest: manifest.to_vec(),
            paths: packaging::manifest_paths(manifest)?,
            files: Files::new(),
        })
    }
}

/// Collects the runtime build of each firmware build of the camera and a set of
/// compiled effects, then assembles the package.
pub struct Builder {
    builds: Vec<Build>,
    effects: EffectSet,
    modules: Vec<Vec<u8>>,
    camera: &'static Camera,
    language: Language,
    guide: Option<Vec<u8>>,
    /// The import library of the site (`grkit_core::imports`), which the camera does not read.
    imports: Option<Vec<u8>>,
}

impl Builder {
    /// Starts a package from the `build.json` of the camera's first runtime
    /// build, which is build 0.
    pub fn new(manifest: &[u8]) -> Result<Self> {
        Ok(Self {
            builds: vec![Build::new(manifest)?],
            effects: EffectSet::default(),
            modules: Vec::new(),
            camera: &camera::GR3,
            language: Language::English,
            guide: None,
            imports: None,
        })
    }

    /// Adds the next runtime build of the camera, in release order, by its
    /// `build.json`, and returns its number.
    pub fn add_build(&mut self, manifest: &[u8]) -> Result<usize> {
        self.builds.push(Build::new(manifest)?);
        Ok(self.builds.len() - 1)
    }

    fn build_mut(&mut self, build: usize) -> Result<&mut Build> {
        self.builds
            .get_mut(build)
            .with_context(|| format!("No runtime build {build}"))
    }

    /// The runtime files of a build to fetch, as paths under its `sd/` directory.
    pub fn runtime_paths(&self, build: usize) -> Result<&[String]> {
        Ok(&self
            .builds
            .get(build)
            .with_context(|| format!("No runtime build {build}"))?
            .paths)
    }

    /// Adds one runtime file of a build. `build` checks its size and SHA-256
    /// against the manifest.
    pub fn add_runtime_file(&mut self, build: usize, path: &str, bytes: &[u8]) -> Result<()> {
        let build = self.build_mut(build)?;
        ensure!(
            build.paths.iter().any(|known| known == path),
            "Not a runtime file of this build: {path}"
        );
        ensure!(
            !build.files.contains_key(path),
            "Duplicate runtime file: {path}"
        );
        build.files.insert(path.to_owned(), bytes.to_vec());
        Ok(())
    }

    /// Adds one compiled effect and returns its ID. The package filename comes from the ID.
    pub fn add_effect(&mut self, blob: &[u8]) -> Result<String> {
        self.effects.insert(blob.to_vec())
    }

    /// Adds one module file (`grkit_core::grm`) and returns its ID. The
    /// installer installs each module of the package.
    pub fn add_module(&mut self, bytes: &[u8]) -> Result<String> {
        let id = grkit_core::grm::Module::decode(bytes)?.id;
        for other in &self.modules {
            ensure!(
                grkit_core::grm::Module::decode(other)?.id != id,
                "Module added twice: {id}"
            );
        }
        self.modules.push(bytes.to_vec());
        Ok(id)
    }

    /// Selects the camera by its ID in `grkit_core::camera::CAMERAS`, GR III by default.
    /// The package gets its ModeSet file. `build` rejects a camera that does not run this runtime.
    pub fn set_camera(&mut self, id: &str) -> Result<()> {
        self.camera = camera::find(id)?;
        Ok(())
    }

    /// Selects the language of the installation guide by its code
    /// (`Language::code`), English by default.
    pub fn set_language(&mut self, code: &str) -> Result<()> {
        self.language = Language::parse(code)?;
        Ok(())
    }

    /// Sets the PDF of the installation guide for the camera, in the language of
    /// `set_language`. The site publishes one for each camera and language.
    pub fn set_guide(&mut self, pdf: &[u8]) {
        self.guide = Some(pdf.to_vec());
    }

    /// Sets the import library of the package (`grkit_core::imports`). The package keeps it for
    /// another browser, and the camera does not read it.
    pub fn set_imports(&mut self, library: &[u8]) -> Result<()> {
        grkit_core::imports::decode(library)?;
        self.imports = Some(library.to_vec());
        Ok(())
    }

    /// Verifies each runtime build against its manifest and returns the verified
    /// package ZIP.
    pub fn build(&self) -> Result<Vec<u8>> {
        let runtimes = self
            .builds
            .iter()
            .map(|build| packaging::load_runtime(&build.manifest, &build.files))
            .collect::<Result<Vec<_>>>()?;
        let package = packaging::assemble_with(
            &runtimes,
            &self.effects,
            &self.modules,
            self.camera,
            &packaging::Guide {
                language: self.language,
                pdf: self
                    .guide
                    .as_deref()
                    .context("Set the installation guide first")?,
            },
            packaging::Extras {
                imports: self.imports.as_deref(),
            },
        )?;
        Ok(package.bytes)
    }

    /// The files that the package puts on the card, at their paths from the card root:
    /// the `SD_ROOT` entries of the verified package that `build` returns.
    pub fn card_files(&self) -> Result<Files> {
        packaging::card_files(&self.build()?)
    }
}

/// What the page shows of an effect file on a card.
pub struct Label {
    pub id: String,
    pub name: String,
    /// The ADJ icon: 40 x 40 RGBA pixels.
    pub icon: Vec<u8>,
    pub monochrome: bool,
}

/// The label of a compiled `.GRF` effect, for an effect file on a card.
pub fn effect_label(blob: &[u8]) -> Result<Label> {
    let (effect, icon) = effects::decode(blob)?;
    Ok(Label {
        id: effect.id,
        name: effect.name,
        icon,
        monochrome: effect.flags & effects::MONOCHROME != 0,
    })
}

/// A compiled `.GRF` effect with a new name, and a new icon when `icon` is given (40 x 40 RGBA
/// pixels, as `iconPixels` gives them). The ID, and so the file name and the ImageTone code of the
/// effect, stay, so the camera keeps its Fn adjustments.
pub fn relabel_effect(blob: &[u8], name: &str, icon: Option<&[u8]>) -> Result<Vec<u8>> {
    let (mut effect, own_icon) = effects::decode(blob)?;
    effects::check_identity(&effect.id, name)?;
    effect.name = name.to_owned();
    effect.encode(icon.unwrap_or(&own_icon))
}

/// The files that a GRKit ZIP of any release puts on the card, at their paths from the card
/// root, so the page can read the ZIP as a card (`packaging::package_files`).
pub fn package_files(bytes: &[u8]) -> Result<Files> {
    packaging::package_files(bytes)
}

/// The RAW preview table of a compiled `.GRF` effect over `base_gamma`, the table of the site
/// catalog. Without `tone`, it is the table that `import_lut` gives for the same effect. With
/// the tone curves of a native look (`NativeEffect::tone`), it is the table of its unlock.
pub fn effect_preview(blob: &[u8], base_gamma: &[u16], tone: &[u16]) -> Result<Vec<u8>> {
    let base_gamma = site::base_gamma(base_gamma)?;
    let (effect, _) = effects::decode(blob)?;
    let curves = match tone {
        [] => None,
        values => Some(site::ToneCurves::from_values(values)?),
    };
    site::native_preview_lut(&base_gamma, &effect, curves.as_ref())
}

/// Builds a GRKit SD package from the runtime build of each firmware build of
/// the camera and compiled effects.
///
/// Each failing call throws an `Error` whose message has the full cause chain.
#[wasm_bindgen]
pub struct Package(Builder);

#[wasm_bindgen]
impl Package {
    /// Starts a package from the `build.json` bytes of the camera's first
    /// runtime build, which is build 0.
    #[wasm_bindgen(constructor)]
    pub fn new(manifest: &[u8]) -> Result<Package, JsError> {
        Builder::new(manifest).map(Package).map_err(js_error)
    }

    /// Adds the next runtime build of the camera, in release order, and returns
    /// its number.
    #[wasm_bindgen(js_name = addBuild)]
    pub fn add_build(&mut self, manifest: &[u8]) -> Result<usize, JsError> {
        self.0.add_build(manifest).map_err(js_error)
    }

    /// The runtime files of a build to fetch, as paths under its `sd/` directory.
    #[wasm_bindgen(js_name = runtimePaths)]
    pub fn runtime_paths(&self, build: usize) -> Result<Vec<String>, JsError> {
        self.0
            .runtime_paths(build)
            .map(<[String]>::to_vec)
            .map_err(js_error)
    }

    /// Adds one runtime file of a build. `build()` checks its size and SHA-256.
    #[wasm_bindgen(js_name = addRuntimeFile)]
    pub fn add_runtime_file(
        &mut self,
        build: usize,
        path: &str,
        bytes: &[u8],
    ) -> Result<(), JsError> {
        self.0
            .add_runtime_file(build, path, bytes)
            .map_err(js_error)
    }

    /// Adds one compiled `.GRF` effect and returns its effect ID.
    #[wasm_bindgen(js_name = addEffect)]
    pub fn add_effect(&mut self, blob: &[u8]) -> Result<String, JsError> {
        self.0.add_effect(blob).map_err(js_error)
    }

    /// Adds one module file (`.GRM`) and returns its module ID.
    #[wasm_bindgen(js_name = addModule)]
    pub fn add_module(&mut self, bytes: &[u8]) -> Result<String, JsError> {
        self.0.add_module(bytes).map_err(js_error)
    }

    /// Selects the camera by its key in `CAMERA_RELEASES` (`generated/cameras.ts`), `gr3`
    /// by default.
    #[wasm_bindgen(js_name = setCamera)]
    pub fn set_camera(&mut self, id: &str) -> Result<(), JsError> {
        self.0.set_camera(id).map_err(js_error)
    }

    /// Selects the language of the installation guide by its site language tag, `en` by
    /// default.
    #[wasm_bindgen(js_name = setLanguage)]
    pub fn set_language(&mut self, code: &str) -> Result<(), JsError> {
        self.0.set_language(code).map_err(js_error)
    }

    /// Sets the PDF of the installation guide for the camera, in the selected language.
    #[wasm_bindgen(js_name = setGuide)]
    pub fn set_guide(&mut self, pdf: &[u8]) {
        self.0.set_guide(pdf);
    }

    /// Sets the import library (`ImportLibrary.bytes`), which the package keeps for another
    /// browser. The camera does not read it.
    #[wasm_bindgen(js_name = setImports)]
    pub fn set_imports(&mut self, library: &[u8]) -> Result<(), JsError> {
        self.0.set_imports(library).map_err(js_error)
    }

    /// Returns the verified package as ZIP bytes.
    pub fn build(&self) -> Result<Vec<u8>, JsError> {
        self.0.build().map_err(js_error)
    }

    /// Returns the files that the verified package puts on the card, at their paths from
    /// the card root, in path order.
    #[wasm_bindgen(js_name = cardFiles)]
    pub fn card_files(&self) -> Result<Vec<CardFile>, JsError> {
        self.0
            .card_files()
            .map(|files| {
                files
                    .into_iter()
                    .map(|(path, bytes)| CardFile { path, bytes })
                    .collect()
            })
            .map_err(js_error)
    }
}

/// One file of a package on the card. See `Package.cardFiles`.
#[wasm_bindgen]
pub struct CardFile {
    path: String,
    bytes: Vec<u8>,
}

#[wasm_bindgen]
impl CardFile {
    /// The path from the card root, such as `SCRIPT/GRKIT/CORE.BIN`.
    #[wasm_bindgen(getter)]
    pub fn path(&self) -> String {
        self.path.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

/// The label of an effect file on a card. See `effectLabel`.
#[wasm_bindgen(js_name = EffectLabel)]
pub struct EffectLabel(Label);

#[wasm_bindgen(js_class = EffectLabel)]
impl EffectLabel {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.0.id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.0.name.clone()
    }

    /// The ADJ icon: 40 x 40 RGBA pixels.
    #[wasm_bindgen(getter)]
    pub fn icon(&self) -> Vec<u8> {
        self.0.icon.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn monochrome(&self) -> bool {
        self.0.monochrome
    }
}

/// The ID, the name, the icon, and the color type of a compiled `.GRF` effect. It fails for a
/// file that is not an effect.
#[wasm_bindgen(js_name = effectLabel)]
pub fn effect_label_js(blob: &[u8]) -> Result<EffectLabel, JsError> {
    effect_label(blob).map(EffectLabel).map_err(js_error)
}

/// Returns a compiled `.GRF` effect with the name `name`, and with `icon` (40 x 40 RGBA pixels
/// from `iconPixels`) as its icon when it is given. The ID stays. It fails for a file that is not
/// an effect, a name that the camera cannot show, or an icon of another size.
#[wasm_bindgen(js_name = relabelEffect)]
pub fn relabel_effect_js(
    blob: &[u8],
    name: &str,
    icon: Option<Vec<u8>>,
) -> Result<Vec<u8>, JsError> {
    relabel_effect(blob, name, icon.as_deref()).map_err(js_error)
}

/// Returns the files that a GRKit ZIP of any release puts on the card, at their paths from the
/// card root, in path order. It fails for a file that is not a ZIP within the package limits.
#[wasm_bindgen(js_name = packageFiles)]
pub fn package_files_js(bytes: &[u8]) -> Result<Vec<CardFile>, JsError> {
    package_files(bytes)
        .map(|files| {
            files
                .into_iter()
                .map(|(path, bytes)| CardFile { path, bytes })
                .collect()
        })
        .map_err(js_error)
}

/// Collects the imports of a browser for the import library of a package
/// (`grkit_core::imports`).
#[wasm_bindgen]
#[derive(Default)]
pub struct ImportLibrary(Vec<grkit_core::imports::Import>);

#[wasm_bindgen]
impl ImportLibrary {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an import: the JSON object text of its details and its compiled `.GRF` effect.
    pub fn add(&mut self, details: String, effect: Vec<u8>) {
        self.0.push(grkit_core::imports::Import { details, effect });
    }

    /// Returns the library file. It fails for details that are not a JSON object, an effect
    /// that is not complete, or an ID that occurs twice.
    pub fn bytes(&self) -> Result<Vec<u8>, JsError> {
        grkit_core::imports::encode(&self.0).map_err(js_error)
    }
}

/// One import of an import library. See `readImportLibrary`.
#[wasm_bindgen]
pub struct LibraryImport(grkit_core::imports::Import);

#[wasm_bindgen]
impl LibraryImport {
    /// The JSON object text of the details.
    #[wasm_bindgen(getter)]
    pub fn details(&self) -> String {
        self.0.details.clone()
    }

    /// The compiled `.GRF` effect.
    #[wasm_bindgen(getter)]
    pub fn effect(&self) -> Vec<u8> {
        self.0.effect.clone()
    }
}

/// Returns the imports of an import library file, in file order. It fails for a file that is
/// not a complete library.
#[wasm_bindgen(js_name = readImportLibrary)]
pub fn read_import_library(bytes: &[u8]) -> Result<Vec<LibraryImport>, JsError> {
    grkit_core::imports::decode(bytes)
        .map(|imports| imports.into_iter().map(LibraryImport).collect())
        .map_err(js_error)
}

/// Returns the RAW preview table of a compiled `.GRF` effect. `baseGamma` is the `base_gamma`
/// array of the site catalog. `tone` is empty, or the tone curves of a native look
/// (`NativeLookEffect.tone`).
#[wasm_bindgen(js_name = effectPreview)]
pub fn effect_preview_js(
    blob: &[u8],
    base_gamma: &[u16],
    tone: &[u16],
) -> Result<Vec<u8>, JsError> {
    effect_preview(blob, base_gamma, tone).map_err(js_error)
}

/// Returns the ZIP that turns SD scripts off on a camera, by
/// its key in `CAMERA_RELEASES` (`generated/cameras.ts`).
#[wasm_bindgen(js_name = scriptsOffPackage)]
pub fn scripts_off_package(camera: &str) -> Result<Vec<u8>, JsError> {
    grkit_core::camera::find(camera)
        .and_then(grkit_core::packaging::assemble_scripts_off)
        .map_err(js_error)
}

/// The files that turn SD scripts off on a camera, at their paths from the card root: its ModeSet
/// file with the disable command and `DEVELOP.MOD`. `camera` is a key of `CAMERA_RELEASES`.
#[wasm_bindgen(js_name = scriptsOffFiles)]
pub fn scripts_off_files(camera: &str) -> Result<Vec<CardFile>, JsError> {
    grkit_core::camera::find(camera)
        .map(|camera| {
            grkit_core::runtime::deactivation_files(camera)
                .into_iter()
                .map(|(path, bytes)| CardFile { path, bytes })
                .collect()
        })
        .map_err(js_error)
}

/// A look of a GR IV firmware file. See `nativeLooks`.
#[wasm_bindgen(js_name = NativeLookEffect)]
pub struct NativeLookEffect(NativeEffect);

#[wasm_bindgen(js_class = NativeLookEffect)]
impl NativeLookEffect {
    /// The GR IV version of the file, such as `1.20`.
    #[wasm_bindgen(getter)]
    pub fn version(&self) -> String {
        self.0.version.to_owned()
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.0.id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.0.name.clone()
    }

    /// The effect.json text.
    #[wasm_bindgen(getter)]
    pub fn definition(&self) -> String {
        self.0.definition.clone()
    }

    /// The compiled `.GRF` effect, for `Package.addEffect`.
    #[wasm_bindgen(getter)]
    pub fn blob(&self) -> Vec<u8> {
        self.0.blob.clone()
    }

    /// The RAW preview table: 33 x 33 x 33 RGB bytes over sRGB-shaped linear input.
    #[wasm_bindgen(getter)]
    pub fn preview(&self) -> Vec<u8> {
        self.0.preview.clone()
    }

    /// The tone curves of the preview, empty for a look without default tone steps. Keep them
    /// to draw the preview again with `effectPreview`.
    #[wasm_bindgen(getter)]
    pub fn tone(&self) -> Vec<u16> {
        self.0.tone.clone()
    }

    /// The ADJ icon: 40 x 40 RGBA pixels.
    #[wasm_bindgen(getter)]
    pub fn icon(&self) -> Vec<u8> {
        self.0.icon.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn monochrome(&self) -> bool {
        self.0.monochrome
    }
}

/// Reads the GR IV looks from an official GR IV zip or its firmware file, which the user
/// chose. `baseGamma` is the `base_gamma` array of the site catalog. It
/// decodes about 40 MB, so call it from a Worker.
#[wasm_bindgen(js_name = nativeLooks)]
pub fn native_looks_js(bytes: &[u8], base_gamma: &[u16]) -> Result<Vec<NativeLookEffect>, JsError> {
    native_looks(bytes, base_gamma)
        .map(|looks| looks.into_iter().map(NativeLookEffect).collect())
        .map_err(js_error)
}

/// An effect fitted from a LUT. See `importLut`.
#[wasm_bindgen(js_name = ImportedEffect)]
pub struct ImportedEffect(Imported);

#[wasm_bindgen(js_class = ImportedEffect)]
impl ImportedEffect {
    /// The effect.json text.
    #[wasm_bindgen(getter)]
    pub fn definition(&self) -> String {
        self.0.definition.clone()
    }

    /// The compiled `.GRF` effect, for `Package.addEffect`.
    #[wasm_bindgen(getter)]
    pub fn blob(&self) -> Vec<u8> {
        self.0.blob.clone()
    }

    /// The RAW preview table: 33 x 33 x 33 RGB bytes over sRGB-shaped linear input.
    #[wasm_bindgen(getter)]
    pub fn preview(&self) -> Vec<u8> {
        self.0.preview.clone()
    }

    /// The ADJ icon: 40 x 40 RGBA pixels.
    #[wasm_bindgen(getter)]
    pub fn icon(&self) -> Vec<u8> {
        self.0.icon.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn monochrome(&self) -> bool {
        self.0.monochrome
    }

    /// A to D: how closely the effect follows the LUT.
    #[wasm_bindgen(getter)]
    pub fn grade(&self) -> String {
        self.0.grade.to_owned()
    }

    /// Conditions that the user should check before using the effect, in English.
    #[wasm_bindgen(getter)]
    pub fn warnings(&self) -> Vec<String> {
        self.0.warnings.clone()
    }
}

#[wasm_bindgen]
extern "C" {
    /// A function that takes the progress of an import, a fraction from 0 to 1.
    #[wasm_bindgen(typescript_type = "(fraction: number) => void")]
    pub type ImportProgress;

    #[wasm_bindgen(method, js_name = call)]
    fn report(this: &ImportProgress, context: &JsValue, fraction: f64);
}

/// Fits a `.cube` file or a LUT image (PNG or TIFF) into an effect, as `grkit effects import`
/// does. `baseGamma` is the `base_gamma` array of the site catalog. The fit takes seconds,
/// so call it from a Worker. `progress` receives the progress of the fit, a fraction from 0
/// to 1 that does not decrease.
/// `icon` is the JSON text of the effect.json `icon`, such as
/// `{"template":"box","brand":"Kodak","name":"PORTRA","number":"400","band":true}`.
#[wasm_bindgen(js_name = importLut)]
// The arguments are the fields of the worker's import request, one JavaScript value each.
#[allow(clippy::too_many_arguments)]
pub fn import_lut_js(
    bytes: &[u8],
    source: &str,
    input: &str,
    base_gamma: &[u16],
    effect_id: &str,
    name: &str,
    icon: &str,
    progress: &ImportProgress,
) -> Result<ImportedEffect, JsError> {
    let icon: Value =
        serde_json::from_str(icon).map_err(|error| JsError::new(&error.to_string()))?;
    let request = ImportRequest {
        effect_id,
        name,
        icon: &icon,
        source,
        input,
    };
    import_lut_with_progress(bytes, base_gamma, &request, &|fraction| {
        progress.report(&JsValue::NULL, fraction)
    })
    .map(ImportedEffect)
    .map_err(js_error)
}

/// The input that a `.cube` file or LUT image most likely expects (`standard`, `vlog`,
/// `flog`, `flog2`, or `flog2c`), from its data alone. `baseGamma` is the `base_gamma` array
/// of the site catalog. It reads the whole file, so call it from a Worker.
#[wasm_bindgen(js_name = guessInput)]
pub fn guess_input_js(bytes: &[u8], base_gamma: &[u16]) -> Result<String, JsError> {
    guess_input(bytes, base_gamma)
        .map(str::to_owned)
        .map_err(js_error)
}

/// The camera pixels of an effect.json `icon` (JSON text): 40 x 40 RGBA. It fails with the
/// reason when a text does not fit, so the page can check an icon before an import.
#[wasm_bindgen(js_name = iconPixels)]
pub fn icon_pixels_js(icon: &str) -> Result<Vec<u8>, JsError> {
    let icon: Value =
        serde_json::from_str(icon).map_err(|error| JsError::new(&error.to_string()))?;
    effects::icon_pixels(&icon).map_err(js_error)
}

/// An identity HALD PNG of `level` (2 to 16), to turn an editor preset into a LUT image.
#[wasm_bindgen(js_name = haldImage)]
pub fn hald_image(level: usize) -> Result<Vec<u8>, JsError> {
    lut_image::identity_png(level).map_err(js_error)
}

fn js_error(error: anyhow::Error) -> JsError {
    JsError::new(&format!("{error:#}"))
}

/// A fast-boot update file (`grkit_firmware`), which the page writes under the name of
/// the official file.
#[wasm_bindgen]
pub struct FastbootFirmware(grkit_firmware::Patched);

#[wasm_bindgen]
impl FastbootFirmware {
    /// The release, such as `gr3-2.10`.
    #[wasm_bindgen(getter)]
    pub fn release(&self) -> String {
        self.0.release.name()
    }

    /// The name of the update file, such as `fwdc239b.bin`.
    #[wasm_bindgen(getter, js_name = fileName)]
    pub fn file_name(&self) -> String {
        self.0.release.update.name.clone().unwrap_or_default()
    }

    /// The SHA-256 of the new file.
    #[wasm_bindgen(getter)]
    pub fn sha256(&self) -> String {
        self.0.release.result.sha256.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn bytes(&self) -> Vec<u8> {
        self.0.bytes.clone()
    }
}

/// Returns the fast-boot update file of an official update file, or of the official ZIP that
/// holds it, and undefined for any other file. It fails only when the patch of an official file
/// fails.
#[wasm_bindgen(js_name = patchFirmware)]
pub fn patch_firmware(bytes: &[u8]) -> Result<Option<FastbootFirmware>, JsError> {
    let Ok(update) = grkit_firmware::official_update(bytes) else {
        return Ok(None);
    };
    let Ok(release) = grkit_firmware::release_of(&update) else {
        return Ok(None);
    };
    grkit_firmware::patch_with(&update, release)
        .map(|patched| Some(FastbootFirmware(patched)))
        .map_err(js_error)
}
