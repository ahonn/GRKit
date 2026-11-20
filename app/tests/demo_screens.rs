//! Each demo screen id starts on its design state with one key text visible.

use std::{
    env,
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use grkit_app::App;
use grkit_app::store::{DEMO_SCREEN_IDS, key_text, shows_alert};
use nsh_core::platform::Os;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;

const WAIT: Duration = Duration::from_secs(10);

static DEMO_LOCK: Mutex<()> = Mutex::new(());

fn lock_demo() -> MutexGuard<'static, ()> {
    DEMO_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn start_screen(id: &str) -> Headless {
    unsafe {
        env::set_var("GRKIT_DEMO", "1");
        env::set_var("GRKIT_DEMO_SCREEN", id);
        env::remove_var("GRKIT_DEMO_FAIL");
    }
    let mut config = EngineConfig::new(Root::of::<App>());
    config.environment.os = Os::Web;
    Headless::start(config).unwrap()
}

#[test]
fn each_demo_screen_shows_its_key_text() {
    let _guard = lock_demo();
    for &(id, screen) in DEMO_SCREEN_IDS {
        let key = key_text(screen);
        let mut app = start_screen(id);
        if shows_alert(screen) {
            // Alert is presented asynchronously; poll dialogs until the title appears.
            let mut found = false;
            for _ in 0..40 {
                app.wait(Duration::from_millis(50)).unwrap();
                let dialogs = app.screen().dialogs();
                found = dialogs
                    .iter()
                    .any(|d| d.title.as_deref().is_some_and(|t| t == key));
                if found {
                    break;
                }
            }
            assert!(
                found,
                "screen {id} missing alert title `{key}`; dialogs={:?}",
                app.screen().dialogs()
            );
            continue;
        }
        app.wait_for_within(key, WAIT)
            .unwrap_or_else(|e| panic!("screen {id} missing `{key}`: {e}"));
    }
}
