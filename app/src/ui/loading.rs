//! Static placeholders preserve card geometry without shimmer or artificial delays.
use super::grouped_card;
use crate::strings::tr;
use nsh_base::theme;
use nsh_core::prelude::*;

use crate::ui::ty::text;

pub fn skeleton_bar(width: f32, height: f32) -> impl IntoElement {
    view()
        .w(px(width))
        .h(px(height))
        .rounded(px(5.0))
        .bg(move || theme().muted)
}

pub fn camera_skeleton() -> impl IntoElement {
    grouped_card(
        view()
            .p_4()
            .gap_4()
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .gap_4()
                    .child(skeleton_bar(80.0, 54.0))
                    .child(
                        view()
                            .gap_2()
                            .child(skeleton_bar(170.0, 22.0))
                            .child(skeleton_bar(100.0, 14.0)),
                    ),
            )
            .child(
                view()
                    .flex_row()
                    .gap_2()
                    .child(
                        view()
                            .flex_1()
                            .h(px(68.0))
                            .rounded(px(8.0))
                            .bg(move || theme().muted),
                    )
                    .child(
                        view()
                            .flex_1()
                            .h(px(68.0))
                            .rounded(px(8.0))
                            .bg(move || theme().muted),
                    ),
            )
            .child(
                text(tr("loading.camera", &[]))
                    .text_size(px(crate::ui::ty::FOOTNOTE))
                    .line_height(px(18.0))
                    .text_color(move || theme().muted_foreground),
            ),
    )
}
