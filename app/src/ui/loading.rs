//! Static placeholders that hold the geometry of what loads, with no shimmer
//! and no artificial delay: a page keeps its layout from the connect to the
//! read lists, and only the words change.
use nsh_core::prelude::*;

use crate::strings::tr;
use crate::ui::{busy_state, token};

/// A gray bar in place of words that are not read yet.
pub fn skeleton_bar(width: f32, height: f32) -> AnyElement {
    view()
        .w(px(width))
        .h(px(height))
        .rounded(px(height / 2.0))
        .bg(move || token("skeleton"))
        .into_element()
}

/// The photo card of the Look in use before it is read: its area, the cover
/// and the two lines at its foot, at the places of the photo card's own.
pub fn hero_skeleton() -> AnyElement {
    view()
        .w_full()
        .h(px(super::media::HERO_HEIGHT))
        .bg(move || token("skeleton-area"))
        .child(
            view()
                .absolute()
                .left(px(14.0))
                .bottom(px(14.0))
                .flex_row()
                .items_end()
                .gap(px(12.0))
                .child(
                    view()
                        .w(px(56.0))
                        .h(px(56.0 * 34.0 / 40.0))
                        .rounded(px(3.0))
                        .bg(move || token("skeleton")),
                )
                .child(
                    view()
                        .gap(px(8.0))
                        .pb(px(3.0))
                        .child(skeleton_bar(72.0, 10.0))
                        .child(skeleton_bar(132.0, 18.0)),
                ),
        )
        .label(tr("loading.reading", &[]))
        .accessibility_state(busy_state(true))
        .into_element()
}

/// A row with a cover before it is read: the cover, the name and the line
/// under it, in the geometry of a list row with a subtitle.
pub fn cover_row_skeleton() -> AnyElement {
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .pl_4()
        .min_h(px(super::list::ROW_TWO_LINES))
        .child(
            view()
                .w(px(40.0))
                .h(px(40.0 * 34.0 / 40.0))
                .rounded(px(3.0))
                .bg(move || token("skeleton")),
        )
        .child(
            view()
                .flex_1()
                .pr_4()
                .py(px(10.0))
                .gap(px(6.0))
                .child(skeleton_bar(96.0, 14.0))
                .child(skeleton_bar(56.0, 10.0)),
        )
        .label(tr("loading.reading", &[]))
        .accessibility_state(busy_state(true))
        .into_element()
}
