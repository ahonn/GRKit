//! Update state machine that mirrors the camera service.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// How long the phone waits for the confirmation on the camera.
pub const CONFIRM_DEADLINE: Duration = Duration::from_secs(5 * 60);

/// What a software request does: install a newer package.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateKind {
    Install,
}

/// Why an update stopped. A transport failure can leave its outcome unknown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateError {
    /// The card needs this many free bytes before the send can finish.
    CardFull {
        bytes_needed: u64,
    },
    Service(String),
    /// The release could not be fetched: the network or the site.
    Download(String),
    /// The camera has a module that the update does not have.
    ModuleNotKept(String),
    /// The release does not make a valid package for this camera.
    PackageInvalid,
    /// The camera refused the package.
    Refused,
    /// The user did not confirm within [`CONFIRM_DEADLINE`].
    NotConfirmed,
    /// The link dropped; reconnect and inspect the camera before retrying.
    ConnectionLost,
}

impl UpdateError {
    /// The step of the update sheet that this error stops at: 0 download,
    /// 1 send, 2 confirm on the camera.
    pub fn failed_step(&self) -> usize {
        match self {
            UpdateError::Download(_)
            | UpdateError::ModuleNotKept(_)
            | UpdateError::PackageInvalid => 0,
            UpdateError::NotConfirmed => 2,
            UpdateError::CardFull { .. }
            | UpdateError::Service(_)
            | UpdateError::Refused
            | UpdateError::ConnectionLost => 1,
        }
    }
}

/// Where a restart that the phone runs is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartStep {
    /// The phone asked the camera to turn off.
    PoweringOff,
    /// The camera is off; the phone wakes it over Bluetooth.
    PoweringOn,
    /// The camera is on; the phone joins it and reads the version.
    Reconnecting,
}

/// Phases of an update on the phone.
#[derive(Clone, Debug, PartialEq)]
pub enum UpdatePhase {
    Idle,
    /// The phone downloads and assembles the package.
    Downloading {
        progress: f32,
        kind: UpdateKind,
    },
    Sending {
        progress: f32,
        kind: UpdateKind,
    },
    WaitingConfirm {
        kind: UpdateKind,
        deadline: Instant,
    },
    Confirmed {
        kind: UpdateKind,
    },
    Restart {
        kind: UpdateKind,
    },
    /// The phone restarts the camera itself.
    Restarting {
        kind: UpdateKind,
        step: RestartStep,
    },
    /// The person restarts the camera, then reconnects.
    ManualRestart {
        kind: UpdateKind,
    },
    Installed {
        kind: UpdateKind,
        version: String,
    },
    Failed(UpdateError),
}

/// Drives [`UpdatePhase`] transitions. Progress and deadlines are fed in from
/// the camera link; this type only enforces legal moves.
#[derive(Clone, Debug)]
pub struct UpdateFlow {
    phase: UpdatePhase,
    /// Version the camera had when the flow started; used to prove errors
    /// leave the package alone.
    baseline_version: Option<String>,
}

impl UpdateFlow {
    pub fn new() -> Self {
        Self {
            phase: UpdatePhase::Idle,
            baseline_version: None,
        }
    }

    pub fn phase(&self) -> &UpdatePhase {
        &self.phase
    }

    pub fn baseline_version(&self) -> Option<&str> {
        self.baseline_version.as_deref()
    }

    /// Begin sending. `baseline` is the installed version before the send.
    pub fn begin_sending(&mut self, kind: UpdateKind, baseline: impl Into<String>) {
        self.baseline_version = Some(baseline.into());
        self.phase = UpdatePhase::Sending {
            progress: 0.0,
            kind,
        };
    }

    /// Progress while sending, clamped to 0..=1.
    pub fn set_progress(&mut self, progress: f32) {
        if let UpdatePhase::Sending { progress: p, .. } = &mut self.phase {
            *p = progress.clamp(0.0, 1.0);
        }
    }

    /// The package is on the camera and waits for confirmation.
    pub fn mark_waiting(&mut self, now: Instant) {
        let kind = match &self.phase {
            UpdatePhase::Sending { kind, .. } => *kind,
            _ => return,
        };
        self.phase = UpdatePhase::WaitingConfirm {
            kind,
            deadline: now + CONFIRM_DEADLINE,
        };
    }

    /// The camera confirmed; next the user restarts it.
    pub fn mark_confirmed(&mut self) {
        let kind = match &self.phase {
            UpdatePhase::WaitingConfirm { kind, .. } => *kind,
            _ => return,
        };
        self.phase = UpdatePhase::Confirmed { kind };
    }

    /// Show the restart step after confirmation.
    pub fn mark_restart(&mut self) {
        let kind = match &self.phase {
            UpdatePhase::Confirmed { kind } => *kind,
            UpdatePhase::WaitingConfirm { kind, .. } => *kind,
            _ => return,
        };
        self.phase = UpdatePhase::Restart { kind };
    }

    /// The new version is running.
    pub fn mark_installed(&mut self, version: impl Into<String>) {
        let kind = match &self.phase {
            UpdatePhase::Restart { kind }
            | UpdatePhase::Confirmed { kind }
            | UpdatePhase::WaitingConfirm { kind, .. } => *kind,
            _ => return,
        };
        self.phase = UpdatePhase::Installed {
            kind,
            version: version.into(),
        };
    }

    /// Fail without changing the camera package.
    pub fn fail(&mut self, error: UpdateError) {
        match &self.phase {
            UpdatePhase::Idle | UpdatePhase::Installed { .. } | UpdatePhase::Failed(_) => {}
            _ => self.phase = UpdatePhase::Failed(error),
        }
    }

    /// Expire waiting confirmation when `now` is past the deadline.
    pub fn tick_deadline(&mut self, now: Instant) {
        if let UpdatePhase::WaitingConfirm { deadline, .. } = &self.phase
            && now >= *deadline
        {
            self.fail(UpdateError::NotConfirmed);
        }
    }

    /// Back to idle after success, cancel, or dismissing an error.
    pub fn reset(&mut self) {
        self.phase = UpdatePhase::Idle;
        self.baseline_version = None;
    }

    pub fn is_busy(&self) -> bool {
        !matches!(
            self.phase,
            UpdatePhase::Idle | UpdatePhase::Installed { .. } | UpdatePhase::Failed(_)
        )
    }
}

/// The step of the update sheet a phase is at: 0 download, 1 send, 2
/// confirm on the camera, 3 restart, 4 all done.
pub fn update_step(phase: &UpdatePhase) -> usize {
    match phase {
        UpdatePhase::Idle | UpdatePhase::Downloading { .. } => 0,
        UpdatePhase::Sending { .. } => 1,
        UpdatePhase::WaitingConfirm { .. } => 2,
        UpdatePhase::Confirmed { .. }
        | UpdatePhase::Restart { .. }
        | UpdatePhase::Restarting { .. }
        | UpdatePhase::ManualRestart { .. } => 3,
        UpdatePhase::Installed { .. } => 4,
        UpdatePhase::Failed(error) => error.failed_step(),
    }
}

/// The body of the cancel confirmation at `step` ([`update_step`]).
pub fn cancel_body_key(step: usize) -> &'static str {
    match step {
        0 | 1 => "update.cancel_body_send",
        2 => "update.cancel_body_confirm",
        _ => "update.cancel_body_restart",
    }
}

/// Why the phone cannot offer an update: what the Software Update page
/// says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReleaseProblem {
    /// The release site did not answer (no internet on the phone).
    Network,
    /// The camera needs newer firmware; `required` is the newest the app
    /// supports.
    FirmwareTooOld { required: String },
    /// The camera firmware is newer than GRKit supports.
    FirmwareTooNew,
    /// The release is for a newer app: the index or the camera's builds
    /// changed in a way this app does not read.
    AppTooOld,
}

/// Whether the app can update a camera on `firmware`, given the firmware
/// versions it supports (in release order). `None` when supported.
pub fn firmware_problem(supported: &[&str], firmware: &str) -> Option<ReleaseProblem> {
    use super::version_at_least;
    if supported.contains(&firmware) {
        return None;
    }
    let newest = supported
        .iter()
        .copied()
        .reduce(|a, b| if version_at_least(b, a) { b } else { a })?;
    if version_at_least(firmware, newest) {
        Some(ReleaseProblem::FirmwareTooNew)
    } else {
        Some(ReleaseProblem::FirmwareTooOld {
            required: newest.to_owned(),
        })
    }
}

/// The room the card is short of for a package of `package_bytes`, when the
/// camera reports `free_bytes`. The camera keeps `CARD_SPARE_BYTES` free.
pub fn card_shortfall(package_bytes: u64, free_bytes: Option<u64>) -> Option<u64> {
    let need = package_bytes.saturating_add(CARD_SPARE_BYTES);
    free_bytes
        .filter(|free| *free < need)
        .map(|free| need - free)
}

/// What the camera keeps free on the card beside an update.
pub const CARD_SPARE_BYTES: u64 = 4 * 1024 * 1024;

/// The newest service protocol this app speaks (`GET /v1/ping`).
pub const SERVICE_PROTOCOL: u64 = 1;

/// A camera service that speaks a newer protocol needs a newer app.
pub fn needs_newer_app(ping_body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(ping_body)
        .ok()
        .and_then(|v| v.get("protocol")?.as_u64())
        .is_some_and(|protocol| protocol > SERVICE_PROTOCOL)
}

impl Default for UpdateFlow {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_walks_idle_to_installed() {
        let mut flow = UpdateFlow::new();
        let t0 = Instant::now();
        flow.begin_sending(UpdateKind::Install, "1.6.1");
        flow.set_progress(0.5);
        assert!(matches!(
            flow.phase(),
            UpdatePhase::Sending {
                progress,
                kind: UpdateKind::Install
            } if (*progress - 0.5).abs() < f32::EPSILON
        ));
        flow.mark_waiting(t0);
        flow.mark_confirmed();
        flow.mark_restart();
        flow.mark_installed("1.7.0");
        assert_eq!(
            flow.phase(),
            &UpdatePhase::Installed {
                kind: UpdateKind::Install,
                version: "1.7.0".into(),
            }
        );
        assert_eq!(flow.baseline_version(), Some("1.6.1"));
    }

    #[test]
    fn not_confirmed_in_time_leaves_baseline() {
        let mut flow = UpdateFlow::new();
        let t0 = Instant::now();
        flow.begin_sending(UpdateKind::Install, "1.6.1");
        flow.mark_waiting(t0);
        flow.tick_deadline(t0 + CONFIRM_DEADLINE);
        assert_eq!(
            flow.phase(),
            &UpdatePhase::Failed(UpdateError::NotConfirmed)
        );
        assert_eq!(flow.baseline_version(), Some("1.6.1"));
    }

    #[test]
    fn each_error_can_be_set_from_sending_or_waiting() {
        for error in [
            UpdateError::CardFull {
                bytes_needed: 12 * 1024 * 1024,
            },
            UpdateError::Refused,
            UpdateError::ConnectionLost,
            UpdateError::NotConfirmed,
        ] {
            let mut flow = UpdateFlow::new();
            flow.begin_sending(UpdateKind::Install, "1.6.1");
            flow.fail(error.clone());
            assert_eq!(flow.phase(), &UpdatePhase::Failed(error.clone()));
            assert_eq!(flow.baseline_version(), Some("1.6.1"));

            let mut flow = UpdateFlow::new();
            flow.begin_sending(UpdateKind::Install, "1.6.1");
            flow.mark_waiting(Instant::now());
            flow.fail(error.clone());
            assert_eq!(flow.phase(), &UpdatePhase::Failed(error));
        }
    }

    #[test]
    fn each_phase_maps_to_its_sheet_step() {
        let kind = UpdateKind::Install;
        assert_eq!(
            update_step(&UpdatePhase::Downloading {
                progress: 0.0,
                kind
            }),
            0
        );
        assert_eq!(
            update_step(&UpdatePhase::Sending {
                progress: 0.0,
                kind
            }),
            1
        );
        assert_eq!(
            update_step(&UpdatePhase::Restarting {
                kind,
                step: RestartStep::PoweringOn
            }),
            3
        );
        assert_eq!(update_step(&UpdatePhase::ManualRestart { kind }), 3);
        assert_eq!(
            update_step(&UpdatePhase::Failed(UpdateError::NotConfirmed)),
            2
        );
        assert_eq!(
            update_step(&UpdatePhase::Failed(UpdateError::Download(String::new()))),
            0
        );
        assert_eq!(cancel_body_key(3), "update.cancel_body_restart");
    }

    #[test]
    fn firmware_outside_the_list_is_too_old_or_too_new() {
        let supported = ["2.10", "2.21"];
        assert_eq!(firmware_problem(&supported, "2.21"), None);
        assert_eq!(
            firmware_problem(&supported, "2.00"),
            Some(ReleaseProblem::FirmwareTooOld {
                required: "2.21".into()
            })
        );
        assert_eq!(
            firmware_problem(&supported, "2.20"),
            Some(ReleaseProblem::FirmwareTooOld {
                required: "2.21".into()
            })
        );
        assert_eq!(
            firmware_problem(&supported, "2.30"),
            Some(ReleaseProblem::FirmwareTooNew)
        );
    }

    #[test]
    fn the_card_check_keeps_the_camera_spare() {
        assert_eq!(card_shortfall(10, None), None);
        assert_eq!(card_shortfall(10, Some(CARD_SPARE_BYTES + 10)), None);
        assert_eq!(card_shortfall(10, Some(CARD_SPARE_BYTES)), Some(10));
    }

    #[test]
    fn only_a_newer_protocol_needs_a_newer_app() {
        assert!(!needs_newer_app(r#"{"protocol":1,"bootId":"b"}"#));
        assert!(needs_newer_app(r#"{"protocol":2}"#));
        assert!(!needs_newer_app("not json"));
    }

    #[test]
    fn reset_clears_a_finished_or_failed_flow() {
        let mut flow = UpdateFlow::new();
        flow.begin_sending(UpdateKind::Install, "1.6.1");
        flow.fail(UpdateError::Refused);
        flow.reset();
        assert_eq!(flow.phase(), &UpdatePhase::Idle);
        assert!(flow.baseline_version().is_none());
    }
}
