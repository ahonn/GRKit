//! Modal sheets for pairing, connecting, update progress, and errors.

use nsh_base::{SheetEvent, progress, sheet, theme};
use nsh_core::prelude::*;

use crate::ui::ty::text;
use std::time::Duration;

use crate::domain::{CameraModel, LinkError, UpdateError, UpdatePhase, update_step};
use crate::store::{AppState, Sheet, link_error_body, link_error_title};
use crate::strings::{format_bytes, tr};
use crate::ui::{
    NATIVE_SHEET, StepState, activity_indicator, camera_art, connection_button, connection_icon,
    control_border, failed_icon, secondary_button, sheet_primary_button, sheet_surface, token,
    touch_target,
};

/// Every sheet closes with a swipe down or a press on the backdrop, as in
/// the camera app. A restart under way stays.
fn dismissible(app: AppState) -> bool {
    app.sheet().is_some() && !app.update_restarting()
}

/// An update under way asks before a swipe stops it, so the sheet only
/// reports the swipe.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
fn swipe_closes(app: AppState) -> bool {
    dismissible(app)
        && !matches!(
            app.sheet(),
            Some(Sheet::Sending | Sheet::Confirm | Sheet::Restart)
        )
}

/// The user swiped a sheet away: what its own way out does (close, Not
/// Now, Cancel, Done).
fn dismiss(app: AppState) {
    match app.sheet() {
        Some(Sheet::Nearby) => app.dismiss_nearby(),
        Some(Sheet::Scan | Sheet::ImportLook) => app.set_sheet(None),
        Some(Sheet::Connecting { .. }) => app.cancel_connect(),
        Some(Sheet::ConnectError(_)) => app.dismiss_connect_error(),
        // An update under way asks first; a restart cannot be stopped.
        Some(Sheet::Sending | Sheet::Confirm | Sheet::Restart) => {
            if !app.update_restarting() {
                app.ask_cancel_update()
            }
        }
        Some(Sheet::Installed { .. }) => app.dismiss_installed(),
        Some(Sheet::Error(_)) => app.dismiss_error(),
        Some(Sheet::InstallFailed) => app.dismiss_install_failed(),
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
        Some(Sheet::Sending | Sheet::Confirm | Sheet::Restart) => {
            update_sheet(app, None, None).into_element()
        }
        Some(Sheet::Installed { version, .. }) => {
            update_sheet(app, None, Some(version)).into_element()
        }
        Some(Sheet::Error(error)) => update_sheet(app, Some(error), None).into_element(),
        Some(Sheet::InstallFailed) => install_failed(app).into_element(),
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
        Some(Sheet::Nearby) => (None, close(Box::new(move || app.dismiss_nearby()))),
        Some(Sheet::Installed { .. }) => (None, close(Box::new(move || app.dismiss_installed()))),
        Some(Sheet::Scan) => {
            let back: Option<Box<dyn Fn()>> = scan_shows_card(app)
                .then(|| Box::new(move || app.pin_scan_list()) as Box<dyn Fn()>);
            (back, close(Box::new(move || app.set_sheet(None))))
        }
        Some(Sheet::ImportLook) => (None, close(Box::new(move || app.set_sheet(None)))),
        Some(Sheet::ConnectError(_)) => {
            (None, close(Box::new(move || app.dismiss_connect_error())))
        }
        Some(Sheet::Error(_)) => (None, close(Box::new(move || app.dismiss_error()))),
        Some(Sheet::InstallFailed) => (None, close(Box::new(move || app.dismiss_install_failed()))),
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
                    // The system keeps a tall sheet on the screen; its body scrolls.
                    .detents(move || vec![SheetDetent::Height(height.get())])
                    .grabber(move || dismissible(app))
                    .close_on_swipe(move || swipe_closes(app))
                    .close_on_backdrop_press(move || swipe_closes(app))
                    .close_on_escape(move || dismissible(app))
                    .on_cancel(move || dismiss(app))
                    .child(
                        nav_stack()
                            .flex_1()
                            .bg(sheet_surface)
                            .child(dynamic(move || {
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
                                        scroll().flex_1().bg(sheet_surface).bounces(false).child(
                                            view()
                                                .flex_shrink_0()
                                                .on_layout(move |b: &Bounds<Points>| {
                                                    if b.size.height.0 > 0.0 {
                                                        height.set(b.size.height);
                                                    }
                                                })
                                                .child(dynamic(move || sheet_body(app))),
                                        ),
                                    )
                                    .into_element()
                                })
                            })),
                    )
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
        .child(
            // Large text can make a sheet taller than the screen.
            scroll()
                .max_h(move || px(window().height.0 * MAX_SHEET_HEIGHT))
                .bounces(false)
                .child(
                    view()
                        .flex_shrink_0()
                        .child(dynamic(move || sheet_body(app))),
                ),
        )
}

/// The share of the screen a drawn sheet may take before it scrolls.
#[cfg(not(target_os = "ios"))]
const MAX_SHEET_HEIGHT: f32 = 0.92;

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
                        .text_size(px(crate::ui::ty::TITLE2))
                        .font_weight(FontWeight::BOLD)
                        .when(has_back, |t| t.text_center()),
                )
                .child(subtitle.map(|s| {
                    text(s)
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
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
            .border_color(control_border)
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
                .bg(move || theme().foreground)
                .text_color(move || theme().background)
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

/// The title of a step: secondary text once it is not the one under way,
/// at a contrast that stays readable.
fn step_color(state: StepState) -> AdaptiveColor {
    match state {
        StepState::Current | StepState::Failed => theme().foreground,
        StepState::Waiting | StepState::Done => theme().muted_foreground,
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

fn nearby(app: AppState) -> impl IntoElement {
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
                                    .flex_shrink(1.0)
                                    .text_size(px(crate::ui::ty::SUBHEADLINE))
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
                    .gap_1()
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

/// The update sheet: one sheet whose steps advance in place, from the
/// download to the restart, and that turns into the error that stopped it.
fn update_sheet(
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
        .or_else(|| app.package().and_then(|p| p.available))
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
            let extra = detailed.then(|| step_detail(app, &phase, &version));
            update_step_row(&titles[index], state, index + 1 == shown, extra).into_element()
        }))
    });
    connection_shell(
        title,
        Some(subtitle),
        failed || done,
        move || {
            if done {
                app.dismiss_installed()
            } else {
                app.dismiss_error()
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

fn update_error_copy(error: &UpdateError) -> (String, String) {
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
        UpdateError::NotConfirmed => (
            tr("error.not_confirmed_title", &[]),
            tr("error.not_confirmed_body", &[]),
        ),
        UpdateError::Refused => (
            tr("error.refused_title", &[]),
            tr("error.refused_body", &[]),
        ),
        UpdateError::ConnectionLost => (tr("error.lost_title", &[]), tr("error.lost_body", &[])),
    }
}

/// What the step under way shows below its name.
fn step_detail(app: AppState, phase: &UpdatePhase, version: &str) -> AnyElement {
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
        UpdatePhase::WaitingConfirm { .. } => confirm_detail(app, version).into_element(),
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

/// The confirm step: the steps on the camera and the time left.
fn confirm_detail(app: AppState, version: &str) -> impl IntoElement {
    let value = tr("update.menu_value", &[("version", version)]);
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
                    .bg(move || theme().foreground)
                    .child(
                        text(n.to_string())
                            .text_size(px(crate::ui::ty::CAPTION))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(move || theme().background),
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
        .child(
            text(move || {
                tr(
                    "confirm.waiting",
                    &[("time", &app.confirm_deadline_label())],
                )
            })
            .text_size(px(crate::ui::ty::SUBHEADLINE))
            .line_height(px(20.0))
            .text_color(move || theme().muted_foreground),
        )
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

fn update_buttons(
    app: AppState,
    phase: &UpdatePhase,
    error: Option<UpdateError>,
    installed: bool,
) -> AnyElement {
    let buttons = view().w_full().gap_1();
    if app.update_cancelling() && !installed {
        return buttons.child(cancelling_row()).into_element();
    }
    if installed {
        return buttons
            .child(sheet_primary_button(tr("installed.done", &[]), move || {
                app.dismiss_installed()
            }))
            .into_element();
    }
    if let Some(error) = error {
        let (primary, key): (Option<Box<dyn Fn()>>, &str) = match error {
            UpdateError::Refused | UpdateError::ModuleNotKept(_) | UpdateError::PackageInvalid => {
                (None, "")
            }
            UpdateError::ConnectionLost => (
                Some(Box::new(move || app.retry_update())),
                "update.reconnect",
            ),
            UpdateError::NotConfirmed => (
                Some(Box::new(move || app.retry_update())),
                "error.send_again",
            ),
            _ => (
                Some(Box::new(move || app.retry_update())),
                "error.try_again",
            ),
        };
        return buttons
            .child(match primary {
                Some(action) => sheet_primary_button(tr(key, &[]), action).into_element(),
                None => sheet_primary_button(tr("update.ok", &[]), move || app.dismiss_error())
                    .into_element(),
            })
            .child((!app.demo() && app.has_pending_update()).then(|| {
                secondary_button(tr("error.cancel_pending", &[]), move || app.cancel_update())
            }))
            .child(
                primary_is_some(&error)
                    .then(|| secondary_button(tr("error.close", &[]), move || app.dismiss_error())),
            )
            .into_element();
    }
    match phase {
        UpdatePhase::Confirmed { .. } | UpdatePhase::Restart { .. } => buttons
            .child(sheet_primary_button(tr("restart.run", &[]), move || {
                app.restart_for_update()
            }))
            .child(secondary_button(tr("update.cancel", &[]), move || {
                app.ask_cancel_update()
            }))
            .into_element(),
        UpdatePhase::ManualRestart { .. } => buttons
            .child(sheet_primary_button(
                tr("update.reconnect", &[]),
                move || app.reconnect_for_update(),
            ))
            .child(secondary_button(tr("update.cancel", &[]), move || {
                app.ask_cancel_update()
            }))
            .into_element(),
        UpdatePhase::Restarting { .. } => buttons.into_element(),
        _ => buttons
            .child(secondary_button(tr("update.cancel", &[]), move || {
                app.ask_cancel_update()
            }))
            .into_element(),
    }
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
        UpdateError::Refused | UpdateError::ModuleNotKept(_) | UpdateError::PackageInvalid
    )
}

/// A confirmed update did not install: the camera runs without GRKit.
fn install_failed(app: AppState) -> impl IntoElement {
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
                    .child(secondary_button(tr("update.open_site", &[]), || {
                        spawn(async {
                            let _ = open_url(crate::store::INSTALL_URL).await;
                        });
                    })),
            ),
    )
}
