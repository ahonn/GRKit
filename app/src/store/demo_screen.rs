//! Demo screen ids for screenshots: `GRKIT_DEMO_SCREEN` / `--demo-screen`.
//!
//! When demo mode is on and an id is set, [`apply`] jumps the FakeCamera and UI
//! to that design screen (sheet, push, or alert) and freezes the ticker so the
//! frame stays still for a screenshot. Unknown ids leave the normal demo.

use std::{
    env,
    time::{Duration, Instant},
};

use nsh_core::prelude::*;

use crate::domain::fake_looks::DEMO_CUSTOM_ID;
use crate::domain::release_notes::VersionNotes;
use crate::domain::{
    CameraLink, FakeCamera, FakeConfig, ReleaseProblem, UpdateError, UpdateKind, UpdatePhase,
    block_on_ready,
};
use crate::store::{AppState, Route, Sheet, Tab};

/// Design-file screen ids used for demo screenshots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemoScreen {
    /// A1 — first launch (empty Camera tab).
    A1,
    /// A2 — camera nearby sheet.
    A2,
    /// A3 — add a camera sheet.
    A3,
    /// A4 — connecting checklist.
    A4,
    /// A5 — connected, update available.
    A5,
    /// A6 — connected, up to date.
    A6,
    /// A7 — Settings tab.
    A7,
    /// B1 — Software Update (update available).
    B1,
    /// B2 — sending (~62% progress).
    B2,
    /// B3 — confirm on the camera (countdown).
    B3,
    /// B4 — restart the camera.
    B4,
    /// B5 — update installed.
    B5,
    /// C4 — card full error.
    C4,
    /// C5 — not confirmed error.
    C5,
    /// L1 — Looks on the camera, in menu order.
    L1,
    /// L2 — an official Look.
    L2,
    /// L3 — an imported Look.
    L3,
    /// L4 — a new Look converted, ready to send.
    L4,
    /// L5 — delete the Look in use (alert).
    L5,
    /// L6 — a Look that recipes use cannot be deleted (alert).
    L6,
    /// L7 — the Add page, one Look being added.
    L7,
    /// L8 — import a Look sheet.
    L8,
    /// L9 — a new Look: file, name, LUT input and cover.
    L9,
    /// L10 — Looks without a camera.
    L10,
    /// L11 — an update waits on the camera (read only).
    L11,
    /// L12 — no room on the camera (alert).
    L12,
    /// L13 — the cover editor of an official Look.
    L13,
    /// L14 — changes wait for a restart of the camera.
    L14,
    /// L15 — the menu of a held Look.
    L15,
    /// L16 — Cover Colors.
    L16,
    /// L17 — rename a Look.
    L17,
    /// L18 — the card holds as many Looks as it can (alert).
    L18,
    /// L19 — the connection was lost during a change.
    L19,
    /// A8 — saved cameras, none connected.
    A8,
    /// A9 — the connection was lost.
    A9,
    /// A10 — Add Camera found one camera.
    A10,
    /// A11 — a connect error, in the connect sheet.
    A11,
    /// A12 — the camera has no GRKit.
    A12,
    /// A13 — Settings › Saved Cameras.
    A13,
    /// U1 — Software Update, up to date.
    U1,
    /// U2 — Software Update, checking.
    U2,
    /// U3 — Software Update, the check failed.
    U3,
    /// U4 — Software Update, the camera firmware is too old.
    U4,
    /// U5 — Software Update, the firmware is newer than GRKit.
    U5,
    /// U6 — Software Update, the app is too old.
    U6,
    /// U7 — The update sheet, downloading.
    U7,
    /// U8 — The restart the phone runs.
    U8,
    /// U9 — The person restarts the camera.
    U9,
    /// U10 — Cancel the update (alert).
    U10,
    /// U11 — The connection was lost during the update.
    U11,
    /// U12 — The download failed.
    U12,
    /// U13 — The camera refused the package.
    U13,
    /// U14 — Camera tab, an update waits for its confirmation.
    U14,
    /// U15 — Camera tab, an update waits for a restart.
    U15,
    /// U16 — A confirmed update did not install.
    U16,
    /// U17 — GRKit was updated while the app was away (toast).
    U17,
    /// U18 — The camera's GRKit is newer than the app.
    U18,
    /// U19 — Update History.
    U19,
    /// R1 — Recipes list.
    R1,
    /// R2 — Recipe on a Film Look.
    R2,
    /// R3 — Recipe on Image Control.
    R3,
    /// R4 — Recipe menu.
    R4,
    /// R5 — Applied, with Undo.
    R5,
    /// R6 — Preset recipes.
    R6,
    /// R7 — Preset that adds its Film Look.
    R7,
    /// R8 — Rename a recipe.
    R8,
    /// R9 — Delete a recipe.
    R9,
    /// R10 — Camera menu full.
    R10,
    /// R11 — No recipes.
    R11,
    /// R12 — Recipes without a camera.
    R12,
}

/// Every id string the env / CLI accept, in design order.
pub const DEMO_SCREEN_IDS: &[(&str, DemoScreen)] = &[
    ("A1", DemoScreen::A1),
    ("A2", DemoScreen::A2),
    ("A3", DemoScreen::A3),
    ("A4", DemoScreen::A4),
    ("A5", DemoScreen::A5),
    ("A6", DemoScreen::A6),
    ("A7", DemoScreen::A7),
    ("B1", DemoScreen::B1),
    ("B2", DemoScreen::B2),
    ("B3", DemoScreen::B3),
    ("B4", DemoScreen::B4),
    ("B5", DemoScreen::B5),
    ("C4", DemoScreen::C4),
    ("C5", DemoScreen::C5),
    ("L1", DemoScreen::L1),
    ("L2", DemoScreen::L2),
    ("L3", DemoScreen::L3),
    ("L4", DemoScreen::L4),
    ("L5", DemoScreen::L5),
    ("L6", DemoScreen::L6),
    ("L7", DemoScreen::L7),
    ("L8", DemoScreen::L8),
    ("L9", DemoScreen::L9),
    ("L10", DemoScreen::L10),
    ("L11", DemoScreen::L11),
    ("L12", DemoScreen::L12),
    ("L13", DemoScreen::L13),
    ("L14", DemoScreen::L14),
    ("L15", DemoScreen::L15),
    ("L16", DemoScreen::L16),
    ("L17", DemoScreen::L17),
    ("L18", DemoScreen::L18),
    ("L19", DemoScreen::L19),
    ("A8", DemoScreen::A8),
    ("A9", DemoScreen::A9),
    ("A10", DemoScreen::A10),
    ("A11", DemoScreen::A11),
    ("A12", DemoScreen::A12),
    ("A13", DemoScreen::A13),
    ("U1", DemoScreen::U1),
    ("U2", DemoScreen::U2),
    ("U3", DemoScreen::U3),
    ("U4", DemoScreen::U4),
    ("U5", DemoScreen::U5),
    ("U6", DemoScreen::U6),
    ("U7", DemoScreen::U7),
    ("U8", DemoScreen::U8),
    ("U9", DemoScreen::U9),
    ("U10", DemoScreen::U10),
    ("U11", DemoScreen::U11),
    ("U12", DemoScreen::U12),
    ("U13", DemoScreen::U13),
    ("U14", DemoScreen::U14),
    ("U15", DemoScreen::U15),
    ("U16", DemoScreen::U16),
    ("U17", DemoScreen::U17),
    ("U18", DemoScreen::U18),
    ("U19", DemoScreen::U19),
    ("R1", DemoScreen::R1),
    ("R2", DemoScreen::R2),
    ("R3", DemoScreen::R3),
    ("R4", DemoScreen::R4),
    ("R5", DemoScreen::R5),
    ("R6", DemoScreen::R6),
    ("R7", DemoScreen::R7),
    ("R8", DemoScreen::R8),
    ("R9", DemoScreen::R9),
    ("R10", DemoScreen::R10),
    ("R11", DemoScreen::R11),
    ("R12", DemoScreen::R12),
];

/// Parse a design id (`A5`, `a5`, …). Unknown → `None` (normal demo).
pub fn parse_demo_screen(id: &str) -> Option<DemoScreen> {
    let id = id.trim();
    DEMO_SCREEN_IDS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(id))
        .map(|(_, screen)| *screen)
}

/// Screen from `GRKIT_DEMO_SCREEN` or `--demo-screen <id>`.
pub fn demo_screen_from_env() -> Option<DemoScreen> {
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

/// Release notes for the demo: the features of the version on offer, and
/// the real notes of three releases.
fn demo_notes() -> Vec<VersionNotes> {
    [
        (
            "1.7.0",
            &[
                "The GRKit app updates GRKit over the camera's Wi‑Fi: the camera checks the signature of the update, you confirm it in ADJ > MOD, and it installs at the next start.",
                "The GRKit app lists the film looks on the camera, turns them on or off, orders them, and changes their names and covers.",
                "The camera runs the GRKit Wi‑Fi service only while its Wi‑Fi is on.",
                "Without a connected phone, the Wi‑Fi service uses no processor time.",
            ][..],
        ),
        (
            "1.6.4",
            &[
                "In ADJ, OK on a GRKit value returns to the GRKit list instead of closing ADJ, and Left or Right keeps the highlighted value.",
                "Battery in Simple Display shows the battery icon in the simple display, and Display Settings in GRKit Settings now holds it and Distance Unit.",
                "With the fast-boot firmware, GRKit loads faster from exFAT cards that a Mac copied files to, and a camera whose fast start fails loads GRKit the normal way from the next power-on.",
                "Film looks can show the colors of their film box, for color looks only or for all, and each film look can have its own name and cover.",
                "GRKit uses less processor time while the camera is on and no setting changes, and Grain takes less time for each photo on GR III and GR IIIx.",
                "Photos record their GRKit settings in XMP, so exiftool and other XMP readers show them.",
                "RAW development keeps the Framing ratio of a photo that has a Border.",
                "The RAW development preview keeps the Border color with Shading or Clarity, and puts the Shading vignette in the middle of the picture.",
                "RAW development starts from the GRKit settings that the DNG records, and has a GRKit row that sets Grain, Glow, Border, and Date for that development only.",
                "GRKit Settings has Uncropped RAW, which keeps the whole sensor picture in each DNG, and Lightroom and Camera Raw open it with the crop of the shot, which they can undo.",
            ][..],
        ),
        (
            "1.6.3",
            &[
                "Color covers shows the packaging colors of each film look on the shelf and writes them onto the card.",
                "Import a Lightroom .xmp preset as a film look, and optionally add a recipe with its exposure, grain, and white balance.",
            ][..],
        ),
        (
            "1.6.2",
            &[
                "With a border, the review after a cropped shot shows the border at once, and the picture stays in place when playback opens.",
                "The crop badge in the live view looks the same for each focal length.",
                "Order in the MENU page of Film Look, Recipes, Framing, Guides, and Date Imprint sets the order of their choices in ADJ.",
            ][..],
        ),
    ]
    .into_iter()
    .map(|(version, items)| VersionNotes {
        version: version.into(),
        items: items.iter().map(|i| (*i).to_owned()).collect(),
    })
    .collect()
}

/// Jump `app` to `screen` and freeze timed advances for a stable screenshot.
pub fn apply(app: AppState, screen: DemoScreen) {
    app.set_frozen(true);
    app.release_notes.set(demo_notes());
    app.update_shown.set(true);
    match screen {
        DemoScreen::A1 => {
            // Empty first launch: nearby fake stays for later Add Camera.
            app.status.set(None);
            app.package.set(None);
            app.set_sheet(None);
            app.set_tab(Tab::Camera);
        }
        DemoScreen::A2 => {
            app.status.set(None);
            app.package.set(None);
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Nearby));
        }
        DemoScreen::A3 => {
            app.status.set(None);
            app.package.set(None);
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Scan));
            app.pin_scan_list();
        }
        DemoScreen::A8 => {
            seed_saved(app);
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
        }
        DemoScreen::A9 => {
            seed_saved(app);
            app.show_toast(crate::strings::tr("cam.lost_title", &[]));
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
        }
        DemoScreen::A10 => {
            app.status.set(None);
            app.package.set(None);
            app.set_tab(Tab::Camera);
            app.open_scan();
        }
        DemoScreen::A11 => {
            seed_saved(app);
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::ConnectError(
                crate::domain::LinkError::WifiJoinTimeout,
            )));
        }
        DemoScreen::A12 => {
            app.status.set(None);
            app.package.set(None);
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::ConnectError(
                crate::domain::LinkError::NeedsGrkit,
            )));
        }
        DemoScreen::A13 => {
            seed_saved(app);
            seed_connected_available(app);
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.push(Route::Settings);
            app.push(Route::SavedCameras);
        }
        DemoScreen::A4 => {
            app.status.set(None);
            app.package.set(None);
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Connecting { step: 1 }));
        }
        DemoScreen::A5 => {
            seed_saved(app);
            seed_connected_available(app);
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.refresh_looks();
            app.refresh_recipes();
        }
        DemoScreen::A6 => {
            seed_connected_up_to_date(app);
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
        }
        DemoScreen::A7 => {
            seed_saved(app);
            seed_connected_available(app);
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.push(Route::Settings);
        }
        DemoScreen::B1 => {
            seed_connected_available(app);
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.push(Route::SoftwareUpdate);
        }
        DemoScreen::B2 => {
            seed_connected_available(app);
            seed_sending(app, UpdateKind::Install, 0.62);
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Sending));
        }
        DemoScreen::B3 => {
            seed_connected_available(app);
            seed_waiting(app, UpdateKind::Install);
            app.set_tab(Tab::Camera);
            app.confirm_deadline
                .set(Some(Instant::now() + Duration::from_secs(4 * 60 + 32)));
            app.set_sheet(Some(Sheet::Confirm));
        }
        DemoScreen::B4 => {
            seed_connected_available(app);
            seed_restart(app, UpdateKind::Install);
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Restart));
        }
        DemoScreen::B5 => {
            seed_connected_up_to_date(app);
            app.baseline.set(Some("1.6.1".into()));
            app.phase.set(UpdatePhase::Installed {
                kind: UpdateKind::Install,
                version: "1.7.0".into(),
            });
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Installed {
                version: "1.7.0".into(),
                previous: "1.6.1".into(),
            }));
        }
        DemoScreen::C4 => {
            seed_connected_available(app);
            app.phase.set(UpdatePhase::Failed(UpdateError::CardFull {
                bytes_needed: 12 * 1024 * 1024,
            }));
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Error(UpdateError::CardFull {
                bytes_needed: 12 * 1024 * 1024,
            })));
        }
        DemoScreen::C5 => {
            seed_connected_available(app);
            app.phase
                .set(UpdatePhase::Failed(UpdateError::NotConfirmed));
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Error(UpdateError::NotConfirmed)));
        }
        DemoScreen::U1 => {
            // A version with notes, so the page shows them.
            seed_connected_up_to_date_at(app, "1.6.4");
            app.set_tab(Tab::Camera);
            app.push(Route::SoftwareUpdate);
        }
        DemoScreen::U2 => software_with(app, None, true),
        DemoScreen::U3 => software_with(app, Some(ReleaseProblem::Network), false),
        DemoScreen::U4 => software_with(
            app,
            Some(ReleaseProblem::FirmwareTooOld {
                required: "2.21".into(),
            }),
            false,
        ),
        DemoScreen::U5 => software_with(app, Some(ReleaseProblem::FirmwareTooNew), false),
        DemoScreen::U6 => software_with(app, Some(ReleaseProblem::AppTooOld), false),
        DemoScreen::U7 => {
            seed_connected_available(app);
            app.update_shown.set(true);
            app.phase.set(UpdatePhase::Downloading {
                progress: 0.35,
                kind: UpdateKind::Install,
            });
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Sending));
        }
        DemoScreen::U8 => {
            seed_connected_available(app);
            seed_restart(app, UpdateKind::Install);
            app.update_shown.set(true);
            app.phase.set(UpdatePhase::Restarting {
                kind: UpdateKind::Install,
                step: crate::domain::RestartStep::PoweringOn,
            });
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Restart));
        }
        DemoScreen::U9 => {
            seed_connected_available(app);
            seed_restart(app, UpdateKind::Install);
            app.update_shown.set(true);
            app.phase.set(UpdatePhase::ManualRestart {
                kind: UpdateKind::Install,
            });
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Restart));
        }
        DemoScreen::U10 => {
            seed_connected_available(app);
            seed_sending(app, UpdateKind::Install, 0.42);
            app.update_shown.set(true);
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::Sending));
            spawn(async move {
                next_frame().await;
                next_frame().await;
                app.ask_cancel_update();
            });
        }
        DemoScreen::U11 => update_error(app, UpdateError::ConnectionLost),
        DemoScreen::U12 => update_error(app, UpdateError::Download(String::new())),
        DemoScreen::U13 => update_error(app, UpdateError::Refused),
        DemoScreen::U14 => {
            seed_connected_available(app);
            app.journal.set(Some(crate::domain::JournalState::Waiting));
            app.confirm_deadline
                .set(Some(Instant::now() + Duration::from_secs(4 * 60 + 12)));
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.refresh_looks();
        }
        DemoScreen::U15 => {
            seed_connected_available(app);
            app.journal
                .set(Some(crate::domain::JournalState::Confirmed));
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.refresh_looks();
        }
        DemoScreen::U16 => {
            seed_saved(app);
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::InstallFailed));
        }
        DemoScreen::U17 => {
            seed_connected_up_to_date(app);
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.refresh_looks();
            app.show_toast(crate::strings::tr(
                "update.updated_toast",
                &[("version", "1.7.0")],
            ));
        }
        DemoScreen::U19 => {
            seed_connected_available(app);
            app.set_tab(Tab::Camera);
            app.set_sheet(None);
            app.push(Route::SoftwareUpdate);
            app.push(Route::ReleaseHistory);
        }
        DemoScreen::U18 => {
            seed_saved(app);
            app.set_tab(Tab::Camera);
            app.set_sheet(Some(Sheet::ConnectError(
                crate::domain::LinkError::AppTooOld,
            )));
        }
        DemoScreen::L1 => seed_looks(app),
        DemoScreen::L2 => {
            seed_looks(app);
            app.push(Route::LookDetail(PORTRA.into()));
        }
        DemoScreen::L3 => {
            seed_looks(app);
            app.push(Route::LookDetail(DEMO_CUSTOM_ID.into()));
        }
        DemoScreen::L4 => {
            seed_looks(app);
            app.seed_import_draft();
            app.push(Route::LookImport);
            app.finish_import();
        }
        DemoScreen::L5 => {
            app.with_fake_looks(|f| f.set_uses(PORTRA, Vec::new()));
            seed_looks(app);
            app.push(Route::LookDetail(PORTRA.into()));
            ask_delete(app);
        }
        DemoScreen::L6 => {
            seed_looks(app);
            app.push(Route::LookDetail(PORTRA.into()));
            ask_delete(app);
        }
        DemoScreen::L7 => {
            seed_looks(app);
            app.open_add();
            app.seed_adding("provia-100f-v1");
        }
        DemoScreen::L8 => {
            seed_looks(app);
            app.set_sheet(Some(Sheet::ImportLook));
        }
        DemoScreen::L9 => {
            seed_looks(app);
            app.seed_import_draft();
            app.push(Route::LookImport);
        }
        DemoScreen::L10 => {
            app.status.set(None);
            app.package.set(None);
            app.set_sheet(None);
            app.set_tab(Tab::Looks);
        }
        DemoScreen::L11 => {
            app.with_fake_looks(|f| f.update_pending = true);
            seed_looks(app);
        }
        DemoScreen::L12 => {
            seed_looks(app);
            app.open_add();
            spawn(async move {
                next_frame().await;
                next_frame().await;
                app.alert_look_error(crate::domain::LookError::NoRoom);
            });
        }
        DemoScreen::L13 => {
            seed_looks(app);
            app.push(Route::LookDetail(PORTRA.into()));
            app.push(Route::LookCover(PORTRA.into()));
        }
        DemoScreen::L14 => {
            app.with_fake_looks(|f| f.set_pending(DEMO_CUSTOM_ID));
            seed_looks(app);
        }
        DemoScreen::L15 => {
            seed_looks(app);
            spawn(async move {
                next_frame().await;
                next_frame().await;
                if let Some(look) = app.look(PORTRA) {
                    crate::pages::look_menu(app, look);
                }
            });
        }
        DemoScreen::L16 => {
            seed_looks(app);
            app.push(Route::LookScheme);
        }
        DemoScreen::L17 => {
            seed_looks(app);
            app.push(Route::LookDetail(PORTRA.into()));
            spawn(async move {
                timer(std::time::Duration::from_millis(600)).await;
                app.ask_rename_look(PORTRA.into());
            });
        }
        DemoScreen::L18 => {
            app.with_fake_looks(|f| f.max_listed = f.snapshot().looks.len());
            seed_looks(app);
            app.open_add();
            spawn(async move {
                next_frame().await;
                next_frame().await;
                app.alert_look_error(crate::domain::LookError::CardFull);
            });
        }
        DemoScreen::L19 => {
            seed_looks(app);
            app.status.set(None);
            app.alert_look_error(crate::domain::LookError::ConnectionLost);
        }
        DemoScreen::R1 => seed_recipes(app),
        DemoScreen::R2 => {
            seed_recipes(app);
            app.push(Route::RecipeDetail(1));
        }
        DemoScreen::R3 => {
            seed_recipes(app);
            app.push(Route::RecipeDetail(3));
        }
        DemoScreen::R4 => {
            seed_recipes(app);
            spawn(async move {
                next_frame().await;
                next_frame().await;
                if let Some(item) = app.recipe(1) {
                    crate::pages::recipe_menu(app, item, Box::new(|| {}));
                }
            });
        }
        DemoScreen::R5 => {
            seed_recipes(app);
            app.apply_recipe(2);
        }
        DemoScreen::R6 => {
            seed_recipes(app);
            app.open_recipe_presets();
        }
        DemoScreen::R7 => {
            seed_recipes(app);
            app.load_catalog();
            app.open_recipe_presets();
            app.push(Route::RecipePreset("cinestill-800t".into()));
        }
        DemoScreen::R8 => {
            seed_recipes(app);
            app.push(Route::RecipeDetail(1));
            spawn(async move {
                timer(std::time::Duration::from_millis(600)).await;
                app.ask_rename_recipe(1);
            });
        }
        DemoScreen::R9 => {
            seed_recipes(app);
            app.push(Route::RecipeDetail(1));
            spawn(async move {
                next_frame().await;
                next_frame().await;
                crate::pages::confirm_delete_recipe(app, 1, "Odd Film".into());
            });
        }
        DemoScreen::R10 => {
            app.with_fake_recipes(|f| f.max_on = f.on_count());
            seed_recipes(app);
            app.set_recipe_enabled(6, true);
        }
        DemoScreen::R11 => {
            app.with_fake_recipes(|f| f.recipes.clear());
            seed_recipes(app);
        }
        DemoScreen::R12 => {
            app.status.set(None);
            app.package.set(None);
            app.set_sheet(None);
            app.set_tab(Tab::Recipes);
        }
    }
}

const PORTRA: &str = "portra-400-endura-v1";

/// Connected, on the Looks tab, with the demo camera's Looks read.
fn seed_looks(app: AppState) {
    seed_connected_up_to_date(app);
    app.set_sheet(None);
    app.set_tab(Tab::Looks);
    app.refresh_looks();
}

/// Connected, on the Recipes tab, with the demo camera's recipes and
/// Looks read.
fn seed_recipes(app: AppState) {
    seed_connected_up_to_date(app);
    app.set_sheet(None);
    app.refresh_looks();
    app.set_tab(Tab::Recipes);
}

/// Ask to delete Portra 400 after the page shows.
fn ask_delete(app: AppState) {
    spawn(async move {
        next_frame().await;
        next_frame().await;
        if let Some(look) = app.look(PORTRA) {
            crate::pages::confirm_delete(app, look);
        }
    });
}

/// Screens whose key text is an alert title.
pub fn shows_alert(screen: DemoScreen) -> bool {
    matches!(
        screen,
        DemoScreen::L5
            | DemoScreen::L6
            | DemoScreen::L12
            | DemoScreen::L15
            | DemoScreen::L18
            | DemoScreen::U10
            | DemoScreen::R4
            | DemoScreen::R9
            | DemoScreen::R10
    )
}

/// Two saved cameras, the GR III the most recent.
fn seed_saved(app: AppState) {
    use crate::domain::{CameraModel, RememberedCamera};
    app.remember_camera(RememberedCamera::new(
        "DEMO-GR4",
        CameraModel::Gr4,
        "GR_7F2E10",
    ));
    app.remember_camera(RememberedCamera::new(
        "DEMO-GR3",
        CameraModel::Gr3,
        "GR_A1B2C3",
    ));
}

fn seed_connected_available(app: AppState) {
    app.camera.with(|slot| {
        let mut slot = slot.borrow_mut();
        let cam = slot.get_or_insert_with(|| FakeCamera::new(FakeConfig::default()));
        let _ = block_on_ready(cam.connect());
        if let Ok(status) = block_on_ready(cam.read_status()) {
            app.status.set(Some(status));
        }
        if let Ok(package) = block_on_ready(cam.read_package()) {
            app.package.set(Some(package));
        }
    });
}

fn seed_connected_up_to_date(app: AppState) {
    seed_connected_up_to_date_at(app, "1.7.0");
}

fn seed_connected_up_to_date_at(app: AppState, version: &str) {
    app.camera.with(|slot| {
        let mut slot = slot.borrow_mut();
        let cfg = FakeConfig {
            installed: version.into(),
            available: version.into(),
            ..FakeConfig::default()
        };
        let mut cam = FakeCamera::new(cfg);
        let _ = block_on_ready(cam.connect());
        cam.set_up_to_date(version);
        if let Ok(status) = block_on_ready(cam.read_status()) {
            app.status.set(Some(status));
        }
        if let Ok(package) = block_on_ready(cam.read_package()) {
            app.package.set(Some(package));
        }
        *slot = Some(cam);
    });
}

/// Software Update with no release: under way, or with `problem`.
fn software_with(app: AppState, problem: Option<ReleaseProblem>, checking: bool) {
    seed_connected_available(app);
    app.package.update(|p| {
        if let Some(p) = p {
            p.available = None;
        }
    });
    app.release_busy.set(checking);
    app.release_problem.set(problem);
    app.set_tab(Tab::Camera);
    app.push(Route::SoftwareUpdate);
}

/// The update sheet stopped by `error`.
fn update_error(app: AppState, error: UpdateError) {
    seed_connected_available(app);
    app.update_shown.set(true);
    app.phase.set(UpdatePhase::Failed(error.clone()));
    app.set_tab(Tab::Camera);
    app.set_sheet(Some(Sheet::Error(error)));
}

fn seed_sending(app: AppState, kind: UpdateKind, progress: f32) {
    app.camera.with(|slot| {
        if let Some(cam) = slot.borrow_mut().as_mut() {
            cam.seed_sending(kind, progress);
            app.phase.set(cam.flow().phase().clone());
            app.baseline
                .set(cam.flow().baseline_version().map(str::to_owned));
        }
    });
}

fn seed_waiting(app: AppState, kind: UpdateKind) {
    app.camera.with(|slot| {
        if let Some(cam) = slot.borrow_mut().as_mut() {
            cam.seed_waiting(kind);
            app.phase.set(cam.flow().phase().clone());
            app.baseline
                .set(cam.flow().baseline_version().map(str::to_owned));
        }
    });
}

fn seed_restart(app: AppState, kind: UpdateKind) {
    app.camera.with(|slot| {
        if let Some(cam) = slot.borrow_mut().as_mut() {
            cam.seed_restart(kind);
            app.phase.set(cam.flow().phase().clone());
            app.baseline
                .set(cam.flow().baseline_version().map(str::to_owned));
        }
    });
}

/// Key English text a headless test looks for on each screen.
pub fn key_text(screen: DemoScreen) -> &'static str {
    match screen {
        DemoScreen::A1 => "Add Your Camera",
        DemoScreen::A2 => "Connect",
        DemoScreen::A3 => "Add a Camera",
        DemoScreen::A4 => "Connecting to RICOH GR III",
        DemoScreen::A5 => "GRKit 1.7.0 Available",
        DemoScreen::A6 => "1.7.0",
        DemoScreen::A7 => "More from the Maker",
        DemoScreen::B1 => "What's New",
        DemoScreen::B2 => "62%",
        DemoScreen::B3 => "Confirm on the Camera",
        DemoScreen::B4 => "Restart the Camera",
        DemoScreen::B5 => "GRKit 1.7.0 Is Installed",
        DemoScreen::C4 => "Card Is Full",
        DemoScreen::C5 => "Not Confirmed on the Camera",
        DemoScreen::L1 => "In Camera Menu",
        DemoScreen::L2 => "Enabled",
        DemoScreen::L3 => "My LUT",
        DemoScreen::L4 => "Put on Camera",
        DemoScreen::L5 => "Delete Portra 400?",
        DemoScreen::L6 => "Can’t Delete Portra 400",
        DemoScreen::L7 => "Import a File",
        DemoScreen::L8 => "Lightroom Preset",
        DemoScreen::L9 => "LUT Input",
        DemoScreen::L10 => "No Camera Connected",
        DemoScreen::L11 => "An update is waiting on the camera",
        DemoScreen::L12 => "Camera Menu Is Full",
        DemoScreen::L13 => "Film name",
        DemoScreen::L14 => "1 change applies after a restart",
        DemoScreen::L15 => "Portra 400",
        DemoScreen::L16 => "Every built-in film look has a gray cover",
        DemoScreen::L17 => "Name",
        DemoScreen::L18 => "No Room on the Card",
        DemoScreen::L19 => "Disconnected. This change wasn't saved.",
        DemoScreen::A8 => "Other Cameras",
        DemoScreen::A9 => "Lost Connection",
        DemoScreen::A10 => "First time with this camera? On the camera, choose",
        DemoScreen::A11 => "Camera Wi‑Fi Unreachable",
        DemoScreen::A12 => "No GRKit on the Camera",
        DemoScreen::A13 => "Only one camera can connect at a time.",
        DemoScreen::U1 => "What's New in This Version",
        DemoScreen::U2 => "Checking for updates…",
        DemoScreen::U3 => "Couldn't check for updates",
        DemoScreen::U4 => "Camera firmware needs an update",
        DemoScreen::U5 => "This firmware isn't supported yet",
        DemoScreen::U6 => "GRKit app needs an update",
        DemoScreen::U7 => "35%",
        DemoScreen::U8 => "Restarting the Camera",
        DemoScreen::U9 => "Turn the camera off and on, then tap Reconnect.",
        DemoScreen::U10 => "Cancel the Update?",
        DemoScreen::U11 => "Connection Lost",
        DemoScreen::U12 => "Couldn't Download the Update",
        DemoScreen::U13 => "The Camera Refused the Update",
        DemoScreen::U14 => "GRKit 1.7.0 to Confirm",
        DemoScreen::U15 => "GRKit 1.7.0 to Install",
        DemoScreen::U16 => "Update Didn't Finish",
        DemoScreen::U17 => "GRKit updated to 1.7.0",
        DemoScreen::U18 => "GRKit App Needs an Update",
        DemoScreen::U19 => "View on GitHub",
        DemoScreen::R1 => "In Camera Menu",
        DemoScreen::R2 => "Apply to Camera",
        DemoScreen::R3 => "Image Control",
        DemoScreen::R4 => "Odd Film",
        DemoScreen::R5 => "Applied Street",
        DemoScreen::R6 => "Presets for GR III.",
        DemoScreen::R7 => "Add to Camera",
        DemoScreen::R8 => "Rename",
        DemoScreen::R9 => "Delete Odd Film?",
        DemoScreen::R10 => "Camera Menu Full",
        DemoScreen::R11 => "No Recipes Yet",
        DemoScreen::R12 => "No Camera Connected",
    }
}
