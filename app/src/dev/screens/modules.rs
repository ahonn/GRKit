//! Camera modules.

use nsh_core::prelude::*;

use super::DemoScreen;
use crate::dev::fake_modules;
use crate::domain::LookError;
use crate::store::{ModulePart, ModulesBlock, Route, Tab};

pub(super) const SCREENS: &[DemoScreen] = &[
    // M1 — Modules list.
    DemoScreen {
        id: "M1",
        key_text: "Same order as ADJ on the camera. A switch applies at once.",
        alert: false,
        seed: |seed| seed.modules(),
    },
    // M2 — Edit the module order.
    DemoScreen {
        id: "M2",
        key_text: "Top Row · 2/4",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.modules();
            app.seed_modules_editing();
        },
    },
    // M3 — A module with values only.
    DemoScreen {
        id: "M3",
        key_text: "Strength",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.modules();
            app.push(Route::ModuleDetail("GRAIN".into()));
        },
    },
    // M4 — A module with options and order.
    DemoScreen {
        id: "M4",
        key_text: "Options and Order",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.modules();
            app.push(Route::ModuleDetail("FRAMING".into()));
        },
    },
    // M5 — Options and order.
    DemoScreen {
        id: "M5",
        key_text: "Shown in ADJ",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.modules();
            app.push(Route::ModuleDetail("FRAMING".into()));
            app.push(Route::ModuleOptions("FRAMING".into()));
        },
    },
    // M6 — Choose a value (menu).
    DemoScreen {
        id: "M6",
        key_text: "Strength",
        alert: true,
        seed: |seed| {
            let app = seed.app;
            seed.modules();
            app.push(Route::ModuleDetail("GRAIN".into()));
            spawn(async move {
                next_frame().await;
                next_frame().await;
                if let Some(row) = app.module_row("GRAIN") {
                    let setting = row.settings()[0].clone();
                    let current = row.value_of(&setting);
                    crate::pages::value_menu(app, "GRAIN".into(), setting, current).show();
                }
            });
        },
    },
    // M7 — A module with groups.
    DemoScreen {
        id: "M7",
        key_text: "Halation",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.modules();
            app.push(Route::ModuleDetail("GLOW".into()));
        },
    },
    // M8 — A module managed by another tab.
    DemoScreen {
        id: "M8",
        key_text: "Manage Film Looks",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.modules();
            app.refresh_looks();
            app.push(Route::ModuleDetail("FILM".into()));
        },
    },
    // M9 — Modules turned off.
    DemoScreen {
        id: "M9",
        key_text: "Not Used",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            let demo = seed.demo;
            demo.edit_modules(|state| {
                for module in &mut state.modules {
                    if matches!(module.id.as_str(), "GRAIN" | "BORDER" | "DATE") {
                        module.enabled = false;
                        module.status = "stopped".into();
                    }
                }
            });
            seed.connected_up_to_date();
            app.set_sheet(None);
            app.open_modules_page();
        },
    },
    // M10 — No camera: the Modules page has no way in, the home page shows.
    DemoScreen {
        id: "M10",
        key_text: "Add Your Camera",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.disconnected();
            app.set_sheet(None);
            // Without a camera, the Modules page has no way in.
            app.set_tab(Tab::Camera);
        },
    },
    // M11 — The camera is busy (alert).
    DemoScreen {
        id: "M11",
        key_text: "Camera Busy",
        alert: true,
        seed: |seed| {
            let app = seed.app;
            seed.modules();
            app.alert_module_error(LookError::CameraBusy);
        },
    },
    // M12 — A value being saved.
    DemoScreen {
        id: "M12",
        key_text: "Strength",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.modules();
            app.seed_module_saving("GRAIN", ModulePart::Setting("grain.level".into()));
            app.push(Route::ModuleDetail("GRAIN".into()));
        },
    },
    // M13 — GRKit too old for module settings.
    DemoScreen {
        id: "M13",
        key_text: "Update GRKit",
        alert: false,
        seed: |seed| seed.modules_blocked(ModulesBlock::Unsupported),
    },
    // M14 — A catalog newer than the app.
    DemoScreen {
        id: "M14",
        key_text: "Update the App",
        alert: false,
        seed: |seed| seed.modules_blocked(ModulesBlock::AppTooOld),
    },
    // M15 — Modules being read.
    DemoScreen {
        id: "M15",
        key_text: "label:Reading modules…",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_up_to_date();
            app.set_sheet(None);
            app.open_modules_page();
            app.seed_modules_loading();
        },
    },
    // M16 — A saved camera, not connected: the home page shows.
    DemoScreen {
        id: "M16",
        key_text: "Not Connected",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.saved();
            app.seed_modules(
                fake_modules::demo_catalog(crate::strings::app_language()),
                fake_modules::demo_state(),
            );
            seed.disconnected();
            app.set_sheet(None);
            // Without a camera, the Modules page has no way in.
            app.set_tab(Tab::Camera);
        },
    },
];
