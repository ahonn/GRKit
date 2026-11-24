//! Screens for the Camera, Looks, Recipes, Modules and Settings tabs.

mod camera;
mod cover_editor;
pub mod grkit_settings;
mod look_add;
pub(crate) mod look_pages;
mod looks;
mod module_pages;
mod modules;
mod recipe_pages;
mod recipe_text;
mod recipes;
mod settings;
mod sheets;
mod software;

pub use camera::camera_root;
pub use cover_editor::{CoverTarget, cover_page};
pub use look_add::{add_page, unlock_page};
pub use look_pages::{confirm_delete, import_page, look_page};
pub use looks::{import_sheet, looks_root, scheme_page};
pub use module_pages::{module_options_page, module_page, value_menu};
pub use modules::modules_page;
pub use recipe_pages::{preset_page, presets_page, recipe_page};
pub use recipes::{confirm_delete_recipe, recipes_root};
pub use settings::{confirm_disconnect, confirm_power_off, saved_cameras_page, settings_root};
pub use sheets::overlay;
pub use software::{release_history_page, software_page};
