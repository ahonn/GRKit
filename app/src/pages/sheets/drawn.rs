//! Android and desktop: the open sheet, drawn at the bottom of the screen.

use nsh_base::{SheetEvent, sheet};
use nsh_core::prelude::*;

use super::{content, dismiss, dismissible};
use crate::store::AppState;
use crate::ui::sheet_surface;

/// The open sheet. A swipe or a press on the backdrop does what the
/// sheet's own way out does (`dismiss`).
pub fn overlay(app: AppState) -> impl IntoElement {
    let dismissible = move || dismissible(app);
    sheet()
        .open({
            let app = app.clone();
            move || Some(app.sheet().is_some())
        })
        .side(Placement::Bottom)
        .close_on_backdrop_press(dismissible)
        .close_on_swipe(dismissible)
        .close_on_escape(dismissible)
        .backdrop_style(|s| s.bg(rgba(0x00000047)))
        .popup_style(|s| {
            s.mx(px(if cfg!(target_os = "android") {
                0.0
            } else {
                8.0
            }))
            .mb(px(if cfg!(target_os = "android") {
                0.0
            } else {
                8.0
            }))
            .rounded_t(px(if cfg!(target_os = "android") {
                28.0
            } else {
                30.0
            }))
            .rounded_b(px(if cfg!(target_os = "android") {
                0.0
            } else {
                30.0
            }))
            .bg(sheet_surface)
            .shadow([BoxShadow {
                offset_x: Points(0.0),
                offset_y: Points(20.0),
                blur: Points(60.0),
                spread: Points(0.0),
                color: rgba(0x00000040).into(),
                inset: false,
            }])
        })
        .on_open_change({
            let app = app.clone();
            move |e: &SheetEvent| {
                if let SheetEvent::OpenChange(false, _) = e {
                    dismiss(app)
                }
            }
        })
        .child(
            // Large text can make a sheet taller than the screen.
            scroll()
                .max_h(move || px(window().height.0 * MAX_SHEET_HEIGHT))
                .bounces(false)
                .child(content(app)),
        )
}

/// The share of the screen a drawn sheet may take before it scrolls.
const MAX_SHEET_HEIGHT: f32 = 0.92;
