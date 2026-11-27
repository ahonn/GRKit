//! The demo camera: [`CameraApi`] answered from memory. It follows the
//! camera service's rules that the UI depends on, and steps through an
//! update on a timer: a download, a send, a confirmation on the camera,
//! and an install at the next start.

use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Duration};

use grkit_core::release_index::{IndexPreset, VersionNotes};
use nsh_core::prelude::timer;
use serde::{Deserialize, Serialize};

use super::{fake_looks, fake_modules, fake_recipes};
use crate::camera_api::{
    CameraApi, CatalogProblem, DownloadError, Fut, ModuleAccess, ModuleCache, ReleaseOffer,
    ReleasePackage,
};
use crate::domain::{
    Camera, CameraModel, CameraStatus, Catalog, CatalogLook, ConnectionState, JournalState,
    LinkError, LookError, LooksSnapshot, PackageCamera, PackageInfo, PackageResponse,
    ReleaseProblem, RememberedCamera, UpdateError, UpdateIntent, UpdateSnapshot,
    camera_settings::{CameraSettings, SettingChange, parse_camera_settings},
    look_media,
    looks::LookChange,
    modules::{
        CatalogAnswer, ModuleCatalog, ModuleDraft, ModuleLayout, ModulePatch, ModuleState,
        ModulesState,
    },
    recipes::{RecipeChange, RecipeError, RecipesSnapshot},
};

/// The camera start of the demo camera.
const BOOT_ID: &str = "demo-boot";
/// How long the demo camera takes to write a staged package.
const SEND_TIME: Duration = Duration::from_millis(600);
/// The steps of the download, and the time of each.
const DOWNLOAD_STEPS: u32 = 4;
const DOWNLOAD_STEP: Duration = Duration::from_millis(150);
/// How many reads of the update journal the person takes to confirm the
/// update on the camera.
const CONFIRM_READS: u32 = 3;

/// How a demo update fails, if at all (`GRKIT_DEMO_FAIL`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DemoFail {
    #[default]
    None,
    CardFull,
    Refused,
    ConnectionLost,
}

impl DemoFail {
    pub fn from_env() -> Self {
        match std::env::var("GRKIT_DEMO_FAIL").as_deref() {
            Ok("card_full") => DemoFail::CardFull,
            Ok("refused") => DemoFail::Refused,
            Ok("connection_lost") => DemoFail::ConnectionLost,
            _ => DemoFail::None,
        }
    }
}

/// The demo camera and its GRKit.
#[derive(Clone, Debug)]
pub struct FakeConfig {
    pub model: CameraModel,
    pub firmware: String,
    pub installed: String,
    /// The release that the site offers.
    pub available: String,
    pub release_notes: String,
    pub package_bytes: u64,
    pub fail: DemoFail,
    /// What a card-full error says is missing.
    pub card_full_bytes: u64,
}

impl Default for FakeConfig {
    fn default() -> Self {
        Self {
            model: CameraModel::Gr3,
            firmware: "2.21".into(),
            installed: "1.6.1".into(),
            available: "1.7.0".into(),
            release_notes: "Faster Wi-Fi updates and clearer confirm steps on the camera.".into(),
            package_bytes: 4_700_000,
            fail: DemoFail::None,
            card_full_bytes: 12 * 1024 * 1024,
        }
    }
}

/// How far the demo camera's GRKit serves module settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModuleSupport {
    #[default]
    Full,
    /// A GRKit from before module settings.
    None,
    /// A catalog in a format newer than the app.
    TooNew,
}

/// The update that a demo package carries, and that the card holds once
/// it is staged.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Staged {
    version: String,
    modules: Option<Vec<String>>,
}

struct State {
    config: FakeConfig,
    installed: String,
    package_modules: Option<Vec<String>>,
    journal: JournalState,
    staged: Option<Staged>,
    /// Journal reads since the update was staged.
    reads: u32,
    looks: fake_looks::FakeLooks,
    catalog: Vec<(CatalogLook, Vec<u8>)>,
    recipes: RecipesSnapshot,
    modules: ModulesState,
    module_support: ModuleSupport,
    settings: CameraSettings,
    picture: Option<Vec<u8>>,
    // What the phone keeps, in memory: the demo leaves no files.
    intent: Option<UpdateIntent>,
    seen: BTreeMap<String, String>,
    drafts: BTreeMap<String, ModuleDraft>,
    saved: Vec<RememberedCamera>,
}

impl State {
    fn set_journal(&mut self, journal: JournalState) {
        self.journal = journal;
        // As the camera does: an update that waits refuses Look changes.
        self.looks.update_pending = journal != JournalState::None;
    }

    fn camera(&self) -> Camera {
        Camera {
            model: self.config.model,
            firmware: self.config.firmware.clone(),
            installed_version: self.installed.clone(),
            connection: ConnectionState::Connected,
        }
    }

    fn package(&self) -> PackageResponse {
        PackageResponse {
            version: Some(self.installed.clone()),
            modules: self
                .package_modules
                .as_ref()
                .map(|ids| serde_json::json!(ids)),
            effects: None,
            left_out: None,
            camera: None,
        }
    }

    /// The camera starts again: a confirmed update installs.
    fn start(&mut self) {
        if self.journal != JournalState::Confirmed {
            return;
        }
        if let Some(staged) = self.staged.take() {
            self.installed = staged.version;
            if staged.modules.is_some() {
                self.package_modules = staged.modules;
            }
        }
        self.set_journal(JournalState::None);
    }
}

/// The demo camera. Clones share it: the demo screens seed the one that
/// the app talks to.
#[derive(Clone)]
pub struct FakeCamera(Rc<RefCell<State>>);

fn ready<T: 'static>(value: T) -> Fut<T> {
    Box::pin(std::future::ready(value))
}

impl FakeCamera {
    pub fn new(config: FakeConfig) -> Self {
        Self(Rc::new(RefCell::new(State {
            installed: config.installed.clone(),
            config,
            package_modules: None,
            journal: JournalState::None,
            staged: None,
            reads: 0,
            looks: fake_looks::FakeLooks::demo(),
            catalog: fake_looks::demo_catalog(),
            recipes: fake_recipes::demo_recipes(),
            modules: fake_modules::demo_state(),
            module_support: ModuleSupport::Full,
            settings: demo_settings(),
            picture: None,
            intent: None,
            seen: BTreeMap::new(),
            drafts: BTreeMap::new(),
            saved: Vec::new(),
        })))
    }

    /// Another camera or GRKit version: the update starts over.
    pub fn configure(&self, config: FakeConfig) {
        let mut state = self.0.borrow_mut();
        state.installed = config.installed.clone();
        state.config = config;
        state.staged = None;
        state.set_journal(JournalState::None);
    }

    pub fn config(&self) -> FakeConfig {
        self.0.borrow().config.clone()
    }

    pub fn edit_looks(&self, edit: impl FnOnce(&mut fake_looks::FakeLooks)) {
        edit(&mut self.0.borrow_mut().looks);
    }

    pub fn edit_recipes(&self, edit: impl FnOnce(&mut RecipesSnapshot)) {
        edit(&mut self.0.borrow_mut().recipes);
    }

    pub fn edit_modules(&self, edit: impl FnOnce(&mut ModulesState)) {
        edit(&mut self.0.borrow_mut().modules);
    }

    pub fn set_module_support(&self, support: ModuleSupport) {
        self.0.borrow_mut().module_support = support;
    }

    /// An update of the release that waits on the camera in `journal`.
    pub fn set_journal(&self, journal: JournalState) {
        let mut state = self.0.borrow_mut();
        state.staged = (journal != JournalState::None).then(|| Staged {
            version: state.config.available.clone(),
            modules: None,
        });
        state.reads = 0;
        state.set_journal(journal);
    }

    /// The camera and its GRKit, as a connect reads them.
    pub fn session(&self) -> (CameraStatus, PackageInfo) {
        let state = self.0.borrow();
        let status = CameraStatus {
            camera: state.camera(),
            powered_on: true,
            device_id: None,
        };
        let package = PackageInfo {
            installed: state.installed.clone(),
            available: Some(state.config.available.clone()),
            firmware: state.config.firmware.clone(),
            release_notes: state.config.release_notes.clone(),
            package_bytes: state.config.package_bytes,
        };
        (status, package)
    }

    /// The file of a Look on the demo camera.
    pub fn look_content(&self, id: &str) -> Option<Vec<u8>> {
        self.0.borrow().looks.content(id)
    }
}

/// The package of the demo release: it names the version, and the modules
/// it keeps.
struct FakeRelease {
    version: String,
}

impl ReleasePackage for FakeRelease {
    fn download(
        &self,
        before: &PackageResponse,
        progress: Box<dyn Fn(f32)>,
    ) -> Fut<Result<Vec<u8>, DownloadError>> {
        let staged = Staged {
            version: self.version.clone(),
            modules: before
                .modules
                .clone()
                .and_then(|ids| serde_json::from_value(ids).ok()),
        };
        Box::pin(async move {
            for step in 1..=DOWNLOAD_STEPS {
                timer(DOWNLOAD_STEP).await;
                progress(step as f32 / DOWNLOAD_STEPS as f32);
            }
            serde_json::to_vec(&staged).map_err(|error| DownloadError::Package(error.to_string()))
        })
    }
}

impl CameraApi for FakeCamera {
    fn simulated(&self) -> Option<CameraModel> {
        Some(self.0.borrow().config.model)
    }

    fn read_package(
        &self,
        _fallback: CameraModel,
    ) -> Fut<Result<(CameraStatus, PackageInfo), LinkError>> {
        ready(Ok(self.session()))
    }

    fn finish(&self, power_off: bool) -> Fut<bool> {
        if power_off {
            // The demo camera is back at once.
            self.0.borrow_mut().start();
        }
        ready(true)
    }

    fn looks(&self) -> Fut<Result<LooksSnapshot, LookError>> {
        ready(Ok(self.0.borrow().looks.snapshot()))
    }

    fn look_content(&self, id: &str, _content_hash: &str) -> Fut<Result<Vec<u8>, LookError>> {
        ready(self.0.borrow().looks.content(id).ok_or(LookError::NotFound))
    }

    fn change_look(&self, _boot_id: &str, change: LookChange) -> Fut<Result<(), LookError>> {
        ready(self.0.borrow_mut().looks.apply(change))
    }

    fn look_catalog(
        &self,
        _model: CameraModel,
        _firmware: &str,
    ) -> Fut<Result<(String, Catalog), CatalogProblem>> {
        let looks = self
            .0
            .borrow()
            .catalog
            .iter()
            .map(|(look, _)| look.clone())
            .collect();
        let catalog = Catalog {
            target: "demo".into(),
            base_gamma: look_media::demo_base_gamma(),
            looks,
        };
        ready(Ok(("demo".into(), catalog)))
    }

    fn catalog_file(&self, _target: &str, look: &CatalogLook) -> Fut<Result<Vec<u8>, String>> {
        ready(catalog_grf(&self.0.borrow(), look))
    }

    fn preview_table(&self, _target: &str, look: &CatalogLook) -> Fut<Result<Vec<u8>, String>> {
        // The site's table of an official Look, made here from its file.
        let grf = catalog_grf(&self.0.borrow(), look);
        Box::pin(async move {
            let grf = grf?;
            crate::store::off_thread(move || {
                look_media::preview_table(&grf, &look_media::demo_base_gamma())
                    .map_err(|error| error.to_string())
            })
            .await
            .unwrap_or_else(|| Err("The preview was not made.".into()))
        })
    }

    fn standard_table(&self, _target: &str) -> Fut<Result<Vec<u8>, String>> {
        // The site's table of the camera's Standard, made here as the site
        // makes it.
        Box::pin(async move {
            crate::store::off_thread(|| {
                let base: [u16; 256] = look_media::demo_base_gamma()
                    .try_into()
                    .map_err(|_| "Expected 256 base gamma values".to_owned())?;
                Ok(grkit_core::site::standard_linear_preview_lut(&base))
            })
            .await
            .unwrap_or_else(|| Err("The preview was not made.".into()))
        })
    }

    fn recipes(&self) -> Fut<Result<RecipesSnapshot, RecipeError>> {
        let state = self.0.borrow();
        ready(Ok(fake_recipes::served(
            &state.recipes,
            &state.looks.snapshot(),
        )))
    }

    fn change_recipe(
        &self,
        _boot_id: &str,
        change: RecipeChange,
    ) -> Fut<Result<RecipesSnapshot, RecipeError>> {
        let mut state = self.0.borrow_mut();
        let answer = fake_recipes::apply(&mut state.recipes, &change)
            .map(|()| fake_recipes::served(&state.recipes, &state.looks.snapshot()));
        ready(answer)
    }

    fn module_access(&self) -> Fut<Result<ModuleAccess, LookError>> {
        let served = self.0.borrow().module_support != ModuleSupport::None;
        ready(Ok(ModuleAccess {
            read: served,
            write: served,
            boot_id: BOOT_ID.into(),
        }))
    }

    fn module_catalog(
        &self,
        language: &str,
        _cached: Option<ModuleCatalog>,
    ) -> Fut<Result<CatalogAnswer, LookError>> {
        ready(Ok(match self.0.borrow().module_support {
            ModuleSupport::TooNew => CatalogAnswer::TooNew,
            _ => CatalogAnswer::Fresh(fake_modules::demo_catalog(language), Vec::new()),
        }))
    }

    fn module_state(&self) -> Fut<Result<ModulesState, LookError>> {
        ready(Ok(self.0.borrow().modules.clone()))
    }

    fn module_icon(&self, id: &str) -> Fut<Result<Option<Vec<u8>>, LookError>> {
        ready(Ok(fake_modules::demo_icon(id).map(<[u8]>::to_vec)))
    }

    fn patch_module(
        &self,
        _boot_id: &str,
        id: &str,
        patch: ModulePatch,
    ) -> Fut<Result<(ModuleState, u64), LookError>> {
        ready(fake_modules::patch(
            &mut self.0.borrow_mut().modules,
            id,
            &patch,
        ))
    }

    fn put_module_order(
        &self,
        _boot_id: &str,
        ids: &[String],
    ) -> Fut<Result<ModulesState, LookError>> {
        let mut state = self.0.borrow_mut();
        state.modules = state.modules.with_order(ids);
        state.modules.revision += 1;
        ready(Ok(state.modules.clone()))
    }

    fn put_module_layout(
        &self,
        _boot_id: &str,
        layout: &ModuleLayout,
    ) -> Fut<Result<ModulesState, LookError>> {
        let mut state = self.0.borrow_mut();
        state.modules = state.modules.with_layout(layout);
        state.modules.revision += 1;
        ready(Ok(state.modules.clone()))
    }

    fn module_cache(&self, _camera: &str, _language: &str) -> Option<ModuleCache> {
        None
    }

    fn module_draft(&self, camera: &str) -> ModuleDraft {
        self.0
            .borrow()
            .drafts
            .get(camera)
            .cloned()
            .unwrap_or_default()
    }

    fn save_module_draft(&self, camera: &str, draft: &ModuleDraft) {
        self.0
            .borrow_mut()
            .drafts
            .insert(camera.to_owned(), draft.clone());
    }

    fn camera_settings(&self) -> Fut<Result<Option<CameraSettings>, LookError>> {
        ready(Ok(Some(self.0.borrow().settings.clone())))
    }

    fn change_camera_setting(
        &self,
        _boot_id: &str,
        change: SettingChange,
    ) -> Fut<Result<CameraSettings, LookError>> {
        let mut state = self.0.borrow_mut();
        state.settings = state.settings.with(change);
        ready(Ok(state.settings.clone()))
    }

    fn shutdown_picture_writable(&self) -> Fut<Result<bool, LookError>> {
        ready(Ok(true))
    }

    fn shutdown_picture(&self) -> Fut<Result<Option<Vec<u8>>, LookError>> {
        ready(Ok(self.0.borrow().picture.clone()))
    }

    fn put_shutdown_picture(
        &self,
        _boot_id: &str,
        jpeg: Option<Vec<u8>>,
    ) -> Fut<Result<(), LookError>> {
        self.0.borrow_mut().picture = jpeg;
        ready(Ok(()))
    }

    fn latest_release(
        &self,
        _model: CameraModel,
        _firmware: &str,
    ) -> Fut<Result<ReleaseOffer, ReleaseProblem>> {
        let config = self.0.borrow().config.clone();
        ready(Ok(ReleaseOffer {
            version: config.available.clone(),
            notes: demo_notes(),
            presets: IndexPreset::all(&Default::default()),
            bytes: config.package_bytes,
            offline: false,
            modules: Vec::new(),
            package: Rc::new(FakeRelease {
                version: config.available,
            }),
        }))
    }

    fn update_snapshot(&self) -> Fut<Result<UpdateSnapshot, UpdateError>> {
        let mut state = self.0.borrow_mut();
        if state.journal == JournalState::Waiting {
            state.reads += 1;
            if state.reads >= CONFIRM_READS {
                state.set_journal(JournalState::Confirmed);
            }
        }
        ready(Ok(UpdateSnapshot {
            state: state.journal,
            package: state.package(),
            request: None,
        }))
    }

    fn update_package(&self) -> Fut<Result<PackageResponse, UpdateError>> {
        let state = self.0.borrow();
        let mut package = state.package();
        package.camera = Some(PackageCamera {
            model: Some(state.config.model.display_name().to_owned()),
            firmware: Some(state.config.firmware.clone()),
        });
        ready(Ok(package))
    }

    fn boot_id(&self) -> Fut<Result<String, UpdateError>> {
        ready(Ok(BOOT_ID.into()))
    }

    fn free_bytes(&self) -> Fut<Option<u64>> {
        ready(None)
    }

    fn stage_update(&self, body: Vec<u8>, _boot_id: &str) -> Fut<Result<(), UpdateError>> {
        let state = self.0.clone();
        Box::pin(async move {
            timer(SEND_TIME).await;
            let mut state = state.borrow_mut();
            match state.config.fail {
                DemoFail::CardFull => Err(UpdateError::CardFull {
                    bytes_needed: state.config.card_full_bytes,
                }),
                DemoFail::Refused => Err(UpdateError::Refused),
                DemoFail::ConnectionLost => Err(UpdateError::ConnectionLost),
                DemoFail::None => {
                    let staged =
                        serde_json::from_slice(&body).map_err(|_| UpdateError::PackageInvalid)?;
                    state.staged = Some(staged);
                    state.reads = 0;
                    state.set_journal(JournalState::Waiting);
                    Ok(())
                }
            }
        })
    }

    fn cancel_update(&self, _boot_id: &str) -> Fut<Result<(), UpdateError>> {
        let mut state = self.0.borrow_mut();
        state.staged = None;
        state.set_journal(JournalState::None);
        ready(Ok(()))
    }

    fn intent(&self) -> Option<UpdateIntent> {
        self.0.borrow().intent.clone()
    }

    fn save_intent(&self, intent: &UpdateIntent) -> Result<(), UpdateError> {
        self.0.borrow_mut().intent = Some(intent.clone());
        Ok(())
    }

    fn clear_intent(&self) {
        self.0.borrow_mut().intent = None;
    }

    fn seen_version(&self, camera: &str) -> Option<String> {
        self.0.borrow().seen.get(camera).cloned()
    }

    fn save_seen_version(&self, camera: &str, version: &str) {
        self.0
            .borrow_mut()
            .seen
            .insert(camera.to_owned(), version.to_owned());
    }

    fn saved_cameras(&self) -> Vec<RememberedCamera> {
        self.0.borrow().saved.clone()
    }

    fn save_cameras(&self, cameras: &[RememberedCamera]) -> Result<(), String> {
        self.0.borrow_mut().saved = cameras.to_vec();
        Ok(())
    }
}

fn catalog_grf(state: &State, look: &CatalogLook) -> Result<Vec<u8>, String> {
    state
        .catalog
        .iter()
        .find(|(l, _)| l.id == look.id)
        .map(|(_, grf)| grf.clone())
        .ok_or_else(|| format!("{} is not in the demo catalog", look.name))
}

/// The GRKit settings of the demo camera.
fn demo_settings() -> CameraSettings {
    parse_camera_settings(
        br#"{"settings":{"start":"last","distanceUnit":"meters","uncroppedRaw":false,
            "simpleBattery":true,"batteryPercent":true,"playbackIcons":true,
            "customPicture":true,"showText":true}}"#,
    )
    .expect("demo settings parse")
}

/// Release notes for the demo: the features of the version on offer, and
/// the real notes of three releases.
pub fn demo_notes() -> Vec<VersionNotes> {
    [
        (
            "1.7.0",
            &[
                "The GRKit app updates GRKit over the camera's Wi‑Fi: the camera checks the signature of the update, you confirm it in ADJ > MOD, and it installs at the next start.",
                "The GRKit app lists the film looks on the camera, turns them on or off, orders them, and changes their names and covers.",
                "The camera runs the GRKit Wi‑Fi service only while its Wi‑Fi is on.",
                "Without a connected phone, the Wi‑Fi service uses no processor time.",
            ][..],
        ),
        (
            "1.6.4",
            &[
                "In ADJ, OK on a GRKit value returns to the GRKit list instead of closing ADJ, and Left or Right keeps the highlighted value.",
                "Battery in Simple Display shows the battery icon in the simple display, and Display Settings in GRKit Settings now holds it and Distance Unit.",
                "With the fast-boot firmware, GRKit loads faster from exFAT cards that a Mac copied files to, and a camera whose fast start fails loads GRKit the normal way from the next power-on.",
                "Film looks can show the colors of their film box, for color looks only or for all, and each film look can have its own name and cover.",
                "GRKit uses less processor time while the camera is on and no setting changes, and Grain takes less time for each photo on GR III and GR IIIx.",
                "Photos record their GRKit settings in XMP, so exiftool and other XMP readers show them.",
                "RAW development keeps the Framing ratio of a photo that has a Border.",
                "The RAW development preview keeps the Border color with Shading or Clarity, and puts the Shading vignette in the middle of the picture.",
                "RAW development starts from the GRKit settings that the DNG records, and has a GRKit row that sets Grain, Glow, Border, and Date for that development only.",
                "GRKit Settings has Uncropped RAW, which keeps the whole sensor picture in each DNG, and Lightroom and Camera Raw open it with the crop of the shot, which they can undo.",
            ][..],
        ),
        (
            "1.6.3",
            &[
                "Color covers shows the packaging colors of each film look on the shelf and writes them onto the card.",
                "Import a Lightroom .xmp preset as a film look, and optionally add a recipe with its exposure, grain, and white balance.",
            ][..],
        ),
        (
            "1.6.2",
            &[
                "With a border, the review after a cropped shot shows the border at once, and the picture stays in place when playback opens.",
                "The crop badge in the live view looks the same for each focal length.",
                "Order in the MENU page of Film Look, Recipes, Framing, Guides, and Date Imprint sets the order of their choices in ADJ.",
            ][..],
        ),
    ]
    .into_iter()
    .map(|(version, items)| VersionNotes {
        version: version.into(),
        items: items.iter().map(|i| (*i).to_owned()).collect(),
    })
    .collect()
}
