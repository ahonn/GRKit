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
    Download(String),
    /// The camera refused the package.
    Refused,
    /// The user did not confirm within [`CONFIRM_DEADLINE`].
    NotConfirmed,
    /// The link dropped; reconnect and inspect the camera before retrying.
    ConnectionLost,
}

/// Phases of an update on the phone.
#[derive(Clone, Debug, PartialEq)]
pub enum UpdatePhase {
    Idle,
    Sending { progress: f32, kind: UpdateKind },
    WaitingConfirm { kind: UpdateKind, deadline: Instant },
    Confirmed { kind: UpdateKind },
    Restart { kind: UpdateKind },
    Installed { kind: UpdateKind, version: String },
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
    fn reset_clears_a_finished_or_failed_flow() {
        let mut flow = UpdateFlow::new();
        flow.begin_sending(UpdateKind::Install, "1.6.1");
        flow.fail(UpdateError::Refused);
        flow.reset();
        assert_eq!(flow.phase(), &UpdatePhase::Idle);
        assert!(flow.baseline_version().is_none());
    }
}
