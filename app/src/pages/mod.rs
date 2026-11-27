//! Screens for the Camera, Looks, Recipes, Modules and Settings tabs, and
//! the alerts of the app.

mod alerts;
mod camera;
mod cover_editor;
pub mod grkit_settings;
mod look_add;
pub(crate) mod look_pages;
mod looks;
mod module_pages;
mod modules;
mod page_scroll;
mod recipe_pages;
mod recipe_text;
mod recipes;
mod settings;
mod sheets;
mod software;

pub use alerts::{ask_cancel_update, present_alerts};
pub use camera::camera_root;
pub use cover_editor::{CoverTarget, cover_page};
pub use look_add::{add_page, unlock_page};
#[cfg(demo)]
pub use look_pages::confirm_delete;
pub use look_pages::{import_page, look_page};
pub use looks::{import_sheet, looks_root};
#[cfg(demo)]
pub use module_pages::value_menu;
pub use module_pages::{module_options_page, module_page};
pub use modules::modules_page;
pub use page_scroll::page_scroll;
pub use recipe_pages::{preset_page, presets_page, recipe_page};
#[cfg(demo)]
pub use recipes::confirm_delete_recipe;
pub use recipes::recipes_root;
pub use settings::{confirm_disconnect, confirm_power_off, saved_cameras_page, settings_root};
pub use sheets::overlay;
pub use software::{release_history_page, software_page};
