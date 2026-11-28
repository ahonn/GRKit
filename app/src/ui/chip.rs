//! Short states beside a title or on a card: In Use, Current, an update to
//! install, a change that waits for a restart; and the connection line of
//! a camera.

use nsh_core::prelude::*;

use crate::ui::token;
use crate::ui::ty::text;

/// The colors of a [`chip`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChipTone {
    /// The tint on its soft fill: the thing in use or current.
    Tint,
    /// White on the tint: something to act on, such as an update.
    Strong,
    /// Orange on its soft fill: a change that waits for a restart.
    Pending,
    /// Gray: a plain fact, such as B&W.
    Neutral,
}

impl ChipTone {
    /// The token names of its fill and ink; `None` for the fixed white ink
    /// of [`ChipTone::Strong`].
    pub fn tokens(self) -> (&'static str, Option<&'static str>) {
        match self {
            ChipTone::Tint => ("tint-soft", Some("tint")),
            ChipTone::Strong => ("tint", None),
            ChipTone::Pending => ("pending-soft", Some("pending")),
            ChipTone::Neutral => ("list-separator", Some("list-secondary")),
        }
    }
}

/// A capsule of `title` in the colors of `tone`.
pub fn chip(title: impl Into<String>, tone: ChipTone) -> AnyElement {
    let (fill, ink) = tone.tokens();
    let ink = move || match ink {
        Some(name) => token(name),
        None => rgb(0xFFFFFF).into(),
    };
    view()
        .h(px(24.0))
        .px(px(9.0))
        .flex_shrink(0.0)
        .rounded_full()
        .items_center()
        .justify_center()
        .bg(move || token(fill))
        .child(
            text(title.into())
                .text_size(px(crate::ui::ty::CAPTION))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ink),
        )
        .into_element()
}

/// A state at the end of a list row, in place of a Small button in the same
/// column (Current, On camera): the button's height and title size, so the
/// column keeps one height and one right edge.
pub fn row_chip(title: impl Into<String>, tone: ChipTone) -> AnyElement {
    let (fill, ink) = tone.tokens();
    let ink = move || match ink {
        Some(name) => token(name),
        None => rgb(0xFFFFFF).into(),
    };
    let size = crate::ui::ButtonSize::Small;
    view()
        .h(px(size.height()))
        .px(px(10.0))
        .flex_shrink(0.0)
        .rounded_full()
        .items_center()
        .justify_center()
        .bg(move || token(fill))
        .child(
            text(title.into())
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ink),
        )
        .into_element()
}

/// A camera's connection: a green dot and `title` while `online`, else
/// `title` alone in the secondary color.
pub fn status_line(title: impl Into<String>, online: bool) -> AnyElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .child(online.then(|| {
            view()
                .size(px(8.0))
                .rounded_full()
                .bg(move || token("online-dot"))
        }))
        .child(
            text(title.into())
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .text_color(move || {
                    if online {
                        token("online")
                    } else {
                        token("list-secondary")
                    }
                }),
        )
        .into_element()
}
