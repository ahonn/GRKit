//! Module settings of the connected camera (`/v1/modules`, `/v1/params/modules`).
//!
//! The camera describes its modules: the app has no list of modules, settings,
//! values, or labels. Parsing skips what it cannot read, so a later GRKit with
//! new fields, settings, values, or modules works with this app. Kept free of
//! NSH so unit tests run without an engine.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::{LookError, look_error};

/// The highest catalog `schema` this app understands.
pub const CATALOG_SCHEMA: u64 = 1;

/// The items of a list that this app can read. One that it cannot read is
/// left out, and a field that is not a list reads as empty.
fn readable_items<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = Value::deserialize(deserializer)?;
    Ok(value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| serde_json::from_value(item.clone()).ok())
                .collect()
        })
        .unwrap_or_default())
}

/// The whole-number entries of a JSON object.
fn number_entries<'de, D>(deserializer: D) -> Result<BTreeMap<String, i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(value
        .as_object()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|(key, value)| Some((key.clone(), value.as_i64()?)))
                .collect()
        })
        .unwrap_or_default())
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModuleCatalog {
    pub schema: u64,
    pub catalog: String,
    pub language: String,
    #[serde(deserialize_with = "readable_items")]
    pub modules: Vec<CatalogModule>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CatalogModule {
    pub id: String,
    pub title: String,
    pub managed: Option<String>,
    #[serde(deserialize_with = "readable_items")]
    pub settings: Vec<CatalogSetting>,
    pub options: Option<CatalogOptions>,
    /// The camera sent a setting or a value that this app cannot read.
    #[serde(skip)]
    pub partial: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CatalogSetting {
    pub key: String,
    pub group: Option<String>,
    pub label: String,
    #[serde(deserialize_with = "readable_items")]
    pub values: Vec<CatalogValue>,
}

impl CatalogSetting {
    pub fn value_label(&self, value: i64) -> Option<&str> {
        self.values
            .iter()
            .find(|v| v.value == value)
            .map(|v| v.label.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CatalogValue {
    pub value: i64,
    #[serde(default)]
    pub label: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CatalogOptions {
    pub setting: String,
    pub ordered: bool,
}

/// What a catalog answer holds for this app.
#[derive(Clone, Debug, PartialEq)]
pub enum CatalogRead {
    Ready(ModuleCatalog),
    /// A later format: showing it could show wrong data.
    TooNew,
}

/// Parse `GET /v1/modules/catalog`.
pub fn parse_module_catalog(body: &[u8]) -> Result<CatalogRead, String> {
    let value: Value = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    match value["schema"].as_u64() {
        Some(schema) if schema > CATALOG_SCHEMA => return Ok(CatalogRead::TooNew),
        Some(schema) if schema > 0 => {}
        _ => return Err("The camera sent a catalog without a schema.".into()),
    }
    let raw = value.clone();
    let mut catalog: ModuleCatalog = serde_json::from_value(value).map_err(|e| e.to_string())?;
    catalog.modules.retain(|module| !module.id.is_empty());
    for module in &mut catalog.modules {
        module
            .settings
            .retain(|setting| !setting.key.is_empty() && !setting.values.is_empty());
        module.partial = raw["modules"]
            .as_array()
            .and_then(|modules| modules.iter().find(|m| m["id"] == module.id.as_str()))
            .is_some_and(|sent| has_unread_parts(module, sent));
    }
    Ok(CatalogRead::Ready(catalog))
}

/// Whether `sent` holds a setting or a value that `module` left out.
fn has_unread_parts(module: &CatalogModule, sent: &Value) -> bool {
    let settings = sent["settings"].as_array().map_or(&[][..], Vec::as_slice);
    settings.len() != module.settings.len()
        || module.settings.iter().any(|setting| {
            settings
                .iter()
                .find(|s| s["key"] == setting.key.as_str())
                .and_then(|s| s["values"].as_array())
                .is_some_and(|values| values.len() != setting.values.len())
        })
}

/// The catalog to use after a conditional read.
#[derive(Clone, Debug, PartialEq)]
pub enum CatalogAnswer {
    /// A new catalog, with the bytes to cache.
    Fresh(ModuleCatalog, Vec<u8>),
    /// 304: the cached catalog is current.
    Cached(ModuleCatalog),
    TooNew,
}

/// Decide what a catalog response means, given the cached catalog that named
/// its hash in `If-None-Match`.
pub fn catalog_answer(
    status: u16,
    body: Vec<u8>,
    cached: Option<ModuleCatalog>,
) -> Result<CatalogAnswer, LookError> {
    if status == 304 {
        return cached.map(CatalogAnswer::Cached).ok_or_else(|| {
            LookError::Service("The camera answered 304 without a cached catalog.".into())
        });
    }
    if !(200..300).contains(&status) {
        return Err(module_error(status, &String::from_utf8_lossy(&body)));
    }
    match parse_module_catalog(&body).map_err(LookError::Service)? {
        CatalogRead::Ready(catalog) => Ok(CatalogAnswer::Fresh(catalog, body)),
        CatalogRead::TooNew => Ok(CatalogAnswer::TooNew),
    }
}

/// `GET /v1/params/modules`, in ADJ order.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModulesState {
    pub revision: u64,
    /// GRKit runs: neither its switch nor the mode dial stops the modules.
    #[serde(rename = "grkitEnabled", default = "running")]
    pub grkit_enabled: bool,
    /// A camera with a monochrome sensor, which has no color-only module.
    pub monochrome: bool,
    /// What runs now. A camera that does not report it has each feature.
    pub features: Option<ModuleFeatures>,
    #[serde(deserialize_with = "readable_items")]
    pub modules: Vec<ModuleState>,
}

fn running() -> bool {
    true
}

/// The features of GRKit that have a tab of their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleFeatures {
    pub looks: bool,
    pub recipes: bool,
}

/// Modules to add and remove with the next software update.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModuleDraft {
    pub add: BTreeSet<String>,
    pub remove: BTreeSet<String>,
}

impl ModuleDraft {
    pub fn is_empty(&self) -> bool {
        self.add.is_empty() && self.remove.is_empty()
    }

    pub fn count(&self) -> usize {
        self.add.len() + self.remove.len()
    }

    pub fn toggle_add(&mut self, id: &str) {
        if !self.add.remove(id) {
            self.add.insert(id.to_owned());
        }
    }

    pub fn toggle_remove(&mut self, id: &str) {
        if !self.remove.remove(id) {
            self.remove.insert(id.to_owned());
        }
    }

    /// The package modules after this draft, sorted.
    pub fn after(&self, installed: &[String]) -> Vec<String> {
        installed
            .iter()
            .filter(|id| !self.remove.contains(*id))
            .chain(&self.add)
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

/// A package shows something: it has a module other than Film Look, which
/// an update without Looks cannot show alone.
pub fn package_can_install(modules: &[String]) -> bool {
    modules
        .iter()
        .any(|id| grkit_core::modules::official(id).is_none_or(|module| !module.needs_effects))
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ModuleState {
    pub id: String,
    pub enabled: bool,
    pub status: String,
    /// The person's own module, which only a card reader manages.
    pub custom: bool,
    pub place: String,
    pub orderable: bool,
    #[serde(deserialize_with = "number_entries")]
    pub values: BTreeMap<String, i64>,
    #[serde(deserialize_with = "readable_items")]
    pub hidden_values: Vec<i64>,
    #[serde(deserialize_with = "readable_items")]
    pub value_order: Vec<i64>,
}

pub fn parse_modules_state(body: &[u8]) -> Result<ModulesState, String> {
    let mut state: ModulesState = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    state.modules.retain(|module| !module.id.is_empty());
    Ok(state)
}

/// The answer of `PATCH /v1/params/modules/{id}`: the module and the revision.
pub fn parse_module_patch(body: &[u8]) -> Result<(ModuleState, u64), String> {
    let value: Value = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    let revision = value["revision"].as_u64().unwrap_or(0);
    let module: ModuleState = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if module.id.is_empty() {
        return Err("The camera sent a module without an ID.".into());
    }
    Ok((module, revision))
}

impl ModulesState {
    /// This state with `module` as the camera answered it.
    pub fn with_module(&self, module: ModuleState, revision: u64) -> Self {
        let mut next = self.clone();
        next.revision = revision;
        if let Some(slot) = next.modules.iter_mut().find(|m| m.id == module.id) {
            *slot = module;
        }
        next
    }

    pub fn orderable_ids(&self) -> Vec<String> {
        self.modules
            .iter()
            .filter(|m| m.orderable)
            .map(|m| m.id.clone())
            .collect()
    }

    /// The orderable IDs with the one at `from` moved to `to`, both counted
    /// among them.
    pub fn moved_ids(&self, from: usize, to: usize) -> Vec<String> {
        let mut ids = self.orderable_ids();
        if from < ids.len() && to < ids.len() {
            let id = ids.remove(from);
            ids.insert(to, id);
        }
        ids
    }

    /// This state with the orderable modules in the order of `ids`. Toolbar
    /// and hidden modules keep their places.
    pub fn with_order(&self, ids: &[String]) -> Self {
        let mut movable: Vec<ModuleState> = ids
            .iter()
            .filter_map(|id| {
                self.modules
                    .iter()
                    .find(|m| m.orderable && &m.id == id)
                    .cloned()
            })
            .collect();
        movable.reverse();
        let mut next = self.clone();
        for slot in next.modules.iter_mut().filter(|m| m.orderable) {
            if let Some(module) = movable.pop() {
                *slot = module;
            }
        }
        next
    }

    /// This state with the shown and hidden values of one module.
    pub fn with_choices(&self, id: &str, choices: &OptionChoices, ordered: bool) -> Self {
        let mut next = self.clone();
        if let Some(module) = next.modules.iter_mut().find(|m| m.id == id) {
            module.hidden_values = choices.hidden.clone();
            if ordered {
                module.value_order = choices.order();
            }
        }
        next
    }
}

/// What the app shows after a change it displayed before the camera answered:
/// the camera's answer, or the state from before the change and the error.
pub fn settle<T, E>(before: T, answer: Result<T, E>) -> (T, Option<E>) {
    match answer {
        Ok(state) => (state, None),
        Err(error) => (before, Some(error)),
    }
}

/// How a managed module links to the tab that manages its items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Managed {
    Looks,
    Recipes,
}

/// The line under a module's title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Summary {
    Value(String),
    /// The module is on, and its page value is Off.
    NotUsed,
    None,
}

/// One module: its state, and its definition when the catalog has it.
#[derive(Clone, Debug, PartialEq)]
pub struct ModuleRow {
    pub state: ModuleState,
    pub definition: Option<CatalogModule>,
}

/// Join the catalog and the state, in ADJ order. A module the catalog does
/// not name lists by its ID, with its switch.
pub fn module_rows(catalog: &ModuleCatalog, state: &ModulesState) -> Vec<ModuleRow> {
    state
        .modules
        .iter()
        .map(|module| ModuleRow {
            state: module.clone(),
            definition: catalog.modules.iter().find(|m| m.id == module.id).cloned(),
        })
        .collect()
}

/// The fixed rows before the orderable ones, the orderable rows, and the
/// fixed rows after them.
pub fn split_for_edit(rows: Vec<ModuleRow>) -> (Vec<ModuleRow>, Vec<ModuleRow>, Vec<ModuleRow>) {
    let first = rows
        .iter()
        .position(|r| r.state.orderable)
        .unwrap_or(rows.len());
    let (mut before, mut movable, mut after) = (Vec::new(), Vec::new(), Vec::new());
    for (index, row) in rows.into_iter().enumerate() {
        if row.state.orderable {
            movable.push(row);
        } else if index < first {
            before.push(row);
        } else {
            after.push(row);
        }
    }
    (before, movable, after)
}

impl ModuleRow {
    pub fn id(&self) -> &str {
        &self.state.id
    }

    pub fn title(&self) -> &str {
        self.definition
            .as_ref()
            .map(|d| d.title.as_str())
            .filter(|t| !t.is_empty())
            .unwrap_or(&self.state.id)
    }

    /// Listed without controls: it did not load.
    pub fn failed(&self) -> bool {
        self.state.status == "failed"
    }

    /// Enable is on, but the GRKit switch or the mode dial stops the module.
    pub fn paused(&self) -> bool {
        self.state.enabled && self.state.status == "stopped"
    }

    /// Safe mode kept the module out of this start.
    pub fn safe(&self) -> bool {
        self.state.status == "safe"
    }

    /// The catalog lacks the module, or a part that this app cannot read.
    pub fn needs_newer_app(&self) -> bool {
        self.definition.as_ref().is_none_or(|d| d.partial)
    }

    pub fn managed(&self) -> Option<Managed> {
        match self.definition.as_ref()?.managed.as_deref()? {
            "looks" => Some(Managed::Looks),
            "recipes" => Some(Managed::Recipes),
            _ => None,
        }
    }

    pub fn settings(&self) -> &[CatalogSetting] {
        self.definition.as_ref().map_or(&[], |d| &d.settings)
    }

    pub fn value_of(&self, setting: &CatalogSetting) -> Option<i64> {
        self.state.values.get(&setting.key).copied()
    }

    /// The settings that say whether the module is in use: the options
    /// setting, else the first setting of each group.
    fn page_settings(&self) -> Vec<&CatalogSetting> {
        let settings = self.settings();
        if let Some(options) = self.definition.as_ref().and_then(|d| d.options.as_ref())
            && let Some(setting) = settings.iter().find(|s| s.key == options.setting)
        {
            return vec![setting];
        }
        let mut seen = BTreeSet::new();
        settings
            .iter()
            .filter(|s| seen.insert(s.group.as_deref()))
            .collect()
    }

    /// What the list shows under the title.
    pub fn summary(&self) -> Summary {
        let page = self.page_settings();
        if page.is_empty() {
            return Summary::None;
        }
        if page.iter().all(|s| self.value_of(s) == Some(0)) {
            return Summary::NotUsed;
        }
        let shown = page.iter().find_map(|setting| {
            let value = self.value_of(setting).filter(|v| *v != 0)?;
            let label = setting.value_label(value)?;
            Some(match &setting.group {
                Some(group) => format!("{group} {label}"),
                None => label.to_owned(),
            })
        });
        shown.map_or(Summary::None, Summary::Value)
    }

    /// The settings in camera order, in runs of one group.
    pub fn setting_groups(&self) -> Vec<(Option<String>, Vec<CatalogSetting>)> {
        let mut groups: Vec<(Option<String>, Vec<CatalogSetting>)> = Vec::new();
        for setting in self.settings() {
            match groups.last_mut() {
                Some((group, items)) if *group == setting.group => items.push(setting.clone()),
                _ => groups.push((setting.group.clone(), vec![setting.clone()])),
            }
        }
        groups
    }

    /// `setting` with the values that can be chosen. The camera refuses a
    /// value that ADJ does not show, so the options setting lists the values
    /// of ADJ, in ADJ order.
    pub fn choosable(&self, setting: &CatalogSetting) -> CatalogSetting {
        let mut out = setting.clone();
        if let Some(options) = self.choices().filter(|c| c.setting.key == setting.key) {
            out.values = std::iter::once(&options.choices.first)
                .chain(&options.choices.shown)
                .filter_map(|v| setting.values.iter().find(|item| item.value == *v))
                .cloned()
                .collect();
        }
        out
    }

    /// The ADJ options of the page setting, when the module has them.
    pub fn choices(&self) -> Option<ModuleChoices> {
        let options = self.definition.as_ref()?.options.as_ref()?;
        let setting = self
            .settings()
            .iter()
            .find(|s| s.key == options.setting)?
            .clone();
        let first = setting.values.first()?.value;
        let offered: Vec<i64> = setting.values.iter().map(|v| v.value).collect();
        // The camera's sequence, then any value it does not place yet.
        let mut sequence: Vec<i64> = self
            .state
            .value_order
            .iter()
            .copied()
            .filter(|v| offered.contains(v))
            .collect();
        for value in &offered {
            if !sequence.contains(value) {
                sequence.push(*value);
            }
        }
        sequence.retain(|v| *v != first);
        let (hidden, shown) = sequence
            .into_iter()
            .partition(|v| self.state.hidden_values.contains(v));
        Some(ModuleChoices {
            setting,
            ordered: options.ordered,
            choices: OptionChoices {
                first,
                shown,
                hidden,
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleChoices {
    pub setting: CatalogSetting,
    pub ordered: bool,
    pub choices: OptionChoices,
}

/// The values of an options page. `first` is the Off row, which stays first
/// and is not listed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionChoices {
    pub first: i64,
    pub shown: Vec<i64>,
    pub hidden: Vec<i64>,
}

impl OptionChoices {
    /// The whole sequence the camera takes, first row included.
    pub fn order(&self) -> Vec<i64> {
        let mut order = vec![self.first];
        order.extend(&self.shown);
        order.extend(&self.hidden);
        order
    }

    /// These choices with the shown value at `from` moved to `to`.
    pub fn moved(&self, from: usize, to: usize) -> Self {
        let mut next = self.clone();
        if from < next.shown.len() && to < next.shown.len() {
            let value = next.shown.remove(from);
            next.shown.insert(to, value);
        }
        next
    }

    /// These choices with `value` shown or hidden. `None` when it is the last
    /// shown value, which must stay.
    pub fn toggled(&self, value: i64) -> Option<Self> {
        let mut next = self.clone();
        if let Some(at) = next.shown.iter().position(|v| *v == value) {
            if next.shown.len() == 1 {
                return None;
            }
            next.shown.remove(at);
            next.hidden.push(value);
        } else {
            let at = next.hidden.iter().position(|v| *v == value)?;
            next.hidden.remove(at);
            next.shown.push(value);
        }
        Some(next)
    }
}

/// Map an error answer of the module routes.
pub fn module_error(status: u16, body: &str) -> LookError {
    let value = serde_json::from_str::<Value>(body).unwrap_or_default();
    let problem = if value.get("code").is_some() {
        &value
    } else {
        value.get("error").unwrap_or(&value)
    };
    match problem.get("code").and_then(Value::as_str) {
        Some("runtime_changed") => LookError::Changed,
        _ => look_error(status, body),
    }
}

/// A PNG as a `data:` URI for an image view. `None` for other bytes.
pub fn png_data_uri(bytes: &[u8]) -> Option<String> {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    if !bytes.starts_with(SIGNATURE) {
        return None;
    }
    let mut uri = String::with_capacity(22 + bytes.len().div_ceil(3) * 4);
    uri.push_str("data:image/png;base64,");
    for chunk in bytes.chunks(3) {
        let at = |i: usize| chunk.get(i).copied().unwrap_or(0) as u32;
        let word = at(0) << 16 | at(1) << 8 | at(2);
        for index in 0..4 {
            if index <= chunk.len() {
                uri.push(ALPHABET[(word >> (18 - index * 6) & 63) as usize] as char);
            } else {
                uri.push('=');
            }
        }
    }
    Some(uri)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATALOG: &str = r#"{
      "schema": 1, "catalog": "00112233aabbccdd", "language": "en", "future": {"a": 1},
      "modules": [
        {"id": "A", "title": "Alpha", "managed": null, "soon": true,
         "settings": [
           {"key": "a.page", "group": null, "label": "Page", "kind": "new",
            "values": [{"value": 0, "label": "Off"}, {"value": 1, "label": "One"},
                       {"value": "later", "label": "Unreadable"}, {"value": 2, "label": "Two"}]},
           {"shape": "a setting this app cannot read", "key": 7}
         ],
         "options": {"setting": "a.page", "ordered": true}},
        {"id": 12},
        {"id": "G", "title": "Glow", "managed": null, "options": null, "settings": [
          {"key": "g.b", "group": "Bloom", "label": "Level",
           "values": [{"value": 0, "label": "Off"}, {"value": 1, "label": "Low"}]},
          {"key": "g.h", "group": "Halation", "label": "Level",
           "values": [{"value": 0, "label": "Off"}, {"value": 1, "label": "Low"}]}
        ]}
      ]
    }"#;

    fn catalog() -> ModuleCatalog {
        match parse_module_catalog(CATALOG.as_bytes()).unwrap() {
            CatalogRead::Ready(catalog) => catalog,
            CatalogRead::TooNew => panic!("schema 1"),
        }
    }

    fn module(id: &str, place: &str, orderable: bool) -> ModuleState {
        ModuleState {
            id: id.into(),
            enabled: true,
            status: "running".into(),
            place: place.into(),
            orderable,
            ..ModuleState::default()
        }
    }

    #[test]
    fn a_catalog_with_unknown_parts_still_parses() {
        let catalog = catalog();
        assert_eq!(catalog.modules.len(), 2);
        let settings = &catalog.modules[0].settings;
        assert_eq!(settings.len(), 1);
        let values: Vec<i64> = settings[0].values.iter().map(|v| v.value).collect();
        assert_eq!(values, [0, 1, 2]);
        assert!(catalog.modules[0].partial);
        assert!(!catalog.modules[1].partial);
        let later = br#"{"schema": 2, "catalog": "x", "modules": "another shape"}"#;
        assert_eq!(parse_module_catalog(later), Ok(CatalogRead::TooNew));
    }

    #[test]
    fn a_draft_changes_the_package_and_film_look_cannot_stay_alone() {
        let installed = ["FILMLOOK".to_owned(), "GLOW".to_owned()];
        let mut draft = ModuleDraft::default();
        draft.toggle_add("GRAIN");
        draft.toggle_remove("GLOW");
        assert_eq!(draft.after(&installed), ["FILMLOOK", "GRAIN"]);
        assert!(package_can_install(&draft.after(&installed)));
        draft.toggle_add("GRAIN");
        assert_eq!(draft.count(), 1);
        assert!(!package_can_install(&draft.after(&installed)));
        assert!(!package_can_install(&[]));
    }

    #[test]
    fn a_304_uses_the_cache_and_a_200_replaces_it() {
        let cached = catalog();
        assert_eq!(
            catalog_answer(304, Vec::new(), Some(cached.clone())),
            Ok(CatalogAnswer::Cached(cached.clone()))
        );
        assert!(catalog_answer(304, Vec::new(), None).is_err());
        let body = CATALOG.as_bytes().to_vec();
        assert_eq!(
            catalog_answer(200, body.clone(), Some(ModuleCatalog::default())),
            Ok(CatalogAnswer::Fresh(cached, body))
        );
    }

    #[test]
    fn the_summary_follows_the_page_values() {
        let catalog = catalog();
        let mut glow = module("G", "root", true);
        glow.values = BTreeMap::from([("g.b".into(), 0), ("g.h".into(), 0)]);
        let row = |state: &ModuleState| ModuleRow {
            state: state.clone(),
            definition: catalog.modules.iter().find(|m| m.id == state.id).cloned(),
        };
        assert_eq!(row(&glow).summary(), Summary::NotUsed);
        glow.values.insert("g.h".into(), 1);
        assert_eq!(row(&glow).summary(), Summary::Value("Halation Low".into()));
    }

    #[test]
    fn a_move_keeps_fixed_modules_and_a_failure_restores_the_order() {
        let state = ModulesState {
            revision: 3,
            modules: vec![
                module("T", "toolbar", false),
                module("A", "root", true),
                module("B", "root", true),
                module("C", "root", true),
                module("H", "hidden", false),
            ],
            ..ModulesState::default()
        };
        let ids = state.moved_ids(2, 0);
        assert_eq!(ids, ["C", "A", "B"]);
        let shown = state.with_order(&ids);
        let order: Vec<&str> = shown.modules.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(order, ["T", "C", "A", "B", "H"]);
        let (kept, error) = settle(state.clone(), Err::<ModulesState, _>(LookError::CameraBusy));
        assert_eq!((kept, error), (state, Some(LookError::CameraBusy)));
    }

    #[test]
    fn options_keep_off_first_and_one_value_shown() {
        let catalog = catalog();
        let mut state = module("A", "root", true);
        state.value_order = vec![0, 2, 1];
        state.hidden_values = vec![1];
        let row = ModuleRow {
            state,
            definition: Some(catalog.modules[0].clone()),
        };
        let module_choices = row.choices().unwrap();
        let listed: Vec<i64> = row
            .choosable(&module_choices.setting)
            .values
            .iter()
            .map(|v| v.value)
            .collect();
        assert_eq!(listed, [0, 2]);
        let choices = module_choices.choices;
        assert_eq!(
            (choices.shown.clone(), choices.hidden.clone()),
            (vec![2], vec![1])
        );
        assert_eq!(choices.toggled(2), None);
        let shown = choices.toggled(1).unwrap();
        assert_eq!(shown.moved(1, 0).order(), [0, 1, 2]);
    }
}
