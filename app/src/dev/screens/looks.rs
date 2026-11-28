//! The Looks tab.

use nsh_core::prelude::*;

use super::DemoScreen;
use super::{EKTAR, PORTRA};
use crate::dev::fake_looks::DEMO_CUSTOM_ID;
use crate::domain::looks::LookUse;
use crate::domain::{JournalState, LinkError, LookError};

const TRIX: &str = "trix-400-mono-v1";
use crate::store::{Alert, Route, Sheet, Tab};

pub(super) const SCREENS: &[DemoScreen] = &[
    // L1 — Looks on the camera, in menu order.
    DemoScreen {
        id: "L1",
        key_text: "In Camera Menu",
        alert: false,
        seed: |seed| seed.looks(),
    },
    // L2 — an official Look.
    DemoScreen {
        id: "L2",
        key_text: "Enable",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.looks();
            app.push(Route::LookDetail(PORTRA.into()));
        },
    },
    // L3 — an imported Look.
    DemoScreen {
        id: "L3",
        key_text: "My LUT",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.looks();
            app.push(Route::LookDetail(DEMO_CUSTOM_ID.into()));
        },
    },
    // L4 — a new Look converted, ready to send.
    DemoScreen {
        id: "L4",
        key_text: "Put on Camera",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.looks();
            seed.import_draft();
            app.push(Route::LookImport);
            app.finish_import();
        },
    },
    // L5 — delete the Look in use (alert).
    DemoScreen {
        id: "L5",
        key_text: "Delete Portra 400?",
        alert: true,
        seed: |seed| {
            let app = seed.app;
            let demo = seed.demo;
            demo.edit_looks(|f| f.set_uses(PORTRA, Vec::new()));
            seed.looks();
            app.push(Route::LookDetail(PORTRA.into()));
            seed.ask_delete();
        },
    },
    // L6 — a Look that recipes use cannot be deleted (alert).
    DemoScreen {
        id: "L6",
        key_text: "Can’t Delete Portra 400",
        alert: true,
        seed: |seed| {
            let app = seed.app;
            seed.looks();
            app.push(Route::LookDetail(PORTRA.into()));
            seed.ask_delete();
        },
    },
    // L7 — the Add page, one Look being added.
    DemoScreen {
        id: "L7",
        key_text: "Import a File",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.looks();
            app.open_add();
            app.seed_adding("provia-100f-v1");
        },
    },
    // L8 — import a Look sheet.
    DemoScreen {
        id: "L8",
        key_text: "Lightroom Preset",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.looks();
            app.set_sheet(Some(Sheet::ImportLook));
        },
    },
    // L9 — a new Look: file, name, LUT input and cover.
    DemoScreen {
        id: "L9",
        key_text: "LUT Input",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.looks();
            seed.import_draft();
            app.push(Route::LookImport);
        },
    },
    // L10 — The Looks tab without a camera: the home page shows.
    DemoScreen {
        id: "L10",
        key_text: "Add Your Camera",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.disconnected();
            app.set_sheet(None);
            app.set_tab(Tab::Looks);
        },
    },
    // L11 — an update waits on the camera (read only).
    DemoScreen {
        id: "L11",
        key_text: "An update is waiting on the camera",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            let demo = seed.demo;
            seed.looks();
            // An update waits on the camera, which refuses Look changes.
            demo.set_journal(JournalState::Waiting);
            app.refresh_looks();
        },
    },
    // L12 — no room on the camera (alert).
    DemoScreen {
        id: "L12",
        key_text: "Camera Menu Is Full",
        alert: true,
        seed: |seed| {
            let app = seed.app;
            seed.looks();
            app.open_add();
            seed.alert_after_frames(Alert::Look(LookError::NoRoom));
        },
    },
    // L13 — the cover editor of an official Look.
    DemoScreen {
        id: "L13",
        key_text: "Film name",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.looks();
            app.push(Route::LookDetail(PORTRA.into()));
            app.push(Route::LookCover(PORTRA.into()));
        },
    },
    // L14 — changes wait for a restart of the camera.
    DemoScreen {
        id: "L14",
        key_text: "1 change applies after a restart",
        alert: false,
        seed: |seed| {
            let demo = seed.demo;
            demo.edit_looks(|f| f.set_pending(DEMO_CUSTOM_ID));
            seed.looks();
        },
    },
    // L15 — a Look detail.
    DemoScreen {
        id: "L15",
        key_text: "Portra 400",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.looks();
            app.push(Route::LookDetail(PORTRA.into()));
        },
    },
    // L17 — rename a Look.
    DemoScreen {
        id: "L17",
        key_text: "Name",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.looks();
            app.push(Route::LookDetail(PORTRA.into()));
            spawn(async move {
                timer(std::time::Duration::from_millis(600)).await;
                app.ask_rename_look(PORTRA.into());
            });
        },
    },
    // L18 — the card holds as many Looks as it can (alert).
    DemoScreen {
        id: "L18",
        key_text: "No Room on the Card",
        alert: true,
        seed: |seed| {
            let app = seed.app;
            seed.looks();
            app.open_add();
            seed.alert_after_frames(Alert::Look(LookError::CardFull));
        },
    },
    // L19 — the connection was lost during a change.
    DemoScreen {
        id: "L19",
        key_text: "Disconnected. This change wasn't saved.",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.looks();
            app.seed_status(None);
            app.alert_look_error(LookError::ConnectionLost);
        },
    },
    // L20 — a restart for changed Looks, on the Looks tab.
    DemoScreen {
        id: "L20",
        key_text: "Camera Not Connected",
        alert: false,
        seed: |seed| seed.look_restart(),
    },
    // L21 — the connect that ends the restart.
    DemoScreen {
        id: "L21",
        key_text: "Camera Not Connected",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.look_restart();
            app.seed_look_restart(false, true);
            app.set_sheet(Some(Sheet::Connecting { step: 1 }));
        },
    },
    // L22 — that connect failed.
    DemoScreen {
        id: "L22",
        key_text: "Camera Not Connected",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.look_restart();
            app.seed_look_restart(false, true);
            app.set_sheet(Some(Sheet::ConnectError(LinkError::NotNearby)));
        },
    },
    // L23 — the camera has no standby: restart it by hand (alert).
    DemoScreen {
        id: "L23",
        key_text: "Restart the Camera",
        alert: true,
        seed: |seed| {
            seed.look_restart();
            seed.alert_after_frames(Alert::ManualRestart);
        },
    },
    // L24 — the camera is full, with changes waiting for a restart.
    DemoScreen {
        id: "L24",
        key_text: "40/40 on Camera · Full",
        alert: false,
        seed: |seed| {
            seed.demo.edit_looks(|f| {
                f.fill_camera();
                f.set_pending(DEMO_CUSTOM_ID);
            });
            seed.looks();
        },
    },
    // L25 — a Look on the card only, used by a recipe.
    DemoScreen {
        id: "L25",
        key_text: "Used by Recipes",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.recipe_on(2, EKTAR);
            seed.looks();
            app.refresh_recipes();
            app.push(Route::LookDetail(EKTAR.0.into()));
        },
    },
    // L26 — a Look on the card only, and the camera is full.
    DemoScreen {
        id: "L26",
        key_text: "The camera is full (40/40). Move a film look off the camera first.",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.demo.edit_looks(|f| f.fill_camera());
            seed.recipe_on(2, EKTAR);
            seed.looks();
            app.refresh_recipes();
            app.push(Route::LookDetail(EKTAR.0.into()));
        },
    },
    // L27 — move a Look that a recipe uses off the camera (alert).
    DemoScreen {
        id: "L27",
        key_text: "Move Tri-X 400 Off the Camera?",
        alert: true,
        seed: |seed| {
            let app = seed.app;
            seed.demo.edit_looks(|f| {
                f.set_uses(
                    TRIX,
                    vec![LookUse::Recipe {
                        slot: 1,
                        name: Some("Street".into()),
                        number: 0,
                    }],
                )
            });
            seed.looks();
            app.push(Route::LookDetail(TRIX.into()));
            spawn(async move {
                next_frame().await;
                next_frame().await;
                if let Some(look) = app.look(TRIX) {
                    crate::pages::look_pages::confirm_move_to_card(app, look);
                }
            });
        },
    },
    // L28 — the Add page when the camera is full.
    DemoScreen {
        id: "L28",
        key_text: "New film looks stay on the card only.",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.demo.edit_looks(|f| f.fill_camera());
            seed.looks();
            app.open_add();
        },
    },
];
