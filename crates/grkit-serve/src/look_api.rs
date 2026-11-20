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
//! Each request answers when it ends: the service keeps no record of a Look
//! request. On, off, and the order are absolute values, and a file write
//! replaces the file, so a request whose answer did not arrive can be sent
//! again. A change names the start of the camera that the phone read
//! (`X-GRKit-Boot-Id`): the Looks that wait for a start differ after it.
//!
//! Each Look carries the cover that draws its icon, when the service knows
//! one (`covers`). A content write can bring a cover in `X-GRKit-Cover`.
//! The covers of the built-in Looks (official Looks and native looks)
//! without an individual cover follow one scheme (`cover::Scheme`): one
//! request rewrites their files for another scheme, and one puts the
//! individual covers of a group back to it.
use crate::{
    api::Reply,
    changes::{Changed, Changes},
    looks::{self, LookFile, MAX_LISTED},
    protocol::Problem,
    runtime_backend::{look_key, RuntimeBackend, RuntimeLooks, UseOrigin},
    Request,
};
use covers::{built_in, Covers, Official};
use grkit_core::{
    cover::{Cover, Scheme},
    effects, imports, recipes,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::Path,
    sync::{Mutex, MutexGuard, TryLockError},
};

type Result<T> = std::result::Result<T, Problem>;

/// The camera loads at most this many Looks that are On.
pub const MAX_LOADED: usize = 40;
/// The longest `X-GRKit-Cover` value. A cover is less than 512 bytes of JSON.
const MAX_COVER: usize = 1024;

pub struct RequestContext<'a> {
    pub boot_id: &'a str,
    pub card_id: &'a str,
    pub software_pending: bool,
    /// The mounted card.
    pub root: &'a Path,
    /// The keys that sign the release whose official Looks the card names.
    pub release_keys: &'a [[u8; 32]],
    /// The recipe family of the camera, when its model is known.
    pub family: Option<grkit_core::recipes::Family>,
}

/// What the service keeps about one card while the camera stays on.
#[derive(Default)]
struct Session {
    card_id: String,
    /// The content hash of each Look when the service first read the card:
    /// the content that the running camera loaded.
    started: BTreeMap<String, String>,
    /// The official Looks of the installed release and the covers.
    covers: Covers,
    /// The runtime answered a Looks read.
    verified: bool,
}

pub struct LookApi {
    backend: Mutex<Box<dyn RuntimeBackend>>,
    session: Mutex<Option<Session>>,
}

/// The uses of a Look that its info names; `usedBy` counts them all.
const USES_LISTED: usize = 8;

/// A recipe (its name, or none for the automatic one, and its number) or a
/// user mode of the mode dial.
fn use_json(origin: &UseOrigin) -> Value {
    match origin {
        UseOrigin::Recipe { slot, number, name } => {
            json!({"kind":"recipe","slot":slot,"number":number,"name":name})
        }
        UseOrigin::Dial { mode } => json!({"kind":"dial","mode":mode}),
        UseOrigin::Unknown => json!({"kind":"unknown"}),
    }
}

/// One Look of the list.
struct Look {
    file: LookFile,
    source: &'static str,
    enabled: bool,
    loaded: bool,
    selected: bool,
    /// The recipes and mode-dial sets that name the Look.
    used_by: usize,
    /// Where the first `USES_LISTED` of them are.
    uses: Vec<UseOrigin>,
    /// "add" or "update" while the file waits for the next start.
    pending: Option<&'static str>,
    /// The key in the running camera, when the camera lists the Look.
    key: Option<u32>,
    /// The cover that draws the icon of the file, when the service knows it.
    cover: Option<Cover>,
    /// For a built-in Look, the cover that the scheme of the card gives it.
    default_cover: Option<Cover>,
}
impl Look {
    fn configuration(&self) -> Value {
        json!({"id":self.file.id,"name":self.file.name,"contentHash":self.file.content_hash,"enabled":self.enabled,
            "cover":self.cover.as_ref().map(Cover::to_json),
            "defaultCover":self.default_cover.as_ref().map(Cover::to_json)})
    }
    fn info(&self) -> Value {
        json!({"id":self.file.id,"source":self.source,"loaded":self.loaded,"selected":self.selected,
            "usedBy":self.used_by,"uses":self.uses.iter().map(use_json).collect::<Vec<_>>(),
            "pending":self.pending})
    }
}

/// The Looks in the order of the camera menu, then the Looks that wait for
/// the next start.
struct View {
    looks: Vec<Look>,
    /// Files that share an ID with an earlier file: the camera lists one.
    duplicates: BTreeSet<String>,
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
        self.looks
            .iter()
            .find(|look| look.file.id == id)
            .ok_or_else(|| problem(404, "look_not_found", "The card has no Look with this ID."))
    }
    fn ids(&self) -> Vec<&str> {
        self.looks
            .iter()
            .map(|look| look.file.id.as_str())
            .collect()
    }
    fn value(&self) -> Value {
        let looks: Vec<Value> = self
            .looks
            .iter()
            .enumerate()
            .map(|(position, look)| {
                let mut value = look.configuration();
                value["position"] = json!(position);
                value["runtime"] = look.info();
                value
            })
            .collect();
        let on = self.looks.iter().filter(|look| look.enabled).count();
        json!({"looks":looks,"officialCovers":self.scheme.id(),
            "counts":{"listed":self.looks.len(),"loaded":on},
            "limits":{"listedLooks":MAX_LISTED,"loadedLooks":MAX_LOADED}})
    }
}

impl LookApi {
    pub fn new(backend: Box<dyn RuntimeBackend>) -> Self {
        Self {
            backend: Mutex::new(backend),
            session: Mutex::new(None),
        }
    }

    /// The count of setting changes of the running camera, when the service
    /// has a session with it and no request holds it. Sends no command.
    pub fn state_changes(&self) -> Option<u32> {
        try_lock(&self.backend).ok()?.state_changes()
    }

    /// Mark the service as running or stopped for the camera. True when the
    /// mark is in place.
    pub fn announce(&self, running: bool) -> bool {
        lock(&self.backend).announce(running).is_ok()
    }

    /// The Looks state for /props, without a runtime command.
    pub fn runtime_properties(&self, card_id: &str) -> Value {
        let verified = lock(&self.session)
            .as_ref()
            .is_some_and(|session| session.card_id == card_id && session.verified);
        json!({"state":if verified {"ready"} else {"unverified"},"busyReason":null,
            "capabilities":{"read":true,"write":true,"appliesAtRestart":["content","name","delete","covers"]}})
    }

    /// None means this path belongs to another router.
    pub fn handle(
        &self,
        request: &Request,
        body: &[u8],
        context: &RequestContext<'_>,
        changes: &Changes,
    ) -> Option<Reply> {
        let route = Route::parse(&request.path)?;
        let result = match request.method.as_str() {
            "GET" => self.read(&route, context),
            method if route.accepts(method) => self.mutate(&route, request, body, context, changes),
            _ => Err(problem(
                405,
                "method_not_allowed",
                "This method is not supported for the resource.",
            )),
        };
        Some(result.unwrap_or_else(Reply::problem))
    }

    /// The files of the card joined with the choices of the camera.
    fn view(&self, backend: &mut dyn RuntimeBackend, context: &RequestContext<'_>) -> Result<View> {
        if context.card_id == "unavailable" {
            return Err(problem(
                503,
                "card_identity_unavailable",
                "The card identity is not available yet.",
            ));
        }
        let runtime = backend.looks()?;
        let scan = looks::scan(context.root).map_err(|_| {
            problem(
                503,
                "library_unavailable",
                "The Looks of the card cannot be read.",
            )
        })?;
        let mut guard = lock(&self.session);
        if guard
            .as_ref()
            .is_none_or(|session| session.card_id != context.card_id)
        {
            *guard = Some(Session {
                card_id: context.card_id.to_owned(),
                started: scan
                    .files
                    .iter()
                    .map(|file| (file.id.clone(), file.content_hash.clone()))
                    .collect(),
                covers: Covers::new(
                    crate::trust::installed_release(context.root, context.release_keys)
                        .map(Official::of)
                        .unwrap_or_default(),
                ),
                verified: false,
            });
        }
        let session = guard.as_mut().expect("session");
        session.verified = true;
        Ok(join(
            context.root,
            scan.files,
            &runtime,
            session,
            looks::left_out(context.root),
        ))
    }

    fn read(&self, route: &Route, context: &RequestContext<'_>) -> Result<Reply> {
        let mut backend = try_lock(&self.backend)?;
        let view = self.view(backend.as_mut(), context)?;
        Ok(match route {
            Route::Collection => {
                let mut body = view.value();
                body["bootId"] = json!(context.boot_id);
                body["cardId"] = json!(context.card_id);
                Reply::json(200, body)
            }
            Route::Configuration(id) => Reply::json(200, view.get(id)?.configuration()),
            Route::Info(id) => Reply::json(200, view.get(id)?.info()),
            Route::Content(id) => {
                let look = view.get(id)?;
                Reply::binary(200, look.file.bytes.clone(), "application/octet-stream")
                    .header("X-GRKit-Content-SHA256", look.file.content_hash.clone())
            }
            Route::Order => Reply::json(200, json!({"ids": view.ids()})),
            Route::Scheme => Reply::json(200, json!({"scheme": view.scheme.id()})),
            Route::Reset => {
                return Err(problem(
                    405,
                    "method_not_allowed",
                    "This method is not supported for the resource.",
                ))
            }
        })
    }

    fn mutate(
        &self,
        route: &Route,
        request: &Request,
        body: &[u8],
        context: &RequestContext<'_>,
        changes: &Changes,
    ) -> Result<Reply> {
        validate_body(route, request, body)?;
        if header(request, "x-grkit-boot-id")? != context.boot_id {
            return Err(problem(
                409,
                "context_changed",
                "The camera started again. Read the Looks before changing them.",
            ));
        }
        if context.software_pending {
            return Err(problem(
                409,
                "software_pending",
                "Finish or cancel the pending software operation before changing Looks.",
            ));
        }
        if request.query.is_some()
            && !(request.method == "DELETE" && request.query.as_deref() == Some("active=off"))
        {
            return Err(problem(
                422,
                "invalid_query",
                "This mutation query is not supported.",
            ));
        }
        let mut backend = try_lock(&self.backend)?;
        let view = self.view(backend.as_mut(), context)?;
        // A problem after the file changed: the answer reports it, and the
        // phone still hears of the change.
        let mut late = None;
        let mut rewritten = Vec::new();
        let (status, id) = {
            let mut guard = lock(&self.session);
            let session = guard.as_mut().ok_or_else(coordinator)?;
            match route {
                Route::Content(id) => {
                    let created = view.get(id).is_err();
                    late = put_content(context.root, &view, session, id, request, body)?;
                    // The camera still lists a Look that was removed in this
                    // start, Off. The Look that comes back is On, as each
                    // new Look is.
                    let key = look_key(id);
                    let on = view.looks.iter().filter(|look| look.enabled).count();
                    if created && view.off.contains(&key) && on < MAX_LOADED {
                        if let Err(problem) = backend.turn(key, false) {
                            late.get_or_insert(problem);
                        }
                    }
                    (if created { 201 } else { 200 }, Some(id))
                }
                Route::Configuration(id) => {
                    let look = view.get(id)?;
                    if request.method == "PATCH" {
                        patch(context.root, backend.as_mut(), &view, session, look, body)?;
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
                Route::Order => {
                    order(backend.as_mut(), &view, body)?;
                    (200, None)
                }
                Route::Scheme | Route::Reset => {
                    let (scheme, reset) = covers_request(route, body, view.scheme)?;
                    (rewritten, late) =
                        rewrite_covers(context.root, &view, session, scheme, reset)?;
                    session.covers.chosen = Some(scheme);
                    (200, None)
                }
                Route::Collection | Route::Info(_) => {
                    return Err(problem(
                        405,
                        "method_not_allowed",
                        "This resource is not mutable.",
                    ))
                }
            }
        };
        // The result is the state after the change, read again from the card
        // and the camera.
        let view = self.view(backend.as_mut(), context)?;
        changes.note(Changed::Looks);
        if let Some(problem) = late {
            return Err(problem);
        }
        let result = match (route, id) {
            (Route::Scheme | Route::Reset, _) => {
                json!({"officialCovers":view.scheme.id(),"changed":rewritten})
            }
            (_, Some(id)) => view
                .get(id)
                .map_or_else(|_| json!({"id":id,"removed":true}), Look::configuration),
            (_, None) => json!({"ids": view.ids()}),
        };
        Ok(Reply::json(status, result))
    }
}

/// Join the files with the runtime list. A file that the camera does not
/// list waits for the next start. A Look that the camera lists without a
/// file was removed: it is not a Look of the card any more.
fn join(
    root: &Path,
    files: Vec<LookFile>,
    runtime: &RuntimeLooks,
    session: &mut Session,
    left_out: BTreeSet<String>,
) -> View {
    let order = runtime.menu_order();
    let mut duplicates = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut looks: Vec<(usize, Look)> = Vec::new();
    for file in files {
        if left_out.contains(&file.file.to_ascii_uppercase()) {
            continue;
        }
        if !seen.insert(file.id.clone()) {
            duplicates.insert(file.id.clone());
            continue;
        }
        let key = look_key(&file.id);
        let listed = runtime.looks.iter().find(|look| look.key == key);
        let code = u32::from(recipes::effect_code(&file.id));
        let source = match session.covers.official.source(&file) {
            "custom" if grkit_core::cover::native_cover(&file.id).is_some() => "native",
            source => source,
        };
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
                source,
                enabled: listed.is_none_or(|look| look.enabled),
                loaded: listed.is_some_and(|look| look.loaded),
                selected: runtime.selected == Some(key),
                used_by: runtime.uses.iter().filter(|used| used.code == code).count(),
                uses: runtime
                    .uses
                    .iter()
                    .filter(|used| used.code == code)
                    .take(USES_LISTED)
                    .map(|used| used.origin.clone())
                    .collect(),
                pending: match listed {
                    None => Some("add"),
                    Some(_) if changed => Some("update"),
                    Some(_) => None,
                },
                key: listed.map(|look| look.key),
                cover: None,
                default_cover: None,
                file,
            },
        ));
    }
    looks.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.file.id.cmp(&b.1.file.id)));
    let mut looks: Vec<Look> = looks.into_iter().map(|(_, look)| look).collect();
    session.covers.resolve(root, &mut looks);
    let scheme = session.covers.scheme();
    for look in looks.iter_mut().filter(|look| built_in(look.source)) {
        look.default_cover = session
            .covers
            .default_cover(&look.file.id, look.source, scheme);
    }
    View {
        scheme,
        looks,
        duplicates,
        left_out,
        off: runtime
            .looks
            .iter()
            .filter(|look| !look.enabled)
            .map(|look| look.key)
            .collect(),
    }
}

/// Write an effect file and read it back. A file that the write did not
/// complete fails the check of the installer at the next start, so the
/// camera skips it.
fn write_file(root: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    if write_checked(&looks::library(root).join(name), bytes) {
        return Ok(());
    }
    Err(problem(
        507,
        "card_write_failed",
        "The Look could not be written to the card.",
    ))
}

/// Write the whole file in place, as the card cannot replace a file, and
/// read it back. True when the card holds `bytes`.
fn write_checked(path: &Path, bytes: &[u8]) -> bool {
    let write = || -> std::io::Result<Vec<u8>> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        let mut stored = Vec::with_capacity(bytes.len());
        fs::File::open(path)?
            .take(bytes.len() as u64 + 1)
            .read_to_end(&mut stored)?;
        Ok(stored)
    };
    write().is_ok_and(|stored| stored == bytes)
}

/// Remove an effect file. On the camera, the lookup of the next new name in
/// a folder can fail after a delete in that folder, and Linux keeps that
/// answer. The scan before this call looked up each existing file; one
/// lookup of a name that no file has takes the failure.
fn remove_file(root: &Path, name: &str) -> Result<()> {
    let directory = looks::library(root);
    fs::remove_file(directory.join(name)).map_err(|_| {
        problem(
            507,
            "card_write_failed",
            "The Look could not be removed from the card.",
        )
    })?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |time| time.subsec_nanos() ^ time.as_secs() as u32);
    let _ = fs::metadata(directory.join(format!("{stamp:08X}.TMP")));
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

/// A new Look, new content for a Look that the phone made, or a new name
/// or cover for an official Look. A cover goes to the import library after
/// the file. Ok(Some) is a problem after the file was written.
fn put_content(
    root: &Path,
    view: &View,
    session: &mut Session,
    id: &str,
    request: &Request,
    bytes: &[u8],
) -> Result<Option<Problem>> {
    let (_, icon) = decode(id, bytes)?;
    let cover = cover_header(request, &icon)?;
    if view.duplicates.contains(id) {
        return Err(ambiguous());
    }
    let official = &session.covers.official;
    // An official Look changes only its name and cover: its body stays the
    // official one.
    let verified = official.verified(id, bytes, &icon, cover.is_some());
    let name = match view.get(id) {
        Ok(look) if look.source == "official" && verified => look.file.file.clone(),
        Ok(look) if !matches!(look.source, "custom" | "native") => return Err(official_read_only()),
        Ok(look) => look.file.file.clone(),
        Err(_) => {
            // The content of an official ID comes only from its release.
            if official.looks.contains_key(id) && !verified {
                return Err(official_read_only());
            }
            let code = looks::base_code(id);
            // The native page of GR IV, on each camera, so that an ID is
            // usable on each camera.
            if code >> 8 == 0x81
                || view
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
            if view.looks.len() >= MAX_LISTED {
                return Err(capacity());
            }
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
            name
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
    write_file(root, &name, bytes)?;
    if let Some(staged) = staged {
        if !session.covers.commit(root, staged) {
            // The Look then shows only a cover that still draws its icon.
            return Ok(Some(problem(
                507,
                "cover_not_saved",
                "The Look was written, but its cover could not be saved.",
            )));
        }
    }
    Ok(None)
}

/// The scheme and the group to reset (true for B&W) of a covers request:
/// `{"scheme": id}` for `Route::Scheme`, `{"group": "color" | "mono"}` for
/// `Route::Reset`, which keeps the scheme of the card.
fn covers_request(route: &Route, body: &[u8], current: Scheme) -> Result<(Scheme, Option<bool>)> {
    let (key, detail) = match route {
        Route::Scheme => (
            "scheme",
            "Supply only a scheme: gray, colorFilmBox, or filmBox.",
        ),
        _ => ("group", "Supply only a group: color or mono."),
    };
    let invalid = || problem(422, "invalid_covers", detail);
    let value = effects::parse_json(body).map_err(|_| invalid())?;
    let text = value
        .as_object()
        .filter(|object| object.len() == 1)
        .and_then(|object| object.get(key)?.as_str())
        .ok_or_else(invalid)?;
    match route {
        Route::Scheme => Ok((Scheme::parse(text).ok_or_else(invalid)?, None)),
        _ => match text {
            "color" => Ok((current, Some(false))),
            "mono" => Ok((current, Some(true))),
            _ => Err(invalid()),
        },
    }
}

/// Give each built-in Look the cover of `scheme`: each without an
/// individual cover, or with `reset`, each of that group (true for B&W) with
/// one. Names stay. Returns the IDs of the rewritten files, and a problem
/// after a file was written. Each file is staged and written as a cover save
/// does, so a failure leaves the Looks before it rewritten.
fn rewrite_covers(
    root: &Path,
    view: &View,
    session: &mut Session,
    scheme: Scheme,
    reset: Option<bool>,
) -> Result<(Vec<String>, Option<Problem>)> {
    let mut rewritten = Vec::new();
    let mut late = None;
    for look in &view.looks {
        let id = look.file.id.as_str();
        let (Some(base), Some((individual, monochrome))) = (
            session.covers.base(id, look.source),
            session.covers.kind(id),
        ) else {
            continue;
        };
        let chosen = match reset {
            None => !individual,
            Some(group) => individual && group == monochrome,
        };
        if !chosen || view.duplicates.contains(id) {
            continue;
        }
        let cover = scheme.cover(&base, monochrome);
        let Ok((effect, icon)) = effects::decode(&look.file.bytes) else {
            continue;
        };
        let Ok(pixels) = cover.pixels() else {
            continue;
        };
        if pixels == icon && !individual {
            continue;
        }
        let bytes = effect
            .encode(&pixels)
            .map_err(|_| problem(422, "invalid_grf", "The Look cannot be encoded."))?;
        let staged = match look.source {
            "official" => session
                .covers
                .stage_official(root, id, &bytes, Some(&cover)),
            _ => session.covers.stage_native(root, id, &bytes, Some(&cover)),
        }
        .map_err(|()| library_unavailable())?;
        write_file(root, &look.file.file, &bytes)?;
        rewritten.push(id.to_owned());
        if let Some(staged) = staged {
            if !session.covers.commit(root, staged) {
                late = Some(problem(
                    507,
                    "cover_not_saved",
                    "The Look was written, but its cover could not be saved.",
                ));
            }
        }
    }
    Ok((rewritten, late))
}

fn patch(
    root: &Path,
    backend: &mut dyn RuntimeBackend,
    view: &View,
    session: &mut Session,
    look: &Look,
    body: &[u8],
) -> Result<()> {
    let invalid = || {
        problem(
            422,
            "invalid_patch",
            "Supply only a name string or an enabled boolean.",
        )
    };
    let value = effects::parse_json(body).map_err(|_| invalid())?;
    let fields = value.as_object().ok_or_else(invalid)?;
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
            return Err(problem(
                422,
                "invalid_name",
                "Use 1 to 16 printable ASCII characters.",
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
            backend.turn(key, !enabled)?;
        }
    }
    if let Some(name) = name.filter(|name| *name != look.file.name) {
        let (mut effect, icon) = decode(&look.file.id, &look.file.bytes)?;
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
            write_file(root, &look.file.file, &bytes)?;
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
            write_file(root, &look.file.file, &bytes)?;
            if let Some(staged) = staged {
                session.covers.commit(root, staged);
            }
        } else {
            write_file(root, &look.file.file, &bytes)?;
            // A failed write keeps the old import, whose file is older but
            // whose cover still shows only while it draws the icon.
            session
                .covers
                .renamed(root, id, bytes, redrawn.map(|(_, cover)| cover));
        }
    }
    Ok(())
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
        return Err(problem(
            409,
            "active_look",
            "Removing the selected Look requires active=off.",
        ));
    }
    // Off first: the camera menu hides the Look at once, and a camera that
    // is busy keeps the file.
    if let Some(key) = look.key.filter(|_| look.enabled) {
        backend.turn(key, true)?;
    }
    remove_file(root, &look.file.file)?;
    session.covers.removed(root, &look.file.id);
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
    backend.order(&keys)
}

enum Route {
    Collection,
    Configuration(String),
    Info(String),
    Content(String),
    Order,
    /// The scheme of the built-in covers.
    Scheme,
    /// The individual covers of a group back to the scheme.
    Reset,
}
impl Route {
    fn parse(path: &str) -> Option<Self> {
        let parts: Vec<_> = path.split('/').collect();
        match parts.as_slice() {
            ["", "v1", "looks"] => Some(Self::Collection),
            ["", "v1", "looks", "covers", "scheme"] => Some(Self::Scheme),
            ["", "v1", "looks", "covers", "reset"] => Some(Self::Reset),
            ["", "v1", "looks", id] if effects::valid_id(id) => {
                Some(Self::Configuration((*id).into()))
            }
            ["", "v1", "looks", id, "info"] if effects::valid_id(id) => {
                Some(Self::Info((*id).into()))
            }
            ["", "v1", "looks", id, "content"] if effects::valid_id(id) => {
                Some(Self::Content((*id).into()))
            }
            ["", "v1", "params", "looks", "order"] => Some(Self::Order),
            _ => None,
        }
    }
    fn accepts(&self, method: &str) -> bool {
        matches!(
            (self, method),
            (Self::Content(_), "PUT")
                | (Self::Configuration(_), "PATCH" | "DELETE")
                | (Self::Order, "PUT")
                | (Self::Scheme, "PUT")
                | (Self::Reset, "POST")
        )
    }
}

fn validate_body(route: &Route, request: &Request, body: &[u8]) -> Result<()> {
    if request.length != body.len() {
        return Err(problem(
            400,
            "incomplete_body",
            "The complete request body is required.",
        ));
    }
    let expected_type = match route {
        Route::Content(_) => Some("application/octet-stream"),
        Route::Order | Route::Scheme | Route::Reset => Some("application/json"),
        Route::Configuration(_) if request.method == "PATCH" => Some("application/json"),
        _ => None,
    };
    if let Some(expected) = expected_type {
        if request
            .headers
            .get("content-type")
            .is_none_or(|value| value.split(';').next().unwrap_or("").trim() != expected)
        {
            return Err(problem(
                415,
                "unsupported_media_type",
                "The request content type is not supported.",
            ));
        }
    }
    match route {
        Route::Content(_) => {
            if !matches!(body.len(), effects::SIZE | effects::SIZE_WITH_LEVELS) {
                return Err(problem(
                    413,
                    "invalid_content_length",
                    "A GRF must contain 8192 or 8704 bytes.",
                ));
            }
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
        }
        Route::Order if body.len() > 4096 => {
            return Err(problem(
                413,
                "order_too_large",
                "The order body exceeds its size limit.",
            ));
        }
        Route::Scheme | Route::Reset if body.len() > 1024 => {
            return Err(problem(
                413,
                "covers_too_large",
                "The covers body exceeds its size limit.",
            ));
        }
        Route::Configuration(_) if request.method == "PATCH" && body.len() > 1024 => {
            return Err(problem(
                413,
                "patch_too_large",
                "The Look patch exceeds its size limit.",
            ));
        }
        _ if expected_type.is_none() && !body.is_empty() => {
            return Err(problem(
                400,
                "unexpected_body",
                "This mutation does not accept a body.",
            ));
        }
        _ => {}
    }
    Ok(())
}

fn header<'a>(request: &'a Request, name: &str) -> Result<&'a str> {
    request
        .headers
        .get(name)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            problem(
                400,
                "missing_header",
                "A required request header is missing.",
            )
        })
}
/// A request that panics ends alone (the service unwinds), and the requests
/// after it take the state that it left.
fn lock<T: ?Sized>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        mutex.clear_poison();
        poisoned.into_inner()
    })
}
fn try_lock<T: ?Sized>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>> {
    match mutex.try_lock() {
        Ok(guard) => Ok(guard),
        Err(TryLockError::WouldBlock) => Err(busy()),
        Err(TryLockError::Poisoned(poisoned)) => {
            mutex.clear_poison();
            Ok(poisoned.into_inner())
        }
    }
}
fn busy() -> Problem {
    problem(
        503,
        "runtime_busy",
        "The camera is answering another request.",
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
fn problem(status: u16, code: &'static str, detail: &'static str) -> Problem {
    Problem::new(status, code, detail)
}

mod covers;
mod recipe_api;
#[cfg(test)]
mod tests;
