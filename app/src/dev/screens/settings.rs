//! GRKit settings on the camera.

use super::DemoScreen;
use crate::store::Route;

pub(super) const SCREENS: &[DemoScreen] = &[
    // S1 — Shooting Settings of GRKit Settings.
    DemoScreen {
        id: "S1",
        key_text: "Uncropped RAW",
        alert: false,
        seed: |seed| seed.grkit_settings(Route::ShootingSettings),
    },
    // S2 — Display Settings of GRKit Settings.
    DemoScreen {
        id: "S2",
        key_text: "Battery Percentage",
        alert: false,
        seed: |seed| seed.grkit_settings(Route::DisplaySettings),
    },
    // S3 — Shutdown Screen.
    DemoScreen {
        id: "S3",
        key_text: "Show Text",
        alert: false,
        seed: |seed| seed.grkit_settings(Route::ShutdownScreen),
    },
    // S4 — Unlock GR IV Film Looks.
    DemoScreen {
        id: "S4",
        key_text: "Choose Firmware File",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.looks();
            app.open_add();
            app.push(Route::LookUnlock(crate::store::native_looks::Firmware::Gr4));
        },
    },
];
