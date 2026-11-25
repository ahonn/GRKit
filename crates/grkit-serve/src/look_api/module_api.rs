//! The module settings of the running camera (`runtime/control_modules.h`):
//! the catalog that the phone shows, the current values, and the changes
//! that MENU can make. The phone has no list of modules of its own. A module
//! list is fixed for a start of the camera, so the service keeps each
//! catalog and icon that it read until the service ends.
use super::{
    codec::{word, Codes, Page},
    header, lock, problem, try_lock, LookApi, RequestContext, Result,
};
use crate::{api::Reply, protocol::Problem, runtime_backend::RuntimeBackend, Request};
use grkit_core::{
    abi::*, capabilities, card, effects::ICON_SIZE, language::Language, packaging::sha256,
};
use serde_json::{json, Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    sync::Arc,
};

/// A body that the service answers again, and its entity tag.
struct Cached {
    tag: String,
    body: Vec<u8>,
}

/// What the service keeps about the modules of this start.
#[derive(Default)]
pub(super) struct ModuleCache {
    /// The answer of the runtime to the first module command.
    supported: Option<bool>,
    /// The answer of the runtime to the first `CONTROL_SETTINGS_READ`.
    pub(super) settings: Option<bool>,
    catalogs: BTreeMap<usize, Arc<Cached>>,
    icons: BTreeMap<String, Arc<Cached>>,
}

/// One module of `CONTROL_MODULES_READ`.
struct State {
    index: u32,
    id: String,
    word: u32,
    hidden: u32,
    values: Vec<(u32, u32)>,
    order: Vec<u8>,
}
struct States {
    modules: Vec<State>,
    revision: u32,
    running: bool,
    capabilities: u32,
}

/// What the card and the camera add to the state of the modules.
#[derive(Default)]
struct ModuleView {
    /// The modules of the installed package, when its list can be read. A
    /// module that it does not name is the person's own.
    package: Option<BTreeSet<String>>,
    monochrome: bool,
}

impl ModuleView {
    fn of(context: &RequestContext<'_>) -> Self {
        let package = fs::read(context.root.join(card::PACKAGE_LIST))
            .ok()
            .map(|list| {
                grkit_core::packaging::PackageList::parse(&list)
                    .modules()
                    .map(str::to_owned)
                    .collect()
            });
        Self {
            package,
            monochrome: context.family.is_some_and(|family| family.monochrome()),
        }
    }
}

const CODES: Codes = Codes {
    list: "The module list",
    changed: "The camera does not list this module. Read the modules again.",
    unsupported: "This camera cannot manage module settings.",
    invalid: Some(|| invalid("The camera does not offer this change.")),
};
fn malformed() -> Problem {
    CODES.malformed()
}
fn not_found() -> Problem {
    problem(
        404,
        "module_not_found",
        "The camera has no module with this ID.",
    )
}
fn invalid(detail: &'static str) -> Problem {
    problem(422, "invalid_request", detail)
}
fn checked(result: u32, bytes: Vec<u8>) -> Result<Vec<u8>> {
    CODES.checked((result, bytes))
}
fn key_text(key: u32) -> String {
    format!("{key:08x}")
}
fn key_value(text: &str) -> Option<u32> {
    (text.len() == 8)
        .then(|| u32::from_str_radix(text, 16).ok())
        .flatten()
}
fn bits(mask: u32) -> Vec<u32> {
    (0..32).filter(|bit| mask >> bit & 1 != 0).collect()
}

fn read_states(backend: &mut dyn RuntimeBackend) -> Result<States> {
    let mut modules = Vec::new();
    let last = CODES.read_pages(backend, CONTROL_MODULES_READ, |bytes| {
        let head = |i: usize| word(bytes, 4 * i).ok_or_else(malformed);
        let (count, at, k) = (head(0)?, head(3)?, head(4)?);
        if count as usize > MODULES_MAX {
            return Err(malformed());
        }
        let mut offset = CONTROL_MODULES_HEADER;
        for _ in 0..k {
            let entry = bytes
                .get(offset..offset + CONTROL_MODULE_ENTRY)
                .ok_or_else(malformed)?;
            let field = |at: usize| word(entry, at).ok_or_else(malformed);
            let value = field(0)?;
            let id: String = entry[CONTROL_MODULE_NAME_AT..][..CONTROL_MODULE_NAME_BYTES]
                .iter()
                .take_while(|byte| **byte != 0)
                .map(|byte| char::from(*byte))
                .collect();
            let hidden = field(CONTROL_MODULE_HIDDEN_AT)?;
            let sizes = field(CONTROL_MODULE_SIZES_AT)?;
            let (settings, ordered) = ((sizes & 0xff) as usize, (sizes >> 8) as usize);
            offset += CONTROL_MODULE_ENTRY;
            let mut values = Vec::with_capacity(settings);
            for s in 0..settings {
                values.push((
                    word(bytes, offset + 8 * s).ok_or_else(malformed)?,
                    word(bytes, offset + 8 * s + 4).ok_or_else(malformed)?,
                ));
            }
            offset += 8 * settings;
            let order = bytes
                .get(offset..offset + ordered)
                .ok_or_else(malformed)?
                .to_vec();
            offset += ordered.next_multiple_of(4);
            modules.push(State {
                index: value & 0xff,
                id,
                word: value,
                hidden,
                values,
                order,
            });
        }
        if offset != bytes.len() {
            return Err(malformed());
        }
        Ok(Page {
            count,
            at,
            entries: k,
        })
    })?;
    let head = |i: usize| word(&last, 4 * i).ok_or_else(malformed);
    Ok(States {
        modules,
        revision: head(1)?,
        running: head(2)? != 0,
        capabilities: head(5)?,
    })
}

fn state_json(state: &State, view: &ModuleView) -> Value {
    let status = match state.word >> CONTROL_MODULE_STATUS_SHIFT & CONTROL_MODULE_STATUS_MASK {
        CONTROL_MODULE_RUNNING => "running",
        CONTROL_MODULE_STOPPED => "stopped",
        CONTROL_MODULE_SAFE => "safe",
        _ => "failed",
    };
    let place = match state.word >> CONTROL_MODULE_PLACE_SHIFT & CONTROL_MODULE_PLACE_MASK {
        CONTROL_MODULE_TOOLBAR => "toolbar",
        CONTROL_MODULE_HIDDEN => "hidden",
        _ => "root",
    };
    let values: Map<String, Value> = state
        .values
        .iter()
        .map(|(key, value)| (key_text(*key), json!(value)))
        .collect();
    let custom = view
        .package
        .as_ref()
        .is_some_and(|package| !package.contains(&state.id));
    json!({"id":state.id,"enabled":state.word & CONTROL_MODULE_ENABLED != 0,"status":status,"place":place,
        "custom":custom,
        "orderable":state.word & CONTROL_MODULE_ORDERABLE != 0,
        "placeable":state.word & CONTROL_MODULE_PLACEABLE != 0,"values":values,
        "hiddenValues":bits(state.hidden),"valueOrder":state.order})
}
fn states_json(states: &States, view: &ModuleView) -> Value {
    let modules: Vec<Value> = states
        .modules
        .iter()
        .map(|state| state_json(state, view))
        .collect();
    let runs = |key: &str| states.capabilities & capabilities::bit_of(key) != 0;
    json!({"revision":states.revision,"grkitEnabled":states.running,"monochrome":view.monochrome,
        "features":{"looks":runs("TONE"),"recipes":runs("RECIPE")},"toolbarMax":capabilities::ADJ_ENTRIES,
        "modules":modules})
}

/// The records of a module in one language, across as many commands as they
/// take.
fn module_catalog(
    backend: &mut dyn RuntimeBackend,
    state: &State,
    language: usize,
) -> Result<Value> {
    let mut cursor = 0u32;
    let mut module = None;
    let mut group: Option<String> = None;
    let mut settings: Vec<Value> = Vec::new();
    loop {
        let payload = [state.index, language as u32, cursor]
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect::<Vec<u8>>();
        let (result, bytes) = backend.exchange(CONTROL_MODULE_TEXTS, &payload)?;
        let bytes = checked(result, bytes)?;
        let next = word(&bytes, 0).ok_or_else(malformed)?;
        let count = word(&bytes, 4).ok_or_else(malformed)?;
        let mut at = CONTROL_MODULE_TEXTS_HEADER;
        for _ in 0..count {
            let head = word(&bytes, at).ok_or_else(malformed)?;
            let (a, b) = (
                word(&bytes, at + 4).ok_or_else(malformed)?,
                word(&bytes, at + 8).ok_or_else(malformed)?,
            );
            let units = (head >> 8) as usize;
            let raw = bytes
                .get(at + 12..at + 12 + 2 * units)
                .ok_or_else(malformed)?;
            let text = String::from_utf16_lossy(
                &raw.chunks_exact(2)
                    .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
                    .collect::<Vec<u16>>(),
            );
            at += 12 + (2 * units).next_multiple_of(4);
            match head & 0xff {
                CONTROL_MODULE_RECORD_MODULE => module = Some((a, b, text)),
                CONTROL_MODULE_RECORD_GROUP => group = Some(text),
                CONTROL_MODULE_RECORD_SETTING => {
                    settings.push(json!({"key":key_text(a),"group":group,"label":text,"values":[]}))
                }
                CONTROL_MODULE_RECORD_VALUE => settings
                    .last_mut()
                    .and_then(|setting| setting["values"].as_array_mut())
                    .ok_or_else(malformed)?
                    .push(json!({"value":a,"label":text})),
                _ => return Err(malformed()),
            }
        }
        if at != bytes.len() {
            return Err(malformed());
        }
        if next == CONTROL_MODULE_TEXTS_END {
            break;
        }
        if next <= cursor {
            return Err(malformed());
        }
        cursor = next;
    }
    let (flags, key, title) = module.ok_or_else(malformed)?;
    let managed = match flags & 0xff {
        1 => json!("looks"),
        2 => json!("recipes"),
        _ => Value::Null,
    };
    let options = (flags & CONTROL_MODULE_HAS_OPTIONS != 0)
        .then(|| json!({"setting":key_text(key),"ordered":flags & CONTROL_MODULE_ORDERED != 0}));
    Ok(json!({"id":state.id,"title":title,"managed":managed,"settings":settings,"options":options}))
}

/// A 40 x 40 RGBA PNG with one stored deflate block. An icon is small, and
/// the service runs in the RAM of the camera, so it links no PNG encoder.
fn icon_png(pixels: &[u8]) -> Vec<u8> {
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        let mut crc = !0u32;
        for byte in kind.iter().chain(data) {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = crc >> 1 ^ (0xedb8_8320 & (crc & 1).wrapping_neg());
            }
        }
        out.extend((data.len() as u32).to_be_bytes());
        out.extend(kind);
        out.extend(data);
        out.extend((!crc).to_be_bytes());
    }
    let raw: Vec<u8> = pixels
        .chunks_exact(160)
        .flat_map(|row| std::iter::once(0).chain(row.iter().copied()))
        .collect();
    let (mut a, mut b) = (1u32, 0u32);
    for byte in &raw {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    let size = raw.len() as u16;
    let mut data = vec![0x78, 0x01, 1];
    data.extend(size.to_le_bytes());
    data.extend((!size).to_le_bytes());
    data.extend(&raw);
    data.extend((b << 16 | a).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut head = Vec::with_capacity(13);
    head.extend(40u32.to_be_bytes());
    head.extend(40u32.to_be_bytes());
    head.extend([8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &head);
    chunk(&mut out, b"IDAT", &data);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// The value of an `If-None-Match` header names this entity tag.
fn matches(request: &Request, tag: &str) -> bool {
    request
        .headers
        .get("if-none-match")
        .is_some_and(|value| value.split(',').any(|item| item.trim() == tag))
}
fn cached_reply(request: &Request, cached: &Cached, content_type: &str) -> Reply {
    if matches(request, &cached.tag) {
        return Reply::binary(304, Vec::new(), content_type).header("ETag", cached.tag.clone());
    }
    Reply::binary(200, cached.body.clone(), content_type).header("ETag", cached.tag.clone())
}

fn numbers(value: &Value) -> Result<Vec<u32>> {
    value
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_u64().and_then(|item| u32::try_from(item).ok()))
                .collect()
        })
        .ok_or_else(|| invalid("Use arrays of whole numbers."))
}

/// The payload of `CONTROL_MODULE_SET` for the body of a PATCH.
fn patch_payload(index: u32, value: &Value) -> Result<Vec<u8>> {
    let object = value
        .as_object()
        .filter(|object| !object.is_empty())
        .ok_or_else(|| invalid("Change enabled, values, hiddenValues, or valueOrder."))?;
    if object.keys().any(|key| {
        !matches!(
            key.as_str(),
            "enabled" | "values" | "hiddenValues" | "valueOrder"
        )
    }) {
        return Err(invalid(
            "Change enabled, values, hiddenValues, or valueOrder.",
        ));
    }
    let (mut fields, mut off, mut mask) = (0, 0, 0u32);
    if let Some(enabled) = object.get("enabled") {
        let enabled = enabled
            .as_bool()
            .ok_or_else(|| invalid("enabled is true or false."))?;
        fields |= CONTROL_MODULE_SET_ENABLED;
        off = u32::from(!enabled);
    }
    let mut pairs = Vec::new();
    if let Some(values) = object.get("values") {
        for (key, item) in values
            .as_object()
            .ok_or_else(|| invalid("values maps a key to a number."))?
        {
            let key = key_value(key).ok_or_else(|| invalid("The module has no such setting."))?;
            let item = item
                .as_u64()
                .and_then(|item| u32::try_from(item).ok())
                .ok_or_else(|| invalid("A value is a whole number."))?;
            pairs.push((key, item));
        }
    }
    if let Some(hidden) = object.get("hiddenValues") {
        fields |= CONTROL_MODULE_SET_HIDDEN;
        for bit in numbers(hidden)? {
            if bit >= 32 || mask >> bit & 1 != 0 {
                return Err(invalid("List each hidden value once."));
            }
            mask |= 1 << bit;
        }
    }
    let order = match object.get("valueOrder") {
        Some(order) => {
            fields |= CONTROL_MODULE_SET_ORDER;
            numbers(order)?
        }
        None => Vec::new(),
    };
    let mut words = vec![
        index,
        fields,
        off,
        mask,
        pairs.len() as u32,
        order.len() as u32,
    ];
    words.extend(pairs.iter().flat_map(|(key, value)| [*key, *value]));
    words.extend(order);
    if 4 * words.len() > CONTROL_PAYLOAD_BYTES {
        return Err(invalid("The change is too long."));
    }
    Ok(words.iter().flat_map(|w| w.to_le_bytes()).collect())
}

impl LookApi {
    /// Whether the runtime answers the module commands. It asks once: a
    /// camera that does not answer yet is asked again at the next read.
    pub fn module_settings(&self) -> bool {
        if let Some(supported) = lock(&self.modules).supported {
            return supported;
        }
        let Ok(mut backend) = try_lock(&self.backend) else {
            return false;
        };
        let answer = match backend.exchange(CONTROL_MODULES_READ, &0u32.to_le_bytes()) {
            Ok((0, _)) => Some(true),
            Ok((CONTROL_UNSUPPORTED, _)) => Some(false),
            _ => None,
        };
        lock(&self.modules).supported = answer;
        answer.unwrap_or(false)
    }

    /// `/v1/modules/catalog`, `/v1/modules/{id}/icon`, `/v1/params/modules`,
    /// `/v1/params/modules/order`, `/v1/params/modules/layout`,
    /// `/v1/params/modules/{id}`. None for another path.
    pub fn handle_modules(
        &self,
        request: &Request,
        body: &[u8],
        context: &RequestContext<'_>,
    ) -> Option<Reply> {
        let parts: Vec<&str> = request.path.split('/').skip(2).collect();
        let result = match parts.as_slice() {
            ["modules", "catalog"] if request.method == "GET" => self.catalog(request),
            ["modules", id, "icon"] if request.method == "GET" => self.icon(request, id),
            ["modules", "catalog"] | ["modules", _, "icon"] => Err(not_allowed()),
            ["params", "modules"] if request.method == "GET" => try_lock(&self.backend)
                .and_then(|mut backend| read_states(backend.as_mut()))
                .map(|states| Reply::json(200, states_json(&states, &ModuleView::of(context)))),
            ["params", "modules"] => Err(not_allowed()),
            ["params", "modules", "order"] if request.method == "PUT" => {
                self.put_order(request, body, context)
            }
            ["params", "modules", "layout"] if request.method == "PUT" => {
                self.put_layout(request, body, context)
            }
            ["params", "modules", id] if request.method == "PATCH" => {
                self.patch_module(request, id, body, context)
            }
            ["params", "modules", ..] => Err(not_allowed()),
            _ => return None,
        };
        Some(result.unwrap_or_else(Reply::problem))
    }

    fn catalog(&self, request: &Request) -> Result<Reply> {
        let language = request
            .query
            .as_deref()
            .into_iter()
            .flat_map(|query| query.split('&'))
            .find_map(|pair| pair.strip_prefix("language="))
            .and_then(|code| Language::parse(code).ok())
            .unwrap_or_default();
        let index = Language::ALL
            .iter()
            .position(|item| *item == language)
            .unwrap_or(0);
        let known = lock(&self.modules).catalogs.get(&index).cloned();
        let cached = match known {
            Some(cached) => cached,
            None => {
                let mut backend = try_lock(&self.backend)?;
                let states = read_states(backend.as_mut())?;
                let modules = states
                    .modules
                    .iter()
                    .map(|state| module_catalog(backend.as_mut(), state, index))
                    .collect::<Result<Vec<_>>>()?;
                let content = json!({"language":language.code(),"modules":modules});
                let hash = sha256(&serde_json::to_vec(&content).expect("JSON value"));
                let catalog = &hash[..16];
                let body = json!({"schema":1,"catalog":catalog,"language":language.code(),"modules":content["modules"]});
                let cached = Arc::new(Cached {
                    tag: format!("\"{catalog}\""),
                    body: serde_json::to_vec(&body).expect("JSON value"),
                });
                lock(&self.modules).catalogs.insert(index, cached.clone());
                cached
            }
        };
        Ok(cached_reply(request, &cached, "application/json"))
    }

    fn icon(&self, request: &Request, id: &str) -> Result<Reply> {
        let known = lock(&self.modules).icons.get(id).cloned();
        let cached = match known {
            Some(cached) => cached,
            None => {
                let mut backend = try_lock(&self.backend)?;
                let states = read_states(backend.as_mut())?;
                let state = states
                    .modules
                    .iter()
                    .find(|state| state.id == id)
                    .ok_or_else(not_found)?;
                let mut pixels = Vec::with_capacity(ICON_SIZE);
                while pixels.len() < ICON_SIZE {
                    let payload = [state.index, pixels.len() as u32]
                        .iter()
                        .flat_map(|w| w.to_le_bytes())
                        .collect::<Vec<u8>>();
                    let (result, bytes) = backend.exchange(CONTROL_MODULE_TILE, &payload)?;
                    if result == CONTROL_INVALID {
                        return Err(problem(404, "icon_not_found", "This module has no icon."));
                    }
                    let bytes = checked(result, bytes)?;
                    if bytes.is_empty() || bytes.len() > ICON_SIZE - pixels.len() {
                        return Err(malformed());
                    }
                    pixels.extend_from_slice(&bytes);
                }
                let png = icon_png(&pixels);
                let cached = Arc::new(Cached {
                    tag: format!("\"{}\"", &sha256(&png)[..16]),
                    body: png,
                });
                lock(&self.modules)
                    .icons
                    .insert(id.to_owned(), cached.clone());
                cached
            }
        };
        Ok(cached_reply(request, &cached, "image/png"))
    }

    /// The checks that each change shares, as for the recipes, and its JSON
    /// body.
    pub(super) fn admit(
        &self,
        request: &Request,
        body: &[u8],
        context: &RequestContext<'_>,
    ) -> Result<Value> {
        admit_change(request, context)?;
        serde_json::from_slice(body)
            .map_err(|_| problem(400, "invalid_json", "The body is not valid JSON."))
    }

    fn patch_module(
        &self,
        request: &Request,
        id: &str,
        body: &[u8],
        context: &RequestContext<'_>,
    ) -> Result<Reply> {
        let value = self.admit(request, body, context)?;
        let mut backend = try_lock(&self.backend)?;
        let states = read_states(backend.as_mut())?;
        let state = states
            .modules
            .iter()
            .find(|state| state.id == id)
            .ok_or_else(not_found)?;
        let payload = patch_payload(state.index, &value)?;
        let (result, bytes) = backend.exchange(CONTROL_MODULE_SET, &payload)?;
        // More settings or values than any page has.
        if result == CONTROL_BAD_LENGTH {
            return Err(invalid("The camera does not offer this change."));
        }
        checked(result, bytes)?;
        let states = read_states(backend.as_mut())?;
        let state = states
            .modules
            .iter()
            .find(|state| state.id == id)
            .ok_or_else(not_found)?;
        let mut body = state_json(state, &ModuleView::of(context));
        body["revision"] = json!(states.revision);
        Ok(Reply::json(200, body))
    }

    fn put_order(
        &self,
        request: &Request,
        body: &[u8],
        context: &RequestContext<'_>,
    ) -> Result<Reply> {
        let value = self.admit(request, body, context)?;
        let ids: Vec<&str> = value
            .as_object()
            .filter(|object| object.len() == 1)
            .and_then(|object| object.get("modules")?.as_array())
            .and_then(|ids| ids.iter().map(Value::as_str).collect())
            .ok_or_else(|| invalid("Send the IDs of the orderable modules."))?;
        let mut backend = try_lock(&self.backend)?;
        let states = read_states(backend.as_mut())?;
        let orderable = states
            .modules
            .iter()
            .filter(|state| state.word & CONTROL_MODULE_ORDERABLE != 0);
        if ids.len() != orderable.clone().count()
            || orderable
                .clone()
                .any(|state| ids.iter().filter(|id| **id == state.id).count() != 1)
        {
            return Err(invalid("List each orderable module once."));
        }
        let payload: Vec<u8> = ids
            .iter()
            .filter_map(|id| states.modules.iter().find(|state| state.id == *id))
            .flat_map(|state| state.index.to_le_bytes())
            .collect();
        let (result, bytes) = backend.exchange(CONTROL_MODULES_ORDER, &payload)?;
        checked(result, bytes)?;
        Ok(Reply::json(
            200,
            states_json(&read_states(backend.as_mut())?, &ModuleView::of(context)),
        ))
    }
}

/// The checks of a change of the camera or the card that need no body: the
/// start that the phone read, no software operation that waits, no query.
pub(super) fn admit_change(request: &Request, context: &RequestContext<'_>) -> Result<()> {
    if header(request, "x-grkit-boot-id")? != context.boot_id {
        return Err(problem(
            409,
            "context_changed",
            "The camera started again. Read its state before changing it.",
        ));
    }
    if context.software_pending {
        return Err(problem(
            409,
            "software_pending",
            "Finish or cancel the pending software operation first.",
        ));
    }
    if request.query.is_some() {
        return Err(problem(
            422,
            "invalid_query",
            "This mutation query is not supported.",
        ));
    }
    Ok(())
}

fn invalid_layout() -> Problem {
    problem(
        400,
        "invalid_layout",
        "Send toolbar, root, and hidden: arrays of module IDs, each once, at most toolbarMax in the toolbar.",
    )
}
fn layout_changed() -> Problem {
    problem(
        409,
        "layout_changed",
        "The modules that ADJ can show differ. Read the modules again.",
    )
}

/// The module IDs of each place of a layout body, in order: toolbar, root,
/// hidden.
fn layout_places(value: &Value) -> Result<[Vec<&str>; 3]> {
    let object = value
        .as_object()
        .filter(|object| object.len() == 3)
        .ok_or_else(invalid_layout)?;
    let place = |key: &str| -> Result<Vec<&str>> {
        object
            .get(key)
            .and_then(Value::as_array)
            .and_then(|ids| ids.iter().map(Value::as_str).collect())
            .ok_or_else(invalid_layout)
    };
    let places = [place("toolbar")?, place("root")?, place("hidden")?];
    let ids: Vec<&str> = places.iter().flatten().copied().collect();
    if places[0].len() > capabilities::ADJ_ENTRIES as usize
        || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
    {
        return Err(invalid_layout());
    }
    Ok(places)
}

impl LookApi {
    /// Place each module that ADJ can show, as MENU > ADJ Order does.
    fn put_layout(
        &self,
        request: &Request,
        body: &[u8],
        context: &RequestContext<'_>,
    ) -> Result<Reply> {
        let value = self.admit(request, body, context)?;
        let places = layout_places(&value)?;
        let mut backend = try_lock(&self.backend)?;
        let states = read_states(backend.as_mut())?;
        let placeable: BTreeSet<&str> = states
            .modules
            .iter()
            .filter(|state| state.word & CONTROL_MODULE_PLACEABLE != 0)
            .map(|state| state.id.as_str())
            .collect();
        let sent: BTreeSet<&str> = places.iter().flatten().copied().collect();
        if sent != placeable {
            return Err(layout_changed());
        }
        let mut payload = Vec::with_capacity(4 * sent.len());
        for (ids, place) in places.iter().zip([
            CONTROL_MODULE_TOOLBAR,
            CONTROL_MODULE_ROOT,
            CONTROL_MODULE_HIDDEN,
        ]) {
            for id in ids {
                let state = states.modules.iter().find(|state| state.id == *id);
                let index = state.ok_or_else(layout_changed)?.index;
                payload.extend((index | place << CONTROL_MODULE_PLACE_SHIFT).to_le_bytes());
            }
        }
        let (result, bytes) = backend.exchange(CONTROL_MODULES_LAYOUT, &payload)?;
        match result {
            CONTROL_CHANGED => return Err(layout_changed()),
            CONTROL_INVALID => return Err(invalid_layout()),
            _ => checked(result, bytes)?,
        };
        Ok(Reply::json(
            200,
            states_json(&read_states(backend.as_mut())?, &ModuleView::of(context)),
        ))
    }
}

fn not_allowed() -> Problem {
    problem(
        405,
        "method_not_allowed",
        "This method is not supported for the resource.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A camera with one module: a page of two settings with options.
    struct Fake {
        sent: Vec<(u32, Vec<u8>)>,
        value: u32,
        answer: u32,
    }
    fn words(items: &[u32]) -> Vec<u8> {
        items.iter().flat_map(|w| w.to_le_bytes()).collect()
    }
    fn record(kind: u32, a: u32, b: u32, text: &str) -> Vec<u32> {
        let units: Vec<u16> = text.encode_utf16().collect();
        let mut out = vec![kind | (units.len() as u32) << 8, a, b];
        for pair in units.chunks(2) {
            out.push(u32::from(pair[0]) | u32::from(*pair.get(1).unwrap_or(&0)) << 16);
        }
        out
    }
    impl RuntimeBackend for Fake {
        fn exchange(&mut self, command: u32, payload: &[u8]) -> Result<(u32, Vec<u8>)> {
            self.sent.push((command, payload.to_vec()));
            Ok(match command {
                CONTROL_MODULES_READ => {
                    let flags = CONTROL_MODULE_ENABLED
                        | CONTROL_MODULE_ORDERABLE
                        | CONTROL_MODULE_PLACEABLE
                        | CONTROL_MODULE_ROOT << CONTROL_MODULE_PLACE_SHIFT;
                    let name = u32::from_le_bytes(*b"GUID");
                    let rest = u32::from_le_bytes(*b"ES\0\0");
                    (
                        0,
                        words(&[
                            1,
                            12,
                            1,
                            0,
                            1,
                            capabilities::bit_of("TONE"),
                            flags,
                            name,
                            rest,
                            1 << 3,
                            1 | 3 << 8,
                            7,
                            self.value,
                            u32::from_le_bytes([0, 2, 1, 0]),
                        ]),
                    )
                }
                CONTROL_MODULE_TEXTS => {
                    let mut out = vec![CONTROL_MODULE_TEXTS_END, 3];
                    out.extend(record(
                        CONTROL_MODULE_RECORD_MODULE,
                        CONTROL_MODULE_HAS_OPTIONS | CONTROL_MODULE_ORDERED,
                        7,
                        "Guides",
                    ));
                    out.extend(record(CONTROL_MODULE_RECORD_SETTING, 7, 0, "Style"));
                    out.extend(record(CONTROL_MODULE_RECORD_VALUE, 2, 0, "ガイド"));
                    (0, words(&out))
                }
                CONTROL_MODULE_TILE => (
                    0,
                    vec![9; 2048.min(ICON_SIZE - word(payload, 4).unwrap() as usize)],
                ),
                // The layout of the test: GUIDES hidden.
                CONTROL_MODULES_LAYOUT if self.answer == 0 => {
                    let hidden = words(&[CONTROL_MODULE_HIDDEN << CONTROL_MODULE_PLACE_SHIFT]);
                    (
                        if payload == hidden {
                            0
                        } else {
                            CONTROL_INVALID
                        },
                        Vec::new(),
                    )
                }
                _ => (self.answer, Vec::new()),
            })
        }
    }

    #[test]
    fn the_state_and_the_catalog_decode() {
        let mut camera = Fake {
            sent: Vec::new(),
            value: 2,
            answer: 0,
        };
        let states = read_states(&mut camera).unwrap();
        let value = states_json(&states, &ModuleView::default());
        assert_eq!(value["features"], json!({"looks": true, "recipes": false}));
        assert_eq!(value["revision"], 12);
        assert_eq!(value["modules"][0]["id"], "GUIDES");
        assert_eq!(value["modules"][0]["place"], "root");
        assert_eq!(value["modules"][0]["values"]["00000007"], 2);
        assert_eq!(value["modules"][0]["hiddenValues"], json!([3]));
        assert_eq!(value["modules"][0]["valueOrder"], json!([0, 2, 1]));
        let catalog = module_catalog(&mut camera, &states.modules[0], 3).unwrap();
        assert_eq!(catalog["title"], "Guides");
        assert_eq!(
            catalog["options"],
            json!({"setting":"00000007","ordered":true})
        );
        assert_eq!(
            catalog["settings"][0]["values"][0],
            json!({"value":2,"label":"ガイド"})
        );
    }

    /// Counts the commands that reach the camera.
    struct Counted(Fake, Arc<std::sync::atomic::AtomicUsize>);
    impl RuntimeBackend for Counted {
        fn exchange(&mut self, command: u32, payload: &[u8]) -> Result<(u32, Vec<u8>)> {
            self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.0.exchange(command, payload)
        }
    }
    fn call(api: &LookApi, head: &str, body: &[u8], pending: bool) -> Reply {
        let text = format!("{head} HTTP/1.1\r\nContent-Length: {}\r\n\r\n", body.len());
        let request = crate::parse(text.as_bytes()).unwrap();
        call_with(api, request, body, pending)
    }
    fn call_with(api: &LookApi, request: Request, body: &[u8], pending: bool) -> Reply {
        let context = RequestContext {
            boot_id: "boot",
            card_id: "card".into(),
            software_pending: pending,
            root: std::path::Path::new("/"),
            release_keys: &[],
            family: None,
        };
        api.handle_modules(&request, body, &context).unwrap()
    }
    fn change(api: &LookApi, head: &str, body: &[u8], pending: bool) -> Reply {
        let text = format!(
            "{head} HTTP/1.1\r\nContent-Length: {}\r\nX-GRKit-Boot-Id: boot\r\n\r\n",
            body.len()
        );
        call_with(api, crate::parse(text.as_bytes()).unwrap(), body, pending)
    }
    fn code(reply: &Reply) -> String {
        let value: Value = serde_json::from_slice(&reply.body).unwrap();
        value["code"].as_str().unwrap().to_owned()
    }

    #[test]
    fn a_catalog_is_read_once_and_a_change_maps_its_result() {
        let commands = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let camera = Fake {
            sent: Vec::new(),
            value: 2,
            answer: CONTROL_BUSY,
        };
        let api = LookApi::new(Box::new(Counted(camera, commands.clone())));
        let first = call(&api, "GET /v1/modules/catalog?language=ja", b"", false);
        assert_eq!(first.status, 200);
        let value: Value = serde_json::from_slice(&first.body).unwrap();
        assert_eq!(
            (value["schema"].as_u64(), value["language"].as_str()),
            (Some(1), Some("ja"))
        );
        let catalog = value["catalog"].as_str().unwrap();
        assert_eq!(catalog.len(), 16);
        let tag = format!("\"{catalog}\"");
        assert_eq!(first.header_value("ETag"), Some(tag.as_str()));
        let read = commands.load(std::sync::atomic::Ordering::SeqCst);
        let text =
            format!("GET /v1/modules/catalog?language=ja HTTP/1.1\r\nIf-None-Match: {tag}\r\n\r\n");
        let again = call_with(&api, crate::parse(text.as_bytes()).unwrap(), b"", false);
        assert!(again.status == 304 && again.body.is_empty());
        assert_eq!(commands.load(std::sync::atomic::Ordering::SeqCst), read);

        let patch = br#"{"enabled":false}"#;
        assert_eq!(
            code(&call(&api, "PATCH /v1/params/modules/GUIDES", patch, false)),
            "missing_header"
        );
        assert_eq!(
            code(&change(
                &api,
                "PATCH /v1/params/modules/GUIDES",
                patch,
                true
            )),
            "software_pending"
        );
        assert_eq!(
            code(&change(
                &api,
                "PATCH /v1/params/modules/OTHER",
                patch,
                false
            )),
            "module_not_found"
        );
        assert_eq!(
            code(&change(
                &api,
                "PATCH /v1/params/modules/GUIDES",
                patch,
                false
            )),
            "camera_busy"
        );
    }

    #[test]
    fn the_icon_is_a_png_of_the_tile() {
        let pixels: Vec<u8> = (0..ICON_SIZE).map(|i| (i * 7 % 251) as u8).collect();
        assert_eq!(
            grkit_core::png::decode_rgba8(&icon_png(&pixels)).unwrap(),
            (40, 40, pixels)
        );
    }

    #[test]
    fn a_patch_packs_every_field_and_rejects_a_bad_one() {
        let payload = patch_payload(
            2,
            &json!({"enabled":false,"values":{"00000007":3},"hiddenValues":[5],"valueOrder":[0,2,1]}),
        )
        .unwrap();
        assert_eq!(
            payload,
            words(&[
                2,
                CONTROL_MODULE_SET_ENABLED | CONTROL_MODULE_SET_HIDDEN | CONTROL_MODULE_SET_ORDER,
                1,
                1 << 5,
                1,
                3,
                7,
                3,
                0,
                2,
                1
            ])
        );
        for bad in [
            json!({}),
            json!({"other":1}),
            json!({"enabled":1}),
            json!({"values":{"x":1}}),
            json!({"hiddenValues":[40]}),
            json!({"hiddenValues":[1,1]}),
        ] {
            assert_eq!(
                patch_payload(0, &bad).unwrap_err().code,
                "invalid_request",
                "{bad}"
            );
        }
    }

    #[test]
    fn a_layout_places_each_module_once() {
        let layout = |answer| {
            LookApi::new(Box::new(Fake {
                sent: Vec::new(),
                value: 2,
                answer,
            }))
        };
        let api = layout(0);
        let read = call(&api, "GET /v1/params/modules", b"", false);
        let value: Value = serde_json::from_slice(&read.body).unwrap();
        assert_eq!(value["toolbarMax"], 4);
        assert_eq!(value["modules"][0]["placeable"], true);
        let hidden = br#"{"toolbar":[],"root":[],"hidden":["GUIDES"]}"#;
        let put = change(&api, "PUT /v1/params/modules/layout", hidden, false);
        assert_eq!(put.status, 200);
        let value: Value = serde_json::from_slice(&put.body).unwrap();
        assert_eq!(value["toolbarMax"], 4);
        assert_eq!(value["modules"][0]["placeable"], true);
        for (body, status, error) in [
            (
                &br#"{"toolbar":["GUIDES"],"root":["GUIDES"],"hidden":[]}"#[..],
                400,
                "invalid_layout",
            ),
            (
                br#"{"toolbar":["A","B","C","D","GUIDES"],"root":[],"hidden":[]}"#,
                400,
                "invalid_layout",
            ),
            (br#"{"root":["GUIDES"]}"#, 400, "invalid_layout"),
            (
                br#"{"toolbar":[],"root":[],"hidden":[]}"#,
                409,
                "layout_changed",
            ),
            (
                br#"{"toolbar":[],"root":["GUIDES","OTHER"],"hidden":[]}"#,
                409,
                "layout_changed",
            ),
        ] {
            let reply = change(&api, "PUT /v1/params/modules/layout", body, false);
            assert_eq!((reply.status, code(&reply)), (status, error.to_owned()));
        }
        for (answer, error) in [
            (CONTROL_BUSY, "camera_busy"),
            (CONTROL_CHANGED, "layout_changed"),
        ] {
            let reply = change(
                &layout(answer),
                "PUT /v1/params/modules/layout",
                hidden,
                false,
            );
            assert_eq!(code(&reply), error);
        }
    }
}
