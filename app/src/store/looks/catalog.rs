//! The official Look catalog of the camera's firmware, and the Looks added
//! from it.

use grkit_core::cover::{Cover, Variant};
use nsh_core::prelude::*;

use super::{LookAction, Source};
use crate::camera_api::CatalogProblem;
use crate::domain::{CatalogLook, LookError, LookGroup, group_of, look_media, looks::LookChange};
use crate::store::{AppState, Route};
use crate::strings::tr;

/// An official Look of the catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogItem {
    pub look: CatalogLook,
    pub group: LookGroup,
}

impl AppState {
    /// Open the page of the official Looks that the camera does not have.
    pub fn open_add(self) {
        self.load_catalog();
        self.push(Route::LookAdd);
    }

    pub fn catalog_items(self) -> Option<Vec<CatalogItem>> {
        self.looks.catalog.get()
    }

    pub fn catalog_error(self) -> Option<String> {
        self.looks.catalog_error.get()
    }

    pub fn catalog_adding(self, id: &str) -> bool {
        self.looks.adding.get().contains(id)
    }

    /// Load the official catalog for the camera firmware.
    pub(super) async fn ensure_catalog(self) {
        if self.looks.catalog.get().is_some() {
            return;
        }
        let Some(status) = self.status.get() else {
            return;
        };
        let catalog = self
            .api()
            .look_catalog(status.camera.model, &status.camera.firmware)
            .await;
        match catalog {
            Err(CatalogProblem::Unsupported) => {
                self.looks
                    .catalog_error
                    .set(Some(tr("looks.catalog_unsupported", &[])));
            }
            Ok((target, catalog)) => {
                self.looks
                    .base_gamma
                    .with(|b| *b.borrow_mut() = Some(catalog.base_gamma.clone()));
                self.looks.catalog_target.set(Some(target));
                self.looks.catalog_error.set(None);
                self.looks.catalog.set(Some(
                    catalog
                        .looks
                        .into_iter()
                        .map(|look| CatalogItem {
                            group: group_of(&look.id, look.monochrome),
                            look,
                        })
                        .collect(),
                ));
            }
            Err(CatalogProblem::Offline(error)) => {
                log::warn!("Could not load the Look catalog: {error}");
                self.looks
                    .catalog_error
                    .set(Some(tr("looks.catalog_offline", &[])));
            }
        }
    }

    /// Show the All Looks list: load the catalog.
    pub fn load_catalog(self) {
        if self.looks.catalog.get().is_some() {
            return;
        }
        self.root.run(|| {
            spawn(async move {
                self.ensure_catalog().await;
            });
        });
    }

    /// Add an official Look from the catalog.
    pub fn add_catalog_look(self, id: String) {
        if self
            .looks
            .snapshot
            .get()
            .and_then(|snapshot| snapshot.room_problem())
            .map(|problem| self.alert_look_error(problem))
            .is_some()
        {
            return;
        }
        let Some(item) = self
            .looks
            .catalog
            .get()
            .and_then(|items| items.into_iter().find(|item| item.look.id == id))
        else {
            return;
        };
        // The slot covers the download too: a second add waits its turn
        // instead of losing the slot after its download.
        if !self.claim_look(&id, LookAction::Add) {
            return;
        }
        let work = (id.clone(), LookAction::Add);
        self.looks.adding.update(|adding| {
            adding.insert(id.clone());
        });
        self.run(async move {
            let (grf, cover) = match self.official_grf(&item).await {
                Ok(parts) => parts,
                Err(error) => {
                    self.release_look(&work);
                    self.looks.adding.update(|a| {
                        a.remove(&id);
                    });
                    self.alert_look_error(error);
                    return;
                }
            };
            let finish = move |app: AppState, _ok: bool| {
                app.looks.adding.update(|a| {
                    a.remove(&item.look.id);
                });
            };
            self.run_change(work, LookChange::Content { id, grf, cover }, finish);
        });
    }

    /// The file of an official Look as the camera gets it, with the cover of
    /// the scheme of the card, and the cover it sends.
    async fn official_grf(self, item: &CatalogItem) -> Result<(Vec<u8>, Option<Cover>), LookError> {
        let source = Source::Catalog {
            look: item.look.clone(),
        };
        let grf = self
            .source_bytes(&source)
            .await
            .ok_or_else(|| LookError::Service(tr("looks.catalog_offline", &[])))?;
        // The Look gets the cover of the scheme of the card. Its file has the
        // gray cover, which the camera knows.
        let scheme = self
            .looks
            .snapshot
            .get()
            .map(|snapshot| snapshot.official_covers)
            .unwrap_or_default();
        let film_box = scheme.variant(item.look.monochrome) == Variant::FilmBox;
        let cover = item
            .look
            .film_cover()
            .filter(|_| film_box)
            .map(|statement| statement.film_box());
        let grf = match &cover {
            Some(cover) => look_media::with_cover(&grf, cover)
                .map_err(|error| LookError::Service(error.to_string()))?,
            None => grf,
        };
        Ok((grf, cover))
    }

    /// Add the official Look `id` that a recipe needs, unless the camera has
    /// it. The camera lists a new Look after it starts again.
    pub(crate) async fn ensure_official_look(self, id: &str) -> Result<(), LookError> {
        if self.look(id).is_some() {
            return Ok(());
        }
        self.ensure_catalog().await;
        let item = self
            .looks
            .catalog
            .get()
            .and_then(|items| items.into_iter().find(|item| item.look.id == id))
            .ok_or_else(|| LookError::Service(tr("looks.catalog_offline", &[])))?;
        let (grf, cover) = self.official_grf(&item).await?;
        let snapshot = self.looks.snapshot.get().ok_or(LookError::Changed)?;
        let change = LookChange::Content {
            id: id.to_owned(),
            grf,
            cover,
        };
        let result = self.api().change_look(&snapshot.boot_id, change).await;
        self.refresh_looks();
        result
    }
}

/// Demo screens (`dev::screens`).
#[cfg(demo)]
impl AppState {
    /// A catalog Look being added.
    pub(crate) fn seed_adding(self, id: &str) {
        self.looks.adding.update(|a| {
            a.insert(id.to_owned());
        });
    }
}
