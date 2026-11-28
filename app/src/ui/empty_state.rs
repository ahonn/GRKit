//! A page with nothing to list: not connected, nothing on the camera yet,
//! or a list that could not be read. A faint symbol, a title, a note and at
//! most one button, in the middle of the space between the bars, on the
//! page's background and not on a card.

use nsh_base::theme;
use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use crate::ui::token;
use crate::ui::ty::text;

/// The symbol's size and opacity: present, but quieter than the words.
const ICON: f32 = 64.0;
const ICON_OPACITY: f32 = 0.25;
/// The words stay narrow, so that a note breaks into short lines.
const TEXT_WIDTH: f32 = 280.0;
const BUTTON_HEIGHT: f32 = 44.0;
const BUTTON_MIN_WIDTH: f32 = 140.0;
/// What a page keeps above and below its content on a phone: the bar under
/// the status bar, the content's top margin, and the floating tab bar.
const BARS: f32 = 54.0 + 16.0 + crate::ui::TAB_BAR_CLEARANCE;
/// How far the group sits above the middle: a little high, as the eye
/// reads a page.
const RAISE: f32 = 120.0;

/// An empty page. Build it with [`empty_state`].
pub struct EmptyState {
    icon: &'static str,
    title: String,
    body: Option<String>,
    action: Option<(String, Box<dyn Fn()>)>,
}

/// An empty page titled `title` under `icon`, one of [`crate::ui::glyph`].
pub fn empty_state(icon: &'static str, title: impl Into<String>) -> EmptyState {
    EmptyState {
        icon,
        title: title.into(),
        body: None,
        action: None,
    }
}

impl EmptyState {
    /// The note under the title: what to do, or why the page is empty.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// The one button under the note.
    pub fn action(mut self, title: impl Into<String>, on_press: impl Fn() + 'static) -> Self {
        self.action = Some((title.into(), Box::new(on_press)));
        self
    }
}

/// The height between the bars, so that the group can sit in its middle.
fn page_height() -> f32 {
    let insets = safe_area();
    (window().height.0 - insets.top.0 - BARS).max(240.0)
}

impl IntoElement for EmptyState {
    fn into_element(self) -> AnyElement {
        let EmptyState {
            icon,
            title,
            body,
            action,
        } = self;
        let markup = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">{icon}</svg>"#
        );
        view()
            .min_h(move || px(page_height()))
            .pb(px(RAISE))
            .items_center()
            .justify_center()
            .child(
                view()
                    .w(px(TEXT_WIDTH))
                    .items_center()
                    .gap(px(16.0))
                    .child(
                        svg(ImageSrc::svg_markup(&markup))
                            .size(px(ICON))
                            .opacity(ICON_OPACITY)
                            .text_color(move || theme().foreground),
                    )
                    .child(
                        text(title)
                            .text_size(px(20.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_center()
                            .role(AccessibilityRole::Header),
                    )
                    .child(body.map(|body| {
                        text(body)
                            .text_size(px(crate::ui::ty::CALLOUT))
                            .text_center()
                            .text_color(move || token("list-secondary"))
                    }))
                    .child(action.map(|(title, on_press)| {
                        view().mt(px(8.0)).child(capsule(title, on_press))
                    })),
            )
            .into_element()
    }
}

/// A light capsule button, as wide as its title and at least
/// [`BUTTON_MIN_WIDTH`]: the system's glass button over a plain background.
fn capsule(title: String, on_press: Box<dyn Fn()>) -> AnyElement {
    view()
        .h(px(BUTTON_HEIGHT))
        .min_w(px(BUTTON_MIN_WIDTH))
        .max_w(px(TEXT_WIDTH))
        .px(px(20.0))
        .rounded(px(BUTTON_HEIGHT / 2.0))
        .items_center()
        .justify_center()
        .bg(move || token("list-card"))
        .border_1()
        .border_color(move || token("list-separator"))
        .shadow([BoxShadow {
            offset_x: Points(0.0),
            offset_y: Points(2.0),
            blur: Points(10.0),
            spread: Points(0.0),
            color: rgba(0x00000014).into(),
            inset: false,
        }])
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .on_press(on_press)
        .active(|s| s.opacity(0.72))
        .child(
            text(title)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::MEDIUM)
                .text_center()
                .text_color(move || theme().foreground),
        )
        .into_element()
}
