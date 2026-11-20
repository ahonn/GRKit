//! Recipes of the connected camera: the list, changes, apply with undo, and
//! the presets of the release index. Each change answers with the list as
//! the camera has it after the change.

use nsh_base::alert_dialog;
use nsh_core::prelude::*;

use grkit_core::recipes::{Family, Look};
use grkit_core::release_index::IndexPreset;

use super::{AppState, Route, Tab};
use crate::domain::{
    ConnectionState, LookError,
    recipes::{PresetItem, RecipeError, RecipeItem, RecipesSnapshot, family_of},
};
use crate::platform::recipes as remote;
use crate::strings::tr;

/// Recipes state, kept in the app state (`Copy` handles).
#[derive(Clone, Copy)]
pub struct RecipesStore {
    snapshot: Signal<Option<RecipesSnapshot>>,
    loading: Signal<bool>,
    error: Signal<Option<RecipeError>>,
    /// The recipe (or `order`, `undo`, a preset ID) being changed.
    busy: Signal<Option<String>>,
    /// The presets of the release index; the demo has the built-in ones.
    pub(crate) presets: Signal<Vec<IndexPreset>>,
    /// The demo camera's recipes.
    fake: Signal<Option<RecipesSnapshot>>,
}

impl RecipesStore {
    pub fn new(demo: bool) -> Self {
        Self {
            snapshot: signal(None),
            loading: signal(false),
            error: signal(None),
            busy: signal(None),
            presets: signal(if demo {
                IndexPreset::all(&Default::default())
            } else {
                Vec::new()
            }),
            fake: signal(demo.then(demo_recipes)),
        }
    }
}

/// What a change does.
enum Change {
    Enabled(u32, bool),
    Rename(u32, Option<String>),
    Delete(u32),
    Order(Vec<u32>),
    Apply(u32),
    Undo,
    Add(String),
}

impl AppState {
    pub fn recipes_snapshot(self) -> Option<RecipesSnapshot> {
        self.recipes.snapshot.get()
    }

    pub fn recipes_error(self) -> Option<RecipeError> {
        self.recipes.error.get()
    }

    pub fn recipe(self, id: u32) -> Option<RecipeItem> {
        self.recipes.snapshot.get().and_then(|s| s.get(id).cloned())
    }

    pub fn recipe_busy(self) -> Option<String> {
        self.recipes.busy.get()
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
        item.display_name(native_type_name, &tr("recipes.look_missing", &[]))
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
        if self.demo() {
            self.recipes.snapshot.set(self.recipes.fake.get());
            self.recipes.error.set(None);
            return;
        }
        let generation = self.connect_generation.get();
        self.recipes.loading.set(true);
        self.root.run(|| {
            spawn(async move {
                let result = remote::list().await;
                if self.connect_generation.get() != generation {
                    return;
                }
                self.recipes.loading.set(false);
                match result {
                    Ok(snapshot) => {
                        self.recipes.snapshot.set(Some(snapshot));
                        self.recipes.error.set(None);
                    }
                    Err(error) => {
                        log::warn!("Could not read recipes: {error:?}");
                        self.recipes.error.set(Some(error));
                    }
                }
            });
        });
    }

    /// Present a failed change as an alert.
    pub fn alert_recipe_error(self, error: RecipeError) {
        if error == RecipeError::Look(LookError::ConnectionLost) {
            self.show_toast(tr("looks.lost_toast", &[]));
            self.pop_to(Tab::Recipes, 0);
            return;
        }
        let (title, body) = error.keys();
        let limits = self.recipes.snapshot.get();
        let max = match &error {
            RecipeError::MenuFull => limits.map(|s| s.max_on),
            RecipeError::CardFull => limits.map(|s| s.max_listed),
            _ => None,
        }
        .unwrap_or_default()
        .to_string();
        let body = match &error {
            RecipeError::Look(LookError::Service(detail)) => format!("{} {detail}", tr(body, &[])),
            _ => tr(body, &[("max", &max)]),
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

    /// Send one change and take the list it answers with.
    fn recipe_change(self, key: String, change: Change, done: impl FnOnce(AppState) + 'static) {
        if self.recipes.busy.get().is_some() {
            return;
        }
        if self.demo() {
            let mut result = Err(RecipeError::Look(LookError::ConnectionLost));
            self.recipes.fake.update(|fake| {
                if let Some(f) = fake.as_mut() {
                    result = demo_change(f, &change);
                }
            });
            match result {
                Ok(()) => {
                    self.refresh_recipes();
                    done(self);
                }
                Err(error) => self.alert_recipe_error(error),
            }
            return;
        }
        let Some(snapshot) = self.recipes.snapshot.get() else {
            return;
        };
        let boot_id = snapshot.boot_id.clone();
        self.recipes.busy.set(Some(key));
        self.root.run(|| {
            spawn(async move {
                let (method, path, body) = match &change {
                    Change::Enabled(id, on) => (
                        "PATCH",
                        format!("/{id}"),
                        Some(serde_json::json!({"enabled": on})),
                    ),
                    Change::Rename(id, name) => (
                        "PATCH",
                        format!("/{id}"),
                        Some(serde_json::json!({"name": name})),
                    ),
                    Change::Delete(id) => ("DELETE", format!("/{id}"), None),
                    Change::Order(ids) => (
                        "PUT",
                        "/order".to_owned(),
                        Some(serde_json::json!({"ids": ids})),
                    ),
                    Change::Apply(id) => ("POST", format!("/{id}/apply"), None),
                    Change::Undo => ("POST", "/undo".to_owned(), None),
                    Change::Add(preset) => (
                        "POST",
                        String::new(),
                        Some(serde_json::json!({"preset": preset})),
                    ),
                };
                let result = remote::change(&boot_id, method, &path, body).await;
                self.recipes.busy.set(None);
                match result {
                    Ok(snapshot) => {
                        self.recipes.snapshot.set(Some(snapshot));
                        done(self);
                    }
                    Err(error) => {
                        self.refresh_recipes();
                        self.alert_recipe_error(error);
                    }
                }
            });
        });
    }

    pub fn set_recipe_enabled(self, id: u32, enabled: bool) {
        self.recipe_change(id.to_string(), Change::Enabled(id, enabled), |_| {});
    }

    pub fn rename_recipe(self, id: u32, name: String) {
        let name = name.trim().to_owned();
        if !name.is_empty() && !crate::domain::valid_name(&name) {
            self.alert_recipe_error(RecipeError::Look(LookError::InvalidName));
            return;
        }
        let name = (!name.is_empty()).then_some(name);
        self.recipe_change(id.to_string(), Change::Rename(id, name), |_| {});
    }

    pub fn delete_recipe(self, id: u32) {
        self.recipe_change(id.to_string(), Change::Delete(id), |app| {
            app.pop_to(Tab::Recipes, 0)
        });
    }

    /// Move the recipe that is on at `from` to `to`, both counted among the
    /// ones that are on. The list shows the new order while the camera takes
    /// it.
    pub fn move_recipe(self, from: usize, to: usize) {
        let Some(snapshot) = self.recipes.snapshot.get() else {
            return;
        };
        if from == to {
            return;
        }
        let ids = snapshot.moved(from, to);
        self.recipes.snapshot.update(|s| {
            if let Some(s) = s {
                s.recipes
                    .sort_by_key(|r| ids.iter().position(|id| *id == r.id).unwrap_or(usize::MAX));
            }
        });
        self.recipe_change("order".into(), Change::Order(ids), |_| {});
    }

    /// Apply a recipe to the camera, with Undo in the notice.
    pub fn apply_recipe(self, id: u32) {
        let Some(item) = self.recipe(id) else { return };
        let name = self.recipe_name(&item);
        self.recipe_change(id.to_string(), Change::Apply(id), move |app| {
            app.show_toast_action(
                tr("recipes.applied", &[("name", &name)]),
                tr("recipes.undo", &[]),
                move || app.undo_recipe(),
            );
        });
    }

    pub fn undo_recipe(self) {
        self.recipe_change("undo".into(), Change::Undo, |app| {
            app.show_toast(tr("recipes.undone", &[]))
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
        if self.recipes.busy.get().is_some() {
            return;
        }
        self.recipes.busy.set(Some(id.clone()));
        self.root.run(|| {
            spawn(async move {
                if let Some(effect) = preset.effect.as_deref()
                    && let Err(error) = self.ensure_official_look(effect).await
                {
                    self.recipes.busy.set(None);
                    self.alert_recipe_error(RecipeError::Look(error));
                    return;
                }
                self.recipes.busy.set(None);
                let name = preset.recipe.name.clone();
                self.recipe_change(id.clone(), Change::Add(id), move |app| {
                    app.show_toast(tr("recipes.added", &[("name", &name)]))
                });
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

    /// Demo screens: edit the demo camera's recipes.
    pub(crate) fn with_fake_recipes(self, edit: impl FnOnce(&mut RecipesSnapshot)) {
        self.recipes.fake.update(|f| {
            if let Some(f) = f {
                edit(f)
            }
        });
    }
}

/// The short name of an Image Control type, for a tile.
pub fn native_type_short(image_type: usize) -> String {
    tr(&format!("recipes.type_short.{image_type}"), &[])
}

/// The name of an Image Control type in the app language.
pub fn native_type_name(image_type: usize) -> String {
    tr(&format!("recipes.type.{image_type}"), &[])
}

fn demo_change(f: &mut RecipesSnapshot, change: &Change) -> Result<(), RecipeError> {
    let at = |f: &RecipesSnapshot, id: u32| {
        f.recipes
            .iter()
            .position(|r| r.id == id)
            .ok_or(RecipeError::Look(LookError::NotFound))
    };
    match change {
        Change::Enabled(id, on) => {
            let i = at(f, *id)?;
            if *on && f.on_count() >= f.max_on {
                return Err(RecipeError::MenuFull);
            }
            let mut item = f.recipes.remove(i);
            item.enabled = *on;
            let to = f.on_count();
            f.recipes.insert(to, item);
        }
        Change::Rename(id, name) => {
            let i = at(f, *id)?;
            f.recipes[i].name = name.clone();
        }
        Change::Delete(id) => {
            let i = at(f, *id)?;
            f.recipes.remove(i);
        }
        Change::Order(ids) => {
            f.recipes
                .sort_by_key(|r| ids.iter().position(|id| *id == r.id).unwrap_or(usize::MAX));
        }
        Change::Apply(id) => {
            at(f, *id)?;
            for r in &mut f.recipes {
                r.current = r.id == *id;
            }
            f.undo = (f.undo + 1).min(5);
        }
        Change::Undo => {
            if f.undo == 0 {
                return Err(RecipeError::NothingToUndo);
            }
            f.undo -= 1;
            for r in &mut f.recipes {
                r.current = false;
            }
        }
        Change::Add(preset) => {
            if f.recipes.len() >= f.max_listed {
                return Err(RecipeError::CardFull);
            }
            let p = PresetItem::for_family(&IndexPreset::all(&Default::default()), Family::Gr3)
                .into_iter()
                .find(|p| &p.id == preset)
                .ok_or(RecipeError::Look(LookError::NotFound))?;
            let id = f.recipes.iter().map(|r| r.id).max().unwrap_or(0) + 1;
            let enabled = f.on_count() < f.max_on;
            let item = demo_item(
                id,
                Some(p.recipe.name.clone()),
                p.recipe,
                enabled,
                false,
                None,
            );
            let to = if enabled {
                f.on_count()
            } else {
                f.recipes.len()
            };
            f.recipes.insert(to, item);
        }
    }
    Ok(())
}

fn demo_item(
    id: u32,
    name: Option<String>,
    recipe: grkit_core::recipes::Recipe,
    enabled: bool,
    current: bool,
    effect: Option<(&str, &str)>,
) -> RecipeItem {
    RecipeItem {
        id,
        name,
        enabled,
        current,
        ready: !matches!(recipe.look, Look::Effect { .. }) || effect.is_some(),
        effect_id: effect.map(|(id, _)| id.to_owned()),
        effect_name: effect.map(|(_, name)| name.to_owned()),
        recipe,
    }
}

/// The demo camera's recipes: two on Film Looks, the rest presets.
fn demo_recipes() -> RecipesSnapshot {
    let presets = PresetItem::for_family(&IndexPreset::all(&Default::default()), Family::Gr3);
    let find = |id: &str| {
        presets
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.recipe.clone())
    };
    let effect = find("cinestill-800t").expect("preset");
    let mut recipes = vec![
        demo_item(
            1,
            Some("Odd Film".into()),
            effect.clone(),
            true,
            true,
            Some(("portra-400-endura-v1", "Portra 400")),
        ),
        demo_item(
            2,
            Some("Street".into()),
            effect.clone(),
            true,
            false,
            Some(("trix-400-mono-v1", "Tri-X 400")),
        ),
    ];
    let mut numbered = effect;
    numbered.number = 2;
    let mut id = 3;
    for (preset, on) in [("bennet-800", true), ("reggie-negative", true)] {
        let recipe = find(preset).expect("preset");
        recipes.push(demo_item(
            id,
            Some(recipe.name.clone()),
            recipe,
            on,
            false,
            None,
        ));
        id += 1;
    }
    recipes.push(demo_item(
        id,
        None,
        numbered,
        true,
        false,
        Some(("portra-400-endura-v1", "Portra 400")),
    ));
    id += 1;
    for preset in ["dante-sisofo-street-bw", "candy-color-400"] {
        let recipe = find(preset).expect("preset");
        recipes.push(demo_item(
            id,
            Some(recipe.name.clone()),
            recipe,
            false,
            false,
            None,
        ));
        id += 1;
    }
    RecipesSnapshot {
        boot_id: "demo-boot".into(),
        recipes,
        max_on: 40,
        max_listed: 64,
        undo: 0,
    }
}
