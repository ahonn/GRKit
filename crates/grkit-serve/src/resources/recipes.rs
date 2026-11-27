//! The recipes of the running camera (`runtime/control_recipes.h`). The
//! recipes live in the App task, so each change applies at once, and the
//! camera saves the recipe file as after a change in MENU. A recipe is
//! found by its ID, which stays with it when it moves.
use super::{looks::EffectName, problem, try_lock, RequestContext, Resources, Result};
use crate::codec::{word, Codes, Page};
use crate::{
    api::Reply,
    camera_looks::recipe_name,
    changes::{Changed, Changes},
    protocol::Problem,
    route::RecipeRoute,
    runtime_backend::RuntimeBackend,
    Request,
};
use grkit_core::abi::*;
use grkit_core::recipes::{self, name_words, Family, Look, Recipe};
use grkit_protocol::recipes::RecipeStatus;
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

const CODES: Codes = Codes {
    list: "The recipe list",
    changed: "The camera does not list this recipe. Read the recipes again.",
    unsupported: "This camera has no recipes to manage.",
    invalid: None,
};
fn malformed() -> Problem {
    CODES.malformed()
}
/// The result of a recipe command, for the codes every command shares.
fn checked(result: u32, bytes: Vec<u8>) -> Result<Vec<u8>> {
    CODES.checked((result, bytes))
}

fn read_list(backend: &mut dyn RuntimeBackend) -> Result<List> {
    let mut entries = Vec::new();
    let last = CODES.read_pages(backend, CONTROL_RECIPES_READ, |bytes| {
        let header = |i: usize| word(bytes, 4 * i).ok_or_else(malformed);
        let (count, at, k) = (header(0)?, header(5)?, header(6)?);
        if count as usize > RECIPE_SLOTS
            || bytes.len() != CONTROL_RECIPES_HEADER + k as usize * CONTROL_RECIPE_ENTRY
        {
            return Err(malformed());
        }
        for i in 0..k as usize {
            let base = CONTROL_RECIPES_HEADER + i * CONTROL_RECIPE_ENTRY;
            let at = |w: usize| word(bytes, base + 4 * w).ok_or_else(malformed);
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
        Ok(Page {
            count,
            at,
            entries: k,
        })
    })?;
    let header = |i: usize| word(&last, 4 * i).ok_or_else(malformed);
    let (on, current, revision, undo) = (header(1)?, header(2)?, header(3)?, header(4)?);
    let current = (current != CONTROL_RECIPE_NONE)
        .then(|| entries.get(current as usize).map(|e| e.id))
        .flatten();
    Ok(List {
        entries,
        on: on as usize,
        current,
        revision,
        undo,
    })
}

impl Resources {
    /// A request of the recipes, with a method of its route that the
    /// service admitted (`api::Service`).
    pub fn handle_recipes(
        &self,
        route: &RecipeRoute,
        request: &Request,
        body: &[u8],
        context: &RequestContext<'_>,
        changes: &Changes,
    ) -> Reply {
        let result = (|| -> Result<Reply> {
            let family = context.family.ok_or_else(|| {
                problem(
                    503,
                    "camera_unavailable",
                    "The camera model is not known yet.",
                )
            })?;
            let mut backend = try_lock(&self.backend)?;
            let names = self.effect_names(context);
            if request.method == "GET" {
                return Ok(Reply::json(
                    200,
                    list_json(&read_list(backend.as_mut())?, family, &names, context),
                ));
            }
            let value: Value = if body.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(body)
                    .map_err(|_| problem(400, "invalid_json", "The body is not valid JSON."))?
            };
            let b = backend.as_mut();
            let mut extra = json!({});
            match (request.method.as_str(), route) {
                ("PATCH", RecipeRoute::Recipe(id)) => patch(b, *id, &value)?,
                ("DELETE", RecipeRoute::Recipe(id)) => {
                    let (result, bytes) = b.exchange(CONTROL_RECIPE_DELETE, &id.to_le_bytes())?;
                    checked(result, bytes)?;
                }
                (_, RecipeRoute::Order) => order(b, &value)?,
                (_, RecipeRoute::Undo) => {
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
                (_, RecipeRoute::Apply(id)) => extra = apply(b, *id)?,
                (_, RecipeRoute::Collection) => extra = add(b, family, &value)?,
                (_, RecipeRoute::Recipe(_)) => unreachable!("the route takes PATCH and DELETE"),
            }
            changes.note(Changed::Looks);
            let mut body = list_json(&read_list(b)?, family, &names, context);
            if let (Some(body), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
                body.extend(extra.clone());
            }
            Ok(Reply::json(200, body))
        })();
        result.unwrap_or_else(Reply::problem)
    }
}

fn list_json(
    list: &List,
    family: Family,
    names: &[EffectName],
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
                Look::Effect { code, .. } => names.iter().find(|look| look.code == *code),
                _ => None,
            };
            let status = RecipeStatus {
                id: entry.id,
                name: entry.name.clone(),
                enabled: slot < list.on,
                current: list.current == Some(entry.id),
                effect_id: effect.map(|look| look.id.clone()),
                effect_name: effect.map(|look| look.name.clone()),
                // A Look on the card only does not apply (`recipe_look_missing`).
                ready: match recipe.look {
                    Look::Effect { .. } => effect.is_some_and(|look| !look.stored),
                    _ => true,
                },
                effect_stored: effect.is_some_and(|look| look.stored),
            };
            if let (Some(value), Value::Object(status)) =
                (value.as_object_mut(), serde_json::to_value(status).ok()?)
            {
                value.extend(status);
            }
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
            return Err(Problem::new(
                409,
                "menu_full",
                format!(
                    "The camera menu shows at most {RECIPE_ON_MAX} recipes. Turn one off first."
                ),
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
                Problem::new(
                    422,
                    "invalid_name",
                    format!("Use 1 to {RECIPE_NAME_CHARS} letters, digits, spaces, or symbols."),
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
        return Err(Problem::new(
            409,
            "card_full",
            format!("The card holds at most {RECIPE_SLOTS} recipes. Delete one first."),
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
            card_id: Some("c".into()),
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
    fn a_recipe_whose_look_is_on_the_card_only_is_not_ready() {
        let preset = recipes::presets()
            .into_iter()
            .find(|p| p.id == "cinestill-800t")
            .unwrap();
        let recipe = &preset.recipes[0];
        let list = List {
            entries: vec![Entry {
                id: 7,
                words: recipe.words().unwrap(),
                name: None,
            }],
            on: 1,
            current: None,
            revision: 0,
            undo: 0,
        };
        let id = "vision3-500t-supra-v1";
        let mut names = [EffectName {
            code: recipes::effect_code(id),
            id: id.into(),
            name: "Vision3".into(),
            stored: true,
        }];
        let context = RequestContext {
            boot_id: "b",
            card_id: Some("c".into()),
            root: std::path::Path::new("/"),
            release_keys: &[],
            family: Some(recipe.family),
        };
        let item = |names: &[EffectName]| {
            list_json(&list, recipe.family, names, &context)["recipes"][0].clone()
        };
        let stored = item(&names);
        assert_eq!(
            (
                &stored["effectName"],
                &stored["ready"],
                &stored["effectStored"]
            ),
            (&json!("Vision3"), &json!(false), &json!(true))
        );
        names[0].stored = false;
        let on_camera = item(&names);
        assert_eq!(
            (&on_camera["ready"], &on_camera["effectStored"]),
            (&json!(true), &json!(false))
        );
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
