//! Recipes of the connected camera: the list, changes, apply with undo, and
//! the presets of the release index. Each change answers with the list as
//! the camera has it after the change.

use nsh_core::prelude::*;

use grkit_core::recipes::Family;
use grkit_core::release_index::IndexPreset;

use super::{Alert, AppState, Route, Tab};
use crate::domain::{
    ConnectionState, LookError,
    recipes::{PresetItem, RecipeChange, RecipeError, RecipeItem, RecipesSnapshot, family_of},
};
use crate::strings::tr;

/// What a change to the recipes does, so that only its own control shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeAction {
    Apply,
    Toggle,
    Rename,
    Delete,
    Undo,
    Add,
    Order,
}

/// The key of the recipe (or `order`, `undo`, a preset ID) being changed,
/// and how.
type RecipeWork = (String, RecipeAction);

/// How long an Undo runs before a notice says so.
const UNDO_NOTICE_DELAY: std::time::Duration = std::time::Duration::from_millis(300);

/// Recipes state, kept in the app state (`Copy` handles).
#[derive(Clone, Copy)]
pub struct RecipesStore {
    snapshot: Signal<Option<RecipesSnapshot>>,
    /// The peripheral ID of the camera the snapshot is from.
    owner: Signal<Option<String>>,
    loading: Signal<bool>,
    error: Signal<Option<RecipeError>>,
    /// The change under way.
    busy: Signal<Option<RecipeWork>>,
    /// The presets of the release index.
    pub(crate) presets: Signal<Vec<IndexPreset>>,
}

impl RecipesStore {
    pub fn new() -> Self {
        Self {
            snapshot: signal(None),
            owner: signal(None),
            loading: signal(false),
            error: signal(None),
            busy: signal(None),
            presets: signal(Vec::new()),
        }
    }
}

impl Default for RecipesStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn recipes_snapshot(self) -> Option<RecipesSnapshot> {
        self.recipes.snapshot.get()
    }

    /// The recipes of the saved camera as the phone last read them, while
    /// no camera is connected.
    pub fn offline_recipes(self) -> Option<RecipesSnapshot> {
        if self.camera_online() {
            return None;
        }
        let camera = self.remembered()?;
        let owner = self.recipes.owner.get()?;
        owner
            .eq_ignore_ascii_case(&camera.peripheral_id)
            .then(|| self.recipes.snapshot.get())
            .flatten()
    }

    pub fn recipes_error(self) -> Option<RecipeError> {
        self.recipes.error.get()
    }

    pub fn recipe(self, id: u32) -> Option<RecipeItem> {
        self.recipes.snapshot.get().and_then(|s| s.get(id).cloned())
    }

    pub fn recipe_busy(self) -> Option<String> {
        self.recipes.busy.get().map(|(key, _)| key)
    }

    /// `action` is under way on the recipe or preset `key`.
    pub fn recipe_working(self, key: &str, action: RecipeAction) -> bool {
        self.recipes
            .busy
            .with(|busy| matches!(busy, Some((k, a)) if k == key && *a == action))
    }

    /// A change is under way on the recipe `id`.
    pub fn recipe_row_busy(self, id: u32) -> bool {
        let key = id.to_string();
        self.recipes
            .busy
            .with(|busy| matches!(busy, Some((k, _)) if *k == key))
    }

    pub fn recipes_loading(self) -> bool {
        self.recipes.loading.get()
    }

    /// The list is being read, or an order or Undo is being saved.
    pub fn recipes_refreshing(self) -> bool {
        self.recipes.loading.get()
            || self.recipe_working("order", RecipeAction::Order)
            || self.recipe_working("undo", RecipeAction::Undo)
    }

    /// A read that a lost link cut off never reports back.
    pub(super) fn reset_recipe_read(self) {
        self.recipes.loading.set(false);
        self.recipes.error.set(None);
    }

    /// Take the one change slot, unless a change holds it.
    fn claim_recipe(self, key: &str, action: RecipeAction) -> bool {
        if self.recipes.busy.with(Option::is_some) {
            return false;
        }
        self.recipes.busy.set(Some((key.to_owned(), action)));
        true
    }

    fn release_recipe(self, work: &RecipeWork) {
        if self.recipes.busy.with(|busy| busy.as_ref() == Some(work)) {
            self.recipes.busy.set(None);
        }
    }

    /// The recipe family of the connected camera.
    pub fn recipe_family(self) -> Option<Family> {
        self.status.get().map(|s| family_of(s.camera.model))
    }

    /// The presets for the connected camera.
    pub fn recipe_presets(self) -> Vec<PresetItem> {
        let family = self.recipe_family().unwrap_or(Family::Gr3);
        PresetItem::for_family(&self.recipes.presets.get(), family)
    }

    /// The name of a recipe as the camera shows it.
    pub fn recipe_name(self, item: &RecipeItem) -> String {
        let missing = if self.looks_available() {
            "recipes.look_missing"
        } else {
            "recipes.needs_look"
        };
        item.display_name(native_type_name, &tr(missing, &[]))
    }

    fn recipes_connected(self) -> bool {
        self.status
            .get()
            .is_some_and(|s| s.camera.connection == ConnectionState::Connected)
    }

    /// The Recipes tab became visible: read the recipes, and the Looks for
    /// their covers.
    pub fn open_recipes(self) {
        if self.recipes_connected() && !self.recipes.loading.get() {
            self.refresh_recipes();
            if self.looks_snapshot().is_none() {
                self.refresh_looks();
            }
        }
    }

    pub fn refresh_recipes(self) {
        if !self.recipes_connected() {
            return;
        }
        if !self.recipes_available() {
            self.recipes.error.set(None);
            return;
        }
        let generation = self.connect_generation.get();
        self.recipes.loading.set(true);
        let list = self.api().recipes();
        self.run(async move {
            let result = list.await;
            if self.connect_generation.get() != generation {
                return;
            }
            self.recipes.loading.set(false);
            match result {
                Ok(snapshot) => {
                    self.recipes.snapshot.set(Some(snapshot));
                    self.recipes
                        .owner
                        .set(self.remembered().map(|c| c.peripheral_id));
                    self.recipes.error.set(None);
                }
                Err(error) => {
                    log::warn!("Could not read recipes: {error:?}");
                    self.recipes.error.set(Some(error));
                }
            }
        });
    }

    /// Report a failed change: an alert, or for a lost link a notice.
    pub fn alert_recipe_error(self, error: RecipeError) {
        if error == RecipeError::Look(LookError::ConnectionLost) {
            self.clear_name_draft();
            self.show_toast(tr("looks.lost_toast", &[]));
            self.pop_to(Tab::Recipes, 0);
            return;
        }
        self.alert(Alert::Recipe(error));
    }

    /// Send one change and take the list it answers with.
    fn recipe_change(
        self,
        key: String,
        action: RecipeAction,
        change: RecipeChange,
        done: impl FnOnce(AppState) + 'static,
    ) {
        self.recipe_change_or(key, action, change, done, |_| {});
    }

    /// [`Self::recipe_change`] with `failed` for a change that was refused
    /// or did not reach the camera.
    fn recipe_change_or(
        self,
        key: String,
        action: RecipeAction,
        change: RecipeChange,
        done: impl FnOnce(AppState) + 'static,
        failed: impl FnOnce(AppState) + 'static,
    ) {
        if !self.claim_recipe(&key, action) {
            failed(self);
            return;
        }
        self.run_recipe_change((key, action), change, done, failed);
    }

    /// Run a change that holds the slot, and give the slot back.
    fn run_recipe_change(
        self,
        work: RecipeWork,
        change: RecipeChange,
        done: impl FnOnce(AppState) + 'static,
        failed: impl FnOnce(AppState) + 'static,
    ) {
        let Some(snapshot) = self.recipes.snapshot.get() else {
            self.release_recipe(&work);
            failed(self);
            return;
        };
        let answer = self.api().change_recipe(&snapshot.boot_id, change);
        self.run(async move {
            match answer.await {
                Ok(snapshot) => {
                    // One update, so a switch moves no more than once.
                    batch(|| {
                        self.release_recipe(&work);
                        self.recipes.snapshot.set(Some(snapshot));
                    });
                    done(self);
                }
                Err(error) => {
                    self.release_recipe(&work);
                    failed(self);
                    self.refresh_recipes();
                    self.alert_recipe_error(error);
                }
            }
        });
    }

    pub fn set_recipe_enabled(self, id: u32, enabled: bool) {
        self.recipe_change(
            id.to_string(),
            RecipeAction::Toggle,
            RecipeChange::Enabled(id, enabled),
            |_| {},
        );
    }

    pub fn rename_recipe(self, id: u32, name: String) {
        let name = name.trim().to_owned();
        if !name.is_empty() && !crate::domain::valid_name(&name) {
            self.alert_recipe_error(RecipeError::Look(LookError::InvalidName));
            return;
        }
        let name = (!name.is_empty()).then_some(name);
        self.recipe_change(
            id.to_string(),
            RecipeAction::Rename,
            RecipeChange::Rename(id, name),
            |app| app.clear_name_draft(),
        );
    }

    pub fn delete_recipe(self, id: u32) {
        self.recipe_change(
            id.to_string(),
            RecipeAction::Delete,
            RecipeChange::Delete(id),
            |app| app.pop_to(Tab::Recipes, 0),
        );
    }

    /// Move the recipe that is on at `from` to `to`, both counted among the
    /// ones that are on. The list shows the new order while the camera takes
    /// it.
    pub fn move_recipe(self, from: usize, to: usize) {
        let Some(snapshot) = self.recipes.snapshot.get() else {
            return;
        };
        // While another change runs the camera would not take this order.
        if from == to || self.recipes.busy.with(Option::is_some) {
            return;
        }
        let saved: Vec<u32> = snapshot.recipes.iter().map(|r| r.id).collect();
        let ids = snapshot.moved(from, to);
        self.recipes.snapshot.update(|s| {
            if let Some(s) = s {
                s.recipes
                    .sort_by_key(|r| ids.iter().position(|id| *id == r.id).unwrap_or(usize::MAX));
            }
        });
        self.recipe_change_or(
            "order".into(),
            RecipeAction::Order,
            RecipeChange::Order(ids),
            |_| {},
            move |app| {
                // The camera kept its order, also when it cannot be read now.
                app.recipes.snapshot.update(|s| {
                    if let Some(s) = s {
                        s.recipes.sort_by_key(|r| {
                            saved
                                .iter()
                                .position(|id| *id == r.id)
                                .unwrap_or(usize::MAX)
                        });
                    }
                });
            },
        );
    }

    /// Apply a recipe to the camera, with Undo in the notice.
    pub fn apply_recipe(self, id: u32) {
        let Some(item) = self.recipe(id) else { return };
        let name = self.recipe_name(&item);
        self.recipe_change(
            id.to_string(),
            RecipeAction::Apply,
            RecipeChange::Apply(id),
            move |app| {
                let skipped: Vec<String> = app
                    .recipes_snapshot()
                    .map(|snapshot| snapshot.skipped)
                    .unwrap_or_default()
                    .iter()
                    .map(|id| app.module_title(id))
                    .collect();
                let message = if skipped.is_empty() {
                    tr("recipes.applied", &[("name", &name)])
                } else {
                    tr(
                        "recipes.applied_partial",
                        &[("name", &name), ("modules", &skipped.join("、"))],
                    )
                };
                app.show_toast_action(message, tr("recipes.undo", &[]), move || app.undo_recipe());
            },
        );
    }

    pub fn undo_recipe(self) {
        self.recipe_change(
            "undo".into(),
            RecipeAction::Undo,
            RecipeChange::Undo,
            |app| app.show_toast(tr("recipes.undone", &[])),
        );
        // The notice that started the Undo has closed.
        if !self.recipe_working("undo", RecipeAction::Undo) {
            return;
        }
        self.root.run(|| {
            spawn(async move {
                timer(UNDO_NOTICE_DELAY).await;
                if self.recipe_working("undo", RecipeAction::Undo) {
                    self.show_toast(tr("recipes.undoing", &[]));
                }
            });
        });
    }

    /// Add a preset; its official Look first when the camera does not have
    /// it.
    pub fn add_preset(self, id: String) {
        let Some(preset) = self.recipe_presets().into_iter().find(|p| p.id == id) else {
            return;
        };
        if let Some(problem) = self
            .recipes
            .snapshot
            .get()
            .and_then(|s| (s.recipes.len() >= s.max_listed).then_some(RecipeError::CardFull))
        {
            self.alert_recipe_error(problem);
            return;
        }
        if !self.claim_recipe(&id, RecipeAction::Add) {
            return;
        }
        let work = (id.clone(), RecipeAction::Add);
        self.root.run(|| {
            spawn(async move {
                if let Some(effect) = preset.effect.as_deref()
                    && let Err(error) = self.ensure_official_look(effect).await
                {
                    self.release_recipe(&work);
                    self.alert_recipe_error(RecipeError::Look(error));
                    return;
                }
                let name = preset.recipe.name.clone();
                self.run_recipe_change(
                    work,
                    RecipeChange::Add(id),
                    move |app| app.show_toast(tr("recipes.added", &[("name", &name)])),
                    |_| {},
                );
            });
        });
    }

    /// The preset is on the camera: a recipe with its name and settings.
    pub fn preset_added(self, preset: &PresetItem) -> bool {
        self.recipes.snapshot.get().is_some_and(|s| {
            s.recipes.iter().any(|r| {
                r.name.as_deref() == Some(preset.recipe.name.as_str())
                    && r.recipe.look == preset.recipe.look
            })
        })
    }

    pub fn open_recipe_presets(self) {
        // The catalog names the Looks that presets add.
        self.load_catalog();
        if self.recipes.presets.get().is_empty() {
            self.refresh_update_info();
        }
        self.push(Route::RecipePresets);
    }
}

/// The name of an Image Control type in the app language.
pub fn native_type_name(image_type: usize) -> String {
    tr(&format!("recipes.type.{image_type}"), &[])
}

/// Demo screens (`dev::screens`).
#[cfg(demo)]
impl AppState {
    pub(crate) fn seed_recipes_failed(self) {
        self.recipes.snapshot.set(None);
        self.recipes.loading.set(false);
        self.recipes
            .error
            .set(Some(RecipeError::Look(LookError::ConnectionLost)));
    }
}
