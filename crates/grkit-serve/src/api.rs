//! The v1 router.
use crate::update::{SoftwareContext, SoftwareRequest, SoftwareStatus};
use crate::{
    changes::{Changed, Changes},
    protocol::Problem,
    Env, Files, Request, Response,
};
use serde_json::{json, Value};

pub struct Reply {
    pub status: u16,
    pub body: Vec<u8>,
    headers: Vec<(String, String)>,
}
impl Reply {
    pub fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            body: serde_json::to_vec(&body).expect("JSON value"),
            headers: vec![
                (
                    "Content-Type".into(),
                    if status >= 400 {
                        "application/problem+json"
                    } else {
                        "application/json"
                    }
                    .into(),
                ),
                ("Cache-Control".into(), "no-store".into()),
            ],
        }
    }
    pub fn binary(status: u16, body: Vec<u8>, content_type: &str) -> Self {
        Self {
            status,
            body,
            headers: vec![("Cache-Control".into(), "no-cache".into())],
        }
        .header("Content-Type", content_type.to_owned())
    }
    /// The value of a response header.
    #[cfg(test)]
    pub(crate) fn header_value(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
    pub fn header(mut self, name: &str, value: String) -> Self {
        if name.is_empty()
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || value.bytes().any(|b| b < 32 || b == 127)
        {
            return Self::problem(Problem::new(
                500,
                "invalid_response_header",
                "The server could not construct a response header.",
            ));
        }
        self.headers
            .retain(|(key, _)| !key.eq_ignore_ascii_case(name));
        self.headers.push((name.to_owned(), value));
        self
    }
    pub fn problem(problem: Problem) -> Self {
        Self::json(
            problem.status,
            serde_json::to_value(problem).expect("problem JSON"),
        )
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut head = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
            self.status,
            crate::reason(self.status),
            self.body.len()
        );
        for (name, value) in &self.headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str("\r\n");
        let mut bytes = head.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}
impl From<Response> for Reply {
    fn from(response: Response) -> Self {
        Self::json(response.status, response.body)
    }
}

pub struct Service {
    pub changes: Changes,
    pub boot_id: String,
    card_identity: fn(&std::path::Path) -> anyhow::Result<String>,
    request_id: fn() -> anyhow::Result<String>,
    release_keys: Vec<[u8; 32]>,
    looks: crate::look_api::LookApi,
}
impl Service {
    pub fn new(boot_id: String) -> Self {
        Self::with_runtime_backend(
            boot_id,
            Box::new(crate::runtime_backend::UnavailableBackend),
        )
    }
    pub fn with_runtime_backend(
        boot_id: String,
        backend: Box<dyn crate::runtime_backend::RuntimeBackend>,
    ) -> Self {
        Self {
            changes: Changes::default(),
            card_identity: crate::identity::session_card_id,
            request_id: crate::identity::request_id,
            release_keys: grkit_core::release::OFFICIAL_KEYS.to_vec(),
            looks: crate::look_api::LookApi::new(backend),
            boot_id,
        }
    }
    /// The identity of the card: its marker file (`identity::card_id`).
    pub fn current_card_id(&self, root: &std::path::Path) -> anyhow::Result<String> {
        (self.card_identity)(root)
    }
    /// The count of setting changes of the running camera, for the change
    /// stream (`changes`). None while the service has no session with it.
    pub fn setting_changes(&self) -> Option<u32> {
        self.looks.state_changes()
    }
    /// Mark the service as running or stopped for the camera, whose resident
    /// loader starts and stops it with the access point.
    pub fn announce(&self, running: bool) -> bool {
        self.looks.announce(running)
    }
    pub fn handle(&self, request: &Request, body: &[u8], env: &Env<impl Files>) -> Reply {
        let reply = self.answer(request, body, env);
        // Only a request that is not a read can change the card.
        if request.method != "GET" {
            crate::update::forget_summary();
        }
        reply
    }
    fn answer(&self, request: &Request, body: &[u8], env: &Env<impl Files>) -> Reply {
        if request.method == "GET" && request.path == "/v1/ping" {
            return Reply::json(200, json!({"protocol":1,"bootId":self.boot_id}));
        }
        let route = request.path.strip_prefix("/v1/").unwrap_or("");
        if route == "looks"
            || route.starts_with("looks/")
            || route.starts_with("params/looks/")
            || route == "recipes"
            || route.starts_with("recipes/")
        {
            let card_id = self.current_card_id(env.card.root()).ok();
            let pending = env.card.state().map_or(true, |state| state != "none")
                || env.card.software_request().map_or(true, |request| {
                    request.is_some_and(|request| request.status == SoftwareStatus::Running)
                });
            let context = crate::look_api::RequestContext {
                boot_id: &self.boot_id,
                card_id: card_id.as_deref().unwrap_or("unavailable"),
                software_pending: pending,
                root: env.card.root(),
                release_keys: &self.release_keys,
                family: (env.device)()
                    .ok()
                    .and_then(|device| crate::update::recipe_family(&device.model)),
            };
            if let Some(reply) = self
                .looks
                .handle_recipes(request, body, &context, &self.changes)
            {
                return reply;
            }
            if let Some(reply) = self.looks.handle(request, body, &context, &self.changes) {
                return reply;
            }
        }

        match (request.method.as_str(), route) {
            ("GET", "props") => self.props(env),
            ("GET", "debug/diagnostics") => crate::diagnostics(env).into(),
            ("GET", "update") => self.software(env),
            ("PUT", "update") | ("POST", "update/cancel") => {
                self.mutate_software(request, body, env)
            }
            _ => Reply::problem(Problem::new(
                404,
                "not_found",
                "The resource does not exist.",
            )),
        }
    }
    fn props(&self, env: &Env<impl Files>) -> Reply {
        let card_id = self.current_card_id(env.card.root()).ok();
        let readable = env.card.ensure_readable().is_ok();
        let runtime = match card_id.as_deref() {
            Some(id) => self.looks.runtime_properties(id),
            None => json!({"state":"unavailable","busyReason":"card_identity_unavailable"}),
        };
        let looks_capabilities = runtime
            .get("capabilities")
            .cloned()
            .unwrap_or_else(|| json!({"read":false,"write":false}));
        let camera = (env.device)().ok();
        let active = env
            .card
            .software_request()
            .ok()
            .flatten()
            .filter(|request| {
                request.status == SoftwareStatus::Running
                    && card_id.as_deref() == Some(&request.card_id)
            })
            .map(|request| software_summary(&request));
        let writable = readable && card_id.is_some();
        Reply::json(
            200,
            json!({
                "deviceId":camera.as_ref().and_then(|c| c.serial_number.as_ref()).map(|serial| format!("camera-{}", grkit_core::packaging::sha256(serial.as_bytes()))),"bootId":self.boot_id,"cardId":card_id,
                "camera":camera.as_ref().map(|c| json!({"model":c.model,"firmwareVersion":c.firmware})),
                "grkit":{"version":env.card.package().ok().and_then(|package|package["version"].as_str().map(str::to_owned))},
                "storage":{"state":if readable {"ready"} else {"unavailable"},"writable":writable,"freeBytes":camera.as_ref().and_then(|c| c.free_bytes)},
                "runtime":runtime,
                "writeAccess":{"allowed":writable,"reason":if writable {None} else {Some("identity_unavailable")}},
                "capabilities":{"looks":looks_capabilities,"recipes":{"read":true,"write":true},"moduleSettings":{"read":false,"write":false}},
                "limits":{"loadedLooks":40,"listedLooks":64,"lookNameAsciiCharacters":16,"maxLookBytes":8704,"maxPackageBytes":crate::update::MAX_PACKAGE},
                "activeRequests":active.into_iter().collect::<Vec<_>>()
            }),
        )
    }
    /// The update resource: what the card holds, and the request that waits
    /// or the last one that ended.
    fn software(&self, env: &Env<impl Files>) -> Reply {
        let card_id = self.current_card_id(env.card.root()).ok();
        let request = match env.card.software_request() {
            Ok(value) => value
                .filter(|request| card_id.as_deref() == Some(&request.card_id))
                .map(|request| software_summary(&request)),
            Err(error) => return unavailable("recovery_required", &format!("{error:#}")),
        };
        match env.card.inventory() {
            Ok(inventory) => Reply::json(200, json!({"inventory":inventory,"request":request})),
            Err(error) => unavailable("card_unavailable", &format!("{error:#}")),
        }
    }

    /// The checks of an update that need no body. The transport runs them
    /// before it receives a package of several megabytes, and answers at once
    /// when one fails. The request repeats them after the body.
    pub fn precheck(&self, request: &Request, env: &Env<impl Files>) -> Option<Reply> {
        if (request.method.as_str(), request.path.as_str()) != ("PUT", "/v1/update") {
            return None;
        }
        self.admit_software(request, env).err()
    }
    /// A software change names the start of the camera that the phone read,
    /// and no other request waits.
    fn admit_software(&self, request: &Request, env: &Env<impl Files>) -> Result<String, Reply> {
        if request.headers.get("x-grkit-boot-id") != Some(&self.boot_id) {
            return Err(Reply::problem(Problem::new(
                409,
                "context_changed",
                "The camera started again. Read its state before changing it.",
            )));
        }
        if request.query.is_some() {
            return Err(Reply::problem(Problem::new(
                400,
                "invalid_query",
                "This request accepts no query.",
            )));
        }
        let card_id = self.current_card_id(env.card.root()).map_err(|_| {
            unavailable(
                "card_identity_unavailable",
                "The card identity is unavailable.",
            )
        })?;
        if !request.path.ends_with("/cancel") {
            if let Some(reply) = software_pending(env.card) {
                return Err(reply);
            }
        }
        Ok(card_id)
    }
    /// Stage an update, or cancel the one that waits. Each answers when it
    /// ends. A package that the phone sends again after an unknown result
    /// meets the request that waits (`software_pending`): the phone then reads
    /// the resource.
    fn mutate_software(&self, request: &Request, body: &[u8], env: &Env<impl Files>) -> Reply {
        let card_id = match self.admit_software(request, env) {
            Ok(card_id) => card_id,
            Err(reply) => return reply,
        };
        let cancel = request.path.ends_with("/cancel");
        let rejected = |error: anyhow::Error| {
            Reply::problem(Problem::new(
                if request.method == "PUT" { 422 } else { 409 },
                "software_request_rejected",
                format!("{error:#}"),
            ))
        };
        let outcome = if cancel {
            match env.card.software_request() {
                Ok(Some(target)) if target.card_id == card_id => {
                    env.card.cancel_software_request(&target.request_id)
                }
                Ok(_) => {
                    return Reply::problem(Problem::new(
                        409,
                        "no_software_request",
                        "No request of this kind waits on the card.",
                    ))
                }
                Err(error) => return unavailable("recovery_required", &format!("{error:#}")),
            }
        } else {
            let context = match (self.request_id)() {
                Ok(request_id) => SoftwareContext {
                    request_id,
                    boot_id: self.boot_id.clone(),
                    card_id: card_id.clone(),
                },
                Err(error) => return unavailable("coordinator_unavailable", &format!("{error:#}")),
            };
            let staged = match (env.device)() {
                Ok(device) => crate::update::stage_request(
                    env.card,
                    body,
                    &device,
                    &self.release_keys,
                    &context,
                ),
                Err(error) => return unavailable("camera_unavailable", &format!("{error:#}")),
            };
            // A failed journal publication can follow a durable request
            // checkpoint. Report that request instead of declaring failure.
            staged.or_else(|error| match env.card.software_request() {
                Ok(Some(record)) if record.request_id == context.request_id => Ok(record),
                _ => Err(error),
            })
        };
        // An error can also come after the cancellation is durable.
        let outcome = outcome.or_else(|error| match env.card.software_request() {
            Ok(Some(record))
                if cancel
                    && record.status == SoftwareStatus::Cancelled
                    && record.card_id == card_id =>
            {
                Ok(record)
            }
            _ => Err(error),
        });
        match outcome {
            Ok(result) => {
                self.changes.note(Changed::Software);
                let status = if result.status == SoftwareStatus::Running {
                    202
                } else {
                    200
                };
                Reply::json(status, software_summary(&result))
            }
            Err(error) => rejected(error),
        }
    }
}

fn software_summary(request: &SoftwareRequest) -> Value {
    use crate::update::SoftwarePhase;
    json!({"requestId":request.request_id,"domain":request.domain,"bootId":request.boot_id,"cardId":request.card_id,"status":request.status,"phase":request.phase,"progress":null,"cancellable":request.cancellable,
    "requiredAction":match request.phase {SoftwarePhase::AwaitingConfirmation => Some("confirmOnCamera"),SoftwarePhase::AwaitingRestart => Some("restartCamera"),_=>None},
    "result":if request.status==SoftwareStatus::Succeeded {json!({"identity":request.expected.digest})} else {Value::Null},
    "error":request.error.as_deref().map(|code| json!({"code":code,"detail":match code {
        crate::update::CONFIRMATION_EXPIRED => "Nobody confirmed the request on the camera in time.",
        crate::update::STAGING_FAILED => "The update could not be written to the card.",
        _ => "The request failed.",
    }}))})
}

/// A request that waits on the camera, or an earlier one whose outcome is
/// not yet known, blocks a new update.
fn software_pending(card: &crate::update::Card) -> Option<Reply> {
    let pending = || {
        Some(Reply::problem(Problem::new(
            409,
            "software_pending",
            "Finish or cancel the pending software request first.",
        )))
    };
    match card.state() {
        Ok("none") => {}
        Ok(_) => return pending(),
        Err(error) => return Some(unavailable("card_not_ready", &format!("{error:#}"))),
    }
    match card.software_request() {
        Ok(Some(previous)) if previous.status == SoftwareStatus::Running => pending(),
        Ok(_) => None,
        Err(error) => Some(unavailable("recovery_required", &format!("{error:#}"))),
    }
}

fn unavailable(code: &'static str, detail: &str) -> Reply {
    Reply::problem(Problem::new(503, code, detail))
}

#[cfg(test)]
mod tests {
    use super::*;
    use grkit_core::{camera, packaging, testing};
    fn service(boot: &str) -> Service {
        let mut service = Service::new(boot.into());
        service.card_identity = |_| Ok("card-test".into());
        service.release_keys = vec![testing::release_key()];
        service
    }
    fn card() -> (tempfile::TempDir, crate::update::Card) {
        let root = tempfile::tempdir().unwrap();
        let zip = testing::signed_package(&camera::GR3, "1.6.2", &["first"]);
        for (path, bytes) in packaging::card_files(&zip).unwrap() {
            let path = root.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        let card = crate::update::Card::new(root.path());
        (root, card)
    }
    fn request(service: &Service, method: &str, path: &str, length: usize) -> Request {
        let mut request = crate::parse(
            format!("{method} {path} HTTP/1.1\r\nContent-Length: {length}\r\n\r\n").as_bytes(),
        )
        .unwrap();
        request
            .headers
            .insert("x-grkit-boot-id".into(), service.boot_id.clone());
        request
    }
    fn with_env<T>(
        card: &crate::update::Card,
        run: impl FnOnce(&Env<'_, crate::System>) -> T,
    ) -> T {
        run(&Env {
            files: &crate::System,
            connection: &crate::Connection {
                local: "127.0.0.1:8080".into(),
                peer: "127.0.0.1:50000".into(),
            },
            card,
            device: &|| {
                Ok(crate::update::Device {
                    model: "RICOH GR III".into(),
                    firmware: "2.21".into(),
                    free_bytes: Some(1 << 30),
                    serial_number: Some("test-camera".into()),
                })
            },
        })
    }
    fn call(
        service: &Service,
        card: &crate::update::Card,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Reply {
        let request = request(service, method, path, body.len());
        with_env(card, |env| service.handle(&request, body, env))
    }
    fn body(reply: &Reply) -> Value {
        serde_json::from_slice(&reply.body).unwrap()
    }
    fn update() -> Vec<u8> {
        testing::signed_package(&camera::GR3, "1.7.0", &["second"])
    }

    #[test]
    fn an_update_that_cannot_succeed_is_answered_before_its_package() {
        let (_root, card) = card();
        let service = service("boot-test");
        let precheck = |boot: &str| {
            let mut request = request(&service, "PUT", "/v1/update", 5_000_000);
            request
                .headers
                .insert("x-grkit-boot-id".into(), boot.into());
            with_env(&card, |env| service.precheck(&request, env))
                .map(|reply| (reply.status, body(&reply)["code"].clone()))
        };
        assert_eq!(precheck("boot-test"), None);
        assert_eq!(
            precheck("boot-before").unwrap(),
            (409, json!("context_changed"))
        );
        // A request that waits on the camera blocks the next one.
        let staged = call(&service, &card, "PUT", "/v1/update", &update());
        assert_eq!(staged.status, 202, "{}", body(&staged));
        assert_eq!(
            precheck("boot-test").unwrap(),
            (409, json!("software_pending"))
        );
        // Only an update has a body to answer before.
        let cancel = request(&service, "POST", "/v1/update/cancel", 0);
        assert!(with_env(&card, |env| service.precheck(&cancel, env)).is_none());
    }

    #[test]
    fn an_update_waits_for_the_camera_and_can_be_cancelled_after_a_restart() {
        let (_root, card) = card();
        let service = service("boot-test");
        assert!(body(&call(&service, &card, "GET", "/v1/update", b""))["request"].is_null());
        let before = service.changes.counts();
        let staged = call(&service, &card, "PUT", "/v1/update", &update());
        assert_eq!(staged.status, 202, "{}", body(&staged));
        let summary = body(&staged);
        assert_eq!(summary["status"], "running");
        assert_eq!(summary["phase"], "awaitingConfirmation");
        assert_eq!(summary["requiredAction"], "confirmOnCamera");
        assert_ne!(
            service.changes.counts(),
            before,
            "the stream tells the phone"
        );
        assert_eq!(
            body(&call(&service, &card, "GET", "/v1/update", b""))["request"],
            summary
        );
        assert_eq!(
            body(&call(&service, &card, "GET", "/v1/props", b""))["activeRequests"],
            json!([summary])
        );
        // The same package again, as after an answer that did not arrive:
        // nothing changes, and the resource has the request that waits.
        let again = call(&service, &card, "PUT", "/v1/update", &update());
        assert_eq!(again.status, 409);
        assert_eq!(body(&again)["code"], "software_pending");

        // The service of the next start reads the request from the card.
        let restarted = self::service("boot-next");
        assert_eq!(
            body(&call(&restarted, &card, "GET", "/v1/update", b""))["request"]["requestId"],
            summary["requestId"]
        );
        let cancelled = call(&restarted, &card, "POST", "/v1/update/cancel", b"");
        assert_eq!(cancelled.status, 200, "{}", body(&cancelled));
        assert_eq!(body(&cancelled)["status"], "cancelled");
        assert_eq!(card.state().unwrap(), "none");
        // A cancel that the phone sends again has the same answer.
        let repeated = call(&restarted, &card, "POST", "/v1/update/cancel", b"");
        assert_eq!(body(&repeated)["status"], "cancelled");
        // The card takes the next update.
        let next = call(&restarted, &card, "PUT", "/v1/update", &update());
        assert_eq!(next.status, 202, "{}", body(&next));
    }

    #[test]
    fn a_change_names_the_start_that_the_phone_read() {
        let (_root, card) = card();
        let service = service("boot-test");
        for (method, path) in [("PUT", "/v1/update"), ("POST", "/v1/update/cancel")] {
            let mut request = request(&service, method, path, 0);
            request
                .headers
                .insert("x-grkit-boot-id".into(), "boot-before".into());
            let reply = with_env(&card, |env| service.handle(&request, b"", env));
            assert_eq!(body(&reply)["code"], "context_changed", "{path}");
            request.headers.remove("x-grkit-boot-id");
            let reply = with_env(&card, |env| service.handle(&request, b"", env));
            assert_eq!(body(&reply)["code"], "context_changed", "{path}");
        }
        let query = call(&service, &card, "POST", "/v1/update/cancel?force=1", b"");
        assert_eq!(body(&query)["code"], "invalid_query");
        assert_eq!(card.state().unwrap(), "none");
    }

    #[test]
    fn a_request_without_its_condition_is_rejected() {
        let (_root, card) = card();
        let service = service("boot-test");
        // Nothing waits.
        let cancel = call(&service, &card, "POST", "/v1/update/cancel", b"");
        assert_eq!(body(&cancel)["code"], "no_software_request");
        // Not a signed package.
        let invalid = call(&service, &card, "PUT", "/v1/update", b"not a package");
        assert_eq!(invalid.status, 422);
        assert_eq!(body(&invalid)["code"], "software_request_rejected");
        assert_eq!(card.state().unwrap(), "none");
    }

    #[test]
    fn the_request_of_another_card_is_not_shown_or_cancelled() {
        let (_root, card) = card();
        let service = service("boot-test");
        assert_eq!(
            call(&service, &card, "PUT", "/v1/update", &update()).status,
            202
        );
        // The same files under another card identity: a copy of the folder.
        let mut other = self::service("boot-test");
        other.card_identity = |_| Ok("card-other".into());
        assert!(body(&call(&other, &card, "GET", "/v1/update", b""))["request"].is_null());
        let cancel = call(&other, &card, "POST", "/v1/update/cancel", b"");
        assert_eq!(body(&cancel)["code"], "no_software_request");
        assert_eq!(card.state().unwrap(), "waiting");
    }

    #[test]
    fn props_and_errors_have_their_media_types() {
        let (_root, card) = card();
        let service = service("boot-test");
        let props = call(&service, &card, "GET", "/v1/props", b"");
        assert_eq!(props.status, 200);
        assert_eq!(props.header_value("Content-Type"), Some("application/json"));
        assert_eq!(props.header_value("ETag"), None);
        let value = body(&props);
        assert_eq!(value["bootId"], "boot-test");
        assert_eq!(value["cardId"], "card-test");
        assert_eq!(value["camera"]["model"], "RICOH GR III");
        assert_eq!(value["grkit"]["version"], "1.6.2");
        assert_eq!(value["writeAccess"]["allowed"], true);
        let missing = call(&service, &card, "GET", "/v1/other", b"");
        assert_eq!(missing.status, 404);
        assert_eq!(
            missing.header_value("Content-Type"),
            Some("application/problem+json")
        );
        let ping = call(&service, &card, "GET", "/v1/ping", b"");
        assert_eq!(body(&ping), json!({"protocol":1,"bootId":"boot-test"}));
    }
}
