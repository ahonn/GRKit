//! The link to a simulated camera (`CameraApi::simulated`, the demo camera),
//! which has no Bluetooth or Wi-Fi: a connect and a restart step on a
//! timer, and end as a real one does. Everything else reaches the camera
//! through `CameraApi`, as with a real camera.

use nsh_core::prelude::*;

use super::{AppState, Sheet, connect::Reached};
use crate::domain::{
    CameraModel, ConnectOptions, NearbyCamera, RestartStep, UpdateIntent, UpdatePhase,
};

/// The peripheral ID of the simulated camera.
const SIMULATED_ID: &str = "demo";
/// How long each step of a simulated connect takes.
const CONNECT_STEP_MS: u128 = 250;
/// How long each step of a simulated restart takes.
const RESTART_STEP: std::time::Duration = std::time::Duration::from_millis(900);

impl AppState {
    /// The simulated camera, always nearby.
    pub(super) fn simulated_nearby(self) -> Vec<NearbyCamera> {
        self.api()
            .simulated()
            .map(|model| {
                vec![NearbyCamera {
                    peripheral_id: SIMULATED_ID.into(),
                    model,
                    powered_on: true,
                    name: Some("GR_A1B2C3".into()),
                    rssi: Some(-45),
                }]
            })
            .unwrap_or_default()
    }

    /// Advance the connect sheet. After its steps the camera answers, and
    /// the connect ends as one over the network does.
    pub(super) fn tick_connecting(self) {
        let Some(started) = self.connect_started.get() else {
            return;
        };
        if !matches!(self.sheet.get(), Some(Sheet::Connecting { .. })) {
            return;
        }
        let step = (started.elapsed().as_millis() / CONNECT_STEP_MS) as u8;
        if step < 3 {
            self.sheet.set(Some(Sheet::Connecting { step }));
            return;
        }
        let id = self
            .selected_id
            .get()
            .unwrap_or_else(|| SIMULATED_ID.to_owned());
        let model = self.selected_model().unwrap_or(CameraModel::Gr3);
        let read = self.api().read_package(model);
        self.run(async move {
            match read.await {
                Ok((status, package)) => self.finish_connect(
                    &id,
                    Reached::Network { status, package },
                    ConnectOptions::default(),
                ),
                Err(error) => {
                    self.connect_started.set(None);
                    self.sheet.set(Some(Sheet::ConnectError(error)));
                }
            }
        });
    }

    /// The phone's restart of the camera for an update: off, on, and the
    /// connect again.
    pub(super) async fn simulated_restart(self, intent: &UpdateIntent) -> bool {
        let kind = intent.kind;
        for step in [RestartStep::PoweringOn, RestartStep::Reconnecting] {
            timer(RESTART_STEP).await;
            self.phase.set(UpdatePhase::Restarting { kind, step });
        }
        timer(RESTART_STEP).await;
        self.simulated_rejoin(intent).await
    }

    /// The camera of `intent` started again: a confirmed update installs,
    /// and the phone connects to it.
    pub(super) async fn simulated_rejoin(self, intent: &UpdateIntent) -> bool {
        let api = self.api();
        api.finish(true).await;
        let model = api.simulated().unwrap_or(CameraModel::Gr3);
        match api.read_package(model).await {
            Ok((status, package)) => {
                self.finish_connect(
                    &intent.camera_id,
                    Reached::Network { status, package },
                    ConnectOptions::default(),
                );
                true
            }
            Err(_) => {
                self.phase
                    .set(UpdatePhase::ManualRestart { kind: intent.kind });
                false
            }
        }
    }
}
