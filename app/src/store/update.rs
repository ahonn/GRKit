//! Real update orchestration. The camera journal is authoritative across reconnects.
use super::*;
use crate::domain::{
    JournalState, UpdateIntent, UpdateObservation, observe_update, package_to_models,
};

impl AppState {
    pub fn release_checking(self) -> bool {
        self.release_busy.get()
    }
    pub fn release_error(self) -> Option<String> {
        self.release_error.get()
    }
    pub fn has_pending_update(self) -> bool {
        platform::update::load_intent()
            .is_some_and(|i| self.camera_id().as_deref() == Some(&i.camera_id))
    }
    fn camera_id(self) -> Option<String> {
        self.selected_id
            .get()
            .or_else(|| self.remembered().map(|r| r.peripheral_id))
    }

    pub fn refresh_update_info(self) {
        if self.demo() || self.release_busy.get() || self.update_busy.get() {
            return;
        }
        let Some(status) = self.status.get() else {
            return;
        };
        let generation = self.connect_generation.get();
        self.release_busy.set(true);
        self.release_error.set(None);
        self.root.run(|| {
            spawn(async move {
                let release =
                    platform::release::latest(status.camera.model, &status.camera.firmware).await;
                if self.connect_generation.get() == generation {
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
                            self.release.with(|slot| *slot.borrow_mut() = Some(release));
                        }
                        Err(message) => {
                            // The detail is for the log, not for the person.
                            log::warn!("Release check failed: {message}");
                            self.release_error
                                .set(Some(crate::strings::tr("software.unavailable", &[])));
                        }
                    }
                }
                self.release_busy.set(false);
            })
        });
    }

    /// Read the update state of the camera again, as after a software
    /// request of another phone. This reads only the camera, never the
    /// release site.
    pub(super) fn refresh_software(self) {
        if self.demo() || self.package.get().is_none() || self.update_busy.get() {
            return;
        }
        self.root
            .run(|| spawn(async move { self.refresh_camera_package().await }));
    }

    fn fail_update(self, error: UpdateError) {
        self.phase.set(UpdatePhase::Failed(error.clone()));
        self.sheet.set(Some(Sheet::Error(error)));
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
        if platform::update::load_intent().is_some() {
            self.fail_update(UpdateError::Service("A previous update needs to be checked before starting another one. Reconnect to that camera first.".into()));
            return;
        }
        let Some(target) = package.available else {
            self.fail_update(UpdateError::Download("Update information is unavailable. Check for updates while connected to the internet.".into()));
            return;
        };
        let intent = UpdateIntent {
            camera_id,
            kind,
            baseline: package.installed,
            target,
        };
        self.update_busy.set(true);
        self.update_cancel.set(false);
        self.baseline.set(Some(intent.baseline.clone()));
        self.phase.set(UpdatePhase::Sending {
            progress: -1.0,
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
            })
        });
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
        let release = self
            .release
            .with(|slot| slot.borrow().clone())
            .ok_or_else(|| {
                UpdateError::Download(
                    "Check for updates while connected to the internet, then try again.".into(),
                )
            })?;
        if release.version != intent.target {
            return Err(UpdateError::Download(
                "The available release changed. Check for updates and try again.".into(),
            ));
        }
        let cancelled = async {
            while !self.update_cancel.get() {
                timer(Duration::from_millis(100)).await;
            }
        };
        let body = match futures_util::future::select(
            Box::pin(platform::release::download(&release, &before, |_| {})),
            Box::pin(cancelled),
        )
        .await
        {
            futures_util::future::Either::Left((result, _)) => result.map_err(|message| {
                log::warn!("Release download failed: {message}");
                UpdateError::Download(crate::strings::tr("error.download_body", &[]))
            })?,
            futures_util::future::Either::Right(_) => {
                self.dismiss_error();
                return Ok(());
            }
        };
        if self.update_cancel.get() {
            self.dismiss_error();
            return Ok(());
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
            })
        });
    }

    async fn monitor_update(self, intent: UpdateIntent) {
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
            if self.update_reconnect.get() {
                self.update_reconnect.set(false);
                let model = self
                    .status
                    .get()
                    .map(|s| s.camera.model)
                    .unwrap_or(CameraModel::Gr3);
                let options = super::warmup::options_for(&intent.camera_id);
                match platform::connect_camera(&intent.camera_id, model, options, |_| {}).await {
                    Ok(connected) => self.status.set(Some(connected.status)),
                    Err(_) => {
                        self.fail_update(UpdateError::ConnectionLost);
                        return;
                    }
                }
            }
            let result = async {
                let state = platform::update::snapshot().await?;
                Ok::<_, UpdateError>(state)
            }
            .await;
            match result {
                Ok(state) => {
                    failures = 0;
                    let version = state.package.version.as_deref().unwrap_or_default();
                    match observe_update(state.state, version, &intent.target) {
                        UpdateObservation::Waiting => {
                            let deadline = *self
                                .confirm_deadline
                                .get()
                                .get_or_insert_with(|| Instant::now() + Duration::from_secs(300));
                            self.confirm_deadline.set(Some(deadline));
                            self.phase.set(UpdatePhase::WaitingConfirm {
                                kind: intent.kind,
                                deadline,
                            });
                            self.sheet.set(Some(Sheet::Confirm));
                        }
                        UpdateObservation::Restart => {
                            self.phase.set(UpdatePhase::Restart { kind: intent.kind });
                            self.sheet.set(Some(Sheet::Restart));
                        }
                        UpdateObservation::Installed => {
                            self.refresh_camera_package().await;
                            self.phase.set(UpdatePhase::Installed {
                                kind: intent.kind,
                                version: intent.target.clone(),
                            });
                            self.sheet.set(Some(Sheet::Installed {
                                version: intent.target.clone(),
                                previous: intent.baseline.clone(),
                            }));
                            self.confirm_deadline.set(None);
                            platform::update::clear_intent();
                            return;
                        }
                        UpdateObservation::Stopped => {
                            platform::update::clear_intent();
                            self.fail_update(UpdateError::NotConfirmed);
                            return;
                        }
                    }
                }
                Err(_) => {
                    failures += 1;
                    if failures >= 3 && !matches!(self.flow_phase(), UpdatePhase::Restart { .. }) {
                        self.fail_update(UpdateError::ConnectionLost);
                        return;
                    }
                }
            }
            // Leave the intent for a later reconnect; never infer success or delete on timeout.
            if started.elapsed() > Duration::from_secs(600) {
                self.fail_update(UpdateError::ConnectionLost);
                return;
            }
            timer(Duration::from_secs(1)).await;
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

    pub(super) fn cancel_real_update(self) {
        self.update_cancel.set(true);
        if !self.update_busy.get() {
            self.resume_update();
            self.update_cancel.set(true);
        }
    }
}
