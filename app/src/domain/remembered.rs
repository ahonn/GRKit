//! Persist the cameras the phone has joined, the most recent first.

use serde::{Deserialize, Serialize};

use crate::domain::CameraModel;

/// Saved after a successful connect so the app can reconnect later.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RememberedCamera {
    pub peripheral_id: String,
    pub model: CameraModel,
    pub last_ssid: String,
    /// The camera body's stable identity, once a connect has read it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

impl RememberedCamera {
    pub fn new(
        peripheral_id: impl Into<String>,
        model: CameraModel,
        last_ssid: impl Into<String>,
    ) -> Self {
        Self {
            peripheral_id: peripheral_id.into(),
            model,
            last_ssid: last_ssid.into(),
            device_id: None,
        }
    }

    pub fn with_device_id(mut self, device_id: Option<String>) -> Self {
        self.device_id = device_id;
        self
    }
}

/// The most cameras the app keeps.
pub const SAVED_MAX: usize = 10;

/// Serialize the saved cameras for a small file in the app data directory.
pub fn encode(cameras: &[RememberedCamera]) -> Result<String, String> {
    serde_json::to_string_pretty(cameras).map_err(|e| e.to_string())
}

/// Load the saved cameras from the file contents.
pub fn decode(text: &str) -> Result<Vec<RememberedCamera>, String> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}

/// Put `camera` first, replacing the record of the same peripheral, and keep
/// at most `SAVED_MAX` cameras.
pub fn upsert(cameras: &mut Vec<RememberedCamera>, camera: RememberedCamera) {
    cameras.retain(|c| !c.peripheral_id.eq_ignore_ascii_case(&camera.peripheral_id));
    cameras.insert(0, camera);
    cameras.truncate(SAVED_MAX);
}

/// Remove the camera with this peripheral id.
pub fn remove(cameras: &mut Vec<RememberedCamera>, peripheral_id: &str) {
    cameras.retain(|c| !c.peripheral_id.eq_ignore_ascii_case(peripheral_id));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let cams = vec![
            RememberedCamera::new("UUID-1", CameraModel::Gr4, "GR_AP")
                .with_device_id(Some("camera-1".into())),
            RememberedCamera::new("UUID-2", CameraModel::Gr3, "GR_B"),
        ];
        let text = encode(&cams).unwrap();
        assert!(text.contains("\"gr4\""));
        assert_eq!(decode(&text).unwrap(), cams);
    }

    #[test]
    fn a_record_without_an_identity_still_loads() {
        let text = r#"[{"peripheral_id":"UUID-1","model":"gr3","last_ssid":"GR_AP"}]"#;
        assert_eq!(decode(text).unwrap()[0].device_id, None);
    }

    #[test]
    fn upsert_puts_the_camera_first_once_and_keeps_ten() {
        let mut cams: Vec<_> = (0..SAVED_MAX)
            .map(|i| RememberedCamera::new(format!("ID-{i}"), CameraModel::Gr3, "GR"))
            .collect();
        upsert(
            &mut cams,
            RememberedCamera::new("id-5", CameraModel::Gr4, "GR"),
        );
        assert_eq!(cams.len(), SAVED_MAX);
        assert_eq!(cams[0].peripheral_id, "id-5");
        assert_eq!(
            cams.iter()
                .filter(|c| c.peripheral_id.eq_ignore_ascii_case("id-5"))
                .count(),
            1
        );
        upsert(
            &mut cams,
            RememberedCamera::new("NEW", CameraModel::Gr3, "GR"),
        );
        assert_eq!(cams.len(), SAVED_MAX);
        assert_eq!(cams[0].peripheral_id, "NEW");
        remove(&mut cams, "new");
        assert!(cams.iter().all(|c| c.peripheral_id != "NEW"));
    }
}
