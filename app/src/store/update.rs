//! The update of GRKit on the camera: the release check, the download and
//! send, the camera's confirmation, and the restart that installs it. The
//! camera journal is authoritative across reconnects.
use super::*;
use crate::camera_api::DownloadError;
use crate::domain::{
    JournalState, ReleaseProblem, RestartStep, UpdateIntent, UpdateObservation, card_shortfall,
    observe_update, package_to_models,
};

/// How long the phone waits for a camera it turned off to advertise again
/// before it asks the person to restart the camera.
const STANDBY_WAIT: Duration = Duration::from_secs(15);

impl AppState {
    pub fn release_checking(self) -> bool {
        self.release_busy.get()
    }
    /// The notes of recent releases, from the last release check.
    pub fn release_notes(self) -> Vec<crate::domain::release_notes::VersionNotes> {
        self.release_notes.get()
    }
    /// Why the phone cannot offer an update, after a failed check.
    pub fn release_problem(self) -> Option<ReleaseProblem> {
        self.release_problem.get()
    }
    pub fn has_pending_update(self) -> bool {
        self.api()
            .intent()
            .is_some_and(|i| self.camera_id().as_deref() == Some(&i.camera_id))
    }
    /// The update journal of the connected camera: an update that waits for
    /// its confirmation or for a restart.
    pub fn camera_journal(self) -> Option<JournalState> {
        self.journal.get()
    }
    /// The version of the update that waits on the camera, when known: this
    /// phone's own, else the release the phone offers.
    pub fn pending_version(self) -> Option<String> {
        self.api()
            .intent()
            .filter(|i| self.camera_id().as_deref() == Some(&i.camera_id) && !i.target.is_empty())
            .map(|i| i.target)
            .or_else(|| self.package.get().and_then(|p| p.available))
    }
    fn camera_id(self) -> Option<String> {
        self.selected_id
            .get()
            .or_else(|| self.remembered().map(|r| r.peripheral_id))
    }

    /// The person's cancel of an update is under way.
    pub fn update_cancelling(self) -> bool {
        self.update_run.get() == UpdateRun::Cancelling
    }

    /// This phone's update task runs.
    pub(super) fn update_running(self) -> bool {
        self.update_run.get() != UpdateRun::Idle
    }

    /// The phone restarts the camera so it reads changed Look files.
    pub fn camera_restarting(self) -> bool {
        self.camera_restarting.get()
    }

    /// Check the release site. A check that a reconnect overtakes runs again
    /// for the new connection, so no page waits on a dropped answer.
    pub fn refresh_update_info(self) {
        if self.release_busy.get() {
            return;
        }
        if self.update_running() {
            // A reconnect during an update reads a package without an offer.
            let known = self
                .release
                .with(|slot| slot.borrow().as_ref().map(|r| r.version.clone()));
            if known.is_some() {
                self.package.update(|p| {
                    if let Some(p) = p {
                        p.available = known;
                    }
                });
            }
            return;
        }
        if !self.link_connected() {
            return;
        }
        let Some(status) = self.status.get() else {
            return;
        };
        let generation = self.connect_generation.get();
        self.release_busy.set(true);
        self.release_problem.set(None);
        let release = self
            .api()
            .latest_release(status.camera.model, &status.camera.firmware);
        self.run(async move {
            let release = release.await;
            let current = self.connect_generation.get() == generation;
            if current {
                match release {
                    Ok(release) => {
                        log::info!(
                            "Release {} transfer estimate {} bytes",
                            release.version,
                            release.bytes
                        );
                        self.package.update(|p| {
                            if let Some(p) = p {
                                p.available = Some(release.version.clone());
                                // Transfer estimates are not the final archive's size.
                                p.package_bytes = 0;
                            }
                        });
                        self.release_notes.set(release.notes.clone());
                        self.recipes.presets.set(release.presets.clone());
                        // The cached release still offers its update, but
                        // the check itself failed.
                        if release.offline {
                            self.release_problem.set(Some(ReleaseProblem::Network));
                        }
                        self.release.with(|slot| *slot.borrow_mut() = Some(release));
                    }
                    Err(problem) => self.release_problem.set(Some(problem)),
                }
            }
            self.release_busy.set(false);
            if !current {
                self.refresh_update_info();
            }
        });
        self.refresh_journal();
    }

    /// Read the update journal of the camera for the camera tab.
    pub(super) fn refresh_journal(self) {
        let snapshot = self.api().update_snapshot();
        self.run(async move {
            if let Ok(snapshot) = snapshot.await {
                self.note_journal(snapshot.state);
            }
        });
    }

    fn note_journal(self, state: JournalState) {
        self.journal
            .set((state != JournalState::None).then_some(state));
    }

    /// Read the update state of the camera again, as after a software
    /// request of another phone. This reads only the camera, never the
    /// release site.
    pub(super) fn refresh_software(self) {
        if self.package.get().is_none() || self.update_running() {
            return;
        }
        self.run(async move {
            self.refresh_camera_package().await;
            if let Ok(snapshot) = self.api().update_snapshot().await {
                self.note_journal(snapshot.state);
            }
        });
    }

    fn fail_update(self, error: UpdateError) {
        if self.update_shown.get() {
            self.phase.set(UpdatePhase::Failed(error.clone()));
            self.sheet.set(Some(Sheet::Error(error)));
        } else {
            // After the send the update shows on Software Update, which says
            // why it stopped; the link watch and auto connect wait for an
            // idle phase.
            app_info!("update stopped after its send: {error:?}");
            self.phase.set(UpdatePhase::Idle);
            self.update_notice.set(Some(error));
        }
    }

    /// Why the last update stopped after its send, if it did.
    pub fn update_notice(self) -> Option<UpdateError> {
        self.update_notice.get()
    }

    /// The update waits on the camera for a restart: the camera going away
    /// is the restart, not a lost connection.
    pub(super) fn update_restart_pending(self) -> bool {
        self.has_pending_update()
            && (self.journal.get() == Some(JournalState::Confirmed)
                || matches!(
                    self.flow_phase(),
                    UpdatePhase::Restart { .. }
                        | UpdatePhase::Restarting { .. }
                        | UpdatePhase::ManualRestart { .. }
                ))
    }

    /// Open the update that waits on the camera: the sheet continues at
    /// its step. Another phone or the website may have sent it; then its
    /// version is unknown.
    pub fn open_pending_update(self) {
        let confirmed = self.journal.get() == Some(JournalState::Confirmed);
        let Some(camera_id) = self.camera_id() else {
            return;
        };
        if self.api().intent().is_none() {
            let Some(package) = self.package.get() else {
                return;
            };
            let intent = UpdateIntent {
                camera_id,
                kind: UpdateKind::Install,
                baseline: package.installed,
                target: package.available.unwrap_or_default(),
                confirmed: self.journal.get() == Some(JournalState::Confirmed),
                modules: Vec::new(),
            };
            if self.api().save_intent(&intent).is_err() {
                return;
            }
        }
        // The sheet shows the camera's step until the monitor reads it; a
        // restart under way keeps its own.
        if !self.update_running() {
            let kind = UpdateKind::Install;
            self.phase.set(if confirmed {
                UpdatePhase::Restart { kind }
            } else {
                UpdatePhase::WaitingConfirm { kind }
            });
        }
        self.update_shown.set(true);
        self.sheet.set(Some(if confirmed {
            Sheet::Restart
        } else {
            Sheet::Confirm
        }));
        self.resume_update();
    }

    /// The update waits on the camera for the person: its sheet can close,
    /// and the update stays there.
    pub fn update_waits_on_camera(self) -> bool {
        matches!(
            self.flow_phase(),
            UpdatePhase::WaitingConfirm { .. }
                | UpdatePhase::Confirmed { .. }
                | UpdatePhase::Restart { .. }
        )
    }

    /// Close the update sheet and keep the update on the camera: the camera
    /// tab and Software Update show where it is, and open the sheet again.
    pub fn hide_update_sheet(self) {
        self.sheet.set(None);
        self.update_shown.set(false);
    }

    /// Download the release and send it to the camera; the camera then
    /// asks the person to confirm it, and installs it at its next start.
    pub fn begin_update(self, kind: UpdateKind) {
        if self.update_running() {
            return;
        }
        self.update_kind.set(kind);
        let Some(package) = self.package.get() else {
            return;
        };
        let Some(camera_id) = self.camera_id() else {
            return;
        };
        if self
            .status
            .get()
            .is_none_or(|s| s.camera.connection != ConnectionState::Connected)
        {
            return;
        }
        self.update_shown.set(true);
        self.update_notice.set(None);
        if self.api().intent().is_some() {
            self.fail_update(UpdateError::Service("A previous update needs to be checked before starting another one. Reconnect to that camera first.".into()));
            return;
        }
        let Some(target) = package.available else {
            self.fail_update(UpdateError::Download(crate::strings::tr(
                "update.download_body",
                &[],
            )));
            return;
        };
        let intent = UpdateIntent {
            camera_id,
            kind,
            baseline: package.installed,
            target,
            confirmed: false,
            modules: self.drafted_modules(),
        };
        self.update_run.set(UpdateRun::Running);
        self.baseline.set(Some(intent.baseline.clone()));
        self.phase.set(UpdatePhase::Downloading {
            progress: 0.0,
            kind,
        });
        self.sheet.set(Some(Sheet::Sending));
        self.root.run(|| {
            spawn(async move {
                let result = self.prepare_and_stage(&intent).await;
                match result {
                    Ok(()) => {
                        let intent = self.api().intent().unwrap_or(intent);
                        self.monitor_update(intent).await;
                    }
                    Err(_) if self.update_cancelling() && !self.has_pending_update() => {
                        self.dismiss_error()
                    }
                    Err(error) => self.fail_update(error),
                }
                self.update_run.set(UpdateRun::Idle);
            })
        });
    }

    /// The package for `intent`: the one assembled for a send that did not
    /// finish, else a new download. `None` after a cancel.
    async fn package_for(
        self,
        intent: &UpdateIntent,
        before: &crate::domain::PackageResponse,
    ) -> Result<Option<Vec<u8>>, UpdateError> {
        // A package holds the modules of this update as well as its version.
        let staged_key = format!("{}|{}", intent.target, intent.modules.join(","));
        let mut before = before.clone();
        if !intent.modules.is_empty() {
            before.modules = Some(serde_json::json!(intent.modules));
        }
        let before = &before;
        if let Some(body) = self.staged.with(|slot| {
            slot.borrow()
                .as_ref()
                .filter(|(key, _)| *key == staged_key)
                .map(|(_, body)| body.clone())
        }) {
            return Ok(Some(body));
        }
        let release = self
            .release
            .with(|slot| slot.borrow().clone())
            .ok_or_else(|| {
                UpdateError::Download(crate::strings::tr("update.download_body", &[]))
            })?;
        if release.version != intent.target {
            return Err(UpdateError::Download(
                "The available release changed. Check for updates and try again.".into(),
            ));
        }
        let kind = intent.kind;
        let cancelled = async {
            while !self.update_cancelling() {
                timer(Duration::from_millis(100)).await;
            }
        };
        let progress = move |progress: f32| {
            self.phase.set(UpdatePhase::Downloading { progress, kind });
        };
        match futures_util::future::select(
            release.package.download(before, Box::new(progress)),
            Box::pin(cancelled),
        )
        .await
        {
            futures_util::future::Either::Left((result, _)) => {
                let body = result.map_err(|error| {
                    log::warn!("Release download failed: {error}");
                    match error {
                        DownloadError::Network(_) => {
                            UpdateError::Download(crate::strings::tr("update.download_body", &[]))
                        }
                        DownloadError::ModuleNotKept(id) => UpdateError::ModuleNotKept(id),
                        DownloadError::Package(_) => UpdateError::PackageInvalid,
                    }
                })?;
                self.staged.with(|slot| {
                    *slot.borrow_mut() = Some((staged_key.clone(), body.clone()));
                });
                Ok(Some(body))
            }
            futures_util::future::Either::Right(_) => Ok(None),
        }
    }

    async fn prepare_and_stage(self, intent: &UpdateIntent) -> Result<(), UpdateError> {
        let api = self.api();
        let state = api.update_snapshot().await?;
        // The start of the camera that these reads saw: the request names
        // it, so the camera refuses the request after a restart.
        let boot_id = api.boot_id().await?;
        if state.state != JournalState::None {
            return Err(UpdateError::Service("The camera already has a pending update. Confirm or cancel it on the camera first.".into()));
        }
        let before = state.package.clone();
        if before.version.as_deref() != Some(&intent.baseline) {
            return Err(UpdateError::Service(
                "The camera version changed. Reconnect before updating.".into(),
            ));
        }
        let Some(body) = self.package_for(intent, &before).await? else {
            self.dismiss_error();
            return Ok(());
        };
        if self.update_cancelling() {
            self.dismiss_error();
            return Ok(());
        }
        self.phase.set(UpdatePhase::Sending {
            progress: -1.0,
            kind: intent.kind,
        });
        // Room on the card, before the send rather than after it.
        if let Some(bytes_needed) = card_shortfall(body.len() as u64, api.free_bytes().await) {
            return Err(UpdateError::CardFull { bytes_needed });
        }
        // Recheck after downloading. Do not overwrite changes made by another client.
        let fresh = api.update_snapshot().await?;
        if fresh.package != before || fresh.state != JournalState::None {
            return Err(UpdateError::Service(
                "The camera package changed while preparing the update. Reconnect and try again."
                    .into(),
            ));
        }
        if self.update_cancelling() {
            self.dismiss_error();
            return Ok(());
        }
        api.save_intent(intent)?;
        // Do not abort an in-flight PUT: cancellation must run after it settles.
        let staged = api.stage_update(body, &boot_id).await;
        if self.update_cancelling() {
            // A request can wait even when its answer did not arrive; a
            // cancel without one changes nothing.
            let _ = self.cancel_staged().await;
            api.clear_intent();
            self.dismiss_error();
            return Ok(());
        }
        match staged {
            Ok(()) => Ok(()),
            Err(error) => {
                // Only a fresh camera response can resolve an uncertain mutation.
                if api
                    .update_snapshot()
                    .await
                    .is_ok_and(|s| s.state == JournalState::None)
                {
                    api.clear_intent();
                }
                Err(error)
            }
        }
    }

    /// Cancel the update that waits on the camera.
    async fn cancel_staged(self) -> Result<(), UpdateError> {
        let api = self.api();
        let boot_id = api.boot_id().await?;
        api.cancel_update(&boot_id).await
    }

    pub(super) fn resume_update(self) {
        if self.update_running() {
            return;
        }
        let Some(intent) = self.api().intent() else {
            return;
        };
        if self.camera_id().as_deref() != Some(&intent.camera_id) {
            return;
        }
        self.update_kind.set(intent.kind);
        self.baseline.set(Some(intent.baseline.clone()));
        self.update_run.set(UpdateRun::Running);
        self.root.run(|| {
            spawn(async move {
                self.monitor_update(intent).await;
                self.update_run.set(UpdateRun::Idle);
            })
        });
    }

    /// Restart the camera so it reads changed Look files: off and on as an
    /// update's restart does, then the usual connect. Without a camera in
    /// standby, the person restarts it.
    pub fn restart_camera_now(self) {
        if !self.camera_online() || self.camera_restarting.get() {
            return;
        }
        let Some(camera) = self.remembered() else {
            return;
        };
        let id = camera.peripheral_id;
        self.camera_restarting.set(true);
        self.show_toast(crate::strings::tr("restart.running_title", &[]));
        self.root.run(|| {
            spawn(async move {
                let off_at = now_ms();
                self.power_off_for_update(&id).await;
                let started = Instant::now();
                let mut heard = false;
                while started.elapsed() < STANDBY_WAIT {
                    timer(Duration::from_millis(500)).await;
                    heard = self.nearby_watch.with(|w| {
                        let window = now_ms().saturating_sub(off_at);
                        w.borrow().heard_since(&id, window, now_ms()).is_some()
                    });
                    if heard {
                        break;
                    }
                }
                if heard {
                    self.camera_restarting.set(false);
                    self.connect_camera(&id);
                    // After the connect began, which ends any earlier hold.
                    self.restart_connect.set(true);
                    return;
                }
                app_info!("look restart: no standby camera; manual restart");
                self.alert_closed(Alert::ManualRestart).await;
                self.camera_restarting.set(false);
            })
        });
    }

    /// The phone restarts the camera: off over its API (else Bluetooth),
    /// then on over Bluetooth once it advertises in standby. Without a
    /// standby camera the person restarts it.
    async fn restart_camera(self, intent: &UpdateIntent) -> bool {
        let kind = intent.kind;
        self.phase.set(UpdatePhase::Restarting {
            kind,
            step: RestartStep::PoweringOff,
        });
        #[cfg(demo)]
        if self.demo() {
            return self.simulated_restart(intent).await;
        }
        let off_at = now_ms();
        self.power_off_for_update(&intent.camera_id).await;
        self.phase.set(UpdatePhase::Restarting {
            kind,
            step: RestartStep::PoweringOn,
        });
        let started = Instant::now();
        let mut heard = false;
        while started.elapsed() < STANDBY_WAIT {
            timer(Duration::from_millis(500)).await;
            heard = self.nearby_watch.with(|w| {
                let w = w.borrow();
                let window = now_ms().saturating_sub(off_at);
                w.heard_since(&intent.camera_id, window, now_ms()).is_some()
            });
            if heard {
                break;
            }
        }
        if !heard {
            app_info!("update restart: no standby camera; manual restart");
            self.phase.set(UpdatePhase::ManualRestart { kind });
            return false;
        }
        self.phase.set(UpdatePhase::Restarting {
            kind,
            step: RestartStep::Reconnecting,
        });
        self.rejoin_for_update(intent).await
    }

    /// Wake and join the camera of `intent` again. A camera whose confirmed
    /// update left GRKit off answers without the service: that update did
    /// not install.
    async fn rejoin_for_update(self, intent: &UpdateIntent) -> bool {
        #[cfg(demo)]
        if self.demo() {
            return self.simulated_rejoin(intent).await;
        }
        let model = self
            .remembered()
            .map(|r| r.model)
            .or_else(|| self.status.get().map(|s| s.camera.model))
            .unwrap_or(CameraModel::Gr3);
        let options = super::warmup::options_for(&intent.camera_id);
        match platform::connect_camera(&intent.camera_id, model, options, |_| {}).await {
            Ok(connected) => {
                self.adopt_update_reconnect(&intent.camera_id, connected, options);
                true
            }
            Err(LinkError::NeedsGrkit) if intent.confirmed => {
                self.phase.set(UpdatePhase::Idle);
                self.sheet.set(Some(Sheet::InstallFailed));
                false
            }
            Err(error) => {
                app_info!("update reconnect failed: {error:?}");
                self.phase
                    .set(UpdatePhase::ManualRestart { kind: intent.kind });
                false
            }
        }
    }

    async fn monitor_update(self, mut intent: UpdateIntent) {
        let api = self.api();
        if api.intent().is_none() {
            return;
        }
        // The time limit counts from the last answer of the camera, so an
        // update that waits for the person on the camera never runs out.
        let mut started = Instant::now();
        let mut failures = 0;
        loop {
            if self.update_cancelling() {
                match self.cancel_staged().await {
                    Ok(()) => {
                        api.clear_intent();
                        self.journal.set(None);
                        self.dismiss_error();
                        self.refresh_camera_package().await;
                    }
                    Err(error) => self.fail_update(error),
                }
                return;
            }
            if self.restart_request.get() {
                self.restart_request.set(false);
                if !self.restart_camera(&intent).await
                    && matches!(self.sheet(), Some(Sheet::InstallFailed))
                {
                    return;
                }
            }
            if self.update_reconnect.get() {
                self.update_reconnect.set(false);
                self.phase.set(UpdatePhase::Restarting {
                    kind: intent.kind,
                    step: RestartStep::Reconnecting,
                });
                if !self.rejoin_for_update(&intent).await {
                    if matches!(self.sheet(), Some(Sheet::InstallFailed)) {
                        return;
                    }
                    timer(Duration::from_secs(1)).await;
                    continue;
                }
            }
            let waiting_for_person = matches!(
                self.flow_phase(),
                UpdatePhase::Restart { .. }
                    | UpdatePhase::ManualRestart { .. }
                    | UpdatePhase::Restarting { .. }
            );
            // The camera tab's connect owns the sheet until it ends.
            if waiting_for_person && (!self.link_connected() || self.connect_in_flight()) {
                timer(Duration::from_millis(500)).await;
                continue;
            }
            match api.update_snapshot().await {
                Ok(state) => {
                    failures = 0;
                    self.note_journal(state.state);
                    let version = state.package.version.as_deref().unwrap_or_default();
                    match observe_update(
                        state.state,
                        version,
                        state.package.modules.as_ref(),
                        &intent,
                    ) {
                        UpdateObservation::Waiting => {
                            started = Instant::now();
                            self.phase
                                .set(UpdatePhase::WaitingConfirm { kind: intent.kind });
                        }
                        UpdateObservation::Restart => {
                            if !intent.confirmed {
                                intent.confirmed = true;
                                let _ = api.save_intent(&intent);
                            }
                            if !matches!(
                                self.flow_phase(),
                                UpdatePhase::Restarting { .. } | UpdatePhase::ManualRestart { .. }
                            ) {
                                self.phase.set(UpdatePhase::Restart { kind: intent.kind });
                            }
                        }
                        UpdateObservation::Installed => {
                            self.refresh_camera_package().await;
                            let version = if intent.target.is_empty() {
                                version.to_owned()
                            } else {
                                intent.target.clone()
                            };
                            self.phase.set(UpdatePhase::Installed {
                                kind: intent.kind,
                                version: version.clone(),
                            });
                            self.journal.set(None);
                            self.staged.with(|slot| *slot.borrow_mut() = None);
                            api.clear_intent();
                            if !intent.modules.is_empty() {
                                self.clear_module_draft();
                            }
                            api.save_seen_version(&intent.camera_id, &version);
                            if self.update_shown.get() {
                                haptic(Haptic::Notification(HapticNotification::Success));
                                self.sheet.set(Some(Sheet::Installed {
                                    version,
                                    previous: intent.baseline.clone(),
                                }));
                            } else {
                                // Software Update reads the new version; a
                                // toast says it once, and the flow is idle.
                                self.show_toast(crate::strings::tr(
                                    "update.updated_toast",
                                    &[("version", &version)],
                                ));
                                self.phase.set(UpdatePhase::Idle);
                            }
                            return;
                        }
                        UpdateObservation::Stopped => {
                            // The update left the camera without an
                            // install, as when another phone cancelled it:
                            // the pages show the camera as it is.
                            api.clear_intent();
                            self.journal.set(None);
                            self.phase.set(UpdatePhase::Idle);
                            self.sheet.set(None);
                            self.update_shown.set(false);
                            self.refresh_camera_package().await;
                            return;
                        }
                    }
                }
                Err(_) => {
                    failures += 1;
                    let restarting = matches!(
                        self.flow_phase(),
                        UpdatePhase::Restart { .. }
                            | UpdatePhase::Restarting { .. }
                            | UpdatePhase::ManualRestart { .. }
                    );
                    if failures >= 3 && !restarting {
                        self.fail_update(UpdateError::ConnectionLost);
                        return;
                    }
                }
            }
            // Leave the intent for a later reconnect; never infer success or
            // delete on timeout. A restart the person runs may take longer.
            let manual = matches!(
                self.flow_phase(),
                UpdatePhase::Restart { .. } | UpdatePhase::ManualRestart { .. }
            );
            if !manual && started.elapsed() > Duration::from_secs(600) {
                self.fail_update(UpdateError::ConnectionLost);
                return;
            }
            timer(Duration::from_secs(1)).await;
        }
    }

    pub(super) async fn refresh_camera_package(self) {
        let model = self
            .status
            .get()
            .map(|s| s.camera.model)
            .unwrap_or(CameraModel::Gr3);
        let api = self.api();
        if let Ok(raw) = api.update_package().await
            && let Ok((camera, mut package)) = package_to_models(&raw, model)
        {
            package.available = self
                .release
                .with(|slot| slot.borrow().as_ref().map(|r| r.version.clone()));
            // Prefer camera metadata from the live props read when package omits it.
            if camera.firmware == "—"
                && let Ok((status, _)) = api.read_package(model).await
            {
                self.status.set(Some(status));
            } else {
                let device_id = self.status.get().and_then(|s| s.device_id);
                self.status.set(Some(CameraStatus {
                    camera,
                    powered_on: true,
                    device_id,
                }));
            }
            self.package.set(Some(package));
        }
    }

    /// After a connect: a GRKit that went up since this phone last saw the
    /// camera, with no update of this phone pending, was updated elsewhere
    /// or while the app was away.
    pub(super) fn note_installed_version(self, camera_id: &str, installed: &str) {
        let api = self.api();
        if api.intent().is_some() {
            return;
        }
        if let Some(seen) = api.seen_version(camera_id)
            && seen != installed
            && crate::domain::version_at_least(installed, &seen)
        {
            self.show_toast(crate::strings::tr(
                "update.updated_toast",
                &[("version", installed)],
            ));
        }
        api.save_seen_version(camera_id, installed);
    }

    /// The person cancels the update: the running task, or the one that
    /// waits on the camera, ends it.
    pub fn cancel_update(self) {
        if !self.update_running() {
            self.resume_update();
        }
        // Without an update to resume, nothing would end the cancel.
        if self.update_running() {
            self.update_run.set(UpdateRun::Cancelling);
        }
    }

    /// This phone sent an update to `camera_id` that the camera confirmed:
    /// a camera without GRKit now means the install did not finish.
    pub(super) fn update_confirmed_for(self, camera_id: &str) -> bool {
        self.api()
            .intent()
            .is_some_and(|i| i.confirmed && i.camera_id.eq_ignore_ascii_case(camera_id))
    }

    /// The restart step's main action: the phone turns the camera off and on
    /// again, then reads the version.
    pub fn restart_for_update(self) {
        self.set_frozen(false);
        self.restart_request.set(true);
        self.resume_update();
        // The step answers the tap at once; the running update turns the
        // camera off on its next pass.
        if self.update_running() {
            self.phase.set(UpdatePhase::Restarting {
                kind: self.update_kind.get(),
                step: RestartStep::PoweringOff,
            });
        }
    }

    /// Reconnect after the person restarted the camera: the camera tab runs
    /// its usual connect, and the update goes on from what the camera answers.
    pub fn reconnect_for_update(self) {
        // The update's own reconnect: no connect sheet over Software Update.
        self.update_reconnect.set(true);
        self.resume_update();
    }

    /// The restart is under way: it cannot be stopped or swiped away.
    pub fn update_restarting(self) -> bool {
        matches!(self.flow_phase(), UpdatePhase::Restarting { .. })
    }

    /// The user restarts the camera by hand: the sheet closes and the update
    /// waits on the camera, where the camera tab offers it again.
    pub fn restart_manually(self) {
        self.sheet.set(None);
        self.update_shown.set(false);
        // The connect after the restart must be allowed, and the update must
        // wait as long as the person needs.
        if self.update_running() {
            self.phase.set(UpdatePhase::ManualRestart {
                kind: self.update_kind.get(),
            });
        }
    }

    pub fn dismiss_install_failed(self) {
        self.sheet.set(None);
        self.update_shown.set(false);
    }

    /// Connect again after a restart that left GRKit off.
    pub fn reconnect_install_failed(self) {
        self.dismiss_install_failed();
        if let Some(camera) = self.remembered() {
            self.connect_camera(&camera.peripheral_id);
        }
    }

    pub fn dismiss_installed(self) {
        self.phase.set(UpdatePhase::Idle);
        self.sheet.set(None);
        self.update_shown.set(false);
        self.journal.set(None);
        self.stacks
            .update(|stacks| stacks[tab_index(Tab::Camera)].clear());
    }

    pub fn dismiss_error(self) {
        self.phase.set(UpdatePhase::Idle);
        self.sheet.set(None);
        self.update_shown.set(false);
    }

    pub fn retry_update(self) {
        self.dismiss_error();
        if self.api().intent().is_some() {
            self.update_reconnect.set(true);
            self.resume_update();
        } else {
            self.begin_update(self.update_kind.get());
        }
    }
}
