//! Desktop placeholder: no Bluetooth and no camera link in this build.

use crate::domain::{CameraModel, ConnectOptions, ConnectStep, LinkError, RememberedCamera};

pub(super) use super::generic::{
    camera_request, config, connect_socket, network_snapshot, open_app_settings, watch_network_loss,
};

pub fn bluetooth_unavailable() -> bool {
    true
}

pub fn load_saved() -> Vec<RememberedCamera> {
    Vec::new()
}

pub fn save_saved(_cameras: &[RememberedCamera]) -> Result<(), String> {
    Ok(())
}

pub async fn end_session(_peripheral_id: &str, _power_off: bool) -> bool {
    false
}

pub fn start_scanning(
    _tx: async_channel::Sender<crate::domain::nearby_watch::Frame>,
    _remembered_id: Option<&str>,
    _ask: bool,
) {
}

pub fn stop_scanning() {}

pub fn release_link(_peripheral_id: &str) {}

pub fn watch_disconnects(_tx: async_channel::Sender<String>) {}

pub async fn connect_camera(
    _peripheral_id: &str,
    _model_hint: CameraModel,
    _options: ConnectOptions,
    _on_step: &mut dyn FnMut(ConnectStep),
) -> Result<super::Connected, LinkError> {
    Err(LinkError::BluetoothUnavailable)
}

/// No system text alert here. Tests answer through `GRKIT_TEST_NAME`;
/// `<keep>` saves the text the alert opened with.
pub async fn ask_name(prompt: super::NamePrompt) -> Option<String> {
    let answer = std::env::var("GRKIT_TEST_NAME").ok()?;
    Some(if answer == "<keep>" {
        prompt.value
    } else {
        answer
    })
}
