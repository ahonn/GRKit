//! Real store and HTTP transport with a camera journal controlled by the test.
use std::{
    env,
    ffi::OsString,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use grkit_app::App;
use nsh_core::platform::{HttpBody, HttpRequest, Os};
use nsh_engine::{EngineConfig, Root};
use nsh_testing::{DialogAnswer, Headless, StubNetwork, StubResponse, StubRoute};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(10);
static ENV_LOCK: Mutex<()> = Mutex::new(());

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
            env::temp_dir().join(format!("grkit-update-test-{}-{unique}", std::process::id()));
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

struct Camera {
    version: &'static str,
    journal: &'static str,
    /// The update request that waits on the camera.
    request_id: Option<String>,
}

fn json_response(body: serde_json::Value) -> StubResponse {
    let mut response = StubResponse::default();
    response.status = 200;
    response.body = serde_json::to_vec(&body).unwrap();
    response
}

/// A software request names the camera start and nothing else, and has no
/// body.
fn require_context(request: &HttpRequest) {
    let header = |name: &str| {
        request
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(
        header("X-GRKit-Boot-Id"),
        Some("boot-test"),
        "mutation requires matching boot id"
    );
    for removed in [
        "X-GRKit-Request-Id",
        "X-GRKit-Card-Id",
        "X-GRKit-Content-SHA256",
        "If-Match",
    ] {
        assert_eq!(header(removed), None, "{removed} is not sent");
    }
    assert!(request_body(request).is_empty(), "no body");
}

fn summary(id: &str, status: &str, phase: &str) -> serde_json::Value {
    json!({
        "requestId": id,
        "domain": "update",
        "bootId": "boot-test",
        "cardId": "card-test",
        "status": status,
        "phase": phase,
        "progress": null,
        "cancellable": status == "running",
        "requiredAction": if phase == "awaitingConfirmation" { json!("confirmOnCamera") } else { json!(null) },
        "result": null,
        "error": null
    })
}

fn request_body(request: &HttpRequest) -> &[u8] {
    match &request.body {
        HttpBody::Bytes(bytes) => bytes,
        HttpBody::Empty => &[],
        HttpBody::Stream => panic!("unexpected streamed body"),
    }
}

fn start_phone(
    environment: &TestEnvironment,
    camera: Camera,
) -> (Headless, Arc<Mutex<Camera>>, Arc<StubNetwork>) {
    start_phone_with_delay(environment, camera, Duration::ZERO)
}

fn camera() -> Camera {
    Camera {
        version: "1.7.0",
        journal: "none",
        request_id: None,
    }
}

/// The phone saves what it updates before it sends the package. Here the
/// answer to `PUT /v1/update` did not arrive, and the update waits on the
/// camera for its confirmation.
fn start_phone_after_a_lost_answer(
    environment: &TestEnvironment,
) -> (Headless, Arc<Mutex<Camera>>, Arc<StubNetwork>) {
    // The engine's data folder is the app's own.
    let data = environment.directory.clone();
    fs::create_dir_all(&data).unwrap();
    fs::write(
        data.join("pending_update.json"),
        r#"{"camera_id":"dev-url","kind":"Install","baseline":"1.7.0","target":"1.8.0"}"#,
    )
    .unwrap();
    start_phone(
        environment,
        Camera {
            journal: "waiting",
            request_id: Some("request-test".into()),
            ..camera()
        },
    )
}

fn sent_packages(network: &StubNetwork) -> usize {
    network
        .requests()
        .iter()
        .filter(|r| r.method == "PUT" && r.url.ends_with("/v1/update"))
        .count()
}

fn start_phone_with_delay(
    environment: &TestEnvironment,
    camera: Camera,
    delay: Duration,
) -> (Headless, Arc<Mutex<Camera>>, Arc<StubNetwork>) {
    let camera = Arc::new(Mutex::new(camera));
    let handler_camera = camera.clone();
    let handler = StubRoute::Handler(Arc::new(move |request, reply| {
        let mut camera = handler_camera.lock().unwrap();
        let body = match (request.method.as_str(), request.url.as_str()) {
            ("GET", "http://camera.test/v1/props") => json!({
                "bootId": "boot-test",
                "cardId": "card-test",
                "camera": {"model": "RICOH GR III", "firmwareVersion": "2.21"},
                "grkit": {"version": camera.version},
                "storage": {"state": "ready", "writable": true, "freeBytes": 1_000_000_000u64},
                "runtime": {"state": "ready", "busyReason": null},
                "writeAccess": {"allowed": true, "reason": null},
                "capabilities": {
                    "looks": {"read": true, "write": true},
                    "recipes": {"read": false, "write": false},
                    "moduleSettings": {"read": false, "write": false}
                },
                "limits": {},
                "activeRequests": []
            }),
            ("GET", "http://camera.test/v1/ping") => json!({"protocol": 1, "bootId": "boot-test"}),
            ("GET", "http://camera.test/v1/update") => json!({
                "inventory": {
                    "package": {
                        "version": camera.version,
                        "modules": [],
                        "effects": [],
                        "left_out": []
                    },
                    "contentHashes": {},
                    "state": camera.journal
                },
                "request": camera.request_id.as_ref().map(|id| summary(
                    id,
                    "running",
                    if camera.journal == "confirmed" { "awaitingRestart" } else { "awaitingConfirmation" }
                ))
            }),
            // The camera tab reads the Looks after a connect.
            ("GET", "http://camera.test/v1/looks") => json!({
                "looks": [],
                "counts": {"loaded": 0},
                "limits": {"loadedLooks": 40, "listedLooks": 64},
                "bootId": "boot-test",
                "cardId": "card-test"
            }),
            ("POST", "http://camera.test/v1/update/cancel") => {
                require_context(&request);
                let id = camera.request_id.take().expect("an update waits");
                camera.journal = "none";
                summary(&id, "cancelled", "completed")
            }
            _ => panic!(
                "unexpected camera request: {} {}",
                request.method, request.url
            ),
        };
        let mut response = json_response(body);
        response.delay = delay;
        reply.send(response);
    }));
    let mut release_missing = StubResponse::default();
    release_missing.status = 404;
    release_missing.body = b"not published".to_vec();
    let network = Arc::new(StubNetwork::new([
        ("GET http://camera.test/*".into(), handler.clone()),
        ("PUT http://camera.test/*".into(), handler.clone()),
        ("POST http://camera.test/*".into(), handler),
        (
            "GET https://release.test/*".into(),
            StubRoute::Response(release_missing),
        ),
    ]));
    let mut config = EngineConfig::new(Root::of::<App>());
    config.environment.os = Os::Web;
    config.data_dir = Some(environment.directory.clone());
    config.network = Some(network.clone());
    let app = Headless::start(config).unwrap();
    (app, camera, network)
}

#[test]
fn an_update_whose_answer_was_lost_is_found_on_the_camera_and_completes() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let (mut app, camera, network) = start_phone_after_a_lost_answer(&environment);
    // The update found on the camera shows on the camera tab first.
    app.wait_for_within("GRKit 1.8.0 to Confirm", WAIT).unwrap();
    app.press("GRKit 1.8.0 to Confirm").unwrap();
    app.wait_for_within("Confirm on the Camera", WAIT).unwrap();
    camera.lock().unwrap().journal = "confirmed";
    app.wait_for_within("Restart the Camera", WAIT).unwrap();
    {
        let mut camera = camera.lock().unwrap();
        camera.journal = "none";
        camera.version = "1.8.0";
        camera.request_id = None;
    }
    // The restart the person runs, then a reconnect.
    app.press("Restart Manually").unwrap();
    app.press("Reconnect").unwrap();
    app.wait_for_within("GRKit 1.8.0 Is Installed", WAIT)
        .unwrap();
    app.press("Done").unwrap();
    app.wait_for_gone("GRKit 1.8.0 Is Installed", WAIT).unwrap();
    // The phone read the update resource instead of sending again.
    assert_eq!(sent_packages(&network), 0);
}

#[test]
fn cancelling_an_update_posts_cancel_and_verifies_the_camera_journal() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let (mut app, camera, network) = start_phone_after_a_lost_answer(&environment);
    // The update found on the camera shows on the camera tab first.
    app.wait_for_within("GRKit 1.8.0 to Confirm", WAIT).unwrap();
    app.press("GRKit 1.8.0 to Confirm").unwrap();
    app.wait_for_within("Confirm on the Camera", WAIT).unwrap();
    app.on_dialog(|_| DialogAnswer::Press("Cancel Update".into()));
    app.press("Cancel Update").unwrap();
    app.wait_for_gone("Confirm on the Camera", WAIT).unwrap();
    let requests = network.requests();
    let cancellation = requests
        .iter()
        .position(|r| r.method == "POST" && r.url.ends_with("/v1/update/cancel"))
        .expect("cancel must reach the camera");
    assert!(
        requests[cancellation + 1..]
            .iter()
            .any(|r| r.method == "GET" && r.url.ends_with("/v1/update")),
        "cancel must verify the resulting journal"
    );
    assert_eq!(camera.lock().unwrap().journal, "none");
    assert_eq!(sent_packages(&network), 0);
}

#[test]
fn automatic_connection_starts_with_a_local_placeholder() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let environment = TestEnvironment::new();
    let (mut app, _, _) =
        start_phone_with_delay(&environment, camera(), Duration::from_millis(300));
    app.wait_for("Connecting to your camera…").unwrap();
    app.wait_for_gone("Add Your Camera", WAIT).unwrap();
    app.wait_for_gone("Connecting to RICOH GR III", WAIT)
        .unwrap();
    app.wait_for_within("Firmware", WAIT).unwrap();
    app.wait_for_gone("Connecting to your camera…", WAIT)
        .unwrap();
}
