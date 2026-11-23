//! Health of an established camera link: ready while probes answer, degraded
//! after a failed probe, recovering after a run of them, and down once the
//! camera is gone. HTTP decides; a Bluetooth or network loss is only a hint
//! that makes the next probe come sooner, except where the route itself is
//! gone.

use std::time::Duration;

use super::{CameraModel, PropsResponse};

/// How often a connected camera is probed in the foreground.
pub const LOOP_INTERVAL: Duration = Duration::from_secs(5);
pub const WARM_PROBE_TIMEOUT: Duration = Duration::from_secs(10);
/// Failed warm probes in a row before one recovery probe decides.
pub const MAX_WEAK_FAILURES: u32 = 4;
/// After a return to the foreground the old routes may be gone: fail fast.
pub const FOREGROUND_RESUME_PROBE_TIMEOUT: Duration = Duration::from_millis(2_500);
pub const FOREGROUND_RESUME_RETRY_DELAY: Duration = Duration::from_millis(500);
pub const FOREGROUND_RESUME_TRIES: u32 = 2;

/// Where the camera link stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LinkState {
    /// No session.
    #[default]
    Idle,
    Ready,
    /// A probe failed; the camera still counts as online.
    Degraded,
    /// Enough probes failed that one more decides.
    Recovering,
    Down,
}

impl LinkState {
    /// Camera work may go on.
    pub fn online(self) -> bool {
        matches!(self, LinkState::Ready | LinkState::Degraded)
    }
}

/// What one probe of the connected camera found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeOutcome {
    Ok,
    /// Another camera answered at the camera address.
    Mismatch,
    Failed,
}

/// Why the link went down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownReason {
    /// The camera reported that it powered off.
    PoweredOff,
    IdentityMismatch,
    /// The probes after a return to the foreground failed.
    ResumeFailed,
    RecoveryExhausted,
    /// The phone dropped the camera network.
    NetworkLost,
    /// The user disconnected or turned the camera off.
    User,
}

impl DownReason {
    /// Every loss but a power-off or the user's own is unexpected: it is reported and the app
    /// tries once to reach the camera again.
    pub fn unexpected(self) -> bool {
        !matches!(self, DownReason::PoweredOff | DownReason::User)
    }
}

/// What the caller does after a probe result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    Continue,
    /// Probe once more; its result goes to [`LinkHealth::recovered`].
    Recover,
    Down(DownReason),
}

/// The probe policy of one camera session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinkHealth {
    state: LinkState,
    weak_failures: u32,
}

impl LinkHealth {
    /// A session that just connected.
    pub fn ready() -> Self {
        Self {
            state: LinkState::Ready,
            weak_failures: 0,
        }
    }

    pub fn state(&self) -> LinkState {
        self.state
    }

    /// The result of a periodic probe.
    pub fn warm_probe(&mut self, outcome: ProbeOutcome) -> Next {
        match outcome {
            ProbeOutcome::Ok => {
                self.set_ready();
                Next::Continue
            }
            ProbeOutcome::Mismatch => self.down(DownReason::IdentityMismatch),
            ProbeOutcome::Failed => {
                self.weak_failures += 1;
                if self.weak_failures >= MAX_WEAK_FAILURES {
                    self.state = LinkState::Recovering;
                    Next::Recover
                } else {
                    self.state = LinkState::Degraded;
                    Next::Continue
                }
            }
        }
    }

    /// The result of the recovery probe.
    pub fn recovered(&mut self, outcome: ProbeOutcome) -> Next {
        if outcome == ProbeOutcome::Ok {
            self.set_ready();
            Next::Continue
        } else {
            self.down(DownReason::RecoveryExhausted)
        }
    }

    /// The result of the probes made at a return to the foreground.
    pub fn resumed(&mut self, outcome: ProbeOutcome) -> Next {
        match outcome {
            ProbeOutcome::Ok => {
                self.set_ready();
                Next::Continue
            }
            ProbeOutcome::Mismatch => self.down(DownReason::IdentityMismatch),
            ProbeOutcome::Failed => self.down(DownReason::ResumeFailed),
        }
    }

    pub fn down(&mut self, reason: DownReason) -> Next {
        self.state = LinkState::Down;
        self.weak_failures = 0;
        Next::Down(reason)
    }

    fn set_ready(&mut self) {
        self.state = LinkState::Ready;
        self.weak_failures = 0;
    }
}

/// Whether `/v1/props` came from the camera of this session. A stable
/// identity decides when both sides have one; without it, a different model
/// is the only proof of another camera.
pub fn same_camera(props: &PropsResponse, expected: Option<&str>, model: CameraModel) -> bool {
    if let (Some(expected), Some(answered)) = (expected, props.device_id.as_deref()) {
        return expected == answered;
    }
    props
        .camera
        .as_ref()
        .and_then(|camera| camera.model.as_deref())
        .and_then(super::http_api::model_from_name)
        .is_none_or(|answered| answered == model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::parse_props;

    #[test]
    fn failed_probes_degrade_then_recover_or_go_down() {
        let mut health = LinkHealth::ready();
        for _ in 0..MAX_WEAK_FAILURES - 1 {
            assert_eq!(health.warm_probe(ProbeOutcome::Failed), Next::Continue);
            assert_eq!(health.state(), LinkState::Degraded);
            assert!(health.state().online());
        }
        assert_eq!(health.warm_probe(ProbeOutcome::Failed), Next::Recover);
        assert_eq!(health.state(), LinkState::Recovering);
        assert!(!health.state().online());
        assert_eq!(health.recovered(ProbeOutcome::Ok), Next::Continue);
        assert_eq!(health.state(), LinkState::Ready);
        // The count starts again after a recovery.
        assert_eq!(health.warm_probe(ProbeOutcome::Failed), Next::Continue);
        let mut health = LinkHealth::ready();
        for _ in 0..MAX_WEAK_FAILURES {
            health.warm_probe(ProbeOutcome::Failed);
        }
        assert_eq!(
            health.recovered(ProbeOutcome::Mismatch),
            Next::Down(DownReason::RecoveryExhausted)
        );
        assert_eq!(health.state(), LinkState::Down);
    }

    #[test]
    fn a_good_probe_clears_the_failures() {
        let mut health = LinkHealth::ready();
        for _ in 0..MAX_WEAK_FAILURES - 1 {
            health.warm_probe(ProbeOutcome::Failed);
        }
        health.warm_probe(ProbeOutcome::Ok);
        assert_eq!(health.state(), LinkState::Ready);
        for _ in 0..MAX_WEAK_FAILURES - 1 {
            assert_eq!(health.warm_probe(ProbeOutcome::Failed), Next::Continue);
        }
    }

    #[test]
    fn another_camera_or_a_failed_resume_ends_the_link_at_once() {
        let mut health = LinkHealth::ready();
        assert_eq!(
            health.warm_probe(ProbeOutcome::Mismatch),
            Next::Down(DownReason::IdentityMismatch)
        );
        let mut health = LinkHealth::ready();
        assert_eq!(
            health.resumed(ProbeOutcome::Failed),
            Next::Down(DownReason::ResumeFailed)
        );
        assert!(DownReason::ResumeFailed.unexpected());
        assert!(!DownReason::PoweredOff.unexpected());
    }

    fn props(device_id: Option<&str>, model: Option<&str>) -> PropsResponse {
        let mut body = serde_json::json!({"bootId":"b","grkit":{"version":"1.7.0"}});
        if let Some(id) = device_id {
            body["deviceId"] = id.into();
        }
        if let Some(model) = model {
            body["camera"] = serde_json::json!({"model": model});
        }
        parse_props(&body.to_string()).unwrap()
    }

    #[test]
    fn the_stable_identity_decides_when_both_sides_have_one() {
        let gr3 = CameraModel::Gr3;
        assert!(same_camera(
            &props(Some("camera-a"), None),
            Some("camera-a"),
            gr3
        ));
        // Same model, different body.
        assert!(!same_camera(
            &props(Some("camera-b"), Some("RICOH GR III")),
            Some("camera-a"),
            gr3
        ));
        // No identity on either side: the model decides.
        assert!(same_camera(
            &props(None, Some("RICOH GR III")),
            Some("camera-a"),
            gr3
        ));
        assert!(!same_camera(
            &props(None, Some("RICOH GR IV")),
            Some("camera-a"),
            gr3
        ));
        assert!(same_camera(&props(Some("camera-b"), None), None, gr3));
    }
}
