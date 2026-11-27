//! What the connect and update sheets share: the shell with its title and
//! buttons, the step indicator, and the camera art.

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::domain::CameraModel;
use crate::strings::tr;
use crate::ui::ty::text;
use crate::ui::{
    NATIVE_SHEET, StepState, activity_indicator, camera_art, connection_icon, control_border,
    failed_icon, token, touch_target,
};

/// Connection sheets follow the camera discovery layout without changing update sheets.
pub(super) fn connection_shell(
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
pub(super) fn connection_shell_with_back(
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
        // The native sheet's height leaves out the bottom safe area, so the
        // body's own padding is the gap under its last button, as at its top.
        .when(NATIVE_SHEET, |v| v.pb(px(24.0)))
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
pub(super) const STEP_INDICATOR: f32 = 22.0;

const CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5.5 12.5l4.5 4.5L18.5 7.5"/></svg>"#;

pub(super) fn step_indicator(state: StepState, changed: bool) -> AnyElement {
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
                // A step behind: gray, as its title. The tint marks only
                // the step under way and what the person has to do.
                .bg(move || theme().muted_foreground)
                .text_color(rgb(0xFFFFFF))
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
pub(super) fn step_color(state: StepState) -> AdaptiveColor {
    match state {
        StepState::Current | StepState::Failed => theme().foreground,
        StepState::Waiting | StepState::Done => theme().muted_foreground,
    }
}

/// The camera art on a quiet panel.
pub(super) fn art_box(model: CameraModel) -> impl IntoElement {
    view()
        .w_full()
        .rounded(px(8.0))
        .bg(move || token("connection-muted"))
        .py_5()
        .items_center()
        .child(camera_art(model, 150.0, false))
}
