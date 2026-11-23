//! Film Looks on the camera (`/v1/looks`) and in the official catalog.
//!
//! Kept free of NSH so unit tests run without an engine. Shapes match
//! `grkit-serve` (`look_api.rs`) and the site catalog (`site::CATALOG_SCHEMA`).

use std::collections::BTreeSet;

use grkit_core::cover::{Cover, Film, Scheme};
use serde::Deserialize;

/// The longest Look name the camera menu shows (`effects::NAME_MAX`).
pub const NAME_MAX: usize = 16;
/// The longest effect ID (`effects::ID_MAX`).
pub const ID_MAX: usize = 31;
/// The ID prefix of Looks imported on a phone; no catalog ID starts with it.
pub const IMPORT_PREFIX: &str = "lut-";

/// Where a Look on the card came from, as the camera reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LookSource {
    Official,
    Custom,
    /// A native look of GR IV (Cinema Yellow, Cinema Green) that the site
    /// added: its cover follows the scheme as an official Look's does.
    Native,
    Unknown,
}

/// The film family of a catalog Look, in the order the catalog lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LookGroup {
    Negative,
    Slide,
    Cinema,
    Instant,
    Mono,
}

impl LookGroup {
    pub const ORDER: [LookGroup; 5] = [
        LookGroup::Negative,
        LookGroup::Slide,
        LookGroup::Cinema,
        LookGroup::Instant,
        LookGroup::Mono,
    ];

    /// String key of the group's label (`strings.rs`).
    pub fn label_key(self) -> &'static str {
        match self {
            LookGroup::Negative => "looks.group.negative",
            LookGroup::Slide => "looks.group.slide",
            LookGroup::Cinema => "looks.group.cinema",
            LookGroup::Instant => "looks.group.instant",
            LookGroup::Mono => "looks.group.mono",
        }
    }

    pub fn monochrome(self) -> bool {
        self == LookGroup::Mono
    }
}

/// The catalog filter of the All Looks list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookFilter {
    All,
    Color,
    Mono,
}

impl LookFilter {
    pub fn accepts(self, group: LookGroup) -> bool {
        match self {
            LookFilter::All => true,
            LookFilter::Color => !group.monochrome(),
            LookFilter::Mono => group.monochrome(),
        }
    }
}

/// Catalog groups of the official Looks (the site's `effectGroups.ts`).
const GROUPS: &[(&str, LookGroup)] = &[
    ("aerocolor-iv-v1", LookGroup::Negative),
    ("c200-premier-v1", LookGroup::Negative),
    ("ektar-100-edge-v1", LookGroup::Negative),
    ("fuji-160c-v1", LookGroup::Negative),
    ("gold-200-premier-v1", LookGroup::Negative),
    ("portra-160-vc-v1", LookGroup::Negative),
    ("portra-400-endura-v1", LookGroup::Negative),
    ("portra-800-push2-v1", LookGroup::Negative),
    ("pro-400h-crystal-v1", LookGroup::Negative),
    ("ultramax-400-ultra-v1", LookGroup::Negative),
    ("vista-200-v1", LookGroup::Negative),
    ("xtra-400-crystal-v1", LookGroup::Negative),
    ("astia-100f-v1", LookGroup::Slide),
    ("ektachrome-e100-v1", LookGroup::Slide),
    ("kodachrome-64-v1", LookGroup::Slide),
    ("precisa-100-v1", LookGroup::Slide),
    ("provia-100f-v1", LookGroup::Slide),
    ("velvia-100-v1", LookGroup::Slide),
    ("eterna-500t-3523xd-v1", LookGroup::Cinema),
    ("vision3-250d-2383-v1", LookGroup::Cinema),
    ("vision3-500t-supra-v1", LookGroup::Cinema),
    ("instax-v1", LookGroup::Instant),
    ("delta-100-v1", LookGroup::Mono),
    ("double-x-5222-mono-v1", LookGroup::Mono),
    ("hp5-plus-400-mono-v1", LookGroup::Mono),
    ("neopan-acros-100-v1", LookGroup::Mono),
    ("red-filter-mono-v1", LookGroup::Mono),
    ("t-max-400-v1", LookGroup::Mono),
    ("trix-400-mono-v1", LookGroup::Mono),
    ("xp2-super-v1", LookGroup::Mono),
];

/// The group of a Look. An unknown ID falls back to color negative or B&W.
pub fn group_of(id: &str, monochrome: bool) -> LookGroup {
    GROUPS
        .iter()
        .find(|(known, _)| *known == id)
        .map(|(_, group)| *group)
        .unwrap_or(if monochrome {
            LookGroup::Mono
        } else {
            LookGroup::Negative
        })
}

/// One Look on the card, with its runtime state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Look {
    pub id: String,
    pub name: String,
    pub content_hash: String,
    /// Shown in the camera menu.
    pub enabled: bool,
    /// Position in the camera menu, from 0.
    pub position: usize,
    pub source: LookSource,
    /// Loaded by the runtime (at most `max_loaded` are).
    pub loaded: bool,
    /// The Look in use.
    pub selected: bool,
    /// The number of recipes and mode-dial sets that use the Look.
    pub used_by: usize,
    /// Where the first of them are, as the camera names them.
    pub uses: Vec<LookUse>,
    /// The change that waits for the next start of the camera.
    pub pending: Option<PendingChange>,
    /// The cover that draws the icon of the file, when the camera knows it.
    pub cover: Option<Cover>,
    /// For a built-in Look, the cover that the scheme of the card gives it.
    pub default_cover: Option<Cover>,
}

/// A recipe or a mode-dial set that uses a Look.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LookUse {
    /// A recipe: the name that the person gave it, or none for the automatic
    /// name (the Look's name and the number).
    Recipe {
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        number: u32,
    },
    /// A user mode of the mode dial: U1 is 1.
    Dial { mode: u32 },
    #[serde(other)]
    Unknown,
}

/// How many uses the message that blocks a delete names.
pub const USES_NAMED: usize = 3;

/// What blocks the delete of `look`: the names of its uses, at most
/// `USES_NAMED`, and how many more there are. `recipe` and `dial` turn a
/// recipe name or a dial mode into its label.
pub fn use_labels(
    look: &Look,
    recipe: impl Fn(&str) -> String,
    dial: impl Fn(u32) -> String,
) -> (Vec<String>, usize) {
    let named: Vec<String> = look
        .uses
        .iter()
        .filter_map(|use_| match use_ {
            LookUse::Recipe { name, number } => Some(recipe(&match name {
                Some(name) => name.clone(),
                None if *number > 0 => format!("{} {number}", look.name),
                None => look.name.clone(),
            })),
            LookUse::Dial { mode } => Some(dial(*mode)),
            LookUse::Unknown => None,
        })
        .take(USES_NAMED)
        .collect();
    let more = look.used_by.saturating_sub(named.len());
    (named, more)
}

/// A group of built-in Looks whose covers reset together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverGroup {
    Color,
    Mono,
}

impl CoverGroup {
    /// The id of the API.
    pub fn id(self) -> &'static str {
        match self {
            CoverGroup::Color => "color",
            CoverGroup::Mono => "mono",
        }
    }
}

/// A change of a Look file. The camera reads its Look files when it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PendingChange {
    /// A new Look: the camera does not list it yet.
    Add,
    /// A new name, content or cover: the camera still has the old ones.
    Update,
}

impl Look {
    /// Phone-made and official Looks change their name and cover. The
    /// camera refuses changes to a Look of unknown source.
    pub fn editable(&self) -> bool {
        self.source != LookSource::Unknown
    }

    /// The cover the phone shows: the camera's, else the default cover of
    /// the name.
    pub fn shown_cover(&self) -> Cover {
        self.cover
            .clone()
            .unwrap_or_else(|| Cover::for_name(&self.name))
    }

    /// The group of a built-in Look: a native look is a color Look.
    pub fn cover_group(&self) -> CoverGroup {
        match self.source == LookSource::Official && group_of(&self.id, false).monochrome() {
            true => CoverGroup::Mono,
            false => CoverGroup::Color,
        }
    }

    /// A built-in Look with a cover that is neither of the variants of its
    /// base cover. The default cover is one of them; base covers have no
    /// colors, so the default cover without colors is the base cover.
    pub fn individual_cover(&self) -> bool {
        let (Some(cover), Some(default)) = (&self.cover, &self.default_cover) else {
            return false;
        };
        let base = match default {
            Cover::Film(film) => Cover::Film(Film {
                colors: None,
                ..film.clone()
            }),
            label => label.clone(),
        };
        base.variant_of(cover).is_none()
    }

    /// The camera lists the Look, so it can show or hide it at once.
    pub fn listed(&self) -> bool {
        self.pending != Some(PendingChange::Add)
    }
}

/// The camera's Looks, in camera menu order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LooksSnapshot {
    pub looks: Vec<Look>,
    pub loaded: usize,
    pub max_loaded: usize,
    pub max_listed: usize,
    pub boot_id: String,
    pub card_id: String,
    /// The color scheme of the covers of the official Looks on the card.
    pub official_covers: Scheme,
}

impl LooksSnapshot {
    pub fn get(&self, id: &str) -> Option<&Look> {
        self.looks.iter().find(|look| look.id == id)
    }

    pub fn ids(&self) -> Vec<String> {
        self.looks.iter().map(|look| look.id.clone()).collect()
    }

    /// Whether the card has a built-in Look of the color scheme.
    pub fn has_built_in(&self) -> bool {
        self.looks.iter().any(|look| look.default_cover.is_some())
    }

    /// Whether a built-in Look of `group` has an individual cover.
    pub fn has_individual_covers(&self, group: CoverGroup) -> bool {
        self.looks
            .iter()
            .any(|look| look.cover_group() == group && look.individual_cover())
    }

    /// Room for one more enabled Look.
    pub fn has_room(&self) -> bool {
        self.room_problem().is_none()
    }

    /// Why one more Look does not fit: the card holds `max_listed` Looks,
    /// the camera menu shows `max_loaded`.
    pub fn room_problem(&self) -> Option<LookError> {
        if self.card_full() {
            Some(LookError::CardFull)
        } else if self.loaded >= self.max_loaded {
            Some(LookError::NoRoom)
        } else {
            None
        }
    }

    /// The card holds as many Looks as the camera lists.
    pub fn card_full(&self) -> bool {
        self.looks.len() >= self.max_listed
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireList {
    looks: Vec<WireLook>,
    counts: WireCounts,
    limits: WireLimits,
    boot_id: String,
    card_id: String,
    #[serde(default)]
    official_covers: Option<String>,
}

#[derive(Deserialize)]
struct WireCounts {
    loaded: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireLimits {
    listed_looks: usize,
    loaded_looks: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireLook {
    id: String,
    name: String,
    content_hash: String,
    enabled: bool,
    position: usize,
    runtime: WireRuntime,
    #[serde(default)]
    cover: Option<serde_json::Value>,
    #[serde(default)]
    default_cover: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireRuntime {
    source: LookSource,
    loaded: bool,
    selected: bool,
    #[serde(default)]
    used_by: usize,
    #[serde(default)]
    uses: Vec<LookUse>,
    #[serde(default)]
    pending: Option<PendingChange>,
}

/// Parse a `GET /v1/looks` body.
pub fn parse_looks(body: &[u8]) -> Result<LooksSnapshot, String> {
    let wire: WireList = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    let mut looks: Vec<Look> = wire
        .looks
        .into_iter()
        .map(|look| Look {
            id: look.id,
            name: look.name,
            content_hash: look.content_hash,
            enabled: look.enabled,
            position: look.position,
            source: look.runtime.source,
            loaded: look.runtime.loaded,
            selected: look.runtime.selected,
            used_by: look.runtime.used_by,
            uses: look.runtime.uses,
            pending: look.runtime.pending,
            // A cover this app cannot read shows as the default one.
            cover: look.cover.and_then(|cover| Cover::from_json(&cover).ok()),
            default_cover: look
                .default_cover
                .and_then(|cover| Cover::from_json(&cover).ok()),
        })
        .collect();
    looks.sort_by_key(|look| look.position);
    Ok(LooksSnapshot {
        looks,
        loaded: wire.counts.loaded,
        max_loaded: wire.limits.loaded_looks,
        max_listed: wire.limits.listed_looks,
        boot_id: wire.boot_id,
        card_id: wire.card_id,
        // A camera before schemes, or an unknown scheme, has gray covers.
        official_covers: wire
            .official_covers
            .as_deref()
            .and_then(Scheme::parse)
            .unwrap_or_default(),
    })
}

/// Why a Looks request failed, in terms the UI explains.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookError {
    /// The phone lost the camera network.
    ConnectionLost,
    /// The card or runtime is not ready yet.
    NotReady,
    /// The camera shows a GRKit page, which keeps its rows while it shows.
    CameraBusy,
    /// The camera lists the Look only after it starts again.
    RestartRequired,
    /// Recipes or dial sets use the Look.
    InUse,
    /// Deleting the Look in use needs the camera to switch to Off.
    ActiveLook,
    /// Official Looks keep their content.
    OfficialReadOnly,
    /// The camera menu shows as many Looks as it can.
    NoRoom,
    /// The card holds as many Looks as the camera lists.
    CardFull,
    /// A software update waits on the camera.
    UpdatePending,
    /// Phone changes are off on the camera.
    ChangesOff,
    /// Another change is running.
    Busy,
    /// The Looks on the camera differ from the ones the phone read, as
    /// after a restart of the camera.
    Changed,
    InvalidName,
    NotFound,
    /// The cover list of the card cannot be read or is full: nothing changed.
    CoverLibrary,
    /// The Look file changed, but its cover was not saved.
    CoverNotSaved,
    /// The file is not a LUT or preset the phone can convert.
    ImportFailed(String),
    Service(String),
}

impl LookError {
    /// String keys of the title and body (`strings.rs`).
    pub fn keys(&self) -> (&'static str, &'static str) {
        match self {
            LookError::ConnectionLost => ("looks.error.lost_title", "looks.error.lost_body"),
            LookError::NotReady => ("looks.error.not_ready_title", "looks.error.not_ready_body"),
            LookError::CameraBusy => ("looks.error.camera_title", "looks.error.camera_body"),
            LookError::RestartRequired => ("looks.error.restart_title", "looks.error.restart_body"),
            LookError::InUse => ("looks.error.in_use_title", "looks.error.in_use_body"),
            LookError::ActiveLook => ("looks.error.active_title", "looks.error.active_body"),
            LookError::OfficialReadOnly => {
                ("looks.error.official_title", "looks.error.official_body")
            }
            LookError::NoRoom => ("looks.error.full_title", "looks.error.full_body"),
            LookError::CardFull => ("looks.error.card_full_title", "looks.error.card_full_body"),
            LookError::UpdatePending => ("looks.error.pending_title", "looks.error.pending_body"),
            LookError::ChangesOff => ("looks.error.off_title", "looks.error.off_body"),
            LookError::Busy => ("looks.error.busy_title", "looks.error.busy_body"),
            LookError::Changed => ("looks.error.changed_title", "looks.error.changed_body"),
            LookError::InvalidName => ("looks.error.name_title", "looks.error.name_body"),
            LookError::NotFound => ("looks.error.gone_title", "looks.error.gone_body"),
            LookError::CoverLibrary => ("looks.error.cover_title", "looks.error.cover_list_body"),
            LookError::CoverNotSaved => ("looks.error.cover_title", "looks.error.cover_saved_body"),
            LookError::ImportFailed(_) => ("looks.error.import_title", "looks.error.import_body"),
            LookError::Service(_) => ("looks.error.service_title", "looks.error.service_body"),
        }
    }
}

/// Map an error answer: a problem document (`code`) or a failed request
/// record (`error.code`).
pub fn look_error(status: u16, body: &str) -> LookError {
    let value = serde_json::from_str::<serde_json::Value>(body).unwrap_or_default();
    let problem = if value.get("code").is_some() {
        &value
    } else {
        value.get("error").unwrap_or(&value)
    };
    let code = problem.get("code").and_then(|v| v.as_str()).unwrap_or("");
    match code {
        "dependency_in_use" | "dependencies_unavailable" => return LookError::InUse,
        "active_look" => return LookError::ActiveLook,
        "official_look_read_only" | "origin_unknown" | "origin_conflict" => {
            return LookError::OfficialReadOnly;
        }
        "look_capacity" => return LookError::NoRoom,
        "software_pending" => return LookError::UpdatePending,
        "writes_disabled" => return LookError::ChangesOff,
        "camera_busy" => return LookError::CameraBusy,
        "restart_required" => return LookError::RestartRequired,
        "runtime_busy" => return LookError::Busy,
        "context_changed" => return LookError::Changed,
        "invalid_name" => return LookError::InvalidName,
        "look_not_found" => return LookError::NotFound,
        "cover_library_unavailable" => return LookError::CoverLibrary,
        "cover_not_saved" => return LookError::CoverNotSaved,
        "card_unavailable" | "runtime_unavailable" | "runtime_protocol_unsupported" => {
            return LookError::NotReady;
        }
        _ => {}
    }
    match status {
        404 => LookError::NotFound,
        503 => LookError::NotReady,
        _ => LookError::Service(
            problem
                .get("detail")
                .and_then(|v| v.as_str())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("Camera returned HTTP {status}.")),
        ),
    }
}

/// A Look name the camera accepts: 1 to 16 printable ASCII characters
/// without spaces at either end.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= NAME_MAX
        && name == name.trim()
        && name.bytes().all(|b| (0x20..=0x7e).contains(&b))
}

/// Words of LUT file names that name an encoding or a format, not the look.
fn technical(word: &str) -> bool {
    let w = word.to_ascii_lowercase();
    let w = w.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    matches!(
        w,
        "flog"
            | "f-log"
            | "flog2"
            | "f-log2"
            | "flog2c"
            | "vlog"
            | "v-log"
            | "slog"
            | "slog2"
            | "slog3"
            | "s-log3"
            | "logc"
            | "logc3"
            | "logc4"
            | "log"
            | "to"
            | "bt709"
            | "bt.709"
            | "rec709"
            | "rec.709"
            | "709"
            | "srgb"
            | "p3"
            | "displayp3"
            | "d65"
            | "lut"
            | "luts"
            | "cube"
            | "xmp"
    ) || (w.ends_with("grid") && w[..w.len() - 4].bytes().all(|b| b.is_ascii_digit()))
}

/// A Look name from a file name: the stem without technical words, cut to the
/// whole words that fit 16 characters.
pub fn name_from_file(file: &str) -> String {
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    let words: Vec<String> = stem
        .split(|c: char| c == '_' || c.is_whitespace() || !(c.is_ascii_graphic()))
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect();
    let kept: Vec<&String> = words.iter().filter(|w| !technical(w)).collect();
    let chosen: Vec<&String> = if kept.is_empty() {
        words.iter().collect()
    } else {
        kept
    };
    let mut text = String::new();
    for word in chosen {
        let next = if text.is_empty() {
            word.clone()
        } else {
            format!("{text} {word}")
        };
        if next.len() > NAME_MAX {
            if text.is_empty() {
                text = word.chars().take(NAME_MAX).collect();
            }
            break;
        }
        text = next;
    }
    let text = text.trim().to_owned();
    if valid_name(&text) {
        text
    } else {
        "Imported Look".into()
    }
}

/// A new Look ID for `name`: `lut-` and the lowercase letters and digits of
/// the name, with a number when the ID is taken.
pub fn import_id(name: &str, taken: &BTreeSet<String>) -> String {
    let mut slug = String::new();
    for c in name.to_ascii_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "look" } else { slug };
    let mut base: String = format!("{IMPORT_PREFIX}{slug}")
        .chars()
        .take(ID_MAX - 3)
        .collect();
    while base.ends_with('-') {
        base.pop();
    }
    let mut id = base.clone();
    let mut number = 2;
    while taken.contains(&id) {
        id = format!("{base}-{number}");
        number += 1;
    }
    id
}

/// The menu order after the shown Look at `from` moves to `to`, both counted
/// among the shown Looks. `looks` is the menu order with each Look's shown
/// state; hidden Looks keep their places.
pub fn move_shown<'a>(
    looks: impl IntoIterator<Item = (&'a str, bool)> + Clone,
    from: usize,
    to: usize,
) -> Vec<String> {
    let mut shown: Vec<&str> = looks
        .clone()
        .into_iter()
        .filter_map(|(id, on)| on.then_some(id))
        .collect();
    if from < shown.len() && to < shown.len() {
        let id = shown.remove(from);
        shown.insert(to, id);
    }
    let mut shown = shown.into_iter();
    looks
        .into_iter()
        .map(|(id, on)| match on {
            true => shown.next().unwrap_or(id).to_owned(),
            false => id.to_owned(),
        })
        .collect()
}

/// One official Look of the site catalog for a firmware target.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CatalogLook {
    pub id: String,
    pub name: String,
    pub monochrome: bool,
    /// File name under `effects/<target>/`.
    pub file: String,
    pub sha256: String,
    /// The cover JSON of the Look file, when the Look has a film cover.
    #[serde(default)]
    pub cover: Option<serde_json::Value>,
}

impl CatalogLook {
    /// The cover of the catalog, when this app can read it.
    pub fn film_cover(&self) -> Option<Cover> {
        self.cover
            .as_ref()
            .and_then(|cover| Cover::from_json(cover).ok())
    }

    /// The cover the phone shows: the catalog's, else the default one.
    pub fn shown_cover(&self) -> Cover {
        self.film_cover()
            .unwrap_or_else(|| Cover::for_name(&self.name))
    }
}

/// The site catalog of one firmware target.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Catalog {
    pub target: String,
    /// The camera's base gamma: imports fit to it.
    pub base_gamma: Vec<u16>,
    #[serde(rename = "effects")]
    pub looks: Vec<CatalogLook>,
}

/// Parse `effects/<target>/catalog.json`.
pub fn parse_catalog(bytes: &[u8]) -> Result<Catalog, String> {
    #[derive(Deserialize)]
    struct Schema {
        schema: String,
    }
    let schema: Schema = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if schema.schema != grkit_core::site::CATALOG_SCHEMA {
        return Err(format!("Unsupported catalog schema: {}", schema.schema));
    }
    let catalog: Catalog = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if catalog.base_gamma.len() != 256 {
        return Err("Catalog base gamma must have 256 values".into());
    }
    for look in &catalog.looks {
        let valid_file = look.file.len() == 12
            && look.file.ends_with(".GRF")
            && look.file[..8].bytes().all(|b| b.is_ascii_hexdigit());
        if !grkit_core::effects::valid_id(&look.id) || !valid_file {
            return Err(format!("Invalid catalog entry: {}", look.id));
        }
    }
    Ok(catalog)
}

/// Where a dragged row lands: `offset` points from row `from`, past each
/// row whose middle it reached. Rows differ in height at large text sizes.
pub fn reorder_target(heights: &[f32], from: usize, offset: f32) -> usize {
    let mut to = from;
    let mut passed = 0.0;
    let reach = offset.abs();
    let mut pass = |index: usize| {
        if reach < passed + heights[index] / 2.0 {
            return false;
        }
        passed += heights[index];
        to = index;
        true
    };
    if offset > 0.0 {
        for index in from + 1..heights.len() {
            if !pass(index) {
                break;
            }
        }
    } else {
        for index in (0..from).rev() {
            if !pass(index) {
                break;
            }
        }
    }
    to
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_drag_lands_by_the_real_row_heights() {
        let heights = [64.0, 120.0, 64.0];
        assert_eq!(super::reorder_target(&heights, 0, 59.0), 0);
        assert_eq!(super::reorder_target(&heights, 0, 60.0), 1);
        assert_eq!(super::reorder_target(&heights, 0, 151.0), 1);
        assert_eq!(super::reorder_target(&heights, 0, 152.0), 2);
        assert_eq!(super::reorder_target(&heights, 2, -59.0), 2);
        assert_eq!(super::reorder_target(&heights, 2, -60.0), 1);
        assert_eq!(super::reorder_target(&heights, 2, -900.0), 0);
    }

    use super::*;

    const LIST: &str = r#"{"looks":[
        {"id":"trix-400-mono-v1","name":"Tri-X 400","contentHash":"b","enabled":true,"position":1,
         "runtime":{"id":"trix-400-mono-v1","source":"official","loaded":true,"selected":false,"usedBy":0,"pending":"update"},
         "cover":{"template":"stack","brand":"Kodak","name":"TRI-X"}},
        {"id":"portra-400-endura-v1","name":"Portra 400","contentHash":"a","enabled":true,"position":0,
         "runtime":{"id":"portra-400-endura-v1","source":"official","loaded":true,"selected":true,"usedBy":1,"pending":null},"cover":null}],
        "counts":{"listed":2,"loaded":2},"limits":{"listedLooks":64,"loadedLooks":40},
        "bootId":"boot","cardId":"card"}"#;

    #[test]
    fn parses_the_service_list_in_menu_order() {
        let snapshot = parse_looks(LIST.as_bytes()).unwrap();
        assert_eq!(snapshot.ids(), ["portra-400-endura-v1", "trix-400-mono-v1"]);
        let portra = snapshot.get("portra-400-endura-v1").unwrap();
        assert!(portra.selected);
        assert_eq!((portra.used_by, portra.pending), (1, None));
        let trix = snapshot.get("trix-400-mono-v1").unwrap();
        assert_eq!(trix.pending, Some(PendingChange::Update));
        assert!(trix.listed());
        assert_eq!(portra.source, LookSource::Official);
        // The phone draws the cover of the camera, else the default one.
        assert_eq!(
            trix.shown_cover().to_json(),
            serde_json::json!({"template": "stack", "brand": "Kodak", "name": "TRI-X"})
        );
        assert_eq!(portra.cover, None);
        assert_eq!(portra.shown_cover(), Cover::for_name("Portra 400"));
        assert_eq!((snapshot.loaded, snapshot.max_loaded), (2, 40));
        assert_eq!(snapshot.boot_id, "boot");
        assert!(snapshot.has_room());
    }

    #[test]
    fn uses_name_recipes_and_dial_modes_and_count_the_rest() {
        let list = LIST.replace(
            r#""usedBy":1,"#,
            r#""usedBy":5,"uses":[{"kind":"recipe","name":"Street","number":0},
                {"kind":"recipe","name":null,"number":2},{"kind":"dial","mode":1},
                {"kind":"recipe","name":"Night","number":0}],"#,
        );
        let snapshot = parse_looks(list.as_bytes()).unwrap();
        let portra = snapshot.get("portra-400-endura-v1").unwrap();
        let (names, more) = use_labels(portra, |n| format!("R:{n}"), |m| format!("U{m}"));
        // The automatic name of a recipe is the Look's name and its number.
        assert_eq!(names, ["R:Street", "R:Portra 400 2", "U1"]);
        assert_eq!(more, 2);
    }

    #[test]
    fn a_full_card_and_a_full_menu_are_told_apart() {
        let mut snapshot = parse_looks(LIST.as_bytes()).unwrap();
        assert_eq!(snapshot.room_problem(), None);
        snapshot.max_loaded = 2;
        assert_eq!(snapshot.room_problem(), Some(LookError::NoRoom));
        snapshot.max_listed = 2;
        assert_eq!(snapshot.room_problem(), Some(LookError::CardFull));
        assert!(snapshot.card_full());
    }

    #[test]
    fn an_individual_cover_is_no_variant_of_the_default_cover() {
        let mut snapshot = parse_looks(LIST.as_bytes()).unwrap();
        let statement = Cover::for_name("Kodak Tri-X 400");
        let trix = &mut snapshot.looks[1];
        trix.source = LookSource::Official;
        trix.default_cover = Some(statement.film_box());
        for (cover, individual) in [
            (statement.clone(), false),
            (statement.film_box(), false),
            (Cover::for_name("Mine"), true),
        ] {
            trix.cover = Some(cover);
            assert_eq!(trix.individual_cover(), individual);
        }
        assert_eq!(trix.cover_group(), CoverGroup::Mono);
        assert!(snapshot.has_individual_covers(CoverGroup::Mono));
        assert!(!snapshot.has_individual_covers(CoverGroup::Color));
    }

    #[test]
    fn maps_problem_documents_and_failed_records() {
        assert_eq!(
            look_error(409, r#"{"status":409,"code":"dependency_in_use"}"#),
            LookError::InUse
        );
        assert_eq!(
            look_error(
                409,
                r#"{"requestId":"x","status":"failed","error":{"status":409,"code":"active_look"}}"#
            ),
            LookError::ActiveLook
        );
        assert_eq!(
            look_error(409, r#"{"status":409,"code":"context_changed"}"#),
            LookError::Changed
        );
        assert_eq!(
            look_error(409, r#"{"code":"software_pending"}"#),
            LookError::UpdatePending
        );
        assert_eq!(
            look_error(507, r#"{"status":507,"code":"cover_not_saved"}"#),
            LookError::CoverNotSaved
        );
        assert_eq!(
            look_error(422, r#"{"code":"x","detail":"Bad"}"#),
            LookError::Service("Bad".into())
        );
    }

    #[test]
    fn groups_fall_back_by_monochrome() {
        assert_eq!(group_of("velvia-100-v1", false), LookGroup::Slide);
        assert_eq!(group_of("lut-x", true), LookGroup::Mono);
        assert_eq!(group_of("lut-x", false), LookGroup::Negative);
        assert!(LookFilter::Color.accepts(LookGroup::Cinema));
        assert!(!LookFilter::Color.accepts(LookGroup::Mono));
    }

    #[test]
    fn names_follow_the_camera_rule() {
        assert!(valid_name("Portra 400"));
        assert!(!valid_name(""));
        assert!(!valid_name(" Portra"));
        assert!(!valid_name("A name that is too long"));
        assert!(!valid_name("Fuji\u{e9}"));
        assert_eq!(
            name_from_file("Kodak_Portra_400_F-Log2_to_Rec709.cube"),
            "Kodak Portra 400"
        );
        assert_eq!(name_from_file("cube.cube"), "cube");
        assert_eq!(
            name_from_file("Superlongfilenamewithoutspaces.cube"),
            "Superlongfilenam"
        );
    }

    #[test]
    fn import_ids_are_unique_and_valid() {
        let mut taken = BTreeSet::new();
        let id = import_id("Kodak Portra 400", &taken);
        assert_eq!(id, "lut-kodak-portra-400");
        taken.insert(id);
        assert_eq!(
            import_id("Kodak Portra 400", &taken),
            "lut-kodak-portra-400-2"
        );
        let long = import_id("A very long name for a look indeed", &BTreeSet::new());
        assert!(long.len() <= ID_MAX - 3);
        assert!(grkit_core::effects::valid_id(&long));
    }

    #[test]
    fn moves_a_shown_look_past_the_hidden_ones() {
        let looks = [("a", true), ("b", false), ("c", true), ("d", true)];
        assert_eq!(move_shown(looks, 0, 2), ["c", "b", "d", "a"]);
    }

    #[test]
    fn parses_the_site_catalog() {
        let body = serde_json::json!({
            "schema": grkit_core::site::CATALOG_SCHEMA,
            "target": "gr3-2.21",
            "base_gamma": vec![0u16; 256],
            "preview": {},
            "effects": [{"id":"velvia-100-v1","name":"Velvia 100","monochrome":false,
                "file":"0A1B2C3D.GRF","sha256":"00","icon":"","look":"x","origin":"x","license":"x","preview":"p",
                "cover":{"template":"box","brand":"FUJIFILM","name":"VELVIA","number":"100","band":true}}]
        });
        let catalog = parse_catalog(&serde_json::to_vec(&body).unwrap()).unwrap();
        assert_eq!(catalog.looks[0].name, "Velvia 100");
        assert_eq!(
            catalog.looks[0].shown_cover().to_json(),
            body["effects"][0]["cover"]
        );
        let mut bad = body.clone();
        bad["effects"][0]["file"] = "../x".into();
        assert!(parse_catalog(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
}
