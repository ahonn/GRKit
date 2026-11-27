//! Film Look HTTP resources.
//!
//! The Looks are the effect files of the card (`GRKIT/EFFECTS`) and the
//! choices of the running camera. Two kinds of change exist:
//!
//! - On or off and the order are settings of the camera. The App task sets
//!   them at once, as the checkbox and Order in MENU do
//!   (`runtime_backend`).
//! - A new, renamed, replaced, or removed Look is a change of a file. The
//!   service writes the file, and the camera reads it at its next start. A
//!   removed Look also turns off at once, so the camera menu hides it.
//!
//! A Look on the camera has its file in `GRKIT/EFFECTS`. A Look on the card
//! only has its file in `GRKIT/STORED`: the camera does not read it, so the
//! card holds more Looks than the camera loads. Moving a Look between the
//! two folders applies at the next start, as a new or removed file does. A
//! new Look goes to the camera while the camera has room for it, else to
//! the card.
//!
//! Each request answers when it ends: the service keeps no record of a Look
//! request. On, off, and the order are absolute values, and a file write
//! replaces the file, so a request whose answer did not arrive can be sent
//! again. A change names the start of the camera that the phone read
//! (`X-GRKit-Boot-Id`): the Looks that wait for a start differ after it.
//!
//! Each Look carries the cover that draws its icon, when the service knows
//! one (`covers`). A content write can bring a cover in `X-GRKit-Cover`.
//! The covers of the built-in Looks (official Looks and native looks)
//! without an individual cover follow one scheme (`cover::Scheme`), which
//! the covers on the card give.
use super::{lock, problem, try_lock, write_checked, RequestContext, Resources, Result};
use crate::{
    api::Reply,
    camera_looks::{self, look_key, RuntimeLooks, UseOrigin},
    changes::{Changed, Changes},
    looks::{self, LookFile, MAX_LISTED},
    protocol::Problem,
    route::LookRoute,
    runtime_backend::RuntimeBackend,
    Request,
};
use covers::{built_in, Covers, Official};
use grkit_core::{
    cover::{Cover, Scheme},
    effects, imports, packaging, recipes,
};
use grkit_protocol::looks as wire;
use serde_json::{json, Value};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

/// The camera loads at most this many Looks that are On. It is also the most
/// Looks that the service puts on the camera: each Look there loads.
pub const MAX_LOADED: usize = grkit_core::abi::MAX_COUNT;
/// The longest `X-GRKit-Cover` value. A cover is less than 512 bytes of JSON.
const MAX_COVER: usize = 1024;

/// What the service keeps about one card while the camera stays on.
#[derive(Default)]
pub(super) struct Session {
    card_id: String,
    /// The content hash of each Look when the service first read the card:
    /// the content that the running camera loaded.
    started: BTreeMap<String, String>,
    /// The official Looks of the installed release and the covers.
    covers: Covers,
    /// The runtime answered a Looks read.
    verified: bool,
    /// The Looks of `GRKIT/STORED` that the service read, by file name
    /// (`stored_files`).
    stored: BTreeMap<String, Known>,
    /// What a phone changed in each Look since the session started, by ID,
    /// so that a pending change names itself (`LookRuntime::edits`).
    edits: BTreeMap<String, BTreeSet<wire::LookEdit>>,
}

/// A Look on the card only as the service last read it, without its bytes.
struct Known {
    stamp: looks::Stamp,
    file: LookFile,
    source: &'static str,
}

impl Session {
    /// The session of the card `card_id`. The first read of a card starts a
    /// new one, with `files`, the Looks on the camera.
    fn current<'a>(
        session: &'a mut Option<Session>,
        card_id: &str,
        context: &RequestContext<'_>,
        files: &[LookFile],
    ) -> &'a mut Session {
        if session
            .as_ref()
            .is_none_or(|session| session.card_id != card_id)
        {
            *session = Some(Session {
                card_id: card_id.to_owned(),
                started: files
                    .iter()
                    .map(|file| (file.id.clone(), file.content_hash.clone()))
                    .collect(),
                covers: Covers::new(
                    crate::trust::installed_release(context.root, context.release_keys)
                        .map(Official::of)
                        .unwrap_or_default(),
                ),
                ..Default::default()
            });
        }
        session.as_mut().expect("session")
    }

    fn source(&self, file: &LookFile) -> &'static str {
        match self.covers.official.source(file) {
            "custom" if grkit_core::cover::native_cover(&file.id).is_some() => "native",
            source => source,
        }
    }

    /// The Looks of `GRKIT/STORED`, without their bytes, and the source of
    /// each. `GRKIT/STORED` holds up to `looks::MAX_STORED` files, and a read
    /// of each on every request costs the camera time and heat. So the
    /// service reads a file once, for its ID, name, hash, source, and cover,
    /// and again only when its stamp changes. Err when the folder cannot be
    /// listed.
    fn stored_files(&mut self, root: &Path) -> anyhow::Result<Vec<(LookFile, &'static str)>> {
        let mut known = std::mem::take(&mut self.stored);
        let mut files = Vec::new();
        for entry in looks::list_stored(root)?.entries {
            let Some(stamp) = entry.stamp() else {
                continue;
            };
            let look = match known.remove(&entry.name).filter(|look| look.stamp == stamp) {
                Some(look) => look,
                None => {
                    let Ok(mut file) = entry.read() else {
                        continue;
                    };
                    let source = self.source(&file);
                    self.covers.learn(root, &file, source);
                    file.bytes = Vec::new();
                    Known {
                        stamp,
                        file,
                        source,
                    }
                }
            };
            files.push((look.file.clone(), look.source));
            self.stored.insert(entry.name, look);
        }
        Ok(files)
    }

    /// Forget the file of the Look `id` on the card only before a change:
    /// a write in the same second as the last read can keep its stamp.
    fn forget(&mut self, id: &str) {
        let name = effects::filename(id).ok();
        self.stored
            .retain(|file, look| look.file.id != id && name.as_ref() != Some(file));
    }
}

/// The uses of a Look that its info names; `usedBy` counts them all.
const USES_LISTED: usize = 8;

/// A recipe (its name, or none for the automatic one, and its number) or a
/// user mode of the mode dial.
fn wire_use(origin: &UseOrigin) -> wire::LookUse {
    match origin {
        UseOrigin::Recipe { slot, number, name } => wire::LookUse::Recipe {
            slot: *slot,
            number: *number,
            name: name.clone(),
        },
        UseOrigin::Dial { mode } => wire::LookUse::Dial { mode: *mode },
        UseOrigin::Unknown => wire::LookUse::Unknown,
    }
}

fn wire_source(source: &str) -> wire::LookSource {
    match source {
        "official" => wire::LookSource::Official,
        "custom" => wire::LookSource::Custom,
        "native" => wire::LookSource::Native,
        _ => wire::LookSource::Unknown,
    }
}

fn to_json(value: &impl serde::Serialize) -> Value {
    serde_json::to_value(value).expect("a wire type serializes")
}

/// One Look of the list.
pub(super) struct Look {
    /// Without its bytes for a Look on the card only (`Look::bytes`).
    file: LookFile,
    /// The file is in `GRKIT/STORED`: the Look is on the card only.
    stored: bool,
    source: &'static str,
    enabled: bool,
    loaded: bool,
    selected: bool,
    /// The recipes and mode-dial sets that name the Look.
    used_by: usize,
    /// Where the first `USES_LISTED` of them are.
    uses: Vec<UseOrigin>,
    /// The change of the file that waits for the next start.
    pending: Option<wire::PendingChange>,
    /// What a phone changed since the session started.
    edits: Vec<wire::LookEdit>,
    /// The key in the running camera, when the camera lists the Look.
    key: Option<u32>,
    /// The cover that draws the icon of the file, when the service knows it.
    cover: Option<Cover>,
    /// For a built-in Look, the cover that the scheme of the card gives it.
    default_cover: Option<Cover>,
}
impl Look {
    /// The Look of `file`, with the recipes and mode-dial sets that name
    /// it: not listed by the camera, nothing pending, and no cover yet.
    fn from_file(
        file: LookFile,
        stored: bool,
        source: &'static str,
        runtime: &RuntimeLooks,
    ) -> Self {
        let code = u32::from(recipes::effect_code(&file.id));
        let uses = || runtime.uses.iter().filter(|used| used.code == code);
        Look {
            stored,
            source,
            enabled: false,
            loaded: false,
            selected: false,
            used_by: uses().count(),
            uses: uses()
                .take(USES_LISTED)
                .map(|used| used.origin.clone())
                .collect(),
            pending: None,
            edits: Vec::new(),
            key: None,
            cover: None,
            default_cover: None,
            file,
        }
    }

    fn configuration(&self) -> wire::LookConfiguration {
        wire::LookConfiguration {
            id: self.file.id.clone(),
            name: self.file.name.clone(),
            content_hash: self.file.content_hash.clone(),
            enabled: self.enabled,
            place: if self.stored {
                wire::Place::Card
            } else {
                wire::Place::Camera
            },
            cover: self.cover.as_ref().map(Cover::to_json),
            default_cover: self.default_cover.as_ref().map(Cover::to_json),
        }
    }
    fn info(&self) -> wire::LookRuntime {
        wire::LookRuntime {
            id: self.file.id.clone(),
            source: wire_source(self.source),
            loaded: self.loaded,
            selected: self.selected,
            used_by: self.used_by,
            uses: self.uses.iter().map(wire_use).collect(),
            pending: self.pending,
            edits: self.edits.clone(),
        }
    }
    /// The file of the Look: the bytes of the scan, or the file on the card
    /// when it still has the content of the scan.
    fn bytes(&self, root: &Path) -> Result<Cow<'_, [u8]>> {
        if !self.file.bytes.is_empty() {
            return Ok(Cow::Borrowed(&self.file.bytes));
        }
        looks::read_grf(&folder(root, self.stored).join(&self.file.file))
            .ok()
            .filter(|bytes| packaging::sha256(bytes) == self.file.content_hash)
            .map(Cow::Owned)
            .ok_or_else(looks_unavailable)
    }
}

/// The Looks in the order of the camera menu, then the Looks that wait for
/// the next start, and the Looks on the card only.
pub(super) struct View {
    looks: Vec<Look>,
    /// The Looks of `GRKIT/STORED`, by name.
    stored: Vec<Look>,
    /// `GRKIT/STORED` cannot be read: the list has no Looks on the card
    /// only, and no Look goes there.
    stored_unavailable: bool,
    /// Files that share an ID with an earlier file: the camera lists one.
    duplicates: BTreeSet<String>,
    /// The file in `GRKIT/STORED` of each Look on the camera that has a copy
    /// there (`join`).
    copies: BTreeMap<String, String>,
    /// The file names that the package list leaves out: the camera does not
    /// list a file with one of these names.
    left_out: BTreeSet<String>,
    /// The color scheme of the covers of the built-in Looks.
    scheme: Scheme,
    /// The effects that are Off in the camera, also one whose file is gone.
    off: BTreeSet<u32>,
}
impl View {
    fn get(&self, id: &str) -> Result<&Look> {
        self.all()
            .find(|look| look.file.id == id)
            .ok_or_else(|| problem(404, "look_not_found", "The card has no Look with this ID."))
    }
    fn all(&self) -> impl Iterator<Item = &Look> {
        self.looks.iter().chain(&self.stored)
    }
    fn ids(&self) -> Vec<&str> {
        self.looks
            .iter()
            .map(|look| look.file.id.as_str())
            .collect()
    }
    fn list(&self, context: &RequestContext<'_>) -> wire::LookList {
        let list = |looks: &[Look]| -> Vec<wire::Look> {
            looks
                .iter()
                .enumerate()
                .map(|(position, look)| wire::Look {
                    configuration: look.configuration(),
                    position,
                    runtime: look.info(),
                })
                .collect()
        };
        wire::LookList {
            looks: list(&self.looks),
            stored: list(&self.stored),
            official_covers: Some(self.scheme.id().to_owned()),
            counts: wire::Counts {
                listed: self.looks.len(),
                loaded: self.looks.iter().filter(|look| look.enabled).count(),
                stored: self.stored.len(),
            },
            limits: wire::Limits {
                listed_looks: MAX_LISTED,
                loaded_looks: MAX_LOADED,
                stored_looks: Some(looks::MAX_STORED),
            },
            diagnostics: self
                .stored_unavailable
                .then(|| wire::Diagnostic {
                    code: wire::STORED_UNREADABLE.to_owned(),
                })
                .into_iter()
                .collect(),
            boot_id: context.boot_id.to_owned(),
            // A view needs the card identity (`Resources::view`).
            card_id: context.card_id.clone().unwrap_or_default(),
        }
    }
    /// The camera has room for one more Look.
    fn camera_room(&self) -> bool {
        self.looks.len() < MAX_LOADED.min(MAX_LISTED)
    }
}

/// The Film Look of a recipe code: on the camera, or on the card only.
pub(super) struct EffectName {
    pub code: u16,
    pub id: String,
    pub name: String,
    pub stored: bool,
}

impl Resources {
    /// The Looks state for /props, without a runtime command.
    pub fn runtime_properties(&self, card_id: &str) -> Value {
        let verified = lock(&self.session)
            .as_ref()
            .is_some_and(|session| session.card_id == card_id && session.verified);
        json!({"state":if verified {"ready"} else {"unverified"},"busyReason":null,
            "capabilities":{"read":true,"write":true,"appliesAtRestart":["content","name","delete","covers","place"]}})
    }

    /// The Film Look of each recipe code, without a runtime command and
    /// without covers: the cheapest read for a recipe request. A Look on the
    /// camera comes before a Look on the card only. Empty while the Looks
    /// cannot be read.
    pub(super) fn effect_names(&self, context: &RequestContext<'_>) -> Vec<EffectName> {
        let Some(card_id) = context.card_id.as_deref() else {
            return Vec::new();
        };
        let Ok(scan) = looks::scan(context.root) else {
            return Vec::new();
        };
        let left_out = looks::left_out(context.root);
        let mut guard = lock(&self.session);
        let session = Session::current(&mut guard, card_id, context, &scan.files);
        let stored = session.stored_files(context.root).unwrap_or_default();
        let camera = scan
            .files
            .into_iter()
            .filter(|file| !left_out.contains(&file.file.to_ascii_uppercase()));
        camera
            .map(|file| (file, false))
            .chain(stored.into_iter().map(|(file, _)| (file, true)))
            .map(|(file, stored)| EffectName {
                code: recipes::effect_code(&file.id),
                id: file.id,
                name: file.name,
                stored,
            })
            .collect()
    }

    /// A request of the Looks, with a method of its route that the service
    /// admitted (`api::Service`).
    pub fn handle(
        &self,
        route: &LookRoute,
        request: &Request,
        body: &[u8],
        context: &RequestContext<'_>,
        changes: &Changes,
    ) -> Reply {
        let result = match request.method.as_str() {
            "GET" => self.read(route, context),
            _ => self.mutate(route, request, body, context, changes),
        };
        result.unwrap_or_else(Reply::problem)
    }

    /// The files of the card joined with the choices of the camera.
    pub(super) fn view(
        &self,
        backend: &mut dyn RuntimeBackend,
        context: &RequestContext<'_>,
    ) -> Result<View> {
        let Some(card_id) = context.card_id.as_deref() else {
            return Err(problem(
                503,
                "card_identity_unavailable",
                "The card identity is not available yet.",
            ));
        };
        let runtime = camera_looks::read(backend)?;
        let scan = looks::scan(context.root).map_err(|_| looks_unavailable())?;
        let mut guard = lock(&self.session);
        let session = Session::current(&mut guard, card_id, context, &scan.files);
        session.verified = true;
        // The Looks on the camera do not depend on `GRKIT/STORED`.
        let stored = session.stored_files(context.root).ok();
        Ok(join(
            context.root,
            scan.files,
            stored,
            &runtime,
            session,
            looks::left_out(context.root),
        ))
    }

    fn read(&self, route: &LookRoute, context: &RequestContext<'_>) -> Result<Reply> {
        let mut backend = try_lock(&self.backend)?;
        let view = self.view(backend.as_mut(), context)?;
        Ok(match route {
            LookRoute::Collection => Reply::json(200, to_json(&view.list(context))),
            LookRoute::Configuration(id) => {
                Reply::json(200, to_json(&view.get(id)?.configuration()))
            }
            LookRoute::Content(id) => {
                let look = view.get(id)?;
                let bytes = look.bytes(context.root)?.into_owned();
                Reply::binary(200, bytes, "application/octet-stream")
                    .header("X-GRKit-Content-SHA256", look.file.content_hash.clone())
            }
            LookRoute::Order => Reply::json(200, json!({"ids": view.ids()})),
        })
    }

    fn mutate(
        &self,
        route: &LookRoute,
        request: &Request,
        body: &[u8],
        context: &RequestContext<'_>,
        changes: &Changes,
    ) -> Result<Reply> {
        if request
            .headers
            .get("x-grkit-cover")
            .is_some_and(|cover| cover.len() > MAX_COVER)
        {
            return Err(problem(
                413,
                "cover_too_large",
                "The cover exceeds its size limit.",
            ));
        }
        let mut backend = try_lock(&self.backend)?;
        let view = self.view(backend.as_mut(), context)?;
        // A problem after the file changed: the answer reports it, and the
        // phone still hears of the change.
        let mut late = None;
        let (status, id) = {
            let mut guard = lock(&self.session);
            let session = guard.as_mut().ok_or_else(coordinator)?;
            if let LookRoute::Content(id) | LookRoute::Configuration(id) = route {
                session.forget(id);
                // The copy that a move left behind goes before the change
                // (`join`).
                if let Some(copy) = view.copies.get(id) {
                    remove_file(context.root, true, copy)?;
                }
            }
            match route {
                LookRoute::Content(id) => {
                    let created = view.get(id).is_err();
                    let stored;
                    (stored, late) = put_content(context.root, &view, session, id, request, body)?;
                    if created && !stored {
                        if let Err(problem) = turn_back_on(backend.as_mut(), &view, id) {
                            late.get_or_insert(problem);
                        }
                    }
                    (if created { 201 } else { 200 }, Some(id))
                }
                LookRoute::Configuration(id) => {
                    let look = view.get(id)?;
                    if request.method == "PATCH" {
                        late = patch(context.root, backend.as_mut(), &view, session, look, body)?;
                    } else {
                        delete(
                            context.root,
                            backend.as_mut(),
                            session,
                            look,
                            request.query.as_deref() == Some("active=off"),
                        )?;
                    }
                    (200, Some(id))
                }
                LookRoute::Order => {
                    order(backend.as_mut(), &view, body)?;
                    (200, None)
                }
                LookRoute::Collection => {
                    return Err(problem(
                        405,
                        "method_not_allowed",
                        "This resource is not mutable.",
                    ))
                }
            }
        };
        // The result is the state after the change, read again from the card
        // and the camera. The view before the change goes first: each view
        // holds the files of the camera.
        drop(view);
        let view = self.view(backend.as_mut(), context)?;
        changes.note(Changed::Looks);
        if let Some(problem) = late {
            return Err(problem);
        }
        let result = match (route, id) {
            (_, Some(id)) => view.get(id).map_or_else(
                |_| json!({"id":id,"removed":true}),
                |look| to_json(&look.configuration()),
            ),
            (_, None) => json!({"ids": view.ids()}),
        };
        Ok(Reply::json(status, result))
    }
}

/// Join the files with the runtime list. A file that the camera does not
/// list waits for the next start. A Look that the camera lists without a
/// file was removed: it is not a Look of the card any more. `stored_files`
/// is None when `GRKIT/STORED` cannot be read.
///
/// A file of `GRKIT/STORED` with the ID of a file of the camera is a
/// duplicate, and a change of the Look is ambiguous. When the two files are
/// the same, the stored file is a copy that a move left behind, after a
/// failed rollback or a power loss. The Look is then on the camera, and the
/// next change of the Look removes the copy first (`mutate`): a Look on
/// both places is never lost, and the person can still move it.
fn join(
    root: &Path,
    files: Vec<LookFile>,
    stored_files: Option<Vec<(LookFile, &'static str)>>,
    runtime: &RuntimeLooks,
    session: &mut Session,
    left_out: BTreeSet<String>,
) -> View {
    let order = runtime.menu_order();
    let mut duplicates = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut hashes = BTreeMap::new();
    let mut looks: Vec<(usize, Look)> = Vec::new();
    for file in files {
        if left_out.contains(&file.file.to_ascii_uppercase()) {
            continue;
        }
        if !seen.insert(file.id.clone()) {
            duplicates.insert(file.id.clone());
            continue;
        }
        hashes.insert(file.id.clone(), file.content_hash.clone());
        let key = look_key(&file.id);
        let listed = runtime.looks.iter().find(|look| look.key == key);
        let source = session.source(&file);
        let changed = session
            .started
            .get(&file.id)
            .is_none_or(|hash| *hash != file.content_hash);
        let rank = listed.map_or(usize::MAX, |_| {
            order
                .iter()
                .position(|other| *other == key)
                .unwrap_or(usize::MAX)
        });
        looks.push((
            rank,
            Look {
                enabled: listed.is_none_or(|look| look.enabled),
                loaded: listed.is_some_and(|look| look.loaded),
                selected: runtime.selected == Some(key),
                pending: match listed {
                    None => Some(wire::PendingChange::Add),
                    Some(_) if changed => Some(wire::PendingChange::Update),
                    Some(_) => None,
                },
                key: listed.map(|look| look.key),
                edits: session
                    .edits
                    .get(&file.id)
                    .map(|edits| edits.iter().copied().collect())
                    .unwrap_or_default(),
                ..Look::from_file(file, false, source, runtime)
            },
        ));
    }
    looks.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.file.id.cmp(&b.1.file.id)));
    let mut looks: Vec<Look> = looks.into_iter().map(|(_, look)| look).collect();
    let on_camera = looks.len();
    let stored_unavailable = stored_files.is_none();
    let mut copies = BTreeMap::new();
    for (file, source) in stored_files.into_iter().flatten() {
        if !seen.insert(file.id.clone()) {
            if hashes.get(&file.id) == Some(&file.content_hash) {
                copies.insert(file.id, file.file);
            } else {
                duplicates.insert(file.id);
            }
            continue;
        }
        looks.push(Look::from_file(file, true, source, runtime));
    }
    // One pass gives each Look its cover: the covers keep only the Looks
    // that it sees.
    session.covers.resolve(root, &mut looks);
    let scheme = session.covers.scheme();
    for look in looks.iter_mut().filter(|look| built_in(look.source)) {
        look.default_cover = session
            .covers
            .default_cover(&look.file.id, look.source, scheme);
    }
    let mut stored = looks.split_off(on_camera);
    stored.sort_by(|a, b| {
        a.file
            .name
            .to_ascii_lowercase()
            .cmp(&b.file.name.to_ascii_lowercase())
            .then(a.file.id.cmp(&b.file.id))
    });
    View {
        scheme,
        looks,
        stored,
        stored_unavailable,
        duplicates,
        copies,
        left_out,
        off: runtime
            .looks
            .iter()
            .filter(|look| !look.enabled)
            .map(|look| look.key)
            .collect(),
    }
}

/// The folder of the file of a Look.
fn folder(root: &Path, stored: bool) -> std::path::PathBuf {
    if stored {
        looks::stored(root)
    } else {
        looks::library(root)
    }
}

/// Write an effect file and read it back. A file that the write did not
/// complete fails the check of the installer at the next start, so the
/// camera skips it.
fn write_file(root: &Path, stored: bool, name: &str, bytes: &[u8]) -> Result<()> {
    let directory = folder(root, stored);
    if stored && fs::create_dir_all(&directory).is_err() {
        return Err(card_write_failed());
    }
    if write_checked(&directory.join(name), bytes) {
        return Ok(());
    }
    Err(card_write_failed())
}

/// Remove an effect file. On the camera, the lookup of the next new name in
/// a folder can fail after a delete in that folder, and Linux keeps that
/// answer. The scan before this call looked up each existing file; one
/// lookup of a name that no file has takes the failure.
fn remove_file(root: &Path, stored: bool, name: &str) -> Result<()> {
    let directory = folder(root, stored);
    fs::remove_file(directory.join(name)).map_err(|_| {
        problem(
            507,
            "card_write_failed",
            "The Look could not be removed from the card.",
        )
    })?;
    crate::update::look_up_unused(&directory);
    Ok(())
}

fn decode(id: &str, bytes: &[u8]) -> Result<(effects::Effect, Vec<u8>)> {
    let (effect, icon) = effects::decode(bytes)
        .map_err(|_| problem(422, "invalid_grf", "The GRF content is invalid."))?;
    if effect.id != id {
        return Err(problem(
            422,
            "id_mismatch",
            "The GRF ID does not match the resource ID.",
        ));
    }
    Ok((effect, icon))
}

/// The cover of `X-GRKit-Cover`, which must draw `icon`, the icon of the
/// GRF of the request.
fn cover_header(request: &Request, icon: &[u8]) -> Result<Option<Cover>> {
    let Some(text) = request.headers.get("x-grkit-cover") else {
        return Ok(None);
    };
    effects::parse_json(text.as_bytes())
        .ok()
        .and_then(|value| Cover::from_json(&value).ok())
        .filter(|cover| cover.matches(icon))
        .map(Some)
        .ok_or_else(|| {
            problem(
                422,
                "cover_mismatch",
                "The cover is invalid or does not draw the icon of the GRF.",
            )
        })
}

/// The file name of `id` in `GRKIT/EFFECTS`, for a Look that goes to the
/// camera.
fn camera_name(root: &Path, view: &View, id: &str) -> Result<String> {
    let mut name = effects::filename(id).map_err(|_| invalid_id())?;
    // The package list hides the usual file name of a Look that the
    // person left out of the package. The Look that the phone adds
    // gets a name that no package file has: the usual names are
    // hexadecimal.
    if view.left_out.contains(&name) {
        name.replace_range(..1, "P");
    }
    // Another ID has this file name: the camera lists that ID.
    if looks::library(root).join(&name).exists() {
        return Err(ambiguous());
    }
    Ok(name)
}

/// The file name of `id` in `GRKIT/STORED`.
fn stored_name(root: &Path, id: &str) -> Result<String> {
    let name = effects::filename(id).map_err(|_| invalid_id())?;
    if looks::stored(root).join(&name).exists() {
        return Err(ambiguous());
    }
    Ok(name)
}

/// A new Look, new content for a Look that the phone made, or a new name
/// or cover for an official Look. A new Look goes to the camera while it has
/// room, else to the card only. A cover goes to the import library after
/// the file. Returns where the file is, and Some problem after the file was
/// written.
fn put_content(
    root: &Path,
    view: &View,
    session: &mut Session,
    id: &str,
    request: &Request,
    bytes: &[u8],
) -> Result<(bool, Option<Problem>)> {
    let (_, icon) = decode(id, bytes)?;
    let cover = cover_header(request, &icon)?;
    if view.duplicates.contains(id) {
        return Err(ambiguous());
    }
    let official = &session.covers.official;
    // An official Look changes only its name and cover: its body stays the
    // official one.
    let verified = official.verified(id, bytes, &icon, cover.is_some());
    let (stored, name) = match view.get(id) {
        Ok(look) if look.source == "official" && verified => (look.stored, look.file.file.clone()),
        Ok(look) if !matches!(look.source, "custom" | "native") => return Err(official_read_only()),
        Ok(look) => (look.stored, look.file.file.clone()),
        Err(_) => {
            // The content of an official ID comes only from its release.
            if official.looks.contains_key(id) && !verified {
                return Err(official_read_only());
            }
            let code = looks::base_code(id);
            // The native page of GR IV, on each camera, so that an ID is
            // usable on each camera. A Look on the card counts: it can go
            // to the camera.
            if code >> 8 == 0x81
                || view
                    .all()
                    .any(|other| looks::base_code(&other.file.id) == code)
            {
                return Err(problem(
                    409,
                    "effect_code_collision",
                    "Choose another Look ID because its runtime code is already in use.",
                ));
            }
            if view.camera_room() {
                (false, camera_name(root, view, id)?)
            } else if view.stored_unavailable {
                return Err(looks_unavailable());
            } else if view.stored.len() < looks::MAX_STORED {
                (true, stored_name(root, id)?)
            } else {
                return Err(capacity());
            }
        }
    };
    // The library takes the cover, and the name of an official Look, before
    // the file changes, so that a library that cannot take them changes
    // nothing.
    let staged = if official.looks.contains_key(id) {
        session
            .covers
            .stage_official(root, id, bytes, cover.as_ref())
            .map_err(|()| library_unavailable())?
    } else if view.get(id).is_ok_and(|look| look.source == "native") {
        session
            .covers
            .stage_native(root, id, bytes, cover.as_ref())
            .map_err(|()| library_unavailable())?
    } else {
        match &cover {
            Some(cover) => Some(
                session
                    .covers
                    .stage(root, id, |imports| {
                        imports::upsert(imports, bytes.to_vec(), cover, false)
                    })
                    .ok_or_else(library_unavailable)?,
            ),
            None => None,
        }
    };
    if let Ok(look) = view.get(id) {
        let edits = content_edits(id, &look.bytes(root)?, bytes)?;
        session
            .edits
            .entry(id.to_owned())
            .or_default()
            .extend(edits);
    }
    write_file(root, stored, &name, bytes)?;
    if let Some(staged) = staged {
        if !session.covers.commit(root, staged) {
            // The Look then shows only a cover that still draws its icon.
            return Ok((stored, Some(cover_not_saved())));
        }
    }
    Ok((stored, None))
}

/// What a new file of a Look changes: its name, its cover (the icon), and
/// the rest of its content.
fn content_edits(id: &str, old: &[u8], new: &[u8]) -> Result<Vec<wire::LookEdit>> {
    let (mut before, old_icon) = decode(id, old)?;
    let (after, new_icon) = decode(id, new)?;
    let mut edits = Vec::new();
    if before.name != after.name {
        edits.push(wire::LookEdit::Name);
    }
    if old_icon != new_icon {
        edits.push(wire::LookEdit::Cover);
    }
    before.name.clone_from(&after.name);
    if before.encode(&new_icon).ok() != after.encode(&new_icon).ok() {
        edits.push(wire::LookEdit::Content);
    }
    Ok(edits)
}

/// A new name, on or off, or a move. Returns Some problem after a move,
/// whose file changed.
fn patch(
    root: &Path,
    backend: &mut dyn RuntimeBackend,
    view: &View,
    session: &mut Session,
    look: &Look,
    body: &[u8],
) -> Result<Option<Problem>> {
    let invalid = || {
        problem(
            422,
            "invalid_patch",
            "Supply only a name string or an enabled boolean, or a place with activeOff.",
        )
    };
    let value = effects::parse_json(body).map_err(|_| invalid())?;
    let fields = value.as_object().ok_or_else(invalid)?;
    if let Some(place) = fields.get("place") {
        let to_card = match place.as_str() {
            Some("card") => true,
            Some("camera") => false,
            _ => return Err(invalid()),
        };
        let active_off = match fields.get("activeOff") {
            None => false,
            Some(Value::Bool(value)) => *value,
            Some(_) => return Err(invalid()),
        };
        if fields.len() > 1 + usize::from(fields.contains_key("activeOff")) {
            return Err(invalid());
        }
        if view.duplicates.contains(&look.file.id) {
            return Err(ambiguous());
        }
        let moved = move_look(root, backend, view, look, to_card, active_off)?;
        let edits = session.edits.entry(look.file.id.clone()).or_default();
        if to_card {
            edits.remove(&wire::LookEdit::Place);
        } else {
            edits.insert(wire::LookEdit::Place);
        }
        return Ok(moved);
    }
    if fields.is_empty()
        || fields.iter().any(|(key, value)| match key.as_str() {
            "name" => !value.is_string(),
            "enabled" => !value.is_boolean(),
            _ => true,
        })
    {
        return Err(invalid());
    }
    if view.duplicates.contains(&look.file.id) {
        return Err(ambiguous());
    }
    let name = fields.get("name").and_then(Value::as_str);
    let enabled = fields.get("enabled").and_then(Value::as_bool);
    // Check each field before the first change.
    if let Some(name) = name {
        let renamable = match look.source {
            "custom" | "native" => true,
            "official" => session.covers.official.renamable(&look.file.id),
            _ => false,
        };
        if !renamable {
            return Err(official_read_only());
        }
        if !(1..=effects::NAME_MAX).contains(&name.len())
            || !name.bytes().all(|b| (32..=126).contains(&b))
        {
            return Err(Problem::new(
                422,
                "invalid_name",
                format!("Use 1 to {} printable ASCII characters.", effects::NAME_MAX),
            ));
        }
    }
    if let Some(enabled) = enabled {
        let Some(key) = look.key else {
            return Err(restart_required());
        };
        let on = view.looks.iter().filter(|other| other.enabled).count();
        if enabled && !look.enabled && on >= MAX_LOADED {
            return Err(capacity());
        }
        if enabled != look.enabled {
            camera_looks::turn(backend, key, !enabled)?;
        }
    }
    if let Some(name) = name.filter(|name| *name != look.file.name) {
        session
            .edits
            .entry(look.file.id.clone())
            .or_default()
            .insert(wire::LookEdit::Name);
        let (mut effect, icon) = decode(&look.file.id, &look.bytes(root)?)?;
        // The default cover names the Look, so it follows the new name when
        // that fits. A Look without a known cover has it when its icon is it.
        let old = Cover::for_name(&look.file.name);
        let redrawn = look
            .cover
            .as_ref()
            .map_or_else(|| old.matches(&icon), |cover| *cover == old)
            .then(|| Cover::for_name(name))
            .and_then(|cover| Some((cover.pixels().ok()?, cover)));
        effect.name = name.to_owned();
        let bytes = effect
            .encode(redrawn.as_ref().map_or(&icon, |(pixels, _)| pixels))
            .map_err(|_| problem(422, "invalid_grf", "The renamed Look cannot be encoded."))?;
        let id = &look.file.id;
        if look.source == "official" {
            // The import of an official Look records its name, so the
            // library must take it before the file changes.
            let cover = redrawn
                .as_ref()
                .map(|(_, cover)| cover)
                .or(look.cover.as_ref());
            let staged = session
                .covers
                .stage_official(root, id, &bytes, cover)
                .map_err(|()| library_unavailable())?;
            write_file(root, look.stored, &look.file.file, &bytes)?;
            if let Some(staged) = staged {
                session.covers.commit(root, staged);
            }
        } else if look.source == "native" {
            // The import of a native look takes the renamed file with the
            // cover that it has. An unusable library keeps the old import.
            let staged = session
                .covers
                .stage_native(root, id, &bytes, look.cover.as_ref())
                .unwrap_or(None);
            write_file(root, look.stored, &look.file.file, &bytes)?;
            if let Some(staged) = staged {
                session.covers.commit(root, staged);
            }
        } else {
            write_file(root, look.stored, &look.file.file, &bytes)?;
            // A failed write keeps the old import, whose file is older but
            // whose cover still shows only while it draws the icon.
            session
                .covers
                .renamed(root, id, bytes, redrawn.map(|(_, cover)| cover));
        }
    }
    Ok(None)
}

/// Move the file of a Look to the card only, or back to the camera. The
/// camera hides a Look that leaves it at once, as a removed one; it loads a
/// Look that comes back at its next start. A recipe that uses a Look on the
/// card only cannot apply until the Look is on the camera again. Returns
/// Some problem after the file moved.
fn move_look(
    root: &Path,
    backend: &mut dyn RuntimeBackend,
    view: &View,
    look: &Look,
    to_card: bool,
    active_off: bool,
) -> Result<Option<Problem>> {
    if to_card == look.stored {
        return Ok(None);
    }
    let bytes = look.bytes(root)?;
    let (name, off) = if to_card {
        if look.selected && !active_off {
            return Err(active_look());
        }
        if view.stored_unavailable {
            return Err(looks_unavailable());
        }
        if view.stored.len() >= looks::MAX_STORED {
            return Err(capacity());
        }
        let name = stored_name(root, &look.file.id)?;
        // Off first: the camera menu hides the Look at once.
        let off = look.key.filter(|_| look.enabled);
        if let Some(key) = off {
            camera_looks::turn(backend, key, true)?;
        }
        (name, off)
    } else {
        if !view.camera_room() {
            return Err(capacity());
        }
        let code = looks::base_code(&look.file.id);
        if view
            .looks
            .iter()
            .any(|other| looks::base_code(&other.file.id) == code)
        {
            return Err(problem(
                409,
                "effect_code_collision",
                "Choose another Look ID because its runtime code is already in use.",
            ));
        }
        (camera_name(root, view, &look.file.id)?, None)
    };
    // The new file is written and checked, then the old one goes. When one
    // of them fails, the new file goes (no other file had its name), and the
    // Look stays where it was, On again when it was On.
    let moved = write_file(root, to_card, &name, &bytes)
        .and_then(|()| remove_file(root, look.stored, &look.file.file));
    if let Err(problem) = moved {
        let _ = remove_file(root, to_card, &name);
        if let Some(key) = off {
            let _ = camera_looks::turn(backend, key, false);
        }
        return Err(problem);
    }
    Ok(if to_card {
        None
    } else {
        turn_back_on(backend, view, &look.file.id).err()
    })
}

fn delete(
    root: &Path,
    backend: &mut dyn RuntimeBackend,
    session: &mut Session,
    look: &Look,
    active_off: bool,
) -> Result<()> {
    if look.used_by != 0 {
        return Err(problem(
            409,
            "dependency_in_use",
            "Recipes or mode-dial settings use this Look.",
        ));
    }
    if look.selected && !active_off {
        return Err(active_look());
    }
    // Off first: the camera menu hides the Look at once, and a camera that
    // is busy keeps the file.
    if let Some(key) = look.key.filter(|_| look.enabled) {
        camera_looks::turn(backend, key, true)?;
    }
    remove_file(root, look.stored, &look.file.file)?;
    session.covers.removed(root, &look.file.id);
    Ok(())
}

/// The camera still lists a Look that was removed or moved to the card in
/// this start, Off. The Look that comes back to the camera is On, as each
/// new Look is.
fn turn_back_on(backend: &mut dyn RuntimeBackend, view: &View, id: &str) -> Result<()> {
    let key = look_key(id);
    let on = view.looks.iter().filter(|look| look.enabled).count();
    if view.off.contains(&key) && on < MAX_LOADED {
        camera_looks::turn(backend, key, false)?;
    }
    Ok(())
}

fn order(backend: &mut dyn RuntimeBackend, view: &View, body: &[u8]) -> Result<()> {
    let invalid = |detail: &'static str| problem(422, "invalid_order", detail);
    let value = effects::parse_json(body).map_err(|_| invalid("The order JSON is invalid."))?;
    let object = value
        .as_object()
        .filter(|object| object.len() == 1 && object.contains_key("ids"))
        .ok_or_else(|| invalid("Supply only the complete ids array."))?;
    let ids: Vec<String> = serde_json::from_value(object["ids"].clone())
        .map_err(|_| invalid("The order must contain string IDs."))?;
    let mut sorted: Vec<&str> = ids.iter().map(String::as_str).collect();
    sorted.sort_unstable();
    let mut current = view.ids();
    current.sort_unstable();
    if sorted != current {
        return Err(invalid("List each Look ID exactly once."));
    }
    // A Look that waits for the next start has no position in the camera yet.
    let keys: Vec<u32> = ids
        .iter()
        .filter_map(|id| view.get(id).ok().and_then(|look| look.key))
        .collect();
    camera_looks::order(backend, &keys)
}

fn active_look() -> Problem {
    problem(
        409,
        "active_look",
        "Removing the selected Look requires active=off.",
    )
}
fn cover_not_saved() -> Problem {
    problem(
        507,
        "cover_not_saved",
        "The Look was written, but its cover could not be saved.",
    )
}
fn card_write_failed() -> Problem {
    problem(
        507,
        "card_write_failed",
        "The Look could not be written to the card.",
    )
}
fn coordinator() -> Problem {
    problem(
        503,
        "coordinator_unavailable",
        "The coordinator is unavailable.",
    )
}
fn official_read_only() -> Problem {
    problem(
        409,
        "official_look_read_only",
        "An official Look keeps its color content. A new icon needs its cover in X-GRKit-Cover.",
    )
}
fn library_unavailable() -> Problem {
    problem(
        409,
        "cover_library_unavailable",
        "The import library of the card cannot be read or cannot take the cover.",
    )
}
fn restart_required() -> Problem {
    problem(
        409,
        "restart_required",
        "The camera lists this Look after it starts again.",
    )
}
fn capacity() -> Problem {
    problem(
        409,
        "look_capacity",
        "The camera cannot list or load another Look.",
    )
}
fn looks_unavailable() -> Problem {
    problem(
        503,
        "library_unavailable",
        "The Looks of the card cannot be read.",
    )
}
fn ambiguous() -> Problem {
    problem(
        409,
        "ambiguous_look",
        "Two files of the card have this Look ID or its file name.",
    )
}
fn invalid_id() -> Problem {
    problem(422, "invalid_id", "The Look ID is invalid.")
}

mod covers;
#[cfg(test)]
mod tests;
