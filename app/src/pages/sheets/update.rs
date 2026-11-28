//! The update sheet, from the download to the restart, and the sheet for
//! an update that did not install.

use nsh_base::{progress, theme};
use nsh_core::prelude::*;

use super::parts::{STEP_INDICATOR, art_box, connection_shell, step_color, step_indicator};
use crate::domain::{CameraModel, UpdateError, UpdatePhase, update_step};
use crate::store::AppState;
use crate::strings::{format_bytes, tr};
use crate::ui::ty::text;
use crate::ui::{
    StepState, activity_indicator, secondary_button, sheet_primary_button, token, touch_target,
};

/// The update sheet: one sheet whose steps advance in place, from the
/// download to the restart, and that turns into the error that stopped it.
pub(super) fn update_sheet(
    app: AppState,
    error: Option<UpdateError>,
    installed: Option<String>,
) -> impl IntoElement {
    let phase = app.flow_phase();
    let step = match (&error, &installed) {
        (Some(error), _) => error.failed_step(),
        (_, Some(_)) => 4,
        _ => update_step(&phase),
    };
    let version = installed
        .clone()
        // The version that this update sends, or that waits on the camera.
        .or_else(|| app.pending_version())
        .unwrap_or_default();
    let model = format!("RICOH {}", app.model_name());
    let (title, subtitle) = update_copy(&phase, error.as_ref(), installed.as_deref(), &model);
    let titles = [
        tr("update.step_download", &[]),
        tr("update.step_send", &[]),
        tr("update.step_confirm", &[]),
        tr("update.step_restart", &[]),
    ];
    let failed = error.is_some();
    let done = installed.is_some();
    // An error shows the steps up to the one it stopped at, dimmed.
    let shown = if failed { step + 1 } else { 4 };
    // A restart the person runs has nothing under way: the sheet only says
    // what to do.
    let manual = !failed && matches!(phase, UpdatePhase::ManualRestart { .. });
    let steps = (!manual).then(|| {
        view().w_full().children((0..shown).map(|index| {
            let state = if failed && index == step {
                StepState::Failed
            } else if index < step {
                StepState::Done
            } else if index == step {
                StepState::Current
            } else {
                StepState::Waiting
            };
            // The restart shows as one step: its row and the sheet title say it all.
            let detailed =
                !failed && index == step && !matches!(phase, UpdatePhase::Restarting { .. });
            let extra = detailed.then(|| step_detail(&phase, &version));
            update_step_row(&titles[index], state, index + 1 == shown, extra).into_element()
        }))
    });
    let waits = !failed && !done && app.update_waits_on_camera();
    connection_shell(
        title,
        Some(subtitle),
        failed || done || waits,
        move || {
            if done {
                app.dismiss_installed()
            } else if failed {
                app.dismiss_error()
            } else {
                app.hide_update_sheet()
            }
        },
        view().w_full().gap_4().child(steps).child(update_buttons(
            app,
            &phase,
            error,
            installed.is_some(),
        )),
    )
}

/// The title and the line under it for the update sheet.
fn update_copy(
    phase: &UpdatePhase,
    error: Option<&UpdateError>,
    installed: Option<&str>,
    model: &str,
) -> (String, String) {
    if let Some(error) = error {
        return update_error_copy(error);
    }
    if let Some(version) = installed {
        return (
            tr("installed.title", &[("version", version)]),
            tr("installed.verified", &[]),
        );
    }
    let key = |k: &str| tr(k, &[]);
    match phase {
        UpdatePhase::WaitingConfirm { .. } => (key("confirm.title"), key("confirm.subtitle")),
        UpdatePhase::Confirmed { .. } | UpdatePhase::Restart { .. } => {
            (key("restart.title"), key("restart.subtitle"))
        }
        UpdatePhase::Restarting { .. } => {
            (key("restart.running_title"), key("restart.running_body"))
        }
        UpdatePhase::ManualRestart { .. } => (key("restart.title"), key("restart.manual_body")),
        _ => (
            tr("update.title", &[("model", model)]),
            key("update.keep_on"),
        ),
    }
}

pub(crate) fn update_error_copy(error: &UpdateError) -> (String, String) {
    match error {
        UpdateError::Service(message) => (tr("error.service_title", &[]), message.clone()),
        UpdateError::Download(_) => (
            tr("error.download_title", &[]),
            tr("update.download_body", &[]),
        ),
        UpdateError::ModuleNotKept(id) => (
            tr("error.module_title", &[]),
            tr("error.module_body", &[("module", id)]),
        ),
        UpdateError::PackageInvalid => (
            tr("error.package_title", &[]),
            tr("error.package_body", &[]),
        ),
        UpdateError::CardFull { bytes_needed } => (
            tr("error.card_full_title", &[]),
            tr(
                "error.card_full_body",
                &[("size", &format_bytes(*bytes_needed))],
            ),
        ),
        UpdateError::Refused => (
            tr("error.refused_title", &[]),
            tr("error.refused_body", &[]),
        ),
        UpdateError::ConnectionLost => (tr("error.lost_title", &[]), tr("error.lost_body", &[])),
    }
}

/// What the step under way shows below its name.
fn step_detail(phase: &UpdatePhase, version: &str) -> AnyElement {
    let note = |key: &'static str| {
        text(tr(key, &[]))
            .text_size(px(crate::ui::ty::FOOTNOTE))
            .line_height(px(18.0))
            .text_color(move || theme().muted_foreground)
            .into_element()
    };
    match phase {
        UpdatePhase::Downloading { progress, .. } | UpdatePhase::Sending { progress, .. } => view()
            .w_full()
            .gap_2()
            .child((*progress >= 0.0).then(|| progress_bar(*progress)))
            .into_element(),
        UpdatePhase::WaitingConfirm { .. } => confirm_detail(version).into_element(),
        UpdatePhase::Confirmed { .. } | UpdatePhase::Restart { .. } => note("restart.auto_body"),
        _ => view().into_element(),
    }
}

/// A black bar with its percentage.
fn progress_bar(value: f32) -> impl IntoElement {
    let percent = (value * 100.0).clamp(0.0, 100.0);
    view()
        .w_full()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .child(
            progress()
                .flex_1()
                .value(percent)
                .accessibility_label(tr("update.title", &[]))
                .track_style(|s| {
                    s.h(px(4.0))
                        .rounded(px(2.0))
                        .overflow_hidden()
                        .bg(move || if dark() { rgb(0x3A3A3C) } else { rgb(0xE3E3E8) })
                })
                .indicator_style(|s| s.h(px(4.0)).bg(move || theme().foreground)),
        )
        .child(
            text(format!("{percent:.0}%"))
                .text_size(px(crate::ui::ty::FOOTNOTE))
                .text_color(move || theme().muted_foreground),
        )
}

/// The camera's menu value for an update. An update another phone sent has
/// no version here; the camera shows it.
fn camera_menu_value(version: &str) -> String {
    tr("update.menu_value", &[("version", version)])
        .trim()
        .to_owned()
}

/// The confirm step: the steps on the camera.
pub(crate) fn confirm_detail(version: &str) -> impl IntoElement {
    let value = camera_menu_value(version);
    let number = |n: u8, copy: String| {
        view()
            .flex_row()
            .items_center()
            .gap(px(10.0))
            .child(
                view()
                    .size(px(20.0))
                    .rounded_full()
                    .items_center()
                    .justify_center()
                    .bg(move || token("tint-fill"))
                    .child(
                        text(n.to_string())
                            .text_size(px(crate::ui::ty::CAPTION))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xFFFFFF)),
                    ),
            )
            .child(
                text(copy)
                    .flex_1()
                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                    .line_height(px(20.0)),
            )
    };
    view()
        .w_full()
        .gap(px(8.0))
        .child(number(1, tr("confirm.step1", &[])))
        .child(number(2, tr("confirm.step2", &[("update", &value)])))
        .child(number(3, tr("confirm.step3", &[])))
}

/// One step of the update sheet: the connect step's look, with room for
/// the step's own detail.
fn update_step_row(
    title: &str,
    state: StepState,
    last: bool,
    extra: Option<AnyElement>,
) -> impl IntoElement {
    view()
        .flex_row()
        .child(
            view()
                .w(px(STEP_INDICATOR))
                .items_center()
                .child(step_indicator(state, false))
                .child((!last).then(|| {
                    view()
                        .flex_1()
                        .w(px(1.5))
                        .min_h(px(10.0))
                        .mt_1()
                        .bg(move || theme().border)
                })),
        )
        .child(
            view()
                .flex_1()
                .pl_3()
                .pb(px(14.0))
                .gap_2()
                .child(
                    text(title.to_owned())
                        .text_size(px(crate::ui::ty::CALLOUT))
                        .line_height(px(22.0))
                        .font_weight(if matches!(state, StepState::Waiting) {
                            FontWeight::NORMAL
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_color(move || step_color(state)),
                )
                .child(extra),
        )
}

/// The buttons under the steps, or `None` while the camera restarts: the
/// sheet then ends at its steps, without a gap for an empty row.
fn update_buttons(
    app: AppState,
    phase: &UpdatePhase,
    error: Option<UpdateError>,
    installed: bool,
) -> Option<AnyElement> {
    let buttons = view().w_full().gap_3();
    if app.update_cancelling() && !installed {
        return Some(buttons.child(cancelling_row()).into_element());
    }
    if installed {
        return Some(
            buttons
                .child(sheet_primary_button(tr("installed.done", &[]), move || {
                    app.dismiss_installed()
                }))
                .into_element(),
        );
    }
    if let Some(error) = error {
        let (primary, key): (Option<Box<dyn Fn()>>, &str) = match error {
            // The camera did not take the package: a new check tells why,
            // such as firmware that the release does not support.
            UpdateError::Refused => (
                Some(Box::new(move || {
                    app.dismiss_error();
                    app.refresh_update_info();
                })),
                "software.check",
            ),
            UpdateError::ModuleNotKept(_) | UpdateError::PackageInvalid => (None, ""),
            UpdateError::ConnectionLost => (
                Some(Box::new(move || app.retry_update())),
                "update.reconnect",
            ),
            _ => (
                Some(Box::new(move || app.retry_update())),
                "error.try_again",
            ),
        };
        return Some(
            buttons
                .child(match primary {
                    Some(action) => sheet_primary_button(tr(key, &[]), action).into_element(),
                    None => sheet_primary_button(tr("update.ok", &[]), move || app.dismiss_error())
                        .into_element(),
                })
                .child(app.has_pending_update().then(|| {
                    secondary_button(tr("error.cancel_pending", &[]), move || app.cancel_update())
                }))
                .child(
                    primary_is_some(&error).then(|| {
                        secondary_button(tr("error.close", &[]), move || app.dismiss_error())
                    }),
                )
                .into_element(),
        );
    }
    let buttons = match phase {
        UpdatePhase::Confirmed { .. } | UpdatePhase::Restart { .. } => buttons
            .child(sheet_primary_button(tr("restart.run", &[]), move || {
                app.restart_for_update()
            }))
            .child(secondary_button(tr("restart.manual", &[]), move || {
                app.restart_manually()
            }))
            .into_element(),
        UpdatePhase::ManualRestart { .. } => buttons
            .child(sheet_primary_button(
                tr("update.reconnect", &[]),
                move || app.reconnect_for_update(),
            ))
            .child(secondary_button(tr("update.cancel", &[]), move || {
                crate::pages::ask_cancel_update(app)
            }))
            .into_element(),
        UpdatePhase::Restarting { .. } => return None,
        _ => buttons
            .child(secondary_button(tr("update.cancel", &[]), move || {
                crate::pages::ask_cancel_update(app)
            }))
            .into_element(),
    };
    Some(buttons)
}

/// The cancel is under way: it waits for a send in flight to settle.
fn cancelling_row() -> impl IntoElement {
    let title = tr("update.cancelling", &[]);
    view()
        .w_full()
        .flex_row()
        .min_h(px(touch_target()))
        .items_center()
        .justify_center()
        .gap_2()
        .label(title.clone())
        .accessibility_state(|| AccessibilityState {
            busy: true,
            ..AccessibilityState::default()
        })
        .child(activity_indicator(16.0))
        .child(
            text(title)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .line_height(px(22.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || theme().muted_foreground),
        )
}

fn primary_is_some(error: &UpdateError) -> bool {
    !matches!(
        error,
        UpdateError::ModuleNotKept(_) | UpdateError::PackageInvalid
    )
}

/// A confirmed update did not install: the camera runs without GRKit.
pub(super) fn install_failed(app: AppState) -> impl IntoElement {
    let model = app.selected_model().unwrap_or(CameraModel::Gr3);
    connection_shell(
        tr("update.failed_title", &[]),
        Some(tr("update.failed_sub", &[])),
        true,
        move || app.dismiss_install_failed(),
        view()
            .w_full()
            .gap_4()
            .child(art_box(model))
            .child(
                text(tr("update.failed_body", &[]))
                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                    .line_height(px(20.0))
                    .text_color(move || theme().foreground),
            )
            .child(
                view()
                    .w_full()
                    .gap_1()
                    .child(sheet_primary_button(
                        tr("update.reconnect", &[]),
                        move || app.reconnect_install_failed(),
                    ))
                    .child(crate::ui::text_link(
                        tr("update.open_site", &[]),
                        true,
                        || {
                            spawn(async {
                                let _ = open_url(crate::store::INSTALL_URL).await;
                            });
                        },
                    )),
            ),
    )
}
