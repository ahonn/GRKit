//! The recipes of the running camera (`runtime/control_recipes.h`). The
//! recipes live in the App task, so each change applies at once, and the
//! camera saves the recipe file as after a change in MENU. A recipe is
//! found by its ID, which stays with it when it moves.
use super::{header, problem, try_lock, LookApi, RequestContext, Result};
use crate::{
    api::Reply,
    changes::{Changed, Changes},
    protocol::Problem,
    runtime_backend::{camera_busy, recipe_name, RuntimeBackend},
    Request,
};
use grkit_core::abi::*;
use grkit_core::recipes::{self, Family, Look, Recipe};
use serde_json::{json, Value};

/// A recipe of the camera as its words give it.
struct Entry {
    id: u32,
    words: [u32; RECIPE_WORDS],
    name: Option<String>,
}
struct List {
    entries: Vec<Entry>,
    on: usize,
    current: Option<u32>,
    revision: u32,
    undo: u32,
}

fn word(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}
fn malformed() -> Problem {
    Problem::new(
        503,
        "runtime_invalid",
        "The recipe list of the camera is malformed.",
    )
}
/// The result of a recipe command, for the codes every command shares.
fn checked(result: u32, bytes: Vec<u8>) -> Result<Vec<u8>> {
    match result {
        0 => Ok(bytes),
        CONTROL_BUSY => Err(camera_busy()),
        CONTROL_CHANGED => Err(problem(
            409,
            "runtime_changed",
            "The camera does not list this recipe. Read the recipes again.",
        )),
        CONTROL_UNSUPPORTED => Err(problem(
            503,
            "runtime_protocol_unsupported",
            "This camera has no recipes to manage.",
        )),
        _ => Err(Problem::new(
            503,
            "runtime_invalid",
            "The camera rejected the command.",
        )),
    }
}

fn read_list(backend: &mut dyn RuntimeBackend) -> Result<List> {
    let mut entries = Vec::new();
    let mut first = 0u32;
    loop {
        let (result, bytes) = backend.exchange(CONTROL_RECIPES_READ, &first.to_le_bytes())?;
        let bytes = checked(result, bytes)?;
        let header = |i: usize| word(&bytes, 4 * i).ok_or_else(malformed);
        let (count, on, current, revision, undo, at, k) = (
            header(0)?,
            header(1)?,
            header(2)?,
            header(3)?,
            header(4)?,
            header(5)?,
            header(6)?,
        );
        if at != first
            || count as usize > RECIPE_SLOTS
            || bytes.len() != CONTROL_RECIPES_HEADER + k as usize * CONTROL_RECIPE_ENTRY
        {
            return Err(malformed());
        }
        for i in 0..k as usize {
            let base = CONTROL_RECIPES_HEADER + i * CONTROL_RECIPE_ENTRY;
            let at = |w: usize| word(&bytes, base + 4 * w).ok_or_else(malformed);
            let mut words = [0; RECIPE_WORDS];
            for (w, slot) in words.iter_mut().enumerate() {
                *slot = at(1 + w)?;
            }
            let name: Vec<u32> = (0..RECIPE_NAME_WORDS)
                .map(|w| at(1 + RECIPE_WORDS + w))
                .collect::<Result<_>>()?;
            entries.push(Entry {
                id: at(0)?,
                words,
                name: recipe_name(&name),
            });
        }
        first += k;
        if first >= count || k == 0 {
            if first != count {
                return Err(malformed());
            }
            let current = (current != CONTROL_RECIPE_NONE)
                .then(|| entries.get(current as usize).map(|e| e.id))
                .flatten();
            return Ok(List {
                entries,
                on: on as usize,
                current,
                revision,
                undo,
            });
        }
    }
}

/// The 7-bit name words of `name`, which `check_name` accepted.
fn name_words(name: &str) -> [u32; RECIPE_NAME_WORDS] {
    let mut words = [0; RECIPE_NAME_WORDS];
    for (i, c) in name.bytes().take(RECIPE_NAME_CHARS).enumerate() {
        words[i / 4] |= u32::from(c & 0x7f) << (7 * (i % 4));
    }
    words
}

impl LookApi {
    /// `/v1/recipes`, `/v1/recipes/order`, `/v1/recipes/undo`,
    /// `/v1/recipes/{id}`, `/v1/recipes/{id}/apply`. None for another path.
    pub fn handle_recipes(
        &self,
        request: &Request,
        body: &[u8],
        context: &RequestContext<'_>,
        changes: &Changes,
    ) -> Option<Reply> {
        let route = request.path.strip_prefix("/v1/recipes")?;
        let parts: Vec<&str> = route.split('/').filter(|p| !p.is_empty()).collect();
        if !route.is_empty() && !route.starts_with('/') {
            return None;
        }
        let result = (|| -> Result<Reply> {
            let family = context.family.ok_or_else(|| {
                problem(
                    503,
                    "camera_unavailable",
                    "The camera model is not known yet.",
                )
            })?;
            let mut backend = try_lock(&self.backend)?;
            let names = self.effect_names(backend.as_mut(), context);
            if request.method == "GET" {
                return match parts.as_slice() {
                    [] => Ok(Reply::json(
                        200,
                        list_json(&read_list(backend.as_mut())?, family, &names, context),
                    )),
                    _ => Err(not_found()),
                };
            }
            if header(request, "x-grkit-boot-id")? != context.boot_id {
                return Err(problem(
                    409,
                    "context_changed",
                    "The camera started again. Read the recipes before changing them.",
                ));
            }
            if context.software_pending {
                return Err(problem(
                    409,
                    "software_pending",
                    "Finish or cancel the pending software operation before changing recipes.",
                ));
            }
            let value: Value = if body.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(body)
                    .map_err(|_| problem(400, "invalid_json", "The body is not valid JSON."))?
            };
            let id_of = |text: &str| text.parse::<u32>().map_err(|_| not_found());
            let b = backend.as_mut();
            let mut extra = json!({});
            match (request.method.as_str(), parts.as_slice()) {
                ("PATCH", [id]) => patch(b, id_of(id)?, &value)?,
                ("DELETE", [id]) => {
                    let (result, bytes) =
                        b.exchange(CONTROL_RECIPE_DELETE, &id_of(id)?.to_le_bytes())?;
                    checked(result, bytes)?;
                }
                ("PUT", ["order"]) => order(b, &value)?,
                ("POST", ["undo"]) => {
                    let (result, bytes) = b.exchange(CONTROL_RECIPE_UNDO, &[])?;
                    if result == CONTROL_INVALID {
                        return Err(problem(
                            409,
                            "nothing_to_undo",
                            "The camera has no recipe to undo.",
                        ));
                    }
                    let bytes = checked(result, bytes)?;
                    extra = json!({"undo": word(&bytes, 0).ok_or_else(malformed)?});
                }
                ("POST", [id, "apply"]) => extra = apply(b, id_of(id)?)?,
                ("POST", []) => extra = add(b, family, &value)?,
                _ => {
                    return Err(problem(
                        405,
                        "method_not_allowed",
                        "This method is not supported for the resource.",
                    ))
                }
            }
            changes.note(Changed::Looks);
            let mut body = list_json(&read_list(b)?, family, &names, context);
            if let (Some(body), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
                body.extend(extra.clone());
            }
            Ok(Reply::json(200, body))
        })();
        Some(result.unwrap_or_else(Reply::problem))
    }

    /// The ID and name of each Look on the card, by the recipe code of its
    /// effect. Empty while the Looks cannot be read.
    fn effect_names(
        &self,
        backend: &mut dyn RuntimeBackend,
        context: &RequestContext<'_>,
    ) -> Vec<(u16, String, String)> {
        self.view(backend, context)
            .map(|view| {
                view.looks
                    .iter()
                    .map(|look| {
                        (
                            recipes::effect_code(&look.file.id),
                            look.file.id.clone(),
                            look.file.name.clone(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn not_found() -> Problem {
    problem(
        404,
        "recipe_not_found",
        "The camera has no recipe with this ID.",
    )
}

fn list_json(
    list: &List,
    family: Family,
    names: &[(u16, String, String)],
    context: &RequestContext<'_>,
) -> Value {
    let recipes: Vec<Value> = list
        .entries
        .iter()
        .enumerate()
        .filter_map(|(slot, entry)| {
            let recipe =
                Recipe::from_words(entry.name.clone().unwrap_or_default(), family, entry.words)
                    .ok()?;
            let mut value = serde_json::to_value(&recipe).ok()?;
            let effect = match &recipe.look {
                Look::Effect { code, .. } => names.iter().find(|(c, _, _)| c == code),
                _ => None,
            };
            value["id"] = json!(entry.id);
            value["name"] = json!(entry.name);
            value["enabled"] = json!(slot < list.on);
            value["current"] = json!(list.current == Some(entry.id));
            value["effectId"] = json!(effect.map(|(_, id, _)| id));
            value["effectName"] = json!(effect.map(|(_, _, name)| name));
            value["ready"] = json!(!matches!(recipe.look, Look::Effect { .. }) || effect.is_some());
            Some(value)
        })
        .collect();
    json!({"bootId":context.boot_id,"revision":list.revision,"undo":list.undo,
        "counts":{"listed":list.entries.len(),"on":list.on},
        "limits":{"listed":RECIPE_SLOTS,"on":RECIPE_ON_MAX},"recipes":recipes})
}

fn patch(backend: &mut dyn RuntimeBackend, id: u32, value: &Value) -> Result<()> {
    let object = value
        .as_object()
        .filter(|o| o.len() == 1)
        .ok_or_else(|| problem(422, "invalid_patch", "Change one of enabled or name."))?;
    if let Some(enabled) = object.get("enabled") {
        let enabled = enabled
            .as_bool()
            .ok_or_else(|| problem(422, "invalid_patch", "enabled is true or false."))?;
        let payload = [id.to_le_bytes(), u32::from(!enabled).to_le_bytes()].concat();
        let (result, bytes) = backend.exchange(CONTROL_RECIPE_TURN, &payload)?;
        if result == CONTROL_INVALID {
            return Err(problem(
                409,
                "menu_full",
                "The camera menu shows at most 40 recipes. Turn one off first.",
            ));
        }
        return checked(result, bytes).map(drop);
    }
    let name = match object.get("name") {
        Some(Value::Null) => String::new(),
        Some(Value::String(name)) if name.trim().is_empty() => String::new(),
        Some(Value::String(name)) => {
            let name = name.trim().to_owned();
            recipes::check_name(&name).map_err(|_| {
                problem(
                    422,
                    "invalid_name",
                    "Use 1 to 16 letters, digits, spaces, or symbols.",
                )
            })?;
            name
        }
        _ => {
            return Err(problem(
                422,
                "invalid_patch",
                "Change one of enabled or name.",
            ))
        }
    };
    // The mailbox carries whole words: the name is NUL-padded.
    let mut payload = [id.to_le_bytes().as_slice(), name.as_bytes()].concat();
    payload.resize(payload.len().next_multiple_of(4), 0);
    let (result, bytes) = backend.exchange(CONTROL_RECIPE_RENAME, &payload)?;
    checked(result, bytes).map(drop)
}

fn order(backend: &mut dyn RuntimeBackend, value: &Value) -> Result<()> {
    let ids: Vec<u32> = value["ids"]
        .as_array()
        .and_then(|ids| {
            ids.iter()
                .map(|id| id.as_u64().and_then(|id| u32::try_from(id).ok()))
                .collect()
        })
        .ok_or_else(|| {
            problem(
                422,
                "invalid_order",
                "Send the IDs of the recipes that are on.",
            )
        })?;
    let payload: Vec<u8> = ids.iter().flat_map(|id| id.to_le_bytes()).collect();
    let (result, bytes) = backend.exchange(CONTROL_RECIPES_ORDER, &payload)?;
    if result == CONTROL_INVALID {
        return Err(problem(
            422,
            "invalid_order",
            "Each recipe that is on, once.",
        ));
    }
    checked(result, bytes).map(drop)
}

/// Apply as OK in ADJ does; the answer names the settings it left out.
fn apply(backend: &mut dyn RuntimeBackend, id: u32) -> Result<Value> {
    let (result, bytes) = backend.exchange(CONTROL_RECIPE_APPLY, &id.to_le_bytes())?;
    let bytes = checked(result, bytes)?;
    let outcome = word(&bytes, 0).ok_or_else(malformed)?;
    match outcome & 0xff {
        0 => {}
        1 => {
            return Err(problem(
                409,
                "recipe_look_missing",
                "The Film Look of this recipe is not on the camera.",
            ))
        }
        _ => {
            return Err(problem(
                409,
                "recipe_unsupported",
                "This camera does not have a setting of this recipe.",
            ))
        }
    }
    let modules: Vec<&str> = recipes::setting_modules()
        .into_iter()
        .enumerate()
        .filter(|(m, _)| outcome & (0x200 << m) != 0)
        .map(|(_, module)| module)
        .collect();
    Ok(json!({"applied": id, "skipped": {"grain": outcome & 0x100 != 0, "modules": modules}}))
}

/// Add a preset of the site for this camera family.
fn add(backend: &mut dyn RuntimeBackend, family: Family, value: &Value) -> Result<Value> {
    let id = value["preset"]
        .as_str()
        .ok_or_else(|| problem(422, "invalid_preset", "Name a preset."))?;
    let recipe = recipes::presets()
        .into_iter()
        .find(|preset| preset.id == id)
        .and_then(|preset| preset.recipes.into_iter().find(|r| r.family == family))
        .ok_or_else(|| {
            problem(
                404,
                "preset_not_found",
                "This camera has no preset with this ID.",
            )
        })?;
    let words = recipe.words().map_err(|_| {
        problem(
            422,
            "invalid_preset",
            "The preset does not fit this camera.",
        )
    })?;
    let payload: Vec<u8> = words
        .iter()
        .chain(name_words(&recipe.name).iter())
        .flat_map(|w| w.to_le_bytes())
        .collect();
    let list = read_list(backend)?;
    let (result, bytes) = backend.exchange(CONTROL_RECIPE_ADD, &payload)?;
    if result == CONTROL_INVALID && list.entries.len() >= RECIPE_SLOTS {
        return Err(problem(
            409,
            "card_full",
            "The card holds at most 64 recipes. Delete one first.",
        ));
    }
    let bytes = checked(result, bytes)?;
    Ok(json!({"added": word(&bytes, 0).ok_or_else(malformed)?}))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A camera with `recipes` (ID, words, name words), the first `on` on.
    struct Fake {
        recipes: Vec<(u32, [u32; RECIPE_WORDS], [u32; RECIPE_NAME_WORDS])>,
        on: usize,
        sent: Vec<(u32, Vec<u8>)>,
        answer: u32,
    }
    impl RuntimeBackend for Fake {
        fn exchange(&mut self, command: u32, payload: &[u8]) -> Result<(u32, Vec<u8>)> {
            self.sent.push((command, payload.to_vec()));
            if command != CONTROL_RECIPES_READ {
                return Ok((self.answer, 0x300u32.to_le_bytes().to_vec()));
            }
            let first = word(payload, 0).unwrap() as usize;
            let page: Vec<_> = self
                .recipes
                .iter()
                .skip(first)
                .take(CONTROL_RECIPES_PAGE)
                .collect();
            let mut words = vec![
                self.recipes.len() as u32,
                self.on as u32,
                1,
                9,
                0,
                first as u32,
                page.len() as u32,
            ];
            for (id, w, name) in page {
                words.push(*id);
                words.extend(w);
                words.extend(name);
            }
            Ok((0, words.iter().flat_map(|w| w.to_le_bytes()).collect()))
        }
    }
    fn native_words() -> [u32; RECIPE_WORDS] {
        recipes::presets()
            .into_iter()
            .find(|p| p.id == "bennet-800")
            .unwrap()
            .recipes[0]
            .words()
            .unwrap()
    }
    fn fake(n: usize) -> Fake {
        Fake {
            recipes: (0..n as u32)
                .map(|i| {
                    (
                        100 + i,
                        native_words(),
                        name_words(if i == 0 { "Odd" } else { "" }),
                    )
                })
                .collect(),
            on: n.min(1),
            sent: Vec::new(),
            answer: 0,
        }
    }

    #[test]
    fn the_list_reads_every_page_with_names_and_the_current_recipe() {
        let mut camera = fake(40);
        let list = read_list(&mut camera).unwrap();
        assert_eq!(
            (list.entries.len(), list.on, list.current),
            (40, 1, Some(101))
        );
        assert_eq!(camera.sent.len(), 2);
        assert_eq!(list.entries[0].name.as_deref(), Some("Odd"));
        assert_eq!(list.entries[1].name, None);
        let boot = "b";
        let context = RequestContext {
            boot_id: boot,
            card_id: "c",
            software_pending: false,
            root: std::path::Path::new("/"),
            release_keys: &[],
            family: Some(Family::Gr3),
        };
        let value = list_json(&list, Family::Gr3, &[], &context);
        assert_eq!(value["recipes"][0]["enabled"], true);
        assert_eq!(value["recipes"][1]["enabled"], false);
        assert_eq!(value["recipes"][1]["current"], true);
        assert_eq!(value["recipes"][0]["look"]["kind"], "native");
    }

    #[test]
    fn changes_map_their_results() {
        let mut camera = fake(1);
        camera.answer = CONTROL_INVALID;
        let full = patch(&mut camera, 100, &json!({"enabled": true})).unwrap_err();
        assert_eq!(full.code, "menu_full");
        assert_eq!(
            camera.sent[0].1,
            [100u32.to_le_bytes(), 0u32.to_le_bytes()].concat()
        );
        camera.answer = CONTROL_BUSY;
        assert_eq!(
            patch(&mut camera, 100, &json!({"name": "Street"}))
                .unwrap_err()
                .code,
            "camera_busy"
        );
        // NUL-padded to whole words.
        assert_eq!(
            camera.sent[1].1,
            [100u32.to_le_bytes().as_slice(), b"Street\0\0"].concat()
        );
        assert_eq!(
            patch(&mut camera, 100, &json!({"name": "日本"}))
                .unwrap_err()
                .code,
            "invalid_name"
        );
        camera.answer = 0;
        let applied = apply(&mut camera, 100).unwrap();
        assert_eq!(applied["skipped"]["grain"], true);
        assert_eq!(applied["skipped"]["modules"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn names_pack_seven_bits_per_character() {
        assert_eq!(
            recipe_name(&name_words("Bennet 800")).as_deref(),
            Some("Bennet 800")
        );
        assert_eq!(recipe_name(&name_words("")), None);
    }
}
