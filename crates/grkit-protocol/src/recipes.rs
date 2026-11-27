//! The recipes of `GET /v1/recipes`.
use serde::{Deserialize, Serialize};

/// What the service tells of one recipe of the list. The same object also
/// holds the settings of the recipe (`grkit_core::recipes::Recipe`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeStatus {
    pub id: u32,
    /// The name that the person gave, or none for the automatic name.
    pub name: Option<String>,
    /// The recipe shows in the camera menu.
    pub enabled: bool,
    /// The camera settings match the recipe now.
    #[serde(default)]
    pub current: bool,
    /// The Film Look of the recipe, when the card has it on the camera or
    /// on the card only.
    #[serde(default)]
    pub effect_id: Option<String>,
    #[serde(default)]
    pub effect_name: Option<String>,
    /// The camera can apply the recipe: the recipe has no Film Look, or its
    /// Film Look is on the camera.
    #[serde(default = "yes")]
    pub ready: bool,
    /// The Film Look of the recipe is on the card only. The recipe applies
    /// after the Look is on the camera again.
    #[serde(default)]
    pub effect_stored: bool,
}

fn yes() -> bool {
    true
}
