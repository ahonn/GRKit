//! Non-iOS placeholder: Bluetooth is not available in this build.

use crate::domain::{LinkError, RememberedCamera};

pub fn load_saved() -> Vec<RememberedCamera> {
    Vec::new()
}

#[allow(dead_code)]
pub fn save_saved(_cameras: &[RememberedCamera]) -> Result<(), String> {
    Ok(())
}

pub fn start_scanning(
    _tx: async_channel::Sender<crate::domain::nearby_watch::Frame>,
    _remembered_id: Option<&str>,
    _ask: bool,
) {
}

#[allow(dead_code)]
pub fn stop_scanning() {}

pub fn watch_disconnects(_tx: async_channel::Sender<String>) {}

pub async fn connect_camera() -> Result<super::Connected, LinkError> {
    Err(LinkError::BluetoothUnavailable)
}

/// No system text alert here. Tests answer through `GRKIT_TEST_NAME`;
/// `<keep>` saves the text the alert opened with.
pub fn ask_name(prompt: super::NamePrompt) -> Option<String> {
    let answer = std::env::var("GRKIT_TEST_NAME").ok()?;
    Some(if answer == "<keep>" {
        prompt.value
    } else {
        answer
    })
}
