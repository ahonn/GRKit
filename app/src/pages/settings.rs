//! Settings page: the cameras, About, and More from the Maker (GR Snap).

use nsh_base::{alert_dialog, theme};
use nsh_core::prelude::*;

use crate::ui::ty::text;

use super::camera::saved_row;
use crate::store::{
    APP_BUILD, APP_VERSION, AppState, GR_SNAP_URL, HELP_URL, PRIVACY_URL, Route, rate_url,
};
use crate::strings::tr;
use crate::ui::{
    camera_art, footnote, gr_snap_icon, grouped_card, section_title, token, touch_target,
};

/// Settings (push, from the camera tab): the cameras, About, and More
/// from the Maker. Disconnect and turn off are on the camera card.
pub fn settings_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    crate::ui::app_page()
        .title(move || tr("settings.title", &[]))
        .on_dismiss(on_dismiss)
        .child(
            crate::ui::page_scroll()
                .flex_1()
                .bg(move || token("grouped"))
                .child(
                    view()
                        .gap_8()
                        .pt_4()
                        .px_6()
                        .pb_12()
                        .child(dynamic(move || camera_section(app)))
                        .child(
                            view()
                                .gap_2()
                                .child(section_title(tr("settings.about", &[])))
                                .child(grouped_card(
                                    view()
                                        .child(value_row(
                                            tr("settings.version", &[]),
                                            format!("{APP_VERSION} ({APP_BUILD})"),
                                            false,
                                        ))
                                        .child(link_row(tr("cam.help", &[]), false, true, || {
                                            spawn(async move {
                                                let _ = open_url(HELP_URL).await;
                                            });
                                        }))
                                        .child((!cfg!(target_os = "android")).then(|| {
                                            link_row(tr("settings.rate", &[]), false, false, || {
                                                spawn(async move {
                                                    let _ = open_url(rate_url()).await;
                                                });
                                            })
                                        }))
                                        .child(link_row(
                                            tr("settings.privacy", &[]),
                                            true,
                                            false,
                                            || {
                                                spawn(async move {
                                                    let _ = open_url(PRIVACY_URL).await;
                                                });
                                            },
                                        )),
                                )),
                        )
                        .child((!cfg!(target_os = "android")).then(|| {
                            view()
                                .gap_2()
                                .child(section_title(tr("settings.more_maker", &[])))
                                .child(grouped_card(gr_snap_row()))
                        })),
                ),
        )
}

pub fn confirm_disconnect(app: AppState) {
    spawn(async move {
        let answer = alert_dialog()
            .title(tr("cam.disconnect_title", &[]))
            .description(tr("cam.disconnect_body", &[]))
            .cancel("cancel", tr("cam.cancel", &[]))
            .destructive("disconnect", tr("cam.disconnect_confirm", &[]))
            .show()
            .await;
        if answer.ok().and_then(|a| a.action).as_deref() == Some("disconnect") {
            app.disconnect_camera();
        }
    });
}

pub fn confirm_power_off(app: AppState) {
    spawn(async move {
        let answer = alert_dialog()
            .title(tr("cam.power_title", &[]))
            .description(tr("cam.power_body", &[]))
            .cancel("cancel", tr("cam.cancel", &[]))
            .destructive("power", tr("cam.power_confirm", &[]))
            .show()
            .await;
        if answer.ok().and_then(|a| a.action).as_deref() == Some("power") {
            app.power_off_camera();
        }
    });
}

/// The most recent camera and the way to the saved cameras.
fn camera_section(app: AppState) -> Option<AnyElement> {
    let camera = app.remembered()?;
    let online = app.camera_online();
    let count = app.saved_cameras().len();
    let status = if online {
        format!("{} · {}", tr("cam.connected", &[]), camera.last_ssid)
    } else {
        tr("connected.offline", &[])
    };
    Some(
        view()
            .gap_2()
            .child(section_title(tr("cam.settings_group", &[])))
            .child(grouped_card(
                view()
                    .child(
                        view()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .min_h(px(68.0))
                            .px_4()
                            .border_b_1()
                            .border_b_color(move || theme().border)
                            .child(camera_art(camera.model, 48.0, false))
                            .child(
                                view()
                                    .flex_1()
                                    .gap(px(2.0))
                                    .child(
                                        text(format!("RICOH {}", camera.model.display_name()))
                                            .text_size(px(crate::ui::ty::HEADLINE))
                                            .font_weight(FontWeight::SEMIBOLD),
                                    )
                                    .child(
                                        text(status)
                                            .text_size(px(crate::ui::ty::FOOTNOTE))
                                            .text_color(move || theme().muted_foreground),
                                    ),
                            ),
                    )
                    .child(
                        view()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .min_h(px(52.0))
                            .px_4()
                            .role(AccessibilityRole::Button)
                            .label(tr("cam.saved", &[]))
                            .on_press(move || app.push(Route::SavedCameras))
                            .active(|st| st.bg(move || theme().accent))
                            .child(text(tr("cam.saved", &[])).text_size(px(crate::ui::ty::BODY)))
                            .child(
                                view()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(10.0))
                                    .child(
                                        text(count.to_string())
                                            .text_size(px(crate::ui::ty::BODY))
                                            .text_color(move || theme().muted_foreground),
                                    )
                                    .child(crate::ui::chevron()),
                            ),
                    ),
            ))
            .into_element(),
    )
}

/// The saved cameras: tap to connect, press and hold to remove.
pub fn saved_cameras_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    crate::ui::app_page()
        .title(move || tr("cam.saved", &[]))
        .on_dismiss(on_dismiss)
        .child(
            crate::ui::page_scroll()
                .flex_1()
                .bg(move || token("grouped"))
                .child(
                    view()
                        .gap_2()
                        .pt_4()
                        .px_6()
                        .pb_12()
                        .child(dynamic(move || {
                            let cameras = app.saved_cameras();
                            let count = cameras.len();
                            let online = app.camera_online();
                            grouped_card(view().children(cameras.into_iter().enumerate().map(
                                move |(index, camera)| {
                                    saved_row(app, camera, index + 1 == count, online && index == 0)
                                        .into_element()
                                },
                            )))
                        }))
                        .child(footnote(tr("cam.saved_note", &[]))),
                ),
        )
}

fn value_row(title: String, value: String, last: bool) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .min_h(px(52.0))
        .px_4()
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .child(
            text(title)
                .flex_shrink(1.0)
                .text_size(px(crate::ui::ty::BODY)),
        )
        .child(
            text(value)
                .flex_shrink(1.0)
                .text_size(px(crate::ui::ty::BODY))
                .text_color(move || theme().muted_foreground),
        )
}

/// A row that opens a page outside the app; `external` marks it with ↗.
fn link_row(
    title: String,
    last: bool,
    external: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let label = title.clone();
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .min_h(px(52.0))
        .px_4()
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .role(AccessibilityRole::Button)
        .label(label)
        .on_press(on_press)
        .active(|st| st.bg(move || theme().accent))
        .child(text(title).text_size(px(crate::ui::ty::BODY)))
        .child(external.then(|| {
            text("↗")
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .text_color(move || theme().muted_foreground)
        }))
}

fn gr_snap_row() -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .p_4()
        .min_h(px(72.0))
        .child(
            image(gr_snap_icon())
                .size(px(56.0))
                .rounded(px(13.0))
                .object_fit(ResizeMode::Cover),
        )
        .child(
            view()
                .flex_1()
                .gap(px(4.0))
                .child(
                    text(move || tr("settings.gr_snap_name", &[]))
                        .text_size(px(crate::ui::ty::HEADLINE))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    text(move || tr("settings.gr_snap_desc", &[]))
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                        .line_height(px(21.0))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(
            view()
                .min_h(px(touch_target()))
                .px_3()
                .rounded(px(8.0))
                .items_center()
                .justify_center()
                .bg(move || theme().muted)
                .role(AccessibilityRole::Button)
                .label(tr("settings.view", &[]))
                .active(|s| s.opacity(0.72))
                .on_press(|| {
                    spawn(async move {
                        let _ = open_url(GR_SNAP_URL).await;
                    });
                })
                .child(
                    text(move || tr("settings.view", &[]))
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || token("link")),
                ),
        )
}
