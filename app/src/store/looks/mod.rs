//! Film Looks of the connected camera: the list, changes, covers, the
//! official catalog (`catalog`), the sample photos (`preview`), and imports
//! (`import`). The camera is authoritative: every change ends with a fresh
//! read of its Looks, also after a failure.

mod catalog;
mod import;
mod preview;

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

use grkit_core::cover::Cover;
use nsh_core::prelude::*;

use super::{Alert, AppState, Route, Tab};
use crate::domain::{
    CatalogLook, ConnectionState, JournalState, Look, LookError, LookFilter, LooksSnapshot,
    look_media, looks::LookChange, valid_name,
};
use crate::strings::tr;

pub use catalog::CatalogItem;
pub use import::ImportDraft;

/// What a change to the Looks does, so that only its own control shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookAction {
    Toggle,
    Rename,
    Cover,
    DefaultCover,
    Delete,
    /// To the card only, or back to the camera.
    Move,
    TakeBack,
    Add,
    Order,
    Send,
}

/// The key of the Look (or `order`) being changed, and how.
type LookWork = (String, LookAction);

/// Where the bytes of one Look content come from.
#[derive(Clone, Debug)]
enum Source {
    Camera { id: String },
    Catalog { look: CatalogLook },
}

/// Looks state, kept in the app state (`Copy` handles).
#[derive(Clone, Copy)]
pub struct LooksStore {
    snapshot: Signal<Option<LooksSnapshot>>,
    /// The Looks read before the camera went away, and the camera's
    /// peripheral ID: the tab shows them read-only until it is back.
    offline: Signal<Option<(String, LooksSnapshot)>>,
    loading: Signal<bool>,
    error: Signal<Option<LookError>>,
    /// Why changes are not possible now (an update waits on the camera).
    blocked: Signal<Option<LookError>>,
    /// The change under way.
    busy: Signal<Option<LookWork>>,
    filter: Signal<LookFilter>,
    catalog: Signal<Option<Vec<CatalogItem>>>,
    catalog_error: Signal<Option<String>>,
    catalog_target: Signal<Option<String>>,
    adding: Signal<BTreeSet<String>>,
    /// The camera's base gamma, from the catalog: imports fit to it.
    base_gamma: Stored<RefCell<Option<Vec<u16>>>>,
    draft: Signal<Option<ImportDraft>>,
    draft_grf: Stored<RefCell<Option<Vec<u8>>>>,
    /// The file of the draft, kept to convert again with another input.
    draft_source: Stored<RefCell<Option<Vec<u8>>>>,
    /// The sample photo with each Look, as PNG `data:` URIs, by the digest
    /// of the Look's file; `None` when it cannot be made.
    previews: Signal<BTreeMap<String, Option<String>>>,
    /// The previews asked for, so each is made once.
    preview_requests: Stored<RefCell<BTreeSet<String>>>,
}

impl LooksStore {
    pub fn new() -> Self {
        Self {
            snapshot: signal(None),
            offline: signal(None),
            loading: signal(false),
            error: signal(None),
            blocked: signal(None),
            busy: signal(None),
            filter: signal(LookFilter::All),
            catalog: signal(None),
            catalog_error: signal(None),
            catalog_target: signal(None),
            adding: signal(BTreeSet::new()),
            base_gamma: stored(RefCell::new(None)),
            draft: signal(None),
            draft_grf: stored(RefCell::new(None)),
            draft_source: stored(RefCell::new(None)),
            previews: signal(BTreeMap::new()),
            preview_requests: stored(RefCell::new(BTreeSet::new())),
        }
    }
}

impl Default for LooksStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Run `work` on a worker thread and await its result.
pub(crate) async fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (tx, rx) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let _ = tx.send_blocking(work());
    });
    rx.recv().await.ok()
}

impl AppState {
    fn looks_store(self) -> LooksStore {
        self.looks
    }

    pub fn looks_snapshot(self) -> Option<LooksSnapshot> {
        self.looks.snapshot.get()
    }

    /// The Looks of the saved camera as the phone last read them, while
    /// no camera is connected.
    pub fn offline_looks(self) -> Option<LooksSnapshot> {
        if self.camera_online() {
            return None;
        }
        let camera = self.remembered()?;
        self.looks
            .offline
            .get()
            .filter(|(id, _)| id.eq_ignore_ascii_case(&camera.peripheral_id))
            .map(|(_, snapshot)| snapshot)
    }

    pub fn looks_loading(self) -> bool {
        self.looks.loading.get()
    }

    pub fn looks_error(self) -> Option<LookError> {
        self.looks.error.get()
    }

    pub fn looks_blocked(self) -> Option<LookError> {
        self.looks.blocked.get()
    }

    pub fn look(self, id: &str) -> Option<Look> {
        self.looks
            .snapshot
            .get()
            .and_then(|snapshot| snapshot.get(id).cloned())
    }

    pub fn look_busy(self) -> Option<String> {
        self.looks.busy.get().map(|(key, _)| key)
    }

    /// `action` is under way on the Look (or group) `key`.
    pub fn look_working(self, key: &str, action: LookAction) -> bool {
        self.looks
            .busy
            .with(|busy| matches!(busy, Some((k, a)) if k == key && *a == action))
    }

    /// A change is under way on the Look `key`.
    pub fn look_row_busy(self, key: &str) -> bool {
        self.looks
            .busy
            .with(|busy| matches!(busy, Some((k, _)) if k == key))
    }

    /// The list is being read, or its new order saved.
    pub fn looks_refreshing(self) -> bool {
        self.looks.loading.get() || self.look_row_busy("order")
    }

    /// Take the one change slot, unless a change holds it.
    fn claim_look(self, key: &str, action: LookAction) -> bool {
        if self.looks.busy.with(Option::is_some) {
            return false;
        }
        self.looks.busy.set(Some((key.to_owned(), action)));
        true
    }

    /// Give the slot back. A link loss may have given it to a newer change.
    fn release_look(self, work: &LookWork) {
        if self.looks.busy.with(|busy| busy.as_ref() == Some(work)) {
            self.looks.busy.set(None);
        }
    }

    pub fn looks_filter(self) -> LookFilter {
        self.looks.filter.get()
    }

    pub fn set_looks_filter(self, filter: LookFilter) {
        self.looks.filter.set(filter);
    }

    fn connected(self) -> bool {
        self.status
            .get()
            .is_some_and(|s| s.camera.connection == ConnectionState::Connected)
    }

    /// The Looks tab became visible: read the camera's Looks.
    pub fn open_looks(self) {
        if self.connected() && !self.looks.loading.get() {
            self.refresh_looks();
        }
    }

    /// Read the camera's Looks.
    pub fn refresh_looks(self) {
        if !self.connected() {
            return;
        }
        if !self.looks_available() {
            self.looks.error.set(None);
            return;
        }
        let generation = self.connect_generation.get();
        self.looks.loading.set(true);
        let api = self.api();
        self.run(async move {
            let result = api.looks().await;
            let journal = api.update_snapshot().await.map(|s| s.state);
            if self.connect_generation.get() != generation {
                return;
            }
            self.looks.loading.set(false);
            self.looks.blocked.set(
                matches!(journal, Ok(JournalState::Waiting | JournalState::Confirmed))
                    .then_some(LookError::UpdatePending),
            );
            match result {
                Ok(snapshot) => {
                    self.looks.snapshot.set(Some(snapshot));
                    self.looks.error.set(None);
                }
                Err(error) => {
                    log::warn!("Could not read Looks: {error:?}");
                    self.looks.error.set(Some(error));
                }
            }
        });
    }

    async fn source_bytes(self, source: &Source) -> Option<Vec<u8>> {
        match source {
            Source::Camera { id } => {
                let hash = self.look(id).map(|look| look.content_hash)?;
                self.api().look_content(id, &hash).await.ok()
            }
            Source::Catalog { look } => {
                let target = self.looks.catalog_target.get()?;
                self.api().catalog_file(&target, look).await.ok()
            }
        }
    }

    /// Report a failed change: an alert, or for a lost link a notice.
    pub fn alert_look_error(self, error: LookError) {
        // A change that the lost link cut off: a notice, and the tab shows
        // the camera as not connected once the link reports it.
        if error == LookError::ConnectionLost {
            self.clear_name_draft();
            self.show_toast(tr("looks.lost_toast", &[]));
            self.pop_to(crate::store::Tab::Looks, 0);
            return;
        }
        if error == LookError::UpdatePending {
            self.looks.blocked.set(Some(error.clone()));
        }
        self.alert(Alert::Look(error));
    }

    /// Apply one change on the camera, and read the Looks again, also after
    /// a failure. While another change runs, this one is refused and
    /// `settle` hears so.
    pub(super) fn change(
        self,
        key: String,
        action: LookAction,
        change: LookChange,
        settle: impl FnOnce(AppState, bool) + 'static,
    ) {
        if !self.claim_look(&key, action) {
            settle(self, false);
            return;
        }
        self.run_change((key, action), change, settle);
    }

    /// Run a change that holds the slot, and give the slot back.
    fn run_change(
        self,
        work: LookWork,
        change: LookChange,
        settle: impl FnOnce(AppState, bool) + 'static,
    ) {
        let Some(snapshot) = self.looks.snapshot.get() else {
            self.release_look(&work);
            settle(self, false);
            return;
        };
        let answer = self.api().change_look(&snapshot.boot_id, change);
        self.run(async move {
            let result = answer.await;
            let ok = result.is_ok();
            // One update: the slot is free when the result shows.
            batch(|| {
                self.release_look(&work);
                self.refresh_looks();
                settle(self, ok);
            });
            if let Err(error) = result {
                self.alert_look_error(error);
            }
        });
    }

    pub fn set_look_enabled(self, id: String, enabled: bool) {
        let key_id = id.clone();
        self.change(
            id.clone(),
            LookAction::Toggle,
            LookChange::Enabled { id, enabled },
            // The switch keeps the new value while the list is read again,
            // instead of moving back until it arrives.
            move |app, ok| {
                if ok {
                    app.looks.snapshot.update(|snapshot| {
                        if let Some(look) = snapshot
                            .as_mut()
                            .and_then(|s| s.looks.iter_mut().find(|look| look.id == key_id))
                        {
                            look.enabled = enabled;
                        }
                    });
                }
            },
        );
    }

    pub fn rename_look(self, id: String, name: String) {
        let name = name.trim().to_owned();
        if !valid_name(&name) {
            self.alert_look_error(LookError::InvalidName);
            return;
        }
        self.change(
            id.clone(),
            LookAction::Rename,
            LookChange::Rename { id, name },
            move |app, ok| {
                if ok {
                    app.clear_name_draft();
                    app.pop_to(Tab::Looks, 1);
                }
            },
        );
    }

    /// Give a Look another cover: its file with the icon of `cover`, under
    /// the same ID. The camera shows it after it starts again.
    pub fn save_look_cover(self, id: String, cover: Cover) {
        self.write_cover(id, cover, LookAction::Cover);
    }

    /// Give a Look its default cover.
    pub fn restore_look_cover(self, id: String, cover: Cover) {
        self.write_cover(id, cover, LookAction::DefaultCover);
    }

    fn write_cover(self, id: String, cover: Cover, action: LookAction) {
        let Some(look) = self.look(&id) else {
            return;
        };
        if look.shown_cover() == cover {
            self.close_cover(&id);
            return;
        }
        if !cover.fits() || !self.claim_look(&id, action) {
            return;
        }
        let work = (id.clone(), action);
        self.run(async move {
            let Some(grf) = self.source_bytes(&Source::Camera { id: id.clone() }).await else {
                self.release_look(&work);
                self.alert_look_error(LookError::ConnectionLost);
                return;
            };
            let grf = match look_media::with_cover(&grf, &cover) {
                Ok(grf) => grf,
                Err(error) => {
                    self.release_look(&work);
                    self.alert_look_error(LookError::Service(error.to_string()));
                    return;
                }
            };
            let page_id = id.clone();
            self.run_change(
                work,
                LookChange::Content {
                    id,
                    grf,
                    cover: Some(cover),
                },
                move |app, ok| {
                    if ok {
                        app.close_cover(&page_id);
                    }
                },
            );
        });
    }

    /// Close the cover page of a Look. The page opens from the Look's page
    /// or from the list, so its depth differs.
    fn close_cover(self, id: &str) {
        let page = Some(Route::LookCover(id.to_owned()));
        if let Some(depth) =
            (0..super::MAX_DEPTH).find(|depth| self.route_at(Tab::Looks, *depth) == page)
        {
            self.pop_to(Tab::Looks, depth);
        }
    }

    /// Delete a Look and go back to the Looks tab.
    pub fn delete_look(self, id: String, active_off: bool) {
        self.remove_look(id, active_off, true);
    }

    /// Move a Look to the card only (`to_card`), or back to the camera, and
    /// stay on its page. `active_off` switches the camera to Off when the
    /// Look is in use.
    pub fn move_look(self, id: String, to_card: bool, active_off: bool) {
        self.change(
            id.clone(),
            LookAction::Move,
            LookChange::Place {
                id,
                to_card,
                active_off,
            },
            |_, _| {},
        );
    }

    /// Take back an official Look just added on the Add page, which stays.
    pub fn take_back_look(self, id: String) {
        self.remove_look(id, false, false);
    }

    fn remove_look(self, id: String, active_off: bool, close: bool) {
        let action = if close {
            LookAction::Delete
        } else {
            LookAction::TakeBack
        };
        self.change(
            id.clone(),
            action,
            LookChange::Delete { id, active_off },
            move |app, ok| {
                if ok && close {
                    app.pop_to(Tab::Looks, 0);
                }
            },
        );
    }

    /// Move the shown Look at `from` to `to`, both counted among the shown
    /// Looks. The camera menu takes the order at once.
    pub fn move_shown_look(self, from: usize, to: usize) {
        let Some(snapshot) = self.looks.snapshot.get() else {
            return;
        };
        // While another change runs the camera would not take this order.
        if from == to || self.looks.busy.with(Option::is_some) {
            return;
        }
        let saved: Vec<String> = snapshot.looks.iter().map(|look| look.id.clone()).collect();
        let ids = crate::domain::looks::move_shown(
            snapshot
                .looks
                .iter()
                .map(|look| (look.id.as_str(), look.enabled)),
            from,
            to,
        );
        // The grid shows the new order while the camera takes it.
        self.looks.snapshot.update(|snapshot| {
            if let Some(snapshot) = snapshot {
                snapshot
                    .looks
                    .sort_by_key(|look| ids.iter().position(|id| *id == look.id));
            }
        });
        self.change(
            "order".into(),
            LookAction::Order,
            LookChange::Order(ids),
            move |app, ok| {
                // The camera kept its order, also when it cannot be read now.
                if !ok {
                    app.looks.snapshot.update(|snapshot| {
                        if let Some(snapshot) = snapshot {
                            snapshot
                                .looks
                                .sort_by_key(|look| saved.iter().position(|id| *id == look.id));
                        }
                    });
                }
            },
        );
    }

    /// Clear Looks state when the camera changes or disconnects.
    pub(crate) fn reset_looks(self) {
        let store = self.looks_store();
        if let (Some(snapshot), Some(camera)) = (store.snapshot.get(), self.remembered()) {
            store.offline.set(Some((camera.peripheral_id, snapshot)));
        }
        store.snapshot.set(None);
        store.error.set(None);
        store.blocked.set(None);
        store.busy.set(None);
        // A read that the lost link cut off never reports back.
        store.loading.set(false);
        self.reset_recipe_read();
        self.reset_module_read();
    }
}

/// Demo screens (`dev::screens`).
#[cfg(demo)]
impl AppState {
    pub(crate) fn seed_looks_failed(self) {
        self.looks.snapshot.set(None);
        self.looks.loading.set(false);
        self.looks.error.set(Some(LookError::ConnectionLost));
    }
}
