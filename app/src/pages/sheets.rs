//! Modal sheets for pairing, connecting, update progress, and errors.

use nsh_base::{SheetEvent, progress, sheet, theme};
use nsh_core::prelude::*;
use std::time::Duration;

use crate::domain::{CameraModel, LinkError, UpdateError, UpdatePhase};
use crate::store::{AppState, Sheet, link_error_body, link_error_title};
use crate::strings::{format_bytes, tr};
use crate::ui::{
    NATIVE_SHEET, STEP1, STEP2, STEP3, StepState, activity_indicator, camera_art, checklist_row,
    connection_button, connection_icon, failed_icon, secondary_button, sheet_primary_button,
    sheet_shell, sheet_surface, token, touch_target,
};

/// Every sheet closes with a swipe down or a press on the backdrop, as in
/// the camera app.
fn dismissible(app: AppState) -> bool {
    app.sheet().is_some()
}

/// The user swiped a sheet away: what its own way out does (close, Not
/// Now, Cancel, Done).
fn dismiss(app: AppState) {
    match app.sheet() {
        Some(Sheet::Nearby) => app.dismiss_nearby(),
        Some(Sheet::Scan | Sheet::ImportLook) => app.set_sheet(None),
        Some(Sheet::Connecting { .. }) => app.cancel_connect(),
        Some(Sheet::ConnectError(_)) => app.dismiss_connect_error(),
        Some(Sheet::Sending | Sheet::Confirm | Sheet::Restart) => app.cancel_update(),
        Some(Sheet::Installed { .. }) => app.dismiss_installed(),
        Some(Sheet::Error(_)) => app.dismiss_error(),
        None => {}
    }
}

/// What the open sheet shows.
fn sheet_body(app: AppState) -> AnyElement {
    match app.sheet() {
        Some(Sheet::Nearby) => nearby(app.clone()).into_element(),
        Some(Sheet::Scan) => scan(app.clone()).into_element(),
        Some(Sheet::ImportLook) => crate::pages::import_sheet(app.clone()).into_element(),
        Some(Sheet::Connecting { step }) => connecting(app.clone(), step).into_element(),
        Some(Sheet::ConnectError(error)) => connect_error_sheet(app.clone(), error).into_element(),
        Some(Sheet::Sending) => sending(app.clone()).into_element(),
        Some(Sheet::Confirm) => confirm(app.clone()).into_element(),
        Some(Sheet::Restart) => restart(app.clone()).into_element(),
        Some(Sheet::Installed { version, previous }) => {
            installed(app.clone(), version, previous).into_element()
        }
        Some(Sheet::Error(error)) => error_sheet(app.clone(), error).into_element(),
        None => view().into_element(),
    }
}

type Action = Box<dyn Fn()>;

/// The buttons at the top of the open sheet: back, then close. iOS puts
/// them in the system bar of its native sheet; elsewhere the shells draw
/// them.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
fn sheet_buttons(app: AppState) -> (Option<Action>, Option<Action>) {
    let close = |f: Box<dyn Fn()>| Some(f);
    match app.sheet() {
        Some(Sheet::Scan) => {
            let back: Option<Box<dyn Fn()>> = scan_shows_card(app)
                .then(|| Box::new(move || app.pin_scan_list()) as Box<dyn Fn()>);
            (back, close(Box::new(move || app.set_sheet(None))))
        }
        Some(Sheet::ImportLook) => (None, close(Box::new(move || app.set_sheet(None)))),
        Some(Sheet::ConnectError(_)) => {
            (None, close(Box::new(move || app.dismiss_connect_error())))
        }
        _ => (None, None),
    }
}

/// Whether Add Camera shows its one camera as a card.
fn scan_shows_card(app: AppState) -> bool {
    app.nearby_cameras().len() == 1 && !app.scan_list_pinned()
}

/// iOS: a native sheet sized to its content, with the system bar's
/// buttons.
#[cfg(target_os = "ios")]
pub fn overlay(app: AppState) -> impl IntoElement {
    use nsh_base::{dialog, nav_page, nav_stack};
    use nsh_core::element::header::HeaderItem;
    use nsh_core::{HeaderSystemItem, ModalPresentation, SheetDetent};

    let height = signal(Points(420.0));
    dynamic(move || {
        app.sheet().is_some().then(|| {
            untracked(move || {
                dialog()
                    .presentation(ModalPresentation::PageSheet)
                    .detents(move || vec![SheetDetent::Height(height.get())])
                    .grabber(move || dismissible(app))
                    .close_on_swipe(move || dismissible(app))
                    .close_on_backdrop_press(move || dismissible(app))
                    .close_on_escape(move || dismissible(app))
                    .on_cancel(move || {
                        if dismissible(app) {
                            dismiss(app)
                        }
                    })
                    .child(nav_stack().flex_1().child(dynamic(move || {
                        // A back button comes and goes with the page.
                        let back = sheet_buttons(app).0.is_some();
                        untracked(move || {
                            let page = nav_page()
                                .key(if back { "back" } else { "plain" })
                                .header_transparent(true)
                                .header_right_items(move || {
                                    sheet_buttons(app)
                                        .1
                                        .map(|close| {
                                            vec![
                                                HeaderItem::system(HeaderSystemItem::Close)
                                                    .label(tr("scan.close", &[]))
                                                    .on_press(close),
                                            ]
                                        })
                                        .unwrap_or_default()
                                });
                            let page = if back {
                                page.header_left_symbol("chevron.backward").header_left(
                                    view()
                                        .role(AccessibilityRole::Button)
                                        .label(tr("scan.back", &[]))
                                        .on_press(move || {
                                            if let Some(back) = sheet_buttons(app).0 {
                                                back()
                                            }
                                        }),
                                )
                            } else {
                                page
                            };
                            page.child(
                                view()
                                    .on_layout(move |b: &Bounds<Points>| {
                                        if b.size.height.0 > 0.0 {
                                            height.set(b.size.height);
                                        }
                                    })
                                    .child(dynamic(move || sheet_body(app))),
                            )
                            .into_element()
                        })
                    })))
            })
        })
    })
}

#[cfg(not(target_os = "ios"))]
pub fn overlay(app: AppState) -> impl IntoElement {
    let dismissible = move || dismissible(app);
    sheet()
        .open({
            let app = app.clone();
            move || Some(app.sheet().is_some())
        })
        .side(Placement::Bottom)
        .close_on_backdrop_press(dismissible)
        .close_on_swipe(dismissible)
        .close_on_escape(dismissible)
        .backdrop_style(|s| s.bg(rgba(0x00000047)))
        .popup_style(|s| {
            s.mx(px(if cfg!(target_os = "android") {
                0.0
            } else {
                8.0
            }))
            .mb(px(if cfg!(target_os = "android") {
                0.0
            } else {
                8.0
            }))
            .rounded_t(px(if cfg!(target_os = "android") {
                28.0
            } else {
                30.0
            }))
            .rounded_b(px(if cfg!(target_os = "android") {
                0.0
            } else {
                30.0
            }))
            .bg(sheet_surface)
            .shadow([BoxShadow {
                offset_x: Points(0.0),
                offset_y: Points(20.0),
                blur: Points(60.0),
                spread: Points(0.0),
                color: rgba(0x00000040).into(),
                inset: false,
            }])
        })
        .on_open_change({
            let app = app.clone();
            move |e: &SheetEvent| {
                if let SheetEvent::OpenChange(false, _) = e {
                    dismiss(app)
                }
            }
        })
        .child(dynamic(move || sheet_body(app)))
}

/// Connection sheets follow the camera discovery layout without changing update sheets.
fn connection_shell(
    title: impl Into<String>,
    subtitle: Option<String>,
    show_close: bool,
    on_close: impl Fn() + 'static,
    body: impl IntoElement,
) -> impl IntoElement {
    connection_shell_with_back(title, subtitle, show_close, on_close, None, body)
}

/// [`connection_shell`] with a back button at the top left, the twin of
/// the close button, when `on_back` is set.
fn connection_shell_with_back(
    title: impl Into<String>,
    subtitle: Option<String>,
    show_close: bool,
    on_close: impl Fn() + 'static,
    on_back: Option<Box<dyn Fn()>>,
    body: impl IntoElement,
) -> impl IntoElement {
    let has_back = on_back.is_some();
    view()
        .relative()
        .w_full()
        .gap_5()
        .px(px(24.0))
        .pt(px(if NATIVE_SHEET { 24.0 } else { 6.0 }))
        // A native sheet adds the bottom safe area to its height itself.
        .when(NATIVE_SHEET, |v| v.pb(px(16.0)))
        .when(!NATIVE_SHEET, |v| v.pb_safe_area_plus(px(8.0)))
        .child((!NATIVE_SHEET).then(|| {
            view()
                .self_center()
                .w(px(36.0))
                .h(px(4.0))
                .rounded_full()
                .bg(rgb(0xC7C7CC))
        }))
        .child(
            view()
                .gap(px(6.0))
                .pr(if show_close {
                    px(touch_target())
                } else {
                    px(0.0)
                })
                .pl(if has_back {
                    px(touch_target())
                } else {
                    px(0.0)
                })
                // Between a back and a close button, the title is centered.
                .when(has_back, |v| v.items_center())
                .child(
                    text(title.into())
                        .text_size(px(22.0))
                        .font_weight(FontWeight::BOLD)
                        .when(has_back, |t| t.text_center()),
                )
                .child(subtitle.map(|s| {
                    text(s)
                        .text_size(px(15.0))
                        .line_height(px(22.0))
                        .text_color(move || theme().muted_foreground)
                        .when(has_back, |t| t.text_center())
                })),
        )
        .child((show_close && !NATIVE_SHEET).then(|| {
            view().absolute().right(px(18.0)).top(px(22.0)).child(
                view()
                    .size(px(touch_target()))
                    .items_center()
                    .justify_center()
                    .role(AccessibilityRole::Button)
                    .label(tr("scan.close", &[]))
                    .on_press(on_close)
                    .child(connection_icon("M6 6l12 12M18 6L6 18")),
            )
        }))
        .child(on_back.filter(|_| !NATIVE_SHEET).map(|on_back| {
            view().absolute().left(px(18.0)).top(px(22.0)).child(
                view()
                    .size(px(touch_target()))
                    .items_center()
                    .justify_center()
                    .role(AccessibilityRole::Button)
                    .label(tr("scan.back", &[]))
                    .on_press(on_back)
                    .child(connection_icon("M15 5l-7 7 7 7")),
            )
        }))
        .child(body)
}

/// The indicator column of the connect checklist, after the camera app's:
/// a faint circle while waiting, the platform loader for the step under
/// way, and
/// a filled check once done. Only the step that has just finished pops its
/// check in; the sheet is drawn again at each step, and older checks stay
/// still. The loader fades in when its step starts.
const STEP_INDICATOR: f32 = 22.0;

const CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5.5 12.5l4.5 4.5L18.5 7.5"/></svg>"#;

fn step_indicator(state: StepState, changed: bool) -> AnyElement {
    let entering = |preset| motion().entering(preset);
    match state {
        StepState::Waiting => view()
            .size(px(STEP_INDICATOR))
            .rounded_full()
            .border_2()
            .border_color(move || theme().border)
            .into_element(),
        StepState::Current => {
            let spinner = view()
                .size(px(STEP_INDICATOR))
                .items_center()
                .justify_center()
                .child(activity_indicator(STEP_INDICATOR));
            if changed {
                spinner
                    .bind_motion(&entering(MotionPreset::FadeIn))
                    .into_element()
            } else {
                spinner.into_element()
            }
        }
        StepState::Failed => failed_icon(STEP_INDICATOR).into_element(),
        StepState::Done => {
            let badge = view()
                .size(px(STEP_INDICATOR))
                .rounded_full()
                .items_center()
                .justify_center()
                .bg(move || token("tint"))
                .text_color(move || rgb(0xFFFFFF))
                .child(svg(nsh_core::element::ImageSrc::svg_markup(CHECK)).size(px(12.0)));
            if changed {
                badge
                    .bind_motion(&entering(MotionPreset::ZoomIn))
                    .into_element()
            } else {
                badge.into_element()
            }
        }
    }
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
    let opacity = match state {
        StepState::Waiting => 0.4,
        StepState::Current | StepState::Failed => 1.0,
        StepState::Done => 0.6,
    };
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
                        .text_size(px(16.0))
                        .line_height(px(22.0))
                        .font_weight(if matches!(state, StepState::Waiting) {
                            FontWeight::NORMAL
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_color(move || theme().foreground)
                        .opacity(opacity),
                )
                .child(subtitle.filter(|_| current).map(|s| {
                    text(s.to_owned())
                        .text_size(px(13.0))
                        .line_height(px(18.0))
                        .text_color(move || theme().muted_foreground)
                })),
        )
}

fn nearby(app: AppState) -> impl IntoElement {
    let model = app.selected_model().unwrap_or(CameraModel::Gr3);
    let saved = app.selected_is_saved();
    let name = format!("RICOH {}", model.display_name());
    // A prompt that appeared unasked is declined with Not Now (or a swipe).
    connection_shell(
        tr("cam.prompt_title", &[("model", &name)]),
        Some(tr("cam.prompt_body", &[])),
        false,
        || {},
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
                    .gap_1()
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

/// The camera art on a quiet panel.
fn art_box(model: CameraModel) -> impl IntoElement {
    view()
        .w_full()
        .rounded(px(8.0))
        .bg(move || token("connection-muted"))
        .py_5()
        .items_center()
        .child(camera_art(model, 150.0, false))
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
                .text_size(px(13.0))
                .line_height(px(18.0))
                .text_color(move || theme().muted_foreground),
        ))
        .child(align(
            text(tr(path, &[]))
                .text_size(px(13.0))
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

fn scan(app: AppState) -> impl IntoElement {
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
                                            .text_size(px(17.0))
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
                                                    .text_size(px(13.0))
                                                    .line_height(px(18.0))
                                                    .text_color(move || theme().muted_foreground),
                                            ),
                                    ),
                            )
                            .child(
                                text("›")
                                    .text_size(px(22.0))
                                    .text_color(move || theme().muted_foreground),
                            )
                    }))
                    .child(
                        view()
                            .flex_row()
                            .items_center()
                            .min_h(px(64.0))
                            .justify_center()
                            .gap_2()
                            .px_4()
                            .child(activity_indicator(16.0))
                            .child(
                                text(move || tr("scan.looking", &[]))
                                    .text_size(px(15.0))
                                    .line_height(px(22.0))
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

fn connecting(app: AppState, step: u8) -> impl IntoElement {
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

fn connect_error_sheet(app: AppState, error: LinkError) -> impl IntoElement {
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
    // The steps, dimmed and cut at the one that failed.
    let steps = error.failed_step().map(|failed| {
        let copy = step_copy(app, model);
        view()
            .w_full()
            .opacity(0.5)
            .children((0..=failed).map(|index| {
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
                text(tr("cam.gr4_wifi_note", &[]))
                    .text_size(px(13.0))
                    .line_height(px(18.0))
                    .text_color(move || theme().muted_foreground)
            }))
            .child(
                view()
                    .w_full()
                    .gap_1()
                    .child(if no_grkit {
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
                    .child((!no_grkit).then(|| {
                        secondary_button(tr("cam.detect", &[]), move || {
                            app.dismiss_connect_error();
                            app.detect_now();
                        })
                    })),
            ),
    )
}

fn sending(app: AppState) -> impl IntoElement {
    let model = app.model_name();
    let progress_val = match app.flow_phase() {
        UpdatePhase::Sending { progress, .. } => progress * 100.0,
        _ => 100.0,
    };
    let indeterminate = progress_val < 0.0;
    let version = app.package().and_then(|p| p.available);
    sheet_shell(
        tr("sending.title", &[("model", &model)]),
        version.map(|version| format!("GRKit {version}")),
        Some(app.nearby_model().unwrap_or(CameraModel::Gr3)),
        false,
        || {},
        view()
            .w_full()
            .gap_4()
            .items_center()
            .child(
                view()
                    .w_full()
                    .gap_3()
                    .p_4()
                    .rounded(px(8.0))
                    .bg(move || theme().muted)
                    .child(
                        view()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(
                                text(tr(
                                    if indeterminate {
                                        "sending.preparing"
                                    } else {
                                        "sending.progress"
                                    },
                                    &[],
                                ))
                                .text_size(px(17.0))
                                .font_weight(FontWeight::SEMIBOLD),
                            )
                            .child((!indeterminate).then(|| {
                                text(format!("{progress_val:.0}%"))
                                    .text_size(px(15.0))
                                    .line_height(px(22.0))
                                    .text_color(move || theme().muted_foreground)
                            })),
                    )
                    .child((!indeterminate).then(|| {
                        progress()
                            .value(progress_val)
                            .accessibility_label(tr("sending.progress", &[]))
                            .track_style(|s| {
                                s.h(px(4.0))
                                    .rounded(px(3.0))
                                    .overflow_hidden()
                                    .bg(move || if dark() { rgb(0x3A3A3C) } else { rgb(0xE3E3E8) })
                            })
                            .indicator_style(|s| {
                                s.h(px(4.0)).rounded(px(3.0)).bg(move || token("tint"))
                            })
                    })),
            )
            .child(
                view()
                    .w_full()
                    .gap_1()
                    .child(checklist_row(
                        &tr("sending.step_confirm", &[]),
                        None,
                        StepState::Waiting,
                    ))
                    .child(checklist_row(
                        &tr("sending.step_restart", &[]),
                        None,
                        StepState::Waiting,
                    )),
            )
            .child(
                text(move || tr("sending.keep_on", &[]))
                    .text_size(px(13.0))
                    .line_height(px(18.0))
                    .text_color(move || theme().muted_foreground),
            )
            .child(secondary_button(tr("sending.cancel", &[]), move || {
                app.cancel_update()
            })),
    )
}

fn confirm(app: AppState) -> impl IntoElement {
    sheet_shell(
        tr("confirm.title", &[]),
        Some(tr("confirm.subtitle", &[])),
        None,
        false,
        || {},
        view()
            .w_full()
            .gap_4()
            .child(installation_steps())
            .child(
                view()
                    .w_full()
                    .p_4()
                    .rounded(px(8.0))
                    .bg(move || theme().muted)
                    .child(
                        text(move || {
                            tr(
                                "confirm.waiting",
                                &[("time", &app.confirm_deadline_label())],
                            )
                        })
                        .text_size(px(15.0))
                        .line_height(px(22.0))
                        .text_color(move || token("link"))
                        .text_center(),
                    ),
            )
            .child(secondary_button(tr("sending.cancel", &[]), move || {
                app.cancel_update()
            })),
    )
}

// The guide has a fixed cross-axis extent. Native iOS scroll views can otherwise
// move vertically when their content exceeds their height, even in horizontal mode.
const GUIDE_WIDTH: f32 = 300.0;
const GUIDE_HEIGHT: f32 = 258.0;
const GUIDE_STRIDE: f32 = GUIDE_WIDTH + 12.0;

fn installation_steps() -> impl IntoElement {
    let index = signal(0u32);
    let track = element_ref();
    view()
        .gap_1()
        .child(
            scroll()
                .element_ref(&track)
                .horizontal()
                .scrollbar(false)
                .bounces(false)
                .snap_interval(Points(GUIDE_STRIDE))
                .disable_interval_momentum(true)
                .deceleration(0.99)
                .h(px(GUIDE_HEIGHT))
                .mx(px(-20.0))
                .px(px(26.0))
                .gap(px(12.0))
                .label("Steps on the camera")
                .on_scroll_end(move |offset: &Point<Points>| {
                    index.set((offset.x.0 / GUIDE_STRIDE).round().clamp(0.0, 2.0) as u32);
                })
                .child(step_slide(STEP1, 1, tr("confirm.step1", &[])))
                .child(step_slide(STEP2, 2, tr("confirm.step2", &[])))
                .child(step_slide(STEP3, 3, tr("confirm.step3", &[]))),
        )
        .child(
            view()
                .flex_row()
                .justify_center()
                .child(page_dot(0, index, track))
                .child(page_dot(1, index, track))
                .child(page_dot(2, index, track)),
        )
}

fn page_dot(i: u32, index: Signal<u32>, track: ElementRef) -> impl IntoElement {
    let caption_key = ["confirm.step1", "confirm.step2", "confirm.step3"][i as usize];
    view()
        .size(px(touch_target()))
        .items_center()
        .justify_center()
        .role(AccessibilityRole::Button)
        .label(format!("{}: {}", i + 1, tr(caption_key, &[])))
        .on_press(move || {
            index.set(i);
            track.scroll_to_offset(Points(i as f32 * GUIDE_STRIDE), !reduce_motion());
        })
        .child(view().size(px(7.0)).rounded_full().bg(move || {
            if index.get() == i {
                token("tint")
            } else {
                rgb(0xC7C7CC).into()
            }
        }))
}

fn step_slide(asset: ImageAsset, number: u8, caption: String) -> impl IntoElement {
    view()
        .w(px(GUIDE_WIDTH))
        .h(px(GUIDE_HEIGHT))
        .overflow_hidden()
        .flex_shrink_0()
        .child(
            view()
                .gap(px(10.0))
                .child(
                    view().p(px(6.0)).rounded(px(10.0)).bg(rgb(0x111113)).child(
                        image(asset)
                            .w_full()
                            .aspect_ratio(3.0 / 2.0)
                            .rounded(px(16.0))
                            .object_fit(ResizeMode::Cover),
                    ),
                )
                .child(
                    view()
                        .flex_row()
                        .h(px(44.0))
                        .items_center()
                        .gap(px(10.0))
                        .px_1()
                        .child(
                            view()
                                .size(px(24.0))
                                .rounded_full()
                                .items_center()
                                .justify_center()
                                .bg(move || token("tint"))
                                .child(
                                    text(format!("{number}"))
                                        .text_size(px(13.0))
                                        .line_height(px(18.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(move || theme().background),
                                ),
                        )
                        .child(
                            text(caption)
                                .flex_1()
                                .line_height(px(22.0))
                                .number_of_lines(2u32)
                                .text_size(px(17.0))
                                .font_weight(FontWeight::MEDIUM),
                        ),
                ),
        )
}

fn restart(app: AppState) -> impl IntoElement {
    sheet_shell(
        tr("restart.title", &[]),
        Some(tr("restart.subtitle", &[])),
        Some(app.nearby_model().unwrap_or(CameraModel::Gr3)),
        false,
        || {},
        view()
            .w_full()
            .gap_4()
            .items_center()
            .child(steps_box(2))
            .child(
                view()
                    .w_full()
                    .gap_1()
                    .child(sheet_primary_button(tr("restart.done", &[]), {
                        let app = app.clone();
                        move || app.finish_restart()
                    }))
                    .child(secondary_button(tr("restart.cancel", &[]), move || {
                        app.cancel_update()
                    })),
            ),
    )
}

fn installed(app: AppState, version: String, _previous: String) -> impl IntoElement {
    sheet_shell(
        tr("installed.title", &[("version", &version)]),
        Some(tr("installed.verified", &[])),
        None,
        false,
        || {},
        view()
            .w_full()
            .gap_5()
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .justify_center()
                    .gap_4()
                    .py_3()
                    .child(camera_art(
                        app.nearby_model().unwrap_or(CameraModel::Gr3),
                        120.0,
                        false,
                    ))
                    .child(
                        view()
                            .size(px(32.0))
                            .rounded_full()
                            .items_center()
                            .justify_center()
                            .bg(move || token("success-soft"))
                            .text_color(move || token("success"))
                            .child(connection_icon("M5 12l4 4L19 6")),
                    ),
            )
            .child(sheet_primary_button(tr("installed.done", &[]), move || {
                app.dismiss_installed()
            })),
    )
}

fn error_sheet(app: AppState, error: UpdateError) -> impl IntoElement {
    let (title, body, retry_label, retry) = match &error {
        UpdateError::Service(message) => (
            tr("error.service_title", &[]),
            message.clone(),
            tr("error.try_again", &[]),
            true,
        ),
        UpdateError::Download(message) => (
            tr("error.download_title", &[]),
            message.clone(),
            tr("error.try_again", &[]),
            true,
        ),
        UpdateError::CardFull { bytes_needed } => (
            tr("error.card_full_title", &[]),
            tr(
                "error.card_full_body",
                &[("size", &format_bytes(*bytes_needed))],
            ),
            tr("error.try_again", &[]),
            true,
        ),
        UpdateError::NotConfirmed => (
            tr("error.not_confirmed_title", &[]),
            tr("error.not_confirmed_body", &[]),
            tr("error.send_again", &[]),
            true,
        ),
        UpdateError::Refused => (
            tr("error.refused_title", &[]),
            tr("error.refused_body", &[]),
            tr("error.try_again", &[]),
            true,
        ),
        UpdateError::ConnectionLost => (
            tr("error.lost_title", &[]),
            tr("error.lost_body", &[]),
            tr("error.try_again", &[]),
            false,
        ),
    };
    sheet_shell(
        title,
        Some(body),
        None,
        false,
        || {},
        view()
            .w_full()
            .gap_1()
            .child(retry.then(|| {
                sheet_primary_button(retry_label, {
                    let app = app.clone();
                    move || app.retry_update()
                })
            }))
            .child((!app.demo() && app.has_pending_update()).then(|| {
                secondary_button(tr("error.cancel_pending", &[]), move || app.cancel_update())
            }))
            .child(secondary_button(tr("error.close", &[]), move || {
                app.dismiss_error()
            })),
    )
}

fn steps_box(current: u8) -> impl IntoElement {
    view()
        .w_full()
        .rounded(px(10.0))
        .bg(move || theme().muted)
        .child(checklist_row(
            &tr("sending.step_send", &[]),
            None,
            step_state(current, 0),
        ))
        .child(checklist_row(
            &tr("sending.step_confirm", &[]),
            None,
            step_state(current, 1),
        ))
        .child(checklist_row(
            &tr("sending.step_restart", &[]),
            None,
            step_state(current, 2),
        ))
}

fn step_state(current: u8, index: u8) -> StepState {
    if current > index {
        StepState::Done
    } else if current == index {
        StepState::Current
    } else {
        StepState::Waiting
    }
}
