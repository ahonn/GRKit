//! Shared phone chrome: grouped rows, camera art, primary buttons.

mod chrome;

pub use chrome::*;

mod loading;
pub use loading::{camera_skeleton, update_skeleton};

mod page_scroll;
pub use page_scroll::page_scroll;

mod toast;
pub use toast::{Toasts, toast_layer};
