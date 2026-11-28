//! Demo mode and the development hooks. Compiled with `cfg(demo)` (debug
//! builds, or the `demo` feature): a release build has none of it, and
//! always talks to a real camera.
//!
//! - `GRKIT_DEMO=1`, `--demo` or `demo`: the app runs on the demo camera
//!   ([`FakeCamera`]), which answers every request from memory.
//! - `GRKIT_DEMO_FAIL`: a demo update fails (`card_full`, `refused`,
//!   `connection_lost`).
//! - `GRKIT_DEMO_SCREEN` or `--demo-screen <id>`: open a design screen
//!   ([`DemoScreen`]); it turns demo mode on.
//! - `GRKIT_START_TAB`: open a tab at launch (`looks`, `recipes`,
//!   `modules`, `looks-add`), for simulator runs without taps.
//!
//! `GRKIT_CAMERA_URL` (HTTP only, without Bluetooth) is read by the store,
//! under the same `cfg(demo)`.

mod fake;
pub mod fake_looks;
pub mod fake_modules;
mod fake_recipes;
mod screens;

use std::{env, rc::Rc};

use grkit_core::release_index::IndexPreset;

use crate::camera_api::HttpCamera;
use crate::store::{AppState, Tab};

pub use fake::{DemoFail, FakeCamera, FakeConfig, ModuleSupport};
pub use screens::{DemoScreen, demo_screen_from_env, demo_screens, parse_demo_screen};

/// The app state at launch: on the demo camera when demo mode is asked
/// for, else on a real one. Opens the tab that `GRKIT_START_TAB` names.
pub fn launch() -> AppState {
    if demo_requested() {
        let camera = FakeCamera::new(FakeConfig {
            fail: DemoFail::from_env(),
            ..FakeConfig::default()
        });
        let app = AppState::new(Rc::new(camera.clone()));
        // The site's presets, as the release check gives them.
        app.seed_presets(IndexPreset::all(&Default::default()));
        open_start_tab(app);
        if let Some(screen) = demo_screen_from_env() {
            screens::apply(app, &camera, screen);
        }
        app
    } else {
        let app = AppState::new(Rc::new(HttpCamera));
        open_start_tab(app);
        app
    }
}

fn demo_requested() -> bool {
    if env::var("GRKIT_DEMO").as_deref() == Ok("1")
        || crate::platform::config_value("GRKIT_DEMO_SCREEN").is_some()
    {
        return true;
    }
    env::args().any(|arg| {
        arg == "--demo"
            || arg == "demo"
            || arg == "--demo-screen"
            || arg.starts_with("--demo-screen=")
    })
}

/// Development: open a tab at launch, for simulator runs without taps.
fn open_start_tab(app: AppState) {
    match crate::platform::config_value("GRKIT_START_TAB").as_deref() {
        Some("looks") => app.seed_tab(Tab::Looks),
        Some("recipes") => app.set_tab(Tab::Recipes),
        Some("modules") => app.open_modules_page(),
        Some("looks-add") => {
            app.seed_tab(Tab::Looks);
            app.open_add();
        }
        _ => {}
    }
}
