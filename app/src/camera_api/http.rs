//! [`CameraApi`] over the camera's HTTP service and the release site, with
//! the phone's records in its own storage. Each method hands its request to
//! its client module in `camera_api`.

use std::rc::Rc;

use serde_json::json;

use super::{CameraApi, CatalogProblem, Fut, ReleaseOffer, ReleasePackage};
use super::{
    looks::{self, CameraContext},
    modules, recipes, release, service, update,
};
use crate::camera_api::{DownloadError, ModuleAccess, ModuleCache};
use crate::domain::{
    CameraModel, CameraStatus, Catalog, CatalogLook, LinkError, LookError, LooksSnapshot,
    PackageInfo, PackageResponse, ReleaseProblem, RememberedCamera, UpdateError, UpdateIntent,
    UpdateSnapshot,
    camera_settings::{CameraSettings, SettingChange},
    looks::LookChange,
    modules::{
        CatalogAnswer, ModuleCatalog, ModuleDraft, ModuleLayout, ModulePatch, ModuleState,
        ModulesState,
    },
    recipes::{RecipeChange, RecipeError, RecipesSnapshot},
};

/// A real camera, reached at `camera_http_base`.
#[derive(Clone, Copy, Debug, Default)]
pub struct HttpCamera;

/// A release that the site signed, as `release` read it.
struct SiteRelease(Rc<release::Release>);

impl ReleasePackage for SiteRelease {
    fn download(
        &self,
        before: &PackageResponse,
        progress: Box<dyn Fn(f32)>,
    ) -> Fut<Result<Vec<u8>, DownloadError>> {
        let release = self.0.clone();
        let before = before.clone();
        Box::pin(async move { release::download(&release, &before, progress).await })
    }
}

impl CameraApi for HttpCamera {
    fn read_package(
        &self,
        fallback: CameraModel,
    ) -> Fut<Result<(CameraStatus, PackageInfo), LinkError>> {
        Box::pin(service::read_package(fallback))
    }

    fn finish(&self, power_off: bool) -> Fut<bool> {
        Box::pin(service::camera_finish(power_off))
    }

    fn looks(&self) -> Fut<Result<LooksSnapshot, LookError>> {
        Box::pin(looks::list())
    }

    fn look_content(&self, id: &str, content_hash: &str) -> Fut<Result<Vec<u8>, LookError>> {
        let (id, hash) = (id.to_owned(), content_hash.to_owned());
        Box::pin(async move { looks::content(&id, &hash).await })
    }

    fn change_look(&self, boot_id: &str, change: LookChange) -> Fut<Result<(), LookError>> {
        let context = CameraContext {
            boot_id: boot_id.to_owned(),
        };
        Box::pin(async move {
            match change {
                LookChange::Enabled { id, enabled } => {
                    looks::set_enabled(&context, &id, enabled).await
                }
                LookChange::Rename { id, name } => looks::rename(&context, &id, &name).await,
                LookChange::Delete { id, active_off } => {
                    looks::delete(&context, &id, active_off).await
                }
                LookChange::Place {
                    id,
                    to_card,
                    active_off,
                } => looks::set_place(&context, &id, to_card, active_off).await,
                LookChange::Order(ids) => looks::set_order(&context, &ids).await,
                LookChange::Content { id, grf, cover } => {
                    looks::put_content(&context, &id, grf, cover.as_ref()).await
                }
            }
        })
    }

    fn look_catalog(
        &self,
        model: CameraModel,
        firmware: &str,
    ) -> Fut<Result<(String, Catalog), CatalogProblem>> {
        let target = looks::target(model, firmware);
        Box::pin(async move {
            let target = target.ok_or(CatalogProblem::Unsupported)?;
            looks::catalog(target)
                .await
                .map(|catalog| (target.to_owned(), catalog))
                .map_err(CatalogProblem::Offline)
        })
    }

    fn catalog_file(&self, target: &str, look: &CatalogLook) -> Fut<Result<Vec<u8>, String>> {
        let (target, look) = (target.to_owned(), look.clone());
        Box::pin(async move { looks::catalog_file(&target, &look).await })
    }

    fn preview_table(&self, target: &str, look: &CatalogLook) -> Fut<Result<Vec<u8>, String>> {
        let (target, look) = (target.to_owned(), look.clone());
        Box::pin(async move { looks::preview_table(&target, &look).await })
    }

    fn standard_table(&self, target: &str) -> Fut<Result<Vec<u8>, String>> {
        let target = target.to_owned();
        Box::pin(async move { looks::standard_table(&target).await })
    }

    fn recipes(&self) -> Fut<Result<RecipesSnapshot, RecipeError>> {
        Box::pin(recipes::list())
    }

    fn change_recipe(
        &self,
        boot_id: &str,
        change: RecipeChange,
    ) -> Fut<Result<RecipesSnapshot, RecipeError>> {
        let boot_id = boot_id.to_owned();
        let (method, path, body) = match change {
            RecipeChange::Enabled(id, on) => {
                ("PATCH", format!("/{id}"), Some(json!({"enabled": on})))
            }
            RecipeChange::Rename(id, name) => {
                ("PATCH", format!("/{id}"), Some(json!({"name": name})))
            }
            RecipeChange::Delete(id) => ("DELETE", format!("/{id}"), None),
            RecipeChange::Order(ids) => ("PUT", "/order".to_owned(), Some(json!({"ids": ids}))),
            RecipeChange::Apply(id) => ("POST", format!("/{id}/apply"), None),
            RecipeChange::Undo => ("POST", "/undo".to_owned(), None),
            RecipeChange::Add(preset) => ("POST", String::new(), Some(json!({"preset": preset}))),
        };
        Box::pin(async move { recipes::change(&boot_id, method, &path, body).await })
    }

    fn module_access(&self) -> Fut<Result<ModuleAccess, LookError>> {
        Box::pin(modules::access())
    }

    fn module_catalog(
        &self,
        language: &str,
        cached: Option<ModuleCatalog>,
    ) -> Fut<Result<CatalogAnswer, LookError>> {
        let language = language.to_owned();
        Box::pin(async move { modules::catalog(&language, cached).await })
    }

    fn module_state(&self) -> Fut<Result<ModulesState, LookError>> {
        Box::pin(modules::state())
    }

    fn module_icon(&self, id: &str) -> Fut<Result<Option<Vec<u8>>, LookError>> {
        let id = id.to_owned();
        Box::pin(async move { modules::icon(&id).await })
    }

    fn patch_module(
        &self,
        boot_id: &str,
        id: &str,
        patch: ModulePatch,
    ) -> Fut<Result<(ModuleState, u64), LookError>> {
        let (boot_id, id) = (boot_id.to_owned(), id.to_owned());
        let body = serde_json::to_value(patch).expect("JSON encodes");
        Box::pin(async move { modules::patch(&boot_id, &id, body).await })
    }

    fn put_module_order(
        &self,
        boot_id: &str,
        ids: &[String],
    ) -> Fut<Result<ModulesState, LookError>> {
        let (boot_id, ids) = (boot_id.to_owned(), ids.to_vec());
        Box::pin(async move { modules::put_order(&boot_id, &ids).await })
    }

    fn put_module_layout(
        &self,
        boot_id: &str,
        layout: &ModuleLayout,
    ) -> Fut<Result<ModulesState, LookError>> {
        let (boot_id, layout) = (boot_id.to_owned(), layout.clone());
        Box::pin(async move { modules::put_layout(&boot_id, &layout).await })
    }

    fn module_cache(&self, camera: &str, language: &str) -> Option<ModuleCache> {
        ModuleCache::open(camera, language)
    }

    fn module_draft(&self, camera: &str) -> ModuleDraft {
        modules::load_draft(camera)
    }

    fn save_module_draft(&self, camera: &str, draft: &ModuleDraft) {
        modules::save_draft(camera, draft);
    }

    fn camera_settings(&self) -> Fut<Result<Option<CameraSettings>, LookError>> {
        Box::pin(modules::camera_settings())
    }

    fn change_camera_setting(
        &self,
        boot_id: &str,
        change: SettingChange,
    ) -> Fut<Result<CameraSettings, LookError>> {
        let boot_id = boot_id.to_owned();
        Box::pin(async move { modules::patch_camera_setting(&boot_id, change).await })
    }

    fn shutdown_picture_writable(&self) -> Fut<Result<bool, LookError>> {
        Box::pin(modules::shutdown_picture_writable())
    }

    fn shutdown_picture(&self) -> Fut<Result<Option<Vec<u8>>, LookError>> {
        Box::pin(modules::shutdown_picture())
    }

    fn put_shutdown_picture(
        &self,
        boot_id: &str,
        jpeg: Option<Vec<u8>>,
    ) -> Fut<Result<(), LookError>> {
        let boot_id = boot_id.to_owned();
        Box::pin(async move { modules::put_shutdown_picture(&boot_id, jpeg).await })
    }

    fn latest_release(
        &self,
        model: CameraModel,
        firmware: &str,
    ) -> Fut<Result<ReleaseOffer, ReleaseProblem>> {
        let firmware = firmware.to_owned();
        Box::pin(async move {
            let release = release::latest(model, &firmware).await?;
            Ok(ReleaseOffer {
                version: release.version.clone(),
                notes: release.notes.clone(),
                presets: release.presets.clone(),
                bytes: release.bytes,
                offline: release.offline,
                modules: release.official_modules(),
                package: Rc::new(SiteRelease(Rc::new(release))),
            })
        })
    }

    fn update_snapshot(&self) -> Fut<Result<UpdateSnapshot, UpdateError>> {
        Box::pin(update::snapshot())
    }

    fn update_package(&self) -> Fut<Result<PackageResponse, UpdateError>> {
        Box::pin(update::package())
    }

    fn boot_id(&self) -> Fut<Result<String, UpdateError>> {
        Box::pin(update::boot_id())
    }

    fn free_bytes(&self) -> Fut<Option<u64>> {
        Box::pin(update::free_bytes())
    }

    fn stage_update(&self, body: Vec<u8>, boot_id: &str) -> Fut<Result<(), UpdateError>> {
        let boot_id = boot_id.to_owned();
        Box::pin(async move { update::stage(body, &boot_id).await })
    }

    fn cancel_update(&self, boot_id: &str) -> Fut<Result<(), UpdateError>> {
        let boot_id = boot_id.to_owned();
        Box::pin(async move { update::cancel(&boot_id).await })
    }

    fn intent(&self) -> Option<UpdateIntent> {
        update::load_intent()
    }

    fn save_intent(&self, intent: &UpdateIntent) -> Result<(), UpdateError> {
        update::save_intent(intent)
    }

    fn clear_intent(&self) {
        update::clear_intent();
    }

    fn seen_version(&self, camera: &str) -> Option<String> {
        update::seen_version(camera)
    }

    fn save_seen_version(&self, camera: &str, version: &str) {
        update::save_seen_version(camera, version);
    }

    fn saved_cameras(&self) -> Vec<RememberedCamera> {
        crate::platform::load_saved()
    }

    fn save_cameras(&self, cameras: &[RememberedCamera]) -> Result<(), String> {
        crate::platform::save_saved(cameras)
    }
}
