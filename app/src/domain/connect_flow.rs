//! The connect flow: Bluetooth setup, access point warm-up, Wi-Fi join and
//! readiness, with the retry budgets and recovery steps of `connect_policy`.
//! Platforms supply the operations through
//! [`ConnectTransport`]; the flow owns every retry and classification.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::domain::connect_policy::{
    self as policy, JoinOutcome, NetworkSnapshot, RouteState, warmup_delay,
};
use crate::domain::{CameraModel, LinkError};

/// Checklist steps shown on the connecting sheet (A4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectStep {
    Bluetooth = 0,
    Wifi = 1,
    Version = 2,
}

impl ConnectStep {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Wi-Fi credentials read over BLE after the access point is started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WifiCredentials {
    pub ssid: String,
    pub passphrase: String,
}

/// What BLE setup returns before the phone joins Wi-Fi.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BleSetup {
    pub peripheral_id: String,
    pub model: CameraModel,
    pub powered_on_already: bool,
    pub credentials: WifiCredentials,
    /// The camera notified that its access point is up.
    pub ap_ready: bool,
}

/// Bluetooth timing used by the platform links.
pub mod timing {
    use super::Duration;

    pub const GATT_TIMEOUT: Duration = Duration::from_secs(15);
    pub const BLE_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
    pub const BLE_CONNECT_ATTEMPTS: u32 = 3;
    pub const BLE_CONNECT_RETRY_DELAY: Duration = Duration::from_secs(2);
    /// Connect timeouts retried inside one connect call.
    pub const BLE_CONNECT_TIMEOUT_RETRIES: u32 = 1;
    /// A camera no advertisement showed gets one pending connect this long,
    /// with no timeout retry: long enough for a sleeping camera's next frame,
    /// short enough that a camera left at home is answered quickly.
    pub const UNSEEN_WAKE_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
    pub const SCAN_CONNECT_SETTLE: Duration = Duration::from_millis(500);
    pub const NETWORK_TYPE_NOTIFY_TIMEOUT: Duration = Duration::from_secs(2);
    pub const NETWORK_TYPE_NOTIFY_ATTEMPTS: u32 = 2;
    pub const NETWORK_TYPE_RETRY_DELAY: Duration = Duration::from_millis(500);
}

/// How a native join ended without a terminal error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Join {
    Joined,
    /// The system reported a failure that HTTP may still disprove.
    SoftFailed(LinkError),
}

/// One readiness probe of the GRKit service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Probe {
    /// The service answered with its properties.
    Reachable,
    /// The card is still mounting (HTTP 503).
    CardNotReady,
    /// Something answered that is not camera JSON (a router page).
    NotCamera,
    /// No answer: refused or timed out. The flow tells them apart by time.
    Failed,
    /// The service answered in a way no retry fixes.
    Fatal(LinkError),
}

/// What the system says about the app's local network access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalNetworkAccess {
    Granted,
    Denied,
    /// The check could not tell; it is asked once more.
    Unknown,
}

type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// Platform or fake operations the connect flow drives.
pub trait ConnectTransport {
    fn ble_setup(&mut self, peripheral_id: &str) -> BoxFut<'_, Result<BleSetup, LinkError>>;
    /// Return once the app is in the foreground: a join from the background
    /// fails on older iOS.
    fn wait_foreground(&mut self) -> BoxFut<'_, ()>;
    /// Join the camera access point. `force` first removes the saved
    /// configuration, so the system reassociates instead of reusing a route.
    fn join_wifi(
        &mut self,
        creds: &WifiCredentials,
        force: bool,
    ) -> BoxFut<'_, Result<Join, LinkError>>;
    /// The phone's Wi-Fi interface now.
    fn network(&mut self) -> BoxFut<'_, NetworkSnapshot>;
    /// One request to the GRKit service, bounded by `timeout`.
    fn probe(&mut self, timeout: Duration) -> BoxFut<'_, Probe>;
    /// Whether the camera's own HTTP API answers, which tells a camera
    /// without the GRKit service apart from an unreachable one.
    fn camera_api_answers(&mut self) -> BoxFut<'_, bool>;
    fn read_version(&mut self, model: CameraModel) -> BoxFut<'_, Result<(), LinkError>>;
    fn pause(&mut self, duration: Duration) -> BoxFut<'_, ()>;
    /// Monotonic time that advances only while the app is in the foreground.
    fn clock(&mut self) -> Duration;
    /// The app's local network access. Platforms without such a permission
    /// keep the default.
    fn local_network_access(&mut self) -> BoxFut<'_, LocalNetworkAccess> {
        Box::pin(async { LocalNetworkAccess::Granted })
    }
}

/// A denied local network fails the attempt up front; an undecided check is
/// asked once more. A failed check never blocks the attempt.
async fn local_network_denied<T: ConnectTransport>(transport: &mut T) -> bool {
    transport.wait_foreground().await;
    let mut access = transport.local_network_access().await;
    if access == LocalNetworkAccess::Unknown {
        transport.wait_foreground().await;
        access = transport.local_network_access().await;
    }
    access == LocalNetworkAccess::Denied
}

/// What the store knows about this camera before the attempt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConnectOptions {
    /// Learned warm-up for this camera, if any.
    pub learned_warmup: Option<Duration>,
    /// No successful connect within the warm reconnect window.
    pub cold_start: bool,
    /// A fresh advertisement says the camera is on, so it needs no wake.
    pub advertised_powered_on: bool,
    /// No advertisement was seen: the Bluetooth connect is bounded to
    /// [`timing::UNSEEN_WAKE_CONNECT_TIMEOUT`] without a timeout retry.
    pub unseen: bool,
}

impl ConnectOptions {
    /// The Bluetooth connect timeout and how many timeouts it retries.
    pub fn ble_connect_budget(&self) -> (Duration, u32) {
        if self.unseen {
            (timing::UNSEEN_WAKE_CONNECT_TIMEOUT, 0)
        } else {
            (
                timing::BLE_CONNECT_TIMEOUT,
                timing::BLE_CONNECT_TIMEOUT_RETRIES,
            )
        }
    }
}

/// A connected camera and what the attempt teaches the warm-up learner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectReport {
    pub setup: BleSetup,
    pub first_join: JoinOutcome,
    pub first_join_elapsed: Duration,
    /// The first join alone reached the camera.
    pub first_join_verified: bool,
}

/// Run Bluetooth setup within the Bluetooth retry budgets.
async fn ble_stage<T: ConnectTransport>(
    transport: &mut T,
    peripheral_id: &str,
) -> Result<BleSetup, LinkError> {
    let (mut pairing, mut dropped) = (0u32, 0u32);
    loop {
        let error = match transport.ble_setup(peripheral_id).await {
            Ok(setup) => return Ok(setup),
            Err(error) => error,
        };
        let spent = pairing + dropped;
        let retry = match error {
            // The pairing prompt can still be open: give the user time.
            LinkError::BleAuthentication | LinkError::BleConnectFailed
                if pairing < policy::MAX_BLE_PAIRING_RETRIES
                    && spent < policy::MAX_BLE_FLOW_RETRIES =>
            {
                pairing += 1;
                true
            }
            LinkError::BleDisconnected
                if dropped < policy::BLE_PAIRING_REQUIRED_RETRIES
                    && spent < policy::MAX_BLE_FLOW_RETRIES =>
            {
                dropped += 1;
                true
            }
            _ => false,
        };
        if retry {
            transport.pause(policy::BLE_PAIRING_RETRY_DELAY).await;
            continue;
        }
        return Err(match error {
            LinkError::BleAuthentication => LinkError::PairingFailed,
            // A second drop leaves the pairing in doubt.
            LinkError::BleDisconnected => LinkError::NotPaired,
            LinkError::ConnectionLost => LinkError::BleConnectFailed,
            other => other,
        });
    }
}

/// What a readiness poll found.
#[derive(Clone, Debug, Default)]
struct Readiness {
    reachable: bool,
    attempts: u32,
    quick_rejects: u32,
    trailing_invalid: u32,
    any_invalid: bool,
    last_card_not_ready: bool,
    subnet_conflict: bool,
    deadline: Duration,
}

impl Readiness {
    fn quick_reject_only(&self) -> bool {
        self.attempts > 0 && self.quick_rejects == self.attempts
    }
}

/// Poll the service within `deadline` of foreground time. A run of refused
/// probes consults the route and ends early on a proven address conflict.
async fn poll_readiness<T: ConnectTransport>(
    transport: &mut T,
    deadline: Duration,
) -> Result<Readiness, LinkError> {
    let mut r = Readiness {
        deadline,
        ..Readiness::default()
    };
    let mut consults = 0;
    let start = transport.clock();
    loop {
        let remaining = deadline.saturating_sub(transport.clock() - start);
        if remaining.is_zero() {
            break;
        }
        r.attempts += 1;
        let budget = remaining.min(policy::READINESS_PROBE_TIMEOUT);
        let probe_start = transport.clock();
        let probe = transport.probe(budget).await;
        let elapsed = transport.clock() - probe_start;
        match probe {
            Probe::Reachable => {
                r.reachable = true;
                return Ok(r);
            }
            Probe::Fatal(error) => return Err(error),
            Probe::NotCamera => {
                r.trailing_invalid += 1;
                r.any_invalid = true;
            }
            Probe::CardNotReady | Probe::Failed => r.trailing_invalid = 0,
        }
        r.last_card_not_ready = probe == Probe::CardNotReady;
        if elapsed + policy::QUICK_REJECT_MARGIN < budget {
            r.quick_rejects += 1;
        }
        if consults < policy::READINESS_QUICK_REJECT_EXIT_MAX_CONSULTS
            && r.quick_rejects >= policy::READINESS_QUICK_REJECT_EXIT_REJECTS * (consults + 1)
        {
            consults += 1;
            if transport
                .network()
                .await
                .indicates_subnet_conflict(r.trailing_invalid)
            {
                r.subnet_conflict = true;
                return Ok(r);
            }
        }
        let left = deadline.saturating_sub(transport.clock() - start);
        if left.is_zero() {
            break;
        }
        transport.pause(left.min(policy::READINESS_INTERVAL)).await;
    }
    r.subnet_conflict = transport
        .network()
        .await
        .indicates_subnet_conflict(r.trailing_invalid);
    Ok(r)
}

/// The readiness budget after a join: a native `joined` keeps the full
/// budget; a soft failure off the camera subnet gets a quick probe first.
fn readiness_deadline(
    join: &Join,
    snapshot: NetworkSnapshot,
    model: CameraModel,
    rejoin: bool,
) -> Duration {
    match join {
        Join::Joined => policy::READINESS_DEADLINE,
        Join::SoftFailed(_) if !rejoin && !snapshot.on_camera_subnet(model) => {
            policy::POST_JOIN_QUICK_PROBE
        }
        Join::SoftFailed(_) if rejoin && snapshot.route(model) == RouteState::KnownOtherWifi => {
            policy::POST_JOIN_QUICK_PROBE
        }
        Join::SoftFailed(_) => policy::READINESS_DEADLINE,
    }
}

async fn join_and_poll<T: ConnectTransport>(
    transport: &mut T,
    creds: &WifiCredentials,
    model: CameraModel,
    force: bool,
    rejoin: bool,
    on_step: &mut impl FnMut(ConnectStep),
) -> Result<(Join, Duration, Readiness), LinkError> {
    on_step(ConnectStep::Wifi);
    transport.wait_foreground().await;
    let started = transport.clock();
    let join = transport.join_wifi(creds, force).await?;
    let elapsed = transport.clock() - started;
    on_step(ConnectStep::Version);
    let snapshot = transport.network().await;
    let deadline = if force {
        policy::READINESS_DEADLINE
    } else {
        readiness_deadline(&join, snapshot, model, rejoin)
    };
    let readiness = poll_readiness(transport, deadline).await?;
    if readiness.subnet_conflict {
        return Err(LinkError::SubnetConflict);
    }
    Ok((join, elapsed, readiness))
}

/// Silent verify recovery: a slower verify pass before an error shows.
async fn silent_recovery<T: ConnectTransport>(transport: &mut T) -> bool {
    for attempt in 0..policy::SILENT_VERIFY_TRIES {
        if attempt > 0 {
            transport.pause(policy::SILENT_VERIFY_DELAY).await;
        }
        if transport.probe(policy::SILENT_VERIFY_PER_TRY).await == Probe::Reachable {
            return true;
        }
    }
    false
}

/// Bluetooth → warm-up → Wi-Fi → readiness → version, reporting each step as
/// it starts. The caller bounds the whole attempt (`policy::OUTER_TIMEOUT`).
pub async fn run_connect<T: ConnectTransport>(
    transport: &mut T,
    peripheral_id: &str,
    options: ConnectOptions,
    mut on_step: impl FnMut(ConnectStep),
) -> Result<ConnectReport, LinkError> {
    on_step(ConnectStep::Bluetooth);
    if local_network_denied(transport).await {
        return Err(LinkError::LocalNetworkDenied);
    }
    let setup = ble_stage(transport, peripheral_id)
        .await
        .inspect_err(|e| app_info!("connect: bluetooth failed: {e:?}"))?;
    let model = setup.model;
    let warmup = warmup_delay(
        options.learned_warmup,
        model,
        setup.ap_ready,
        options.cold_start,
    );
    app_info!(
        "connect: bluetooth done model={model:?} ap_ready={} ssid={} warmup={warmup:?}",
        setup.ap_ready,
        setup.credentials.ssid
    );
    transport.pause(warmup).await;

    let creds = setup.credentials.clone();
    let (mut join, first_elapsed, mut readiness) =
        join_and_poll(transport, &creds, model, false, false, &mut on_step)
            .await
            .inspect_err(|e| app_info!("connect: wifi join failed: {e:?}"))?;
    app_info!(
        "connect: first join {join:?} in {first_elapsed:?}, reachable={} card_not_ready={} invalid={}",
        readiness.reachable,
        readiness.last_card_not_ready,
        readiness.any_invalid
    );
    let first_join = match join {
        Join::Joined => JoinOutcome::Joined,
        Join::SoftFailed(_) => JoinOutcome::SoftFailed,
    };
    let mut joins = 1;
    let mut reassociated = false;

    // A soft-failed join that the quick probe could not reach joins again.
    if !readiness.reachable
        && matches!(join, Join::SoftFailed(_))
        && readiness.deadline == policy::POST_JOIN_QUICK_PROBE
        && !transport.network().await.on_camera_subnet(model)
    {
        joins += 1;
        (join, _, readiness) =
            join_and_poll(transport, &creds, model, false, true, &mut on_step).await?;
        app_info!(
            "connect: rejoin {join:?}, reachable={}",
            readiness.reachable
        );
    }

    let mut route = None;
    if !readiness.reachable {
        let decision = transport.network().await.route(model);
        app_info!("connect: service not reachable, route={decision:?}");
        route = Some(decision);
        // A native `joined` without a usable route: force reassociation. On
        // GR IV a refused-only poll means another host answers, and a second
        // system dialog does not recover it.
        let eligible = join == Join::Joined
            && joins < policy::MAX_JOINS_PER_ATTEMPT
            && decision != RouteState::CameraSubnet
            && !(model == CameraModel::Gr4 && readiness.quick_reject_only());
        if eligible {
            joins += 1;
            reassociated = true;
            (join, _, readiness) =
                join_and_poll(transport, &creds, model, true, false, &mut on_step).await?;
            app_info!(
                "connect: reassociated {join:?}, reachable={}",
                readiness.reachable
            );
            if !readiness.reachable {
                route = Some(transport.network().await.route(model));
            }
        }
    }

    if !readiness.reachable {
        let route = route.unwrap_or(RouteState::Transitioning);
        let error = match &join {
            Join::SoftFailed(error) => error.clone(),
            Join::Joined if route != RouteState::CameraSubnet => LinkError::VerifyNetworkNotReady,
            Join::Joined if readiness.last_card_not_ready => LinkError::CardNotReady,
            Join::Joined if !readiness.any_invalid && transport.camera_api_answers().await => {
                LinkError::NeedsGrkit
            }
            Join::Joined => LinkError::VerifyFailed,
        };
        app_info!("connect: not ready: {error:?} (route={route:?})");
        let recover =
            error.is_transient_verify() && route != RouteState::KnownOtherWifi && !reassociated;
        if !(recover && silent_recovery(transport).await) {
            // A camera that never answered may be behind a denied permission.
            if error == LinkError::VerifyFailed && local_network_denied(transport).await {
                return Err(LinkError::LocalNetworkDenied);
            }
            return Err(error);
        }
    }

    on_step(ConnectStep::Version);
    transport
        .read_version(model)
        .await
        .inspect_err(|e| app_info!("connect: version read failed: {e:?}"))?;
    app_info!("connect: done in {joins} join(s)");
    Ok(ConnectReport {
        setup,
        first_join,
        first_join_elapsed: first_elapsed,
        first_join_verified: joins == 1 && first_join == JoinOutcome::Joined,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::net::Ipv4Addr;

    /// A scripted camera. Time advances only through pauses and probes.
    struct Script {
        ble: VecDeque<Result<(), LinkError>>,
        joins: VecDeque<Result<Join, LinkError>>,
        /// Probe answers in order; the last one repeats.
        probes: VecDeque<(Probe, Duration)>,
        network: NetworkSnapshot,
        /// The network after a forced reassociation.
        network_after_force: Option<NetworkSnapshot>,
        camera_api: bool,
        access: VecDeque<LocalNetworkAccess>,
        now: Duration,
        log: Vec<String>,
    }

    fn camera_net() -> NetworkSnapshot {
        NetworkSnapshot {
            wifi: true,
            address: Some(Ipv4Addr::new(192, 168, 0, 5)),
        }
    }

    fn home_net() -> NetworkSnapshot {
        NetworkSnapshot {
            wifi: true,
            address: Some(Ipv4Addr::new(10, 0, 0, 7)),
        }
    }

    impl Script {
        fn new() -> Self {
            Self {
                ble: VecDeque::new(),
                joins: VecDeque::from([Ok(Join::Joined)]),
                probes: VecDeque::from([(Probe::Reachable, Duration::from_millis(50))]),
                network: camera_net(),
                network_after_force: None,
                camera_api: false,
                access: VecDeque::new(),
                now: Duration::ZERO,
                log: Vec::new(),
            }
        }
    }

    impl ConnectTransport for Script {
        fn ble_setup(&mut self, peripheral_id: &str) -> BoxFut<'_, Result<BleSetup, LinkError>> {
            let peripheral_id = peripheral_id.to_owned();
            Box::pin(async move {
                self.log.push("ble".into());
                if let Some(Err(error)) = self.ble.pop_front() {
                    return Err(error);
                }
                Ok(BleSetup {
                    peripheral_id,
                    model: CameraModel::Gr3,
                    powered_on_already: false,
                    credentials: WifiCredentials {
                        ssid: "GR_AP".into(),
                        passphrase: "secret".into(),
                    },
                    ap_ready: true,
                })
            })
        }
        fn wait_foreground(&mut self) -> BoxFut<'_, ()> {
            Box::pin(async {})
        }
        fn join_wifi(
            &mut self,
            _creds: &WifiCredentials,
            force: bool,
        ) -> BoxFut<'_, Result<Join, LinkError>> {
            Box::pin(async move {
                self.log
                    .push(if force { "reassociate" } else { "join" }.into());
                if force && let Some(network) = self.network_after_force {
                    self.network = network;
                }
                self.joins.pop_front().unwrap_or(Ok(Join::Joined))
            })
        }
        fn network(&mut self) -> BoxFut<'_, NetworkSnapshot> {
            Box::pin(async { self.network })
        }
        fn probe(&mut self, timeout: Duration) -> BoxFut<'_, Probe> {
            Box::pin(async move {
                let (probe, cost) = if self.probes.len() > 1 {
                    self.probes.pop_front().unwrap()
                } else {
                    self.probes.front().cloned().unwrap()
                };
                self.now += cost.min(timeout);
                probe
            })
        }
        fn camera_api_answers(&mut self) -> BoxFut<'_, bool> {
            Box::pin(async { self.camera_api })
        }
        fn read_version(&mut self, _model: CameraModel) -> BoxFut<'_, Result<(), LinkError>> {
            Box::pin(async {
                self.log.push("version".into());
                Ok(())
            })
        }
        fn pause(&mut self, duration: Duration) -> BoxFut<'_, ()> {
            Box::pin(async move { self.now += duration })
        }
        fn clock(&mut self) -> Duration {
            self.now
        }
        fn local_network_access(&mut self) -> BoxFut<'_, LocalNetworkAccess> {
            Box::pin(async {
                self.access
                    .pop_front()
                    .unwrap_or(LocalNetworkAccess::Granted)
            })
        }
    }

    fn block_on<T>(fut: impl Future<Output = T>) -> T {
        use std::task::{Context, Poll, Waker};
        let mut fut = Box::pin(fut);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    fn run(script: &mut Script) -> Result<ConnectReport, LinkError> {
        block_on(run_connect(script, "id", ConnectOptions::default(), |_| {}))
    }

    #[test]
    fn happy_path_joins_once_and_verifies() {
        let mut s = Script::new();
        let mut steps = Vec::new();
        let report = block_on(run_connect(
            &mut s,
            "id",
            ConnectOptions::default(),
            |step| steps.push(step),
        ))
        .unwrap();
        assert!(report.first_join_verified);
        assert_eq!(s.log, ["ble", "join", "version"]);
        assert_eq!(
            steps,
            [
                ConnectStep::Bluetooth,
                ConnectStep::Wifi,
                ConnectStep::Version,
                ConnectStep::Version
            ]
        );
    }

    #[test]
    fn bluetooth_budgets_follow_the_camera_app() {
        // One drop is retried; a second asks for pairing.
        let mut s = Script::new();
        s.ble = VecDeque::from([Err(LinkError::BleDisconnected)]);
        run(&mut s).unwrap();
        let mut s = Script::new();
        s.ble = VecDeque::from([
            Err(LinkError::BleDisconnected),
            Err(LinkError::BleDisconnected),
        ]);
        assert_eq!(run(&mut s), Err(LinkError::NotPaired));
        // Authentication waits out the prompt: six retries, then a failure.
        let mut s = Script::new();
        s.ble = (0..7).map(|_| Err(LinkError::BleAuthentication)).collect();
        assert_eq!(run(&mut s), Err(LinkError::PairingFailed));
        assert_eq!(s.log.iter().filter(|l| *l == "ble").count(), 7);
        assert_eq!(s.now, policy::BLE_PAIRING_RETRY_DELAY * 6);
        // The combined budget is six: alternating reasons stop sooner.
        let mut s = Script::new();
        s.ble = VecDeque::from([
            Err(LinkError::BleDisconnected),
            Err(LinkError::BleAuthentication),
            Err(LinkError::BleAuthentication),
            Err(LinkError::BleAuthentication),
            Err(LinkError::BleAuthentication),
            Err(LinkError::BleAuthentication),
            Err(LinkError::BleAuthentication),
        ]);
        assert_eq!(run(&mut s), Err(LinkError::PairingFailed));
        assert_eq!(s.log.iter().filter(|l| *l == "ble").count(), 7);
        // A stale pairing and a timeout are not retried.
        let mut s = Script::new();
        s.ble = VecDeque::from([Err(LinkError::BlePairingStale)]);
        assert_eq!(run(&mut s), Err(LinkError::BlePairingStale));
        assert_eq!(s.log, ["ble"]);
    }

    #[test]
    fn soft_failed_join_off_subnet_rejoins_after_a_quick_probe() {
        let mut s = Script::new();
        s.joins = VecDeque::from([
            Ok(Join::SoftFailed(LinkError::WifiJoinFailed)),
            Ok(Join::Joined),
        ]);
        s.network = home_net();
        s.probes = VecDeque::from([
            (Probe::Failed, Duration::from_millis(1_500)),
            (Probe::Failed, Duration::from_millis(1_500)),
            (Probe::Reachable, Duration::from_millis(40)),
        ]);
        run(&mut s).unwrap();
        assert_eq!(s.log, ["ble", "join", "join", "version"]);
    }

    #[test]
    fn joined_without_a_route_forces_reassociation() {
        let mut s = Script::new();
        s.network = home_net();
        s.network_after_force = Some(camera_net());
        s.probes = (0..11)
            .map(|_| (Probe::Failed, Duration::from_millis(1_500)))
            .chain([(Probe::Reachable, Duration::from_millis(40))])
            .collect();
        run(&mut s).unwrap();
        assert_eq!(s.log, ["ble", "join", "reassociate", "version"]);
    }

    #[test]
    fn unreachable_camera_on_its_subnet_recovers_silently_or_fails_verify() {
        // The 15 s poll spends eight probes; the silent pass reaches it.
        let mut s = Script::new();
        let mut probes: VecDeque<_> = (0..8)
            .map(|_| (Probe::Failed, Duration::from_millis(1_500)))
            .collect();
        probes.push_back((Probe::Reachable, Duration::from_millis(40)));
        s.probes = probes;
        run(&mut s).unwrap();
        // Never reachable: verify-failed, no reassociation on the camera subnet.
        let mut s = Script::new();
        s.probes = VecDeque::from([(Probe::Failed, Duration::from_millis(1_500))]);
        assert_eq!(run(&mut s), Err(LinkError::VerifyFailed));
        assert_eq!(s.log, ["ble", "join"]);
        // The camera API answers but the GRKit service does not.
        let mut s = Script::new();
        s.probes = VecDeque::from([(Probe::Failed, Duration::from_millis(1_500))]);
        s.camera_api = true;
        assert_eq!(run(&mut s), Err(LinkError::NeedsGrkit));
    }

    #[test]
    fn foreign_network_in_the_camera_range_is_a_subnet_conflict() {
        let mut s = Script::new();
        s.network = NetworkSnapshot {
            wifi: true,
            address: Some(Ipv4Addr::new(192, 168, 0, 40)),
        };
        s.probes = VecDeque::from([(Probe::NotCamera, Duration::from_millis(20))]);
        assert_eq!(run(&mut s), Err(LinkError::SubnetConflict));
    }

    #[test]
    fn terminal_join_errors_end_the_attempt() {
        for error in [LinkError::WifiJoinDenied, LinkError::WifiWrongPassword] {
            let mut s = Script::new();
            s.joins = VecDeque::from([Err(error.clone())]);
            assert_eq!(run(&mut s), Err(error));
            assert_eq!(s.log, ["ble", "join"]);
        }
    }

    #[test]
    fn local_network_access_is_checked_before_and_after() {
        let mut s = Script::new();
        s.access = VecDeque::from([LocalNetworkAccess::Denied]);
        assert_eq!(run(&mut s), Err(LinkError::LocalNetworkDenied));
        assert!(s.log.is_empty());
        // Unknown is asked again.
        let mut s = Script::new();
        s.access = VecDeque::from([LocalNetworkAccess::Unknown, LocalNetworkAccess::Granted]);
        run(&mut s).unwrap();
        // A camera that never answered behind a denied permission.
        let mut s = Script::new();
        s.probes = VecDeque::from([(Probe::Failed, Duration::from_millis(1_500))]);
        s.access = VecDeque::from([LocalNetworkAccess::Granted, LocalNetworkAccess::Denied]);
        assert_eq!(run(&mut s), Err(LinkError::LocalNetworkDenied));
    }

    #[test]
    fn an_unseen_camera_gets_one_short_connect() {
        let seen = ConnectOptions::default();
        assert_eq!(
            seen.ble_connect_budget(),
            (
                timing::BLE_CONNECT_TIMEOUT,
                timing::BLE_CONNECT_TIMEOUT_RETRIES
            )
        );
        let unseen = ConnectOptions {
            unseen: true,
            ..ConnectOptions::default()
        };
        assert_eq!(unseen.ble_connect_budget(), (Duration::from_secs(10), 0));
        // Its timeout is the camera being away, not a pairing to retry.
        let mut s = Script::new();
        s.ble = VecDeque::from([Err(LinkError::BleTimeout)]);
        assert_eq!(
            block_on(run_connect(&mut s, "id", unseen, |_| {})),
            Err(LinkError::BleTimeout)
        );
        assert_eq!(s.log, ["ble"]);
    }

    #[test]
    fn a_still_mounting_card_is_reported() {
        let mut s = Script::new();
        s.probes = VecDeque::from([(Probe::CardNotReady, Duration::from_millis(30))]);
        assert_eq!(run(&mut s), Err(LinkError::CardNotReady));
    }
}
