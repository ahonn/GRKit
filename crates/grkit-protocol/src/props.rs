//! The fields of `GET /v1/props` that the app reads. The service sends more
//! fields, such as its capabilities and limits.
use serde::{Deserialize, Serialize};

/// The answer of `GET /v1/props`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropsResponse {
    /// A stable identity of the camera body (a hash of its serial number);
    /// absent until the camera's own API has answered the service.
    #[serde(default)]
    pub device_id: Option<String>,
    pub boot_id: String,
    pub card_id: Option<String>,
    pub camera: Option<PropsCamera>,
    pub grkit: PropsGrkit,
    #[serde(default)]
    pub storage: Option<PropsStorage>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropsCamera {
    pub model: Option<String>,
    #[serde(alias = "firmware")]
    pub firmware_version: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropsGrkit {
    pub version: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropsStorage {
    pub state: Option<String>,
    /// Free bytes on the card, when the service could read them.
    #[serde(default)]
    pub free_bytes: Option<u64>,
}
