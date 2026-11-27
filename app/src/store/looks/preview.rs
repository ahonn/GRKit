//! The sample photo with each Look, made once and kept.

use nsh_core::prelude::*;

use super::{CatalogItem, Source, off_thread};
use crate::domain::look_media;
use crate::store::AppState;

impl AppState {
    /// The sample photo with the Look `id` as a PNG `data:` URI, once made.
    /// The first ask starts it: an official Look takes its preview table
    /// from the site, another Look one made from its file.
    pub fn look_preview(self, id: &str) -> Option<String> {
        let look = self.look(id)?;
        let source = Source::Camera {
            id: look.id.clone(),
        };
        self.preview(look.content_hash.clone(), look.id, source)
    }

    /// The sample photo with an official Look of the catalog, for the Add
    /// page, as [`Self::look_preview`].
    pub fn catalog_preview(self, item: &CatalogItem) -> Option<String> {
        let source = Source::Catalog {
            look: item.look.clone(),
        };
        self.preview(
            format!("catalog:{}", item.look.id),
            item.look.id.clone(),
            source,
        )
    }

    /// The preview kept under `key`, or `None` while it is made: the first
    /// ask starts it.
    fn preview(self, key: String, id: String, source: Source) -> Option<String> {
        if let Some(made) = self.looks.previews.with(|p| p.get(&key).cloned()) {
            return made;
        }
        let first = self
            .looks
            .preview_requests
            .with(|r| r.borrow_mut().insert(key.clone()));
        if first {
            self.root.run(|| {
                spawn(async move {
                    let made = self.make_preview(&id, source).await;
                    self.looks.previews.update(|p| {
                        p.insert(key, made);
                    });
                });
            });
        }
        None
    }

    /// The sample photo as the camera's Standard renders it, without a
    /// Look: the site's table for the camera's firmware.
    pub fn standard_preview(self) -> Option<String> {
        const KEY: &str = "standard";
        if let Some(made) = self.looks.previews.with(|p| p.get(KEY).cloned()) {
            return made;
        }
        let first = self
            .looks
            .preview_requests
            .with(|r| r.borrow_mut().insert(KEY.to_owned()));
        if first {
            self.root.run(|| {
                spawn(async move {
                    self.ensure_catalog().await;
                    let made = match self.looks.catalog_target.get() {
                        Some(target) => match self.api().standard_table(&target).await {
                            Ok(table) => off_thread(move || {
                                crate::domain::look_preview::sample_preview_png(&table).ok()
                            })
                            .await
                            .flatten()
                            .map(|png| crate::domain::images::data_uri("image/png", &png)),
                            Err(_) => None,
                        },
                        None => None,
                    };
                    self.looks.previews.update(|p| {
                        p.insert(KEY.to_owned(), made);
                    });
                });
            });
        }
        None
    }

    /// The sample photo with the Look `id`: the site's table of an official
    /// Look, else a table made from the Look's file in `source`.
    async fn make_preview(self, id: &str, source: Source) -> Option<String> {
        let table = match self.catalog_table(id).await {
            Some(table) => table,
            // The site has the tables of its Looks: an official Look is not
            // downloaded whole for a preview.
            None if matches!(source, Source::Catalog { .. }) => return None,
            None => {
                if self.looks.base_gamma.with(|b| b.borrow().is_none()) {
                    self.ensure_catalog().await;
                }
                let base_gamma = self.looks.base_gamma.with(|b| b.borrow().clone())?;
                let grf = self.source_bytes(&source).await?;
                off_thread(move || look_media::preview_table(&grf, &base_gamma).ok())
                    .await
                    .flatten()?
            }
        };
        let png = off_thread(move || crate::domain::look_preview::sample_preview_png(&table).ok())
            .await
            .flatten()?;
        Some(crate::domain::images::data_uri("image/png", &png))
    }

    /// The site's preview table of the official Look `id`, which shows the
    /// camera's own tone of a Look made from a native one.
    async fn catalog_table(self, id: &str) -> Option<Vec<u8>> {
        self.ensure_catalog().await;
        let target = self.looks.catalog_target.get()?;
        let item = self
            .looks
            .catalog
            .with(|c| c.as_ref()?.iter().find(|i| i.look.id == id).cloned())?;
        self.api().preview_table(&target, &item.look).await.ok()
    }
}
