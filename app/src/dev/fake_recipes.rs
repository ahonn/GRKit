//! The demo camera's recipes: two on Film Looks, the rest presets. A change
//! follows the camera's rules that the UI depends on.

use grkit_core::recipes::{Family, Look};
use grkit_core::release_index::IndexPreset;

use crate::domain::{
    LookError, LooksSnapshot,
    recipes::{PresetItem, RecipeChange, RecipeError, RecipeItem, RecipesSnapshot},
};

/// The recipes as the camera serves them with `looks`: a recipe whose Film
/// Look is on the card only does not apply (`recipe_look_missing`).
pub fn served(f: &RecipesSnapshot, looks: &LooksSnapshot) -> RecipesSnapshot {
    let mut f = f.clone();
    for recipe in &mut f.recipes {
        if let Some(id) = &recipe.effect_id {
            let stored = looks.stored.iter().any(|look| &look.id == id);
            recipe.effect_stored = stored;
            recipe.ready = !stored;
        }
    }
    f
}

/// Apply a change as the camera does.
pub fn apply(f: &mut RecipesSnapshot, change: &RecipeChange) -> Result<(), RecipeError> {
    let at = |f: &RecipesSnapshot, id: u32| {
        f.recipes
            .iter()
            .position(|r| r.id == id)
            .ok_or(RecipeError::Look(LookError::NotFound))
    };
    match change {
        RecipeChange::Enabled(id, on) => {
            let i = at(f, *id)?;
            if *on && f.on_count() >= f.max_on {
                return Err(RecipeError::MenuFull);
            }
            let mut item = f.recipes.remove(i);
            item.enabled = *on;
            let to = f.on_count();
            f.recipes.insert(to, item);
        }
        RecipeChange::Rename(id, name) => {
            let i = at(f, *id)?;
            f.recipes[i].name = name.clone();
        }
        RecipeChange::Delete(id) => {
            let i = at(f, *id)?;
            f.recipes.remove(i);
        }
        RecipeChange::Order(ids) => {
            f.recipes
                .sort_by_key(|r| ids.iter().position(|id| *id == r.id).unwrap_or(usize::MAX));
        }
        RecipeChange::Apply(id) => {
            at(f, *id)?;
            for r in &mut f.recipes {
                r.current = r.id == *id;
            }
            f.undo = (f.undo + 1).min(5);
        }
        RecipeChange::Undo => {
            if f.undo == 0 {
                return Err(RecipeError::NothingToUndo);
            }
            f.undo -= 1;
            for r in &mut f.recipes {
                r.current = false;
            }
        }
        RecipeChange::Add(preset) => {
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
        effect_stored: false,
        recipe,
    }
}

/// The demo camera's recipes: two on Film Looks, the rest presets.
pub fn demo_recipes() -> RecipesSnapshot {
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
        unreadable: 0,
        unreadable_on: Vec::new(),
        skipped: Vec::new(),
        // Negative Film: the hero shows it while no recipe or Look is in use.
        image_control: Some(6),
    }
}
