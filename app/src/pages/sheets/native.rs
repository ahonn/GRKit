//! iOS: the open sheet in a native page sheet, sized to its content, with
//! its buttons in the system bar.

use nsh_base::{dialog, nav_page, nav_stack};
use nsh_core::element::header::HeaderItem;
use nsh_core::prelude::*;
use nsh_core::{HeaderSystemItem, ModalPresentation, SheetDetent};

use super::{content, dismiss, dismissible, open_spec};
use crate::store::AppState;
use crate::strings::tr;
use crate::ui::sheet_surface;

/// An update under way asks before a swipe stops it, so the sheet only
/// reports the swipe. One that waits on the camera only closes.
fn swipe_closes(app: AppState) -> bool {
    dismissible(app) && open_spec(app).is_some_and(|spec| spec.swipe_closes)
}

/// The native sheet of the open sheet.
pub fn overlay(app: AppState) -> impl IntoElement {
    let height = signal(Points(420.0));
    // The tallest step of the update sheet so far: its steps advance in
    // place, so the sheet keeps that height instead of jumping.
    let peak = signal(Points(0.0));
    // Only opening and closing present the native sheet: a new step of the
    // same sheet changes its body in place, without a second presentation.
    let open = computed(move || app.sheet().is_some());
    // The same for the page inside: it changes only when its back button
    // comes or goes.
    let back = computed(move || open_spec(app).is_some_and(|spec| spec.back.is_some()));
    dynamic(move || {
        if !open.get() {
            untracked(move || peak.set(Points(0.0)));
        }
        open.get().then(|| {
            untracked(move || {
                dialog()
                    .presentation(ModalPresentation::PageSheet)
                    // The system keeps a tall sheet on the screen; its body scrolls.
                    .detents(move || vec![SheetDetent::Height(height.get())])
                    .grabber(move || dismissible(app))
                    .close_on_swipe(move || swipe_closes(app))
                    .close_on_backdrop_press(move || swipe_closes(app))
                    .close_on_escape(move || dismissible(app))
                    .on_cancel(move || dismiss(app))
                    .child(
                        nav_stack()
                            .flex_1()
                            .bg(sheet_surface)
                            .child(dynamic(move || {
                                // A back button comes and goes with the page.
                                let back = back.get();
                                untracked(move || {
                                    // A new page for the back button replaces the
                                    // old one in place: the sheet's steps do not
                                    // slide like a push.
                                    let page = nav_page()
                                        .key(if back { "back" } else { "plain" })
                                        .animation(nsh_core::ScreenAnimation::None)
                                        .header_transparent(true)
                                        .header_right_items(move || {
                                            open_spec(app)
                                                .filter(|spec| spec.close)
                                                .map(|spec| {
                                                    vec![
                                                        HeaderItem::system(HeaderSystemItem::Close)
                                                            .label(tr("scan.close", &[]))
                                                            .on_press(spec.dismiss),
                                                    ]
                                                })
                                                .unwrap_or_default()
                                        });
                                    let page = if back {
                                        page.header_left_symbol("chevron.backward").header_left(
                                            view()
                                                .role(AccessibilityRole::Button)
                                                .label(tr("scan.back", &[]))
                                                .on_press(move || {
                                                    if let Some(back) =
                                                        open_spec(app).and_then(|spec| spec.back)
                                                    {
                                                        back()
                                                    }
                                                }),
                                        )
                                    } else {
                                        page
                                    };
                                    page.child(
                                        scroll().flex_1().bg(sheet_surface).bounces(false).child(
                                            content(app).on_layout(move |b: &Bounds<Points>| {
                                                if b.size.height.0 <= 0.0 {
                                                    return;
                                                }
                                                // UIKit adds the bottom safe area to a
                                                // custom detent; the body ends at its own
                                                // padding, the same as its top.
                                                let mut h = Points(
                                                    (b.size.height.0 - safe_area().bottom.0)
                                                        .max(0.0),
                                                );
                                                if open_spec(app)
                                                    .is_some_and(|spec| spec.keeps_height)
                                                {
                                                    h = Points(h.0.max(peak.get().0));
                                                    peak.set(h);
                                                } else {
                                                    peak.set(Points(0.0));
                                                }
                                                height.set(h);
                                            }),
                                        ),
                                    )
                                    .into_element()
                                })
                            })),
                    )
            })
        })
    })
}
