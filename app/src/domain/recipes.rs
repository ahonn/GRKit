//! Recipes of the connected camera (`/v1/recipes`) and the presets of the
//! release index. The camera makes recipes with Save Current Settings, so the
//! phone reads their settings and does not edit them.

use std::collections::BTreeMap;

use grkit_core::recipes::{Family, Look, Recipe};
use grkit_core::release_index::IndexPreset;
use serde_json::Value;

use super::LookError;

/// A recipe on the camera.
#[derive(Clone, Debug, PartialEq)]
pub struct RecipeItem {
    pub id: u32,
    /// The name that the person gave, or `None` for the automatic name.
    pub name: Option<String>,
    pub enabled: bool,
    /// The camera settings match the recipe now.
    pub current: bool,
    /// The Look of a recipe on a Film Look, when the camera has it.
    pub effect_id: Option<String>,
    pub effect_name: Option<String>,
    /// The camera can apply the recipe: it has its Look.
    pub ready: bool,
    pub recipe: Recipe,
}

impl RecipeItem {
    /// The name that the camera shows: the given one, else the name of the
    /// look and the number of the recipe on that look.
    pub fn display_name(&self, native_label: impl Fn(usize) -> String, missing: &str) -> String {
        if let Some(name) = self.name.as_ref().filter(|n| !n.is_empty()) {
            return name.clone();
        }
        let look = match &self.recipe.look {
            Look::Effect { .. } => self
                .effect_name
                .clone()
                .unwrap_or_else(|| missing.to_owned()),
            Look::Native(native) => native_label(native.image_type),
            Look::Off | Look::Kept => missing.to_owned(),
        };
        match self.recipe.number {
            0 | 1 => look,
            n => format!("{look} {n}"),
        }
    }

    /// The recipe holds a Film Look.
    pub fn on_effect(&self) -> bool {
        matches!(self.recipe.look, Look::Effect { .. })
    }
}

/// The recipes of the camera, in camera menu order: the ones that are on
/// first.
#[derive(Clone, Debug, PartialEq)]
pub struct RecipesSnapshot {
    pub boot_id: String,
    pub recipes: Vec<RecipeItem>,
    pub max_on: usize,
    pub max_listed: usize,
    /// The undo steps of the camera (ADJ and the phone share them).
    pub undo: u32,
}

impl RecipesSnapshot {
    pub fn get(&self, id: u32) -> Option<&RecipeItem> {
        self.recipes.iter().find(|r| r.id == id)
    }

    pub fn on_count(&self) -> usize {
        self.recipes.iter().filter(|r| r.enabled).count()
    }

    /// The IDs of the recipes that are on, with the one at `from` moved to
    /// `to`, both counted among them.
    pub fn moved(&self, from: usize, to: usize) -> Vec<u32> {
        let mut ids: Vec<u32> = self
            .recipes
            .iter()
            .filter(|r| r.enabled)
            .map(|r| r.id)
            .collect();
        if from < ids.len() && to < ids.len() {
            let id = ids.remove(from);
            ids.insert(to, id);
        }
        ids
    }
}

fn read_item(value: &Value) -> Option<RecipeItem> {
    // A recipe without a name has `name: null`, which the model reads as
    // the empty name.
    let name = value["name"].as_str().map(str::to_owned);
    let mut fields = value.clone();
    fields["name"] = Value::String(name.clone().unwrap_or_default());
    let recipe: Recipe = serde_json::from_value(fields).ok()?;
    Some(RecipeItem {
        id: u32::try_from(value["id"].as_u64()?).ok()?,
        name,
        enabled: value["enabled"].as_bool()?,
        current: value["current"].as_bool().unwrap_or(false),
        effect_id: value["effectId"].as_str().map(str::to_owned),
        effect_name: value["effectName"].as_str().map(str::to_owned),
        ready: value["ready"].as_bool().unwrap_or(true),
        recipe,
    })
}

/// The answer of `GET /v1/recipes` and of each change.
pub fn parse_recipes(body: &[u8]) -> Result<RecipesSnapshot, String> {
    let value: Value = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    let recipes = value["recipes"]
        .as_array()
        .ok_or("The camera sent no recipes.")?
        .iter()
        .map(|v| {
            read_item(v).ok_or_else(|| "The camera sent a recipe the app cannot read.".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let limit = |key: &str, default: usize| {
        value["limits"][key]
            .as_u64()
            .map_or(default, |n| n as usize)
    };
    Ok(RecipesSnapshot {
        boot_id: value["bootId"].as_str().unwrap_or_default().to_owned(),
        recipes,
        max_on: limit("on", 40),
        max_listed: limit("listed", 64),
        undo: value["undo"].as_u64().unwrap_or(0) as u32,
    })
}

/// The recipe family of a camera model.
pub fn family_of(model: super::CameraModel) -> Family {
    match model {
        super::CameraModel::Gr3 | super::CameraModel::Gr3x => Family::Gr3,
        super::CameraModel::Gr4 => Family::Gr4,
    }
}

/// A preset of the release index for one camera family.
#[derive(Clone, Debug, PartialEq)]
pub struct PresetItem {
    pub id: String,
    /// The official Look that the preset needs.
    pub effect: Option<String>,
    pub author: Option<String>,
    pub text: BTreeMap<String, String>,
    pub recipe: Recipe,
}

impl PresetItem {
    /// The presets of the index that fit `family`.
    pub fn for_family(presets: &[IndexPreset], family: Family) -> Vec<PresetItem> {
        presets
            .iter()
            .filter_map(|p| {
                let recipe = p.recipes.iter().find(|r| r.family == family)?.clone();
                Some(PresetItem {
                    id: p.id.clone(),
                    effect: p.effect.clone(),
                    author: p.author.clone(),
                    text: p.text.clone(),
                    recipe,
                })
            })
            .collect()
    }

    /// The description in `language` (`zh` or `en`), else in English.
    pub fn text_in(&self, language: &str) -> Option<&str> {
        self.text
            .iter()
            .find(|(code, _)| code.starts_with(language))
            .or_else(|| self.text.iter().find(|(code, _)| code.starts_with("en")))
            .map(|(_, text)| text.as_str())
    }

    pub fn monochrome(&self) -> bool {
        match &self.recipe.look {
            Look::Native(native) => monochrome_type(native.image_type),
            _ => self.effect.as_deref().is_some_and(|id| id.contains("mono")),
        }
    }
}

/// A B&W Image Control type (`grkit_core::native::TYPES`).
pub fn monochrome_type(image_type: usize) -> bool {
    matches!(image_type, 2..=5 | 14..=20)
}

/// A failed recipe change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecipeError {
    Look(LookError),
    /// The camera menu shows as many recipes as it can.
    MenuFull,
    /// The card holds as many recipes as it can.
    CardFull,
    NothingToUndo,
    /// The camera does not have the Look of the recipe.
    LookMissing,
    /// The camera cannot apply some settings of the recipe.
    Unsupported,
}

impl RecipeError {
    pub fn keys(&self) -> (&'static str, &'static str) {
        match self {
            RecipeError::Look(LookError::CameraBusy) => {
                ("recipes.error.busy_title", "recipes.error.busy_body")
            }
            RecipeError::Look(error) => error.keys(),
            RecipeError::MenuFull => (
                "recipes.error.menu_full_title",
                "recipes.error.menu_full_body",
            ),
            RecipeError::CardFull => (
                "recipes.error.card_full_title",
                "recipes.error.card_full_body",
            ),
            RecipeError::NothingToUndo => ("recipes.error.undo_title", "recipes.error.undo_body"),
            RecipeError::LookMissing => ("recipes.error.look_title", "recipes.error.look_body"),
            RecipeError::Unsupported => (
                "recipes.error.unsupported_title",
                "recipes.error.unsupported_body",
            ),
        }
    }
}

/// Map an error answer of the recipe routes.
pub fn recipe_error(status: u16, body: &str) -> RecipeError {
    let value = serde_json::from_str::<Value>(body).unwrap_or_default();
    match value["code"].as_str().unwrap_or("") {
        "menu_full" => RecipeError::MenuFull,
        "card_full" => RecipeError::CardFull,
        "nothing_to_undo" => RecipeError::NothingToUndo,
        "recipe_look_missing" => RecipeError::LookMissing,
        "recipe_unsupported" => RecipeError::Unsupported,
        "runtime_changed" => RecipeError::Look(LookError::Changed),
        _ => RecipeError::Look(super::look_error(status, body)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> RecipesSnapshot {
        let presets = grkit_core::recipes::presets();
        let recipe = |id: &str| {
            presets
                .iter()
                .find(|p| p.id == id)
                .unwrap()
                .recipes
                .iter()
                .find(|r| r.family == Family::Gr3)
                .unwrap()
                .clone()
        };
        let mut items = Vec::new();
        for (i, (id, enabled)) in [
            ("bennet-800", true),
            ("reggie-negative", true),
            ("candy-color-400", false),
        ]
        .into_iter()
        .enumerate()
        {
            let mut value = serde_json::to_value(recipe(id)).unwrap();
            value["id"] = (i as u64 + 1).into();
            value["name"] = if i == 1 {
                Value::Null
            } else {
                recipe(id).name.into()
            };
            value["enabled"] = enabled.into();
            value["current"] = (i == 0).into();
            items.push(value);
        }
        let body = serde_json::json!({"bootId":"b","undo":1,"limits":{"on":40,"listed":64},"recipes":items});
        parse_recipes(&serde_json::to_vec(&body).unwrap()).unwrap()
    }

    #[test]
    fn the_list_reads_and_moves() {
        let s = snapshot();
        assert_eq!(s.recipes.len(), 3);
        assert_eq!(s.on_count(), 2);
        assert!(s.get(1).unwrap().current);
        assert_eq!(s.undo, 1);
        assert_eq!(s.moved(1, 0), vec![2, 1]);
        // An unnamed recipe shows the look and its number.
        let unnamed = s.get(2).unwrap();
        assert_eq!(
            unnamed.display_name(|_| "Negative Film".into(), "?"),
            "Negative Film"
        );
    }

    #[test]
    fn errors_map_by_code() {
        assert_eq!(
            recipe_error(409, r#"{"code":"menu_full"}"#),
            RecipeError::MenuFull
        );
        assert_eq!(
            recipe_error(503, r#"{"code":"camera_busy"}"#),
            RecipeError::Look(LookError::CameraBusy)
        );
    }
}
