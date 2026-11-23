//! Connect attempts: a new camera over Bluetooth, the remembered camera
//! through the network first and a Bluetooth wake after, and silent
//! connects when the phone may already be on the camera network.

use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

use nsh_core::prelude::*;

use super::{AppState, Sheet, Tab, now_ms, warmup};
use crate::domain::auto_connect::{self, AutoTrigger, LEASE_DEBOUNCE, Reach};
use crate::domain::nearby_watch::AdvertisedPower;
use crate::domain::saved_connect::{
    PEEK_TIMEOUT, Peek, SavedRoute, SavedTransport, WakePlan, plan_saved,
};
use crate::domain::{
    CameraModel, CameraStatus, ConnectOptions, ConnectStep, ConnectionState, LinkError,
    PackageInfo, Probe, UpdatePhase,
};
use crate::platform::{self, link_support};

/// Which flow an attempt runs; a retry runs the same one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AttemptKind {
    /// A camera picked from a scan: Bluetooth setup and a Wi-Fi join.
    Device,
    /// The remembered camera: the network, then a look for its
    /// advertisement, then a Bluetooth wake.
    Saved,
}

/// How an attempt reached the camera.
enum Reached {
    /// The camera answered on the network the phone is on.
    Network {
        status: CameraStatus,
        package: PackageInfo,
    },
    /// Bluetooth woke the camera and the phone joined its Wi-Fi.
    Wake(platform::Connected),
}

type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// A peek waiting for one camera's next advertisement.
pub(super) type PeekWaiter = (String, futures_channel::oneshot::Sender<AdvertisedPower>);

impl AppState {
    /// A camera from the scan list.
    pub fn select_nearby(self, peripheral_id: String) {
        if self.is_saved(&peripheral_id) {
            self.connect_camera(&peripheral_id);
            return;
        }
        self.selected_id.set(Some(peripheral_id));
        self.start_connect();
    }

    /// Connect from the nearby prompt: the remembered camera takes the
    /// saved-camera steps, any other camera is set up as new.
    pub fn connect_nearby(self) {
        let selected = self.selected_id.get();
        let remembered = self.remembered().map(|r| r.peripheral_id);
        match (selected, remembered) {
            (Some(selected), Some(remembered)) if selected.eq_ignore_ascii_case(&remembered) => {
                self.connect_saved()
            }
            _ => self.start_connect(),
        }
    }

    /// Set up the selected camera over Bluetooth.
    pub fn start_connect(self) {
        self.begin_connect(AttemptKind::Device);
    }

    /// Connect to the remembered camera, or add one when there is none.
    pub fn connect_saved(self) {
        if self.demo() || platform::has_dev_camera_url() {
            self.open_scan();
            return;
        }
        let Some(remembered) = self.remembered() else {
            self.open_scan();
            return;
        };
        // The wake needs Bluetooth, and the camera Wi-Fi is never joined
        // without it.
        if platform::bluetooth_unavailable() {
            self.set_sheet(Some(Sheet::ConnectError(LinkError::BluetoothUnavailable)));
            return;
        }
        self.suppress_nearby(&remembered.peripheral_id);
        self.selected_id.set(Some(remembered.peripheral_id));
        self.begin_connect(AttemptKind::Saved);
    }

    /// The phone may have joined the camera Wi-Fi in Settings: look for the
    /// camera on this network now.
    pub fn detect_now(self) {
        self.auto_suppressed.set(false);
        self.auto_connect(AutoTrigger::Foreground);
    }

    /// Connect to a saved camera: it becomes the most recent one.
    pub fn connect_camera(self, peripheral_id: &str) {
        if let Some(camera) = self
            .saved_cameras()
            .into_iter()
            .find(|c| c.peripheral_id.eq_ignore_ascii_case(peripheral_id))
        {
            self.remember_camera(camera);
        }
        self.connect_saved();
    }

    pub fn retry_connect(self) {
        match self.attempt_kind.get() {
            AttemptKind::Saved => self.connect_saved(),
            AttemptKind::Device => self.start_connect(),
        }
    }

    pub fn dismiss_connect_error(self) {
        self.sheet.set(None);
        self.connect_started.set(None);
    }

    /// Fence progress before aborting so late native callbacks cannot reopen the sheet.
    pub fn cancel_connect(self) {
        self.initial_connect.set(false);
        self.auto_running.set(false);
        self.connect_generation
            .update(|generation| *generation = generation.wrapping_add(1));
        self.connect_started.set(None);
        self.connect_abort.with(|slot| {
            if let Some(abort) = slot.borrow().as_ref() {
                abort.abort();
            }
        });
        self.dismiss_nearby();
    }

    /// A silent connect is looking for the camera.
    pub fn auto_connecting(self) -> bool {
        self.auto_running.get()
    }

    /// A connect is under way, shown or silent.
    pub(super) fn connect_in_flight(self) -> bool {
        self.connect_started.get().is_some()
            || self.connect_abort.with(|slot| slot.borrow().is_some())
    }

    /// The camera link is up.
    pub(super) fn link_connected(self) -> bool {
        self.status
            .get()
            .is_some_and(|s| s.camera.connection == ConnectionState::Connected)
    }

    fn begin_connect(self, kind: AttemptKind) {
        // An update waits for this connect after a restart the person ran.
        if self.update_busy.get() && !matches!(self.flow_phase(), UpdatePhase::ManualRestart { .. })
        {
            return;
        }
        // The user's own attempt replaces a silent one.
        if self.auto_running.get() {
            self.abort_auto_connect();
        }
        if self.connect_in_flight() {
            return;
        }
        self.set_frozen(false);
        self.auto_suppressed.set(false);
        self.attempt_kind.set(kind);
        if !self.initial_connect.get() {
            self.sheet.set(Some(Sheet::Connecting { step: 0 }));
        }
        self.connect_started.set(Some(Instant::now()));
        let generation = self.connect_generation.get().wrapping_add(1);
        self.connect_generation.set(generation);

        if self.demo() {
            return;
        }

        let peripheral_id = self
            .selected_id
            .get()
            .or_else(|| {
                self.nearby_cameras()
                    .first()
                    .map(|c| c.peripheral_id.clone())
            })
            .or_else(|| self.remembered().map(|r| r.peripheral_id.clone()));
        let Some(peripheral_id) = peripheral_id else {
            self.initial_connect.set(false);
            self.connect_started.set(None);
            self.sheet
                .set(Some(Sheet::ConnectError(LinkError::NotNearby)));
            return;
        };
        let model_hint = self.selected_model().unwrap_or(CameraModel::Gr3);
        app_info!("connect requested {peripheral_id} model={model_hint:?} {kind:?}");
        let (abort, registration) = futures_util::future::AbortHandle::new_pair();
        self.connect_abort
            .with(|slot| *slot.borrow_mut() = Some(abort));
        self.root.run(|| {
            spawn(async move {
                app_info!("connect task started");
                platform::begin_connect_attempt();
                let mut options = ConnectOptions {
                    advertised_powered_on: self.advertised_powered_on(&peripheral_id),
                    ..warmup::options_for(&peripheral_id)
                };
                let attempt = async {
                    if kind == AttemptKind::Saved {
                        let mut steps = SavedSteps {
                            app: self,
                            peripheral_id: &peripheral_id,
                            model: model_hint,
                            expected: self.remembered().and_then(|r| r.device_id),
                        };
                        match plan_saved(&mut steps).await {
                            SavedRoute::Direct((status, package)) => {
                                app_info!("saved camera answered on the network");
                                return Ok(Reached::Network { status, package });
                            }
                            SavedRoute::Wake(WakePlan {
                                advertised_powered_on,
                                unseen,
                            }) => {
                                app_info!(
                                    "saved camera wake powered_on={advertised_powered_on} unseen={unseen}"
                                );
                                options.advertised_powered_on = advertised_powered_on;
                                options.unseen = unseen;
                            }
                        }
                    }
                    self.wake(&peripheral_id, model_hint, options, generation)
                        .await
                        .map(Reached::Wake)
                };
                let result = futures_util::future::Abortable::new(attempt, registration).await;
                self.connect_abort.with(|slot| slot.borrow_mut().take());
                if self.connect_generation.get() != generation {
                    return;
                }
                self.initial_connect.set(false);
                match result.unwrap_or(Err(LinkError::Cancelled)) {
                    Ok(reached) => self.finish_connect(&peripheral_id, reached, options),
                    Err(LinkError::Cancelled) => {
                        self.dismiss_nearby();
                        self.connect_started.set(None);
                    }
                    Err(LinkError::NeedsGrkit) if self.update_confirmed_for(&peripheral_id) => {
                        self.sheet.set(Some(Sheet::InstallFailed));
                        self.connect_started.set(None);
                    }
                    Err(error) => {
                        let retryable = error.is_auto_retryable();
                        self.sheet.set(Some(Sheet::ConnectError(error)));
                        self.connect_started.set(None);
                        if retryable {
                            self.rearm_auto_connect();
                        }
                    }
                }
            })
        });
    }

    /// The Bluetooth wake and Wi-Fi join, bounded by foreground time; the
    /// step it was on when time ran out names the timeout.
    async fn wake(
        self,
        peripheral_id: &str,
        model_hint: CameraModel,
        options: ConnectOptions,
        generation: u64,
    ) -> Result<platform::Connected, LinkError> {
        use futures_util::future::{Either, select};
        let last_step = std::rc::Rc::new(std::cell::Cell::new(ConnectStep::Bluetooth));
        let step_seen = last_step.clone();
        let operation = platform::connect_camera(peripheral_id, model_hint, options, |step| {
            step_seen.set(step);
            if self.connect_generation.get() != generation {
                return;
            }
            if !self.initial_connect.get() {
                self.sheet
                    .set(Some(Sheet::Connecting { step: step.as_u8() }));
            }
        });
        match select(Box::pin(operation), Box::pin(connect_deadline())).await {
            Either::Left((result, _)) => result,
            Either::Right((_, operation)) => {
                platform::mark_connect_deadline();
                drop(operation);
                Err(match last_step.get() {
                    ConnectStep::Wifi => LinkError::WifiJoinTimeout,
                    _ => LinkError::BleTimeout,
                })
            }
        }
    }

    /// A connected camera, however it was reached.
    fn finish_connect(self, peripheral_id: &str, reached: Reached, options: ConnectOptions) {
        let (status, package) = match reached {
            Reached::Network { status, package } => {
                self.remember_identity(&status);
                (status, package)
            }
            Reached::Wake(connected) => {
                if let Some(report) = &connected.report {
                    warmup::learn(
                        peripheral_id,
                        connected.status.camera.model,
                        options,
                        report,
                    );
                }
                self.remember_camera(connected.remembered);
                // Steps that must follow a Bluetooth wake go here, before the
                // session starts. A woken camera waits in `bleStartup`.
                self.root.run(|| spawn(platform::http::leave_ble_startup()));
                (connected.status, connected.package)
            }
        };
        // A camera just connected to is not offered again.
        self.suppress_nearby(peripheral_id);
        self.reset_looks();
        let camera_id = peripheral_id.to_owned();
        let installed = package.installed.clone();
        self.status.set(Some(status));
        self.package.set(Some(package));
        // An update's reconnect keeps its sheet. The connect sheet ends here.
        if !self.update_busy.get() || matches!(self.sheet.get(), Some(Sheet::Connecting { .. })) {
            self.sheet.set(None);
        }
        self.connect_started.set(None);
        self.auto_suppressed.set(false);
        self.start_link_session();
        self.note_installed_version(&camera_id, &installed);
        self.refresh_update_info();
        self.resume_update();
        if let Some(tab) = self.return_tab.get() {
            self.return_tab.set(None);
            // Only from where the reconnect left the user.
            if self.tab() == Tab::Camera && tab != Tab::Camera {
                self.set_tab(tab);
            }
        }
        // The camera tab shows the Look in use and the current recipe, so
        // both load at once.
        self.refresh_looks();
        self.refresh_recipes();
    }

    /// The camera an update restarted is back: start its session as any
    /// connect does.
    pub(super) fn adopt_update_reconnect(
        self,
        peripheral_id: &str,
        connected: platform::Connected,
        options: ConnectOptions,
    ) {
        self.finish_connect(peripheral_id, Reached::Wake(connected), options);
    }

    /// Keep the remembered camera's identity current after a connect that
    /// had no Bluetooth step. Another camera body leaves the record alone.
    fn remember_identity(self, status: &CameraStatus) {
        let Some(remembered) = self.remembered() else {
            return;
        };
        let Some(answered) = status.device_id.clone() else {
            return;
        };
        if remembered
            .device_id
            .as_ref()
            .is_some_and(|known| *known != answered)
            || remembered.device_id.as_ref() == Some(&answered)
            || remembered.model != status.camera.model
        {
            return;
        }
        self.remember_camera(remembered.with_device_id(Some(answered)));
    }

    /// Connect without Bluetooth, a sheet or an error when the phone may
    /// already be on the camera network. A miss leaves everything as it was,
    /// including an error sheet it ran over.
    pub(super) fn auto_connect(self, trigger: AutoTrigger) {
        if self.demo()
            || platform::has_dev_camera_url()
            || self.link_connected()
            || self.auto_suppressed.get()
        {
            return;
        }
        if self.update_busy.get()
            || !matches!(self.flow_phase(), UpdatePhase::Idle)
            || self.connect_in_flight()
        {
            return;
        }
        match self.sheet() {
            None | Some(Sheet::Nearby | Sheet::Scan) => {}
            Some(Sheet::ConnectError(error))
                if trigger.may_run_over_error() && error.is_auto_retryable() => {}
            _ => return,
        }
        let model = self
            .remembered()
            .map(|r| r.model)
            .unwrap_or(CameraModel::Gr3);
        let generation = self.connect_generation.get().wrapping_add(1);
        self.connect_generation.set(generation);
        let (abort, registration) = futures_util::future::AbortHandle::new_pair();
        self.connect_abort
            .with(|slot| *slot.borrow_mut() = Some(abort));
        self.auto_running.set(true);
        app_info!("auto connect {trigger:?}");
        self.root.run(|| {
            spawn(async move {
                let attempt = async {
                    if !auto_connect::poll_until_reachable(&mut ServiceReach).await {
                        return None;
                    }
                    platform::http::read_package_within(model, auto_connect::VERIFY_TIMEOUT)
                        .await
                        .ok()
                };
                let result = futures_util::future::Abortable::new(attempt, registration).await;
                if self.connect_generation.get() != generation {
                    return;
                }
                self.connect_abort.with(|slot| slot.borrow_mut().take());
                self.auto_running.set(false);
                match result {
                    Ok(Some((status, package))) => {
                        app_info!("auto connect reached the camera");
                        let id = self
                            .remembered()
                            .map(|r| r.peripheral_id)
                            .unwrap_or_default();
                        self.finish_connect(
                            &id,
                            Reached::Network { status, package },
                            ConnectOptions::default(),
                        );
                    }
                    _ => app_info!("auto connect found no camera ({trigger:?})"),
                }
            })
        });
    }

    fn abort_auto_connect(self) {
        self.connect_generation
            .update(|generation| *generation = generation.wrapping_add(1));
        self.connect_abort.with(|slot| {
            if let Some(abort) = slot.borrow_mut().take() {
                abort.abort();
            }
        });
        self.auto_running.set(false);
    }

    /// A connect failed while the phone held a camera lease: probe that lease
    /// once more after the debounce.
    fn rearm_auto_connect(self) {
        if !self.lease.with(|l| l.borrow_mut().rearm(now_ms())) {
            return;
        }
        self.root.run(|| {
            spawn(async move {
                timer(LEASE_DEBOUNCE).await;
                if self.lease.with(|l| l.borrow_mut().take_due(now_ms())) {
                    self.auto_connect(AutoTrigger::SubnetRearm);
                }
            })
        });
    }

    /// Listen for this camera's advertisement within `timeout`. A frame from
    /// the last few seconds counts: the camera is advertising now.
    async fn peek(self, peripheral_id: &str, timeout: Duration) -> Peek {
        let heard = |power: AdvertisedPower| Peek {
            found: true,
            powered_on: match power {
                AdvertisedPower::On => Some(true),
                AdvertisedPower::Standby => Some(false),
                AdvertisedPower::Unknown => None,
            },
        };
        let recent = self.nearby_watch.with(|w| {
            w.borrow()
                .heard_since(peripheral_id, PEEK_TIMEOUT.as_millis() as u64, now_ms())
        });
        if let Some(power) = recent {
            return heard(power);
        }
        let (tx, rx) = futures_channel::oneshot::channel();
        self.peek_waiter
            .with(|slot| *slot.borrow_mut() = Some((peripheral_id.to_owned(), tx)));
        let answer = platform::with_timeout(rx, timeout).await;
        self.peek_waiter.with(|slot| slot.borrow_mut().take());
        match answer {
            Some(Ok(power)) => heard(power),
            _ => Peek::default(),
        }
    }

    /// A frame arrived: answer a peek waiting for this camera.
    pub(super) fn answer_peek(self, peripheral_id: &str, power: AdvertisedPower) {
        self.peek_waiter.with(|slot| {
            let waiting = slot
                .borrow()
                .as_ref()
                .is_some_and(|(id, _)| id.eq_ignore_ascii_case(peripheral_id));
            if waiting && let Some((_, tx)) = slot.borrow_mut().take() {
                let _ = tx.send(power);
            }
        });
    }
}

/// The opening steps of a saved-camera connect, on this app.
struct SavedSteps<'a> {
    app: AppState,
    peripheral_id: &'a str,
    model: CameraModel,
    /// The remembered camera body, when a connect has read it.
    expected: Option<String>,
}

impl SavedTransport for SavedSteps<'_> {
    type Camera = (CameraStatus, PackageInfo);

    fn direct(&mut self, timeout: Duration) -> BoxFut<'_, Option<Self::Camera>> {
        Box::pin(async move {
            let (status, package) = platform::http::read_package_within(self.model, timeout)
                .await
                .ok()?;
            let same = match (self.expected.as_deref(), status.device_id.as_deref()) {
                (Some(expected), Some(answered)) => expected == answered,
                _ => status.camera.model == self.model,
            };
            if !same {
                app_info!("another camera answered the saved-camera probe");
            }
            same.then_some((status, package))
        })
    }

    fn peek(&mut self, timeout: Duration) -> BoxFut<'_, Peek> {
        Box::pin(self.app.peek(self.peripheral_id, timeout))
    }
}

/// Reachability of the GRKit service, for the silent connect.
struct ServiceReach;

impl Reach for ServiceReach {
    fn reachable(&mut self, timeout: Duration) -> BoxFut<'_, bool> {
        Box::pin(async move { link_support::probe_props(timeout).await == Probe::Reachable })
    }
    fn pause(&mut self, duration: Duration) -> BoxFut<'_, ()> {
        Box::pin(timer(duration))
    }
    fn clock(&mut self) -> Duration {
        link_support::foreground_clock()
    }
}

/// Resolves after `connect_policy::OUTER_TIMEOUT` of foreground time.
async fn connect_deadline() {
    let start = link_support::foreground_clock();
    loop {
        timer(Duration::from_millis(250)).await;
        if link_support::foreground_clock() - start >= crate::domain::connect_policy::OUTER_TIMEOUT
        {
            return;
        }
    }
}
