//! In-memory Looks of the demo camera. Follows the camera service rules that
//! the UI depends on: official Looks change only their cover, a Look in use
//! needs Off before deletion, recipes block deletion, and loading is capped.
//! The covers of the built-in Looks follow a scheme as on a camera.

use grkit_core::{
    cover::{Cover, Scheme, Variant},
    effects,
};

use super::look_media;
use super::looks::{
    CatalogLook, CoverGroup, Look, LookError, LookSource, LookUse, LooksSnapshot, PendingChange,
    valid_name,
};

const PORTRA: &str = include_str!("../../../effects/portra-400-endura-v1/effect.json");
const KODACHROME: &str = include_str!("../../../effects/kodachrome-64-v1/effect.json");
const TRIX: &str = include_str!("../../../effects/trix-400-mono-v1/effect.json");
const ETERNA: &str = include_str!("../../../effects/eterna-500t-3523xd-v1/effect.json");
const VELVIA: &str = include_str!("../../../effects/velvia-100-v1/effect.json");
const GOLD: &str = include_str!("../../../effects/gold-200-premier-v1/effect.json");
const PROVIA: &str = include_str!("../../../effects/provia-100f-v1/effect.json");
const EKTAR: &str = include_str!("../../../effects/ektar-100-edge-v1/effect.json");
const HP5: &str = include_str!("../../../effects/hp5-plus-400-mono-v1/effect.json");
const INSTAX: &str = include_str!("../../../effects/instax-v1/effect.json");
const DEFINITIONS: [&str; 10] = [
    PORTRA, KODACHROME, TRIX, ETERNA, VELVIA, GOLD, PROVIA, EKTAR, HP5, INSTAX,
];

/// The demo's imported Look.
pub const DEMO_CUSTOM_ID: &str = "lut-my-lut";

#[derive(Clone, Debug)]
struct Entry {
    id: String,
    name: String,
    enabled: bool,
    source: LookSource,
    grf: Vec<u8>,
    /// The number of recipes and mode-dial sets that use the Look.
    uses: Vec<LookUse>,
    /// The file change that waits for the next start, as on a camera.
    pending: Option<PendingChange>,
    /// The cover that draws the icon, when the camera knows it.
    cover: Option<Cover>,
    /// The base cover of a built-in Look (`grkit_core::cover::Variant`).
    base: Option<Cover>,
    /// A B&W official Look.
    monochrome: bool,
}

impl Entry {
    /// A built-in Look whose cover is no variant of its base cover.
    fn individual(&self) -> bool {
        self.base
            .as_ref()
            .zip(self.cover.as_ref())
            .is_some_and(|(base, cover)| base.variant_of(cover).is_none())
    }

    fn group(&self) -> CoverGroup {
        match self.monochrome {
            true => CoverGroup::Mono,
            false => CoverGroup::Color,
        }
    }

    /// Give the Look `cover`, which the camera shows after a start.
    fn redraw(&mut self, cover: Cover) {
        if self.cover.as_ref() == Some(&cover) {
            return;
        }
        if let Ok(grf) = look_media::with_cover(&self.grf, &cover) {
            self.grf = grf;
            self.cover = Some(cover);
            self.pending.get_or_insert(PendingChange::Update);
        }
    }
}

/// Demo camera Looks.
#[derive(Clone, Debug)]
pub struct FakeLooks {
    entries: Vec<Entry>,
    selected: Option<String>,
    pub max_loaded: usize,
    pub max_listed: usize,
    /// A software update waits on the camera: changes are refused.
    pub update_pending: bool,
    /// The scheme last set, for a group without built-in Looks.
    chosen: Option<Scheme>,
}

/// The cover of a bundled effect definition, as the release statement has it.
fn definition_cover(definition: &str) -> Option<serde_json::Value> {
    let value = effects::parse_json(definition.as_bytes()).ok()?;
    value.get("icon").cloned()
}

/// The statement cover of a bundled official Look.
fn official_base(id: &str) -> Option<Cover> {
    DEFINITIONS
        .iter()
        .find(|definition| definition.contains(&format!("\"{id}\"")))
        .and_then(|definition| definition_cover(definition))
        .and_then(|cover| Cover::from_json(&cover).ok())
}

fn entry(definition: &str, enabled: bool) -> Entry {
    let grf = look_media::compile(definition).expect("bundled effect compiles");
    let (id, name, monochrome) = look_media::label(&grf).expect("bundled effect decodes");
    let cover = definition_cover(definition).and_then(|cover| Cover::from_json(&cover).ok());
    Entry {
        id,
        name,
        enabled,
        source: LookSource::Official,
        grf,
        uses: Vec::new(),
        pending: None,
        base: cover.clone(),
        cover,
        monochrome,
    }
}

/// The official Looks the demo catalog offers, with their GRF files.
pub fn demo_catalog() -> Vec<(CatalogLook, Vec<u8>)> {
    DEFINITIONS
        .iter()
        .map(|definition| {
            let grf = look_media::compile(definition).expect("bundled effect compiles");
            let (id, name, monochrome) = look_media::label(&grf).expect("bundled effect decodes");
            let file = grkit_core::effects::filename(&id).expect("valid id");
            let look = CatalogLook {
                id,
                name,
                monochrome,
                file,
                sha256: grkit_core::packaging::sha256(&grf),
                cover: definition_cover(definition),
            };
            (look, grf)
        })
        .collect()
}

impl FakeLooks {
    /// Portra 400 in use and used by two recipes, an imported Look, and
    /// Velvia 100 turned off.
    pub fn demo() -> Self {
        let mut portra = entry(PORTRA, true);
        portra.uses = vec![
            LookUse::Recipe {
                name: Some("Street".into()),
                number: 0,
            },
            LookUse::Dial { mode: 1 },
        ];
        let gold = entry(GOLD, true);
        let cover = Cover::for_name("My LUT");
        let custom = Entry {
            id: DEMO_CUSTOM_ID.into(),
            name: "My LUT".into(),
            enabled: true,
            source: LookSource::Custom,
            grf: look_media::relabel(&gold.grf, DEMO_CUSTOM_ID, "My LUT", &cover).expect("copy"),
            uses: Vec::new(),
            pending: None,
            cover: Some(cover),
            base: None,
            monochrome: false,
        };
        Self {
            entries: vec![
                portra,
                entry(KODACHROME, true),
                entry(TRIX, true),
                custom,
                entry(ETERNA, true),
                entry(VELVIA, false),
            ],
            selected: Some("portra-400-endura-v1".into()),
            max_loaded: 40,
            max_listed: 64,
            update_pending: false,
            chosen: None,
        }
    }

    /// The scheme of the built-in Looks without an individual cover, as the
    /// camera derives it (`Scheme::of_or`).
    fn scheme(&self) -> Scheme {
        let votes = self.entries.iter().filter_map(|e| {
            let base = e.base.as_ref().filter(|_| !e.individual())?;
            let variant = e.cover.as_ref().and_then(|cover| base.variant_of(cover));
            Some((e.monochrome, variant.unwrap_or(Variant::Gray)))
        });
        Scheme::of_or(votes, self.chosen.unwrap_or(Scheme::NEW_CARD))
    }

    /// Give each built-in Look without an individual cover the cover of
    /// `scheme`.
    pub fn set_scheme(&mut self, scheme: Scheme) -> Result<(), LookError> {
        self.writable()?;
        for entry in self.entries.iter_mut().filter(|e| !e.individual()) {
            if let Some(base) = entry.base.clone() {
                entry.redraw(scheme.cover(&base, entry.monochrome));
            }
        }
        self.chosen = Some(scheme);
        Ok(())
    }

    /// Put the individual covers of `group` back to the scheme.
    pub fn reset_covers(&mut self, group: CoverGroup) -> Result<(), LookError> {
        self.writable()?;
        let scheme = self.scheme();
        for entry in self
            .entries
            .iter_mut()
            .filter(|e| e.individual() && e.group() == group)
        {
            if let Some(base) = entry.base.clone() {
                entry.redraw(scheme.cover(&base, entry.monochrome));
            }
        }
        Ok(())
    }

    pub fn snapshot(&self) -> LooksSnapshot {
        let scheme = self.scheme();
        let mut loaded = 0;
        let looks = self
            .entries
            .iter()
            .enumerate()
            .map(|(position, e)| {
                let is_loaded =
                    e.enabled && e.pending != Some(PendingChange::Add) && loaded < self.max_loaded;
                if is_loaded {
                    loaded += 1;
                }
                Look {
                    id: e.id.clone(),
                    name: e.name.clone(),
                    content_hash: grkit_core::packaging::sha256(&e.grf),
                    enabled: e.enabled,
                    position,
                    source: e.source,
                    loaded: is_loaded,
                    selected: self.selected.as_deref() == Some(&e.id),
                    used_by: e.uses.len(),
                    uses: e.uses.clone(),
                    pending: e.pending,
                    cover: e.cover.clone(),
                    default_cover: e.base.as_ref().map(|base| scheme.cover(base, e.monochrome)),
                }
            })
            .collect();
        LooksSnapshot {
            looks,
            loaded,
            max_loaded: self.max_loaded,
            max_listed: self.max_listed,
            boot_id: "demo-boot".into(),
            card_id: "demo-card".into(),
            official_covers: scheme,
        }
    }

    /// Mark a Look as changed until the next start.
    pub fn set_pending(&mut self, id: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.pending = Some(PendingChange::Update);
        }
    }

    /// Set the recipes and mode-dial sets that use a Look.
    pub fn set_uses(&mut self, id: &str, uses: Vec<LookUse>) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.uses = uses;
        }
    }

    pub fn content(&self, id: &str) -> Option<Vec<u8>> {
        self.entries
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.grf.clone())
    }

    fn writable(&self) -> Result<(), LookError> {
        if self.update_pending {
            return Err(LookError::UpdatePending);
        }
        Ok(())
    }

    fn find(&mut self, id: &str) -> Result<&mut Entry, LookError> {
        self.entries
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or(LookError::NotFound)
    }

    fn enabled_count(&self) -> usize {
        self.entries.iter().filter(|e| e.enabled).count()
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), LookError> {
        self.writable()?;
        if enabled && self.enabled_count() >= self.max_loaded {
            return Err(LookError::NoRoom);
        }
        let entry = self.find(id)?;
        if entry.pending == Some(PendingChange::Add) {
            return Err(LookError::RestartRequired);
        }
        entry.enabled = enabled;
        Ok(())
    }

    pub fn rename(&mut self, id: &str, name: &str) -> Result<(), LookError> {
        self.writable()?;
        if !valid_name(name) {
            return Err(LookError::InvalidName);
        }
        let entry = self.find(id)?;
        if entry.source == LookSource::Unknown {
            return Err(LookError::OfficialReadOnly);
        }
        if entry.name == name {
            return Ok(());
        }
        // As the camera does: the default cover names the Look, so it follows
        // the new name when that fits. Another cover stays.
        let (mut effect, icon) = effects::decode(&entry.grf).map_err(|_| LookError::InvalidName)?;
        let old = Cover::for_name(&entry.name);
        let redrawn = entry
            .cover
            .as_ref()
            .map_or_else(|| old.matches(&icon), |cover| *cover == old)
            .then(|| Cover::for_name(name))
            .and_then(|cover| Some((cover.pixels().ok()?, cover)));
        effect.name = name.to_owned();
        entry.grf = effect
            .encode(redrawn.as_ref().map_or(&icon, |(pixels, _)| pixels))
            .map_err(|_| LookError::InvalidName)?;
        entry.name = name.to_owned();
        if let Some((_, cover)) = redrawn {
            entry.cover = Some(cover);
        }
        entry.pending.get_or_insert(PendingChange::Update);
        Ok(())
    }

    pub fn delete(&mut self, id: &str, active_off: bool) -> Result<(), LookError> {
        self.writable()?;
        let selected = self.selected.as_deref() == Some(id);
        let entry = self.find(id)?;
        if !entry.uses.is_empty() {
            return Err(LookError::InUse);
        }
        if selected && !active_off {
            return Err(LookError::ActiveLook);
        }
        self.entries.retain(|e| e.id != id);
        if selected {
            self.selected = None;
        }
        Ok(())
    }

    pub fn set_order(&mut self, ids: &[String]) -> Result<(), LookError> {
        self.writable()?;
        let mut current: Vec<&String> = self.entries.iter().map(|e| &e.id).collect();
        let mut wanted: Vec<&String> = ids.iter().collect();
        current.sort();
        wanted.sort();
        if current != wanted {
            return Err(LookError::Changed);
        }
        let mut entries = Vec::with_capacity(ids.len());
        for id in ids {
            let at = self.entries.iter().position(|e| &e.id == id).unwrap();
            entries.push(self.entries.remove(at));
        }
        self.entries = entries;
        Ok(())
    }

    /// Add a Look, replace the content of an imported one, or give an
    /// official Look another name and cover. `cover` must draw the icon of
    /// `grf`.
    pub fn put(
        &mut self,
        grf: &[u8],
        official: bool,
        cover: Option<Cover>,
    ) -> Result<(), LookError> {
        self.writable()?;
        let (effect, icon) = effects::decode(grf).map_err(|e| LookError::Service(e.to_string()))?;
        if cover.as_ref().is_some_and(|cover| !cover.matches(&icon)) {
            return Err(LookError::Service(
                "The cover is invalid or does not draw the icon of the GRF.".into(),
            ));
        }
        if let Some(existing) = self.entries.iter_mut().find(|e| e.id == effect.id) {
            match existing.source {
                LookSource::Custom | LookSource::Native => {}
                // An official Look changes only its name and cover.
                LookSource::Official
                    if cover.is_some()
                        && effects::body_hash(grf).ok()
                            == effects::body_hash(&existing.grf).ok() => {}
                _ => return Err(LookError::OfficialReadOnly),
            }
            existing.name = effect.name;
            existing.grf = grf.to_vec();
            existing.cover = cover
                .or(existing.cover.take())
                .filter(|cover| cover.matches(&icon));
            existing.pending.get_or_insert(PendingChange::Update);
            return Ok(());
        }
        if self.entries.len() >= self.max_listed {
            return Err(LookError::CardFull);
        }
        if self.enabled_count() >= self.max_loaded {
            return Err(LookError::NoRoom);
        }
        let monochrome = official && effect.monochrome();
        self.entries.push(Entry {
            base: official.then(|| official_base(&effect.id)).flatten(),
            monochrome,
            id: effect.id,
            name: effect.name,
            enabled: true,
            source: if official {
                LookSource::Official
            } else {
                LookSource::Custom
            },
            grf: grf.to_vec(),
            uses: Vec::new(),
            pending: Some(PendingChange::Add),
            cover,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_follows_the_service_rules() {
        let mut looks = FakeLooks::demo();
        let snapshot = looks.snapshot();
        assert_eq!(snapshot.looks.len(), 6);
        assert!(snapshot.get("portra-400-endura-v1").unwrap().selected);
        assert!(!snapshot.get("velvia-100-v1").unwrap().loaded);
        assert_eq!(
            looks.delete("portra-400-endura-v1", true),
            Err(LookError::InUse)
        );
        looks.rename("kodachrome-64-v1", "Mine").unwrap();
        looks.rename(DEMO_CUSTOM_ID, "Night").unwrap();
        let custom = looks.snapshot().get(DEMO_CUSTOM_ID).cloned().unwrap();
        assert_eq!(custom.name, "Night");
        // The default cover follows the name. The camera reads both when it
        // starts again.
        assert_eq!(custom.cover, Some(Cover::for_name("Night")));
        assert_eq!(custom.pending, Some(PendingChange::Update));
        looks.max_loaded = looks.snapshot().loaded;
        assert_eq!(
            looks.set_enabled("velvia-100-v1", true),
            Err(LookError::NoRoom)
        );
        let mut ids = looks.snapshot().ids();
        ids.reverse();
        looks.set_order(&ids).unwrap();
        assert_eq!(looks.snapshot().ids(), ids);
        looks.update_pending = true;
        assert_eq!(
            looks.set_enabled("velvia-100-v1", false),
            Err(LookError::UpdatePending)
        );
    }

    #[test]
    fn a_scheme_and_a_reset_redraw_the_built_in_covers() {
        let mut looks = FakeLooks::demo();
        let id = "kodachrome-64-v1";
        let own = Cover::for_name("Slide 64");
        let grf = look_media::with_cover(&looks.content(id).unwrap(), &own).unwrap();
        looks.put(&grf, false, Some(own.clone())).unwrap();
        looks.set_scheme(Scheme::ColorFilmBox).unwrap();
        let snapshot = looks.snapshot();
        assert_eq!(snapshot.official_covers, Scheme::ColorFilmBox);
        let portra = snapshot.get("portra-400-endura-v1").unwrap();
        assert_eq!(portra.cover, portra.default_cover);
        assert_ne!(portra.cover, official_base(&portra.id));
        let trix = snapshot.get("trix-400-mono-v1").unwrap();
        assert_eq!(trix.cover, official_base(&trix.id));
        assert!(snapshot.has_individual_covers(CoverGroup::Color));
        looks.reset_covers(CoverGroup::Color).unwrap();
        let snapshot = looks.snapshot();
        assert!(!snapshot.has_individual_covers(CoverGroup::Color));
        assert_eq!(snapshot.official_covers, Scheme::ColorFilmBox);
    }

    #[test]
    fn deleting_the_look_in_use_needs_off() {
        let mut looks = FakeLooks::demo();
        looks.entries[0].uses.clear();
        assert_eq!(
            looks.delete("portra-400-endura-v1", false),
            Err(LookError::ActiveLook)
        );
        looks.delete("portra-400-endura-v1", true).unwrap();
        assert!(looks.snapshot().looks.iter().all(|l| !l.selected));
    }

    #[test]
    fn catalog_adds_official_looks() {
        let mut looks = FakeLooks::demo();
        let (look, grf) = demo_catalog()
            .into_iter()
            .find(|(l, _)| l.id == "provia-100f-v1")
            .unwrap();
        looks.put(&grf, true, look.film_cover()).unwrap();
        let added = looks.snapshot().get(&look.id).cloned().unwrap();
        assert_eq!(added.source, LookSource::Official);
        // The camera lists a new Look only after it starts again.
        assert_eq!(added.pending, Some(PendingChange::Add));
        assert!(!added.loaded && !added.listed());
        assert_eq!(
            looks.set_enabled(&look.id, false),
            Err(LookError::RestartRequired)
        );
    }

    #[test]
    fn official_looks_change_their_name_and_cover_but_not_their_colors() {
        let mut looks = FakeLooks::demo();
        let id = "kodachrome-64-v1";
        let grf = looks.content(id).unwrap();
        let cover = Cover::for_name("Slide 64");
        let covered = look_media::with_cover(&grf, &cover).unwrap();
        let (mut recolored, icon) = effects::decode(&covered).unwrap();
        recolored.matrix[0] += 1;
        for (grf, cover) in [
            (covered, None),
            (recolored.encode(&icon).unwrap(), Some(cover.clone())),
        ] {
            assert_eq!(
                looks.put(&grf, false, cover),
                Err(LookError::OfficialReadOnly)
            );
        }
        let renamed = look_media::relabel(&grf, id, "Mine", &cover).unwrap();
        looks.put(&renamed, false, Some(cover.clone())).unwrap();
        let look = looks.snapshot().get(id).cloned().unwrap();
        assert_eq!(
            (look.source, look.name.as_str(), look.cover, look.pending),
            (
                LookSource::Official,
                "Mine",
                Some(cover),
                Some(PendingChange::Update)
            )
        );
    }
}
