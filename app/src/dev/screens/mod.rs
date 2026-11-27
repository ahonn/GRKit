//! Demo screen ids for screenshots: `GRKIT_DEMO_SCREEN` / `--demo-screen`.
//!
//! When demo mode is on and an id is set, [`apply`] jumps the demo camera and
//! the UI to that design screen (sheet, push, or alert) and freezes the
//! ticker so the frame stays still for a screenshot. Unknown ids leave the
//! normal demo.
//!
//! Each tab has a table of its screens (`camera`, `update`, `looks`,
//! `recipes`, `modules`, `settings`): an id, what the screen sets up
//! ([`Seed`]), and the text that a headless test finds on it.

mod camera;
mod looks;
mod modules;
mod recipes;
mod seed;
mod settings;
mod update;

use std::env;

use super::fake::{FakeCamera, demo_notes};
use crate::store::AppState;
use seed::Seed;

const PORTRA: &str = "portra-400-endura-v1";

/// One design screen of the demo.
pub struct DemoScreen {
    /// The design id (`A5`).
    pub id: &'static str,
    /// Key English text a headless test looks for on the screen.
    pub key_text: &'static str,
    /// The key text is an alert title.
    pub alert: bool,
    /// What the screen sets up.
    seed: fn(Seed<'_>),
}

/// Every screen, tab by tab.
pub fn demo_screens() -> impl Iterator<Item = &'static DemoScreen> {
    [
        camera::SCREENS,
        update::SCREENS,
        looks::SCREENS,
        recipes::SCREENS,
        modules::SCREENS,
        settings::SCREENS,
    ]
    .into_iter()
    .flatten()
}

/// Parse a design id (`A5`, `a5`, …). Unknown → `None` (normal demo).
pub fn parse_demo_screen(id: &str) -> Option<&'static DemoScreen> {
    let id = id.trim();
    demo_screens().find(|screen| screen.id.eq_ignore_ascii_case(id))
}

/// Screen from `GRKIT_DEMO_SCREEN` or `--demo-screen <id>`.
pub fn demo_screen_from_env() -> Option<&'static DemoScreen> {
    if let Some(id) = crate::platform::config_value("GRKIT_DEMO_SCREEN")
        && let Some(screen) = parse_demo_screen(&id)
    {
        return Some(screen);
    }
    let mut args = env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        if arg == "--demo-screen" {
            return args.next().as_deref().and_then(parse_demo_screen);
        }
        if let Some(id) = arg.strip_prefix("--demo-screen=") {
            return parse_demo_screen(id);
        }
    }
    None
}

/// Jump `app` to `screen` and freeze timed advances for a stable screenshot.
pub fn apply(app: AppState, demo: &FakeCamera, screen: &DemoScreen) {
    app.set_frozen(true);
    app.seed_release_notes(demo_notes());
    app.seed_update_shown(true);
    (screen.seed)(Seed { app, demo });
}
