//! Screens for the Camera, Looks and Settings tabs.

mod camera;
mod cover_editor;
mod look_add;
pub(crate) mod look_pages;
mod looks;
mod settings;
mod sheets;
mod software;

pub use camera::camera_root;
pub use cover_editor::{CoverTarget, cover_page};
pub use look_add::add_page;
pub use look_pages::{confirm_delete, import_page, look_page, rename_page};
pub use looks::{import_sheet, looks_root};
pub use settings::{saved_cameras_page, settings_root};
pub use sheets::overlay;
pub use software::software_page;
