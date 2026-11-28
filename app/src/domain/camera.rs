//! A camera the app can update, and how far a connection has gone.

use serde::{Deserialize, Serialize};

/// Models the app can show and update.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CameraModel {
    Gr3,
    Gr3x,
    Gr4,
}

impl CameraModel {
    /// Marketing name shown in the UI.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Gr3 => "GR III",
            Self::Gr3x => "GR IIIx",
            Self::Gr4 => "GR IV",
        }
    }

    /// First-time pairing menu path on the camera.
    /// The camera menu path that starts pairing, in this model's words.
    pub fn pairing_path_key(self) -> &'static str {
        match self {
            Self::Gr3 | Self::Gr3x => "cam.path_gr3",
            Self::Gr4 => "cam.path_gr4",
        }
    }

    /// What the camera asks for while it pairs.
    pub fn pairing_step_key(self) -> &'static str {
        match self {
            Self::Gr3 | Self::Gr3x => "cam.step_pair_gr3",
            Self::Gr4 => "cam.step_pair_gr4",
        }
    }
}

/// How far the phone has gotten toward talking to the camera.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Nearby,
    Connecting,
    Connected,
}

/// What is installed on the camera and what an update would install.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageInfo {
    pub installed: String,
    pub available: Option<String>,
    pub firmware: String,
    pub release_notes: String,
    pub package_bytes: u64,
}

/// Snapshot of a camera for the UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Camera {
    pub model: CameraModel,
    pub firmware: String,
    pub installed_version: String,
    pub connection: ConnectionState,
}

impl Camera {
    pub fn new(
        model: CameraModel,
        firmware: impl Into<String>,
        installed_version: impl Into<String>,
    ) -> Self {
        Self {
            model,
            firmware: firmware.into(),
            installed_version: installed_version.into(),
            connection: ConnectionState::Disconnected,
        }
    }
}
