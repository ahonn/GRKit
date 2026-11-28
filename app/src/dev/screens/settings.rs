//! GRKit settings on the camera.

use super::DemoScreen;
use crate::domain::images::jpeg_data_uri;
use crate::domain::shutdown_picture::Crop;
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
    // S8 — Shutdown Screen after a picture went to the camera.
    DemoScreen {
        id: "S8",
        key_text: "On the camera. Shows at the next power-off.",
        alert: false,
        seed: |seed| {
            seed.shutdown_photo(|seed, photo, _| {
                let crop = Crop::middle(photo.width, photo.height);
                let Ok(jpeg) = photo.picture(crop) else {
                    return;
                };
                if let Some(uri) = jpeg_data_uri(&jpeg) {
                    seed.demo.set_shutdown_picture(jpeg);
                    seed.app.seed_shutdown_sent(uri);
                }
            });
        },
    },
    // S9 — The crop page of a picked photo.
    DemoScreen {
        id: "S9",
        key_text: "Drag and pinch to frame the picture.",
        alert: false,
        seed: |seed| {
            seed.shutdown_photo(|seed, photo, uri| {
                seed.app.seed_shutdown_draft(photo, uri);
                seed.app.push(Route::ShutdownCrop);
            });
        },
    },
];
