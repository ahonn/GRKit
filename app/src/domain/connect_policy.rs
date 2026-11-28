//! Connection budgets and rules: retry budgets, readiness deadlines, route
//! classification and the learned access point warm-up. Each value was
//! proven on cameras in the field; change one only with field evidence.

use std::net::Ipv4Addr;
use std::time::Duration;

use crate::domain::CameraModel;

/// One connect attempt, from Bluetooth to a verified camera, counted in
/// foreground time only (the system Wi-Fi dialog suspends the app).
pub const OUTER_TIMEOUT: Duration = Duration::from_secs(90);

/// Bluetooth flow retries while the system pairing prompt is answered.
pub const MAX_BLE_PAIRING_RETRIES: u32 = 6;
/// Bluetooth flow retries after the camera drops the link during setup.
pub const BLE_PAIRING_REQUIRED_RETRIES: u32 = 1;
/// Retries across both reasons; below their sum on purpose.
pub const MAX_BLE_FLOW_RETRIES: u32 = 6;
pub const BLE_PAIRING_RETRY_DELAY: Duration = Duration::from_secs(3);

/// Readiness budget after a join, in foreground time.
pub const READINESS_DEADLINE: Duration = Duration::from_secs(15);
/// Readiness budget after a soft-failed join off the camera subnet.
pub const POST_JOIN_QUICK_PROBE: Duration = Duration::from_secs(3);
pub const READINESS_INTERVAL: Duration = Duration::from_millis(600);
pub const READINESS_PROBE_TIMEOUT: Duration = Duration::from_millis(1_500);
/// A probe that fails this much before its timeout was refused, not lost.
pub const QUICK_REJECT_MARGIN: Duration = Duration::from_millis(100);
/// Quick rejects between route consults that may end the poll early.
pub const READINESS_QUICK_REJECT_EXIT_REJECTS: u32 = 3;
pub const READINESS_QUICK_REJECT_EXIT_MAX_CONSULTS: u32 = 4;
/// Non-camera HTTP answers that prove a foreign network at the camera address.
pub const MIN_TRAILING_INVALID_RESPONSES: u32 = 2;
/// Joins per attempt, across the quick-probe rejoin and forced reassociation.
pub const MAX_JOINS_PER_ATTEMPT: u32 = 2;

/// Silent verify recovery before an error shows.
pub const SILENT_VERIFY_TRIES: u32 = 3;
pub const SILENT_VERIFY_DELAY: Duration = Duration::from_secs(3);
pub const SILENT_VERIFY_PER_TRY: Duration = Duration::from_secs(5);

/// The camera access point's /24.
pub const CAMERA_AP_PREFIX: [u8; 3] = [192, 168, 0];

/// What the phone's Wi-Fi interface says right now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NetworkSnapshot {
    /// Wi-Fi is the interface the snapshot read.
    pub wifi: bool,
    /// Its IPv4 address, when it has one.
    pub address: Option<Ipv4Addr>,
}

/// Where the phone's traffic to the camera goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteState {
    /// A lease from the camera access point.
    CameraSubnet,
    /// A settled lease on another Wi-Fi network.
    KnownOtherWifi,
    /// No settled lease yet, cellular, or unknown.
    Transitioning,
}

/// The camera DHCP pool by model. Only GR IV is known (.2 to .10); other
/// models accept any host in the /24.
fn lease_pool(model: CameraModel) -> Option<(u8, u8)> {
    match model {
        CameraModel::Gr4 => Some((2, 10)),
        _ => None,
    }
}

/// The address's last octet when it is in the camera /24.
pub fn camera_host_octet(address: Ipv4Addr) -> Option<u8> {
    let o = address.octets();
    (o[..3] == CAMERA_AP_PREFIX).then_some(o[3])
}

impl NetworkSnapshot {
    pub fn on_camera_subnet(&self, model: CameraModel) -> bool {
        let Some(octet) = self
            .address
            .filter(|_| self.wifi)
            .and_then(camera_host_octet)
        else {
            return false;
        };
        lease_pool(model).is_none_or(|(first, last)| (first..=last).contains(&octet))
    }

    /// A real lease, not "no address yet" or a self-assigned link-local one.
    fn settled_lease(&self) -> bool {
        self.address
            .is_some_and(|a| !a.is_unspecified() && !a.is_link_local())
    }

    pub fn route(&self, model: CameraModel) -> RouteState {
        if self.on_camera_subnet(model) {
            RouteState::CameraSubnet
        } else if self.wifi && self.settled_lease() {
            RouteState::KnownOtherWifi
        } else {
            RouteState::Transitioning
        }
    }

    /// A foreign network in the camera's address range keeps answering
    /// 192.168.0.1 with non-camera HTTP.
    pub fn indicates_subnet_conflict(&self, trailing_invalid_responses: u32) -> bool {
        let in_range = self.wifi && self.address.and_then(camera_host_octet).is_some();
        in_range && trailing_invalid_responses >= MIN_TRAILING_INVALID_RESPONSES
    }
}

/// Wait between the camera's AP start and the join, learned per model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WarmupProfile {
    pub default: Duration,
    pub min: Duration,
    pub max: Duration,
    pub increase: Duration,
    pub decay: Duration,
}

pub const DEFAULT_WARMUP: WarmupProfile = WarmupProfile {
    default: Duration::from_millis(1_500),
    min: Duration::from_millis(500),
    max: Duration::from_millis(6_000),
    increase: Duration::from_millis(1_000),
    decay: Duration::from_millis(500),
};

pub const GR4_WARMUP: WarmupProfile = WarmupProfile {
    default: Duration::from_millis(8_000),
    min: Duration::from_millis(5_000),
    max: Duration::from_millis(15_000),
    increase: Duration::from_millis(2_500),
    decay: Duration::from_millis(500),
};

/// A reconnect within this window finds the access point still up.
pub const WARM_RECONNECT_WINDOW: Duration = Duration::from_secs(180);
pub const WARM_WARMUP: Duration = Duration::from_millis(500);
/// Only a clean join faster than this lowers the learned warm-up.
pub const FAST_JOIN_DECAY_THRESHOLD: Duration = Duration::from_secs(6);
/// Extra wait when the camera did not report its access point ready.
pub const AP_WARMUP_MISSED_EXTRA: Duration = Duration::from_millis(1_500);

pub fn warmup_profile(model: CameraModel) -> WarmupProfile {
    match model {
        CameraModel::Gr4 => GR4_WARMUP,
        _ => DEFAULT_WARMUP,
    }
}

/// The learned estimate; a value below the floor is legacy noise.
fn effective_learned(learned: Option<Duration>, profile: WarmupProfile) -> Duration {
    learned
        .filter(|learned| *learned >= profile.min)
        .unwrap_or(profile.default)
}

/// The wait before joining.
pub fn warmup_delay(
    learned: Option<Duration>,
    model: CameraModel,
    ap_ready_notified: bool,
    cold_start: bool,
) -> Duration {
    if !cold_start {
        return WARM_WARMUP;
    }
    let profile = warmup_profile(model);
    let missed = if ap_ready_notified {
        Duration::ZERO
    } else {
        AP_WARMUP_MISSED_EXTRA
    };
    (effective_learned(learned, profile) + missed).clamp(profile.min, profile.max)
}

/// How a join ended, for the warm-up learner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinOutcome {
    Joined,
    SoftFailed,
}

/// The next learned warm-up: a soft-failed join raises it, a fast clean join
/// that alone reached the camera lowers it, anything else keeps it.
pub fn next_warmup(
    learned: Option<Duration>,
    model: CameraModel,
    outcome: JoinOutcome,
    join_elapsed: Duration,
    first_join_verified: bool,
) -> Duration {
    let profile = warmup_profile(model);
    let current = effective_learned(learned, profile);
    match outcome {
        JoinOutcome::SoftFailed => (current + profile.increase).clamp(profile.min, profile.max),
        JoinOutcome::Joined if first_join_verified && join_elapsed < FAST_JOIN_DECAY_THRESHOLD => {
            current
                .saturating_sub(profile.decay)
                .clamp(profile.min, profile.max)
        }
        JoinOutcome::Joined => current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wifi(a: [u8; 4]) -> NetworkSnapshot {
        NetworkSnapshot {
            wifi: true,
            address: Some(Ipv4Addr::from(a)),
        }
    }

    #[test]
    fn classifies_routes_like_the_camera_app() {
        assert_eq!(
            wifi([192, 168, 0, 3]).route(CameraModel::Gr4),
            RouteState::CameraSubnet
        );
        // Outside the GR IV pool: a home router on the same /24.
        assert_eq!(
            wifi([192, 168, 0, 50]).route(CameraModel::Gr4),
            RouteState::KnownOtherWifi
        );
        assert_eq!(
            wifi([192, 168, 0, 50]).route(CameraModel::Gr3),
            RouteState::CameraSubnet
        );
        assert_eq!(
            wifi([10, 0, 0, 2]).route(CameraModel::Gr3),
            RouteState::KnownOtherWifi
        );
        assert_eq!(
            wifi([169, 254, 1, 2]).route(CameraModel::Gr3),
            RouteState::Transitioning
        );
        assert_eq!(
            wifi([0, 0, 0, 0]).route(CameraModel::Gr3),
            RouteState::Transitioning
        );
        assert_eq!(
            NetworkSnapshot::default().route(CameraModel::Gr3),
            RouteState::Transitioning
        );
    }

    #[test]
    fn subnet_conflict_needs_repeated_foreign_answers() {
        assert!(!wifi([192, 168, 0, 50]).indicates_subnet_conflict(1));
        assert!(wifi([192, 168, 0, 50]).indicates_subnet_conflict(2));
        assert!(!wifi([10, 0, 0, 2]).indicates_subnet_conflict(5));
    }

    #[test]
    fn warmup_follows_the_learner() {
        let gr3 = CameraModel::Gr3;
        assert_eq!(
            warmup_delay(None, gr3, true, true),
            Duration::from_millis(1_500)
        );
        assert_eq!(
            warmup_delay(None, gr3, false, true),
            Duration::from_millis(3_000)
        );
        assert_eq!(warmup_delay(None, gr3, false, false), WARM_WARMUP);
        // Legacy values under the floor are ignored.
        assert_eq!(
            warmup_delay(Some(Duration::from_millis(100)), gr3, true, true),
            Duration::from_millis(1_500)
        );
        let raised = next_warmup(None, gr3, JoinOutcome::SoftFailed, Duration::ZERO, false);
        assert_eq!(raised, Duration::from_millis(2_500));
        let lowered = next_warmup(
            Some(raised),
            gr3,
            JoinOutcome::Joined,
            Duration::from_secs(2),
            true,
        );
        assert_eq!(lowered, Duration::from_millis(2_000));
        let kept = next_warmup(
            Some(raised),
            gr3,
            JoinOutcome::Joined,
            Duration::from_secs(9),
            true,
        );
        assert_eq!(kept, raised);
        let gr4 = next_warmup(
            None,
            CameraModel::Gr4,
            JoinOutcome::SoftFailed,
            Duration::ZERO,
            false,
        );
        assert_eq!(gr4, Duration::from_millis(10_500));
    }
}
