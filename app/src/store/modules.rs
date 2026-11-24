//! Module settings of the connected camera. Everything shown comes from the
//! camera's catalog, kept on the phone by camera and language.
//!
//! A value or an Enable shows once the camera confirms it. A drag shows its
//! new order at once, and the old order returns when the camera refuses it.

use std::collections::BTreeMap;

use nsh_base::alert_dialog;
use nsh_core::prelude::*;

use super::{AppState, MAX_DEPTH, Route, Tab};
use crate::domain::{
    LookError,
    fake_modules::{self, FakeChange},
    modules::{
        CatalogAnswer, Managed, ModuleCatalog, ModuleDraft, ModuleLayout, ModulePlace, ModuleRow,
        ModulesState, module_rows, package_can_install, png_data_uri, settle,
    },
};
use crate::platform::modules::{self as remote, CachedIcon, ModuleCache};
use crate::strings::{app_language, tr};

/// Why the tab shows a message in place of the modules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModulesBlock {
    /// This GRKit does not serve module settings.
    Unsupported,
    /// The camera's catalog is in a format newer than this app.
    AppTooOld,
}

/// The part of a module a change is saving, so only its control shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModulePart {
    Enabled,
    Setting(String),
    /// One value being shown or hidden in ADJ.
    Option(i64),
    /// The order of the values.
    Options,
    Order,
}

type ModuleWork = (String, ModulePart);

/// Module settings state, kept in the app state (`Copy` handles).
#[derive(Clone, Copy)]
pub struct ModulesStore {
    catalog: Signal<Option<ModuleCatalog>>,
    state: Signal<Option<ModulesState>>,
    /// `data:` URIs of the icons by module ID.
    icons: Signal<BTreeMap<String, String>>,
    /// The saved camera and language the catalog and state are from.
    owner: Signal<Option<(String, String)>>,
    /// The camera start of the last read, which changes must name.
    boot_id: Signal<Option<String>>,
    /// This session read the state, and the camera takes changes.
    writable: Signal<bool>,
    block: Signal<Option<ModulesBlock>>,
    loading: Signal<bool>,
    error: Signal<Option<LookError>>,
    busy: Signal<Option<ModuleWork>>,
    /// Counts changes, so a read that started before one does not undo it.
    changes: Stored<u64>,
    /// The state from before a change that shows at once, until it settles.
    rollback: Stored<Option<ModulesState>>,
    pub(crate) editing: Signal<bool>,
    /// The modules to add and remove with the next update.
    draft: Signal<ModuleDraft>,
}

impl ModulesStore {
    pub fn new() -> Self {
        Self {
            catalog: signal(None),
            state: signal(None),
            icons: signal(BTreeMap::new()),
            owner: signal(None),
            boot_id: signal(None),
            writable: signal(false),
            block: signal(None),
            loading: signal(false),
            error: signal(None),
            busy: signal(None),
            changes: stored(0),
            rollback: stored(None),
            editing: signal(false),
            draft: signal(ModuleDraft::default()),
        }
    }
}

impl Default for ModulesStore {
    fn default() -> Self {
        Self::new()
    }
}

/// A change to send to the camera.
enum Change {
    Patch(String, serde_json::Value, FakeChange),
    Order(Vec<String>),
    Layout(ModuleLayout),
}

impl AppState {
    /// The modules in ADJ order, when the catalog and a state are known.
    pub fn module_rows(self) -> Option<Vec<ModuleRow>> {
        let catalog = self.modules.catalog.get()?;
        let state = self.modules.state.get()?;
        Some(module_rows(&catalog, &state))
    }

    pub fn module_row(self, id: &str) -> Option<ModuleRow> {
        self.module_rows()?.into_iter().find(|r| r.id() == id)
    }

    pub fn module_icon(self, id: &str) -> Option<String> {
        self.modules.icons.with(|icons| icons.get(id).cloned())
    }

    pub fn modules_block(self) -> Option<ModulesBlock> {
        self.modules.block.get()
    }

    pub fn modules_error(self) -> Option<LookError> {
        self.modules.error.get()
    }

    pub fn modules_loading(self) -> bool {
        self.modules.loading.get()
    }

    /// Controls work: the camera is connected, this session read its state,
    /// and no other change is running.
    pub fn modules_editable(self) -> bool {
        self.camera_online()
            && self.modules.writable.get()
            && self.modules.busy.with(Option::is_none)
    }

    /// The camera can take changes now, whether or not one is running.
    pub fn modules_writable(self) -> bool {
        self.camera_online() && self.modules.writable.get()
    }

    pub fn module_busy(self) -> Option<String> {
        self.modules
            .busy
            .with(|b| b.as_ref().map(|(id, _)| id.clone()))
    }

    pub fn module_part_busy(self, id: &str, part: &ModulePart) -> bool {
        self.modules
            .busy
            .with(|b| matches!(b, Some((i, p)) if i == id && p == part))
    }

    fn module_owner(self) -> (String, String) {
        let camera = self
            .remembered()
            .map_or_else(|| "camera".to_owned(), |c| c.peripheral_id);
        (camera, app_language().to_owned())
    }

    /// Drop data of another camera or language, and show what the cache holds
    /// for this one until the camera answers.
    fn adopt_module_cache(self) -> Option<ModuleCache> {
        let owner = self.module_owner();
        let cache = ModuleCache::open(&owner.0, &owner.1);
        if self.modules.owner.with(|o| o.as_ref() != Some(&owner)) {
            self.modules.draft.set(remote::load_draft(&owner.0));
            self.modules.owner.set(Some(owner));
            self.modules
                .catalog
                .set(cache.as_ref().and_then(ModuleCache::catalog));
            self.modules
                .state
                .set(cache.as_ref().and_then(ModuleCache::state));
            self.modules.icons.set(BTreeMap::new());
            if let Some(cache) = &cache {
                self.load_cached_icons(cache);
            }
        }
        cache
    }

    fn load_cached_icons(self, cache: &ModuleCache) {
        let Some(catalog) = self.modules.catalog.get() else {
            return;
        };
        let icons = catalog
            .modules
            .iter()
            .filter_map(|module| match cache.icon(&module.id)? {
                CachedIcon::Image(bytes) => Some((module.id.clone(), png_data_uri(&bytes)?)),
                CachedIcon::Missing => None,
            })
            .collect();
        if self.modules.icons.with(|shown| *shown != icons) {
            self.modules.icons.set(icons);
        }
    }

    /// Open the Modules page over the camera tab.
    pub fn open_modules_page(self) {
        self.set_tab(Tab::Camera);
        if !self.modules_page_open() {
            self.push(Route::Modules);
        }
        self.open_modules();
    }

    /// Where the Modules page is on the camera tab's stack, when it is open.
    fn modules_page_depth(self) -> Option<usize> {
        (0..MAX_DEPTH).find(|depth| self.route_at(Tab::Camera, *depth) == Some(Route::Modules))
    }

    pub(super) fn modules_page_open(self) -> bool {
        self.modules_page_depth().is_some()
    }

    /// Close the Modules page and the pages above it.
    pub(super) fn close_modules_page(self) {
        if let Some(depth) = self.modules_page_depth() {
            self.pop_to(Tab::Camera, depth);
        }
    }

    /// The Modules page became visible.
    pub fn open_modules(self) {
        if self.demo() {
            self.refresh_modules();
        } else if self.camera_online() {
            if !self.modules.loading.get() {
                self.refresh_modules();
            }
            if self.looks_snapshot().is_none() {
                self.refresh_looks();
            }
            if self.recipes_snapshot().is_none() {
                self.refresh_recipes();
            }
        } else {
            self.adopt_module_cache();
        }
    }

    pub fn refresh_modules(self) {
        // The GRKit settings come and go with the modules: on a connect, a
        // change on the camera, and a pull.
        self.refresh_camera_settings();
        if self.demo() {
            self.refresh_demo_modules();
            return;
        }
        if !self.camera_online() || self.modules.loading.get() {
            return;
        }
        let cache = self.adopt_module_cache();
        let language = app_language();
        let generation = self.connect_generation.get();
        let changes = self.modules.changes.with(|count| *count);
        self.modules.loading.set(true);
        self.root.run(|| {
            spawn(async move {
                let result = read_modules(language, cache.clone()).await;
                self.modules.loading.set(false);
                if self.connect_generation.get() != generation {
                    // A refresh for the new connection returned while this
                    // one loaded.
                    self.refresh_modules();
                    return;
                }
                match result {
                    Ok(ModulesRead::Blocked(block)) => {
                        self.modules.block.set(Some(block));
                        self.modules.writable.set(false);
                        self.modules.error.set(None);
                    }
                    Ok(ModulesRead::Ready {
                        catalog,
                        state,
                        boot_id,
                        write,
                    }) => {
                        self.modules.block.set(None);
                        self.modules.error.set(None);
                        self.modules.boot_id.set(Some(boot_id));
                        self.modules.writable.set(write);
                        self.modules.catalog.set(Some(catalog));
                        // A change made during the read has the newer state.
                        if self.modules.changes.with(|count| *count) == changes
                            && self.modules.busy.with(Option::is_none)
                        {
                            if let Some(cache) = &cache {
                                cache.save_state(&state);
                            }
                            self.set_module_state(state);
                        }
                        if let Some(cache) = &cache {
                            self.load_cached_icons(cache);
                            self.fetch_module_icons(cache, generation).await;
                        }
                    }
                    Err(error) => {
                        log::warn!("Could not read modules: {error:?}");
                        self.modules.error.set(Some(error));
                    }
                }
            });
        });
    }

    /// Fetch the icons the cache does not hold, one at a time. A module the
    /// camera had no icon for is asked again: a camera can answer so while
    /// its runtime still starts.
    async fn fetch_module_icons(self, cache: &ModuleCache, generation: u64) {
        let Some(catalog) = self.modules.catalog.get() else {
            return;
        };
        for module in &catalog.modules {
            if matches!(cache.icon(&module.id), Some(CachedIcon::Image(_))) {
                continue;
            }
            let icon = match remote::icon(&module.id).await {
                Ok(icon) => icon,
                Err(LookError::ConnectionLost) => return,
                // Not recorded: a later read tries again.
                Err(_) => continue,
            };
            if self.connect_generation.get() != generation {
                return;
            }
            let uri = icon.as_deref().and_then(png_data_uri);
            cache.save_icon(
                &module.id,
                uri.is_some().then_some(icon.as_deref()).flatten(),
            );
            if let Some(uri) = uri {
                self.modules.icons.update(|icons| {
                    icons.insert(module.id.clone(), uri);
                });
            }
        }
    }

    fn refresh_demo_modules(self) {
        if !self.camera_online() {
            return;
        }
        if self.modules.catalog.with(Option::is_none) {
            self.modules
                .catalog
                .set(Some(fake_modules::demo_catalog(app_language())));
        }
        if self.modules.state.with(Option::is_none) {
            self.modules.state.set(Some(fake_modules::demo_state()));
        }
        self.modules.writable.set(self.camera_online());
    }

    /// A read that a lost link cut off never reports back. The catalog and
    /// the state stay, for the page without a camera.
    pub(super) fn reset_module_read(self) {
        self.reset_camera_settings();
        self.modules.loading.set(false);
        self.modules.error.set(None);
        self.modules.busy.set(None);
        self.modules.block.set(None);
        self.modules.writable.set(false);
        self.modules.boot_id.set(None);
        self.modules.editing.set(false);
        // A change the lost link cut off: the next read says what the camera
        // kept.
        if let Some(before) = self.modules.rollback.with(Option::take) {
            self.modules.state.set(Some(before));
        }
    }

    /// Present a failed change as an alert.
    pub fn alert_module_error(self, error: LookError) {
        if error == LookError::ConnectionLost {
            self.show_toast(tr("looks.lost_toast", &[]));
            if let Some(depth) = self.modules_page_depth() {
                self.pop_to(Tab::Camera, depth + 1);
            }
            return;
        }
        let (title, body) = error.keys();
        let body = match &error {
            LookError::Service(detail) => format!("{} {detail}", tr(body, &[])),
            _ => tr(body, &[]),
        };
        spawn(async move {
            let _ = alert_dialog()
                .title(tr(title, &[]))
                .description(body)
                .cancel("ok", tr("looks.ok", &[]))
                .show()
                .await;
        });
    }

    /// Send one change. `shown` is the state to display until the camera
    /// answers, for a change that shows at once.
    fn send_module_change(self, work: ModuleWork, change: Change, shown: Option<ModulesState>) {
        if self.modules.busy.with(Option::is_some) || !self.modules_writable() {
            return;
        }
        let Some(before) = self.modules.state.get() else {
            return;
        };
        self.modules.busy.set(Some(work));
        self.modules.changes.with(|count| *count += 1);
        if let Some(shown) = shown {
            let saved = before.clone();
            self.modules.rollback.with(|slot| *slot = Some(saved));
            self.modules.state.set(Some(shown));
        }
        if self.demo() {
            let mut next = before.clone();
            let fake = match &change {
                Change::Patch(_, _, fake) => fake.clone(),
                Change::Order(ids) => FakeChange::Order(ids.clone()),
                Change::Layout(layout) => FakeChange::Layout(layout.clone()),
            };
            let answer = fake_modules::apply(&mut next, &fake).map(|()| next);
            self.finish_module_change(before, answer);
            return;
        }
        let boot_id = self.modules.boot_id.get().unwrap_or_default();
        let generation = self.connect_generation.get();
        self.root.run(|| {
            spawn(async move {
                let answer = match change {
                    Change::Patch(id, body, _) => remote::patch(&boot_id, &id, body)
                        .await
                        .map(|(module, revision)| before.with_module(module, revision)),
                    Change::Order(ids) => remote::put_order(&boot_id, &ids).await,
                    Change::Layout(layout) => remote::put_layout(&boot_id, &layout).await,
                };
                if self.connect_generation.get() != generation {
                    return;
                }
                self.finish_module_change(before, answer);
            });
        });
    }

    fn finish_module_change(self, before: ModulesState, answer: Result<ModulesState, LookError>) {
        self.modules.busy.set(None);
        self.modules.rollback.with(|slot| *slot = None);
        let (state, error) = settle(before, answer);
        if error.is_none()
            && let Some((camera, language)) = self.modules.owner.get()
            && let Some(cache) = ModuleCache::open(&camera, &language)
            && !self.demo()
        {
            cache.save_state(&state);
        }
        let done = error.is_none();
        self.set_module_state(state);
        if let Some(error) = error {
            if !self.demo() && error != LookError::ConnectionLost {
                self.refresh_modules();
            }
            self.alert_module_error(error);
        } else if done {
            // A change of Enable can start or stop a feature with its tab.
            self.refresh_module_state();
        }
    }

    /// Film Look runs, so the Looks tab shows. Before the first read, or on
    /// a camera that does not report it, the tab shows.
    pub fn looks_available(self) -> bool {
        self.modules
            .state
            .with(|s| s.as_ref().and_then(|s| s.features))
            .is_none_or(|features| features.looks)
    }

    pub fn recipes_available(self) -> bool {
        self.modules
            .state
            .with(|s| s.as_ref().and_then(|s| s.features))
            .is_none_or(|features| features.recipes)
    }

    /// The boot that the module reads came from, for a change to name.
    pub(super) fn modules_boot_id(self) -> String {
        self.modules.boot_id.get().unwrap_or_default()
    }

    pub fn tab_available(self, tab: Tab) -> bool {
        match tab {
            Tab::Camera | Tab::Settings => true,
            Tab::Looks => self.tabs_shown() && self.looks_available(),
            Tab::Recipes => self.tabs_shown() && self.recipes_available(),
        }
    }

    /// The GRKit switch or the mode dial stops every module.
    pub fn grkit_paused(self) -> bool {
        self.modules
            .state
            .with(|s| s.as_ref().is_some_and(|s| !s.grkit_enabled))
    }

    pub fn safe_mode(self) -> bool {
        self.modules.state.with(|s| {
            s.as_ref()
                .is_some_and(|s| s.modules.iter().any(|m| m.status == "safe"))
        })
    }

    /// Keep a state. A feature that stopped closes its tab, and one that
    /// started is read.
    fn set_module_state(self, state: ModulesState) {
        let before = self
            .modules
            .state
            .with(|s| s.as_ref().and_then(|s| s.features));
        let after = state.features;
        self.modules.state.set(Some(state));
        if before == after {
            return;
        }
        let looks_were_on = before.is_none_or(|features| features.looks);
        let recipes_were_on = before.is_none_or(|features| features.recipes);
        if self.looks_available() {
            if !looks_were_on {
                self.refresh_looks();
            }
        } else if self.tab() == Tab::Looks {
            self.leave_stopped_tab(Tab::Looks, "modules.looks_off");
        }
        if self.recipes_available() {
            if !recipes_were_on {
                self.refresh_recipes();
            }
        } else if self.tab() == Tab::Recipes {
            self.leave_stopped_tab(Tab::Recipes, "modules.recipes_off");
        }
    }

    fn leave_stopped_tab(self, tab: Tab, message: &str) {
        self.pop_to(tab, 0);
        self.set_tab(Tab::Camera);
        self.show_toast(tr(message, &[]));
    }

    /// Read what runs on the camera, without its catalog: the tabs follow it
    /// from the first screen on.
    pub(super) fn refresh_module_state(self) {
        // The Settings tab shows the GRKit settings whatever tab is open at a
        // connect.
        self.refresh_camera_settings();
        if self.demo() || !self.camera_online() || self.modules.loading.get() {
            return;
        }
        let cache = self.adopt_module_cache();
        let generation = self.connect_generation.get();
        let changes = self.modules.changes.with(|count| *count);
        self.root.run(|| {
            spawn(async move {
                // A failed read says so on the home row instead of reading
                // on there.
                let failed = move || {
                    if self.connect_generation.get() == generation {
                        self.modules.error.set(Some(LookError::ConnectionLost));
                    }
                };
                let Ok(access) = remote::access().await else {
                    return failed();
                };
                if !access.read {
                    return;
                }
                let Ok(state) = remote::state().await else {
                    return failed();
                };
                if self.connect_generation.get() != generation {
                    return;
                }
                self.modules.error.set(None);
                self.modules.boot_id.set(Some(access.boot_id));
                self.modules.writable.set(access.write);
                if self.modules.changes.with(|count| *count) == changes
                    && self.modules.busy.with(Option::is_none)
                {
                    if let Some(cache) = &cache {
                        cache.save_state(&state);
                    }
                    self.set_module_state(state);
                }
                // The home row and the way to turn a feature on name the
                // modules, so a camera without a cached catalog reads it.
                if self.modules.catalog.with(Option::is_none) {
                    self.refresh_modules();
                }
            });
        });
    }

    /// The Film Look module, while its Enable is off.
    pub fn film_look_off(self) -> Option<String> {
        self.module_rows()?
            .into_iter()
            .find(|row| row.managed() == Some(Managed::Looks) && !row.state.enabled)
            .map(|row| row.id().to_owned())
    }

    pub fn module_draft(self) -> ModuleDraft {
        self.modules.draft.get()
    }

    fn change_module_draft(self, change: impl FnOnce(&mut ModuleDraft)) {
        let mut draft = self.modules.draft.get();
        change(&mut draft);
        if !self.demo()
            && let Some((camera, _)) = self.modules.owner.get()
        {
            remote::save_draft(&camera, &draft);
        }
        self.modules.draft.set(draft);
    }

    pub fn toggle_module_add(self, id: &str) {
        self.change_module_draft(|draft| draft.toggle_add(id));
    }

    pub fn toggle_module_remove(self, id: &str) {
        self.change_module_draft(|draft| draft.toggle_remove(id));
    }

    pub(super) fn clear_module_draft(self) {
        self.change_module_draft(|draft| *draft = ModuleDraft::default());
    }

    /// Modules can be added and removed: the camera is connected and no
    /// update waits or runs.
    pub fn modules_manageable(self) -> bool {
        self.camera_online() && self.camera_journal().is_none() && !self.update_busy.get()
    }

    fn package_module_ids(self) -> Vec<String> {
        self.modules.state.with(|state| {
            state.as_ref().map_or_else(Vec::new, |state| {
                state
                    .modules
                    .iter()
                    .filter(|module| !module.custom)
                    .map(|module| module.id.clone())
                    .collect()
            })
        })
    }

    /// The package modules that the next update installs, when the person
    /// changed them.
    pub(super) fn drafted_modules(self) -> Vec<String> {
        let draft = self.modules.draft.get();
        if draft.is_empty() {
            Vec::new()
        } else {
            draft.after(&self.package_module_ids())
        }
    }

    /// The changes would leave nothing that the camera can show.
    pub fn module_draft_blocked(self) -> bool {
        let draft = self.modules.draft.get();
        !draft.is_empty() && !package_can_install(&draft.after(&self.package_module_ids()))
    }

    fn release_modules(self) -> Vec<(String, grkit_core::release_index::ModuleInfo)> {
        // The package names the release, which a read of the site replaces.
        let _ = self.package.get();
        self.release.with(|slot| {
            slot.borrow()
                .as_ref()
                .map_or_else(Vec::new, |release| release.official_modules())
        })
    }

    /// The name of a module: from the camera, else from the release.
    pub(crate) fn module_title(self, id: &str) -> String {
        if let Some(title) = self.modules.catalog.with(|catalog| {
            catalog
                .as_ref()?
                .modules
                .iter()
                .find(|module| module.id == id)
                .map(|module| module.title.clone())
        }) && !title.is_empty()
        {
            return title;
        }
        self.release_modules()
            .into_iter()
            .find(|(module, _)| module == id)
            .and_then(|(_, info)| {
                info.titles
                    .get(app_language())
                    .or_else(|| info.titles.get("en"))
                    .cloned()
            })
            .unwrap_or_else(|| id.to_owned())
    }

    /// The official modules that this camera can use and does not have, with
    /// their names.
    pub fn installable_modules(self) -> Vec<(String, String)> {
        let (installed, monochrome): (Vec<String>, bool) = self.modules.state.with(|state| {
            state.as_ref().map_or((Vec::new(), false), |state| {
                (
                    state.modules.iter().map(|m| m.id.clone()).collect(),
                    state.monochrome,
                )
            })
        });
        self.release_modules()
            .into_iter()
            .filter(|(id, info)| !installed.contains(id) && !(monochrome && info.color_only))
            .map(|(id, _)| {
                let title = self.module_title(&id);
                (id, title)
            })
            .collect()
    }

    /// The changes of the draft: the ID, the name, and whether it is added.
    pub fn module_changes(self) -> Vec<(String, String, bool)> {
        let draft = self.modules.draft.get();
        draft
            .add
            .iter()
            .map(|id| (id.clone(), self.module_title(id), true))
            .chain(
                draft
                    .remove
                    .iter()
                    .map(|id| (id.clone(), self.module_title(id), false)),
            )
            .collect()
    }

    pub fn set_module_enabled(self, id: &str, enabled: bool) {
        self.send_module_change(
            (id.to_owned(), ModulePart::Enabled),
            Change::Patch(
                id.to_owned(),
                serde_json::json!({ "enabled": enabled }),
                FakeChange::Enabled(id.to_owned(), enabled),
            ),
            None,
        );
    }

    pub fn set_module_value(self, id: &str, key: &str, value: i64) {
        self.send_module_change(
            (id.to_owned(), ModulePart::Setting(key.to_owned())),
            Change::Patch(
                id.to_owned(),
                serde_json::json!({ "values": { key: value } }),
                FakeChange::Value(id.to_owned(), key.to_owned(), value),
            ),
            None,
        );
    }

    /// Show or hide one value in ADJ. The last shown value stays.
    pub fn toggle_module_option(self, id: &str, value: i64) {
        let Some(row) = self.module_row(id) else {
            return;
        };
        let Some(options) = row.choices() else { return };
        let Some(next) = options.choices.toggled(value) else {
            return;
        };
        self.save_module_options(id, next, options.ordered, ModulePart::Option(value), false);
    }

    /// Move a shown value, both places counted among the shown values.
    pub fn move_module_option(self, id: &str, from: usize, to: usize) {
        let Some(options) = self.module_row(id).and_then(|row| row.choices()) else {
            return;
        };
        if from == to || !options.ordered {
            return;
        }
        self.save_module_options(
            id,
            options.choices.moved(from, to),
            true,
            ModulePart::Options,
            true,
        );
    }

    fn save_module_options(
        self,
        id: &str,
        choices: crate::domain::modules::OptionChoices,
        ordered: bool,
        part: ModulePart,
        show_at_once: bool,
    ) {
        let Some(state) = self.modules.state.get() else {
            return;
        };
        let order = ordered.then(|| choices.order());
        let mut body = serde_json::json!({ "hiddenValues": choices.hidden });
        if let Some(order) = &order {
            body["valueOrder"] = serde_json::json!(order);
        }
        let shown = show_at_once.then(|| state.with_choices(id, &choices, ordered));
        self.send_module_change(
            (id.to_owned(), part),
            Change::Patch(
                id.to_owned(),
                body,
                FakeChange::Options(id.to_owned(), choices.hidden.clone(), order),
            ),
            shown,
        );
    }

    /// Move an orderable module, both places counted among the orderable
    /// modules.
    pub fn move_module(self, from: usize, to: usize) {
        let Some(state) = self.modules.state.get() else {
            return;
        };
        if from == to {
            return;
        }
        let ids = state.moved_ids(from, to);
        let shown = state.with_order(&ids);
        self.send_module_change(
            (String::new(), ModulePart::Order),
            Change::Order(ids),
            Some(shown),
        );
    }

    /// The layout of ADJ, from a GRKit that takes a whole layout.
    pub fn module_layout(self) -> Option<ModuleLayout> {
        self.modules
            .state
            .with(|s| s.as_ref().and_then(|s| s.layout()))
    }

    pub fn toolbar_max(self) -> usize {
        self.modules
            .state
            .with(|s| s.as_ref().and_then(|s| s.toolbar_max))
            .unwrap_or(0)
    }

    /// Move a module within its place of ADJ.
    pub fn move_in_place(self, place: ModulePlace, from: usize, to: usize) {
        if let Some(layout) = self.module_layout().filter(|_| from != to) {
            self.save_layout(layout.moved(place, from, to));
        }
    }

    /// Move a module to the end of another place of ADJ.
    pub fn move_to_place(self, id: &str, place: ModulePlace) {
        if let Some(layout) = self.module_layout() {
            self.save_layout(layout.placed(id, place));
        }
    }

    fn save_layout(self, layout: ModuleLayout) {
        let Some(state) = self.modules.state.get() else {
            return;
        };
        let shown = state.with_layout(&layout);
        self.send_module_change(
            (String::new(), ModulePart::Order),
            Change::Layout(layout),
            Some(shown),
        );
    }

    /// Demo screens: show a state of the tab.
    pub(crate) fn seed_demo_modules(
        self,
        edit: impl FnOnce(&mut ModulesState),
        block: Option<ModulesBlock>,
    ) {
        let mut state = fake_modules::demo_state();
        edit(&mut state);
        self.modules
            .catalog
            .set(Some(fake_modules::demo_catalog(app_language())));
        self.modules.state.set(Some(state));
        self.modules.block.set(block);
    }

    /// Demo screens: a read that has not answered yet.
    pub(crate) fn seed_demo_modules_loading(self) {
        self.modules.catalog.set(None);
        self.modules.state.set(None);
        self.modules.loading.set(true);
    }

    /// Demo screens: a row whose change is being saved.
    pub(crate) fn seed_demo_module_saving(self, id: &str, part: ModulePart) {
        self.modules.busy.set(Some((id.to_owned(), part)));
    }
}

enum ModulesRead {
    Blocked(ModulesBlock),
    Ready {
        catalog: ModuleCatalog,
        state: ModulesState,
        boot_id: String,
        write: bool,
    },
}

/// Read what the camera can do, its catalog (through the cache), and its
/// state.
async fn read_modules(
    language: &'static str,
    cache: Option<ModuleCache>,
) -> Result<ModulesRead, LookError> {
    let access = remote::access().await?;
    if !access.read {
        return Ok(ModulesRead::Blocked(ModulesBlock::Unsupported));
    }
    let cached = cache.as_ref().and_then(ModuleCache::catalog);
    let catalog = match remote::catalog(language, cached).await {
        Ok(CatalogAnswer::Fresh(catalog, bytes)) => {
            if let Some(cache) = &cache {
                cache.save_catalog(&bytes);
            }
            catalog
        }
        Ok(CatalogAnswer::Cached(catalog)) => catalog,
        Ok(CatalogAnswer::TooNew) => return Ok(ModulesRead::Blocked(ModulesBlock::AppTooOld)),
        // The capability said yes, but this GRKit has no such route.
        Err(LookError::NotFound) => return Ok(ModulesRead::Blocked(ModulesBlock::Unsupported)),
        Err(error) => return Err(error),
    };
    let state = remote::state().await?;
    Ok(ModulesRead::Ready {
        catalog,
        state,
        boot_id: access.boot_id,
        write: access.write,
    })
}
