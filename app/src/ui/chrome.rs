//! Inset grouped lists and shared controls.

use std::time::Duration;

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::ui::ty::text;

use grkit_core::cover::Cover;

use crate::domain::CameraModel;
use crate::strings::tr;

const ART_GR3: ImageAsset = asset!("art-gr3.png");
const ART_GR3X: ImageAsset = asset!("art-gr3x.png");
const ART_GR4: ImageAsset = asset!("art-gr4.png");
const GR_SNAP: ImageAsset = asset!("gr-snap.png");

/// The platform's own loader, which each platform registers as a native
/// view: Android's indeterminate `ProgressBar` (`MainActivity`), iOS's
/// `UIActivityIndicatorView` (`platform::ios::activity`).
pub const ACTIVITY_COMPONENT: &str = "grkit/activity";

/// The platform's loader in the tint color, `size` points square. Where no
/// platform view exists (desktop runs, headless tests) a turning arc stands
/// in for it.
pub fn activity_indicator(size: f32) -> AnyElement {
    tinted_indicator(size, token("tint"))
}

/// The platform's loader in `tint`, for a surface the tint color does not
/// read on.
pub fn tinted_indicator(size: f32, tint: AdaptiveColor) -> AnyElement {
    if cfg!(any(target_os = "ios", target_os = "android")) {
        native(ACTIVITY_COMPONENT)
            .size(px(size))
            .prop("light", serde_json::json!(tint.light.to_hex()))
            .prop("dark", serde_json::json!(tint.dark.to_hex()))
            .into_element()
    } else {
        view()
            .size(px(size))
            .text_color(move || tint)
            .child(
                svg(nsh_core::element::ImageSrc::svg_markup(
                    r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round"><path d="M12 3a9 9 0 1 1-9 9"/></svg>"#,
                ))
                .size(px(size))
                .animate_spin(),
            )
            .into_element()
    }
}

/// How long a wait lasts before its spinner shows, so that a fast change
/// does not flash one.
const SPINNER_DELAY: Duration = Duration::from_millis(180);

/// A signal that turns true once the wait has lasted [`SPINNER_DELAY`].
fn after_spinner_delay() -> Signal<bool> {
    let shown = signal(false);
    spawn(async move {
        timer(SPINNER_DELAY).await;
        shown.set(true);
    });
    shown
}

/// A loader that holds its place and shows after [`SPINNER_DELAY`].
pub fn quiet_spinner(size: f32) -> AnyElement {
    let shown = after_spinner_delay();
    view()
        .size(px(size))
        .child(dynamic(move || {
            shown.get().then(|| activity_indicator(size))
        }))
        .into_element()
}

/// `content` in its place, with a spinner over it while `loading`. The
/// content keeps the size of the control and stays until the spinner shows.
pub fn loading_content(
    loading: bool,
    size: f32,
    tint: AdaptiveColor,
    content: impl IntoElement,
) -> AnyElement {
    if !loading {
        return content.into_element();
    }
    let shown = after_spinner_delay();
    view()
        .relative()
        .items_center()
        .justify_center()
        .child(
            view()
                .opacity(move || if shown.get() { 0.0_f32 } else { 1.0_f32 })
                .child(content),
        )
        .child(dynamic(move || {
            shown.get().then(|| {
                view()
                    .absolute()
                    .inset_0()
                    .items_center()
                    .justify_center()
                    .child(tinted_indicator(size, tint))
            })
        }))
        .into_element()
}

/// The boundary of a control that shows no fill of its own, such as an
/// empty circle or a text field: 3:1 or better on every surface of both
/// themes. Decorative separators keep `theme().border`.
pub fn control_border() -> AdaptiveColor {
    theme().muted_foreground
}

/// The press area that brings a control drawn `height` points tall to the
/// platform's touch target.
pub fn vertical_hit_slop(height: f32) -> EdgeInsets {
    let extra = Points(((touch_target() - height) / 2.0).max(0.0));
    EdgeInsets {
        top: extra,
        right: Points(0.0),
        bottom: extra,
        left: Points(0.0),
    }
}

/// The accessibility state of a control that is working.
pub fn busy_state(loading: bool) -> impl Fn() -> AccessibilityState + 'static {
    move || AccessibilityState {
        busy: loading,
        disabled: loading,
        ..AccessibilityState::default()
    }
}

/// Minimum touch target for the current platform.
pub fn touch_target() -> f32 {
    if cfg!(target_os = "android") {
        48.0
    } else {
        44.0
    }
}

fn action_radius() -> f32 {
    if cfg!(target_os = "android") {
        25.0
    } else {
        8.0
    }
}

pub fn gr_snap_icon() -> ImageAsset {
    GR_SNAP
}

pub fn token(name: &'static str) -> AdaptiveColor {
    theme().color(name).unwrap_or_else(|| theme().background)
}

/// The menu of a held row. On iOS it is a centered alert, since an action
/// sheet there opens at the edge of the screen; Android lists its actions in
/// a centered dialog, which an alert with more than three actions cannot do.
pub fn held_menu(title: impl Into<SharedString>) -> nsh_base::AlertDialogOptions {
    let menu = nsh_base::alert_dialog().title(title);
    if cfg!(target_os = "ios") {
        menu
    } else {
        menu.action_sheet()
    }
}

/// A page of the navigation stacks. Android's bar otherwise takes its colors
/// from the activity's light theme, whatever the appearance.
pub fn app_page() -> nsh_core::element::Screen {
    let page = nsh_base::nav_page();
    if cfg!(target_os = "android") {
        page.header_style_with(|s| {
            s.bg(theme().background)
                .text_color(theme().foreground)
                .tint_color(theme().foreground)
        })
    } else {
        page
    }
}

/// Floating sheet surface, with an opaque fill in light mode.
pub fn sheet_surface() -> Rgba {
    if dark() {
        rgba(0x242424FF)
    } else {
        rgba(0xFFFFFFFF)
    }
}

/// Line art for a model, tinted with the text color.
pub fn camera_art(model: CameraModel, width: f32, faded: bool) -> impl IntoElement {
    let asset = match model {
        CameraModel::Gr3 => ART_GR3,
        CameraModel::Gr3x => ART_GR3X,
        CameraModel::Gr4 => ART_GR4,
    };
    let opacity = if faded {
        move || if dark() { 0.22_f32 } else { 0.32_f32 }
    } else {
        move || if dark() { 0.85_f32 } else { 1.0_f32 }
    };
    image(asset)
        .w(px(width))
        .object_fit(ResizeMode::Contain)
        .tint_color(move || theme().foreground)
        .opacity(opacity)
}

pub fn primary_button(title: impl Into<String>, on_press: impl Fn() + 'static) -> impl IntoElement {
    primary_button_loading(title, false, on_press)
}

/// [`primary_button`] with a spinner in place of its title while `loading`.
pub fn primary_button_loading(
    title: impl Into<String>,
    loading: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let title = title.into();
    view()
        .min_h(px(50.0))
        .py_3()
        .px_8()
        .rounded(px(action_radius()))
        .items_center()
        .justify_center()
        .bg(move || token("button"))
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .accessibility_state(busy_state(loading))
        .when(!loading, |v| {
            v.on_press(on_press).active(|s| s.opacity(0.72))
        })
        .child(loading_content(
            loading,
            20.0,
            token("button-foreground"),
            text(title)
                .text_size(px(crate::ui::ty::BUTTON))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || token("button-foreground")),
        ))
}

/// Full-width primary action inside a sheet.
pub fn sheet_primary_button(
    title: impl Into<String>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    sheet_primary_button_loading(title, false, on_press)
}

/// [`sheet_primary_button`] with a spinner in place of its title while
/// `loading`.
pub fn sheet_primary_button_loading(
    title: impl Into<String>,
    loading: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let title = title.into();
    view()
        .w_full()
        .min_h(px(50.0))
        .py_3()
        .rounded(px(action_radius()))
        .items_center()
        .justify_center()
        .bg(move || token("button"))
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .accessibility_state(busy_state(loading))
        .when(!loading, |v| {
            v.on_press(on_press).active(|s| s.opacity(0.72))
        })
        .child(loading_content(
            loading,
            20.0,
            token("button-foreground"),
            text(title)
                .text_size(px(crate::ui::ty::BUTTON))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || token("button-foreground")),
        ))
}

pub fn secondary_button(
    title: impl Into<String>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    secondary_button_loading(title, false, on_press)
}

/// [`secondary_button`] with a spinner in place of its title while
/// `loading`.
pub fn secondary_button_loading(
    title: impl Into<String>,
    loading: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let title = title.into();
    view()
        .w_full()
        .min_h(px(touch_target()))
        .items_center()
        .justify_center()
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .accessibility_state(busy_state(loading))
        .when(!loading, |v| {
            v.on_press(on_press).active(|s| s.opacity(0.72))
        })
        .child(loading_content(
            loading,
            18.0,
            token("tint"),
            text(title)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .line_height(px(22.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || token("link")),
        ))
}

pub fn close_chip(on_press: impl Fn() + 'static) -> impl IntoElement {
    view()
        .size(px(touch_target()))
        .rounded_full()
        .items_center()
        .justify_center()
        .bg(move || theme().muted)
        .role(AccessibilityRole::Button)
        .label(tr("scan.close", &[]))
        .on_press(on_press)
        .active(|s| s.opacity(0.72))
        .child(
            text("✕")
                .text_size(px(crate::ui::ty::BODY))
                .text_color(move || theme().muted_foreground),
        )
}

pub fn section_title(title: impl Into<String>) -> impl IntoElement {
    text(title.into())
        .text_size(px(crate::ui::ty::SUBHEADLINE))
        .line_height(px(22.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(move || theme().muted_foreground)
}

pub fn grouped_card(child: impl IntoElement) -> impl IntoElement {
    view()
        .rounded(px(10.0))
        .border_1()
        .border_color(move || theme().border)
        .overflow_hidden()
        .bg(move || theme().card)
        .child(child)
}

pub fn info_row(
    title: impl Into<String>,
    value: impl Into<String>,
    last: bool,
) -> impl IntoElement {
    let title = title.into();
    let value = value.into();
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .min_h(px(50.0))
        .px_4()
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .child(
            text(title)
                .flex_shrink(1.0)
                .text_size(px(crate::ui::ty::BODY)),
        )
        .child(
            text(value)
                .flex_shrink(1.0)
                .text_size(px(crate::ui::ty::BODY))
                .text_color(move || theme().muted_foreground),
        )
}

pub fn chevron_row(
    title: impl Into<String>,
    subtitle: Option<String>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let title = title.into();
    let label = match &subtitle {
        Some(s) => format!("{title}, {s}"),
        None => title.clone(),
    };
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .min_h(px(68.0))
        .py_3()
        .px_4()
        .role(AccessibilityRole::Button)
        .label(label)
        .on_press(on_press)
        .active(|s| s.opacity(0.72).bg(move || theme().accent))
        .child(
            view()
                .flex_1()
                .gap(px(4.0))
                .child(
                    text(title.clone())
                        .text_size(px(crate::ui::ty::HEADLINE))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(subtitle.map(|s| {
                    text(s)
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                        .line_height(px(22.0))
                        .text_color(move || theme().muted_foreground)
                })),
        )
        .child(crate::ui::chevron())
}

#[derive(Clone, Copy)]
pub enum StepState {
    Done,
    Current,
    Waiting,
    /// The step an error stopped at.
    Failed,
}

/// iOS presents the sheets natively: the system draws the grabber, the
/// bar buttons and the bottom safe area.
pub const NATIVE_SHEET: bool = cfg!(target_os = "ios");

pub fn sheet_shell(
    title: impl Into<String>,
    subtitle: Option<String>,
    hero: Option<CameraModel>,
    show_close: bool,
    on_close: impl Fn() + 'static,
    body: impl IntoElement,
) -> impl IntoElement {
    let title = title.into();
    view()
        .relative()
        .flex_col()
        .w_full()
        .gap_5()
        .pt(px(if NATIVE_SHEET { 24.0 } else { 8.0 }))
        .px(px(24.0))
        // A native sheet adds the bottom safe area to its height itself.
        .when(NATIVE_SHEET, |v| v.pb(px(16.0)))
        .when(!NATIVE_SHEET, |v| v.pb_safe_area_plus(px(8.0)))
        .child((!NATIVE_SHEET).then(|| {
            view()
                .self_center()
                .w(px(36.0))
                .h(px(5.0))
                .rounded_full()
                .bg(rgb(0xD1D1D6))
        }))
        .child(hero.map(|model| {
            view()
                .items_center()
                .pt_4()
                .child(camera_art(model, 140.0, false))
        }))
        .child(
            view()
                .gap(px(6.0))
                .pt_2()
                .child(
                    text(title)
                        .text_size(px(crate::ui::ty::TITLE2))
                        .font_weight(FontWeight::BOLD),
                )
                .child(subtitle.map(|s| {
                    text(s)
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                        .line_height(px(22.0))
                        .text_color(move || theme().muted_foreground)
                })),
        )
        .child((show_close && !NATIVE_SHEET).then(|| {
            view()
                .absolute()
                .right(px(14.0))
                .top(px(22.0))
                .child(close_chip(on_close))
        }))
        .child(body)
}

/// The GRKit tile: GR and Kit in its red box on black.
pub fn gr_mark(size: f32) -> impl IntoElement {
    view()
        .size(px(size))
        .rounded(px(size * 0.23))
        .flex_row()
        .items_center()
        .justify_center()
        .bg(rgb(0x0B0B0B))
        .child(
            text("GR")
                .allow_font_scaling(false)
                .text_size(px(size * 0.29))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(0xFFFFFF)),
        )
        .child(
            view().ml(px(2.0)).px(px(3.0)).bg(rgb(0xCF142B)).child(
                text("Kit")
                    .allow_font_scaling(false)
                    .text_size(px(size * 0.21))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xFFFFFF)),
            ),
        )
}

/// The GRKit wordmark: GR and Kit in a red box. `height` sets its size.
pub fn wordmark(height: f32) -> impl IntoElement {
    const LIGHT: &str = include_str!("../../../resources/brand/logo/wordmark.svg");
    const DARK: &str = include_str!("../../../resources/brand/logo/wordmark-inverse.svg");
    // The proportions of the wordmark's view box (2663 × 1018).
    let width = height * 2663.0 / 1018.0;
    view()
        .role(AccessibilityRole::Header)
        .label("GRKit")
        .child(dynamic(move || {
            svg(nsh_core::element::ImageSrc::svg_markup(if dark() {
                DARK
            } else {
                LIGHT
            }))
            .w(px(width))
            .h(px(height))
        }))
}

/// The mark of a step an error stopped at: a white cross on amber.
pub fn failed_icon(size: f32) -> impl IntoElement {
    view()
        .size(px(size))
        .rounded_full()
        .items_center()
        .justify_center()
        .bg(rgb(0xB45309))
        .child(
            svg(nsh_core::element::ImageSrc::svg_markup(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="#FFFFFF" stroke-width="2.4" stroke-linecap="round"><path d="M2 2l8 8M10 2l-8 8"/></svg>"##,
            ))
            .size(px(size * 0.45)),
        )
}

/// Controls scoped to the connection flow.
pub fn connection_button(title: String, on_press: impl Fn() + 'static) -> impl IntoElement {
    view()
        .w_full()
        .min_h(px(50.0))
        .py_3()
        .rounded(px(action_radius()))
        .items_center()
        .justify_center()
        .bg(move || token("connection-accent"))
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .on_press(on_press)
        .active(|s| s.opacity(0.72))
        .child(
            text(title)
                .text_size(px(crate::ui::ty::BUTTON))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || token("connection-accent-foreground")),
        )
}

pub fn connection_icon(path: &'static str) -> impl IntoElement {
    svg(nsh_core::element::ImageSrc::svg_markup(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="{path}"/></svg>"#
    ))).size(px(20.0))
}

/// Round button with a stroked icon, for page actions such as Add.
pub fn circle_button(
    path: &'static str,
    label: String,
    enabled: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .size(px(touch_target()))
        .rounded_full()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(control_border)
        .bg(move || theme().card)
        .role(AccessibilityRole::Button)
        .label(label)
        .opacity(if enabled { 1.0 } else { 0.4 })
        .when(enabled, |v| {
            v.on_press(on_press).active(|s| s.opacity(0.72))
        })
        .child(connection_icon(path))
}

/// One segment: its title, whether it is selected, and its action.
pub type Segment = (String, bool, Box<dyn Fn()>);

/// Segmented control: equal segments, the selected one raised.
pub fn segmented(items: Vec<Segment>) -> impl IntoElement {
    let mut row = view()
        .flex_row()
        .min_h(px(36.0))
        .p(px(2.0))
        .rounded(px(9.0))
        .bg(move || theme().muted);
    for (title, selected, on_press) in items {
        row = row.child(
            view()
                .flex_1()
                .items_center()
                .justify_center()
                .rounded(px(7.0))
                .py_1()
                .hit_slop(vertical_hit_slop(32.0))
                .role(AccessibilityRole::Tab)
                .label(title.clone())
                .accessibility_state(move || AccessibilityState {
                    selected,
                    ..AccessibilityState::default()
                })
                .when(selected, |v| {
                    v.bg(move || theme().card)
                        .border_1()
                        .border_color(control_border)
                        .shadow([BoxShadow {
                            offset_x: Points(0.0),
                            offset_y: Points(1.0),
                            blur: Points(3.0),
                            spread: Points(0.0),
                            color: rgba(0x00000024).into(),
                            inset: false,
                        }])
                })
                .on_press(on_press)
                .child(
                    text(title)
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                        .font_weight(if selected {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_color(move || theme().foreground)
                        .text_center(),
                ),
        );
    }
    row
}

/// On/off switch in the brand tint.
///
/// A spinner shows in its thumb while `loading`.
pub fn toggle_loading(
    on: bool,
    label: String,
    enabled: bool,
    loading: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let enabled = enabled && !loading;
    let spinner_tint: AdaptiveColor = if on {
        token("button")
    } else {
        theme().background
    };
    view()
        .w(px(51.0))
        .h(px(31.0))
        .p(px(0.5))
        .rounded_full()
        .border(px(1.5))
        .border_color(move || {
            if on {
                token("button")
            } else {
                control_border()
            }
        })
        .hit_slop(vertical_hit_slop(31.0))
        .flex_row()
        .justify_start()
        .when(on, |v| v.justify_end())
        .bg(move || if on { token("button") } else { theme().card })
        .opacity(if enabled || loading { 1.0 } else { 0.4 })
        .role(AccessibilityRole::Switch)
        .label(label)
        .accessibility_state(move || AccessibilityState {
            checked: Some(if on {
                nsh_core::Checked::True
            } else {
                nsh_core::Checked::False
            }),
            disabled: !enabled,
            busy: loading,
            ..AccessibilityState::default()
        })
        .when(enabled, |v| v.on_press(on_press))
        .child(
            view()
                .size(px(27.0))
                .rounded_full()
                .items_center()
                .justify_center()
                // Off: the color of the outline, which reads on the empty
                // track in both themes.
                .bg(move || {
                    if on {
                        token("button-foreground")
                    } else {
                        control_border()
                    }
                })
                .shadow([BoxShadow {
                    offset_x: Points(0.0),
                    offset_y: Points(2.0),
                    blur: Points(4.0),
                    spread: Points(0.0),
                    color: rgba(0x00000026).into(),
                    inset: false,
                }])
                .child(
                    loading
                        .then(|| loading_content(true, 16.0, spinner_tint, view().size(px(16.0)))),
                ),
        )
}

/// Small gray chip, such as In Use.
pub fn badge(title: String) -> impl IntoElement {
    view()
        .min_h(px(22.0))
        .px_2()
        .py(px(2.0))
        .rounded_full()
        .items_center()
        .justify_center()
        .bg(move || theme().muted)
        .child(
            text(title)
                .text_size(px(crate::ui::ty::CAPTION))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        )
}

/// A Look cover, `size` points square, drawn from its SVG at the screen's
/// pixels.
pub fn cover_view(cover: &Cover, size: f32, faded: bool) -> AnyElement {
    image(nsh_core::element::ImageSrc::svg_markup(&cover.svg()))
        .w(px(size))
        .h(px(size))
        .object_fit(ResizeMode::Contain)
        .opacity(if faded { 0.45 } else { 1.0 })
        .into_element()
}

/// A row with a title and a trailing control or value.
pub fn control_row(
    title: impl Into<String>,
    trailing: impl IntoElement,
    last: bool,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .min_h(px(50.0))
        .px_4()
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .child(
            text(title.into())
                .text_size(px(crate::ui::ty::BODY))
                .flex_shrink(1.0),
        )
        .child(trailing)
}

/// A pressable row whose title is the action, in `color`.
pub fn action_row(
    title: impl Into<String>,
    destructive: bool,
    last: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    action_row_loading(title, destructive, last, false, on_press)
}

/// [`action_row`] with a spinner at its end while `loading`.
pub fn action_row_loading(
    title: impl Into<String>,
    destructive: bool,
    last: bool,
    loading: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let title = title.into();
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .min_h(px(50.0))
        .px_4()
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .accessibility_state(busy_state(loading))
        .when(!loading, |v| {
            v.on_press(on_press).active(|s| s.opacity(0.72))
        })
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .child(
            text(title)
                .flex_shrink(1.0)
                .text_size(px(crate::ui::ty::BODY))
                .text_color(move || {
                    if destructive {
                        theme().destructive
                    } else {
                        token("link")
                    }
                }),
        )
        .child(loading.then(|| quiet_spinner(18.0)))
}

/// The chevron at the end of a row that opens a page.
pub fn chevron() -> impl IntoElement {
    image(nsh_core::element::ImageSrc::svg_markup(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="14" viewBox="0 0 8 14" fill="none" stroke="#000000" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 1l6 6-6 6"/></svg>"##,
    ))
    .w(px(8.0))
    .h(px(14.0))
    .tint_color(move || theme().muted_foreground)
}

/// A small pill button in the muted fill, such as Apply on a row. The
/// pill is 32 points tall; its touch target is the platform's.
///
/// A spinner replaces its title while `loading`. A pill at work stays at
/// full strength while the others dim.
pub fn pill_button_loading(
    title: String,
    label: String,
    enabled: bool,
    loading: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let pressable = enabled && !loading;
    view()
        .min_h(px(touch_target()))
        .px_1()
        .justify_center()
        .role(AccessibilityRole::Button)
        .label(label)
        .accessibility_state(move || AccessibilityState {
            busy: loading,
            disabled: !pressable,
            ..AccessibilityState::default()
        })
        .opacity(if enabled || loading { 1.0 } else { 0.4 })
        .when(pressable, |v| {
            v.on_press(on_press).active(|s| s.opacity(0.6))
        })
        .child(
            view()
                .min_h(px(32.0))
                .px(px(14.0))
                .py_1()
                .rounded_full()
                .items_center()
                .justify_center()
                .bg(move || theme().muted)
                .child(loading_content(
                    loading,
                    16.0,
                    theme().foreground,
                    text(title)
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().foreground),
                )),
        )
}

/// Footnote under a group.
pub fn footnote(copy: impl Into<String>) -> impl IntoElement {
    text(copy.into())
        .text_size(px(crate::ui::ty::FOOTNOTE))
        .line_height(px(18.0))
        .text_color(move || theme().muted_foreground)
}
