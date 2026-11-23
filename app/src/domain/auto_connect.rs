//! Silent connects over the network alone: when the phone may already be on
//! the camera network, a short reachability poll and one read reconnect
//! without Bluetooth, a sheet, or an error. Also the watch that notices the
//! phone getting a lease from a camera network.

use std::future::Future;
use std::net::Ipv4Addr;
use std::pin::Pin;
use std::time::Duration;

use super::CameraModel;
use super::connect_policy::{NetworkSnapshot, camera_host_octet};

/// The reachability poll of an automatic connect. Far shorter than the
/// readiness budget of a join, because it runs speculatively.
pub const PROBE_DEADLINE: Duration = Duration::from_secs(3);
pub const PROBE_INTERVAL: Duration = Duration::from_millis(600);
pub const PROBE_TIMEOUT: Duration = Duration::from_millis(1_500);
/// The read that confirms the camera once it is reachable.
pub const VERIFY_TIMEOUT: Duration = Duration::from_secs(2);
/// A new camera lease must hold this long before it is probed.
pub const LEASE_DEBOUNCE: Duration = Duration::from_millis(1_500);
/// The least time between two probes of the same lease.
pub const LEASE_COOLDOWN: Duration = Duration::from_secs(10);

/// What started an automatic connect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoTrigger {
    Launch,
    Foreground,
    /// The phone got a lease from a camera network.
    Subnet,
    /// That lease was already there when a connect failed.
    SubnetRearm,
    /// The link went down without the user asking.
    UnexpectedDrop,
}

impl AutoTrigger {
    /// A camera lease appearing is new evidence about the network a standing
    /// error described, so only those triggers may run over its sheet.
    pub fn may_run_over_error(self) -> bool {
        matches!(self, AutoTrigger::Subnet | AutoTrigger::SubnetRearm)
    }
}

type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// The operations of the reachability poll.
pub trait Reach {
    /// One request to the GRKit service within `timeout`.
    fn reachable(&mut self, timeout: Duration) -> BoxFut<'_, bool>;
    fn pause(&mut self, duration: Duration) -> BoxFut<'_, ()>;
    fn clock(&mut self) -> Duration;
}

/// Probe until the service answers or [`PROBE_DEADLINE`] passes. At most
/// `deadline / interval + 1` requests, each bounded by the time left.
pub async fn poll_until_reachable<T: Reach>(transport: &mut T) -> bool {
    let max_attempts = PROBE_DEADLINE
        .as_millis()
        .div_ceil(PROBE_INTERVAL.as_millis()) as u32
        + 1;
    let start = transport.clock();
    for _ in 0..max_attempts {
        let remaining = PROBE_DEADLINE.saturating_sub(transport.clock() - start);
        if remaining.is_zero() {
            break;
        }
        if transport.reachable(remaining.min(PROBE_TIMEOUT)).await {
            return true;
        }
        let left = PROBE_DEADLINE.saturating_sub(transport.clock() - start);
        if left.is_zero() {
            break;
        }
        transport.pause(left.min(PROBE_INTERVAL)).await;
    }
    false
}

/// What one look at the Wi-Fi interface says about a camera lease.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaseObservation {
    /// A lease one of the known camera models could hand out.
    Candidate(Ipv4Addr),
    /// A settled lease on another network.
    DefiniteAway,
    /// No Wi-Fi, no address yet, or a self-assigned one: often route churn
    /// while the camera association holds.
    AmbiguousAway,
}

/// Classify a snapshot. With no known model, any camera /24 lease counts.
pub fn observe_lease(snapshot: NetworkSnapshot, models: &[CameraModel]) -> LeaseObservation {
    let Some(address) = snapshot.address.filter(|_| snapshot.wifi) else {
        return LeaseObservation::AmbiguousAway;
    };
    if address.is_unspecified() || address.is_link_local() {
        return LeaseObservation::AmbiguousAway;
    }
    let candidate = if models.is_empty() {
        camera_host_octet(address).is_some()
    } else {
        models.iter().any(|model| snapshot.on_camera_subnet(*model))
    };
    if candidate {
        LeaseObservation::Candidate(address)
    } else {
        LeaseObservation::DefiniteAway
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Lease {
    generation: u64,
    address: Ipv4Addr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Presence {
    Away,
    Candidate(Lease),
    /// The lease went missing at `since_ms`; `definite` once another network
    /// was seen.
    Gap {
        generation: u64,
        since_ms: u64,
        definite: bool,
    },
}

/// Tracks camera leases across samples of the Wi-Fi interface. A lease that
/// comes back after a short gap keeps its generation, so route churn does not
/// read as a new network; each new lease is probed once, after a debounce,
/// and the same lease never more often than the cooldown allows.
#[derive(Clone, Debug)]
pub struct LeaseWatch {
    presence: Presence,
    next_generation: u64,
    /// A probe waiting for its debounce: the lease and when it was scheduled.
    pending: Option<(Lease, u64)>,
    last_probe: Option<(Lease, u64)>,
    rearmed: Option<u64>,
}

impl Default for LeaseWatch {
    fn default() -> Self {
        Self {
            presence: Presence::Away,
            next_generation: 0,
            pending: None,
            last_probe: None,
            rearmed: None,
        }
    }
}

impl LeaseWatch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply one sample. `disconnected` is false while a camera is connected
    /// or a connect runs; no probe is scheduled then. Returns whether a probe
    /// is pending its debounce.
    pub fn observe(
        &mut self,
        observation: LeaseObservation,
        disconnected: bool,
        now_ms: u64,
    ) -> bool {
        let address = match observation {
            LeaseObservation::Candidate(address) => address,
            away => {
                self.pending = None;
                let definite = away == LeaseObservation::DefiniteAway;
                self.presence = match self.presence {
                    Presence::Candidate(lease) => Presence::Gap {
                        generation: lease.generation,
                        since_ms: now_ms,
                        definite,
                    },
                    Presence::Gap {
                        generation,
                        since_ms,
                        definite: was,
                    } => Presence::Gap {
                        generation,
                        since_ms,
                        definite: was || definite,
                    },
                    Presence::Away => Presence::Away,
                };
                return false;
            }
        };
        let generation = match self.presence {
            Presence::Away => self.new_generation(),
            Presence::Gap {
                generation,
                since_ms,
                definite,
            } => {
                let left = definite
                    || now_ms.saturating_sub(since_ms) >= LEASE_DEBOUNCE.as_millis() as u64;
                if left {
                    self.new_generation()
                } else {
                    generation
                }
            }
            Presence::Candidate(lease) => lease.generation,
        };
        let lease = Lease {
            generation,
            address,
        };
        let changed = self.presence != Presence::Candidate(lease);
        self.presence = Presence::Candidate(lease);
        if !disconnected {
            self.pending = None;
        } else if changed {
            self.pending = Some((lease, now_ms));
        }
        self.pending.is_some()
    }

    /// The pending probe once its debounce passed, the lease still holds and
    /// the cooldown allows it. Taking it starts the cooldown.
    pub fn take_due(&mut self, now_ms: u64) -> bool {
        let Some((lease, scheduled)) = self.pending else {
            return false;
        };
        if now_ms.saturating_sub(scheduled) < LEASE_DEBOUNCE.as_millis() as u64 {
            return false;
        }
        self.pending = None;
        if self.presence != Presence::Candidate(lease) {
            return false;
        }
        if self.last_probe.is_some_and(|(last, at)| {
            last == lease && now_ms.saturating_sub(at) < LEASE_COOLDOWN.as_millis() as u64
        }) {
            return false;
        }
        self.last_probe = Some((lease, now_ms));
        true
    }

    /// A connect failed while a camera lease was present: probe that lease
    /// again, once per lease. Returns whether a probe is now pending.
    pub fn rearm(&mut self, now_ms: u64) -> bool {
        let Presence::Candidate(lease) = self.presence else {
            return false;
        };
        if self.rearmed == Some(lease.generation) {
            return false;
        }
        self.rearmed = Some(lease.generation);
        self.pending = Some((lease, now_ms));
        true
    }

    fn new_generation(&mut self) -> u64 {
        self.next_generation += 1;
        self.next_generation
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

    const CAMERA: [u8; 4] = [192, 168, 0, 3];

    fn candidate() -> LeaseObservation {
        LeaseObservation::Candidate(Ipv4Addr::from(CAMERA))
    }

    #[test]
    fn leases_are_classified_by_the_known_models() {
        let gr4 = [CameraModel::Gr4];
        assert_eq!(observe_lease(wifi(CAMERA), &gr4), candidate());
        // Outside the GR IV pool: a home network on the same /24.
        assert_eq!(
            observe_lease(wifi([192, 168, 0, 50]), &gr4),
            LeaseObservation::DefiniteAway
        );
        assert!(matches!(
            observe_lease(wifi([192, 168, 0, 50]), &[]),
            LeaseObservation::Candidate(_)
        ));
        assert_eq!(
            observe_lease(wifi([10, 0, 0, 2]), &gr4),
            LeaseObservation::DefiniteAway
        );
        assert_eq!(
            observe_lease(wifi([169, 254, 1, 1]), &gr4),
            LeaseObservation::AmbiguousAway
        );
        assert_eq!(
            observe_lease(NetworkSnapshot::default(), &gr4),
            LeaseObservation::AmbiguousAway
        );
    }

    #[test]
    fn a_new_lease_is_probed_once_after_the_debounce() {
        let mut watch = LeaseWatch::new();
        assert!(watch.observe(candidate(), true, 0));
        assert!(!watch.take_due(1_000));
        // The same lease again does not restart or repeat anything.
        assert!(watch.observe(candidate(), true, 1_200));
        assert!(watch.take_due(1_500));
        assert!(!watch.observe(candidate(), true, 5_000));
        assert!(!watch.take_due(20_000));
    }

    #[test]
    fn a_lease_that_left_before_the_debounce_is_not_probed() {
        let mut watch = LeaseWatch::new();
        watch.observe(candidate(), true, 0);
        watch.observe(LeaseObservation::AmbiguousAway, true, 1_000);
        assert!(!watch.take_due(2_000));
    }

    #[test]
    fn a_short_gap_keeps_the_lease_and_a_real_leave_starts_a_new_one() {
        let mut watch = LeaseWatch::new();
        watch.observe(candidate(), true, 0);
        assert!(watch.take_due(1_500));
        // Route churn for half a second: the same lease, held back by the
        // cooldown.
        watch.observe(LeaseObservation::AmbiguousAway, true, 2_000);
        assert!(watch.observe(candidate(), true, 2_500));
        assert!(!watch.take_due(4_000));
        // Another network in between: a new lease, probed again once the
        // cooldown of the old one no longer matters.
        watch.observe(LeaseObservation::DefiniteAway, true, 3_000);
        assert!(watch.observe(candidate(), true, 3_500));
        assert!(watch.take_due(5_000));
    }

    #[test]
    fn the_same_lease_is_not_probed_within_the_cooldown() {
        let mut watch = LeaseWatch::new();
        watch.observe(candidate(), true, 0);
        assert!(watch.take_due(1_500));
        assert!(watch.rearm(2_000));
        assert!(!watch.take_due(4_000));
        // Once per lease.
        assert!(!watch.rearm(20_000));
    }

    #[test]
    fn a_rearm_after_the_cooldown_probes_again() {
        let mut watch = LeaseWatch::new();
        watch.observe(candidate(), true, 0);
        assert!(watch.take_due(1_500));
        assert!(watch.rearm(20_000));
        assert!(watch.take_due(21_500));
    }

    #[test]
    fn nothing_is_scheduled_while_connected() {
        let mut watch = LeaseWatch::new();
        assert!(!watch.observe(candidate(), false, 0));
        assert!(!watch.take_due(5_000));
        // Still the same lease after the link drops: no new edge. The drop
        // itself starts its own automatic connect.
        assert!(!watch.observe(candidate(), true, 6_000));
    }

    struct Clocked {
        answers: Vec<(bool, Duration)>,
        now: Duration,
        requests: u32,
    }

    impl Reach for Clocked {
        fn reachable(&mut self, timeout: Duration) -> BoxFut<'_, bool> {
            Box::pin(async move {
                self.requests += 1;
                let (ok, cost) = if self.answers.len() > 1 {
                    self.answers.remove(0)
                } else {
                    self.answers[0]
                };
                self.now += cost.min(timeout);
                ok
            })
        }
        fn pause(&mut self, duration: Duration) -> BoxFut<'_, ()> {
            Box::pin(async move { self.now += duration })
        }
        fn clock(&mut self) -> Duration {
            self.now
        }
    }

    fn run(answers: Vec<(bool, Duration)>) -> (bool, Clocked) {
        let mut transport = Clocked {
            answers,
            now: Duration::ZERO,
            requests: 0,
        };
        let reached = crate::domain::link::block_on_ready(poll_until_reachable(&mut transport));
        (reached, transport)
    }

    #[test]
    fn the_poll_stays_within_its_deadline_and_attempts() {
        // Refused at once: one request per interval.
        let (reached, t) = run(vec![(false, Duration::ZERO)]);
        assert!(!reached);
        assert_eq!(t.requests, 5);
        assert!(t.now <= PROBE_DEADLINE);
        // Timing out: each probe spends its timeout.
        let (reached, t) = run(vec![(false, Duration::from_secs(5))]);
        assert!(!reached);
        assert_eq!(t.requests, 2);
        assert_eq!(t.now, PROBE_DEADLINE);
        let (reached, t) = run(vec![(false, Duration::ZERO), (true, Duration::ZERO)]);
        assert!(reached);
        assert_eq!(t.requests, 2);
    }
}
