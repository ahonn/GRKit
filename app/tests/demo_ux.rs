//! Headless walks of demo mode: update and card-full error.

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

fn lock_demo() -> MutexGuard<'static, ()> {
    DEMO_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn start_phone() -> Headless {
    let mut config = EngineConfig::new(Root::of::<App>());
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
    let shown = app.wait_for_within("On the Camera", WAIT);
    unsafe {
        env::remove_var("GRKIT_DEMO_SCREEN");
    }
    shown.unwrap();
    app.wait_for_within("Film Looks, Portra 400", WAIT).unwrap();
    app.press("Film Looks, Portra 400").unwrap();
    app.wait_for_within("Enabled", WAIT).unwrap();
}

#[test]
fn demo_add_camera_update_confirm_restart_installed() {
    let _guard = lock_demo();
    set_demo(None);
    let mut app = start_phone();
    app.wait_for("Add Your Camera").unwrap();
    app.press("Add Camera").unwrap();
    app.wait_for("Connect").unwrap();
    app.press("Connect").unwrap();

    app.wait_for_within("GRKit 1.7.0 Available", WAIT).unwrap();
    app.press("GRKit 1.7.0 Available").unwrap();
    app.press("Update").unwrap();

    app.wait_for_within("Updating RICOH GR III", WAIT).unwrap();
    app.wait_for_within("Confirm on the Camera", WAIT).unwrap();
    app.wait_for_within("Restart the Camera", WAIT).unwrap();
    app.press("Restart Camera").unwrap();
    app.wait_for_within("GRKit 1.7.0 Is Installed", WAIT)
        .unwrap();
    app.press("Done").unwrap();
    app.wait_for_gone("GRKit 1.7.0 Available", WAIT).unwrap();
}

#[test]
fn demo_card_full_error() {
    let _guard = lock_demo();
    set_demo(Some("card_full"));
    let mut app = start_phone();
    app.wait_for("Add Your Camera").unwrap();
    app.press("Add Camera").unwrap();
    app.press("Connect").unwrap();
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
    app.press("Connect").unwrap();
    app.wait_for_within("Connected", WAIT).unwrap();
}
