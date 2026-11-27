//! What the demo screens set up, on the app and on the demo camera.

use nsh_core::prelude::*;

use super::PORTRA;
use crate::dev::fake::{FakeCamera, FakeConfig, ModuleSupport};
use crate::dev::fake_looks::DEMO_CUSTOM_ID;
use crate::domain::images::jpeg_data_uri;
use crate::domain::shutdown_picture::Photo;
use crate::domain::{
    ConnectionState, JournalState, ReleaseProblem, UpdateError, UpdateKind, UpdatePhase,
};
use crate::store::{Alert, AppState, ModulesBlock, Route, Sheet, Tab, off_thread};

/// What the screens set up, on the app and on the demo camera.
#[derive(Clone, Copy)]
pub(super) struct Seed<'a> {
    pub(super) app: AppState,
    pub(super) demo: &'a FakeCamera,
}

impl Seed<'_> {
    /// No camera connected.
    pub(super) fn disconnected(self) {
        self.app.seed_status(None);
        self.app.seed_package(None);
    }

    /// Two saved cameras, the GR III the most recent.
    pub(super) fn saved(self) {
        use crate::domain::{CameraModel, RememberedCamera};
        self.app.remember_camera(RememberedCamera::new(
            "DEMO-GR4",
            CameraModel::Gr4,
            "GR_7F2E10",
        ));
        self.app.remember_camera(RememberedCamera::new(
            "DEMO-GR3",
            CameraModel::Gr3,
            "GR_A1B2C3",
        ));
    }

    /// A connected camera whose status alone is read.
    pub(super) fn connected_status(self) {
        let (status, _) = self.demo.session();
        self.app.seed_selected("demo");
        self.app.seed_status(Some(status));
    }

    /// The package of the camera, before the release check answered.
    pub(super) fn package_without_release(self) {
        let (_, mut package) = self.demo.session();
        package.available = None;
        self.app.seed_package(Some(package));
    }

    pub(super) fn connected_available(self) {
        let (status, package) = self.demo.session();
        self.app.seed_selected("demo");
        self.app.seed_status(Some(status));
        self.app.seed_package(Some(package));
        // As a connect: the home shows the Look in use, the current recipe
        // and the modules.
        self.app.refresh_modules();
        self.app.refresh_looks();
        self.app.refresh_recipes();
    }

    pub(super) fn connected_up_to_date(self) {
        self.connected_up_to_date_at("1.7.0");
    }

    pub(super) fn connected_up_to_date_at(self, version: &str) {
        self.demo.configure(FakeConfig {
            installed: version.into(),
            available: version.into(),
            ..FakeConfig::default()
        });
        self.connected_available();
    }

    /// Connected, on a page of GRKit Settings.
    pub(super) fn grkit_settings(self, page: Route) {
        self.connected_up_to_date();
        self.app.set_sheet(None);
        self.app.set_tab(Tab::Settings);
        self.app.refresh_modules();
        self.app.push(page);
    }

    /// Connected, on the Modules page, with the demo camera's modules read.
    pub(super) fn modules(self) {
        self.connected_up_to_date();
        self.app.set_sheet(None);
        self.app.refresh_looks();
        self.app.refresh_recipes();
        self.app.open_modules_page();
    }

    /// The Modules page of a GRKit that the app cannot show modules of.
    pub(super) fn modules_blocked(self, block: ModulesBlock) {
        self.demo.set_module_support(match block {
            ModulesBlock::Unsupported => ModuleSupport::None,
            ModulesBlock::AppTooOld => ModuleSupport::TooNew,
        });
        self.connected_up_to_date();
        self.app.set_sheet(None);
        self.app.open_modules_page();
    }

    /// Connected, on the Looks tab, with the demo camera's Looks read.
    pub(super) fn looks(self) {
        self.connected_up_to_date();
        self.app.set_sheet(None);
        self.app.set_tab(Tab::Looks);
        self.app.refresh_looks();
    }

    /// Connected, on the Recipes tab, with the demo camera's recipes and
    /// Looks read.
    pub(super) fn recipes(self) {
        self.connected_up_to_date();
        self.app.set_sheet(None);
        self.app.refresh_looks();
        self.app.set_tab(Tab::Recipes);
    }

    /// Recipe `recipe` uses the Film Look `look` (id, name) instead of its
    /// own.
    pub(super) fn recipe_on(self, recipe: u32, (id, name): (&str, &str)) {
        self.demo.edit_recipes(|f| {
            if let Some(item) = f.recipes.iter_mut().find(|r| r.id == recipe) {
                item.effect_id = Some(id.into());
                item.effect_name = Some(name.into());
            }
        });
    }

    /// The Looks tab of a camera that a restart for changed Looks took down.
    pub(super) fn look_restart(self) {
        self.saved();
        self.looks();
        self.app.seed_look_restart(true, false);
        self.app.reset_looks();
        if let Some(mut status) = self.app.status() {
            status.camera.connection = ConnectionState::Disconnected;
            self.app.seed_status(Some(status));
        }
    }

    /// An import ready to send, made from a bundled Look.
    pub(super) fn import_draft(self) {
        if let Some(grf) = self.demo.look_content(DEMO_CUSTOM_ID) {
            self.app.seed_import_draft(grf);
        }
    }

    /// Ask to delete Portra 400 after the page shows.
    pub(super) fn ask_delete(self) {
        let app = self.app;
        spawn(async move {
            next_frame().await;
            next_frame().await;
            if let Some(look) = app.look(PORTRA) {
                crate::pages::confirm_delete(app, look);
            }
        });
    }

    /// Shutdown Screen, with the sample photo picked: `then` gets the photo
    /// and its preview once the phone has read it.
    pub(super) fn shutdown_photo(self, then: impl FnOnce(Seed<'_>, Photo, String) + 'static) {
        self.grkit_settings(Route::ShutdownScreen);
        let demo = self.demo.clone();
        spawn(async move {
            let Some((photo, preview)) = off_thread(sample_photo).await.flatten() else {
                return;
            };
            if let Some(uri) = jpeg_data_uri(&preview) {
                then(
                    Seed {
                        app: self.app,
                        demo: &demo,
                    },
                    photo,
                    uri,
                );
            }
        });
    }

    /// Show `alert` once the page is up.
    pub(super) fn alert_after_frames(self, alert: Alert) {
        let app = self.app;
        spawn(async move {
            next_frame().await;
            next_frame().await;
            app.alert(alert);
        });
    }

    /// An update waits on the demo camera in `journal`, as the camera tab
    /// read it.
    pub(super) fn camera_journal(self, journal: JournalState) {
        self.demo.set_journal(journal);
        self.app.seed_journal(Some(journal));
    }

    /// Software Update with no release: under way, or with `problem`.
    pub(super) fn software_with(self, problem: Option<ReleaseProblem>, checking: bool) {
        self.connected_available();
        self.package_without_release();
        self.app.seed_release_check(checking, problem);
        self.app.set_tab(Tab::Camera);
        self.app.push(Route::SoftwareUpdate);
    }

    /// The update sheet stopped by `error`.
    pub(super) fn update_error(self, error: UpdateError) {
        self.connected_available();
        self.app
            .seed_update(UpdatePhase::Failed(error.clone()), None);
        self.app.set_tab(Tab::Camera);
        self.app.push(Route::SoftwareUpdate);
        self.app.set_sheet(Some(Sheet::Error(error)));
    }

    /// The send at `progress`.
    pub(super) fn sending(self, progress: f32) {
        self.app.seed_update(
            UpdatePhase::Sending {
                progress,
                kind: UpdateKind::Install,
            },
            Some(&self.demo.config().installed),
        );
    }

    /// The camera waits for its confirmation.
    pub(super) fn waiting(self) {
        self.app.seed_update(
            UpdatePhase::WaitingConfirm {
                kind: UpdateKind::Install,
            },
            Some(&self.demo.config().installed),
        );
        self.app.seed_journal(Some(JournalState::Waiting));
    }

    /// The camera confirmed: the restart step.
    pub(super) fn restart(self) {
        self.app.seed_update(
            UpdatePhase::Restart {
                kind: UpdateKind::Install,
            },
            Some(&self.demo.config().installed),
        );
        self.app.seed_journal(Some(JournalState::Confirmed));
    }
}

/// The sample photo as the camera's Standard renders it, read as a picked
/// photo, and its preview.
fn sample_photo() -> Option<(Photo, Vec<u8>)> {
    let base: [u16; 256] = crate::domain::look_media::demo_base_gamma()
        .try_into()
        .ok()?;
    let table = grkit_core::site::standard_linear_preview_lut(&base);
    let png = crate::domain::look_preview::sample_preview_png(&table).ok()?;
    let photo = Photo::decode(&png).ok()?;
    let preview = photo.preview();
    Some((photo, preview))
}
