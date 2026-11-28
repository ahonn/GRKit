//! What the store reads and writes outside its own memory: the camera's
//! service, the release site, and what the phone keeps about cameras.
//!
//! [`CameraApi`] is the one way in. [`HttpCamera`] talks to a real camera
//! over Wi-Fi and keeps its records on the phone; the demo camera
//! (`dev::FakeCamera`) answers the same requests from memory, so the store
//! has one flow for both. The Bluetooth and Wi-Fi link to a camera is not
//! part of it: the store drives that through `platform`.
//!
//! The modules under it are the HTTP clients that [`HttpCamera`] hands each
//! request to: the camera service and the release site, over the phone's
//! transport in `platform`.

pub(crate) mod changes;
mod http;
mod looks;
pub(crate) mod modules;
mod recipes;
mod release;
pub(crate) mod service;
mod transport;
mod update;

use std::{future::Future, pin::Pin, rc::Rc};

use grkit_core::release_index::{IndexPreset, ModuleInfo, VersionNotes};

pub use http::HttpCamera;
pub use modules::{ModuleAccess, ModuleCache};
pub use release::DownloadError;
pub(crate) use transport::{camera_http_base, camera_request, has_dev_camera_url};

use crate::domain::{
    CameraModel, CameraStatus, Catalog, CatalogLook, LinkError, LookError, LooksSnapshot,
    PackageInfo, PackageResponse, ReleaseProblem, RememberedCamera, UpdateError, UpdateIntent,
    UpdateSnapshot,
    camera_settings::{CameraSettings, SettingChange},
    looks::LookChange,
    modules::{CatalogAnswer, ModuleCatalog, ModuleLayout, ModulePatch, ModuleState, ModulesState},
    recipes::{RecipeChange, RecipeError, RecipesSnapshot},
};

/// An answer that comes later. It owns what it needs, so a task can await
/// it after the call that made it returns.
pub type Fut<T> = Pin<Box<dyn Future<Output = T>>>;

/// Why the official Look catalog is not there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogProblem {
    /// The site has no catalog for this camera.
    Unsupported,
    /// The site did not answer, and the phone keeps no copy.
    Offline(String),
}

/// The GRKit release that the site offers for a camera.
#[derive(Clone)]
pub struct ReleaseOffer {
    pub version: String,
    /// The notes of recent releases: display only.
    pub notes: Vec<VersionNotes>,
    /// The recipe presets of the site: display and settings only.
    pub presets: Vec<IndexPreset>,
    /// Upper estimate of the release files, before camera selections.
    pub bytes: u64,
    /// Read from the cache because the site did not answer: an update it
    /// offers is real, but it cannot say that none is newer.
    pub offline: bool,
    /// The official modules that this release signs, by ID.
    pub modules: Vec<(String, ModuleInfo)>,
    /// Assembles the update package of this release.
    pub package: Rc<dyn ReleasePackage>,
}

/// The update package of one release.
pub trait ReleasePackage {
    /// Download and assemble the package that keeps the modules that
    /// `before` names. `progress` hears 0..=1.
    fn download(
        &self,
        before: &PackageResponse,
        progress: Box<dyn Fn(f32)>,
    ) -> Fut<Result<Vec<u8>, DownloadError>>;
}

/// The camera, the release site and the phone's records, as the store uses
/// them. Each method mirrors one request of the camera service or the site;
/// a change names the camera start (`boot_id`) that the phone read.
pub trait CameraApi {
    /// The model of a camera that the app reaches without Bluetooth or
    /// Wi-Fi (the demo camera): the store steps through a connect and a
    /// restart of it on a timer. `None` for a real camera.
    fn simulated(&self) -> Option<CameraModel> {
        None
    }

    // The session.

    /// The camera and its GRKit, as a connect reads them.
    fn read_package(
        &self,
        fallback: CameraModel,
    ) -> Fut<Result<(CameraStatus, PackageInfo), LinkError>>;
    /// Close the camera's access point, or turn the camera off. Whether
    /// the camera took it.
    fn finish(&self, power_off: bool) -> Fut<bool>;

    // Film Looks.

    fn looks(&self) -> Fut<Result<LooksSnapshot, LookError>>;
    /// The GRF of a camera Look whose list entry had `content_hash`.
    fn look_content(&self, id: &str, content_hash: &str) -> Fut<Result<Vec<u8>, LookError>>;
    fn change_look(&self, boot_id: &str, change: LookChange) -> Fut<Result<(), LookError>>;
    /// The catalog target of the camera firmware, and its official Looks.
    fn look_catalog(
        &self,
        model: CameraModel,
        firmware: &str,
    ) -> Fut<Result<(String, Catalog), CatalogProblem>>;
    fn catalog_file(&self, target: &str, look: &CatalogLook) -> Fut<Result<Vec<u8>, String>>;
    /// The preview table of an official Look.
    fn preview_table(&self, target: &str, look: &CatalogLook) -> Fut<Result<Vec<u8>, String>>;
    /// The preview table of the camera's own Standard.
    fn standard_table(&self, target: &str) -> Fut<Result<Vec<u8>, String>>;

    // Recipes.

    fn recipes(&self) -> Fut<Result<RecipesSnapshot, RecipeError>>;
    fn change_recipe(
        &self,
        boot_id: &str,
        change: RecipeChange,
    ) -> Fut<Result<RecipesSnapshot, RecipeError>>;

    // Modules.

    fn module_access(&self) -> Fut<Result<ModuleAccess, LookError>>;
    /// The catalog, naming the cached one so that an unchanged catalog
    /// costs no transfer.
    fn module_catalog(
        &self,
        language: &str,
        cached: Option<ModuleCatalog>,
    ) -> Fut<Result<CatalogAnswer, LookError>>;
    fn module_state(&self) -> Fut<Result<ModulesState, LookError>>;
    /// The icon tile of a module; `None` when it has none.
    fn module_icon(&self, id: &str) -> Fut<Result<Option<Vec<u8>>, LookError>>;
    /// Change one module: the answer is the module and the new revision.
    fn patch_module(
        &self,
        boot_id: &str,
        id: &str,
        patch: ModulePatch,
    ) -> Fut<Result<(ModuleState, u64), LookError>>;
    fn put_module_order(
        &self,
        boot_id: &str,
        ids: &[String],
    ) -> Fut<Result<ModulesState, LookError>>;
    fn put_module_layout(
        &self,
        boot_id: &str,
        layout: &ModuleLayout,
    ) -> Fut<Result<ModulesState, LookError>>;
    /// The phone's copy of the module catalog, icons and state of one
    /// camera in one language.
    fn module_cache(&self, camera: &str, language: &str) -> Option<ModuleCache>;
    /// The module changes that wait for the next update of a camera.
    fn module_draft(&self, camera: &str) -> crate::domain::modules::ModuleDraft;
    fn save_module_draft(&self, camera: &str, draft: &crate::domain::modules::ModuleDraft);

    // GRKit settings.

    /// `None` for a GRKit from before the phone could change them.
    fn camera_settings(&self) -> Fut<Result<Option<CameraSettings>, LookError>>;
    fn change_camera_setting(
        &self,
        boot_id: &str,
        change: SettingChange,
    ) -> Fut<Result<CameraSettings, LookError>>;
    fn shutdown_picture_writable(&self) -> Fut<Result<bool, LookError>>;
    fn shutdown_picture(&self) -> Fut<Result<Option<Vec<u8>>, LookError>>;
    /// Put `jpeg` on the card, or remove the picture (`None`).
    fn put_shutdown_picture(
        &self,
        boot_id: &str,
        jpeg: Option<Vec<u8>>,
    ) -> Fut<Result<(), LookError>>;

    // Software.

    fn latest_release(
        &self,
        model: CameraModel,
        firmware: &str,
    ) -> Fut<Result<ReleaseOffer, ReleaseProblem>>;
    /// The camera's update journal and package.
    fn update_snapshot(&self) -> Fut<Result<UpdateSnapshot, UpdateError>>;
    /// The package, with the camera that the props name.
    fn update_package(&self) -> Fut<Result<PackageResponse, UpdateError>>;
    /// The camera start that the phone reads now.
    fn boot_id(&self) -> Fut<Result<String, UpdateError>>;
    /// Free bytes on the card, when the camera reports them.
    fn free_bytes(&self) -> Fut<Option<u64>>;
    /// Stage the update package `body` on the card.
    fn stage_update(&self, body: Vec<u8>, boot_id: &str) -> Fut<Result<(), UpdateError>>;
    /// Cancel the update that waits on the card.
    fn cancel_update(&self, boot_id: &str) -> Fut<Result<(), UpdateError>>;

    // What the phone keeps.

    /// The update this phone sent and has not seen end.
    fn intent(&self) -> Option<UpdateIntent>;
    fn save_intent(&self, intent: &UpdateIntent) -> Result<(), UpdateError>;
    fn clear_intent(&self);
    /// The GRKit version this phone last saw on a camera.
    fn seen_version(&self, camera: &str) -> Option<String>;
    fn save_seen_version(&self, camera: &str, version: &str);
    /// The saved cameras, the most recent first.
    fn saved_cameras(&self) -> Vec<RememberedCamera>;
    fn save_cameras(&self, cameras: &[RememberedCamera]) -> Result<(), String>;
}
