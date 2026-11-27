//! Cards outside the lists: a notice that asks for something (an update to
//! confirm, a restart for changed Looks), and the large card that leads a
//! page (add a camera, connect a saved one).

use nsh_core::prelude::*;

use crate::ui::ty::text;
use crate::ui::{chevron, token};

/// A card that tells of one thing to do. Build it with [`notice_card`].
pub struct NoticeCard {
    title: String,
    subtitle: Option<String>,
    /// A list under the title, one item a line.
    items: Vec<String>,
    leading: Option<AnyElement>,
    action: Option<AnyElement>,
    /// A button under the words, as wide as it needs.
    below: Option<AnyElement>,
    open: Option<Box<dyn Fn()>>,
    emphasis: bool,
    test_id: Option<String>,
}

pub fn notice_card(title: impl Into<String>) -> NoticeCard {
    NoticeCard {
        title: title.into(),
        subtitle: None,
        items: Vec::new(),
        leading: None,
        action: None,
        below: None,
        open: None,
        emphasis: false,
        test_id: None,
    }
}

impl NoticeCard {
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into()).filter(|s: &String| !s.is_empty());
        self
    }

    /// A list under the title, such as the changes that wait for a restart.
    pub fn items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self
    }

    /// A button under the words, for a card whose words need the width.
    pub fn action_below(mut self, action: impl IntoElement) -> Self {
        self.below = Some(action.into_element());
        self
    }

    /// A picture at the start: the camera, or an icon.
    pub fn leading(mut self, leading: impl IntoElement) -> Self {
        self.leading = Some(leading.into_element());
        self
    }

    /// A button at the end, such as Restart Camera.
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_element());
        self
    }

    /// Open what the notice is about when the card is pressed: a chevron
    /// shows.
    pub fn open(mut self, open: impl Fn() + 'static) -> Self {
        self.open = Some(Box::new(open));
        self
    }

    /// The tint around the card and a bold title: the person has to act,
    /// such as an update that waits on the camera.
    pub fn emphasis(mut self) -> Self {
        self.emphasis = true;
        self
    }

    pub fn test_id(mut self, id: impl Into<String>) -> Self {
        self.test_id = Some(id.into());
        self
    }
}

impl IntoElement for NoticeCard {
    fn into_element(self) -> AnyElement {
        let mut label = match &self.subtitle {
            Some(subtitle) => format!("{}, {subtitle}", self.title),
            None => self.title.clone(),
        };
        for item in &self.items {
            label.push_str(", ");
            label.push_str(item);
        }
        let emphasis = self.emphasis;
        let mut card = view()
            .flex_row()
            .items_center()
            .gap(px(14.0))
            .py(px(14.0))
            .pl(px(16.0))
            .pr(px(if self.action.is_some() { 12.0 } else { 16.0 }))
            .rounded(px(crate::ui::list::SECTION_RADIUS))
            .bg(move || token("list-card"))
            .when(emphasis, |v| {
                v.border(px(1.5)).border_color(move || token("tint"))
            })
            .label(label)
            .when_some(self.test_id.as_deref(), |v, id| v.test_id(id))
            .child(self.leading)
            .child(
                view()
                    .flex_1()
                    .gap(px(2.0))
                    .child(
                        text(self.title)
                            .text_size(px(if emphasis {
                                crate::ui::ty::HEADLINE
                            } else {
                                crate::ui::ty::SUBHEADLINE
                            }))
                            .font_weight(if emphasis {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::NORMAL
                            }),
                    )
                    .child(self.subtitle.map(|subtitle| {
                        text(subtitle)
                            .text_size(px(crate::ui::ty::SUBHEADLINE))
                            .text_color(move || token("list-secondary"))
                    }))
                    .children(self.items.into_iter().map(|item| {
                        view()
                            .flex_row()
                            .gap(px(6.0))
                            .child(
                                text("•")
                                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                                    .text_color(move || token("list-secondary")),
                            )
                            .child(
                                text(item)
                                    .flex_1()
                                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                                    .text_color(move || token("list-secondary")),
                            )
                    }))
                    .child(
                        self.below
                            .map(|below| view().flex_row().mt(px(10.0)).child(below)),
                    ),
            )
            .child(self.action)
            .child(self.open.is_some().then(chevron));
        if let Some(open) = self.open {
            card = card
                .role(AccessibilityRole::Button)
                .on_press(open)
                .active(|s| s.opacity(0.72));
        }
        card.into_element()
    }
}

/// The large card that leads a page: a picture, a title, a line under it,
/// and the page's main button. Build it with [`feature_card`].
pub struct FeatureCard {
    art: AnyElement,
    title: String,
    subtitle: Option<String>,
    status: Option<AnyElement>,
    action: Option<AnyElement>,
    centered: bool,
}

/// A card with `art` above `title`.
pub fn feature_card(art: impl IntoElement, title: impl Into<String>) -> FeatureCard {
    FeatureCard {
        art: art.into_element(),
        title: title.into(),
        subtitle: None,
        status: None,
        action: None,
        centered: false,
    }
}

impl FeatureCard {
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into()).filter(|s: &String| !s.is_empty());
        self
    }

    /// A state under the title, such as [`crate::ui::status_line`].
    pub fn status(mut self, status: impl IntoElement) -> Self {
        self.status = Some(status.into_element());
        self
    }

    /// The page's main button, across the card's foot.
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_element());
        self
    }

    /// The words in the middle, under centered art.
    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }
}

impl IntoElement for FeatureCard {
    fn into_element(self) -> AnyElement {
        let centered = self.centered;
        view()
            .rounded(px(28.0))
            .bg(move || token("list-card"))
            .pt(px(28.0))
            .px(px(24.0))
            .pb(px(24.0))
            .child(view().items_center().child(self.art))
            .child(
                view()
                    .mt(px(26.0))
                    .gap(px(6.0))
                    .when(centered, |v| v.items_center())
                    .child(
                        text(self.title)
                            .text_size(px(28.0))
                            .font_weight(FontWeight::BOLD)
                            .role(AccessibilityRole::Header)
                            .when(centered, |t| t.text_center()),
                    )
                    .child(self.subtitle.map(|subtitle| {
                        text(subtitle)
                            .text_size(px(crate::ui::ty::BODY))
                            .text_color(move || token("list-secondary"))
                            .when(centered, |t| t.text_center())
                    }))
                    .child(self.status),
            )
            .child(self.action.map(|action| view().mt(px(24.0)).child(action)))
            .into_element()
    }
}
