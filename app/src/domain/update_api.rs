//! Wire state and persisted intent for an update. A missing journal is not proof of success.
use super::{UpdateError, UpdateKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JournalState {
    None,
    Waiting,
    Confirmed,
}

#[derive(Clone, Debug, Deserialize)]
pub struct InventoryPackage {
    pub version: Option<String>,
    pub modules: Option<serde_json::Value>,
    pub effects: Option<serde_json::Value>,
    pub left_out: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryBody {
    pub package: InventoryPackage,
    pub state: JournalState,
}

/// A software request of the card: the `request` of `GET /v1/update`, and
/// the answer of a stage or a cancel.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareRequestSummary {
    pub request_id: String,
    pub domain: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub phase: Option<String>,
}

impl SoftwareRequestSummary {
    /// The request waits on the camera: for its confirmation or a restart.
    pub fn waits(&self) -> bool {
        self.status.as_deref() == Some("running")
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResource {
    pub inventory: InventoryBody,
    #[serde(default)]
    pub request: Option<SoftwareRequestSummary>,
}

#[derive(Clone, Debug)]
pub struct UpdateSnapshot {
    pub state: JournalState,
    pub package: super::PackageResponse,
    /// The update request that waits, or the last one that ended.
    pub request: Option<SoftwareRequestSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateIntent {
    pub camera_id: String,
    pub kind: UpdateKind,
    pub baseline: String,
    /// The version the update installs; empty for an update another phone
    /// or the website sent, whose version the camera does not report.
    pub target: String,
    /// The camera confirmed the update: the next start installs it.
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateObservation {
    Waiting,
    Restart,
    Installed,
    Stopped,
}

/// What the camera's journal says about `intent`, with `installed` the
/// version the camera runs now. Without a known target, a version other
/// than the baseline is the installed update.
pub fn observe_update(
    state: JournalState,
    installed: &str,
    intent: &UpdateIntent,
) -> UpdateObservation {
    match state {
        JournalState::Waiting => UpdateObservation::Waiting,
        JournalState::Confirmed => UpdateObservation::Restart,
        JournalState::None if installed == intent.target => UpdateObservation::Installed,
        JournalState::None if intent.target.is_empty() && installed != intent.baseline => {
            UpdateObservation::Installed
        }
        JournalState::None => UpdateObservation::Stopped,
    }
}

pub fn response_error(status: u16, body: &str) -> UpdateError {
    let parsed = serde_json::from_str::<serde_json::Value>(body).ok();
    let message = parsed
        .as_ref()
        .and_then(|v| {
            v.get("detail")
                .and_then(|v| v.as_str())
                .or_else(|| v.get("error").and_then(|v| v.as_str()))
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("Camera returned HTTP {status}."));
    if let Some(bytes) = message
        .strip_prefix("The card needs ")
        .and_then(|s| s.strip_suffix(" more bytes for the update"))
        .and_then(|s| s.parse::<u64>().ok())
    {
        UpdateError::CardFull {
            bytes_needed: bytes,
        }
    } else {
        UpdateError::Service(message)
    }
}

/// The `code` of a problem document.
pub fn problem_code(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get("code")?
        .as_str()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_missing_journal_only_succeeds_at_the_target_version() {
        let intent = |target: &str| UpdateIntent {
            camera_id: "c".into(),
            kind: UpdateKind::Install,
            baseline: "1.7.0".into(),
            target: target.into(),
            confirmed: false,
        };
        let known = intent("1.8.0");
        assert_eq!(
            observe_update(JournalState::None, "1.7.0", &known),
            UpdateObservation::Stopped
        );
        assert_eq!(
            observe_update(JournalState::None, "1.8.0", &known),
            UpdateObservation::Installed
        );
        assert_eq!(
            observe_update(JournalState::Confirmed, "1.8.0", &known),
            UpdateObservation::Restart
        );
        assert_eq!(
            observe_update(JournalState::Waiting, "1.8.0", &known),
            UpdateObservation::Waiting
        );
        // An update another client sent: any newer start is the install.
        let unknown = intent("");
        assert_eq!(
            observe_update(JournalState::None, "1.7.0", &unknown),
            UpdateObservation::Stopped
        );
        assert_eq!(
            observe_update(JournalState::None, "1.8.0", &unknown),
            UpdateObservation::Installed
        );
    }
    #[test]
    fn maps_service_errors_without_inventing_free_space() {
        assert_eq!(
            response_error(
                400,
                r#"{"error":"The card needs 123 more bytes for the update"}"#
            ),
            UpdateError::CardFull { bytes_needed: 123 }
        );
        assert!(
            matches!(response_error(409, r#"{"error":"No update"}"#), UpdateError::Service(s) if s == "No update")
        );
        assert!(matches!(
            response_error(
                409,
                r#"{"type":"about:blank","title":"Conflict","status":409,"code":"software_pending","detail":"Finish the pending request."}"#
            ),
            UpdateError::Service(s) if s == "Finish the pending request."
        ));
    }
    #[test]
    fn reads_the_problem_code() {
        assert_eq!(
            problem_code(r#"{"status":409,"code":"software_pending","detail":"x"}"#).as_deref(),
            Some("software_pending")
        );
        assert_eq!(problem_code(r#"{"error":"x"}"#), None);
        assert_eq!(problem_code("not json"), None);
    }
    #[test]
    fn a_running_request_waits() {
        let summary = |status: &str| -> SoftwareRequestSummary {
            serde_json::from_value(serde_json::json!({
                "requestId": "r", "domain": "update", "bootId": "b", "cardId": "c",
                "status": status, "phase": "awaitingConfirmation", "progress": null,
                "cancellable": true, "requiredAction": "confirmOnCamera",
                "result": null, "error": null
            }))
            .unwrap()
        };
        assert!(summary("running").waits());
        assert!(!summary("cancelled").waits());
    }
    #[test]
    fn an_intent_saved_with_a_request_id_still_loads() {
        let intent: UpdateIntent = serde_json::from_str(
            r#"{"camera_id":"c","kind":"Install","baseline":"1.7.0","target":"1.8.0","request_id":"r"}"#,
        )
        .unwrap();
        assert_eq!(intent.target, "1.8.0");
    }
}
