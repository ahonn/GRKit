//! The rows of MENU > GRKit Settings (`runtime/manage.h`), read and set
//! through the mailbox (`CONTROL_SETTINGS_READ`, `CONTROL_SETTINGS_SET`),
//! and the picture of Shutdown Screen on the card (`grkit_core::goodbye`).
use super::{lock, problem, try_lock, write_checked, RequestContext, Resources, Result};
use crate::codec::Codes;
use crate::{
    api::Reply, protocol::Problem, route::SettingsRoute, runtime_backend::RuntimeBackend, Request,
};
use grkit_core::{abi::*, goodbye};
use serde_json::{json, Map, Value};
use std::{fs, path::Path};

/// The kind of values of a setting in JSON.
#[derive(Clone, Copy)]
enum Kind {
    /// The texts of the values, by value index.
    Named(&'static [&'static str]),
    /// 0 is false, 1 is true.
    Switch,
}

/// The settings in the order of `CONTROL_SETTING_*`: their JSON key and kind.
const SETTINGS: [(&str, Kind); CONTROL_SETTINGS as usize] = [
    ("start", Kind::Named(&["on", "off", "last"])),
    ("uncroppedRaw", Kind::Switch),
    ("distanceUnit", Kind::Named(&["meters", "feet"])),
    ("simpleBattery", Kind::Switch),
    ("batteryPercent", Kind::Switch),
    ("playbackIcons", Kind::Switch),
    ("customPicture", Kind::Switch),
    ("showText", Kind::Switch),
];

const CODES: Codes = Codes {
    list: "The settings",
    changed: "The settings of the camera changed. Read them again.",
    unsupported: "This camera cannot manage GRKit settings.",
    invalid: Some(unavailable),
};
fn malformed() -> Problem {
    CODES.malformed()
}
fn invalid_setting() -> Problem {
    problem(
        400,
        "invalid_setting",
        "Send an object of known settings, each with a value that it takes.",
    )
}
fn unavailable() -> Problem {
    problem(
        409,
        "setting_unavailable",
        "This camera does not have the setting.",
    )
}
/// The value of each setting, or None for one that the camera does not have.
fn read_settings(backend: &mut dyn RuntimeBackend) -> Result<Vec<Option<u32>>> {
    let bytes = CODES.checked(backend.exchange(CONTROL_SETTINGS_READ, &[])?)?;
    if bytes.len() != 4 * SETTINGS.len() {
        return Err(malformed());
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|word| u32::from_le_bytes(word.try_into().expect("a word")))
        .map(|value| (value != CONTROL_SETTING_NONE).then_some(value))
        .collect())
}

fn settings_json(values: &[Option<u32>]) -> Result<Value> {
    let mut settings = Map::new();
    for ((key, kind), value) in SETTINGS.iter().zip(values) {
        let Some(value) = *value else { continue };
        let item = match kind {
            Kind::Named(names) => json!(names.get(value as usize).ok_or_else(malformed)?),
            Kind::Switch if value <= 1 => json!(value == 1),
            Kind::Switch => return Err(malformed()),
        };
        settings.insert((*key).to_owned(), item);
    }
    Ok(json!({ "settings": settings }))
}

/// The changes of a PATCH body as (index, value), in the order of
/// `SETTINGS`.
fn changes(body: &Value) -> Result<Vec<(u32, u32)>> {
    let object = body.as_object().ok_or_else(invalid_setting)?;
    if object
        .keys()
        .any(|key| !SETTINGS.iter().any(|(name, _)| name == key))
    {
        return Err(invalid_setting());
    }
    let mut out = Vec::new();
    for (index, (key, kind)) in SETTINGS.iter().enumerate() {
        let Some(item) = object.get(*key) else {
            continue;
        };
        let value = match kind {
            Kind::Named(names) => item
                .as_str()
                .and_then(|text| names.iter().position(|name| *name == text)),
            Kind::Switch => item.as_bool().map(usize::from),
        };
        out.push((index as u32, value.ok_or_else(invalid_setting)? as u32));
    }
    Ok(out)
}

impl Resources {
    /// Whether the runtime answers `CONTROL_SETTINGS_READ`. It asks once: a
    /// camera that does not answer yet is asked again at the next read.
    pub fn camera_settings(&self) -> bool {
        if let Some(supported) = lock(&self.modules).settings {
            return supported;
        }
        let Ok(mut backend) = try_lock(&self.backend) else {
            return false;
        };
        let answer = match backend.exchange(CONTROL_SETTINGS_READ, &[]) {
            Ok((0, _)) => Some(true),
            Ok((CONTROL_UNSUPPORTED, _)) => Some(false),
            _ => None,
        };
        lock(&self.modules).settings = answer;
        answer.unwrap_or(false)
    }

    /// A request of the GRKit settings or the shutdown picture, with a method
    /// of its route that the service admitted (`api::Service`).
    pub fn handle_settings(
        &self,
        route: &SettingsRoute,
        request: &Request,
        body: &[u8],
        context: &RequestContext<'_>,
    ) -> Reply {
        let root = context.root;
        let result = match (route, request.method.as_str()) {
            (SettingsRoute::Settings, "GET") => try_lock(&self.backend)
                .and_then(|mut backend| settings_json(&read_settings(backend.as_mut())?))
                .map(|value| Reply::json(200, value)),
            (SettingsRoute::Settings, _) => self.patch_settings(body),
            (SettingsRoute::ShutdownPicture, "GET") => read_picture(root),
            (SettingsRoute::ShutdownPicture, "PUT") => {
                write_picture(root, body).inspect(|_| self.check_picture())
            }
            (SettingsRoute::ShutdownPicture, _) => {
                remove_picture(root).inspect(|_| self.check_picture())
            }
        };
        result.unwrap_or_else(Reply::problem)
    }

    /// Ask the runtime to check the picture of the card again, so that the
    /// next power-off shows the change. The card already has the change: if
    /// the runtime does not answer, the change applies at the next start.
    fn check_picture(&self) {
        let _ = crate::sync::lock(&self.backend).exchange(CONTROL_GOODBYE_CHECK, &[]);
    }

    /// Check the whole body and that the camera has each setting, then set
    /// each one in the order of `SETTINGS`.
    fn patch_settings(&self, body: &[u8]) -> Result<Reply> {
        let changes = changes(&Resources::json_body(body)?)?;
        let mut backend = try_lock(&self.backend)?;
        let current = read_settings(backend.as_mut())?;
        if changes
            .iter()
            .any(|(index, _)| current[*index as usize].is_none())
        {
            return Err(unavailable());
        }
        for (index, value) in changes {
            let payload: Vec<u8> = [index, value]
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect();
            CODES.checked(backend.exchange(CONTROL_SETTINGS_SET, &payload)?)?;
        }
        Ok(Reply::json(
            200,
            settings_json(&read_settings(backend.as_mut())?)?,
        ))
    }
}

fn picture_path(root: &Path) -> std::path::PathBuf {
    root.join(goodbye::FILE)
}
fn read_picture(root: &Path) -> Result<Reply> {
    match fs::read(picture_path(root)) {
        Ok(bytes) => Ok(Reply::binary(200, bytes, "image/jpeg")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(problem(
            404,
            "not_found",
            "The card has no shutdown picture.",
        )),
        Err(_) => Err(card_unreadable()),
    }
}
/// Write the file in place, as a Look file: the camera shows it after its
/// next start.
fn write_picture(root: &Path, body: &[u8]) -> Result<Reply> {
    if body.len() > goodbye::MAX_BYTES {
        return Err(problem(
            413,
            "body_too_large",
            "The picture is larger than the camera reads.",
        ));
    }
    if !goodbye::usable(body) {
        return Err(problem(
            400,
            "invalid_picture",
            "The picture is not a baseline JPEG of 720 by 480 pixels with 4:2:0 chroma.",
        ));
    }
    if !write_checked(&picture_path(root), body) {
        return Err(problem(
            507,
            "card_write_failed",
            "The picture could not be written to the card.",
        ));
    }
    Ok(Reply::empty(204))
}
/// Remove the file. As for a Look file, the service first looks up each file
/// of the folder, and after the delete one name that no file has: on the
/// camera, the lookup of the next new name in a folder can fail after a
/// delete, and Linux keeps that answer.
fn remove_picture(root: &Path) -> Result<Reply> {
    let path = picture_path(root);
    let directory = path.parent().expect("the data folder");
    let entries = fs::read_dir(directory).map_err(|_| card_unreadable())?;
    for entry in entries.flatten() {
        let _ = fs::metadata(entry.path());
    }
    match fs::remove_file(&path) {
        Ok(()) => crate::update::look_up_unused(directory),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(problem(
                507,
                "card_write_failed",
                "The picture could not be removed from the card.",
            ))
        }
    }
    Ok(Reply::empty(204))
}
fn card_unreadable() -> Problem {
    problem(503, "card_unavailable", "The card cannot be read.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::RuntimeBackend;
    use std::sync::{Arc, Mutex};

    /// A camera with Enable at Start, Distance Unit, and Show Text.
    struct Fake {
        values: Values,
        answer: u32,
        /// The checks of the shutdown picture asked for.
        checks: Arc<Mutex<u32>>,
    }
    impl RuntimeBackend for Fake {
        fn exchange(&mut self, command: u32, payload: &[u8]) -> Result<(u32, Vec<u8>)> {
            let mut values = self.values.lock().unwrap();
            Ok(match command {
                CONTROL_SETTINGS_READ => (0, values.iter().flat_map(|w| w.to_le_bytes()).collect()),
                CONTROL_GOODBYE_CHECK => {
                    *self.checks.lock().unwrap() += 1;
                    (0, vec![1, 0, 0, 0])
                }
                CONTROL_SETTINGS_SET if self.answer == 0 => {
                    let word =
                        |at: usize| u32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
                    values[word(0) as usize] = word(4);
                    (0, Vec::new())
                }
                _ => (self.answer, Vec::new()),
            })
        }
    }
    type Values = Arc<Mutex<[u32; 8]>>;
    fn api(answer: u32) -> (Resources, Values) {
        let (api, values, _) = api_with_checks(answer);
        (api, values)
    }
    fn api_with_checks(answer: u32) -> (Resources, Values, Arc<Mutex<u32>>) {
        let none = CONTROL_SETTING_NONE;
        let values = Arc::new(Mutex::new([2, none, 0, none, none, none, none, 1]));
        let checks = Arc::new(Mutex::new(0));
        let fake = Fake {
            values: values.clone(),
            answer,
            checks: checks.clone(),
        };
        (Resources::new(Box::new(fake)), values, checks)
    }
    fn call(api: &Resources, root: &Path, head: &str, body: &[u8]) -> Reply {
        let text = format!(
            "{head} HTTP/1.1\r\nContent-Length: {}\r\nX-GRKit-Boot-Id: boot\r\n\r\n",
            body.len()
        );
        let request = crate::parse(text.as_bytes()).unwrap();
        let context = RequestContext {
            boot_id: "boot",
            card_id: Some("card".into()),
            root,
            release_keys: &[],
            family: None,
        };
        let Some(crate::route::Route::Settings(route)) = crate::route::Route::parse(&request.path)
        else {
            panic!("not a settings route: {}", request.path);
        };
        api.handle_settings(&route, &request, body, &context)
    }
    fn json_of(reply: &Reply) -> Value {
        reply.value()
    }

    #[test]
    fn the_settings_read_and_change_by_name() {
        let (api, values) = api(0);
        let root = Path::new("/");
        let read = call(&api, root, "GET /v1/params/settings", b"");
        assert_eq!(
            json_of(&read),
            json!({"settings":{"start":"last","distanceUnit":"meters","showText":true}})
        );
        let patch = br#"{"showText":false,"distanceUnit":"feet","start":"on"}"#;
        let changed = call(&api, root, "PATCH /v1/params/settings", patch);
        assert_eq!(
            json_of(&changed),
            json!({"settings":{"start":"on","distanceUnit":"feet","showText":false}})
        );
        // A bad key or value, or a setting that the camera does not have,
        // changes nothing.
        let before = *values.lock().unwrap();
        for (body, status, code) in [
            (&br#"{"start":"later"}"#[..], 400, "invalid_setting"),
            (br#"{"showText":1}"#, 400, "invalid_setting"),
            (br#"{"other":true}"#, 400, "invalid_setting"),
            (br#"[]"#, 400, "invalid_setting"),
            (
                br#"{"start":"off","uncroppedRaw":true}"#,
                409,
                "setting_unavailable",
            ),
        ] {
            let reply = call(&api, root, "PATCH /v1/params/settings", body);
            assert_eq!(
                (reply.status, json_of(&reply)["code"].as_str()),
                (status, Some(code))
            );
        }
        assert_eq!(*values.lock().unwrap(), before);
        assert!(api.camera_settings());
        let (busy, _) = self::api(CONTROL_BUSY);
        let reply = call(
            &busy,
            root,
            "PATCH /v1/params/settings",
            br#"{"start":"off"}"#,
        );
        assert_eq!(json_of(&reply)["code"], "camera_busy");
    }

    #[test]
    fn the_shutdown_picture_is_checked_written_and_removed() {
        let card = tempfile::tempdir().unwrap();
        fs::create_dir_all(card.path().join("GRKIT")).unwrap();
        let (api, _, checks) = api_with_checks(0);
        let missing = call(&api, card.path(), "GET /v1/shutdown-picture", b"");
        assert_eq!(missing.status, 404);
        let picture = goodbye::encode(&vec![128; goodbye::WIDTH * goodbye::HEIGHT * 3]).unwrap();
        let put = call(&api, card.path(), "PUT /v1/shutdown-picture", &picture);
        assert!(put.status == 204 && put.body.is_empty());
        // The runtime checks the new file, for the next power-off.
        assert_eq!(*checks.lock().unwrap(), 1);
        let read = call(&api, card.path(), "GET /v1/shutdown-picture", b"");
        assert_eq!(
            (read.status, read.body.as_slice()),
            (200, picture.as_slice())
        );
        assert_eq!(read.header_value("Content-Type"), Some("image/jpeg"));
        let bad = call(
            &api,
            card.path(),
            "PUT /v1/shutdown-picture",
            &picture[..1000],
        );
        assert_eq!(json_of(&bad)["code"], "invalid_picture");
        let large = vec![0; goodbye::MAX_BYTES + 1];
        assert_eq!(
            call(&api, card.path(), "PUT /v1/shutdown-picture", &large).status,
            413
        );
        assert_eq!(fs::read(card.path().join(goodbye::FILE)).unwrap(), picture);
        for _ in 0..2 {
            let removed = call(&api, card.path(), "DELETE /v1/shutdown-picture", b"");
            assert_eq!(removed.status, 204);
        }
        assert!(!card.path().join(goodbye::FILE).exists());
        // One check for the write and one for each remove; none for a refused file.
        assert_eq!(*checks.lock().unwrap(), 3);
    }
}
