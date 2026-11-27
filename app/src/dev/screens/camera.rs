//! The Camera tab: adding and connecting a camera, and the home page.

use super::DemoScreen;
use crate::domain::{LinkError, ReleaseProblem};
use crate::store::{Route, Sheet, Tab};

pub(super) const SCREENS: &[DemoScreen] = &[
    // A1 — first launch (empty Camera tab).
    DemoScreen {
        id: "A1",
        key_text: "Add Your Camera",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            // Empty first launch: the demo camera stays nearby for Add Camera.
            seed.disconnected();
            app.set_sheet(None);
            app.set_tab(Tab::Camera);
        },
    },
    // A2 — camera nearby sheet.
    DemoScreen {
        id: "A2",
        key_text: "Connect",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.disconnected();
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Nearby));
        },
    },
    // A3 — add a camera sheet.
    DemoScreen {
        id: "A3",
        key_text: "Add a Camera",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.disconnected();
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Scan));
            app.pin_scan_list();
        },
    },
    // A4 — connecting checklist.
    DemoScreen {
        id: "A4",
        key_text: "Connecting to RICOH GR III",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.disconnected();
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Connecting { step: 1 }));
        },
    },
    // A5 — connected, update available.
    DemoScreen {
        id: "A5",
        key_text: "GRKit 1.7.0 Available",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.saved();
            seed.connected_available();
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.refresh_looks();
            app.refresh_recipes();
            app.refresh_modules();
        },
    },
    // A6 — connected, up to date.
    DemoScreen {
        id: "A6",
        key_text: "GRKit is up to date",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_up_to_date();
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.refresh_modules();
        },
    },
    // A7 — Settings tab.
    DemoScreen {
        id: "A7",
        key_text: "More from the Maker",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.saved();
            seed.connected_available();
            app.set_tab(Tab::Settings);
            app.set_sheet(None);
            app.refresh_modules();
        },
    },
    // A8 — saved cameras, none connected.
    DemoScreen {
        id: "A8",
        key_text: "Not Connected",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.saved();
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
        },
    },
    // A9 — the connection was lost.
    DemoScreen {
        id: "A9",
        key_text: "Lost Connection",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.saved();
            app.show_toast(crate::strings::tr("cam.lost_title", &[]));
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
        },
    },
    // A10 — Add Camera found one camera.
    DemoScreen {
        id: "A10",
        key_text: "First time with this camera? On the camera, choose",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.disconnected();
            app.set_tab(Tab::Camera);
            app.open_scan();
        },
    },
    // A11 — a connect error, in the connect sheet.
    DemoScreen {
        id: "A11",
        key_text: "Camera Wi‑Fi Unreachable",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.saved();
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::ConnectError(LinkError::WifiJoinTimeout)));
        },
    },
    // A12 — the camera has no GRKit.
    DemoScreen {
        id: "A12",
        key_text: "No GRKit on the Camera",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.disconnected();
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::ConnectError(LinkError::NeedsGrkit)));
        },
    },
    // A13 — Settings › Saved Cameras.
    DemoScreen {
        id: "A13",
        key_text: "Only one camera can connect at a time.",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.saved();
            seed.connected_available();
            app.set_tab(Tab::Settings);
            app.set_sheet(None);
            app.push(Route::SavedCameras);
        },
    },
    // H1 — Camera tab, connected, nothing read yet.
    DemoScreen {
        id: "H1",
        key_text: "label:Software Update, Reading…",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_status();
            app.seed_modules_loading();
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
        },
    },
    // H2 — Camera tab, the Looks and modules read, the rest not yet.
    DemoScreen {
        id: "H2",
        key_text: "Film Looks",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_status();
            // The release check has not answered yet.
            seed.package_without_release();
            app.seed_release_check(true, None);
            app.refresh_modules();
            app.refresh_looks();
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
        },
    },
    // H3 — Camera tab, every read failed.
    DemoScreen {
        id: "H3",
        key_text: "Couldn't check for updates",
        alert: false,
        seed: |seed| {
            let app = seed.app;
            seed.connected_status();
            seed.package_without_release();
            app.seed_release_check(false, Some(ReleaseProblem::Network));
            app.seed_modules_failed();
            app.seed_looks_failed();
            app.seed_recipes_failed();
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
        },
    },
];
