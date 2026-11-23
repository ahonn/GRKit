//! Real update orchestration. The camera journal is authoritative across reconnects.
use super::*;
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
        platform::update::load_intent()
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
        platform::update::load_intent()
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
        self.update_cancel.get()
    }

    /// The phone restarts the camera so it reads changed Look files.
    pub fn camera_restarting(self) -> bool {
        self.camera_restarting.get()
    }

    /// Check the release site. A check that a reconnect overtakes runs again
    /// for the new connection, so no page waits on a dropped answer.
    pub fn refresh_update_info(self) {
        if self.demo() || self.release_busy.get() {
            return;
        }
        if self.update_busy.get() {
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
        self.root.run(|| {
            spawn(async move {
                let release =
                    platform::release::latest(status.camera.model, &status.camera.firmware).await;
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
                            self.release.with(|slot| *slot.borrow_mut() = Some(release));
                        }
                        Err(problem) => self.release_problem.set(Some(problem)),
                    }
                }
                self.release_busy.set(false);
                if !current {
                    self.refresh_update_info();
                }
            })
        });
        self.refresh_journal();
    }

    /// Read the update journal of the camera for the camera tab.
    pub(super) fn refresh_journal(self) {
        if self.demo() {
            return;
        }
        self.root.run(|| {
            spawn(async move {
                if let Ok(snapshot) = platform::update::snapshot().await {
                    self.note_journal(snapshot.state);
                }
            })
        });
    }

    fn note_journal(self, state: JournalState) {
        if state == JournalState::Waiting && self.confirm_deadline.get().is_none() {
            self.confirm_deadline
                .set(Some(Instant::now() + crate::domain::CONFIRM_DEADLINE));
        }
        self.journal
            .set((state != JournalState::None).then_some(state));
    }

    /// Read the update state of the camera again, as after a software
    /// request of another phone. This reads only the camera, never the
    /// release site.
    pub(super) fn refresh_software(self) {
        if self.demo() || self.package.get().is_none() || self.update_busy.get() {
            return;
        }
        self.root.run(|| {
            spawn(async move {
                self.refresh_camera_package().await;
                if let Ok(snapshot) = platform::update::snapshot().await {
                    self.note_journal(snapshot.state);
                }
            })
        });
    }

    fn fail_update(self, error: UpdateError) {
        self.phase.set(UpdatePhase::Failed(error.clone()));
        if self.update_shown.get() {
            self.sheet.set(Some(Sheet::Error(error)));
        }
    }

    /// Open the update that waits on the camera from the camera tab: the
    /// sheet continues at its step. Another phone or the website may have
    /// sent it; then its version is unknown.
    pub fn open_pending_update(self) {
        if self.demo() {
            self.update_shown.set(true);
            self.sheet.set(Some(match self.journal.get() {
                Some(JournalState::Confirmed) => Sheet::Restart,
                _ => Sheet::Confirm,
            }));
            return;
        }
        let Some(camera_id) = self.camera_id() else {
            return;
        };
        if platform::update::load_intent().is_none() {
            let Some(package) = self.package.get() else {
                return;
            };
            let intent = UpdateIntent {
                camera_id,
                kind: UpdateKind::Install,
                baseline: package.installed,
                target: package.available.unwrap_or_default(),
                confirmed: self.journal.get() == Some(JournalState::Confirmed),
            };
            if platform::update::save_intent(&intent).is_err() {
                return;
            }
        }
        self.update_shown.set(true);
        self.resume_update();
    }

    pub(super) fn begin_real_update(self, kind: UpdateKind) {
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
        if platform::update::load_intent().is_some() {
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
        };
        self.update_busy.set(true);
        self.update_cancel.set(false);
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
                        let intent = platform::update::load_intent().unwrap_or(intent);
                        self.monitor_update(intent).await;
                    }
                    Err(_) if self.update_cancel.get() && !self.has_pending_update() => {
                        self.dismiss_error()
                    }
                    Err(error) => self.fail_update(error),
                }
                self.update_busy.set(false);
                self.update_cancel.set(false);
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
        if let Some(body) = self.staged.with(|slot| {
            slot.borrow()
                .as_ref()
                .filter(|(target, _)| *target == intent.target)
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
            while !self.update_cancel.get() {
                timer(Duration::from_millis(100)).await;
            }
        };
        let progress = move |progress: f32| {
            self.phase.set(UpdatePhase::Downloading { progress, kind });
        };
        match futures_util::future::select(
            Box::pin(platform::release::download(&release, before, progress)),
            Box::pin(cancelled),
        )
        .await
        {
            futures_util::future::Either::Left((result, _)) => {
                let body = result.map_err(|error| {
                    log::warn!("Release download failed: {error}");
                    match error {
                        platform::release::DownloadError::Network(_) => {
                            UpdateError::Download(crate::strings::tr("update.download_body", &[]))
                        }
                        platform::release::DownloadError::ModuleNotKept(id) => {
                            UpdateError::ModuleNotKept(id)
                        }
                        platform::release::DownloadError::Package(_) => UpdateError::PackageInvalid,
                    }
                })?;
                self.staged.with(|slot| {
                    *slot.borrow_mut() = Some((intent.target.clone(), body.clone()));
                });
                Ok(Some(body))
            }
            futures_util::future::Either::Right(_) => Ok(None),
        }
    }

    async fn prepare_and_stage(self, intent: &UpdateIntent) -> Result<(), UpdateError> {
        let state = platform::update::snapshot().await?;
        // The start of the camera that these reads saw: the request names
        // it, so the camera refuses the request after a restart.
        let boot_id = platform::update::boot_id().await?;
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
        if self.update_cancel.get() {
            self.dismiss_error();
            return Ok(());
        }
        self.phase.set(UpdatePhase::Sending {
            progress: -1.0,
            kind: intent.kind,
        });
        // Room on the card, before the send rather than after it.
        if let Some(bytes_needed) =
            card_shortfall(body.len() as u64, platform::update::free_bytes().await)
        {
            return Err(UpdateError::CardFull { bytes_needed });
        }
        // Recheck after downloading. Do not overwrite changes made by another client.
        let fresh = platform::update::snapshot().await?;
        if fresh.package != before || fresh.state != JournalState::None {
            return Err(UpdateError::Service(
                "The camera package changed while preparing the update. Reconnect and try again."
                    .into(),
            ));
        }
        if self.update_cancel.get() {
            self.dismiss_error();
            return Ok(());
        }
        platform::update::save_intent(intent)?;
        // Do not abort an in-flight PUT: cancellation must run after it settles.
        let staged = platform::update::stage(body, &boot_id).await;
        if self.update_cancel.get() {
            // A request can wait even when its answer did not arrive; a
            // cancel without one changes nothing.
            let _ = self.cancel_staged().await;
            platform::update::clear_intent();
            self.dismiss_error();
            return Ok(());
        }
        match staged {
            Ok(()) => Ok(()),
            Err(error) => {
                // Only a fresh camera response can resolve an uncertain mutation.
                if platform::update::snapshot()
                    .await
                    .is_ok_and(|s| s.state == JournalState::None)
                {
                    platform::update::clear_intent();
                }
                Err(error)
            }
        }
    }

    /// Cancel the update that waits on the camera.
    async fn cancel_staged(self) -> Result<(), UpdateError> {
        let boot_id = platform::update::boot_id().await?;
        platform::update::cancel(&boot_id).await
    }

    pub(super) fn resume_update(self) {
        if self.demo() || self.update_busy.get() {
            return;
        }
        let Some(intent) = platform::update::load_intent() else {
            return;
        };
        if self.camera_id().as_deref() != Some(&intent.camera_id) {
            return;
        }
        self.update_kind.set(intent.kind);
        self.baseline.set(Some(intent.baseline.clone()));
        self.update_busy.set(true);
        self.update_cancel.set(false);
        self.root.run(|| {
            spawn(async move {
                self.monitor_update(intent).await;
                self.update_busy.set(false);
                self.update_cancel.set(false);
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
                self.camera_restarting.set(false);
                if heard {
                    self.connect_camera(&id);
                    return;
                }
                app_info!("look restart: no standby camera; manual restart");
                let _ = nsh_base::alert_dialog()
                    .title(crate::strings::tr("restart.title", &[]))
                    .description(crate::strings::tr("restart.manual_body", &[]))
                    .action("ok", crate::strings::tr("update.ok", &[]))
                    .show()
                    .await;
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
        if platform::update::load_intent().is_none() {
            return;
        }
        let started = Instant::now();
        let mut failures = 0;
        loop {
            if self.update_cancel.get() {
                match self.cancel_staged().await {
                    Ok(()) => {
                        platform::update::clear_intent();
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
                UpdatePhase::ManualRestart { .. } | UpdatePhase::Restarting { .. }
            );
            // The camera tab's connect owns the sheet until it ends.
            if waiting_for_person && (!self.link_connected() || self.connect_in_flight()) {
                timer(Duration::from_millis(500)).await;
                continue;
            }
            match platform::update::snapshot().await {
                Ok(state) => {
                    failures = 0;
                    self.note_journal(state.state);
                    let version = state.package.version.as_deref().unwrap_or_default();
                    match observe_update(state.state, version, &intent) {
                        UpdateObservation::Waiting => {
                            let deadline = self.confirm_deadline.get().unwrap_or_else(|| {
                                Instant::now() + crate::domain::CONFIRM_DEADLINE
                            });
                            self.confirm_deadline.set(Some(deadline));
                            self.phase.set(UpdatePhase::WaitingConfirm {
                                kind: intent.kind,
                                deadline,
                            });
                            self.show_update_sheet(Sheet::Confirm);
                        }
                        UpdateObservation::Restart => {
                            if !intent.confirmed {
                                intent.confirmed = true;
                                let _ = platform::update::save_intent(&intent);
                            }
                            if !matches!(
                                self.flow_phase(),
                                UpdatePhase::Restarting { .. } | UpdatePhase::ManualRestart { .. }
                            ) {
                                self.phase.set(UpdatePhase::Restart { kind: intent.kind });
                            }
                            self.show_update_sheet(Sheet::Restart);
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
                            self.confirm_deadline.set(None);
                            self.journal.set(None);
                            self.staged.with(|slot| *slot.borrow_mut() = None);
                            platform::update::clear_intent();
                            platform::update::save_seen_version(&intent.camera_id, &version);
                            if self.update_shown.get() {
                                haptic(Haptic::Notification(HapticNotification::Success));
                                self.sheet.set(Some(Sheet::Installed {
                                    version,
                                    previous: intent.baseline.clone(),
                                }));
                            } else {
                                self.show_toast(crate::strings::tr(
                                    "update.updated_toast",
                                    &[("version", &version)],
                                ));
                            }
                            return;
                        }
                        UpdateObservation::Stopped => {
                            platform::update::clear_intent();
                            self.journal.set(None);
                            self.fail_update(UpdateError::NotConfirmed);
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
            let manual = matches!(self.flow_phase(), UpdatePhase::ManualRestart { .. });
            if !manual && started.elapsed() > Duration::from_secs(600) {
                self.fail_update(UpdateError::ConnectionLost);
                return;
            }
            timer(Duration::from_secs(1)).await;
        }
    }

    /// Show an update step, unless the person has not opened the update
    /// (an update found on the camera shows on the camera tab first).
    fn show_update_sheet(self, sheet: Sheet) {
        if self.update_shown.get() {
            self.sheet.set(Some(sheet));
        }
    }

    async fn refresh_camera_package(self) {
        let model = self
            .status
            .get()
            .map(|s| s.camera.model)
            .unwrap_or(CameraModel::Gr3);
        if let Ok(raw) = platform::update::package().await
            && let Ok((camera, mut package)) = package_to_models(&raw, model)
        {
            package.available = self
                .release
                .with(|slot| slot.borrow().as_ref().map(|r| r.version.clone()));
            // Prefer camera metadata from the live props read when package omits it.
            if camera.firmware == "—"
                && let Ok((status, _)) = platform::http::read_package(model).await
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
        if self.demo() || platform::update::load_intent().is_some() {
            return;
        }
        if let Some(seen) = platform::update::seen_version(camera_id)
            && seen != installed
            && crate::domain::version_at_least(installed, &seen)
        {
            self.show_toast(crate::strings::tr(
                "update.updated_toast",
                &[("version", installed)],
            ));
        }
        platform::update::save_seen_version(camera_id, installed);
    }

    pub(super) fn cancel_real_update(self) {
        self.update_cancel.set(true);
        if !self.update_busy.get() {
            self.resume_update();
            // Without an update to resume, nothing would end the cancel.
            self.update_cancel.set(self.update_busy.get());
        }
    }

    /// This phone sent an update to `camera_id` that the camera confirmed:
    /// a camera without GRKit now means the install did not finish.
    pub(super) fn update_confirmed_for(self, camera_id: &str) -> bool {
        platform::update::load_intent()
            .is_some_and(|i| i.confirmed && i.camera_id.eq_ignore_ascii_case(camera_id))
    }

    /// Restart the camera from the restart step.
    pub(super) fn request_restart(self) {
        self.restart_request.set(true);
        self.resume_update();
    }
}
