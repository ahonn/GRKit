//! Looks pages with the real store and HTTP transport, against a camera that
//! the test controls. Each mutation must name the camera start that the
//! phone read, and nothing else.
use std::{
    env,
    ffi::OsString,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use grkit_app::App;
use grkit_core::{cover::Cover, effects};
use nsh_core::geometry::Point;
use nsh_core::platform::{HttpBody, HttpRequest, Os};
use nsh_core::prelude::Points;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::{DialogAnswer, Headless, StubNetwork, StubResponse, StubRoute};
use serde_json::{Value, json};

const WAIT: Duration = Duration::from_secs(10);
static ENV_LOCK: Mutex<()> = Mutex::new(());

const PORTRA: &str = "portra-400-endura-v1";
const TRIX: &str = "trix-400-mono-v1";
const CUSTOM: &str = "lut-night";

struct TestEnvironment {
    previous: Vec<(&'static str, Option<OsString>)>,
    directory: PathBuf,
}

impl TestEnvironment {
    fn new() -> Self {
        let changes = [
            ("GRKIT_DEMO", None),
            ("GRKIT_DEMO_SCREEN", None),
            ("GRKIT_DEMO_FAIL", None),
            ("GRKIT_CAMERA_URL", Some("http://camera.test")),
            ("GRKIT_RELEASE_BASE", Some("https://release.test")),
        ];
        let previous = changes
            .iter()
            .map(|(key, _)| (*key, env::var_os(key)))
            .collect();
        for (key, value) in changes {
            unsafe {
                match value {
                    Some(value) => env::set_var(key, value),
                    None => env::remove_var(key),
                }
            }
        }
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            env::temp_dir().join(format!("grkit-looks-test-{}-{unique}", std::process::id()));
        Self {
            previous,
            directory,
        }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            unsafe {
                match value {
                    Some(value) => env::set_var(key, value),
                    None => env::remove_var(key),
                }
            }
        }
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[derive(Clone)]
struct Entry {
    id: &'static str,
    name: String,
    enabled: bool,
    source: &'static str,
    used_by: usize,
    /// The file change that waits for the next start of the camera.
    pending: Option<&'static str>,
    revision: u32,
    cover: Value,
    default_cover: Value,
}

struct Camera {
    looks: Vec<Entry>,
    selected: Option<&'static str>,
    /// Answer the next mutation with this status and code.
    refuse: Option<(u16, &'static str)>,
    order_revision: u32,
    /// The color scheme of the covers of the official Looks.
    official_covers: &'static str,
    /// How long the camera takes to answer a change.
    write_delay: Duration,
    /// The person's text size.
    font_scale: f32,
}

fn definition(id: &str) -> Value {
    let definition = match id {
        PORTRA => include_str!("../../effects/portra-400-endura-v1/effect.json"),
        _ => include_str!("../../effects/trix-400-mono-v1/effect.json"),
    };
    grkit_core::effects::parse_json(definition.as_bytes()).unwrap()
}

fn grf(id: &str) -> Vec<u8> {
    grkit_core::effects::compile_definition(&definition(id)).unwrap()
}

fn hash(id: &str, revision: u32) -> String {
    grkit_core::packaging::sha256(format!("{id}:{revision}").as_bytes())
}

impl Camera {
    fn new() -> Self {
        let entry = |id, name: &str, source, used_by| Entry {
            id,
            name: name.into(),
            enabled: true,
            source,
            used_by,
            pending: None,
            revision: 1,
            cover: Value::Null,
            default_cover: Value::Null,
        };
        let mut trix = entry(TRIX, "Tri-X 400", "official", 0);
        trix.cover = definition(TRIX)["icon"].clone();
        Self {
            looks: vec![
                entry(PORTRA, "Portra 400", "official", 1),
                trix,
                entry(CUSTOM, "Night", "custom", 0),
            ],
            selected: Some(PORTRA),
            refuse: None,
            order_revision: 1,
            official_covers: "gray",
            write_delay: Duration::ZERO,
            font_scale: 1.0,
        }
    }

    fn list(&self) -> Value {
        let looks: Vec<Value> = self
            .looks
            .iter()
            .enumerate()
            .map(|(position, e)| {
                json!({
                    "id": e.id, "name": e.name, "contentHash": hash(e.id, e.revision),
                    "enabled": e.enabled, "position": position,
                    "runtime": {
                        "id": e.id, "source": e.source, "loaded": e.enabled,
                        "selected": self.selected == Some(e.id),
                        "usedBy": e.used_by, "pending": e.pending
                    },
                    "cover": e.cover,
                    "defaultCover": e.default_cover
                })
            })
            .collect();
        let loaded = self.looks.iter().filter(|e| e.enabled).count();
        json!({
            "looks": looks,
            "counts": {"listed": self.looks.len(), "loaded": loaded},
            "limits": {"listedLooks": 64, "loadedLooks": 40},
            "officialCovers": self.official_covers,
            "bootId": "boot-test", "cardId": "card-test"
        })
    }
}

fn header<'a>(request: &'a HttpRequest, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

fn body(request: &HttpRequest) -> &[u8] {
    match &request.body {
        HttpBody::Bytes(bytes) => bytes,
        HttpBody::Empty => &[],
        HttpBody::Stream => panic!("unexpected streamed body"),
    }
}

fn response(status: u16, value: Value) -> StubResponse {
    let mut response = StubResponse::default();
    response.status = status;
    response.body = serde_json::to_vec(&value).unwrap();
    response
}

/// Check the context of a mutation and refuse or accept it.
fn admit(camera: &mut Camera, request: &HttpRequest) -> Result<(), StubResponse> {
    assert_eq!(header(request, "X-GRKit-Boot-Id"), Some("boot-test"));
    for removed in [
        "X-GRKit-Request-Id",
        "X-GRKit-Card-Id",
        "X-GRKit-Content-SHA256",
        "If-Match",
        "If-None-Match",
    ] {
        assert_eq!(header(request, removed), None, "{removed} is not sent");
    }
    if let Some((status, code)) = camera.refuse.take() {
        return Err(response(
            status,
            json!({"status": status, "code": code, "detail": "Refused."}),
        ));
    }
    Ok(())
}

/// The camera answers a change when the change ends.
fn finish() -> StubResponse {
    response(200, json!({}))
}

fn start_phone(
    environment: &TestEnvironment,
    camera: Camera,
) -> (Headless, Arc<Mutex<Camera>>, Arc<StubNetwork>) {
    start_phone_with(environment, camera, Vec::new())
}

/// `start_phone` with `site` routes before the release site, which answers
/// 404.
fn start_phone_with(
    environment: &TestEnvironment,
    camera: Camera,
    site: Vec<(String, StubRoute)>,
) -> (Headless, Arc<Mutex<Camera>>, Arc<StubNetwork>) {
    let font_scale = camera.font_scale;
    let camera = Arc::new(Mutex::new(camera));
    let shared = camera.clone();
    let handler = StubRoute::Handler(Arc::new(move |request, reply| {
        let mut camera = shared.lock().unwrap();
        let url = request
            .url
            .strip_prefix("http://camera.test")
            .unwrap()
            .to_owned();
        let (path, query) = url
            .split_once('?')
            .map_or((url.as_str(), None), |(p, q)| (p, Some(q)));
        let parts: Vec<&str> = path.split('/').collect();
        let write_delay = camera.write_delay;
        let is_write = request.method != "GET";
        let mut answer = match (request.method.as_str(), parts.as_slice()) {
            ("GET", ["", "v1", "ping"]) => {
                response(200, json!({"protocol": 1, "bootId": "boot-test"}))
            }
            ("GET", ["", "v1", "props"]) => response(
                200,
                json!({
                    "bootId": "boot-test", "cardId": "card-test",
                    "camera": {"model": "RICOH GR III", "firmwareVersion": "2.21"},
                    "grkit": {"version": "1.7.0"},
                    "storage": {"state": "ready"}
                }),
            ),
            ("GET", ["", "v1", "update"]) => response(
                200,
                json!({"inventory": {"package": {"version": "1.7.0", "modules": [], "effects": [], "left_out": []},
                       "state": "none"}, "request": null}),
            ),
            ("GET", ["", "v1", "looks"]) => response(200, camera.list()),
            // The camera tab reads the recipes at connect; these tests have none.
            ("GET", ["", "v1", "recipes"]) => response(
                200,
                json!({"recipes": [], "limits": {"on": 40, "listed": 64}, "bootId": "boot-test"}),
            ),
            // Disconnect asks the camera to close its access point.
            ("POST", ["", "v1", "device", "wlan", "finish"]) => finish(),
            ("POST", ["", "v1", "device", "finish"]) => finish(),
            ("GET", ["", "v1", "looks", id, "content"]) => {
                let mut r = StubResponse::default();
                r.status = 200;
                r.body = grf(id);
                r
            }
            ("PUT", ["", "v1", "looks", "covers", "scheme"]) => {
                match admit(&mut camera, &request) {
                    Err(refused) => refused,
                    Ok(()) => {
                        let value: Value = serde_json::from_slice(body(&request)).unwrap();
                        let scheme: &'static str = Box::leak(
                            value["scheme"]
                                .as_str()
                                .unwrap()
                                .to_owned()
                                .into_boxed_str(),
                        );
                        camera.official_covers = scheme;
                        response(200, json!({"officialCovers": scheme, "changed": []}))
                    }
                }
            }
            ("POST", ["", "v1", "looks", "covers", "reset"]) => {
                match admit(&mut camera, &request) {
                    Err(refused) => refused,
                    Ok(()) => {
                        for entry in camera.looks.iter_mut() {
                            entry.cover = entry.default_cover.clone();
                        }
                        response(
                            200,
                            json!({"officialCovers": camera.official_covers, "changed": []}),
                        )
                    }
                }
            }
            ("PUT", ["", "v1", "looks", id, "content"]) => match admit(&mut camera, &request) {
                Err(refused) => refused,
                Ok(()) => {
                    assert_eq!(
                        header(&request, "Content-Type"),
                        Some("application/octet-stream")
                    );
                    assert!(matches!(body(&request).len(), 8192 | 8704));
                    let id: &'static str = Box::leak(id.to_string().into_boxed_str());
                    let cover = header(&request, "X-GRKit-Cover")
                        .map_or(Value::Null, |cover| serde_json::from_str(cover).unwrap());
                    // Create or replace: the change waits for the next start.
                    let created = !camera.looks.iter().any(|e| e.id == id);
                    if created {
                        camera.looks.push(Entry {
                            id,
                            name: "Copy".into(),
                            enabled: true,
                            source: "custom",
                            used_by: 0,
                            pending: Some("add"),
                            revision: 1,
                            cover,
                            default_cover: Value::Null,
                        });
                    } else {
                        let entry = camera.looks.iter_mut().find(|e| e.id == id).unwrap();
                        entry.pending.get_or_insert("update");
                        entry.revision += 1;
                        entry.cover = cover;
                    }
                    response(if created { 201 } else { 200 }, json!({"id": id}))
                }
            },
            ("PATCH", ["", "v1", "looks", id]) => match admit(&mut camera, &request) {
                Err(refused) => refused,
                Ok(()) => {
                    assert_eq!(header(&request, "Content-Type"), Some("application/json"));
                    let patch: Value = serde_json::from_slice(body(&request)).unwrap();
                    let entry = camera.looks.iter_mut().find(|e| e.id == *id).unwrap();
                    if let Some(enabled) = patch["enabled"].as_bool() {
                        entry.enabled = enabled;
                    }
                    if let Some(name) = patch["name"].as_str() {
                        entry.name = name.into();
                        entry.revision += 1;
                        entry.pending = Some("update");
                    }
                    finish()
                }
            },
            ("DELETE", ["", "v1", "looks", id]) => match admit(&mut camera, &request) {
                Err(refused) => refused,
                Ok(()) => {
                    let selected = camera.selected == Some(*id);
                    if selected && query != Some("active=off") {
                        response(409, json!({"status": 409, "code": "active_look"}))
                    } else {
                        camera.looks.retain(|e| e.id != *id);
                        if selected {
                            camera.selected = None;
                        }
                        finish()
                    }
                }
            },
            ("PUT", ["", "v1", "params", "looks", "order"]) => match admit(&mut camera, &request) {
                Err(refused) => refused,
                Ok(()) => {
                    let order: Value = serde_json::from_slice(body(&request)).unwrap();
                    let ids: Vec<String> = serde_json::from_value(order["ids"].clone()).unwrap();
                    let mut looks = Vec::new();
                    for id in ids {
                        let at = camera.looks.iter().position(|e| e.id == id).unwrap();
                        looks.push(camera.looks.remove(at));
                    }
                    assert!(camera.looks.is_empty(), "order must list every Look");
                    camera.looks = looks;
                    camera.order_revision += 1;
                    finish()
                }
            },
            _ => panic!(
                "unexpected camera request: {} {}",
                request.method, request.url
            ),
        };
        if is_write {
            answer.delay = write_delay;
        }
        reply.send(answer);
    }));
    let mut missing = StubResponse::default();
    missing.status = 404;
    let mut routes: Vec<(String, StubRoute)> = vec![
        ("GET http://camera.test/*".into(), handler.clone()),
        ("PUT http://camera.test/*".into(), handler.clone()),
        ("PATCH http://camera.test/*".into(), handler.clone()),
        ("DELETE http://camera.test/*".into(), handler.clone()),
        ("POST http://camera.test/*".into(), handler),
    ];
    routes.extend(site);
    routes.push((
        "GET https://release.test/*".into(),
        StubRoute::Response(missing),
    ));
    let network = Arc::new(StubNetwork::new(routes));
    let mut config = EngineConfig::new(Root::of::<App>());
    config.environment.os = Os::Web;
    config.environment.font_scale = font_scale;
    config.data_dir = Some(environment.directory.clone());
    config.network = Some(network.clone());
    let app = Headless::start(config).unwrap();
    (app, camera, network)
}

fn open_looks(app: &mut Headless) {
    // The camera card shows the versions once connected.
    app.wait_for_within("Firmware", WAIT).unwrap();
    app.press("role:button[Film Looks]").unwrap();
    app.wait_for_within("In Camera Menu", WAIT).unwrap();
}

/// Open Cover Colors from the Looks list.
fn open_cover_colors(app: &mut Headless) {
    app.wait_for_within("Cover Colors", WAIT).unwrap();
    app.press("text:Cover Colors").unwrap();
    app.wait_for_within("Every built-in film look has a gray cover", WAIT)
        .unwrap();
}

/// Let the app run until `done` holds, at most `WAIT`.
fn until(app: &mut Headless, mut done: impl FnMut() -> bool) {
    for _ in 0..100 {
        if done() {
            return;
        }
        app.wait(Duration::from_millis(100)).unwrap();
    }
    panic!("condition not reached within {WAIT:?}");
}

/// The cover header and the GRF body of the content PUTs of `id`.
fn content_puts(network: &StubNetwork, id: &str) -> Vec<(Option<Cover>, Vec<u8>)> {
    network
        .requests()
        .iter()
        .filter(|r| r.method == "PUT" && r.url.ends_with(&format!("/v1/looks/{id}/content")))
        .map(|r| {
            let cover = header(r, "X-GRKit-Cover")
                .map(|cover| Cover::from_json(&serde_json::from_str(cover).unwrap()).unwrap());
            (cover, body(r).to_vec())
        })
        .collect()
}

fn count(network: &StubNetwork, method: &str, suffix: &str) -> usize {
    network
        .requests()
        .iter()
        .filter(|r| r.method == method && r.url.ends_with(suffix))
        .count()
}

#[test]
fn turning_a_look_off_patches_it_at_once() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let (mut app, camera, network) = start_phone(&environment, Camera::new());
    open_looks(&mut app);
    app.press("text:Tri-X 400").unwrap();
    app.wait_for_within("Enabled", WAIT).unwrap();
    app.press("role:switch[Enabled]").unwrap();
    until(&mut app, || !camera.lock().unwrap().looks[1].enabled);
    app.wait_for_within("role:switch[Enabled]", WAIT).unwrap();
    assert_eq!(count(&network, "PATCH", "/v1/looks/trix-400-mono-v1"), 1);
    // A change needs no read of its own before it.
    assert!(
        !network
            .requests()
            .iter()
            .any(|r| r.method == "GET" && r.url.ends_with("/v1/looks/trix-400-mono-v1"))
    );
}

#[test]
fn a_disconnected_camera_keeps_both_lists_to_read() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let mut state = Camera::new();
    state.looks[1].enabled = false;
    let (mut app, _camera, _network) = start_phone(&environment, state);
    open_looks(&mut app);
    app.wait_for_within("Not in Menu", WAIT).unwrap();
    app.press("role:button[Camera]").unwrap();
    app.on_dialog(|_| DialogAnswer::Press("Disconnect".into()));
    app.press("label:More").unwrap();
    app.wait_for_gone("Firmware", WAIT).unwrap();
    app.press("role:button[Film Looks]").unwrap();
    app.wait_for_within("Camera Not Connected", WAIT).unwrap();
    app.wait_for_within("Not in Menu", WAIT).unwrap();
    app.wait_for_within("text:Tri-X 400", WAIT).unwrap();
}

#[test]
fn a_restarted_camera_asks_to_check_the_looks_again() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let mut state = Camera::new();
    state.refuse = Some((409, "context_changed"));
    let (mut app, camera, network) = start_phone(&environment, state);
    open_looks(&mut app);
    app.press("text:Tri-X 400").unwrap();
    app.wait_for_within("Enabled", WAIT).unwrap();
    let reads = count(&network, "GET", "/v1/looks");
    let titles = Arc::new(Mutex::new(Vec::new()));
    let seen = titles.clone();
    app.on_dialog(move |dialog| {
        seen.lock()
            .unwrap()
            .push(dialog.title.clone().unwrap_or_default());
        DialogAnswer::Press("OK".into())
    });
    app.press("role:switch[Enabled]").unwrap();
    until(&mut app, || !titles.lock().unwrap().is_empty());
    assert_eq!(titles.lock().unwrap().as_slice(), ["Film Looks Changed"]);
    // The phone reads the Looks again, so a second try names the new start.
    until(&mut app, || count(&network, "GET", "/v1/looks") > reads);
    assert!(camera.lock().unwrap().looks[1].enabled);
}

#[test]
fn an_official_look_is_renamed_like_an_imported_one() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let (mut app, camera, network) = start_phone(&environment, Camera::new());
    open_looks(&mut app);
    app.press("text:Tri-X 400").unwrap();
    app.wait_for_within("role:button[Name, Tri-X 400]", WAIT)
        .unwrap();
    unsafe { std::env::set_var("GRKIT_TEST_NAME", "Tri-X 400 Push") };
    app.press("role:button[Name, Tri-X 400]").unwrap();
    until(&mut app, || {
        camera.lock().unwrap().looks[1].name == "Tri-X 400 Push"
    });
    unsafe { std::env::remove_var("GRKIT_TEST_NAME") };
    let patches: Vec<Value> = network
        .requests()
        .iter()
        .filter(|r| r.method == "PATCH" && r.url.ends_with(&format!("/v1/looks/{TRIX}")))
        .map(|r| serde_json::from_slice(body(r)).unwrap())
        .collect();
    assert_eq!(patches, [json!({"name": "Tri-X 400 Push"})]);
}

#[test]
fn an_official_look_changes_its_cover_and_only_its_icon() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let (mut app, camera, network) = start_phone(&environment, Camera::new());
    open_looks(&mut app);
    app.press("text:Tri-X 400").unwrap();
    app.wait_for_within("Enabled", WAIT).unwrap();
    app.press("role:button[Cover]").unwrap();
    app.wait_for_within("Film name", WAIT).unwrap();
    // The second color set of the editor, named by its look.
    app.press("label:Kodak yellow").unwrap();
    app.press("role:button[Save]").unwrap();
    until(&mut app, || {
        camera.lock().unwrap().looks[1].pending.is_some()
    });
    let puts = content_puts(&network, TRIX);
    let [(Some(cover), body)] = puts.as_slice() else {
        panic!("one PUT with a cover: {puts:?}");
    };
    let Cover::Film(film) = cover else {
        panic!("a film cover: {cover:?}");
    };
    let second = grkit_core::cover::find_look(grkit_core::cover::EDITOR_LOOKS[1]).unwrap();
    assert_eq!(film.colors, Some(second.colors()));
    // Same ID, name and effect: only the icon changes, to the cover.
    let (original, _) = effects::decode(&grf(TRIX)).unwrap();
    let (effect, icon) = effects::decode(body).unwrap();
    assert_eq!(effect, original);
    assert_eq!(icon, cover.pixels().unwrap());
    // Back on the Look, which the camera shows after it starts again.
    app.wait_for_within("Official · B&W · After restart", WAIT)
        .unwrap();
}

#[test]
fn an_official_look_comes_with_the_film_box_cover_of_the_card() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let velvia: Value = grkit_core::effects::parse_json(
        include_str!("../../effects/velvia-100-v1/effect.json").as_bytes(),
    )
    .unwrap();
    let file = grkit_core::effects::compile_definition(&velvia).unwrap();
    let name = effects::filename("velvia-100-v1").unwrap();
    let catalog = json!({
        "schema": grkit_core::site::CATALOG_SCHEMA, "target": "gr3-2.21",
        "base_gamma": vec![0u16; 256],
        "effects": [{"id": "velvia-100-v1", "name": "Velvia 100", "monochrome": false,
            "file": name, "sha256": grkit_core::packaging::sha256(&file),
            "cover": velvia["icon"]}]
    });
    let site = |path: &str, body: Vec<u8>| {
        let mut answer = StubResponse::default();
        answer.status = 200;
        answer.body = body;
        (
            format!("GET https://release.test/effects/gr3-2.21/{path}"),
            StubRoute::Response(answer),
        )
    };
    let mut camera = Camera::new();
    // Color Looks have the film box cover, B&W Looks the gray one.
    camera.official_covers = "colorFilmBox";
    let (mut app, camera, network) = start_phone_with(
        &environment,
        camera,
        vec![
            site("catalog.json", serde_json::to_vec(&catalog).unwrap()),
            site(&name, file.clone()),
        ],
    );
    open_looks(&mut app);
    app.press("label:Add Film Looks").unwrap();
    app.wait_for_within("Velvia 100", WAIT).unwrap();
    app.press("label:Add Velvia 100").unwrap();
    until(&mut app, || camera.lock().unwrap().looks.len() == 4);
    let film_box = Cover::from_json(&velvia["icon"]).unwrap().film_box();
    let puts = content_puts(&network, "velvia-100-v1");
    let [(Some(cover), body)] = puts.as_slice() else {
        panic!("one PUT with a cover: {puts:?}");
    };
    assert_eq!(cover, &film_box);
    let (effect, icon) = effects::decode(body).unwrap();
    assert_eq!(effect, effects::decode(&file).unwrap().0);
    assert_eq!(icon, film_box.pixels().unwrap());
}

#[test]
fn the_default_cover_puts_an_official_look_back_on_the_scheme() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let film_box = Cover::from_json(&definition(TRIX)["icon"])
        .unwrap()
        .film_box();
    let mut camera = Camera::new();
    camera.looks[1].default_cover = film_box.to_json();
    let (mut app, camera, network) = start_phone(&environment, camera);
    open_looks(&mut app);
    app.press("text:Tri-X 400").unwrap();
    app.wait_for_within("Enabled", WAIT).unwrap();
    app.press("role:button[Cover]").unwrap();
    app.wait_for_within("Use the Default Cover", WAIT).unwrap();
    app.press("role:button[Use the Default Cover]").unwrap();
    until(&mut app, || {
        camera.lock().unwrap().looks[1].pending.is_some()
    });
    let puts = content_puts(&network, TRIX);
    let [(Some(cover), body)] = puts.as_slice() else {
        panic!("one PUT with a cover: {puts:?}");
    };
    assert_eq!(cover, &film_box);
    assert_eq!(effects::decode(body).unwrap().1, film_box.pixels().unwrap());
}

#[test]
fn a_busy_camera_asks_for_the_shooting_screen() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let mut state = Camera::new();
    state.refuse = Some((503, "camera_busy"));
    let (mut app, camera, _network) = start_phone(&environment, state);
    open_looks(&mut app);
    app.press("text:Tri-X 400").unwrap();
    app.wait_for_within("Enabled", WAIT).unwrap();
    let titles = Arc::new(Mutex::new(Vec::new()));
    let seen = titles.clone();
    app.on_dialog(move |dialog| {
        seen.lock()
            .unwrap()
            .push(dialog.title.clone().unwrap_or_default());
        DialogAnswer::Press("OK".into())
    });
    app.press("role:switch[Enabled]").unwrap();
    until(&mut app, || !titles.lock().unwrap().is_empty());
    assert_eq!(titles.lock().unwrap().as_slice(), ["Camera Busy"]);
    assert!(camera.lock().unwrap().looks[1].enabled);
}

#[test]
fn a_look_that_waits_for_a_restart_says_so() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let mut state = Camera::new();
    // A Look that the phone added: the camera does not list it yet.
    state.looks[2].pending = Some("add");
    let (mut app, _camera, network) = start_phone(&environment, state);
    open_looks(&mut app);
    app.wait_for_within("1 change applies after a restart", WAIT)
        .unwrap();
    app.wait_for_within("label:Night, After restart", WAIT)
        .unwrap();
    app.press("text:Night").unwrap();
    app.wait_for_within("Enabled", WAIT).unwrap();
    // The camera cannot show or hide a Look that it does not list.
    let _ = app.press("role:switch[Enabled]");
    app.wait(Duration::from_millis(300)).unwrap();
    assert_eq!(count(&network, "PATCH", "/v1/looks/lut-night"), 0);
}

#[test]
fn deleting_the_look_in_use_switches_the_camera_to_off() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let mut state = Camera::new();
    state.looks[0].used_by = 0;
    let (mut app, camera, network) = start_phone(&environment, state);
    open_looks(&mut app);
    app.press("text:Portra 400").unwrap();
    app.wait_for_within("Delete Film Look", WAIT).unwrap();
    app.on_dialog(|_| DialogAnswer::Press("Delete".into()));
    app.press("Delete Film Look").unwrap();
    app.wait_for_within("In Camera Menu", WAIT).unwrap();
    app.wait_for_gone("text:Portra 400", WAIT).unwrap();
    assert!(camera.lock().unwrap().selected.is_none());
    assert!(
        network.requests().iter().any(|r| r.method == "DELETE"
            && r.url.ends_with("/v1/looks/portra-400-endura-v1?active=off"))
    );
}

#[test]
fn a_look_that_recipes_use_is_not_deleted() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let (mut app, _camera, network) = start_phone(&environment, Camera::new());
    open_looks(&mut app);
    app.press("text:Portra 400").unwrap();
    app.wait_for_within("Delete Film Look", WAIT).unwrap();
    app.on_dialog(|dialog| {
        assert_eq!(dialog.title.as_deref(), Some("Can’t Delete Portra 400"));
        DialogAnswer::Press("OK".into())
    });
    app.press("Delete Film Look").unwrap();
    app.wait(Duration::from_millis(300)).unwrap();
    assert_eq!(count(&network, "DELETE", "portra-400-endura-v1"), 0);
}

#[test]
fn a_refused_change_explains_the_pending_update() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let mut state = Camera::new();
    state.refuse = Some((409, "software_pending"));
    let (mut app, camera, _network) = start_phone(&environment, state);
    open_looks(&mut app);
    app.press("text:Tri-X 400").unwrap();
    app.wait_for_within("Enabled", WAIT).unwrap();
    let titles = Arc::new(Mutex::new(Vec::new()));
    let seen = titles.clone();
    app.on_dialog(move |dialog| {
        seen.lock()
            .unwrap()
            .push(dialog.title.clone().unwrap_or_default());
        DialogAnswer::Press("OK".into())
    });
    app.press("role:switch[Enabled]").unwrap();
    until(&mut app, || !titles.lock().unwrap().is_empty());
    assert_eq!(titles.lock().unwrap().as_slice(), ["An Update Is Waiting"]);
    assert!(camera.lock().unwrap().looks[1].enabled);
}

#[test]
fn dragging_a_look_saves_the_complete_order() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let (mut app, camera, _network) = start_phone(&environment, Camera::new());
    open_looks(&mut app);
    // In Edit, the handle of Night, the third row, dragged up two rows.
    app.press("role:button[Edit]").unwrap();
    app.wait_for_within("Drag to move Night", WAIT).unwrap();
    app.drag(
        "label:Drag to move Night",
        Point {
            x: Points(0.0),
            y: Points(-128.0),
        },
    )
    .unwrap();
    until(&mut app, || camera.lock().unwrap().order_revision > 1);
    let ids: Vec<&str> = camera.lock().unwrap().looks.iter().map(|e| e.id).collect();
    assert_eq!(ids, [CUSTOM, PORTRA, TRIX]);
}

/// The JSON bodies of the requests `method` to `suffix`.
fn bodies(network: &StubNetwork, method: &str, suffix: &str) -> Vec<Value> {
    network
        .requests()
        .iter()
        .filter(|r| r.method == method && r.url.ends_with(suffix))
        .map(|r| serde_json::from_slice(body(r)).unwrap())
        .collect()
}

/// A camera whose Tri-X 400 has its gray default cover and `cover`.
fn built_in_camera(cover: &Cover) -> Camera {
    let mut camera = Camera::new();
    camera.looks[1].default_cover = definition(TRIX)["icon"].clone();
    camera.looks[1].cover = cover.to_json();
    camera
}

#[test]
fn the_film_box_colors_set_the_scheme_of_the_camera() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let gray = Cover::from_json(&definition(TRIX)["icon"]).unwrap();
    let (mut app, camera, network) = start_phone(&environment, built_in_camera(&gray));
    open_looks(&mut app);
    open_cover_colors(&mut app);
    // No individual cover: no reset.
    assert!(
        app.wait_for_within("Reset B&W covers", Duration::from_millis(300))
            .is_err()
    );
    app.press("text:All").unwrap();
    until(&mut app, || {
        camera.lock().unwrap().official_covers == "filmBox"
    });
    assert_eq!(
        bodies(&network, "PUT", "/v1/looks/covers/scheme"),
        [json!({"scheme": "filmBox"})]
    );
}

#[test]
fn a_reset_puts_the_individual_covers_of_a_group_back() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let (mut app, camera, network) =
        start_phone(&environment, built_in_camera(&Cover::for_name("Mine")));
    open_looks(&mut app);
    open_cover_colors(&mut app);
    app.wait_for_within("Reset B&W covers", WAIT).unwrap();
    assert!(
        app.wait_for_within("Reset color covers", Duration::from_millis(300))
            .is_err()
    );
    app.press("role:button[Reset B&W covers]").unwrap();
    until(&mut app, || {
        let camera = camera.lock().unwrap();
        camera.looks[1].cover == camera.looks[1].default_cover
    });
    assert_eq!(
        bodies(&network, "POST", "/v1/looks/covers/reset"),
        [json!({"group": "mono"})]
    );
    app.wait_for_gone("Reset B&W covers", WAIT).unwrap();
}

#[test]
fn a_second_add_during_a_download_is_not_left_adding() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let effect = |id: &str| -> (Value, Vec<u8>, String) {
        let path = format!("{}/../effects/{id}/effect.json", env!("CARGO_MANIFEST_DIR"));
        let value = grkit_core::effects::parse_json(&fs::read(path).unwrap()).unwrap();
        let file = grkit_core::effects::compile_definition(&value).unwrap();
        (value, file, effects::filename(id).unwrap())
    };
    let (velvia, velvia_file, velvia_name) = effect("velvia-100-v1");
    let (astia, astia_file, astia_name) = effect("astia-100f-v1");
    let entry = |id: &str, name: &str, value: &Value, file: &[u8], path: &str| {
        json!({"id": id, "name": name, "monochrome": false, "file": path,
            "sha256": grkit_core::packaging::sha256(file), "cover": value["icon"]})
    };
    let catalog = json!({
        "schema": grkit_core::site::CATALOG_SCHEMA, "target": "gr3-2.21",
        "base_gamma": vec![0u16; 256],
        "effects": [
            entry("velvia-100-v1", "Velvia 100", &velvia, &velvia_file, &velvia_name),
            entry("astia-100f-v1", "Astia 100F", &astia, &astia_file, &astia_name),
        ]
    });
    let site = |path: &str, body: Vec<u8>, delay: Duration| {
        let mut answer = StubResponse::default();
        answer.status = 200;
        answer.body = body;
        answer.delay = delay;
        (
            format!("GET https://release.test/effects/gr3-2.21/{path}"),
            StubRoute::Response(answer),
        )
    };
    let (mut app, camera, _network) = start_phone_with(
        &environment,
        Camera::new(),
        vec![
            site(
                "catalog.json",
                serde_json::to_vec(&catalog).unwrap(),
                Duration::ZERO,
            ),
            site(&velvia_name, velvia_file, Duration::from_millis(600)),
            site(&astia_name, astia_file, Duration::ZERO),
        ],
    );
    open_looks(&mut app);
    app.press("label:Add Film Looks").unwrap();
    app.wait_for_within("Astia 100F", WAIT).unwrap();
    app.press("label:Add Velvia 100").unwrap();
    // The second tile waits for the first; a press on it does nothing.
    let _ = app.press("label:Add Astia 100F");
    until(&mut app, || camera.lock().unwrap().looks.len() == 4);
    app.wait_for_within("label:Add Astia 100F", WAIT).unwrap();
    app.wait(Duration::from_millis(200)).unwrap();
    app.press("label:Add Astia 100F").unwrap();
    until(&mut app, || camera.lock().unwrap().looks.len() == 5);
}

#[test]
fn a_drag_during_another_change_saves_no_order() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let mut state = Camera::new();
    state.write_delay = Duration::from_millis(600);
    let (mut app, camera, network) = start_phone(&environment, state);
    open_looks(&mut app);
    app.press("role:button[Edit]").unwrap();
    app.wait_for_within("Drag to move Night", WAIT).unwrap();
    app.press("label:Remove Tri-X 400 from the menu").unwrap();
    let _ = app.drag(
        "label:Drag to move Night",
        Point {
            x: Points(0.0),
            y: Points(-128.0),
        },
    );
    until(&mut app, || !camera.lock().unwrap().looks[1].enabled);
    app.wait(Duration::from_millis(300)).unwrap();
    assert_eq!(count(&network, "PUT", "/v1/looks/order"), 0);
    let ids: Vec<&str> = camera.lock().unwrap().looks.iter().map(|e| e.id).collect();
    assert_eq!(ids, [PORTRA, TRIX, CUSTOM]);
}

#[test]
fn a_refused_rename_keeps_the_typed_name() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let mut state = Camera::new();
    state.refuse = Some((409, "software_pending"));
    let (mut app, camera, _network) = start_phone(&environment, state);
    open_looks(&mut app);
    app.press("text:Tri-X 400").unwrap();
    app.wait_for_within("role:button[Name, Tri-X 400]", WAIT)
        .unwrap();
    let titles = Arc::new(Mutex::new(Vec::new()));
    let seen = titles.clone();
    app.on_dialog(move |dialog| {
        seen.lock()
            .unwrap()
            .push(dialog.title.clone().unwrap_or_default());
        DialogAnswer::Press("OK".into())
    });
    unsafe { std::env::set_var("GRKIT_TEST_NAME", "Tri-X 400 Push") };
    app.press("role:button[Name, Tri-X 400]").unwrap();
    until(&mut app, || !titles.lock().unwrap().is_empty());
    app.wait(Duration::from_millis(300)).unwrap();
    // The next alert opens with the refused text.
    unsafe { std::env::set_var("GRKIT_TEST_NAME", "<keep>") };
    app.press("role:button[Name, Tri-X 400]").unwrap();
    until(&mut app, || {
        camera.lock().unwrap().looks[1].name == "Tri-X 400 Push"
    });
    unsafe { std::env::remove_var("GRKIT_TEST_NAME") };
}

#[test]
fn software_update_closes_when_the_link_goes_down() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let mut state = Camera::new();
    state.looks[2].pending = Some("add");
    let (mut app, _camera, _network) = start_phone(&environment, state);
    app.wait_for_within("Firmware", WAIT).unwrap();
    app.press("role:button[GRKit 1.7.0]").unwrap();
    app.wait_for_within("Couldn't check for updates", WAIT)
        .unwrap();
    // The Looks tab restarts the camera, which ends the link as expected.
    app.press("role:button[Film Looks]").unwrap();
    app.wait_for_within("role:button[Restart Camera]", WAIT)
        .unwrap();
    app.on_dialog(|_| DialogAnswer::Press("Restart".into()));
    app.press("role:button[Restart Camera]").unwrap();
    app.wait_for_within("Camera Not Connected", WAIT).unwrap();
    app.press("role:button[Camera]").unwrap();
    app.wait_for_within("role:button[Connect]", WAIT).unwrap();
    app.wait_for_gone("Couldn't check for updates", WAIT)
        .unwrap();
}

#[test]
fn the_camera_menu_turns_the_camera_off_after_a_confirm() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let mut state = Camera::new();
    state.write_delay = Duration::from_millis(800);
    let (mut app, _camera, network) = start_phone(&environment, state);
    app.wait_for_within("Firmware", WAIT).unwrap();
    assert!(!app.screen().shows("Turn Off Camera"));
    let menus = Arc::new(Mutex::new(Vec::new()));
    let seen = menus.clone();
    app.on_dialog(move |dialog| {
        let title = dialog.title.clone().unwrap_or_default();
        let menu = title == "RICOH GR III";
        seen.lock().unwrap().push(title);
        DialogAnswer::Press(if menu { "Turn Off Camera" } else { "Turn Off" }.into())
    });
    app.press("label:More").unwrap();
    // The control works in place until the camera answers.
    let mut busy = false;
    for _ in 0..40 {
        if let Ok(state) = app.screen().accessibility_state("label:More") {
            busy = state.busy;
        }
        if busy {
            break;
        }
        app.wait(Duration::from_millis(25)).unwrap();
    }
    assert!(busy, "More reports busy while the camera turns off");
    // It takes no press meanwhile.
    let _ = app.press("label:More");
    assert_eq!(menus.lock().unwrap().len(), 2, "the menu, then its confirm");
    app.wait_for_gone("Firmware", WAIT).unwrap();
    assert_eq!(count(&network, "POST", "/v1/device/finish"), 1);
}

#[test]
fn a_drag_saves_the_order_at_a_large_text_size() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let mut state = Camera::new();
    state.font_scale = 2.0;
    let (mut app, camera, _network) = start_phone(&environment, state);
    open_looks(&mut app);
    app.press("role:button[Edit]").unwrap();
    app.wait_for_within("Drag to move Night", WAIT).unwrap();
    app.drag(
        "label:Drag to move Night",
        Point {
            x: Points(0.0),
            y: Points(-600.0),
        },
    )
    .unwrap();
    until(&mut app, || camera.lock().unwrap().order_revision > 1);
    let ids: Vec<&str> = camera.lock().unwrap().looks.iter().map(|e| e.id).collect();
    assert_eq!(ids, [CUSTOM, PORTRA, TRIX]);
}
