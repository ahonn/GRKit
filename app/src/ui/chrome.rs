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
    nsh_base::nav_page().header_style_with(bar_style)
}

/// The title of a page in its bar. iOS keeps the system title. Android's
/// toolbar draws its title at 20 sp, smaller than Material 3: the page draws
/// its own instead, at 28 sp on the root page of a tab and 22 sp on a page
/// pushed above it.
pub trait PageTitle: Sized {
    fn page_title<T>(self, title: T, root: bool) -> Self
    where
        T: IntoReactive<SharedString> + Clone + 'static;
}

impl PageTitle for nsh_core::element::Screen {
    fn page_title<T>(self, title: T, root: bool) -> Self
    where
        T: IntoReactive<SharedString> + Clone + 'static,
    {
        let screen = self.title(title.clone());
        if !cfg!(target_os = "android") {
            return screen;
        }
        screen.header_title(
            crate::ui::ty::text(title)
                .text_size(px(if root { 28.0 } else { 22.0 }))
                .number_of_lines(1)
                .text_color(move || theme().foreground)
                .role(AccessibilityRole::Header),
        )
    }
}

/// The bar in the fill of the page under it, so the two read as one.
/// Android's bar otherwise takes its colors from the activity's light
/// theme, whatever the appearance.
fn bar_style(s: nsh_core::element::StyleRefinement) -> nsh_core::element::StyleRefinement {
    let s = s.bg(token("list-background"));
    if cfg!(target_os = "android") {
        s.text_color(theme().foreground)
            .tint_color(theme().foreground)
    } else {
        s
    }
}

/// A page whose top can be a photo ([`crate::ui::detail_header`]): while
/// `over_photo`, the bar is see-through over the photo and its buttons are
/// white. The bar has no title: the page shows it.
pub fn photo_page(over_photo: impl Fn() -> bool + Clone + 'static) -> nsh_core::element::Screen {
    nsh_base::nav_page()
        .header_title(view())
        .header_transparent(over_photo.clone())
        .header_style_with(move |s| {
            if over_photo() {
                s.tint_color(rgb(0xFFFFFF))
            } else {
                bar_style(s)
            }
        })
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
        .rounded_full()
        .items_center()
        .justify_center()
        // The brand tint, as the main buttons of the pages.
        .bg(move || token("tint-fill"))
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .accessibility_state(busy_state(loading))
        .when(!loading, |v| {
            v.on_press(on_press).active(|s| s.opacity(0.72))
        })
        .child(loading_content(
            loading,
            20.0,
            rgb(0xFFFFFF).into(),
            text(title)
                .text_size(px(crate::ui::ty::BUTTON))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(0xFFFFFF)),
        ))
}

/// The touch target of a [`secondary_button`], and the part of it below its
/// one line of title (a 20-point line).
const SECONDARY_HEIGHT: f32 = 44.0;
const SECONDARY_SLACK: f32 = (SECONDARY_HEIGHT - 20.0) / 2.0;

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
    // The other ways out under the main button: links in the tint, as the
    // links below a page's content (`text_link`). The touch target reaches
    // below the title; on the native sheet that part overlaps the sheet's
    // bottom padding instead of adding to it, so a sheet that ends with a
    // link keeps the same space under it as above its title.
    view()
        .w_full()
        .min_h(px(SECONDARY_HEIGHT))
        .when(NATIVE_SHEET, |v| v.mb(px(-SECONDARY_SLACK)))
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
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || token("tint")),
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
        .rounded_full()
        .items_center()
        .justify_center()
        .bg(move || token("tint-fill"))
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .on_press(on_press)
        .active(|s| s.opacity(0.72))
        .child(
            text(title)
                .text_size(px(crate::ui::ty::BUTTON))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(0xFFFFFF)),
        )
}

pub fn connection_icon(path: &'static str) -> impl IntoElement {
    svg(nsh_core::element::ImageSrc::svg_markup(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="{path}"/></svg>"#
    ))).size(px(20.0))
}

/// The buttons at the end of a page header (Android), as a toolbar's
/// actions: no container, and the system ripple on press. iOS shows its
/// native bar buttons instead.
pub fn header_buttons(buttons: impl IntoIterator<Item = AnyElement>) -> impl IntoElement {
    let mut row = view().flex_row().items_center();
    for button in buttons {
        row = row.child(button);
    }
    row
}

/// The press ripple of a header button: a 40-point circle for an icon,
/// which stays in its touch target and so in the header; the button's
/// bounds for a title.
fn header_ripple(borderless: bool) -> nsh_core::Ripple {
    nsh_core::Ripple {
        borderless,
        radius: borderless.then_some(nsh_core::Points(20.0)),
        ..nsh_core::Ripple::default()
    }
}

/// A 24-point stroked icon in a header, in a touch target of its own.
pub fn header_icon(
    path: &'static str,
    label: String,
    enabled: bool,
    on_press: impl Fn() + 'static,
) -> AnyElement {
    view()
        .size(px(touch_target()))
        .items_center()
        .justify_center()
        .role(AccessibilityRole::Button)
        .label(label)
        .opacity(if enabled { 1.0 } else { 0.38 })
        .when(enabled, |v| {
            v.on_press(on_press).ripple(header_ripple(true))
        })
        .text_color(move || theme().foreground)
        .child(svg(nsh_core::element::ImageSrc::svg_markup(&format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="{path}"/></svg>"#
        )))
        .size(px(24.0)))
        .into_element()
}

/// A title in a header, such as Edit.
pub fn header_text(title: String, on_press: impl Fn() + 'static) -> AnyElement {
    view()
        .min_h(px(touch_target()))
        .px_3()
        .rounded_full()
        .items_center()
        .justify_center()
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .on_press(on_press)
        .ripple(header_ripple(false))
        .child(
            text(title)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        )
        .into_element()
}

/// A header button that works: a spinner in its place.
pub fn header_busy(label: String) -> AnyElement {
    view()
        .size(px(touch_target()))
        .items_center()
        .justify_center()
        .label(label)
        .accessibility_state(busy_state(true))
        .child(quiet_spinner(20.0))
        .into_element()
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

/// A Look cover, `size` points wide ([`crate::ui::cover_thumb`]).
pub fn cover_view(cover: &Cover, size: f32, faded: bool) -> AnyElement {
    crate::ui::cover_thumb(cover, size, faded)
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
