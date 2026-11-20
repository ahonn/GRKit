//! GRKit phone app: update GRKit on a RICOH GR camera over Wi-Fi.
//!
//! Layers, top down:
//! - `shell`: tab bar and navigation stacks
//! - `pages`: Camera, Settings, Software Update, sheets
//! - `ui`: shared phone chrome
//! - `store`: app state and demo mode
//! - `domain`: camera model, update flow, fake camera link
//! - `platform`: Bluetooth / Wi-Fi bridges (iOS / Android) or desktop stubs
//! - `strings`: English and Simplified Chinese copy

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

pub mod domain;
mod pages;
mod platform;
mod shell;
pub mod store;
mod strings;
mod strings_connect;
mod strings_looks;
mod strings_recipes;
mod ui;

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::store::AppState;

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
    let app = AppState::new();
    app.ensure_ticker();
    provide_context(app.clone());
    view()
        .flex_1()
        .bg(move || theme().background)
        .text_color(move || theme().foreground)
        .child(shell::shell(app.clone()))
        .child(pages::overlay(app))
}
