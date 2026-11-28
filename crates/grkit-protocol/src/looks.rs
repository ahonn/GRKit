//! The Film Looks of `GET /v1/looks`.
//!
//! A Look on the camera has its file in `GRKIT/EFFECTS`, and the camera
//! loads it at its start. A Look on the card only has its file in
//! `GRKIT/STORED`, which the camera does not read.
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// The diagnostic of a list without the Looks on the card only: the
/// service cannot read `GRKIT/STORED`, or the folder holds too many files.
/// The Looks on the camera are still complete.
pub const STORED_UNREADABLE: &str = "stored_unreadable";

/// The answer of `GET /v1/looks`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LookList {
    /// The Looks on the camera, in the order of the camera menu, then the
    /// Looks that wait for the next start.
    pub looks: Vec<Look>,
    /// The Looks on the card only, by name.
    #[serde(default)]
    pub stored: Vec<Look>,
    /// The color scheme of the covers of the built-in Looks.
    #[serde(default)]
    pub official_covers: Option<String>,
    pub counts: Counts,
    pub limits: Limits,
    /// What the service could not read (`STORED_UNREADABLE`).
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
    pub boot_id: String,
    pub card_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counts {
    /// The Looks on the camera.
    pub listed: usize,
    /// The Looks on the camera that are On.
    pub loaded: usize,
    /// The Looks on the card only.
    #[serde(default)]
    pub stored: usize,
}

/// The camera takes at most `min(listed_looks, loaded_looks)` Looks: each
/// Look on the camera loads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    /// The most Looks that the camera lists.
    pub listed_looks: usize,
    /// The most Looks that are On.
    pub loaded_looks: usize,
    /// The most Looks on the card only. None for a service that keeps no
    /// Looks on the card only.
    #[serde(default)]
    pub stored_looks: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
}

/// What `GET /v1/looks/<id>` and each change of a Look answer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LookConfiguration {
    pub id: String,
    pub name: String,
    /// The SHA-256 of the GRF file.
    pub content_hash: String,
    /// On in the camera menu. A Look on the card only is Off.
    pub enabled: bool,
    #[serde(default)]
    pub place: Place,
    /// The cover that draws the icon of the file, when the service knows it.
    #[serde(default)]
    pub cover: Option<Value>,
    /// For a built-in Look, the cover that the scheme of the card gives it.
    #[serde(default)]
    pub default_cover: Option<Value>,
}

/// One Look of the list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Look {
    #[serde(flatten)]
    pub configuration: LookConfiguration,
    /// The place in its list.
    pub position: usize,
    pub runtime: LookRuntime,
}

/// The folder of the file of a Look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Place {
    /// `GRKIT/EFFECTS`: the camera loads the Look.
    #[default]
    Camera,
    /// `GRKIT/STORED`: the Look is on the card only.
    Card,
}

/// What the running camera does with a Look.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LookRuntime {
    #[serde(default)]
    pub id: String,
    pub source: LookSource,
    pub loaded: bool,
    pub selected: bool,
    /// The recipes and mode-dial sets that use the Look.
    #[serde(default)]
    pub used_by: usize,
    /// The first of them.
    #[serde(default)]
    pub uses: Vec<LookUse>,
    /// The change of the file that waits for the next start of the camera.
    #[serde(default, deserialize_with = "known_pending")]
    pub pending: Option<PendingChange>,
    /// What a phone changed in the Look since the camera started, so that a
    /// pending change can name itself.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edits: Vec<LookEdit>,
}

/// Where a Look on the card came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LookSource {
    /// A Look of the installed release.
    Official,
    /// A Look that a phone or the site made.
    Custom,
    /// A native look of GR IV (Cinema Yellow, Cinema Green) that the site
    /// added: its cover follows the scheme as an official Look's does.
    Native,
    /// An official ID with other content, and a source that a later
    /// service reports.
    #[serde(other)]
    Unknown,
}

/// A recipe or a mode-dial set that uses a Look.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LookUse {
    /// The recipe in `slot`: the name that the person gave it, or none for
    /// the automatic name (the Look's name and the number).
    Recipe {
        #[serde(default)]
        slot: u32,
        #[serde(default)]
        number: u32,
        #[serde(default)]
        name: Option<String>,
    },
    /// A user mode of the mode dial: U1 is 1.
    Dial { mode: u32 },
    /// A camera that does not say, and a use that a later service reports.
    #[serde(other)]
    Unknown,
}

/// A change of a Look file. The camera reads its Look files when it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PendingChange {
    /// A new Look: the camera does not list it yet.
    Add,
    /// A new name, content, or cover: the camera still has the old ones.
    Update,
}

/// A change that a phone made to a Look while the camera runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LookEdit {
    /// The Look came back from the card to the camera.
    Place,
    /// A new name.
    Name,
    /// A new cover.
    Cover,
    /// New content.
    Content,
    /// A change that a later service reports.
    #[serde(other)]
    Unknown,
}

/// A change that a later service reports says nothing about a restart here.
fn known_pending<'de, D>(deserializer: D) -> Result<Option<PendingChange>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| serde_json::from_value(value).ok()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_look_reads_back_with_the_fields_of_a_later_service() {
        let body = json!({"id":"gold","name":"Gold","contentHash":"c","enabled":false,"place":"card",
            "cover":null,"defaultCover":null,"position":0,
            "runtime":{"id":"gold","source":"later","loaded":false,"selected":false,"usedBy":1,
                "uses":[{"kind":"later"}],"pending":"later"}});
        let look: Look = serde_json::from_value(body).unwrap();
        assert_eq!(look.configuration.place, Place::Card);
        assert_eq!(look.runtime.source, LookSource::Unknown);
        assert_eq!(look.runtime.uses, [LookUse::Unknown]);
        assert_eq!(look.runtime.pending, None);
        let value = serde_json::to_value(&look).unwrap();
        assert_eq!(value["contentHash"], "c");
        assert_eq!(value["runtime"]["pending"], Value::Null);
    }
}
