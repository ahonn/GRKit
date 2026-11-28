//! Software updates: the release check, the update sheet, and its errors.

use nsh_core::prelude::*;

use super::DemoScreen;
use crate::dev::fake::FakeConfig;
use crate::domain::{
    JournalState, LinkError, ReleaseProblem, RestartStep, UpdateError, UpdateKind, UpdatePhase,
};
use crate::store::{Route, Sheet, Tab};

pub(super) const SCREENS: &[DemoScreen] = &[
    // B1 — Software Update (update available).
    DemoScreen {
        id: "B1",
        key_text: "What's New",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_available();
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.push(Route::SoftwareUpdate);
        },
    },
    // B2 — sending (~62% progress).
    DemoScreen {
        id: "B2",
        key_text: "62%",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_available();
            seed.sending(0.62);
            app.set_tab(Tab::Camera);
            // The update sheet opens from Software Update.
            app.push(Route::SoftwareUpdate);
            app.set_sheet(Some(Sheet::Sending));
        },
    },
    // B3 — confirm on the camera (countdown).
    DemoScreen {
        id: "B3",
        key_text: "Confirm on the Camera",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_available();
            seed.waiting();
            app.set_tab(Tab::Camera);
            // The update sheet opens from Software Update.
            app.push(Route::SoftwareUpdate);
            app.set_sheet(Some(Sheet::Confirm));
        },
    },
    // B4 — restart the camera.
    DemoScreen {
        id: "B4",
        key_text: "Restart the Camera",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_available();
            seed.restart();
            app.set_tab(Tab::Camera);
            // The update sheet opens from Software Update.
            app.push(Route::SoftwareUpdate);
            app.set_sheet(Some(Sheet::Restart));
        },
    },
    // B5 — update installed.
    DemoScreen {
        id: "B5",
        key_text: "GRKit 1.7.0 Is Installed",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_up_to_date();
            app.seed_update(
                UpdatePhase::Installed {
                    kind: UpdateKind::Install,
                    version: "1.7.0".into(),
                },
                Some("1.6.1"),
            );
            app.set_tab(Tab::Camera);
            // The update sheet opens from Software Update.
            app.push(Route::SoftwareUpdate);
            app.set_sheet(Some(Sheet::Installed {
                version: "1.7.0".into(),
                previous: "1.6.1".into(),
            }));
        },
    },
    // C4 — card full error.
    DemoScreen {
        id: "C4",
        key_text: "Card Is Full",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_available();
            let error = UpdateError::CardFull {
                bytes_needed: 12 * 1024 * 1024,
            };
            app.seed_update(UpdatePhase::Failed(error.clone()), None);
            app.set_tab(Tab::Camera);
            // The update sheet opens from Software Update.
            app.push(Route::SoftwareUpdate);
            app.set_sheet(Some(Sheet::Error(error)));
        },
    },
    // U1 — Software Update, up to date.
    DemoScreen {
        id: "U1",
        key_text: "What's New in This Version",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            // A version with notes, so the page shows them.
            seed.connected_up_to_date_at("1.6.4");
            app.set_tab(Tab::Camera);
            app.push(Route::SoftwareUpdate);
        },
    },
    // U2 — Software Update, checking.
    DemoScreen {
        id: "U2",
        key_text: "RICOH GR III",
        alert: false,
        seed: |seed| seed.software_with(None, true),
    },
    // U3 — Software Update, the check failed.
    DemoScreen {
        id: "U3",
        key_text: "Couldn't check for updates",
        alert: false,
        seed: |seed| seed.software_with(Some(ReleaseProblem::Network), false),
    },
    // U4 — Software Update, the camera firmware is too old.
    DemoScreen {
        id: "U4",
        key_text: "Camera firmware needs an update",
        alert: false,
        seed: |seed| {
            let demo = seed.demo;
            // A GR III on 2.20, the release needs 2.21.
            demo.configure(FakeConfig {
                firmware: "2.20".into(),
                ..FakeConfig::default()
            });
            seed.software_with(
                Some(ReleaseProblem::FirmwareTooOld {
                    required: "2.21".into(),
                }),
                false,
            )
        },
    },
    // U5 — Software Update, the firmware is newer than GRKit.
    DemoScreen {
        id: "U5",
        key_text: "This firmware isn't supported yet",
        alert: false,
        seed: |seed| seed.software_with(Some(ReleaseProblem::FirmwareTooNew), false),
    },
    // U6 — Software Update, the app is too old.
    DemoScreen {
        id: "U6",
        key_text: "GRKit app needs an update",
        alert: false,
        seed: |seed| seed.software_with(Some(ReleaseProblem::AppTooOld), false),
    },
    // U7 — The update sheet, downloading.
    DemoScreen {
        id: "U7",
        key_text: "35%",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_available();
            app.seed_update(
                UpdatePhase::Downloading {
                    progress: 0.35,
                    kind: UpdateKind::Install,
                },
                None,
            );
            app.set_tab(Tab::Camera);
            // The update sheet opens from Software Update.
            app.push(Route::SoftwareUpdate);
            app.set_sheet(Some(Sheet::Sending));
        },
    },
    // U8 — The restart the phone runs.
    DemoScreen {
        id: "U8",
        key_text: "Restarting the Camera",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_available();
            seed.restart();
            app.seed_update(
                UpdatePhase::Restarting {
                    kind: UpdateKind::Install,
                    step: RestartStep::PoweringOn,
                },
                None,
            );
            app.set_tab(Tab::Camera);
            // The update sheet opens from Software Update.
            app.push(Route::SoftwareUpdate);
            app.set_sheet(Some(Sheet::Restart));
        },
    },
    // U9 — The person restarts the camera.
    DemoScreen {
        id: "U9",
        key_text: "Turn the camera off and on, then tap Reconnect.",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_available();
            seed.restart();
            app.seed_update(
                UpdatePhase::ManualRestart {
                    kind: UpdateKind::Install,
                },
                None,
            );
            app.set_tab(Tab::Camera);
            // The update sheet opens from Software Update.
            app.push(Route::SoftwareUpdate);
            app.set_sheet(Some(Sheet::Restart));
        },
    },
    // U10 — Cancel the update (alert).
    DemoScreen {
        id: "U10",
        key_text: "Cancel the Update?",
        alert: true,
        seed: |seed| {
            let app = seed.app;
            seed.connected_available();
            seed.sending(0.42);
            app.set_tab(Tab::Camera);
            // The update sheet opens from Software Update.
            app.push(Route::SoftwareUpdate);
            app.set_sheet(Some(Sheet::Sending));
            spawn(async move {
                next_frame().await;
                next_frame().await;
                crate::pages::ask_cancel_update(app);
            });
        },
    },
    // U11 — The connection was lost during the update.
    DemoScreen {
        id: "U11",
        key_text: "Connection Lost",
        alert: false,
        seed: |seed| seed.update_error(UpdateError::ConnectionLost),
    },
    // U12 — The download failed.
    DemoScreen {
        id: "U12",
        key_text: "Couldn't Download the Update",
        alert: false,
        seed: |seed| seed.update_error(UpdateError::Download(String::new())),
    },
    // U13 — The camera refused the package.
    DemoScreen {
        id: "U13",
        key_text: "The Camera Refused the Update",
        alert: false,
        seed: |seed| seed.update_error(UpdateError::Refused),
    },
    // U14 — Camera tab, an update waits for its confirmation.
    DemoScreen {
        id: "U14",
        key_text: "GRKit 1.7.0 to Confirm",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_available();
            seed.camera_journal(JournalState::Waiting);
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.refresh_looks();
        },
    },
    // U15 — Camera tab, an update waits for a restart.
    DemoScreen {
        id: "U15",
        key_text: "GRKit 1.7.0 to Install",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_available();
            seed.camera_journal(JournalState::Confirmed);
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.refresh_looks();
        },
    },
    // U16 — A confirmed update did not install.
    DemoScreen {
        id: "U16",
        key_text: "Update Didn't Finish",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.saved();
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::InstallFailed));
        },
    },
    // U17 — GRKit was updated while the app was away (toast).
    DemoScreen {
        id: "U17",
        key_text: "GRKit updated to 1.7.0",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_up_to_date();
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.refresh_looks();
            app.show_toast(crate::strings::tr(
                "update.updated_toast",
                &[("version", "1.7.0")],
            ));
        },
    },
    // U18 — The camera's GRKit is newer than the app.
    DemoScreen {
        id: "U18",
        key_text: "GRKit App Needs an Update",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.saved();
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::ConnectError(LinkError::AppTooOld)));
        },
    },
    // U19 — Update History.
    DemoScreen {
        id: "U19",
        key_text: "View on GitHub",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_available();
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.push(Route::SoftwareUpdate);
            app.push(Route::ReleaseHistory);
        },
    },
];
