//! Each demo screen id starts on its design state with one key text visible.

use std::{
    env,
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use grkit_app::App;
use grkit_app::store::{DEMO_SCREEN_IDS, key_text, shows_alert};
use nsh_core::geometry::Size;
use nsh_core::platform::Os;
use nsh_core::prelude::Points;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::{Headless, LayoutNode};

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

/// The texts that a view around them cuts, or that leave the screen's width.
fn clipped_texts(app: &Headless, width: f32) -> Vec<String> {
    let nodes: Vec<LayoutNode> = app.screen().layout_nodes();
    let parent = |node: &LayoutNode| nodes.iter().find(|n| n.id == node.parent);
    let mut clipped = Vec::new();
    for node in nodes.iter().filter(|n| n.text.is_some() && !n.hidden) {
        let (left, top) = (node.bounds.origin.x.0, node.bounds.origin.y.0);
        let (right, bottom) = (
            left + node.bounds.size.width.0,
            top + node.bounds.size.height.0,
        );
        let mut cut = false;
        let mut sideways = false;
        let mut around = parent(node);
        while let Some(view) = around {
            if let Some(scroll) = &view.scroll {
                // A row that scrolls sideways may run past the screen.
                sideways = scroll.horizontal;
                break;
            }
            if view.clips {
                let (x, y) = (view.bounds.origin.x.0, view.bounds.origin.y.0);
                cut |= right > x + view.bounds.size.width.0 + 1.0
                    || bottom > y + view.bounds.size.height.0 + 1.0;
            }
            around = parent(view);
        }
        cut |= !sideways && (right > width + 1.0 || left < -1.0);
        if cut {
            clipped.push(format!("{:?}", node.text));
        }
    }
    clipped
}

#[test]
fn no_demo_screen_clips_text_at_a_large_text_size() {
    let _guard = lock_demo();
    let width = 390.0;
    let mut failures = Vec::new();
    for &(id, screen) in DEMO_SCREEN_IDS {
        if shows_alert(screen) {
            continue;
        }
        unsafe {
            env::set_var("GRKIT_DEMO", "1");
            env::set_var("GRKIT_DEMO_SCREEN", id);
            env::remove_var("GRKIT_DEMO_FAIL");
        }
        let mut config = EngineConfig::new(Root::of::<App>());
        config.environment.os = Os::Web;
        config.environment.font_scale = 2.0;
        config.viewport = Size {
            width: Points(width),
            height: Points(844.0),
        };
        let mut app = Headless::start(config).unwrap();
        app.wait_for_within(key_text(screen), WAIT)
            .unwrap_or_else(|e| panic!("screen {id}: {e}"));
        app.wait(Duration::from_millis(50)).unwrap();
        let clipped = clipped_texts(&app, width);
        if !clipped.is_empty() {
            failures.push(format!("{id}: {clipped:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
