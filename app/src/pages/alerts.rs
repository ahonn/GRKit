//! The alerts of the app: the ones the store asks for (`store::Alert`), and
//! the question before an update stops.

use nsh_base::alert_dialog;
use nsh_core::prelude::*;

use crate::domain::{LookError, recipes::RecipeError};
use crate::store::{Alert, AlertRequest, AppState};
use crate::strings::tr;

/// Show each alert that the store asks for, for as long as the app runs.
/// Alerts that come while one shows wait for none: each has its dialog.
pub fn present_alerts(app: AppState) {
    spawn(async move {
        while let Some(request) = app.next_alert().await {
            spawn(present(app, request));
        }
    });
}

async fn present(app: AppState, request: AlertRequest) {
    let AlertRequest { alert, closed } = request;
    let (title, body, ok) = words(app, &alert);
    let mut dialog = alert_dialog().title(title).description(body);
    dialog = if alert == Alert::ManualRestart {
        dialog.action("ok", ok)
    } else {
        dialog.cancel("ok", ok)
    };
    let _ = dialog.show().await;
    if let Some(closed) = closed {
        let _ = closed.send(());
    }
}

/// The title, the body and the button of an alert.
fn words(app: AppState, alert: &Alert) -> (String, String, String) {
    match alert {
        Alert::Look(error) => {
            let (title, body) = error.keys();
            let limits = app.looks_snapshot();
            let max = match error {
                LookError::NoRoom => limits.map(|s| s.max_loaded),
                LookError::CardFull => limits.map(|s| s.max_listed),
                _ => None,
            }
            .unwrap_or_default()
            .to_string();
            let body = match error {
                LookError::Service(detail) => format!("{} {detail}", tr(body, &[])),
                _ => tr(body, &[("max", &max)]),
            };
            (tr(title, &[]), body, tr("looks.ok", &[]))
        }
        Alert::Module(error) => {
            let (title, body) = error.keys();
            let body = match error {
                LookError::Service(detail) => format!("{} {detail}", tr(body, &[])),
                _ => tr(body, &[]),
            };
            (tr(title, &[]), body, tr("looks.ok", &[]))
        }
        Alert::Recipe(error) => {
            let (title, body) = error.keys();
            let limits = app.recipes_snapshot();
            let max = match error {
                RecipeError::MenuFull => limits.map(|s| s.max_on),
                RecipeError::CardFull => limits.map(|s| s.max_listed),
                _ => None,
            }
            .unwrap_or_default()
            .to_string();
            let body = match error {
                RecipeError::Look(LookError::Service(detail)) => {
                    format!("{} {detail}", tr(body, &[]))
                }
                _ => tr(body, &[("max", &max)]),
            };
            (tr(title, &[]), body, tr("looks.ok", &[]))
        }
        Alert::ManualRestart => (
            tr("restart.title", &[]),
            tr("restart.manual_body", &[]),
            tr("update.ok", &[]),
        ),
    }
}

/// Ask before an update stops: the cancel button and a swipe down.
pub fn ask_cancel_update(app: AppState) {
    if app.update_cancelling() {
        return;
    }
    let step = crate::domain::update_step(&app.flow_phase());
    spawn(async move {
        let answer = alert_dialog()
            .title(tr("update.cancel_title", &[]))
            .description(tr(crate::domain::cancel_body_key(step), &[]))
            .cancel("keep", tr("update.cancel_keep", &[]))
            .destructive("cancel", tr("update.cancel_confirm", &[]))
            .show()
            .await;
        if answer.ok().and_then(|a| a.action).as_deref() == Some("cancel") {
            app.cancel_update();
        }
    });
}
