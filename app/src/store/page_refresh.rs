//! What each page reloads when it is pulled down (`ui::page_scroll`).

use nsh_core::prelude::*;

use super::AppState;

/// The pages with pull to refresh, as `page_scroll().refresh(..)` names them.
pub const CAMERA: &str = "camera";
pub const MODULES: &str = "modules";
pub const LOOKS: &str = "looks";
pub const RECIPES: &str = "recipes";
pub const SOFTWARE: &str = "software";
pub const SETTINGS: &str = "settings";

impl AppState {
    /// Reload what the page shows. The camera page reads the camera's
    /// version and contents again and checks the release site.
    pub fn refresh_page(self, page: &str) {
        match page {
            CAMERA => {
                if self.camera_online() && !self.update_busy.get() {
                    spawn(async move { self.refresh_camera_package().await });
                }
                self.refresh_update_info();
                self.refresh_modules();
                self.refresh_looks();
                self.refresh_recipes();
            }
            MODULES => self.refresh_modules(),
            LOOKS => self.refresh_looks(),
            RECIPES => self.refresh_recipes(),
            SOFTWARE => self.refresh_update_info(),
            SETTINGS => self.refresh_camera_settings(),
            _ => {}
        }
    }

    /// A reload of the page is under way.
    pub fn page_refreshing(self, page: &str) -> bool {
        match page {
            CAMERA => {
                self.release_checking()
                    || self.modules_loading()
                    || self.looks_loading()
                    || self.recipes_loading()
            }
            MODULES => self.modules_loading(),
            LOOKS => self.looks_loading(),
            RECIPES => self.recipes_loading(),
            SOFTWARE => self.release_checking(),
            SETTINGS => self.camera_settings_loading(),
            _ => false,
        }
    }
}
