//! Headless walks of demo mode: update and card-full error.
//! Demo mode is in debug builds and with the `demo` feature (`cfg(demo)`).
#![cfg(demo)]

use std::{
    env,
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use grkit_app::App;
use nsh_core::platform::Os;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;

const WAIT: Duration = Duration::from_secs(20);

/// Demo flags are process-wide; hold for the whole test so construction stays stable.
static DEMO_LOCK: Mutex<()> = Mutex::new(());

/// Connect on the camera sheet: the tab of a phone without a camera is
/// Connect too, and comes after the sheet.
const SHEET_CONNECT: &str = "Connect >> nth=0";

fn lock_demo() -> MutexGuard<'static, ()> {
    DEMO_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn start_phone() -> Headless {
    let mut config = EngineConfig::new(Root::of::<App>().translations(grkit_app::translations()));
    config.environment.os = Os::Web;
    Headless::start(config).unwrap()
}

fn set_demo(fail: Option<&str>) {
    unsafe {
        env::set_var("GRKIT_DEMO", "1");
        match fail {
            Some(fail) => env::set_var("GRKIT_DEMO_FAIL", fail),
            None => env::remove_var("GRKIT_DEMO_FAIL"),
        }
    }
}

#[test]
fn the_camera_card_opens_the_look_in_use_on_its_tab() {
    let _guard = lock_demo();
    set_demo(None);
    unsafe {
        env::set_var("GRKIT_DEMO_SCREEN", "A5");
    }
    let mut app = start_phone();
    let shown = app.wait_for_within("label:Current Film Look, Portra 400", WAIT);
    unsafe {
        env::remove_var("GRKIT_DEMO_SCREEN");
    }
    shown.unwrap();
    app.press("label:Current Film Look, Portra 400").unwrap();
    app.wait_for_within("Enable", WAIT).unwrap();
}

#[test]
fn a_pending_update_on_the_camera_tab_opens_software_update_under_its_steps() {
    let _guard = lock_demo();
    set_demo(None);
    unsafe {
        env::set_var("GRKIT_DEMO_SCREEN", "U14");
    }
    let mut app = start_phone();
    let shown = app.wait_for_within("GRKit 1.7.0 to Confirm", WAIT);
    unsafe {
        env::remove_var("GRKIT_DEMO_SCREEN");
    }
    shown.unwrap();
    app.press("GRKit 1.7.0 to Confirm").unwrap();
    app.wait_for_within("Confirm on the Camera", WAIT).unwrap();
    // Closed, the steps leave Software Update, not the camera tab.
    app.press("Close").unwrap();
    app.wait_for_gone("Confirm on the Camera", WAIT).unwrap();
    app.wait_for_within("Update History", WAIT).unwrap();
}

#[test]
fn demo_update_sheet_stays_open_and_closes_without_cancelling() {
    let _guard = lock_demo();
    set_demo(None);
    let mut app = start_phone();
    app.wait_for("Add Your Camera").unwrap();
    app.press("Add Camera").unwrap();
    app.wait_for("Connect").unwrap();
    app.press(SHEET_CONNECT).unwrap();

    app.wait_for_within("GRKit 1.7.0 Available", WAIT).unwrap();
    app.press("GRKit 1.7.0 Available").unwrap();
    app.press("Update").unwrap();

    // The sheet stays open through the steps on the camera and ends with
    // the installed version.
    app.wait_for_within("Updating RICOH GR III", WAIT).unwrap();
    app.wait_for_within("Confirm on the Camera", WAIT).unwrap();
    app.wait_for_within("Restart the Camera", WAIT).unwrap();
    // Closed, the update stays on the camera; its row opens the sheet again.
    app.press("Close").unwrap();
    app.wait_for_gone("Restart the Camera", WAIT).unwrap();
    app.press("GRKit 1.7.0 to Install").unwrap();
    app.wait_for_within("Restart the Camera", WAIT).unwrap();
    app.press("Restart Camera").unwrap();
    app.wait_for_within("GRKit 1.7.0 Is Installed", WAIT)
        .unwrap();
    // Done: the camera tab's row says what Software Update says.
    app.press("Done").unwrap();
    app.wait_for_within("GRKit is up to date", WAIT).unwrap();
}

#[test]
fn demo_card_full_error() {
    let _guard = lock_demo();
    set_demo(Some("card_full"));
    let mut app = start_phone();
    app.wait_for("Add Your Camera").unwrap();
    app.press("Add Camera").unwrap();
    app.press(SHEET_CONNECT).unwrap();
    app.wait_for_within("GRKit 1.7.0 Available", WAIT).unwrap();
    app.press("GRKit 1.7.0 Available").unwrap();
    app.press("Update").unwrap();
    app.wait_for_within("Card Is Full", WAIT).unwrap();
    app.wait_for("12 MB more is needed. Delete some photos on the camera, then try again.")
        .unwrap();
    app.press("Close >> nth=1").unwrap();
    app.wait_for_gone("Card Is Full", WAIT).unwrap();
}

#[test]
fn cancelling_connect_can_start_a_fresh_attempt() {
    let _guard = lock_demo();
    set_demo(None);
    unsafe {
        env::set_var("GRKIT_DEMO_SCREEN", "A4");
    }
    let mut app = start_phone();
    let shown = app.wait_for("Connecting to RICOH GR III");
    unsafe {
        env::remove_var("GRKIT_DEMO_SCREEN");
    }
    shown.unwrap();
    app.press("Cancel").unwrap();
    app.wait_for_gone("Connecting to RICOH GR III", WAIT)
        .unwrap();
    app.press("Add Camera").unwrap();
    app.press(SHEET_CONNECT).unwrap();
    app.wait_for_within("label:More", WAIT).unwrap();
}

#[test]
fn a_demo_connect_shows_the_look_in_use_and_the_modules() {
    let _guard = lock_demo();
    set_demo(None);
    let mut app = start_phone();
    app.wait_for_within("Add Camera", WAIT).unwrap();
    app.press("Add Camera").unwrap();
    app.wait_for_within("Connect", WAIT).unwrap();
    app.press(SHEET_CONNECT).unwrap();
    app.wait_for_within("label:Current Film Look, Portra 400", WAIT)
        .unwrap();
    app.wait_for_within("label:/^Modules, Framing/", WAIT)
        .unwrap();
}
