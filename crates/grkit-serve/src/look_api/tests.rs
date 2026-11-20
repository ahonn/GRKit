use super::*;
use crate::runtime_backend::{LookUse, RuntimeLook, RuntimeLooks};
use grkit_core::{camera, card, imports::Import, packaging, release, testing};
use std::sync::Arc;

const BOOT: &str = "boot-test";
const CARD: &str = "card-test";

/// The effect choices of a camera, which the commands change as the App task
/// does.
#[derive(Default)]
struct Camera {
    looks: RuntimeLooks,
    busy: bool,
    commands: Vec<String>,
}
struct Backend(Arc<Mutex<Camera>>);
impl RuntimeBackend for Backend {
    fn looks(&mut self) -> Result<RuntimeLooks> {
        Ok(self.0.lock().unwrap().looks.clone())
    }
    fn turn(&mut self, key: u32, off: bool) -> Result<()> {
        let mut camera = self.0.lock().unwrap();
        if camera.busy {
            return Err(problem(503, "camera_busy", "Busy."));
        }
        camera.commands.push(format!("turn {key} {off}"));
        let look = camera
            .looks
            .looks
            .iter_mut()
            .find(|look| look.key == key)
            .ok_or_else(|| problem(409, "runtime_changed", "No such effect."))?;
        look.enabled = !off;
        if off && camera.looks.selected == Some(key) {
            camera.looks.selected = None;
        }
        Ok(())
    }
    fn order(&mut self, keys: &[u32]) -> Result<()> {
        let mut camera = self.0.lock().unwrap();
        camera.commands.push(format!("order {keys:?}"));
        for (index, key) in keys.iter().enumerate() {
            let look = camera
                .looks
                .looks
                .iter_mut()
                .find(|look| look.key == *key)
                .unwrap();
            look.position = index as u32 + 1;
        }
        Ok(())
    }
}

struct Fixture {
    root: tempfile::TempDir,
    camera: Arc<Mutex<Camera>>,
    api: LookApi,
    changes: Changes,
    pending: bool,
}
impl Fixture {
    /// A card with the files of `ids`, and a camera that lists `listed` in
    /// that order, each On and loaded.
    fn new(ids: &[&str], listed: &[&str]) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(looks::library(root.path())).unwrap();
        fs::create_dir_all(root.path().join(card::PACKAGE_FOLDER)).unwrap();
        for id in ids {
            fs::write(file(root.path(), id), testing::effect(id)).unwrap();
        }
        let camera = Arc::new(Mutex::new(Camera {
            looks: RuntimeLooks {
                looks: listed
                    .iter()
                    .map(|id| RuntimeLook {
                        key: look_key(id),
                        enabled: true,
                        position: 0,
                        loaded: true,
                    })
                    .collect(),
                ..Default::default()
            },
            ..Default::default()
        }));
        Self {
            root,
            api: LookApi::new(Box::new(Backend(Arc::clone(&camera)))),
            camera,
            changes: Changes::default(),
            pending: false,
        }
    }
    fn call(&self, method: &str, path: &str, body: &[u8], headers: &[(&str, &str)]) -> Reply {
        let (path, query) = match path.split_once('?') {
            Some((path, query)) => (path, Some(query.to_owned())),
            None => (path, None),
        };
        let mut request = Request {
            method: method.into(),
            path: path.into(),
            length: body.len(),
            query,
            headers: BTreeMap::from([("x-grkit-boot-id".to_owned(), BOOT.to_owned())]),
        };
        if !body.is_empty() {
            let kind = if path.ends_with("/content") {
                "application/octet-stream"
            } else {
                "application/json"
            };
            request.headers.insert("content-type".into(), kind.into());
        }
        for (name, value) in headers {
            request.headers.insert((*name).into(), (*value).into());
        }
        let keys = [testing::release_key()];
        let context = RequestContext {
            boot_id: BOOT,
            card_id: CARD,
            software_pending: self.pending,
            root: self.root.path(),
            release_keys: &keys,
        };
        self.api
            .handle(&request, body, &context, &self.changes)
            .expect("a Looks route")
    }
    fn list(&self) -> Value {
        let reply = self.call("GET", "/v1/looks", b"", &[]);
        assert_eq!(
            reply.status,
            200,
            "{}",
            String::from_utf8_lossy(&reply.body)
        );
        json(&reply)
    }
    fn look(&self, id: &str) -> Value {
        self.list()["looks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|look| look["id"] == id)
            .cloned()
            .unwrap_or(Value::Null)
    }
    fn change(&self, method: &str, path: &str, body: &[u8]) -> Reply {
        self.call(method, path, body, &[])
    }
    fn commands(&self) -> Vec<String> {
        std::mem::take(&mut self.camera.lock().unwrap().commands)
    }
    /// The service of another start over the same card.
    fn restart(self) -> Self {
        Self {
            api: LookApi::new(Box::new(Backend(Arc::clone(&self.camera)))),
            ..self
        }
    }
}
fn file(root: &Path, id: &str) -> std::path::PathBuf {
    looks::library(root).join(effects::filename(id).unwrap())
}
fn json(reply: &Reply) -> Value {
    serde_json::from_slice(&reply.body).unwrap()
}
fn code(reply: &Reply) -> String {
    json(reply)["code"].as_str().unwrap_or("").to_owned()
}
/// `id` with the name `name`.
fn named(id: &str, name: &str) -> Vec<u8> {
    let (mut effect, icon) = effects::decode(&testing::effect(id)).unwrap();
    effect.name = name.into();
    effect.encode(&icon).unwrap()
}
/// `id` with the name `name` and the icon of `cover`.
fn drawn(id: &str, name: &str, cover: &Cover) -> Vec<u8> {
    let (mut effect, _) = effects::decode(&testing::effect(id)).unwrap();
    effect.name = name.into();
    effect.encode(&cover.pixels().unwrap()).unwrap()
}
fn label(text: &str) -> Cover {
    Cover::from_json(&json!({"label": text, "shape": "plain"})).unwrap()
}
fn library_file(root: &Path) -> std::path::PathBuf {
    root.join(card::IMPORTS_FILE)
}
/// Write the import library: each effect with its cover, and whether the
/// effect is official.
fn write_library(root: &Path, entries: &[(&[u8], &Cover, bool)]) {
    let mut library = Vec::new();
    for (effect, cover, official) in entries {
        imports::upsert(&mut library, effect.to_vec(), cover, *official).unwrap();
    }
    fs::write(library_file(root), imports::encode(&library).unwrap()).unwrap();
}
fn read_library(root: &Path) -> Vec<Import> {
    imports::decode(&fs::read(library_file(root)).unwrap()).unwrap()
}
/// Sign `ids` as the official Looks of the release on the card, as a
/// statement before covers does.
fn sign_official(root: &Path, ids: &[&str]) {
    sign_with(root, |statement| {
        for id in ids {
            statement
                .official_looks
                .insert((*id).into(), packaging::sha256(&testing::effect(id)));
        }
    });
}
/// Sign the official Looks of the release on the card: the hash and the
/// body hash of each file, and its original cover.
fn sign(root: &Path, looks: &[(&str, &[u8], Option<&Cover>)]) {
    sign_with(root, |statement| {
        for (id, bytes, cover) in looks {
            let id = (*id).to_owned();
            statement
                .official_looks
                .insert(id.clone(), packaging::sha256(bytes));
            statement
                .official_look_bodies
                .insert(id.clone(), effects::body_hash(bytes).unwrap());
            if let Some(cover) = cover {
                statement.official_covers.insert(id, cover.to_json());
            }
        }
    });
}
fn sign_with(root: &Path, edit: impl FnOnce(&mut release::Statement)) {
    let mut statement = release::Statement::of_release(
        &testing::release_runtimes(&camera::GR3, "1.7.0"),
        &[],
        &camera::GR3,
    )
    .unwrap();
    edit(&mut statement);
    let bytes = statement.to_bytes().unwrap();
    let signature = release::SigningKey::from_hex(testing::RELEASE_SEED)
        .unwrap()
        .sign(&bytes);
    for (path, bytes) in [
        (release::CARD_STATEMENT, bytes),
        (release::CARD_SIGNATURE, signature.to_vec()),
    ] {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
}

#[test]
fn the_list_joins_the_files_of_the_card_with_the_choices_of_the_camera() {
    let fixture = Fixture::new(&["alpha", "beta", "gamma"], &["alpha", "beta"]);
    sign_official(fixture.root.path(), &["alpha"]);
    {
        let mut camera = fixture.camera.lock().unwrap();
        camera.looks.looks[1].position = 1;
        camera.looks.looks[0].enabled = false;
        camera.looks.looks[0].loaded = false;
        camera.looks.selected = Some(look_key("beta"));
        let code = u32::from(recipes::effect_code("beta"));
        camera.looks.uses = vec![
            LookUse {
                code,
                origin: UseOrigin::Recipe {
                    slot: 3,
                    number: 2,
                    name: Some("Street".into()),
                },
            },
            LookUse {
                code,
                origin: UseOrigin::Dial { mode: 1 },
            },
        ];
    }
    let list = fixture.list();
    let ids: Vec<_> = list["looks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|look| look["id"].as_str().unwrap())
        .collect();
    // The menu order of the camera, then the Look that waits for a start.
    assert_eq!(ids, ["beta", "alpha", "gamma"]);
    assert_eq!(list["bootId"], BOOT);
    assert_eq!(list["cardId"], CARD);
    assert_eq!(list["counts"], json!({"listed": 3, "loaded": 2}));
    assert_eq!(
        list["limits"],
        json!({"listedLooks": 64, "loadedLooks": 40})
    );
    let beta = &list["looks"][0];
    assert_eq!(beta["name"], "Test");
    assert_eq!(beta["position"], 0);
    assert_eq!(beta["enabled"], true);
    assert_eq!(
        beta["contentHash"],
        packaging::sha256(&testing::effect("beta"))
    );
    assert_eq!(
        beta["runtime"],
        json!({"id":"beta","source":"custom","loaded":true,"selected":true,"usedBy":2,
            "uses":[{"kind":"recipe","slot":3,"number":2,"name":"Street"},{"kind":"dial","mode":1}],"pending":null})
    );
    assert_eq!(
        list["looks"][1]["runtime"],
        json!({"id":"alpha","source":"official","loaded":false,"selected":false,"usedBy":0,"uses":[],"pending":null})
    );
    assert_eq!(list["looks"][1]["enabled"], false);
    assert_eq!(list["looks"][2]["runtime"]["pending"], "add");
    assert_eq!(list["looks"][2]["enabled"], true);

    let content = fixture.call("GET", "/v1/looks/beta/content", b"", &[]);
    assert_eq!(content.body, testing::effect("beta"));
    assert_eq!(
        json(&fixture.call("GET", "/v1/looks/beta/info", b"", &[])),
        beta["runtime"]
    );
    assert_eq!(
        json(&fixture.call("GET", "/v1/params/looks/order", b"", &[])),
        json!({"ids": ["beta", "alpha", "gamma"]})
    );
    assert_eq!(fixture.call("GET", "/v1/looks/none", b"", &[]).status, 404);
    assert_eq!(fixture.call("POST", "/v1/looks", b"", &[]).status, 405);
    assert!(fixture.commands().is_empty());
}

#[test]
fn files_that_the_camera_cannot_list_are_not_looks() {
    let fixture = Fixture::new(&["alpha", "beta"], &["alpha", "beta", "removed"]);
    let library = looks::library(fixture.root.path());
    // A second file with the ID of alpha, a file that the package leaves
    // out, and a file that is not an effect.
    fs::write(library.join("COPY.GRF"), testing::effect("alpha")).unwrap();
    fs::write(library.join("BAD.GRF"), vec![0; effects::SIZE]).unwrap();
    let list = fixture.root.path().join(grkit_core::card::PACKAGE_LIST);
    fs::create_dir_all(list.parent().unwrap()).unwrap();
    let beta = effects::filename("beta").unwrap();
    fs::write(&list, format!("-{beta}\n")).unwrap();
    let list = fixture.list();
    assert_eq!(list["looks"].as_array().unwrap().len(), 1);
    assert_eq!(list["looks"][0]["id"], "alpha");
    // Two files have the ID: the phone cannot tell which one the camera read.
    let reply = fixture.change("PATCH", "/v1/looks/alpha", br#"{"enabled":false}"#);
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (409, "ambiguous_look")
    );
}

#[test]
fn off_and_on_set_the_choice_in_the_camera_at_once() {
    let fixture = Fixture::new(&["alpha", "gamma"], &["alpha"]);
    fixture.camera.lock().unwrap().looks.selected = Some(look_key("alpha"));
    let key = look_key("alpha");
    let reply = fixture.change("PATCH", "/v1/looks/alpha", br#"{"enabled":false}"#);
    assert_eq!(
        reply.status,
        200,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    assert_eq!(json(&reply)["enabled"], false);
    assert_eq!(fixture.commands(), [format!("turn {key} true")]);
    // Off of the Look in use: the camera commits Off, as its checkbox does.
    let look = fixture.look("alpha");
    assert_eq!(look["enabled"], false);
    assert_eq!(look["runtime"]["selected"], false);
    assert_eq!(look["runtime"]["pending"], Value::Null);
    // The same value again sends no command.
    let reply = fixture.change("PATCH", "/v1/looks/alpha", br#"{"enabled":false}"#);
    assert_eq!(reply.status, 200);
    assert!(fixture.commands().is_empty());
    fixture.change("PATCH", "/v1/looks/alpha", br#"{"enabled":true}"#);
    assert_eq!(fixture.commands(), [format!("turn {key} false")]);

    // A camera that shows a page with the Looks changes nothing.
    fixture.camera.lock().unwrap().busy = true;
    let reply = fixture.change("PATCH", "/v1/looks/alpha", br#"{"enabled":false}"#);
    assert_eq!((reply.status, code(&reply).as_str()), (503, "camera_busy"));
    assert_eq!(fixture.look("alpha")["enabled"], true);
    fixture.camera.lock().unwrap().busy = false;

    // The camera does not list a new Look until it starts again.
    let reply = fixture.change("PATCH", "/v1/looks/gamma", br#"{"enabled":false}"#);
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (409, "restart_required")
    );

    for body in [
        &br#"{}"#[..],
        br#"{"enabled":1}"#,
        br#"{"other":true}"#,
        br#"[]"#,
    ] {
        let reply = fixture.change("PATCH", "/v1/looks/alpha", body);
        assert_eq!(
            (reply.status, code(&reply).as_str()),
            (422, "invalid_patch")
        );
    }
    assert!(fixture.commands().is_empty());
}

#[test]
fn at_most_forty_looks_are_on() {
    let ids: Vec<String> = (0..41).map(|i| format!("look-{i}")).collect();
    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    let fixture = Fixture::new(&ids, &ids);
    fixture.camera.lock().unwrap().looks.looks[0].enabled = false;
    let reply = fixture.change("PATCH", "/v1/looks/look-0", br#"{"enabled":true}"#);
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (409, "look_capacity")
    );
    assert!(fixture.commands().is_empty());
}

#[test]
fn a_new_name_or_new_content_waits_for_the_next_start() {
    let fixture = Fixture::new(&["alpha", "beta"], &["alpha", "beta"]);
    sign_official(fixture.root.path(), &["alpha"]);
    assert_eq!(fixture.look("beta")["runtime"]["pending"], Value::Null);

    let reply = fixture.change("PATCH", "/v1/looks/beta", br#"{"name":"New Name"}"#);
    assert_eq!(
        reply.status,
        200,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    assert_eq!(json(&reply)["name"], "New Name");
    let stored = fs::read(file(fixture.root.path(), "beta")).unwrap();
    assert_eq!(stored, named("beta", "New Name"));
    let look = fixture.look("beta");
    assert_eq!(look["runtime"]["pending"], "update");
    assert_eq!(look["contentHash"], packaging::sha256(&stored));
    // The Look keeps its choice in the camera while its file waits.
    let key = look_key("beta");
    fixture.change("PATCH", "/v1/looks/beta", br#"{"enabled":false}"#);
    assert_eq!(fixture.commands(), [format!("turn {key} true")]);
    // The first content again: nothing waits any more.
    let reply = fixture.change("PUT", "/v1/looks/beta/content", &testing::effect("beta"));
    assert_eq!(reply.status, 200);
    assert_eq!(fixture.look("beta")["runtime"]["pending"], Value::Null);

    for name in ["", "seventeen letters", "caf\u{e9}"] {
        let body = serde_json::to_vec(&json!({ "name": name })).unwrap();
        let reply = fixture.change("PATCH", "/v1/looks/beta", &body);
        assert_eq!((reply.status, code(&reply).as_str()), (422, "invalid_name"));
    }
    // An official Look of a statement without body hashes keeps its name
    // and its content.
    let reply = fixture.change("PATCH", "/v1/looks/alpha", br#"{"name":"Mine"}"#);
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (409, "official_look_read_only")
    );
    let reply = fixture.change("PUT", "/v1/looks/alpha/content", &named("alpha", "Mine"));
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (409, "official_look_read_only")
    );
    assert_eq!(
        fs::read(file(fixture.root.path(), "alpha")).unwrap(),
        testing::effect("alpha")
    );
}

#[test]
fn a_new_look_is_a_file_that_the_camera_lists_after_a_start() {
    let fixture = Fixture::new(&["alpha"], &["alpha"]);
    sign_official(fixture.root.path(), &["alpha", "official"]);
    let create = [];
    let bytes = testing::effect("fresh");
    let before = fixture.changes.counts();
    let reply = fixture.call("PUT", "/v1/looks/fresh/content", &bytes, &create);
    assert_eq!(
        reply.status,
        201,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    assert_eq!(json(&reply)["contentHash"], packaging::sha256(&bytes));
    assert_eq!(fs::read(file(fixture.root.path(), "fresh")).unwrap(), bytes);
    assert_ne!(
        fixture.changes.counts(),
        before,
        "the stream tells the phone"
    );
    let look = fixture.look("fresh");
    assert_eq!(look["runtime"]["pending"], "add");
    assert_eq!(look["runtime"]["source"], "custom");
    assert_eq!(look["position"], 1);
    assert!(fixture.commands().is_empty());
    // The same request again, as after an answer that did not arrive.
    let reply = fixture.call("PUT", "/v1/looks/fresh/content", &bytes, &create);
    assert_eq!(reply.status, 200);
    assert_eq!(fs::read(file(fixture.root.path(), "fresh")).unwrap(), bytes);

    // The official Look of the release, with its content only.
    let reply = fixture.call(
        "PUT",
        "/v1/looks/official/content",
        &named("official", "Fake"),
        &create,
    );
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (409, "official_look_read_only")
    );
    let reply = fixture.call(
        "PUT",
        "/v1/looks/official/content",
        &testing::effect("official"),
        &create,
    );
    assert_eq!(reply.status, 201);
    assert_eq!(fixture.look("official")["runtime"]["source"], "official");

    // The ID of the resource and a whole GRF.
    let reply = fixture.call("PUT", "/v1/looks/other/content", &bytes, &create);
    assert_eq!((reply.status, code(&reply).as_str()), (422, "id_mismatch"));
    let reply = fixture.call("PUT", "/v1/looks/fresh/content", &bytes[..100], &create);
    assert_eq!(reply.status, 413);
    assert!(!file(fixture.root.path(), "other").exists());
}

#[test]
fn a_look_that_the_package_leaves_out_gets_a_name_that_the_camera_lists() {
    let fixture = Fixture::new(&["alpha", "left"], &["alpha"]);
    let usual = effects::filename("left").unwrap();
    let list = fixture.root.path().join(grkit_core::card::PACKAGE_LIST);
    fs::create_dir_all(list.parent().unwrap()).unwrap();
    fs::write(&list, format!("-{usual}\n")).unwrap();
    // The camera does not list the file with the usual name.
    assert_eq!(fixture.look("left"), Value::Null);
    let bytes = named("left", "Again");
    let reply = fixture.call("PUT", "/v1/looks/left/content", &bytes, &[]);
    assert_eq!(
        reply.status,
        201,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    let added = format!("P{}", &usual[1..]);
    let library = looks::library(fixture.root.path());
    assert_eq!(fs::read(library.join(&added)).unwrap(), bytes);
    assert_eq!(
        fs::read(library.join(&usual)).unwrap(),
        testing::effect("left")
    );
    let look = fixture.look("left");
    assert_eq!(look["name"], "Again");
    assert_eq!(look["runtime"]["pending"], "add");
    // The phone removes the file that it added.
    let reply = fixture.change("DELETE", "/v1/looks/left", b"");
    assert_eq!(reply.status, 200);
    assert!(!library.join(&added).exists() && library.join(&usual).exists());
}

#[test]
fn a_new_look_needs_a_free_code_and_room() {
    // Two IDs with one runtime code.
    let mut seen = BTreeMap::new();
    let (first, second) = (0..=32768)
        .find_map(|i| {
            let id = format!("look-{i}");
            seen.insert(looks::base_code(&id), id.clone())
                .map(|first| (first, id))
        })
        .unwrap();
    let fixture = Fixture::new(&[&first], &[&first]);
    let create = [];
    let reply = fixture.call(
        "PUT",
        &format!("/v1/looks/{second}/content"),
        &testing::effect(&second),
        &create,
    );
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (409, "effect_code_collision")
    );

    let ids: Vec<String> = (0..64).map(|i| format!("full-{i}")).collect();
    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    let fixture = Fixture::new(&ids, &[]);
    let reply = fixture.call(
        "PUT",
        "/v1/looks/one-more/content",
        &testing::effect("one-more"),
        &create,
    );
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (409, "look_capacity")
    );
    assert!(!file(fixture.root.path(), "one-more").exists());
}

#[test]
fn a_removed_look_turns_off_at_once_and_its_file_goes() {
    let fixture = Fixture::new(&["alpha", "beta", "gamma"], &["alpha", "beta"]);
    let (alpha, beta) = (look_key("alpha"), look_key("beta"));
    {
        let mut camera = fixture.camera.lock().unwrap();
        camera.looks.selected = Some(alpha);
        camera.looks.uses = vec![LookUse {
            code: u32::from(recipes::effect_code("beta")),
            origin: UseOrigin::Unknown,
        }];
    }
    // A recipe or a mode-dial set names the Look.
    let reply = fixture.change("DELETE", "/v1/looks/beta", b"");
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (409, "dependency_in_use")
    );
    // The Look in use needs the camera to go to Off.
    let reply = fixture.change("DELETE", "/v1/looks/alpha", b"");
    assert_eq!((reply.status, code(&reply).as_str()), (409, "active_look"));
    // A busy camera keeps the file.
    fixture.camera.lock().unwrap().busy = true;
    let reply = fixture.change("DELETE", "/v1/looks/alpha?active=off", b"");
    assert_eq!((reply.status, code(&reply).as_str()), (503, "camera_busy"));
    assert!(file(fixture.root.path(), "alpha").exists());
    fixture.camera.lock().unwrap().busy = false;
    assert!(fixture.commands().is_empty());

    let reply = fixture.change("DELETE", "/v1/looks/alpha?active=off", b"");
    assert_eq!(
        reply.status,
        200,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    assert_eq!(json(&reply), json!({"id":"alpha","removed":true}));
    assert_eq!(fixture.commands(), [format!("turn {alpha} true")]);
    assert!(!file(fixture.root.path(), "alpha").exists());
    // The camera still lists it, Off, until the next start. The card does not.
    assert!(!fixture.camera.lock().unwrap().looks.looks[0].enabled);
    assert_eq!(fixture.camera.lock().unwrap().looks.selected, None);
    assert_eq!(fixture.look("alpha"), Value::Null);
    assert_eq!(fixture.list()["looks"].as_array().unwrap().len(), 2);

    // A Look that waits for a start has nothing in the camera to turn off.
    let reply = fixture.change("DELETE", "/v1/looks/gamma", b"");
    assert_eq!(reply.status, 200);
    assert!(!file(fixture.root.path(), "gamma").exists());
    assert!(fixture.commands().is_empty());
    assert_eq!(fixture.look("beta")["runtime"]["usedBy"], 1);
    let _ = beta;
}

#[test]
fn a_removed_look_that_comes_back_is_on_again() {
    let fixture = Fixture::new(&["alpha", "beta"], &["alpha", "beta"]);
    let alpha = look_key("alpha");
    assert_eq!(fixture.change("DELETE", "/v1/looks/alpha", b"").status, 200);
    let reply = fixture.change("PUT", "/v1/looks/alpha/content", &testing::effect("alpha"));
    assert_eq!(reply.status, 201);
    assert_eq!(
        fixture.commands(),
        [format!("turn {alpha} true"), format!("turn {alpha} false")]
    );
    assert_eq!(fixture.look("alpha")["enabled"], true);
}

#[test]
fn the_order_goes_to_the_camera_for_the_looks_that_it_lists() {
    let fixture = Fixture::new(&["alpha", "beta", "gamma"], &["alpha", "beta"]);
    let (alpha, beta) = (look_key("alpha"), look_key("beta"));
    let reply = fixture.change(
        "PUT",
        "/v1/params/looks/order",
        br#"{"ids":["gamma","beta","alpha"]}"#,
    );
    assert_eq!(
        reply.status,
        200,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    assert_eq!(fixture.commands(), [format!("order [{beta}, {alpha}]")]);
    // The Look that waits for a start stays after the listed Looks.
    assert_eq!(json(&reply), json!({"ids": ["beta", "alpha", "gamma"]}));
    for body in [
        &br#"{"ids":["beta","alpha"]}"#[..],
        br#"{"ids":["beta","alpha","alpha"]}"#,
        br#"{"ids":["beta","alpha","gamma","none"]}"#,
        br#"{"ids":["beta","alpha","gamma"],"more":1}"#,
        br#"{"ids":[1,2,3]}"#,
        br#"["beta","alpha","gamma"]"#,
    ] {
        let reply = fixture.change("PUT", "/v1/params/looks/order", body);
        assert_eq!(
            (reply.status, code(&reply).as_str()),
            (422, "invalid_order")
        );
    }
    assert!(fixture.commands().is_empty());
}

#[test]
fn a_change_names_the_start_that_the_phone_read() {
    let mut fixture = Fixture::new(&["alpha"], &["alpha"]);
    let body = br#"{"enabled":false}"#;
    let reply = fixture.call(
        "PATCH",
        "/v1/looks/alpha",
        body,
        &[("x-grkit-boot-id", "another")],
    );
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (409, "context_changed")
    );
    let reply = fixture.call(
        "PATCH",
        "/v1/looks/alpha",
        body,
        &[("content-type", "text/plain")],
    );
    assert_eq!(reply.status, 415);
    let reply = fixture.call("PATCH", "/v1/looks/alpha?force=1", body, &[]);
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (422, "invalid_query")
    );
    fixture.pending = true;
    let reply = fixture.call("PATCH", "/v1/looks/alpha", body, &[]);
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (409, "software_pending")
    );
    let before = fixture.changes.counts();
    assert!(fixture.commands().is_empty());
    assert_eq!(fixture.look("alpha")["enabled"], true);
    assert_eq!(
        fixture.changes.counts(),
        before,
        "a read and a refusal are no change"
    );
}

#[test]
fn each_read_gives_the_choices_of_the_camera_at_that_time() {
    let fixture = Fixture::new(&["alpha"], &["alpha"]);
    assert_eq!(fixture.list()["looks"][0]["enabled"], true);
    // A change in the camera menu.
    fixture.camera.lock().unwrap().looks.looks[0].enabled = false;
    assert_eq!(fixture.list()["looks"][0]["enabled"], false);
    // /props reports the Looks without a command to the camera.
    assert_eq!(fixture.api.runtime_properties(CARD)["state"], "ready");
    assert_eq!(
        fixture.api.runtime_properties("other")["state"],
        "unverified"
    );
}

#[test]
fn a_look_shows_the_cover_that_draws_its_icon() {
    let ids = ["alpha", "beta", "gamma", "delta"];
    let fixture = Fixture::new(&ids, &ids);
    let root = fixture.root.path();
    let (original, other) = (label("ORIG"), label("MINE"));
    let alpha = drawn("alpha", "Test", &original);
    let beta = drawn("beta", "Test", &other);
    sign(
        root,
        &[
            ("alpha", &alpha, Some(&original)),
            ("beta", &drawn("beta", "Test", &original), Some(&original)),
            ("delta", &testing::effect("delta"), None),
        ],
    );
    fs::write(file(root, "alpha"), &alpha).unwrap();
    // The official beta with another cover, and the official ID delta with
    // other colors.
    fs::write(file(root, "beta"), &beta).unwrap();
    let (mut delta, icon) = effects::decode(&testing::effect("delta")).unwrap();
    delta.matrix[0] += 1;
    fs::write(file(root, "delta"), delta.encode(&icon).unwrap()).unwrap();
    // The cover of gamma does not draw its icon.
    write_library(
        root,
        &[
            (&beta, &other, true),
            (&drawn("gamma", "Test", &other), &other, false),
        ],
    );
    for (id, source, cover) in [
        ("alpha", "official", original.to_json()),
        ("beta", "official", other.to_json()),
        ("gamma", "custom", Value::Null),
        ("delta", "unknown", Value::Null),
    ] {
        let look = fixture.look(id);
        assert_eq!(
            (look["runtime"]["source"].as_str(), &look["cover"]),
            (Some(source), &cover),
            "{id}"
        );
    }
    assert_eq!(
        json(&fixture.call("GET", "/v1/looks/beta", b"", &[]))["cover"],
        other.to_json()
    );
}

#[test]
fn a_cover_goes_to_the_import_library_after_the_file() {
    let fixture = Fixture::new(&["alpha"], &["alpha"]);
    let root = fixture.root.path().to_owned();
    let cover = label("NEW");
    let bytes = drawn("fresh", "Fresh", &cover);
    let header = cover.to_json().to_string();
    let put = |fixture: &Fixture, cover: &str| {
        fixture.call(
            "PUT",
            "/v1/looks/fresh/content",
            &bytes,
            &[("x-grkit-cover", cover)],
        )
    };
    for wrong in [label("OLD").to_json().to_string(), "{".into()] {
        let reply = put(&fixture, &wrong);
        assert_eq!(
            (reply.status, code(&reply).as_str()),
            (422, "cover_mismatch")
        );
    }
    let reply = put(&fixture, &" ".repeat(MAX_COVER + 1));
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (413, "cover_too_large")
    );
    assert!(!file(&root, "fresh").exists());

    // The library cannot be written: the Look has its file, without a cover.
    fs::create_dir(library_file(&root)).unwrap();
    let before = fixture.changes.counts();
    let reply = put(&fixture, &header);
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (507, "cover_not_saved")
    );
    assert_ne!(fixture.changes.counts(), before);
    assert_eq!(fs::read(file(&root, "fresh")).unwrap(), bytes);
    assert_eq!(fixture.look("fresh")["cover"], Value::Null);
    fs::remove_dir(library_file(&root)).unwrap();

    let reply = put(&fixture, &header);
    assert_eq!(
        reply.status,
        200,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    assert_eq!(json(&reply)["cover"], cover.to_json());
    let library = read_library(&root);
    let import = imports::find(&library, "fresh").unwrap();
    assert_eq!(
        (&import.effect, import.cover()),
        (&bytes, Some(cover.clone()))
    );
    // The service of the next start reads the cover from the card.
    let fixture = fixture.restart();
    assert_eq!(fixture.look("fresh")["cover"], cover.to_json());

    // A damaged library shows no cover and keeps its bytes.
    fs::write(library_file(&root), b"damaged").unwrap();
    let fixture = fixture.restart();
    assert_eq!(fixture.look("fresh")["cover"], Value::Null);
    let reply = put(&fixture, &header);
    assert_eq!(
        (reply.status, code(&reply).as_str()),
        (409, "cover_library_unavailable")
    );
    assert_eq!(fs::read(library_file(&root)).unwrap(), b"damaged");
}

#[test]
fn an_official_look_changes_its_name_and_cover_but_not_its_colors() {
    let fixture = Fixture::new(&["alpha"], &["alpha"]);
    let root = fixture.root.path();
    let original = label("ORIG");
    let alpha = drawn("alpha", "Test", &original);
    fs::write(file(root, "alpha"), &alpha).unwrap();
    sign(
        root,
        &[
            ("alpha", &alpha, Some(&original)),
            ("beta", &testing::effect("beta"), None),
        ],
    );
    // Each change goes to the import of the Look: a name and a cover that
    // differ from the official ones.
    let details = || -> Value {
        imports::find(&read_library(root), "alpha").map_or(Value::Null, |import| {
            serde_json::from_str(&import.details).unwrap()
        })
    };
    let reply = fixture.change("PATCH", "/v1/looks/alpha", br#"{"name":"Mine"}"#);
    assert_eq!(
        reply.status,
        200,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    let look = fixture.look("alpha");
    assert_eq!(
        (&look["name"], &look["runtime"]["source"]),
        (&json!("Mine"), &json!("official"))
    );
    assert_eq!(details(), json!({"kind": "cover", "name": "Mine"}));
    let cover = label("MINE");
    let header = cover.to_json().to_string();
    let with_cover = [("x-grkit-cover", header.as_str())];
    let bytes = drawn("alpha", "Test", &cover);
    let reply = fixture.call("PUT", "/v1/looks/alpha/content", &bytes, &with_cover);
    assert_eq!(reply.status, 200);
    let look = fixture.look("alpha");
    assert_eq!(look["runtime"]["source"], "official");
    assert_eq!(look["cover"], cover.to_json());
    assert_eq!(
        details(),
        json!({"kind": "cover", "cover": cover.to_json()})
    );

    // Other colors, or a new icon without its cover, change nothing.
    let (mut recolored, icon) = effects::decode(&bytes).unwrap();
    recolored.matrix[0] += 1;
    for (bytes, headers) in [
        (recolored.encode(&icon).unwrap(), &with_cover[..]),
        (bytes, &[][..]),
    ] {
        let reply = fixture.call("PUT", "/v1/looks/alpha/content", &bytes, headers);
        assert_eq!(
            (reply.status, code(&reply).as_str()),
            (409, "official_look_read_only")
        );
    }
    // The official file again: the import goes away.
    let reply = fixture.change("PUT", "/v1/looks/alpha/content", &alpha);
    assert_eq!(reply.status, 200);
    assert_eq!(details(), Value::Null);
    // An official Look that the card lacks comes back with another cover.
    let reply = fixture.call(
        "PUT",
        "/v1/looks/beta/content",
        &drawn("beta", "Test", &cover),
        &with_cover,
    );
    assert_eq!(reply.status, 201);
    assert_eq!(fixture.look("beta")["runtime"]["source"], "official");
}

#[test]
fn a_rename_redraws_the_default_cover_and_a_delete_drops_the_import() {
    let fixture = Fixture::new(&["beta", "gamma"], &["beta", "gamma"]);
    let root = fixture.root.path();
    let (default, custom) = (Cover::for_name("Test"), label("MINE"));
    let beta = drawn("beta", "Test", &default);
    let gamma = drawn("gamma", "Test", &custom);
    fs::write(file(root, "beta"), &beta).unwrap();
    fs::write(file(root, "gamma"), &gamma).unwrap();
    write_library(root, &[(&beta, &default, false), (&gamma, &custom, false)]);
    for id in ["beta", "gamma"] {
        let reply = fixture.change(
            "PATCH",
            &format!("/v1/looks/{id}"),
            br#"{"name":"Portra 400"}"#,
        );
        assert_eq!(
            reply.status,
            200,
            "{}",
            String::from_utf8_lossy(&reply.body)
        );
    }
    let renamed = Cover::for_name("Portra 400");
    let library = read_library(root);
    for (id, cover) in [("beta", &renamed), ("gamma", &custom)] {
        let stored = fs::read(file(root, id)).unwrap();
        assert_eq!(stored, drawn(id, "Portra 400", cover), "{id}");
        assert_eq!(fixture.look(id)["cover"], cover.to_json());
        let import = imports::find(&library, id).unwrap();
        assert_eq!(
            (&import.effect, import.cover().as_ref()),
            (&stored, Some(cover))
        );
    }

    let reply = fixture.change("DELETE", "/v1/looks/gamma", b"");
    assert_eq!(reply.status, 200);
    let library = read_library(root);
    assert!(imports::find(&library, "gamma").is_none());
    assert!(imports::find(&library, "beta").is_some());
}

#[test]
fn the_film_box_cover_of_an_official_look_is_its_scheme_and_not_an_import() {
    let fixture = Fixture::new(&["alpha", "beta"], &["alpha", "beta"]);
    let root = fixture.root.path();
    let statement = Cover::for_name("Kodak Portra 400");
    let film_box = statement.film_box();
    // The B&W Look mono.
    let mono = |cover: &Cover| {
        let (mut effect, _) = effects::decode(&drawn("mono", "Test", cover)).unwrap();
        effect.flags = effects::MONOCHROME;
        effect.matrix = [2731, 2731, 2730].repeat(3).try_into().unwrap();
        effect.encode(&cover.pixels().unwrap()).unwrap()
    };
    let gray = [
        drawn("alpha", "Test", &statement),
        drawn("beta", "Test", &statement),
        mono(&statement),
    ];
    sign(
        root,
        &[
            ("alpha", &gray[0], Some(&statement)),
            ("beta", &gray[1], Some(&statement)),
            ("mono", &gray[2], Some(&statement)),
        ],
    );
    fs::write(file(root, "alpha"), &gray[0]).unwrap();
    fs::write(file(root, "beta"), &gray[1]).unwrap();
    assert_eq!(fixture.list()["officialCovers"], "gray");
    assert_eq!(fixture.look("alpha")["defaultCover"], statement.to_json());

    // The film box cover shows, and the import library does not record it.
    let header = film_box.to_json().to_string();
    let reply = fixture.call(
        "PUT",
        "/v1/looks/alpha/content",
        &drawn("alpha", "Test", &film_box),
        &[("x-grkit-cover", header.as_str())],
    );
    assert_eq!(reply.status, 200);
    assert_eq!(fixture.look("alpha")["cover"], film_box.to_json());
    assert!(!library_file(root).exists());
    // Gray beta: a mix of the color Looks is gray.
    assert_eq!(fixture.list()["officialCovers"], "gray");
    // beta with the film box icon of another build of the drawing: each
    // color Look then has the film box cover. No cover draws its icon.
    let (effect, _) = effects::decode(&gray[1]).unwrap();
    let mut icon = film_box.pixels().unwrap();
    // The color bytes of 100 pixels differ; alpha stays.
    for (at, byte) in icon.iter_mut().enumerate().take(400) {
        if at % 4 != 3 {
            *byte ^= 1;
        }
    }
    fs::write(file(root, "beta"), effect.encode(&icon).unwrap()).unwrap();
    assert_eq!(fixture.list()["officialCovers"], "colorFilmBox");
    assert_eq!(fixture.look("alpha")["defaultCover"], film_box.to_json());
    assert_eq!(fixture.look("beta")["cover"], Value::Null);
    // An official Look that the card lacks comes in its film box file: each
    // Look then has the film box cover.
    let reply = fixture.change("PUT", "/v1/looks/mono/content", &mono(&film_box));
    assert_eq!(reply.status, 201);
    let added = fixture.look("mono");
    assert_eq!(
        (&added["runtime"]["source"], &added["cover"]),
        (&json!("official"), &film_box.to_json())
    );
    assert_eq!(fixture.list()["officialCovers"], "filmBox");
}

const CINEMA: &str = "gr4-cinema-yellow";

/// A B&W Look `id` with the icon of `cover`.
fn mono_drawn(id: &str, cover: &Cover) -> Vec<u8> {
    let (mut effect, _) = effects::decode(&testing::effect(id)).unwrap();
    effect.flags = effects::MONOCHROME;
    effect.matrix = [2731, 2731, 2730].repeat(3).try_into().unwrap();
    effect.encode(&cover.pixels().unwrap()).unwrap()
}

/// A card with the color Looks alpha and beta and the B&W Look mono, signed
/// with the statement cover `statement` and in its gray files, and the
/// native look Cinema Yellow with its gray cover and the import of the site.
/// beta and mono have the individual cover `own`.
fn built_in_card(statement: &Cover, own: &Cover) -> Fixture {
    let ids = ["alpha", "beta", "mono", CINEMA];
    let fixture = Fixture::new(&ids, &ids);
    let root = fixture.root.path();
    let gray = [
        drawn("alpha", "Test", statement),
        drawn("beta", "Test", statement),
        mono_drawn("mono", statement),
    ];
    sign(
        root,
        &[
            ("alpha", &gray[0], Some(statement)),
            ("beta", &gray[1], Some(statement)),
            ("mono", &gray[2], Some(statement)),
        ],
    );
    let (beta, mono) = (drawn("beta", "Beta", own), mono_drawn("mono", own));
    let native = grkit_core::cover::native_cover(CINEMA).unwrap();
    let cinema = drawn(CINEMA, "Cinema Yellow", &native);
    for (id, bytes) in [
        ("alpha", &gray[0]),
        ("beta", &beta),
        ("mono", &mono),
        (CINEMA, &cinema),
    ] {
        fs::write(file(root, id), bytes).unwrap();
    }
    let mut library = Vec::new();
    for effect in [&beta, &mono] {
        imports::record_official(
            &mut library,
            effect.clone(),
            Some(own),
            Some(statement),
            false,
        )
        .unwrap();
    }
    library.push(Import {
        details: json!({"kind": "native", "version": "1.20", "cover": native.film_box().to_json()})
            .to_string(),
        effect: cinema,
    });
    fs::write(library_file(root), imports::encode(&library).unwrap()).unwrap();
    fixture
}

fn covers(fixture: &Fixture) -> Vec<(String, Value)> {
    fixture.list()["looks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|look| {
            (
                look["id"].as_str().unwrap().to_owned(),
                look["cover"].clone(),
            )
        })
        .collect()
}

#[test]
fn a_scheme_rewrites_the_built_in_covers_without_an_individual_cover() {
    let (statement, own) = (Cover::for_name("Kodak Portra 400"), label("MINE"));
    let fixture = built_in_card(&statement, &own);
    let root = fixture.root.path();
    let native = grkit_core::cover::native_cover(CINEMA).unwrap();
    let cinema = fixture.look(CINEMA);
    assert_eq!(cinema["runtime"]["source"], "native");
    assert_eq!(cinema["defaultCover"], native.to_json());
    assert_eq!(fixture.list()["officialCovers"], "gray");

    let reply = fixture.change("PUT", "/v1/looks/covers/scheme", br#"{"scheme":"filmBox"}"#);
    assert_eq!(
        reply.status,
        200,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    assert_eq!(
        json(&reply),
        json!({"officialCovers": "filmBox", "changed": ["alpha", CINEMA]})
    );
    let film_box = statement.film_box().to_json();
    let mine = own.to_json();
    let expected = |cinema: &Cover| {
        vec![
            ("alpha".to_owned(), film_box.clone()),
            ("beta".to_owned(), mine.clone()),
            ("mono".to_owned(), mine.clone()),
            (CINEMA.to_owned(), cinema.to_json()),
        ]
    };
    assert_eq!(covers(&fixture), expected(&native.film_box()));
    // Names stay; the imports keep the individual covers, and the native
    // import takes its new file.
    assert_eq!(fixture.look("beta")["name"], "Beta");
    assert_eq!(fixture.look("alpha")["runtime"]["pending"], "update");
    let library = read_library(root);
    assert_eq!(library.len(), 3);
    assert!(imports::find(&library, "alpha").is_none());
    assert_eq!(
        imports::find(&library, CINEMA).unwrap().effect,
        fs::read(file(root, CINEMA)).unwrap()
    );
    // Each B&W Look has an individual cover: the B&W group follows the
    // scheme that the phone chose in this start.
    assert_eq!(fixture.list()["officialCovers"], "filmBox");
    let reply = fixture.change("PUT", "/v1/looks/covers/scheme", br#"{"scheme":"gray"}"#);
    assert_eq!(json(&reply)["changed"], json!(["alpha", CINEMA]));
    assert_eq!(covers(&fixture)[3].1, native.to_json());
    assert_eq!(fixture.list()["officialCovers"], "gray");
    for body in [&br#"{"scheme":"blue"}"#[..], br#"{"group":"color"}"#] {
        let reply = fixture.change("PUT", "/v1/looks/covers/scheme", body);
        assert_eq!((reply.status, code(&reply)), (422, "invalid_covers".into()));
    }
}

#[test]
fn a_reset_puts_the_individual_covers_of_a_group_back_to_the_scheme() {
    let (statement, own) = (Cover::for_name("Kodak Portra 400"), label("MINE"));
    let fixture = built_in_card(&statement, &own);
    let reply = fixture.change(
        "PUT",
        "/v1/looks/covers/scheme",
        br#"{"scheme":"colorFilmBox"}"#,
    );
    assert_eq!(json(&reply)["changed"], json!(["alpha", CINEMA]));

    let reply = fixture.change("POST", "/v1/looks/covers/reset", br#"{"group":"color"}"#);
    assert_eq!(
        json(&reply),
        json!({"officialCovers": "colorFilmBox", "changed": ["beta"]})
    );
    let look = fixture.look("beta");
    assert_eq!(
        (&look["name"], &look["cover"]),
        (&json!("Beta"), &statement.film_box().to_json())
    );
    assert_eq!(fixture.look("mono")["cover"], own.to_json());
    // The import of beta keeps its name only.
    let library = read_library(fixture.root.path());
    let details: Value =
        serde_json::from_str(&imports::find(&library, "beta").unwrap().details).unwrap();
    assert_eq!(details, json!({"kind": "cover", "name": "Beta"}));

    let reply = fixture.change("POST", "/v1/looks/covers/reset", br#"{"group":"mono"}"#);
    assert_eq!(json(&reply)["changed"], json!(["mono"]));
    assert_eq!(fixture.look("mono")["cover"], statement.to_json());
    assert_eq!(fixture.list()["officialCovers"], "colorFilmBox");
}

#[test]
fn a_native_look_counts_for_the_color_scheme_and_changes_its_name_and_cover() {
    let (statement, own) = (Cover::for_name("Kodak Portra 400"), label("MINE"));
    let fixture = built_in_card(&statement, &own);
    let root = fixture.root.path();
    // Each official color Look without an individual cover has the film box
    // cover, but the gray Cinema Yellow counts too.
    fs::write(
        file(root, "alpha"),
        drawn("alpha", "Test", &statement.film_box()),
    )
    .unwrap();
    assert_eq!(fixture.list()["officialCovers"], "gray");
    // Its film box cover is no individual cover.
    let film_box = grkit_core::cover::native_cover(CINEMA).unwrap().film_box();
    let header = film_box.to_json().to_string();
    let reply = fixture.call(
        "PUT",
        &format!("/v1/looks/{CINEMA}/content"),
        &drawn(CINEMA, "Cinema Yellow", &film_box),
        &[("x-grkit-cover", header.as_str())],
    );
    assert_eq!(reply.status, 200);
    assert_eq!(fixture.list()["officialCovers"], "colorFilmBox");
    let reply = fixture.change(
        "PATCH",
        &format!("/v1/looks/{CINEMA}"),
        br#"{"name":"Yellow"}"#,
    );
    assert_eq!(reply.status, 200);
    let look = fixture.look(CINEMA);
    assert_eq!(
        (&look["name"], &look["cover"]),
        (&json!("Yellow"), &film_box.to_json())
    );
    let library = read_library(root);
    assert_eq!(
        imports::find(&library, CINEMA).unwrap().effect,
        fs::read(file(root, CINEMA)).unwrap()
    );
}

#[test]
fn a_request_that_panicked_leaves_the_state_usable() {
    let state = Mutex::new(7);
    let _ = std::panic::catch_unwind(|| {
        let _guard = state.lock().unwrap();
        panic!("request");
    });
    assert_eq!(*try_lock(&state).unwrap(), 7);
    assert!(!state.is_poisoned());
}
