//! The camera that the Looks tests control: its Looks, the HTTP routes it
//! answers, and the phone started against it.

use std::{
    env,
    ffi::OsString,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use grkit_app::App;
use grkit_core::cover::Cover;
use grkit_protocol::looks::{self as wire, LookSource, PendingChange};
use grkit_protocol::props;
use nsh_core::platform::{HttpBody, HttpRequest, Os};
use nsh_engine::{EngineConfig, Root};
use nsh_testing::{Headless, StubNetwork, StubResponse, StubRoute};
use serde_json::{Value, json};

pub const WAIT: Duration = Duration::from_secs(10);
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

pub const PORTRA: &str = "portra-400-endura-v1";
pub const TRIX: &str = "trix-400-mono-v1";
pub const CUSTOM: &str = "lut-night";

pub struct TestEnvironment {
    pub previous: Vec<(&'static str, Option<OsString>)>,
    pub directory: PathBuf,
}

impl TestEnvironment {
    pub fn new() -> Self {
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
pub struct Entry {
    pub id: &'static str,
    pub name: String,
    pub enabled: bool,
    pub source: LookSource,
    pub used_by: usize,
    /// The file change that waits for the next start of the camera.
    pub pending: Option<PendingChange>,
    pub revision: u32,
    pub cover: Value,
    pub default_cover: Value,
    /// On the card only (`GRKIT/STORED`).
    pub stored: bool,
}

pub struct Camera {
    pub looks: Vec<Entry>,
    pub selected: Option<&'static str>,
    /// Answer the next mutation with this status and code.
    pub refuse: Option<(u16, &'static str)>,
    pub order_revision: u32,
    /// The color scheme of the covers of the official Looks.
    pub official_covers: &'static str,
    /// How long the camera takes to answer a change.
    pub write_delay: Duration,
    /// The person's text size.
    pub font_scale: f32,
}

pub fn definition(id: &str) -> Value {
    let definition = match id {
        PORTRA => include_str!("../../../effects/portra-400-endura-v1/effect.json"),
        _ => include_str!("../../../effects/trix-400-mono-v1/effect.json"),
    };
    grkit_core::effects::parse_json(definition.as_bytes()).unwrap()
}

pub fn grf(id: &str) -> Vec<u8> {
    grkit_core::effects::compile_definition(&definition(id)).unwrap()
}

pub fn hash(id: &str, revision: u32) -> String {
    grkit_core::packaging::sha256(format!("{id}:{revision}").as_bytes())
}

impl Camera {
    pub fn new() -> Self {
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
            stored: false,
        };
        let mut trix = entry(TRIX, "Tri-X 400", LookSource::Official, 0);
        trix.cover = definition(TRIX)["icon"].clone();
        Self {
            looks: vec![
                entry(PORTRA, "Portra 400", LookSource::Official, 1),
                trix,
                entry(CUSTOM, "Night", LookSource::Custom, 0),
            ],
            selected: Some(PORTRA),
            refuse: None,
            order_revision: 1,
            official_covers: "gray",
            write_delay: Duration::ZERO,
            font_scale: 1.0,
        }
    }

    /// The list of the camera service (`wire::LookList`).
    pub fn list(&self) -> Value {
        let cover = |cover: &Value| (!cover.is_null()).then(|| cover.clone());
        let list = |stored: bool| -> Vec<wire::Look> {
            self.looks
                .iter()
                .filter(|e| e.stored == stored)
                .enumerate()
                .map(|(position, e)| wire::Look {
                    configuration: wire::LookConfiguration {
                        id: e.id.into(),
                        name: e.name.clone(),
                        content_hash: hash(e.id, e.revision),
                        enabled: e.enabled && !stored,
                        place: if stored {
                            wire::Place::Card
                        } else {
                            wire::Place::Camera
                        },
                        cover: cover(&e.cover),
                        default_cover: cover(&e.default_cover),
                    },
                    position,
                    runtime: wire::LookRuntime {
                        id: e.id.into(),
                        source: e.source,
                        loaded: e.enabled && !stored,
                        selected: self.selected == Some(e.id),
                        used_by: e.used_by,
                        uses: Vec::new(),
                        pending: e.pending,
                    },
                })
                .collect()
        };
        let (looks, stored) = (list(false), list(true));
        let loaded = looks.iter().filter(|l| l.configuration.enabled).count();
        serde_json::to_value(wire::LookList {
            counts: wire::Counts {
                listed: looks.len(),
                loaded,
                stored: stored.len(),
            },
            looks,
            stored,
            official_covers: Some(self.official_covers.into()),
            limits: wire::Limits {
                listed_looks: 64,
                loaded_looks: 40,
                stored_looks: Some(500),
            },
            diagnostics: Vec::new(),
            boot_id: "boot-test".into(),
            card_id: "card-test".into(),
        })
        .unwrap()
    }
}

pub fn header<'a>(request: &'a HttpRequest, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

pub fn body(request: &HttpRequest) -> &[u8] {
    match &request.body {
        HttpBody::Bytes(bytes) => bytes,
        HttpBody::Empty => &[],
        HttpBody::Stream => panic!("unexpected streamed body"),
    }
}

pub fn response(status: u16, value: Value) -> StubResponse {
    let mut response = StubResponse::default();
    response.status = status;
    response.body = serde_json::to_vec(&value).unwrap();
    response
}

/// Check the context of a mutation and refuse or accept it.
pub fn admit(camera: &mut Camera, request: &HttpRequest) -> Result<(), StubResponse> {
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
pub fn finish() -> StubResponse {
    response(200, json!({}))
}

pub fn start_phone(
    environment: &TestEnvironment,
    camera: Camera,
) -> (Headless, Arc<Mutex<Camera>>, Arc<StubNetwork>) {
    start_phone_with(environment, camera, Vec::new())
}

/// `start_phone` with `site` routes before the release site, which answers
/// 404.
pub fn start_phone_with(
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
                serde_json::to_value(props::PropsResponse {
                    device_id: None,
                    boot_id: "boot-test".into(),
                    card_id: Some("card-test".into()),
                    camera: Some(props::PropsCamera {
                        model: Some("RICOH GR III".into()),
                        firmware_version: Some("2.21".into()),
                    }),
                    grkit: props::PropsGrkit {
                        version: Some("1.7.0".into()),
                    },
                    storage: Some(props::PropsStorage {
                        state: Some("ready".into()),
                        free_bytes: None,
                    }),
                })
                .unwrap(),
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
                            source: LookSource::Custom,
                            used_by: 0,
                            pending: Some(PendingChange::Add),
                            revision: 1,
                            cover,
                            default_cover: Value::Null,
                            stored: false,
                        });
                    } else {
                        let entry = camera.looks.iter_mut().find(|e| e.id == id).unwrap();
                        entry.pending.get_or_insert(PendingChange::Update);
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
                        entry.pending = Some(PendingChange::Update);
                    }
                    // A move: the file goes to the other folder.
                    if let Some(place) = patch["place"].as_str() {
                        entry.stored = place == "card";
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
            ("GET", ["", "v1", "params", "settings"]) => {
                response(404, json!({"status": 404, "code": "not_found"}))
            }
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
    let mut config = EngineConfig::new(Root::of::<App>().translations(grkit_app::translations()));
    config.environment.os = Os::Web;
    config.environment.font_scale = font_scale;
    config.data_dir = Some(environment.directory.clone());
    config.network = Some(network.clone());
    let app = Headless::start(config).unwrap();
    (app, camera, network)
}

pub fn open_looks(app: &mut Headless) {
    // The camera card shows the versions once connected.
    app.wait_for_within("label:More", WAIT).unwrap();
    app.press("role:button[Film Looks]").unwrap();
    app.wait_for_within("In Camera Menu", WAIT).unwrap();
}

/// Let the app run until `done` holds, at most `WAIT`.
pub fn until(app: &mut Headless, mut done: impl FnMut() -> bool) {
    for _ in 0..100 {
        if done() {
            return;
        }
        app.wait(Duration::from_millis(100)).unwrap();
    }
    panic!("condition not reached within {WAIT:?}");
}

/// The cover header and the GRF body of the content PUTs of `id`.
pub fn content_puts(network: &StubNetwork, id: &str) -> Vec<(Option<Cover>, Vec<u8>)> {
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

pub fn count(network: &StubNetwork, method: &str, suffix: &str) -> usize {
    network
        .requests()
        .iter()
        .filter(|r| r.method == method && r.url.ends_with(suffix))
        .count()
}
