//! The Modules tab with the real store and HTTP transport, against a camera
//! that the test controls.

use std::{
    env, fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use grkit_app::App;
use nsh_core::platform::{HttpBody, HttpRequest, Os};
use nsh_engine::{EngineConfig, Root};
use nsh_testing::{DialogAnswer, Headless, StubNetwork, StubResponse, StubRoute};
use serde_json::{Value, json};

const WAIT: Duration = Duration::from_secs(10);
const HASH: &str = "00112233aabbccdd";
/// A module icon as the camera serves it: a 40 x 40 PNG.
const PNG: &[u8] = include_bytes!("../../modules/grain/tiles/0-module.png");
static ENV_LOCK: Mutex<()> = Mutex::new(());

struct Camera {
    capable: bool,
    schema: u64,
    level: i64,
    refuse: Option<(u16, &'static str)>,
}

impl Camera {
    fn new() -> Self {
        Self {
            capable: true,
            schema: 1,
            level: 1,
            refuse: None,
        }
    }

    fn module(&self) -> Value {
        json!({
            "id": "GRAIN", "enabled": true, "status": "running", "place": "root",
            "orderable": true, "values": {"level": self.level},
            "hiddenValues": [], "valueOrder": [], "revision": 2, "later": {"field": true}
        })
    }
}

fn response(status: u16, value: Value) -> StubResponse {
    let mut response = StubResponse::default();
    response.status = status;
    response.body = serde_json::to_vec(&value).unwrap();
    response
}

fn header<'a>(request: &'a HttpRequest, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

fn answer(camera: &mut Camera, request: &HttpRequest) -> StubResponse {
    let url = request.url.strip_prefix("http://camera.test").unwrap();
    let path = url.split('?').next().unwrap();
    match (request.method.as_str(), path) {
        ("GET", "/v1/ping") => response(200, json!({"protocol": 1, "bootId": "boot-test"})),
        ("GET", "/v1/props") => response(
            200,
            json!({
                "bootId": "boot-test", "cardId": "card-test",
                "camera": {"model": "RICOH GR III", "firmwareVersion": "2.21"},
                "grkit": {"version": "1.7.0"}, "storage": {"state": "ready"},
                "capabilities": {"moduleSettings": {"read": camera.capable, "write": camera.capable}}
            }),
        ),
        ("GET", "/v1/update") => response(
            200,
            json!({"inventory": {"package": {"version": "1.7.0", "modules": [], "effects": [],
                   "left_out": []}, "state": "none"}, "request": null}),
        ),
        ("GET", "/v1/looks") => response(
            200,
            json!({"bootId": "boot-test", "looks": [], "limits": {"loaded": 40, "listed": 64}}),
        ),
        ("GET", "/v1/recipes") => response(
            200,
            json!({"recipes": [], "limits": {"on": 40, "listed": 64}, "bootId": "boot-test"}),
        ),
        ("GET", "/v1/modules/catalog") => {
            let tag = format!("\"{HASH}\"");
            if header(request, "If-None-Match") == Some(tag.as_str()) {
                return response(304, Value::Null);
            }
            response(
                200,
                json!({
                    "schema": camera.schema, "catalog": HASH, "language": "en",
                    "modules": [{
                        "id": "GRAIN", "title": "Grain", "managed": null, "options": null,
                        "settings": [{"key": "level", "group": null, "label": "Level", "values": [
                            {"value": 0, "label": "Off"}, {"value": 1, "label": "Low"},
                            {"value": 2, "label": "High"}
                        ]}]
                    }]
                }),
            )
        }
        ("GET", "/v1/modules/GRAIN/icon") => {
            let mut icon = StubResponse::default();
            icon.status = 200;
            icon.body = PNG.to_vec();
            icon
        }
        // A GRKit from before the phone could change its GRKit Settings.
        ("GET", "/v1/params/settings") => {
            response(404, json!({"status": 404, "code": "not_found"}))
        }
        ("GET", "/v1/params/modules") => response(
            200,
            json!({"revision": 1, "grkitEnabled": true, "modules": [camera.module()]}),
        ),
        ("PATCH", "/v1/params/modules/GRAIN") => {
            assert_eq!(header(request, "X-GRKit-Boot-Id"), Some("boot-test"));
            if let Some((status, code)) = camera.refuse.take() {
                return response(status, json!({"status": status, "code": code}));
            }
            let HttpBody::Bytes(body) = &request.body else {
                panic!("a change has a body");
            };
            let patch: Value = serde_json::from_slice(body).unwrap();
            camera.level = patch["values"]["level"].as_i64().unwrap();
            response(200, camera.module())
        }
        _ => panic!("unexpected request: {} {}", request.method, request.url),
    }
}

/// A phone whose data directory outlives one app start.
struct Phone {
    directory: PathBuf,
}

impl Phone {
    fn new() -> Self {
        unsafe {
            env::remove_var("GRKIT_DEMO");
            env::remove_var("GRKIT_DEMO_SCREEN");
            env::remove_var("GRKIT_DEMO_FAIL");
            env::set_var("GRKIT_CAMERA_URL", "http://camera.test");
            env::set_var("GRKIT_RELEASE_BASE", "https://release.test");
        }
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self {
            directory: env::temp_dir().join(format!(
                "grkit-modules-test-{}-{unique}",
                std::process::id()
            )),
        }
    }

    fn start(&self, camera: &Arc<Mutex<Camera>>) -> (Headless, Arc<StubNetwork>) {
        let shared = camera.clone();
        let handler = StubRoute::Handler(Arc::new(move |request, reply| {
            let reply_with = answer(&mut shared.lock().unwrap(), &request);
            reply.send(reply_with);
        }));
        let mut missing = StubResponse::default();
        missing.status = 404;
        let network = Arc::new(StubNetwork::new(vec![
            ("GET http://camera.test/*".into(), handler.clone()),
            ("PATCH http://camera.test/*".into(), handler.clone()),
            ("POST http://camera.test/*".into(), handler),
            (
                "GET https://release.test/*".into(),
                StubRoute::Response(missing),
            ),
        ]));
        let mut config = EngineConfig::new(Root::of::<App>());
        config.environment.os = Os::Web;
        config.data_dir = Some(self.directory.clone());
        config.network = Some(network.clone());
        let mut app = Headless::start(config).unwrap();
        app.wait_for_within("label:More", WAIT).unwrap();
        // The Modules page opens from its row on the camera tab.
        // The camera tab draws its rows again once the Looks read ends.
        app.wait_for_within("Could not read the film looks on the camera.", WAIT)
            .unwrap();
        app.press(r"label:/Modules/").unwrap();
        (app, network)
    }
}

impl Drop for Phone {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
        unsafe {
            env::remove_var("GRKIT_CAMERA_URL");
            env::remove_var("GRKIT_RELEASE_BASE");
        }
    }
}

fn count(network: &StubNetwork, method: &str, suffix: &str) -> usize {
    network
        .requests()
        .iter()
        .filter(|r| r.method == method && r.url.ends_with(suffix))
        .count()
}

#[test]
fn a_value_waits_for_the_camera_and_the_catalog_comes_from_the_cache() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let phone = Phone::new();
    let camera = Arc::new(Mutex::new(Camera::new()));
    let (mut app, network) = phone.start(&camera);
    app.wait_for_within("text:Grain", WAIT).unwrap();
    // The camera tab reads the catalog at the connect; the page asks again
    // only with the cached version.
    let full_reads = network
        .requests()
        .iter()
        .filter(|r| {
            r.url.ends_with("/v1/modules/catalog?language=en")
                && header(r, "If-None-Match").is_none()
        })
        .count();
    assert_eq!(full_reads, 1);

    app.press("text:Grain").unwrap();
    app.wait_for_within("role:button[Level, Low]", WAIT)
        .unwrap();
    camera.lock().unwrap().refuse = Some((503, "camera_busy"));
    app.on_dialog(|dialog| {
        DialogAnswer::Press(
            if dialog.title.as_deref() == Some("Level") {
                "High"
            } else {
                "OK"
            }
            .into(),
        )
    });
    app.press("role:button[Level, Low]").unwrap();
    for _ in 0..100 {
        if count(&network, "PATCH", "/v1/params/modules/GRAIN") == 1 {
            break;
        }
        app.wait(Duration::from_millis(50)).unwrap();
    }
    app.wait(Duration::from_millis(200)).unwrap();
    assert_eq!(camera.lock().unwrap().level, 1);
    app.wait_for_within("role:button[Level, Low]", WAIT)
        .unwrap();

    app.press("role:button[Level, Low]").unwrap();
    app.wait_for_within("role:button[Level, High]", WAIT)
        .unwrap();
    assert_eq!(camera.lock().unwrap().level, 2);
    drop(app);

    let (mut app, network) = phone.start(&camera);
    app.wait_for_within("text:Grain", WAIT).unwrap();
    app.wait(Duration::from_millis(200)).unwrap();
    let catalog = network
        .requests()
        .into_iter()
        .find(|r| r.url.contains("/v1/modules/catalog"))
        .unwrap();
    assert_eq!(
        header(&catalog, "If-None-Match"),
        Some(format!("\"{HASH}\"").as_str())
    );
    assert_eq!(count(&network, "GET", "/v1/modules/GRAIN/icon"), 0);
}

#[test]
fn an_older_grkit_or_a_newer_catalog_shows_a_message() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let phone = Phone::new();
    let camera = Arc::new(Mutex::new(Camera::new()));
    camera.lock().unwrap().capable = false;
    let (mut app, network) = phone.start(&camera);
    app.wait_for_within("Update GRKit", WAIT).unwrap();
    assert_eq!(count(&network, "GET", "/v1/params/modules"), 0);
    drop(app);

    {
        let mut camera = camera.lock().unwrap();
        camera.capable = true;
        camera.schema = 2;
    }
    let (mut app, _network) = phone.start(&camera);
    app.wait_for_within("Update the App", WAIT).unwrap();
}
