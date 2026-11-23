//! Scripted camera used by demo mode and tests.

use std::time::{Duration, Instant};

use crate::domain::{
    Camera, CameraLink, CameraModel, CameraStatus, ConnectionState, LinkError, PackageInfo,
    UpdateError, UpdateFlow, UpdateKind, UpdatePhase, UpdatePoll,
};

/// How a demo or test should fail, if at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DemoFail {
    #[default]
    None,
    CardFull,
    Refused,
    NotConfirmed,
    ConnectionLost,
}

/// Tunables for the fake timeline (tests use short durations).
#[derive(Clone, Debug)]
pub struct FakeConfig {
    pub model: CameraModel,
    pub firmware: String,
    pub installed: String,
    pub available: String,
    pub release_notes: String,
    pub package_bytes: u64,
    pub fail: DemoFail,
    pub card_full_bytes: u64,
    /// How many [`FakeCamera::tick`] calls to finish a send.
    pub send_ticks: u32,
    /// How many ticks after waiting before the camera confirms.
    pub confirm_ticks: u32,
    /// Confirm deadline used when failing with [`DemoFail::NotConfirmed`].
    pub confirm_deadline: Duration,
}

impl Default for FakeConfig {
    fn default() -> Self {
        Self {
            model: CameraModel::Gr3,
            firmware: "2.21".into(),
            installed: "1.6.1".into(),
            available: "1.7.0".into(),
            release_notes: "Faster Wi-Fi updates and clearer confirm steps on the camera.".into(),
            package_bytes: 4_700_000,
            fail: DemoFail::None,
            card_full_bytes: 12 * 1024 * 1024,
            send_ticks: 5,
            confirm_ticks: 3,
            confirm_deadline: Duration::from_secs(5 * 60),
        }
    }
}

/// In-memory camera that advances on [`tick`](Self::tick).
#[derive(Debug)]
pub struct FakeCamera {
    config: FakeConfig,
    camera: Camera,
    package: PackageInfo,
    flow: UpdateFlow,
    kind: Option<UpdateKind>,
    send_tick: u32,
    wait_tick: u32,
    /// Version after a successful install; unchanged on errors.
    live_version: String,
    started_at: Instant,
}

impl FakeCamera {
    pub fn new(config: FakeConfig) -> Self {
        let camera = Camera {
            model: config.model,
            firmware: config.firmware.clone(),
            installed_version: config.installed.clone(),
            connection: ConnectionState::Nearby,
        };
        let package = PackageInfo {
            installed: config.installed.clone(),
            available: Some(config.available.clone()),
            firmware: config.firmware.clone(),
            release_notes: config.release_notes.clone(),
            package_bytes: config.package_bytes,
        };
        let live_version = config.installed.clone();
        Self {
            config,
            camera,
            package,
            flow: UpdateFlow::new(),
            kind: None,
            send_tick: 0,
            wait_tick: 0,
            live_version,
            started_at: Instant::now(),
        }
    }

    pub fn demo() -> Self {
        Self::new(FakeConfig::default())
    }

    pub fn with_fail(fail: DemoFail) -> Self {
        let config = if fail == DemoFail::NotConfirmed {
            FakeConfig {
                fail,
                confirm_deadline: Duration::from_millis(50),
                confirm_ticks: 100,
                ..FakeConfig::default()
            }
        } else {
            FakeConfig {
                fail,
                ..FakeConfig::default()
            }
        };
        Self::new(config)
    }

    pub fn flow(&self) -> &UpdateFlow {
        &self.flow
    }

    pub fn live_version(&self) -> &str {
        &self.live_version
    }

    pub fn config(&self) -> &FakeConfig {
        &self.config
    }

    /// Demo screenshot: installed is current.
    pub fn set_up_to_date(&mut self, version: &str) {
        self.live_version = version.to_owned();
        self.camera.installed_version = version.to_owned();
        self.camera.connection = ConnectionState::Connected;
        self.package.installed = version.to_owned();
        self.package.available = None;
        self.flow.reset();
    }

    /// Demo screenshot: mid-send at `progress` (0..=1).
    pub fn seed_sending(&mut self, kind: UpdateKind, progress: f32) {
        self.flow.reset();
        self.flow.begin_sending(kind, self.live_version.clone());
        self.flow.set_progress(progress.clamp(0.0, 1.0));
        self.kind = Some(kind);
        self.send_tick = ((progress.clamp(0.0, 1.0)) * self.config.send_ticks as f32) as u32;
        self.wait_tick = 0;
    }

    /// Demo screenshot: waiting for confirmation on the camera.
    pub fn seed_waiting(&mut self, kind: UpdateKind) {
        self.seed_sending(kind, 1.0);
        self.flow.mark_waiting(Instant::now());
        self.wait_tick = 0;
    }

    /// Demo screenshot: restart step after confirmation.
    pub fn seed_restart(&mut self, kind: UpdateKind) {
        self.seed_waiting(kind);
        self.flow.mark_confirmed();
        self.flow.mark_restart();
    }

    /// Advance the scripted timeline by one step.
    pub fn tick(&mut self) {
        match self.flow.phase().clone() {
            UpdatePhase::Sending { kind, .. } => {
                if self.config.fail == DemoFail::CardFull && self.send_tick >= 1 {
                    self.flow.fail(UpdateError::CardFull {
                        bytes_needed: self.config.card_full_bytes,
                    });
                    return;
                }
                if self.config.fail == DemoFail::Refused && self.send_tick >= 1 {
                    self.flow.fail(UpdateError::Refused);
                    return;
                }
                if self.config.fail == DemoFail::ConnectionLost && self.send_tick >= 1 {
                    self.flow.fail(UpdateError::ConnectionLost);
                    self.camera.connection = ConnectionState::Disconnected;
                    return;
                }
                self.send_tick += 1;
                let progress =
                    (self.send_tick as f32 / self.config.send_ticks.max(1) as f32).min(1.0);
                self.flow.set_progress(progress);
                if self.send_tick >= self.config.send_ticks {
                    self.flow.mark_waiting(self.started_at);
                }
                let _ = kind;
            }
            UpdatePhase::WaitingConfirm { .. } => {
                if self.config.fail == DemoFail::NotConfirmed {
                    self.wait_tick += 1;
                    if self.wait_tick >= 2 {
                        self.flow.fail(UpdateError::NotConfirmed);
                    }
                    return;
                }
                if self.config.fail == DemoFail::ConnectionLost {
                    self.flow.fail(UpdateError::ConnectionLost);
                    self.camera.connection = ConnectionState::Disconnected;
                    return;
                }
                self.wait_tick += 1;
                if self.wait_tick >= self.config.confirm_ticks {
                    self.flow.mark_confirmed();
                }
            }
            UpdatePhase::Confirmed { .. } => {
                self.flow.mark_restart();
            }
            // The camera installs at its next start: see [`Self::restart`].
            _ => {}
        }
    }

    /// The camera restarted after a confirmed update: it runs the new
    /// version.
    pub fn restart(&mut self) {
        if !matches!(
            self.flow.phase(),
            UpdatePhase::Restart { .. } | UpdatePhase::Confirmed { .. }
        ) {
            return;
        }
        self.flow.mark_restart();
        let version = self.config.available.clone();
        self.live_version = version.clone();
        self.camera.installed_version = version.clone();
        self.package.installed = version.clone();
        self.package.available = None;
        self.flow.mark_installed(version);
        self.kind = None;
    }

    fn sync_package_from_live(&mut self) {
        self.package.installed = self.live_version.clone();
        self.camera.installed_version = self.live_version.clone();
    }
}

impl CameraLink for FakeCamera {
    fn nearby(&self) -> Option<CameraModel> {
        if matches!(
            self.camera.connection,
            ConnectionState::Nearby | ConnectionState::Connecting | ConnectionState::Connected
        ) {
            Some(self.camera.model)
        } else if self.camera.connection == ConnectionState::Disconnected
            && !matches!(
                self.flow.phase(),
                UpdatePhase::Failed(UpdateError::ConnectionLost)
            )
        {
            // Demo starts nearby; after a lost connection it stays away until reset.
            Some(self.camera.model)
        } else {
            None
        }
    }

    async fn connect(&mut self) -> Result<CameraStatus, LinkError> {
        if self.camera.connection == ConnectionState::Disconnected
            && matches!(
                self.flow.phase(),
                UpdatePhase::Failed(UpdateError::ConnectionLost)
            )
        {
            return Err(LinkError::ConnectionLost);
        }
        self.camera.connection = ConnectionState::Connected;
        self.sync_package_from_live();
        Ok(CameraStatus {
            camera: self.camera.clone(),
            powered_on: true,
            device_id: None,
        })
    }

    async fn read_status(&self) -> Result<CameraStatus, LinkError> {
        if self.camera.connection == ConnectionState::Disconnected {
            return Err(LinkError::ConnectionLost);
        }
        Ok(CameraStatus {
            camera: self.camera.clone(),
            powered_on: true,
            device_id: None,
        })
    }

    async fn read_package(&self) -> Result<PackageInfo, LinkError> {
        if self.camera.connection != ConnectionState::Connected {
            return Err(LinkError::NotNearby);
        }
        Ok(self.package.clone())
    }

    async fn send_update(&mut self, kind: UpdateKind) -> Result<(), LinkError> {
        if self.camera.connection != ConnectionState::Connected {
            return Err(LinkError::NotNearby);
        }
        if self.flow.is_busy() {
            return Err(LinkError::Busy);
        }
        if self.package.available.is_none() {
            return Err(LinkError::Busy);
        }
        self.kind = Some(kind);
        self.send_tick = 0;
        self.wait_tick = 0;
        self.flow.reset();
        self.flow.begin_sending(kind, self.live_version.clone());
        Ok(())
    }

    async fn poll_update(&mut self) -> Result<UpdatePoll, LinkError> {
        self.tick();
        Ok(match self.flow.phase().clone() {
            UpdatePhase::Downloading { progress, .. } | UpdatePhase::Sending { progress, .. } => {
                UpdatePoll::Sending { progress }
            }
            UpdatePhase::WaitingConfirm { .. } => UpdatePoll::WaitingConfirm,
            UpdatePhase::Confirmed { .. }
            | UpdatePhase::Restart { .. }
            | UpdatePhase::Restarting { .. }
            | UpdatePhase::ManualRestart { .. } => UpdatePoll::Confirmed,
            UpdatePhase::Installed { version, .. } => UpdatePoll::Installed { version },
            UpdatePhase::Failed(error) => UpdatePoll::Failed(error),
            UpdatePhase::Idle => UpdatePoll::Sending { progress: 0.0 },
        })
    }

    async fn cancel(&mut self) -> Result<(), LinkError> {
        if matches!(
            self.flow.phase(),
            UpdatePhase::Installed { .. } | UpdatePhase::Idle
        ) {
            return Ok(());
        }
        // Cancel leaves the live version alone.
        self.flow.reset();
        self.kind = None;
        self.send_tick = 0;
        self.wait_tick = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::block_on_ready;

    fn connected_demo() -> FakeCamera {
        let mut cam = FakeCamera::demo();
        block_on_ready(cam.connect()).unwrap();
        cam
    }

    #[test]
    fn happy_path_install() {
        let mut cam = connected_demo();
        let before = cam.live_version().to_owned();
        block_on_ready(cam.send_update(UpdateKind::Install)).unwrap();
        let mut saw_waiting = false;
        let mut installed = None;
        for _ in 0..40 {
            match block_on_ready(cam.poll_update()).unwrap() {
                UpdatePoll::WaitingConfirm => saw_waiting = true,
                // The new version runs only after a restart.
                UpdatePoll::Confirmed => cam.restart(),
                UpdatePoll::Installed { version } => {
                    installed = Some(version);
                    break;
                }
                UpdatePoll::Failed(e) => panic!("unexpected {e:?}"),
                _ => {}
            }
        }
        assert!(saw_waiting);
        assert_eq!(installed.as_deref(), Some("1.7.0"));
        assert_eq!(cam.live_version(), "1.7.0");
        assert_ne!(cam.live_version(), before);
    }

    #[test]
    fn card_full_leaves_package_unchanged() {
        let mut cam = FakeCamera::with_fail(DemoFail::CardFull);
        block_on_ready(cam.connect()).unwrap();
        let before = cam.live_version().to_owned();
        block_on_ready(cam.send_update(UpdateKind::Install)).unwrap();
        let mut err = None;
        for _ in 0..10 {
            if let UpdatePoll::Failed(e) = block_on_ready(cam.poll_update()).unwrap() {
                err = Some(e);
                break;
            }
        }
        assert_eq!(
            err,
            Some(UpdateError::CardFull {
                bytes_needed: 12 * 1024 * 1024
            })
        );
        assert_eq!(cam.live_version(), before);
    }

    #[test]
    fn refused_leaves_package_unchanged() {
        let mut cam = FakeCamera::with_fail(DemoFail::Refused);
        block_on_ready(cam.connect()).unwrap();
        let before = cam.live_version().to_owned();
        block_on_ready(cam.send_update(UpdateKind::Install)).unwrap();
        let mut err = None;
        for _ in 0..10 {
            if let UpdatePoll::Failed(e) = block_on_ready(cam.poll_update()).unwrap() {
                err = Some(e);
                break;
            }
        }
        assert_eq!(err, Some(UpdateError::Refused));
        assert_eq!(cam.live_version(), before);
    }

    #[test]
    fn not_confirmed_leaves_package_unchanged() {
        let mut cam = FakeCamera::with_fail(DemoFail::NotConfirmed);
        block_on_ready(cam.connect()).unwrap();
        let before = cam.live_version().to_owned();
        block_on_ready(cam.send_update(UpdateKind::Install)).unwrap();
        let mut err = None;
        for _ in 0..20 {
            if let UpdatePoll::Failed(e) = block_on_ready(cam.poll_update()).unwrap() {
                err = Some(e);
                break;
            }
        }
        assert_eq!(err, Some(UpdateError::NotConfirmed));
        assert_eq!(cam.live_version(), before);
    }

    #[test]
    fn connection_lost_leaves_package_unchanged() {
        let mut cam = FakeCamera::with_fail(DemoFail::ConnectionLost);
        block_on_ready(cam.connect()).unwrap();
        let before = cam.live_version().to_owned();
        block_on_ready(cam.send_update(UpdateKind::Install)).unwrap();
        let mut err = None;
        for _ in 0..10 {
            if let UpdatePoll::Failed(e) = block_on_ready(cam.poll_update()).unwrap() {
                err = Some(e);
                break;
            }
        }
        assert_eq!(err, Some(UpdateError::ConnectionLost));
        assert_eq!(cam.live_version(), before);
    }
}
