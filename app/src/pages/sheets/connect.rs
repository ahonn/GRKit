//! The sheets that find a camera and connect to it.

use std::time::Duration;

use nsh_base::theme;
use nsh_core::prelude::*;

use super::parts::{
    STEP_INDICATOR, art_box, connection_shell, connection_shell_with_back, step_color,
    step_indicator,
};
use crate::domain::{CameraModel, LinkError};
use crate::store::{AppState, link_error_body, link_error_title};
use crate::strings::tr;
use crate::ui::ty::text;
use crate::ui::{
    StepState, activity_indicator, camera_art, connection_button, secondary_button,
    sheet_primary_button, token,
};

/// Whether Add Camera shows its one camera as a card.
pub(super) fn scan_shows_card(app: AppState) -> bool {
    app.nearby_cameras().len() == 1 && !app.scan_list_pinned()
}

/// One row of the connect checklist, after the camera app's: the
/// indicator, a line down to the next step, and the step's name.
fn connection_step(
    title: &str,
    subtitle: Option<&str>,
    state: StepState,
    last: bool,
    changed: bool,
) -> impl IntoElement {
    let current = matches!(state, StepState::Current | StepState::Failed);
    view()
        .flex_row()
        .child(
            view()
                .w(px(STEP_INDICATOR))
                .items_center()
                .child(step_indicator(state, changed))
                .child((!last).then(|| {
                    view()
                        .flex_1()
                        .w(px(1.5))
                        .min_h(px(12.0))
                        .mt_1()
                        .bg(move || theme().border)
                })),
        )
        .child(
            view()
                .flex_1()
                .pl_3()
                .pb_4()
                .gap_1()
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
                .child(subtitle.filter(|_| current).map(|s| {
                    text(s.to_owned())
                        .text_size(px(crate::ui::ty::FOOTNOTE))
                        .line_height(px(18.0))
                        .text_color(move || theme().muted_foreground)
                })),
        )
}

pub(super) fn nearby(app: AppState) -> impl IntoElement {
    let model = app.selected_model().unwrap_or(CameraModel::Gr3);
    let saved = app.selected_is_saved();
    let name = format!("RICOH {}", model.display_name());
    // A prompt that appeared unasked is declined with Not Now (or a swipe).
    connection_shell(
        tr("cam.prompt_title", &[("model", &name)]),
        Some(tr("cam.prompt_body", &[])),
        true,
        move || app.dismiss_nearby(),
        view()
            .w_full()
            .gap_5()
            .child(art_box(model))
            .child(
                (!saved)
                    .then(|| pairing_hint("cam.first_time_device", model.pairing_path_key(), true)),
            )
            .child(
                view()
                    .w_full()
                    .gap_3()
                    .child(connection_button(tr("nearby.connect", &[]), {
                        let app = app.clone();
                        move || app.connect_nearby()
                    }))
                    .child(secondary_button(tr("nearby.not_now", &[]), move || {
                        app.dismiss_nearby()
                    })),
            ),
    )
}

/// The pairing guide: a lead-in and the camera menu path in bold.
fn pairing_hint(lead: &'static str, path: &'static str, centered: bool) -> impl IntoElement {
    let align = |t: Text| if centered { t.text_center() } else { t };
    view()
        .w_full()
        .gap(px(2.0))
        .px(px(if centered { 0.0 } else { 6.0 }))
        .when(centered, |v| v.items_center())
        .child(align(
            text(tr(lead, &[]))
                .text_size(px(crate::ui::ty::FOOTNOTE))
                .line_height(px(18.0))
                .text_color(move || theme().muted_foreground),
        ))
        .child(align(
            text(tr(path, &[]))
                .text_size(px(crate::ui::ty::FOOTNOTE))
                .line_height(px(18.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        ))
}

fn power_label(on: bool) -> String {
    tr(
        if on {
            "nearby.powered_on"
        } else {
            "scan.nearby_off"
        },
        &[],
    )
}

/// The Bluetooth name and the power state: `GR_92910A · Standby`.
fn camera_status(cam: &crate::domain::NearbyCamera) -> String {
    match &cam.name {
        Some(name) => format!("{name} · {}", power_label(cam.powered_on)),
        None => power_label(cam.powered_on),
    }
}

fn power_dot(on: bool) -> impl IntoElement {
    view().size(px(7.0)).rounded_full().bg(move || {
        if on {
            rgb(0x2A9D4B).into()
        } else {
            theme().muted_foreground
        }
    })
}

pub(super) fn scan(app: AppState) -> impl IntoElement {
    let cameras = app.nearby_cameras();
    // Exactly one camera: offer it at once, until the user goes back to the list.
    if scan_shows_card(app) {
        return device_card(app, cameras[0].clone()).into_element();
    }
    connection_shell(
        tr("scan.title", &[]),
        Some(tr("scan.subtitle", &[])),
        true,
        {
            let app = app.clone();
            move || app.set_sheet(None)
        },
        view()
            .w_full()
            .gap_2()
            .child(
                view()
                    .rounded(px(20.0))
                    .overflow_hidden()
                    .bg(move || token("connection-muted"))
                    .children(cameras.into_iter().enumerate().map(|(index, cam)| {
                        let app = app.clone();
                        let id = cam.peripheral_id.clone();
                        let model = cam.model;
                        let status = camera_status(&cam);
                        let powered = cam.powered_on;
                        view()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .min_h(px(72.0))
                            .px_5()
                            .style_when(index > 0, |st| {
                                st.border_t_1().border_t_color(move || theme().border)
                            })
                            .role(AccessibilityRole::Button)
                            .label(format!("RICOH {}", model.display_name()))
                            .on_press(move || app.select_nearby(id.clone()))
                            .active(|s| s.bg(move || theme().accent))
                            .child(camera_art(model, 64.0, false))
                            .child(
                                view()
                                    .flex_1()
                                    .gap(px(2.0))
                                    .child(
                                        text(format!("RICOH {}", model.display_name()))
                                            .text_size(px(crate::ui::ty::HEADLINE))
                                            .font_weight(FontWeight::SEMIBOLD),
                                    )
                                    .child(
                                        view()
                                            .flex_row()
                                            .items_center()
                                            .gap_2()
                                            .child(power_dot(powered))
                                            .child(
                                                text(status)
                                                    .flex_shrink(1.0)
                                                    .text_size(px(crate::ui::ty::FOOTNOTE))
                                                    .line_height(px(18.0))
                                                    .text_color(move || theme().muted_foreground),
                                            ),
                                    ),
                            )
                            .child(crate::ui::chevron())
                    }))
                    .child(
                        // The list stays open for more cameras: the words
                        // say so beside the spinner.
                        view()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .min_h(px(64.0))
                            .justify_center()
                            .px_4()
                            .label(tr("scan.looking", &[]))
                            .child(activity_indicator(16.0))
                            .child(
                                text(tr("scan.looking", &[]))
                                    .text_size(px(crate::ui::ty::FOOTNOTE))
                                    .text_color(move || theme().muted_foreground),
                            ),
                    ),
            )
            .child(pairing_hint("cam.first_time", "cam.path_common", false)),
    )
    .into_element()
}

/// The one camera Add Camera found, offered for a single tap.
fn device_card(app: AppState, cam: crate::domain::NearbyCamera) -> impl IntoElement {
    let model = cam.model;
    let id = cam.peripheral_id.clone();
    let back_app = app.clone();
    connection_shell_with_back(
        format!("RICOH {}", model.display_name()),
        Some(camera_status(&cam)),
        true,
        {
            let app = app.clone();
            move || app.set_sheet(None)
        },
        Some(Box::new(move || back_app.pin_scan_list())),
        view()
            .w_full()
            .gap_5()
            .child(art_box(model))
            .child(pairing_hint(
                "cam.first_time_device",
                model.pairing_path_key(),
                true,
            ))
            .child(connection_button(tr("nearby.connect", &[]), move || {
                app.select_nearby(id.clone())
            })),
    )
}

/// Title and hint of each connect step: Bluetooth, Wi-Fi, the camera.
fn step_copy(app: AppState, model: CameraModel) -> [(String, String); 3] {
    let first = if app.connecting_saved() {
        (tr("cam.step_wake", &[]), tr("cam.step_wake_sub", &[]))
    } else {
        (tr("cam.step_pair", &[]), tr(model.pairing_step_key(), &[]))
    };
    [
        first,
        (tr("cam.step_wifi", &[]), tr("cam.step_wifi_sub", &[])),
        (tr("cam.step_verify", &[]), tr("cam.step_verify_sub", &[])),
    ]
}

/// The time on the Wi-Fi step after which GR IV gets its slower start
/// explained.
const SLOW_WIFI: Duration = Duration::from_secs(10);

pub(super) fn connecting(app: AppState, step: u8) -> impl IntoElement {
    let model_kind = app.selected_model().unwrap_or(CameraModel::Gr3);
    let model = format!("RICOH {}", app.model_name());
    // The last step stays under way until the sheet closes.
    let state = move |index: u8| {
        if step > index && index < 2 {
            StepState::Done
        } else if step == index || (index == 2 && step > 2) {
            StepState::Current
        } else {
            StepState::Waiting
        }
    };
    // A step changed at this step: it just finished, or just started. The
    // first step's start is the sheet's own entrance.
    let changed = move |index: u8| step == index + 1 || (step == index && index > 0);
    // GR IV starts its Wi-Fi slowly; after a while on that step, say so.
    let slow = signal(false);
    if step == 1 && model_kind == CameraModel::Gr4 {
        spawn(async move {
            timer(SLOW_WIFI).await;
            slow.set(true);
        });
    }
    let copy = step_copy(app, model_kind);
    let [(t0, s0), (t1, s1), (t2, s2)] = copy;
    connection_shell(
        tr("connecting.title", &[("model", &model)]),
        Some(tr("connecting.keep_on", &[])),
        false,
        move || app.cancel_connect(),
        view()
            .w_full()
            .gap_4()
            .child(
                view()
                    .w_full()
                    .child(connection_step(&t0, Some(&s0), state(0), false, changed(0)))
                    .child(dynamic(move || {
                        let hint = if slow.get() {
                            tr("cam.step_wifi_slow", &[])
                        } else {
                            s1.clone()
                        };
                        connection_step(&t1, Some(&hint), state(1), false, changed(1))
                            .into_element()
                    }))
                    .child(connection_step(&t2, Some(&s2), state(2), true, changed(2))),
            )
            .child(secondary_button(tr("connecting.cancel", &[]), move || {
                app.cancel_connect()
            })),
    )
}

pub(super) fn connect_error_sheet(app: AppState, error: LinkError) -> impl IntoElement {
    use crate::domain::ErrorAction;
    let title = link_error_title(&error);
    let body = link_error_body(&error);
    let action = error.copy().2;
    let model = app.selected_model().unwrap_or(CameraModel::Gr3);
    let retry = matches!(
        action,
        ErrorAction::Retry | ErrorAction::RetryOrSettings | ErrorAction::RetryAfterSwitch
    );
    let retry_label = if action == ErrorAction::RetryAfterSwitch {
        "conn.switched_wifi"
    } else {
        "error.try_again"
    };
    let settings = matches!(action, ErrorAction::Settings | ErrorAction::RetryOrSettings);
    let open_settings = || {
        spawn(async {
            crate::platform::open_app_settings().await;
        });
    };
    let no_grkit = error == LinkError::NeedsGrkit;
    let app_old = error == LinkError::AppTooOld;
    // The steps, dimmed and cut at the one that failed.
    let steps = error.failed_step().map(|failed| {
        let copy = step_copy(app, model);
        view().w_full().children((0..=failed).map(|index| {
            let state = if index == failed {
                StepState::Failed
            } else {
                StepState::Done
            };
            let (title, _) = &copy[index as usize];
            connection_step(title, None, state, index == failed, false).into_element()
        }))
    });
    let gr4_wifi = model == CameraModel::Gr4 && error.failed_step() == Some(1);
    connection_shell(
        title,
        Some(body),
        true,
        {
            let app = app.clone();
            move || app.dismiss_connect_error()
        },
        view()
            .w_full()
            .gap_4()
            .child(steps)
            .child(
                error.needs_pairing().then(|| {
                    pairing_hint("cam.first_time_device", model.pairing_path_key(), false)
                }),
            )
            .child(gr4_wifi.then(|| {
                text(tr("cam.step_wifi_slow", &[]))
                    .text_size(px(crate::ui::ty::FOOTNOTE))
                    .line_height(px(18.0))
                    .text_color(move || theme().muted_foreground)
            }))
            .child(
                view()
                    .w_full()
                    .gap_3()
                    .child(if app_old {
                        sheet_primary_button(tr("app.store", &[]), || {
                            spawn(async {
                                let _ = open_url(crate::store::app_store_url()).await;
                            });
                        })
                        .into_element()
                    } else if no_grkit {
                        sheet_primary_button(tr("cam.install_guide", &[]), || {
                            spawn(async {
                                let _ = open_url(crate::store::INSTALL_URL).await;
                            });
                        })
                        .into_element()
                    } else if retry {
                        sheet_primary_button(tr(retry_label, &[]), {
                            let app = app.clone();
                            move || app.retry_connect()
                        })
                        .into_element()
                    } else if settings {
                        sheet_primary_button(tr("conn.open_settings", &[]), open_settings)
                            .into_element()
                    } else {
                        view().into_element()
                    })
                    .child(no_grkit.then(|| {
                        secondary_button(tr("error.try_again", &[]), {
                            let app = app.clone();
                            move || app.retry_connect()
                        })
                    }))
                    .child(
                        (retry && settings && !no_grkit).then(|| {
                            secondary_button(tr("conn.open_settings", &[]), open_settings)
                        }),
                    )
                    .child((!no_grkit && !app_old).then(|| {
                        secondary_button(tr("cam.detect", &[]), move || {
                            app.dismiss_connect_error();
                            app.detect_now();
                        })
                    })),
            ),
    )
}
