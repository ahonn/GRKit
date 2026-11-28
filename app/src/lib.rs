//! GRKit phone app: update GRKit on a RICOH GR camera over Wi-Fi.
//!
//! Layers, top down:
//! - `dev`: demo mode and development hooks (`cfg(demo)`: debug builds and
//!   the `demo` feature)
//! - `shell`: tab bar and navigation stacks
//! - `pages`: Camera, Settings, Software Update, sheets, alerts
//! - `ui`: shared phone chrome
//! - `store`: app state
//! - `camera_api`: the camera service, the release site and the phone's
//!   records, as the store uses them
//! - `domain`: camera model, update flow, request and answer types
//! - `platform`: Bluetooth / Wi-Fi bridges (iOS / Android) or desktop stubs,
//!   and the HTTP clients
//! - `strings`: UI text from `locales/app.yml` in the system language

#![allow(clippy::clone_on_copy)]

/// Logs a connection step. On Android through `log` to logcat (NSH's
/// logger keeps info there: `adb logcat -s nsh`). Elsewhere to stderr,
/// which `devicectl --console` and Xcode show, because NSH's logger keeps
/// only warnings and errors in packaged builds there.
#[cfg(target_os = "android")]
macro_rules! app_info {
    ($($arg:tt)*) => {
        log::info!(target: "grkit", "{}", format_args!($($arg)*))
    };
}

#[cfg(not(target_os = "android"))]
macro_rules! app_info {
    ($($arg:tt)*) => {
        eprintln!("GRKit {}", format_args!($($arg)*))
    };
}

mod camera_api;
#[cfg(demo)]
pub mod dev;
pub mod domain;
mod pages;
mod platform;
mod shell;
pub mod store;
mod strings;
mod ui;

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::store::AppState;
pub use crate::strings::translations;

/// The crate's native views on iOS: the platform loader
/// (`ui::ACTIVITY_COMPONENT`). Android registers its own in `MainActivity`.
#[native_components(ios)]
fn components(components: &nsh_platform_ios::Components) {
    platform::register_ios_components(components);
}

/// The crate's UI set (`nsh.json` `"rust"`, `"root": "App"`).
#[native_module("grkit/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>().assets(assets!());
}

/// Application root.
#[component]
pub fn App() -> impl IntoElement {
    let app = launch();
    app.ensure_ticker();
    pages::present_alerts(app);
    provide_context(app.clone());
    view()
        .flex_1()
        .bg(move || theme().background)
        .text_color(move || theme().foreground)
        .child(shell::shell(app.clone()))
        .child(pages::overlay(app))
}

/// The app state on the camera that this build talks to: a real one, or in
/// demo mode the demo camera.
fn launch() -> AppState {
    #[cfg(demo)]
    {
        dev::launch()
    }
    #[cfg(not(demo))]
    {
        AppState::new(std::rc::Rc::new(camera_api::HttpCamera))
    }
}
