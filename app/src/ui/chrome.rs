//! Inset grouped lists and shared controls.

use nsh_base::theme;
use nsh_core::prelude::*;

use grkit_core::cover::Cover;

use crate::domain::CameraModel;
use crate::domain::look_media::{ICON_SIDE, enlarge_icon};
use crate::strings::tr;

const ART_GR3: ImageAsset = asset!("art-gr3.png");
const ART_GR3X: ImageAsset = asset!("art-gr3x.png");
const ART_GR4: ImageAsset = asset!("art-gr4.png");
const GR_SNAP: ImageAsset = asset!("gr-snap.png");
pub const STEP1: ImageAsset = asset!("step1.jpg");
pub const STEP2: ImageAsset = asset!("step2.jpg");
pub const STEP3: ImageAsset = asset!("step3.jpg");

/// The platform's own loader, which each platform registers as a native
/// view: Android's indeterminate `ProgressBar` (`MainActivity`), iOS's
/// `UIActivityIndicatorView` (`platform::ios::activity`).
pub const ACTIVITY_COMPONENT: &str = "grkit/activity";

/// The platform's loader in the tint color, `size` points square. Where no
/// platform view exists (desktop runs, headless tests) a turning arc stands
/// in for it.
pub fn activity_indicator(size: f32) -> AnyElement {
    if cfg!(any(target_os = "ios", target_os = "android")) {
        let tint = token("tint");
        native(ACTIVITY_COMPONENT)
            .size(px(size))
            .prop("light", serde_json::json!(tint.light.to_hex()))
            .prop("dark", serde_json::json!(tint.dark.to_hex()))
            .into_element()
    } else {
        view()
            .size(px(size))
            .text_color(move || token("tint"))
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
        .on_press(on_press)
        .active(|s| s.opacity(0.72))
        .child(
            text(title)
                .text_size(px(if cfg!(target_os = "android") {
                    16.0
                } else {
                    17.0
                }))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || token("button-foreground")),
        )
}

/// Full-width primary action inside a sheet.
pub fn sheet_primary_button(
    title: impl Into<String>,
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
        .on_press(on_press)
        .active(|s| s.opacity(0.72))
        .child(
            text(title)
                .text_size(px(if cfg!(target_os = "android") {
                    16.0
                } else {
                    17.0
                }))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || token("button-foreground")),
        )
}

pub fn secondary_button(
    title: impl Into<String>,
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
        .on_press(on_press)
        .active(|s| s.opacity(0.72))
        .child(
            text(title)
                .text_size(px(15.0))
                .line_height(px(22.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || token("link")),
        )
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
                .text_size(px(17.0))
                .text_color(move || theme().muted_foreground),
        )
}

pub fn section_title(title: impl Into<String>) -> impl IntoElement {
    text(title.into())
        .text_size(px(15.0))
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
        .min_h(px(50.0))
        .px_4()
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .child(text(title).text_size(px(17.0)))
        .child(
            text(value)
                .text_size(px(17.0))
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
                        .text_size(px(17.0))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(subtitle.map(|s| {
                    text(s)
                        .text_size(px(15.0))
                        .line_height(px(22.0))
                        .text_color(move || theme().muted_foreground)
                })),
        )
        .child(
            text("›")
                .text_size(px(22.0))
                .text_color(move || theme().muted_foreground),
        )
}

pub fn checklist_row(title: &str, subtitle: Option<&str>, state: StepState) -> impl IntoElement {
    let title = title.to_owned();
    let subtitle = subtitle.map(str::to_owned);
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .min_h(px(50.0))
        .px_4()
        .child(step_icon(state))
        .child(
            view()
                .flex_1()
                .gap(px(2.0))
                .child(
                    text(title)
                        .text_size(px(17.0))
                        .style_when(matches!(state, StepState::Current | StepState::Done), |s| {
                            s.font_weight(FontWeight::SEMIBOLD)
                        })
                        .text_color(move || match state {
                            StepState::Waiting => theme().muted_foreground,
                            _ => theme().foreground,
                        }),
                )
                .child(subtitle.map(|s| {
                    text(s)
                        .text_size(px(13.0))
                        .text_color(move || theme().muted_foreground)
                })),
        )
}

#[derive(Clone, Copy)]
pub enum StepState {
    Done,
    Current,
    Waiting,
    /// The step an error stopped at.
    Failed,
}

fn step_icon(state: StepState) -> impl IntoElement {
    match state {
        StepState::Done => view()
            .size(px(22.0))
            .rounded_full()
            .items_center()
            .justify_center()
            .bg(move || token("success"))
            .child(
                text("✓")
                    .text_size(px(12.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(move || token("button-foreground")),
            )
            .into_element(),
        // Filled ring: outer stroke + solid center (spinner alternative).
        StepState::Current => view()
            .size(px(22.0))
            .rounded_full()
            .items_center()
            .justify_center()
            .border_2()
            .border_color(move || theme().foreground)
            .child(
                view()
                    .size(px(8.0))
                    .rounded_full()
                    .bg(move || theme().foreground),
            )
            .into_element(),
        StepState::Waiting => view()
            .size(px(22.0))
            .rounded_full()
            .border_2()
            .border_color(move || theme().muted_foreground)
            .opacity(0.35)
            .into_element(),
        StepState::Failed => failed_icon(22.0).into_element(),
    }
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
                        .text_size(px(22.0))
                        .font_weight(FontWeight::BOLD),
                )
                .child(subtitle.map(|s| {
                    text(s)
                        .text_size(px(15.0))
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

pub fn gr_mark(size: f32) -> impl IntoElement {
    view()
        .size(px(size))
        .rounded(px(8.0))
        .items_center()
        .justify_center()
        .bg(move || token("button"))
        .child(
            text("GR")
                .text_size(px(size * 0.33))
                .font_weight(FontWeight::BOLD)
                .text_color(move || token("button-foreground")),
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
                .text_size(px(if cfg!(target_os = "android") {
                    16.0
                } else {
                    17.0
                }))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || token("connection-accent-foreground")),
        )
}

pub fn connection_icon(path: &'static str) -> impl IntoElement {
    svg(nsh_core::element::ImageSrc::svg_markup(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="{path}"/></svg>"#
    ))).size(px(20.0))
}

/// Large page title with the brand bar, and an optional trailing control.
pub fn page_title(title: impl Into<String>, trailing: Option<AnyElement>) -> impl IntoElement {
    let title = title.into();
    view()
        .flex_row()
        .items_end()
        .justify_between()
        .child(
            view()
                .gap_2()
                .child(view().w(px(28.0)).h(px(3.0)).bg(move || token("tint")))
                .child(
                    text(title)
                        .text_size(px(28.0))
                        .font_weight(FontWeight::BOLD),
                ),
        )
        .child(trailing)
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
        .border_color(move || theme().border)
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
        .h(px(36.0))
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
                .role(AccessibilityRole::Tab)
                .label(title.clone())
                .accessibility_state(move || AccessibilityState {
                    selected,
                    ..AccessibilityState::default()
                })
                .when(selected, |v| {
                    v.bg(move || theme().card).shadow([BoxShadow {
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
                        .text_size(px(14.0))
                        .font_weight(if selected {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_color(move || theme().foreground),
                ),
        );
    }
    row
}

/// On/off switch in the brand tint.
pub fn toggle(
    on: bool,
    label: String,
    enabled: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .w(px(51.0))
        .h(px(31.0))
        .p(px(2.0))
        .rounded_full()
        .flex_row()
        .justify_start()
        .when(on, |v| v.justify_end())
        .bg(move || if on { token("tint") } else { theme().border })
        .opacity(if enabled { 1.0 } else { 0.4 })
        .role(AccessibilityRole::Switch)
        .label(label)
        .accessibility_state(move || AccessibilityState {
            checked: Some(if on {
                nsh_core::Checked::True
            } else {
                nsh_core::Checked::False
            }),
            disabled: !enabled,
            ..AccessibilityState::default()
        })
        .when(enabled, |v| v.on_press(on_press))
        .child(
            view()
                .size(px(27.0))
                .rounded_full()
                .bg(rgb(0xFFFFFF))
                .shadow([BoxShadow {
                    offset_x: Points(0.0),
                    offset_y: Points(2.0),
                    blur: Points(4.0),
                    spread: Points(0.0),
                    color: rgba(0x00000026).into(),
                    inset: false,
                }]),
        )
}

/// Small label in the accent tint, such as In Use.
pub fn badge(title: String) -> impl IntoElement {
    view()
        .h(px(24.0))
        .px_2()
        .rounded(px(6.0))
        .items_center()
        .justify_center()
        .bg(move || theme().accent)
        .child(
            text(title)
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || token("tint")),
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

/// The 40 x 40 pixels the camera shows for `cover`, about `size` points
/// square. Each pixel is a block of whole screen pixels, so the image is
/// never smoothed.
pub fn camera_icon(cover: &Cover, size: f32) -> AnyElement {
    let ratio = pixel_ratio().max(1.0);
    let factor = ((size * ratio) / ICON_SIDE as f32).round().max(1.0) as usize;
    let shown = (ICON_SIDE * factor) as f32 / ratio;
    match cover.pixels() {
        Ok(icon) => {
            let side = (ICON_SIDE * factor) as u32;
            let pixels: std::sync::Arc<[u8]> = enlarge_icon(&icon, factor).into();
            image(nsh_core::element::ImageSrc::rgba(side, side, pixels))
                .w(px(shown))
                .h(px(shown))
                .into_element()
        }
        Err(_) => view()
            .w(px(shown))
            .h(px(shown))
            .rounded(px(4.0))
            .bg(move || theme().muted)
            .into_element(),
    }
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
        .child(text(title.into()).text_size(px(17.0)).flex_shrink(1.0))
        .child(trailing)
}

/// A pressable row whose title is the action, in `color`.
pub fn action_row(
    title: impl Into<String>,
    destructive: bool,
    last: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let title = title.into();
    view()
        .flex_row()
        .items_center()
        .min_h(px(50.0))
        .px_4()
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .on_press(on_press)
        .active(|s| s.opacity(0.72))
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .child(text(title).text_size(px(17.0)).text_color(move || {
            if destructive {
                theme().destructive
            } else {
                token("link")
            }
        }))
}

/// Footnote under a group.
pub fn footnote(copy: impl Into<String>) -> impl IntoElement {
    text(copy.into())
        .text_size(px(13.0))
        .line_height(px(18.0))
        .text_color(move || theme().muted_foreground)
}
