//! Buttons in a capsule: the one main action of a page, and small actions
//! inside rows and cards.

use nsh_core::prelude::*;

use crate::ui::ty::text;
use crate::ui::{busy_state, quiet_spinner, token};

/// The fill and ink of a [`Button`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    /// White on the brand tint: the main action. A page has one at most.
    Primary,
    /// The tint on a gray fill: an action in a row or a card.
    Tinted,
    /// Red on a gray fill: an action that deletes or cancels.
    Destructive,
}

/// The size of a [`Button`]: the heights, titles, and side margins of the
/// system's capsule buttons on iOS (UIButton.Configuration sizes large,
/// medium, and small).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonSize {
    /// 50 points, across its container: the main action at a page's foot.
    Large,
    /// 34 points, as wide as its title: the action at the end of a card.
    Medium,
    /// 28 points, as wide as its title: an action in a row.
    Small,
}

impl ButtonSize {
    pub fn height(self) -> f32 {
        match self {
            ButtonSize::Large => 50.0,
            ButtonSize::Medium => 34.0,
            ButtonSize::Small => 28.0,
        }
    }

    fn title_size(self) -> f32 {
        match self {
            ButtonSize::Large | ButtonSize::Medium => crate::ui::ty::BUTTON,
            ButtonSize::Small => crate::ui::ty::SUBHEADLINE,
        }
    }

    /// The margin on each side of the title of a button as wide as its title.
    fn side(self) -> f32 {
        match self {
            ButtonSize::Large => 20.0,
            ButtonSize::Medium => 12.0,
            ButtonSize::Small => 10.0,
        }
    }
}

/// A button in a capsule. Build it with [`button`].
pub struct Button {
    title: String,
    label: Option<String>,
    kind: ButtonKind,
    size: ButtonSize,
    enabled: bool,
    loading: bool,
    on_press: Box<dyn Fn()>,
}

/// A button titled `title` that runs `on_press`.
pub fn button(
    title: impl Into<String>,
    kind: ButtonKind,
    size: ButtonSize,
    on_press: impl Fn() + 'static,
) -> Button {
    Button {
        title: title.into(),
        label: None,
        kind,
        size,
        enabled: true,
        loading: false,
        on_press: Box::new(on_press),
    }
}

impl Button {
    /// What screen readers say, when the title alone is not enough
    /// (Apply Portra 400 for Apply).
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The action runs: a spinner takes the title's place, and the button
    /// keeps its size and cannot be pressed.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
}

impl IntoElement for Button {
    fn into_element(self) -> AnyElement {
        let Button {
            title,
            label,
            kind,
            size,
            enabled,
            loading,
            on_press,
        } = self;
        let pressable = enabled && !loading;
        let height = size.height();
        let ink = move || match kind {
            ButtonKind::Primary => rgb(0xFFFFFF).into(),
            ButtonKind::Tinted => token("tint"),
            ButtonKind::Destructive => token("destructive-text"),
        };
        let fill = move || match kind {
            // A darker red under white words in dark mode, for their contrast.
            ButtonKind::Primary => token("tint-fill"),
            // The system's fill: a darker gray would not show on black.
            ButtonKind::Tinted | ButtonKind::Destructive => {
                if dark() {
                    rgba(0x7878805C).into()
                } else {
                    rgba(0x7676801F).into()
                }
            }
        };
        let title_text = text(title.clone())
            .text_size(px(size.title_size()))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(ink)
            // Kept while loading, so the button keeps its width.
            .opacity(if loading { 0.0 } else { 1.0 });
        view()
            .h(px(height))
            .when(size == ButtonSize::Large, |v| v.w_full())
            .when(size != ButtonSize::Large, |v| v.px(px(size.side())))
            .rounded(px(height / 2.0))
            .items_center()
            .justify_center()
            .bg(fill)
            .opacity(if enabled || loading { 1.0 } else { 0.4 })
            .role(AccessibilityRole::Button)
            .label(label.unwrap_or(title))
            .accessibility_state(busy_state(loading))
            .disabled(!pressable)
            // Medium and Small are drawn under the platform's touch target:
            // the press area grows above and below to reach it.
            .hit_slop(crate::ui::vertical_hit_slop(height))
            .when(pressable, |v| {
                v.on_press(on_press).active(|s| s.opacity(0.72))
            })
            .child(title_text)
            .child(loading.then(|| {
                view()
                    .absolute()
                    .inset_0()
                    .items_center()
                    .justify_center()
                    .child(quiet_spinner(18.0))
            }))
            .into_element()
    }
}

/// A link in the tint below a page's content, 44 points tall so that it is
/// easy to press. `external` adds the arrow of a page outside the app.
pub fn text_link(
    title: impl Into<String>,
    external: bool,
    on_press: impl Fn() + 'static,
) -> AnyElement {
    let title = title.into();
    view()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .min_h(px(44.0))
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .on_press(on_press)
        .active(|s| s.opacity(0.72))
        .child(
            text(title)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || token("tint")),
        )
        .child(external.then(|| {
            svg(nsh_core::element::ImageSrc::svg_markup(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M5 2h7v7M12 2L3 11"/></svg>"#,
            ))
            .size(px(12.0))
            .text_color(move || token("tint"))
        }))
        .into_element()
}
