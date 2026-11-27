//! A new Look made from a LUT or preset file on the phone.

use std::collections::BTreeSet;

use grkit_core::{color::Encoding, cover::Cover, fit::Grade};
use nsh_core::picker::FilePickRequest;
use nsh_core::prelude::*;

use super::{LookAction, off_thread};
use crate::domain::{
    LookError, look_media,
    looks::{LookChange, import_id, name_from_file},
    valid_name,
};
use crate::store::{AppState, Route, Tab};
use crate::strings::tr;

/// A Look being made from a file on the phone. The file converts again
/// whenever its LUT input changes; the result shows after Import.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportDraft {
    pub file_name: String,
    pub id: String,
    pub name: String,
    /// The LUT input, or `None` for the one the data suggests.
    pub input: Option<Encoding>,
    /// The cover chosen in the cover editor, or `None` for the default one.
    pub cover: Option<Cover>,
    /// The file is converting.
    pub working: bool,
    /// The last conversion: the LUT input it used (`None` for a preset) and
    /// its grade.
    pub converted: Option<(Option<Encoding>, Grade)>,
    /// Import was pressed: the page shows the result.
    pub done: bool,
    pub sending: bool,
    pub error: Option<LookError>,
}

impl AppState {
    pub fn import_draft(self) -> Option<ImportDraft> {
        self.looks.draft.get()
    }

    /// The cover the Look being imported gets: the one chosen in the cover
    /// editor, else the default cover of its name as it is now.
    pub fn import_cover(self) -> Option<Cover> {
        let draft = self.looks.draft.get()?;
        draft.cover.clone().or_else(|| self.import_default_cover())
    }

    /// The cover of an import without a chosen one.
    pub fn import_default_cover(self) -> Option<Cover> {
        let draft = self.looks.draft.get()?;
        let name = draft.name.trim();
        valid_name(name).then(|| Cover::for_name(name))
    }

    fn taken_ids(self) -> BTreeSet<String> {
        let mut taken: BTreeSet<String> = self
            .looks
            .snapshot
            .get()
            .map(|s| s.ids().into_iter().collect())
            .unwrap_or_default();
        if let Some(items) = self.looks.catalog.get() {
            taken.extend(items.into_iter().map(|item| item.look.id));
        }
        taken
    }

    /// Pick a LUT or preset file and make a new Look from it.
    pub fn begin_import(self) {
        self.set_sheet(None);
        self.root.run(|| {
            spawn(async move {
                let files = pick_files(FilePickRequest::new(false, 1)).await;
                let Some(file) = files.into_iter().next() else {
                    return;
                };
                let bytes = file.data.bytes().to_vec();
                self.start_import(file.name, bytes);
            });
        });
    }

    /// Open the import page for `bytes` (a LUT or preset named
    /// `file_name`) and convert it with the input that its data suggests.
    pub fn start_import(self, file_name: String, bytes: Vec<u8>) {
        let name = name_from_file(&file_name);
        let id = import_id(&name, &self.taken_ids());
        self.looks.draft.set(Some(ImportDraft {
            file_name,
            id,
            name,
            input: None,
            cover: None,
            working: true,
            converted: None,
            done: false,
            sending: false,
            error: None,
        }));
        self.looks.draft_grf.with(|g| *g.borrow_mut() = None);
        self.looks
            .draft_source
            .with(|s| *s.borrow_mut() = Some(bytes));
        if self.tab() != Tab::Looks {
            self.set_tab(Tab::Looks);
        }
        if self.import_depth().is_none() {
            self.push(Route::LookImport);
        }
        self.convert_import();
    }

    /// The depth of the import page on the Looks tab.
    fn import_depth(self) -> Option<usize> {
        (0..crate::store::MAX_DEPTH)
            .find(|depth| self.route_at(Tab::Looks, *depth) == Some(Route::LookImport))
    }

    /// Close the pages above the import page.
    pub fn back_to_import(self) {
        if let Some(depth) = self.import_depth() {
            self.pop_to(Tab::Looks, depth + 1);
        }
    }

    /// Convert the draft's file with its input on a worker thread. A result
    /// for an input that changed meanwhile is dropped.
    fn convert_import(self) {
        let Some(draft) = self.looks.draft.get() else {
            return;
        };
        let Some(bytes) = self.looks.draft_source.with(|s| s.borrow().clone()) else {
            return;
        };
        self.root.run(|| {
            spawn(async move {
                if self.looks.base_gamma.with(|b| b.borrow().is_none()) {
                    self.ensure_catalog().await;
                }
                let Some(base_gamma) = self.looks.base_gamma.with(|b| b.borrow().clone()) else {
                    self.fail_draft(LookError::ImportFailed(tr("looks.catalog_offline", &[])));
                    return;
                };
                let input = draft.input;
                let (file_name, id, name) = (draft.file_name, draft.id, draft.name);
                let result = off_thread(move || {
                    look_media::import(&bytes, &file_name, &id, &name, &base_gamma, input)
                })
                .await;
                if self.looks.draft.get().is_none_or(|d| d.input != input) {
                    return;
                }
                match result {
                    Some(Ok(imported)) => {
                        self.looks
                            .draft_grf
                            .with(|g| *g.borrow_mut() = Some(imported.grf));
                        self.looks.draft.update(|draft| {
                            if let Some(draft) = draft {
                                draft.working = false;
                                draft.converted = Some((imported.input, imported.grade));
                            }
                        });
                    }
                    Some(Err(error)) => {
                        log::warn!("Import failed: {error:#}");
                        self.fail_draft(LookError::ImportFailed(error.to_string()));
                    }
                    None => self.fail_draft(LookError::ImportFailed(String::new())),
                }
            });
        });
    }

    fn fail_draft(self, error: LookError) {
        self.looks.draft.update(|draft| {
            if let Some(draft) = draft {
                draft.working = false;
                draft.error = Some(error);
            }
        });
    }

    pub fn set_import_name(self, name: String) {
        self.looks.draft.update(|draft| {
            if let Some(draft) = draft {
                draft.name = name;
            }
        });
    }

    /// Convert the file again with `input`, or with the suggested input.
    pub fn set_import_input(self, input: Option<Encoding>) {
        let Some(draft) = self.looks.draft.get() else {
            return;
        };
        if draft.input == input {
            return;
        }
        self.looks.draft_grf.with(|g| *g.borrow_mut() = None);
        self.looks.draft.update(|draft| {
            if let Some(draft) = draft {
                draft.input = input;
                draft.working = true;
                draft.converted = None;
            }
        });
        self.convert_import();
    }

    /// The cover from the cover editor, or `None` for the default cover.
    pub fn set_import_cover(self, cover: Option<Cover>) {
        self.looks.draft.update(|draft| {
            if let Some(draft) = draft {
                draft.cover = cover;
            }
        });
    }

    /// Show the result of the conversion, once it is ready and the name is
    /// valid.
    pub fn finish_import(self) {
        let Some(draft) = self.looks.draft.get() else {
            return;
        };
        if draft.working || draft.converted.is_none() {
            return;
        }
        if !valid_name(draft.name.trim()) {
            self.alert_look_error(LookError::InvalidName);
            return;
        }
        self.looks.draft.update(|draft| {
            if let Some(draft) = draft {
                draft.done = true;
            }
        });
    }

    /// Send the converted Look to the camera.
    pub fn send_import(self) {
        let Some(draft) = self.looks.draft.get() else {
            return;
        };
        let Some(grf) = self.looks.draft_grf.with(|g| g.borrow().clone()) else {
            return;
        };
        let name = draft.name.trim().to_owned();
        if !valid_name(&name) {
            self.alert_look_error(LookError::InvalidName);
            return;
        }
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
        let Some(cover) = self.import_cover() else {
            return;
        };
        // The file has the name and cover it was converted with.
        let Ok(grf) = look_media::relabel(&grf, &draft.id, &name, &cover) else {
            self.alert_look_error(LookError::InvalidName);
            return;
        };
        self.looks.draft.update(|d| {
            if let Some(d) = d {
                d.sending = true;
            }
        });
        self.change(
            draft.id.clone(),
            LookAction::Send,
            LookChange::Content {
                id: draft.id.clone(),
                grf,
                cover: Some(cover),
            },
            |app, ok| {
                if ok {
                    app.discard_import();
                    app.pop_to(Tab::Looks, 0);
                } else {
                    // A refused send lets the user try again.
                    app.looks.draft.update(|d| {
                        if let Some(d) = d {
                            d.sending = false;
                        }
                    });
                }
            },
        );
    }

    pub fn discard_import(self) {
        self.looks.draft.set(None);
        self.looks.draft_grf.with(|g| *g.borrow_mut() = None);
        self.looks.draft_source.with(|s| *s.borrow_mut() = None);
    }
}

/// Demo screens (`dev::screens`).
#[cfg(demo)]
impl AppState {
    /// An import ready to send, made into `grf`.
    pub(crate) fn seed_import_draft(self, grf: Vec<u8>) {
        self.looks.draft_grf.with(|g| *g.borrow_mut() = Some(grf));
        self.looks.draft.set(Some(ImportDraft {
            file_name: "Kodak Gold 200.cube".into(),
            id: "lut-kodak-gold-200".into(),
            name: "Kodak Gold 200".into(),
            input: None,
            cover: None,
            working: false,
            converted: Some((Some(Encoding::Standard), Grade::B)),
            done: false,
            sending: false,
            error: None,
        }));
    }
}
