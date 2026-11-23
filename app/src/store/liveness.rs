//! Health of the camera link and what happens around it: HTTP decides
//! whether the camera is still there, a recent frame of the change stream
//! makes a periodic probe wait, a Bluetooth disconnect or a return to the
//! foreground makes the next probe come at once, and a lost camera never
//! brings up a system Wi-Fi dialog. While no camera is connected, the same
//! loop watches for the phone getting a lease from a camera network.

use nsh_core::platform::Lifecycle;
use nsh_core::prelude::*;

use super::{AppState, Route, Tab, now_ms};
use crate::domain::auto_connect::{AutoTrigger, LEASE_DEBOUNCE, observe_lease};
use crate::domain::link_health::{
    DownReason, FOREGROUND_RESUME_PROBE_TIMEOUT, FOREGROUND_RESUME_RETRY_DELAY,
    FOREGROUND_RESUME_TRIES, LOOP_INTERVAL, LinkHealth, LinkState, Next, ProbeOutcome,
    WARM_PROBE_TIMEOUT, same_camera,
};
use crate::domain::{ConnectionState, UpdatePhase};
use crate::platform::{self, link_support};

/// What woke the loop.
enum Wake {
    Tick,
    /// The camera's Bluetooth link dropped: probe now.
    Bluetooth,
    /// The phone dropped the camera network.
    NetworkLost,
}

impl AppState {
    /// One loop for the link: probe a connected camera every few seconds in
    /// the foreground, at once after a Bluetooth disconnect and after a
    /// return to the foreground; watch for a camera lease while disconnected.
    pub(super) fn watch_link(self) {
        let (tx, rx) = async_channel::unbounded();
        platform::watch_disconnects(tx);
        let (loss_tx, loss_rx) = async_channel::unbounded();
        platform::watch_network_loss(loss_tx);
        spawn(async move {
            use futures_util::future::{Either, select};
            let mut was_active = true;
            loop {
                let lost = async {
                    // Platforms that never report a loss close the channel.
                    match loss_rx.recv().await {
                        Ok(()) => Wake::NetworkLost,
                        Err(_) => std::future::pending().await,
                    }
                };
                let disconnect = async {
                    match rx.recv().await {
                        Ok(_) => Some(Wake::Bluetooth),
                        Err(_) => None,
                    }
                };
                let wake = match select(
                    Box::pin(select(Box::pin(disconnect), Box::pin(lost))),
                    Box::pin(timer(LOOP_INTERVAL)),
                )
                .await
                {
                    Either::Left((Either::Left((None, _)), _)) => break,
                    Either::Left((Either::Left((Some(wake), _)), _)) => wake,
                    Either::Left((Either::Right((wake, _)), _)) => wake,
                    Either::Right(_) => Wake::Tick,
                };
                let active = lifecycle() == Lifecycle::Active;
                let resumed = active && !was_active;
                was_active = active;
                if !active || !matches!(self.flow_phase(), UpdatePhase::Idle) {
                    continue;
                }
                if !self.link_connected() {
                    if resumed {
                        self.auto_connect(AutoTrigger::Foreground);
                    }
                    self.watch_lease().await;
                    continue;
                }
                if matches!(wake, Wake::NetworkLost) {
                    // Camera requests have no route left; nothing to probe.
                    if !self.connect_in_flight() {
                        self.health_step(|h| h.down(DownReason::NetworkLost));
                    }
                    continue;
                }
                if self.sheet().is_some() {
                    continue;
                }
                // A recent frame of the change stream shows that the camera
                // answers; a wake for a reason still probes at once.
                if matches!(wake, Wake::Tick) && !resumed && self.change_stream_fresh() {
                    continue;
                }
                self.check_link(resumed).await;
            }
        });
    }

    /// Probe the connected camera and apply the result.
    async fn check_link(self, resumed: bool) {
        let Some(status) = self.status.get() else {
            return;
        };
        let generation = self.connect_generation.get();
        let expected = status.device_id.clone();
        let model = status.camera.model;
        let probe = |timeout| {
            let expected = expected.clone();
            async move {
                match link_support::read_props(timeout).await {
                    None => ProbeOutcome::Failed,
                    Some(props) if same_camera(&props, expected.as_deref(), model) => {
                        ProbeOutcome::Ok
                    }
                    Some(_) => ProbeOutcome::Mismatch,
                }
            }
        };
        // A connect, cancel, or sheet may have come up during a probe.
        let current = move || self.connect_generation.get() == generation && self.sheet().is_none();
        let next = if resumed {
            let mut outcome = ProbeOutcome::Failed;
            for attempt in 0..FOREGROUND_RESUME_TRIES {
                outcome = probe(FOREGROUND_RESUME_PROBE_TIMEOUT).await;
                if outcome != ProbeOutcome::Failed {
                    break;
                }
                if attempt + 1 < FOREGROUND_RESUME_TRIES {
                    timer(FOREGROUND_RESUME_RETRY_DELAY).await;
                }
            }
            if !current() {
                return;
            }
            self.health_step(|h| h.resumed(outcome))
        } else {
            let outcome = probe(WARM_PROBE_TIMEOUT).await;
            if !current() {
                return;
            }
            self.health_step(|h| h.warm_probe(outcome))
        };
        if next == Next::Recover {
            let outcome = probe(WARM_PROBE_TIMEOUT).await;
            if current() {
                self.health_step(|h| h.recovered(outcome));
            }
        }
    }

    /// Apply one step to the session's link health, publish its state, and
    /// end the link when it went down.
    fn health_step(self, step: impl FnOnce(&mut LinkHealth) -> Next) -> Next {
        let (next, state) = self.health.with(|h| {
            let mut h = h.borrow_mut();
            let next = step(&mut h);
            (next, h.state())
        });
        if self.link_state.get() != state {
            app_info!("camera link {state:?}");
            self.link_state.set(state);
        }
        if let Next::Down(reason) = next {
            self.link_down(reason);
        }
        next
    }

    /// A connect succeeded: a new session starts ready.
    pub(super) fn start_link_session(self) {
        self.health.with(|h| *h.borrow_mut() = LinkHealth::ready());
        self.link_state.set(LinkState::Ready);
    }

    /// The user disconnects: the camera closes its access point and the phone
    /// returns to its own network. The app stops at once; closing the access
    /// point is best effort, over the camera's API and then Bluetooth.
    pub fn disconnect_camera(self) {
        self.end_session(false);
    }

    /// The user turns the camera off, over the camera's API and then
    /// Bluetooth. When neither works, the access point still closes.
    pub fn power_off_camera(self) {
        self.end_session(true);
    }

    /// The phone is turning the camera off.
    pub fn powering_off(self) -> bool {
        self.powering_off.get()
    }

    fn end_session(self, power_off: bool) {
        if !self.link_connected() || self.powering_off.get() {
            return;
        }
        let peripheral = self.remembered().map(|r| r.peripheral_id);
        self.auto_suppressed.set(true);
        self.powering_off.set(power_off);
        self.root.run(|| {
            spawn(async move {
                let mut finished = false;
                if power_off {
                    finished = platform::http::camera_finish(true).await;
                    if !finished && let Some(id) = &peripheral {
                        finished = platform::end_session_over_bluetooth(id, true).await;
                    }
                    app_info!("camera power off finished={finished}");
                }
                // The access point request goes out before the link stops.
                let closing = (!finished).then(|| platform::http::camera_finish(false));
                self.powering_off.set(false);
                self.health_step(|h| h.down(DownReason::User));
                if let Some(closing) = closing
                    && !closing.await
                    && let Some(id) = &peripheral
                {
                    platform::end_session_over_bluetooth(id, false).await;
                }
            })
        });
    }

    /// Turn the camera off for an update's restart and end the link the
    /// way the person's own power off does, without a notice.
    pub(super) async fn power_off_for_update(self, peripheral_id: &str) {
        self.auto_suppressed.set(true);
        let mut finished = platform::http::camera_finish(true).await;
        if !finished {
            finished = platform::end_session_over_bluetooth(peripheral_id, true).await;
        }
        app_info!("update restart: power off finished={finished}");
        if self.link_connected() {
            self.health_step(|h| h.down(DownReason::User));
        }
    }

    /// The connected camera advertised that it powered off: it is gone.
    pub(super) fn camera_powered_off(self, peripheral_id: &str) {
        let ours = self
            .remembered()
            .is_some_and(|r| r.peripheral_id.eq_ignore_ascii_case(peripheral_id));
        if ours && self.link_connected() && self.sheet().is_none() {
            self.health_step(|h| h.down(DownReason::PoweredOff));
        }
    }

    /// The camera is gone. Keep its nearby prompt away for this power-on and
    /// give up the Bluetooth link the connect kept: a camera on an open link
    /// does not advertise, so the nearby watch could not see it power on
    /// again. A loss the user did not cause is reported, camera pages close,
    /// and one silent connect tries the network.
    fn link_down(self, reason: DownReason) {
        app_info!("camera link down: {reason:?}");
        if let Some(mut status) = self.status.get() {
            status.camera.connection = ConnectionState::Disconnected;
            self.status.set(Some(status));
        }
        self.reset_looks();
        self.close_software_pages();
        if let Some(remembered) = self.remembered() {
            self.suppress_nearby(&remembered.peripheral_id);
            if self.connect_started.get().is_none() {
                platform::release_link(&remembered.peripheral_id);
            }
        }
        if !reason.unexpected() {
            self.return_tab.set(None);
            return;
        }
        // The toast is read aloud as it shows.
        self.show_toast(crate::strings::tr("cam.lost_title", &[]));
        self.leave_camera_pages();
        if lifecycle() == Lifecycle::Active {
            self.auto_connect(AutoTrigger::UnexpectedDrop);
        }
    }

    /// Close pages that need the camera. The Looks and Recipes tabs go back to the
    /// camera tab, where the reconnect is, and is restored after it.
    fn leave_camera_pages(self) {
        // The Looks tab and every page on it read the camera.
        if matches!(self.tab(), Tab::Looks | Tab::Recipes) {
            self.return_tab.set(Some(self.tab()));
            self.set_tab(Tab::Camera);
        }
        self.pop_to(Tab::Looks, 0);
        self.pop_to(Tab::Recipes, 0);
    }

    /// Software Update reads the camera: without one, it and the pages above
    /// it close. A sheet of an update under way stays over the camera tab.
    fn close_software_pages(self) {
        let page = Some(Route::SoftwareUpdate);
        if let Some(depth) =
            (0..super::MAX_DEPTH).find(|depth| self.route_at(Tab::Camera, *depth) == page)
        {
            self.pop_to(Tab::Camera, depth);
        }
    }

    /// Sample the route while disconnected; a new camera lease, once it
    /// held for the debounce, starts a silent connect.
    async fn watch_lease(self) {
        if !self.sample_lease().await {
            return;
        }
        timer(LEASE_DEBOUNCE).await;
        self.sample_lease().await;
        if self.lease.with(|l| l.borrow_mut().take_due(now_ms())) {
            self.auto_connect(AutoTrigger::Subnet);
        }
    }

    /// One look at the route. Returns whether a probe waits for its debounce.
    async fn sample_lease(self) -> bool {
        let models: Vec<_> = self.remembered().map(|r| r.model).into_iter().collect();
        let snapshot = platform::network_snapshot().await;
        let disconnected = !self.link_connected() && !self.connect_in_flight();
        self.lease.with(|l| {
            l.borrow_mut()
                .observe(observe_lease(snapshot, &models), disconnected, now_ms())
        })
    }
}
