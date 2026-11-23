//! Film Looks of the connected camera: the list, changes, covers, the
//! official catalog, and imports. The camera is authoritative: every change
//! ends with a fresh read of its Looks, also after a failure.

use std::{cell::RefCell, collections::BTreeSet};

use grkit_core::{
    color::Encoding,
    cover::{Cover, Scheme, Variant},
    fit::Grade,
};

use nsh_base::alert_dialog;
use nsh_core::picker::FilePickRequest;
use nsh_core::prelude::*;

use super::{AppState, Route, Tab};
use crate::domain::{
    CatalogLook, ConnectionState, CoverGroup, JournalState, Look, LookError, LookFilter, LookGroup,
    LooksSnapshot,
    fake_looks::{FakeLooks, demo_catalog},
    group_of, look_media,
    looks::{import_id, name_from_file},
    valid_name,
};
use crate::platform::{
    self,
    looks::{self as remote, CameraContext},
};
use crate::strings::tr;

/// The busy key of a covers change.
pub const COVERS_KEY: &str = "covers";

/// The demo catalog: each official Look with its GRF file.
type DemoCatalog = Vec<(CatalogLook, Vec<u8>)>;

/// What a change to the Looks does, so that only its own control shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookAction {
    Toggle,
    Rename,
    Cover,
    DefaultCover,
    Delete,
    TakeBack,
    Add,
    Order,
    Scheme(Scheme),
    Reset(CoverGroup),
    Send,
}

/// The key of the Look (or `order`, `covers`) being changed, and how.
type LookWork = (String, LookAction);

/// An official Look of the catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogItem {
    pub look: CatalogLook,
    pub group: LookGroup,
}

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
    fake: Stored<RefCell<Option<FakeLooks>>>,
    fake_catalog: Stored<RefCell<DemoCatalog>>,
}

impl LooksStore {
    pub fn new(demo: bool) -> Self {
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
            base_gamma: stored(RefCell::new(demo.then(look_media::demo_base_gamma))),
            draft: signal(None),
            draft_grf: stored(RefCell::new(None)),
            draft_source: stored(RefCell::new(None)),
            fake: stored(RefCell::new(demo.then(FakeLooks::demo))),
            fake_catalog: stored(RefCell::new(if demo { demo_catalog() } else { Vec::new() })),
        }
    }
}

/// Run `work` on a worker thread and await its result.
async fn off_thread<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
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

    /// Open the page of the official Looks that the camera does not have.
    pub fn open_add(self) {
        self.load_catalog();
        self.push(Route::LookAdd);
    }

    pub fn looks_filter(self) -> LookFilter {
        self.looks.filter.get()
    }

    pub fn set_looks_filter(self, filter: LookFilter) {
        self.looks.filter.set(filter);
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
        if self.demo() {
            let snapshot = self
                .looks
                .fake
                .with(|fake| fake.borrow().as_ref().map(FakeLooks::snapshot));
            let blocked = self.looks.fake.with(|fake| {
                fake.borrow()
                    .as_ref()
                    .is_some_and(|f| f.update_pending)
                    .then_some(LookError::UpdatePending)
            });
            self.looks.blocked.set(blocked);
            self.looks.snapshot.set(snapshot);
            self.looks.error.set(None);
            return;
        }
        let generation = self.connect_generation.get();
        self.looks.loading.set(true);
        self.root.run(|| {
            spawn(async move {
                let result = remote::list().await;
                let journal = platform::update::snapshot().await.map(|s| s.state);
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
        });
    }

    async fn source_bytes(self, source: &Source) -> Option<Vec<u8>> {
        match source {
            Source::Camera { id } if self.demo() => self
                .looks
                .fake
                .with(|fake| fake.borrow().as_ref().and_then(|f| f.content(id))),
            Source::Camera { id } => {
                let hash = self.look(id).map(|look| look.content_hash)?;
                remote::content(id, &hash).await.ok()
            }
            Source::Catalog { look } if self.demo() => self.looks.fake_catalog.with(|c| {
                c.borrow()
                    .iter()
                    .find(|(l, _)| l.id == look.id)
                    .map(|(_, grf)| grf.clone())
            }),
            Source::Catalog { look } => {
                let target = self.looks.catalog_target.get()?;
                remote::catalog_file(&target, look).await.ok()
            }
        }
    }

    /// Load the official catalog for the camera firmware.
    async fn ensure_catalog(self) {
        if self.looks.catalog.get().is_some() {
            return;
        }
        if self.demo() {
            let items = self.looks.fake_catalog.with(|c| {
                c.borrow()
                    .iter()
                    .map(|(look, _)| CatalogItem {
                        group: group_of(&look.id, look.monochrome),
                        look: look.clone(),
                    })
                    .collect()
            });
            self.looks.catalog.set(Some(items));
            return;
        }
        let Some(status) = self.status.get() else {
            return;
        };
        let Some(target) = remote::target(status.camera.model, &status.camera.firmware) else {
            self.looks
                .catalog_error
                .set(Some(tr("looks.catalog_unsupported", &[])));
            return;
        };
        match remote::catalog(target).await {
            Ok(catalog) => {
                self.looks
                    .base_gamma
                    .with(|b| *b.borrow_mut() = Some(catalog.base_gamma.clone()));
                self.looks.catalog_target.set(Some(target.to_owned()));
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
            Err(error) => {
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

    /// Present a failed change as an alert.
    pub fn alert_look_error(self, error: LookError) {
        // A change that the lost link cut off: a notice, and the tab shows
        // the camera as not connected once the link reports it.
        if error == LookError::ConnectionLost {
            self.clear_name_draft();
            self.show_toast(tr("looks.lost_toast", &[]));
            self.pop_to(crate::store::Tab::Looks, 0);
            return;
        }
        let (title, body) = error.keys();
        let limits = self.looks.snapshot.get();
        let max = match &error {
            LookError::NoRoom => limits.map(|s| s.max_loaded),
            LookError::CardFull => limits.map(|s| s.max_listed),
            _ => None,
        }
        .unwrap_or_default()
        .to_string();
        let body = match &error {
            LookError::Service(detail) => format!("{} {detail}", tr(body, &[])),
            _ => tr(body, &[("max", &max)]),
        };
        if matches!(error, LookError::UpdatePending | LookError::ChangesOff) {
            self.looks.blocked.set(Some(error.clone()));
        }
        spawn(async move {
            let _ = alert_dialog()
                .title(tr(title, &[]))
                .description(body)
                .cancel("ok", tr("looks.ok", &[]))
                .show()
                .await;
        });
    }

    /// Apply one change: on the demo camera directly, on a real camera over
    /// HTTP. Reads the Looks again either way. While another change runs,
    /// this one is refused and `settle` hears so.
    fn change<F, Fut>(
        self,
        key: String,
        action: LookAction,
        demo: impl FnOnce(&mut FakeLooks) -> Result<(), LookError>,
        real: F,
        settle: impl FnOnce(AppState, bool) + 'static,
    ) where
        F: FnOnce(CameraContext) -> Fut + 'static,
        Fut: std::future::Future<Output = Result<(), LookError>> + 'static,
    {
        if !self.claim_look(&key, action) {
            settle(self, false);
            return;
        }
        self.run_change((key, action), demo, real, settle);
    }

    /// Run a change that holds the slot, and give the slot back.
    fn run_change<F, Fut>(
        self,
        work: LookWork,
        demo: impl FnOnce(&mut FakeLooks) -> Result<(), LookError>,
        real: F,
        settle: impl FnOnce(AppState, bool) + 'static,
    ) where
        F: FnOnce(CameraContext) -> Fut + 'static,
        Fut: std::future::Future<Output = Result<(), LookError>> + 'static,
    {
        if self.demo() {
            self.release_look(&work);
            let result = self.looks.fake.with(|fake| {
                fake.borrow_mut()
                    .as_mut()
                    .map_or(Err(LookError::ConnectionLost), demo)
            });
            self.refresh_looks();
            let ok = result.is_ok();
            if let Err(error) = result {
                self.alert_look_error(error);
            }
            settle(self, ok);
            return;
        }
        let Some(snapshot) = self.looks.snapshot.get() else {
            self.release_look(&work);
            settle(self, false);
            return;
        };
        let context = CameraContext::from(&snapshot);
        self.root.run(|| {
            spawn(async move {
                let result = real(context).await;
                self.release_look(&work);
                self.refresh_looks();
                let ok = result.is_ok();
                if let Err(error) = result {
                    self.alert_look_error(error);
                }
                settle(self, ok);
            });
        });
    }

    pub fn set_look_enabled(self, id: String, enabled: bool) {
        let key = id.clone();
        let demo_id = id.clone();
        self.change(
            key,
            LookAction::Toggle,
            move |fake| fake.set_enabled(&demo_id, enabled),
            move |context| async move { remote::set_enabled(&context, &id, enabled).await },
            |_, _| {},
        );
    }

    pub fn rename_look(self, id: String, name: String) {
        let name = name.trim().to_owned();
        if !valid_name(&name) {
            self.alert_look_error(LookError::InvalidName);
            return;
        }
        let (demo_id, demo_name) = (id.clone(), name.clone());
        self.change(
            id.clone(),
            LookAction::Rename,
            move |fake| fake.rename(&demo_id, &demo_name),
            move |context| async move { remote::rename(&context, &id, &name).await },
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
        self.root.run(|| {
            spawn(async move {
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
                let demo_grf = grf.clone();
                let demo_cover = cover.clone();
                let look_id = id.clone();
                let page_id = id;
                self.run_change(
                    work,
                    move |fake| fake.put(&demo_grf, false, Some(demo_cover)),
                    move |context| async move {
                        remote::put_content(&context, &look_id, grf, Some(&cover)).await
                    },
                    move |app, ok| {
                        if ok {
                            app.close_cover(&page_id);
                        }
                    },
                );
            });
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

    /// Give the built-in Looks without an individual cover the covers of
    /// `scheme`. The camera rewrites their files.
    pub fn set_cover_scheme(self, scheme: Scheme) {
        if self
            .looks
            .snapshot
            .get()
            .is_some_and(|snapshot| snapshot.official_covers == scheme)
        {
            return;
        }
        self.change(
            COVERS_KEY.into(),
            LookAction::Scheme(scheme),
            move |fake| fake.set_scheme(scheme),
            move |context| async move { remote::set_cover_scheme(&context, scheme).await },
            |_, _| {},
        );
    }

    /// Put the individual covers of the built-in Looks of `group` back to the
    /// scheme.
    pub fn reset_covers(self, group: CoverGroup) {
        self.change(
            COVERS_KEY.into(),
            LookAction::Reset(group),
            move |fake| fake.reset_covers(group),
            move |context| async move { remote::reset_covers(&context, group).await },
            |_, _| {},
        );
    }

    /// Delete a Look and go back to the Looks tab.
    pub fn delete_look(self, id: String, active_off: bool) {
        self.remove_look(id, active_off, true);
    }

    /// Take back an official Look just added on the Add page, which stays.
    pub fn take_back_look(self, id: String) {
        self.remove_look(id, false, false);
    }

    fn remove_look(self, id: String, active_off: bool, close: bool) {
        let demo_id = id.clone();
        let action = if close {
            LookAction::Delete
        } else {
            LookAction::TakeBack
        };
        self.change(
            id.clone(),
            action,
            move |fake| fake.delete(&demo_id, active_off),
            move |context| async move { remote::delete(&context, &id, active_off).await },
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
        let demo_ids = ids.clone();
        self.change(
            "order".into(),
            LookAction::Order,
            move |fake| fake.set_order(&demo_ids),
            move |context| async move { remote::set_order(&context, &ids).await },
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
        self.root.run(|| {
            spawn(async move {
                let (grf, cover, demo_cover) = match self.official_grf(&item).await {
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
                let demo_grf = grf.clone();
                let look_id = id.clone();
                self.run_change(
                    work,
                    move |fake| fake.put(&demo_grf, true, demo_cover),
                    move |context| async move {
                        remote::put_content(&context, &look_id, grf, cover.as_ref()).await
                    },
                    finish,
                );
            });
        });
    }

    /// The file of an official Look as the camera gets it, with the cover of
    /// the scheme of the card, the cover it sends, and the cover the demo
    /// camera keeps.
    async fn official_grf(
        self,
        item: &CatalogItem,
    ) -> Result<(Vec<u8>, Option<Cover>, Option<Cover>), LookError> {
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
        // The demo camera knows the release covers from the catalog.
        let demo_cover = cover.clone().or_else(|| item.look.film_cover());
        Ok((grf, cover, demo_cover))
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
        let (grf, cover, demo_cover) = self.official_grf(&item).await?;
        let result = if self.demo() {
            self.looks.fake.with(|fake| {
                fake.borrow_mut()
                    .as_mut()
                    .map_or(Err(LookError::ConnectionLost), |f| {
                        f.put(&grf, true, demo_cover)
                    })
            })
        } else {
            let snapshot = self.looks.snapshot.get().ok_or(LookError::Changed)?;
            remote::put_content(&CameraContext::from(&snapshot), id, grf, cover.as_ref()).await
        };
        self.refresh_looks();
        result
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
        (0..super::MAX_DEPTH)
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
        let demo_grf = grf.clone();
        let demo_cover = cover.clone();
        let id = draft.id.clone();
        self.change(
            draft.id.clone(),
            LookAction::Send,
            move |fake| fake.put(&demo_grf, false, Some(demo_cover)),
            move |context| async move {
                remote::put_content(&context, &id, grf, Some(&cover)).await
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

    /// Demo screens: replace the demo camera's state.
    pub(crate) fn with_fake_looks(self, edit: impl FnOnce(&mut FakeLooks)) {
        self.looks.fake.with(|fake| {
            if let Some(fake) = fake.borrow_mut().as_mut() {
                edit(fake);
            }
        });
    }

    /// Demo screens: mark a catalog Look as being added.
    pub(crate) fn seed_adding(self, id: &str) {
        self.looks.adding.update(|a| {
            a.insert(id.to_owned());
        });
    }

    /// Demo screens: an import ready to send, made from a bundled Look.
    pub(crate) fn seed_import_draft(self) {
        let grf = self.looks.fake.with(|f| {
            f.borrow()
                .as_ref()
                .and_then(|f| f.content(crate::domain::fake_looks::DEMO_CUSTOM_ID))
        });
        let Some(grf) = grf else { return };
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
    }
}
