//! iOS Bluetooth central, Wi-Fi join, and remember.

pub mod activity;
pub mod ble;
pub mod link;
mod local_network;
pub mod name_prompt;
pub mod remember;
pub mod wifi;

pub(super) use super::generic::{
    camera_request, config, connect_socket, network_snapshot, open_app_settings, watch_network_loss,
};
pub(super) use ble::{
    bluetooth_unavailable, end_session, release_link, start_scanning, stop_scanning,
    watch_disconnects,
};
pub(super) use link::connect as connect_camera;
pub(super) use name_prompt::ask as ask_name;
pub(super) use remember::{load as load_saved, save as save_saved};
