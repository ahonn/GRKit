//! Camera tab: the first camera to add, the saved cameras, and the
//! connected camera with its update and Looks.

use nsh_base::{alert_dialog, nav_page, theme};
use nsh_core::prelude::*;

use crate::domain::{CameraModel, RememberedCamera};
use crate::store::{AppState, Tab, is_up_to_date};
use crate::strings::tr;
use crate::ui::{
    camera_art, chevron_row, connection_button, cover_view, gr_mark, grouped_card, primary_button,
    secondary_button, section_title, token, wordmark,
};

/// The height of the wordmark at the top of the disconnected camera tab.
const WORDMARK_HEIGHT: f32 = 26.0;
/// The green of a camera that is on and nearby.
const NEARBY_DOT: u32 = 0x2A9D4B;

pub fn camera_root(app: AppState) -> impl IntoElement {
    // The tab draws the wordmark itself, connected or not.
    nav_page()
        .title(move || tr("cam.title", &[]))
        .header_visible(false)
        .child(dynamic(move || {
            if app.initial_connect() {
                return connecting_home().into_element();
            }
            if app.camera_online()
                && let Some(status) = app.status()
            {
                return untracked(|| connected(app, status.camera.model).into_element());
            }
            match app.remembered() {
                Some(camera) => untracked(|| saved_home(app, camera).into_element()),
                None => empty_home(app).into_element(),
            }
        }))
}

/// The wordmark row at the top of the camera tab.
fn brand_header() -> impl IntoElement {
    view()
        .min_h(px(44.0))
        .justify_center()
        .child(wordmark(WORDMARK_HEIGHT))
}

fn connecting_home() -> impl IntoElement {
    crate::ui::page_scroll()
        .flex_1()
        .bg(move || token("grouped"))
        .child(
            view()
                .px_6()
                .pt(px(72.0))
                .gap_4()
                .child(brand_header())
                .child(crate::ui::camera_skeleton()),
        )
}

/// A card with the camera art, a title, a status line and its buttons.
fn camera_card(
    model: CameraModel,
    title: String,
    status: AnyElement,
    buttons: AnyElement,
    on_long_press: Option<Box<dyn Fn()>>,
) -> impl IntoElement {
    view()
        .rounded(px(10.0))
        .border_1()
        .border_color(move || token("connection-line"))
        .bg(move || token("connection-background"))
        .p_6()
        .gap_5()
        .when_some(on_long_press, |v, on_long_press| {
            v.on_long_press(on_long_press)
        })
        .child(
            view()
                .items_center()
                .py_4()
                .child(camera_art(model, 170.0, false)),
        )
        .child(
            view()
                .gap_2()
                .child(
                    text(title)
                        .text_size(px(22.0))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(status),
        )
        .child(buttons)
}

/// The quiet link for a phone that joined the camera Wi-Fi in Settings.
fn detect_link(app: AppState) -> impl IntoElement {
    view()
        .self_center()
        .min_h(px(44.0))
        .justify_center()
        .px_2()
        .role(AccessibilityRole::Button)
        .label(tr("cam.detect", &[]))
        .on_press(move || app.detect_now())
        .active(|s| s.opacity(0.6))
        .child(
            text(tr("cam.detect", &[]))
                .text_size(px(15.0))
                .text_color(move || theme().muted_foreground),
        )
}

fn empty_home(app: AppState) -> impl IntoElement {
    crate::ui::page_scroll()
        .flex_1()
        .bg(move || token("connection-background"))
        .child(
            view()
                .px_6()
                .pt(px(72.0))
                .pb_10()
                .gap_6()
                .child(brand_header())
                .child(camera_card(
                    CameraModel::Gr3,
                    tr("home.title", &[]),
                    muted_line(tr("connecting.bt", &[])).into_element(),
                    connection_button(tr("home.add", &[]), move || app.open_scan()).into_element(),
                    None,
                ))
                .child(detect_link(app)),
        )
}

fn muted_line(copy: String) -> impl IntoElement {
    text(copy)
        .text_size(px(14.0))
        .line_height(px(20.0))
        .text_color(move || theme().muted_foreground)
}

/// Whether the nearby watch sees this camera powered on.
fn seen_nearby(app: AppState, camera: &RememberedCamera) -> bool {
    app.nearby_cameras()
        .iter()
        .any(|c| c.powered_on && c.peripheral_id.eq_ignore_ascii_case(&camera.peripheral_id))
}

/// "On · Nearby" with a green dot, or "Not connected".
fn presence_line(app: AppState, camera: RememberedCamera) -> impl IntoElement {
    dynamic(move || {
        if seen_nearby(app, &camera) {
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(view().size(px(7.0)).rounded_full().bg(rgb(NEARBY_DOT)))
                .child(muted_line(tr("cam.nearby", &[])))
                .into_element()
        } else {
            muted_line(tr("connected.offline", &[])).into_element()
        }
    })
}

/// The saved cameras, the most recent first, while none is connected.
fn saved_home(app: AppState, camera: RememberedCamera) -> impl IntoElement {
    let others: Vec<RememberedCamera> = app.saved_cameras().into_iter().skip(1).collect();
    let menu_camera = camera.clone();
    crate::ui::page_scroll()
        .flex_1()
        .bg(move || token("connection-background"))
        .child(
            view()
                .px_6()
                .pt(px(72.0))
                .pb_10()
                .gap_6()
                .child(brand_header())
                .child(camera_card(
                    camera.model,
                    format!("RICOH {}", camera.model.display_name()),
                    presence_line(app, camera.clone()).into_element(),
                    view()
                        .w_full()
                        .gap_1()
                        .child(connection_button(tr("nearby.connect", &[]), move || {
                            app.connect_saved()
                        }))
                        .child(secondary_button(tr("home.add", &[]), move || {
                            app.open_scan()
                        }))
                        .into_element(),
                    Some(Box::new(move || camera_menu(app, menu_camera.clone()))),
                ))
                .child(detect_link(app))
                .child((!others.is_empty()).then(|| other_cameras(app, others))),
        )
}

fn other_cameras(app: AppState, cameras: Vec<RememberedCamera>) -> impl IntoElement {
    let count = cameras.len();
    view()
        .gap_2()
        .child(section_title(tr("cam.others", &[])))
        .child(grouped_card(view().children(
            cameras.into_iter().enumerate().map(move |(index, camera)| {
                saved_row(app, camera, index + 1 == count, false).into_element()
            }),
        )))
}

/// One saved camera: tap to connect, press and hold for its menu. While
/// another camera is connected it only shows.
pub fn saved_row(
    app: AppState,
    camera: RememberedCamera,
    last: bool,
    connected: bool,
) -> impl IntoElement {
    let name = format!("RICOH {}", camera.model.display_name());
    let id = camera.peripheral_id.clone();
    let menu_camera = camera.clone();
    let busy = app.camera_online();
    let detail = camera.last_ssid.clone();
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .min_h(px(64.0))
        .px_4()
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .role(AccessibilityRole::Button)
        .label(name.clone())
        .when(!busy, |v| {
            v.on_press(move || app.connect_camera(&id))
                .active(|s| s.bg(move || theme().accent))
        })
        .when(!connected, |v| {
            v.on_long_press(move || camera_menu(app, menu_camera.clone()))
        })
        .child(camera_art(camera.model, 48.0, false))
        .child(
            view()
                .flex_1()
                .gap(px(2.0))
                .child(
                    text(name)
                        .text_size(px(17.0))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    text(detail)
                        .text_size(px(13.0))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(if connected {
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(view().size(px(7.0)).rounded_full().bg(rgb(NEARBY_DOT)))
                .child(
                    text(tr("cam.connected", &[]))
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x1D6B33)),
                )
                .into_element()
        } else if busy {
            view().into_element()
        } else {
            text(tr("nearby.connect", &[]))
                .text_size(px(15.0))
                .text_color(move || token("link"))
                .into_element()
        })
}

/// The long-press menu of a saved camera: connect, or remove it.
pub fn camera_menu(app: AppState, camera: RememberedCamera) {
    let name = format!("RICOH {}", camera.model.display_name());
    spawn(async move {
        let mut sheet = alert_dialog().title(name.clone()).action_sheet();
        if !app.camera_online() {
            sheet = sheet.action("connect", tr("nearby.connect", &[]));
        }
        let answer = sheet
            .destructive("remove", tr("cam.remove", &[]))
            .cancel("cancel", tr("cam.cancel", &[]))
            .show()
            .await;
        match answer.ok().and_then(|a| a.action).as_deref() {
            Some("connect") => app.connect_camera(&camera.peripheral_id),
            Some("remove") => confirm_remove(app, camera, name).await,
            _ => {}
        }
    });
}

async fn confirm_remove(app: AppState, camera: RememberedCamera, name: String) {
    let answer = alert_dialog()
        .title(tr("cam.remove", &[]))
        .description(tr("cam.remove_body", &[("name", &name)]))
        .cancel("cancel", tr("cam.cancel", &[]))
        .destructive("remove", tr("cam.remove_confirm", &[]))
        .show()
        .await;
    if answer.ok().and_then(|a| a.action).as_deref() == Some("remove") {
        app.forget_camera(&camera.peripheral_id);
    }
}

fn connected(app: AppState, model: CameraModel) -> impl IntoElement {
    crate::ui::page_scroll()
        .flex_1()
        .bg(move || token("grouped"))
        .child(
            view()
                .gap_4()
                .pt(px(72.0))
                .px_6()
                .pb_10()
                .child(brand_header())
                .child(grouped_card(
                    view()
                        .p_4()
                        .gap_4()
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap_4()
                                .child(camera_art(model, 80.0, false))
                                .child(
                                    view()
                                        .flex_1()
                                        .gap(px(4.0))
                                        .child(
                                            text(format!("RICOH {}", model.display_name()))
                                                .text_size(px(20.0))
                                                .font_weight(FontWeight::SEMIBOLD),
                                        )
                                        .child(
                                            text(move || {
                                                app.remembered()
                                                    .map(|r| r.last_ssid)
                                                    .unwrap_or_else(|| tr("connected.status", &[]))
                                            })
                                            .text_size(px(15.0))
                                            .text_color(move || theme().muted_foreground),
                                        ),
                                ),
                        )
                        .child(dynamic(move || version_card(app))),
                ))
                .child(dynamic(move || update_section(app)))
                .child(dynamic(move || looks_row(app))),
        )
}

fn update_section(app: AppState) -> AnyElement {
    let package = app.package();
    let update_unknown = !app.demo() && package.as_ref().is_none_or(|p| p.available.is_none());
    let up_to_date = package.as_ref().is_some_and(is_up_to_date);
    if update_unknown && app.release_checking() {
        crate::ui::update_skeleton().into_element()
    } else if update_unknown {
        grouped_card(chevron_row(
            tr("connected.software_update", &[]),
            Some(tr(
                if app.release_checking() {
                    "software.checking"
                } else {
                    "software.unavailable"
                },
                &[],
            )),
            move || app.open_software_update(),
        ))
        .into_element()
    } else if up_to_date {
        up_to_date_card(app).into_element()
    } else {
        update_available_card(app).into_element()
    }
}

fn update_available_card(app: AppState) -> impl IntoElement {
    let package = app.package();
    let version = package
        .as_ref()
        .and_then(|p| p.available.clone())
        .unwrap_or_default();
    let subtitle = match package.as_ref().filter(|p| p.package_bytes > 0) {
        Some(p) => tr(
            "cam.update_sub",
            &[(
                "size",
                &format!("{:.1} MB", p.package_bytes as f64 / 1_000_000.0),
            )],
        ),
        None => tr("cam.update_sub_plain", &[]),
    };
    grouped_card(
        view()
            .p_4()
            .gap_4()
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .role(AccessibilityRole::Button)
                    .label(format!("GRKit {version}"))
                    .active(|s| s.opacity(0.72))
                    .on_press({
                        let app = app.clone();
                        move || app.open_software_update()
                    })
                    .child(gr_mark(48.0))
                    .child(
                        view()
                            .flex_1()
                            .gap(px(4.0))
                            .child(
                                text(format!("GRKit {version}"))
                                    .text_size(px(17.0))
                                    .font_weight(FontWeight::SEMIBOLD),
                            )
                            .child(
                                text(subtitle)
                                    .text_size(px(15.0))
                                    .line_height(px(22.0))
                                    .text_color(move || theme().muted_foreground),
                            ),
                    )
                    .child(
                        text("›")
                            .text_size(px(24.0))
                            .text_color(move || theme().muted_foreground),
                    ),
            )
            .child(primary_button(tr("connected.update", &[]), move || {
                app.begin_update(crate::domain::UpdateKind::Install)
            })),
    )
}

fn up_to_date_card(app: AppState) -> impl IntoElement {
    grouped_card(chevron_row(
        tr("connected.up_to_date", &[]),
        Some(tr("connected.software_update", &[])),
        move || app.open_software_update(),
    ))
}

/// The Look in use and how many the camera menu shows; opens the Looks tab.
fn looks_row(app: AppState) -> AnyElement {
    let Some(snapshot) = app.looks_snapshot() else {
        return view().into_element();
    };
    let shown = snapshot
        .looks
        .iter()
        .filter(|l| l.enabled)
        .count()
        .to_string();
    let selected = snapshot.looks.iter().find(|l| l.selected);
    let subtitle = match selected {
        Some(look) => tr(
            "cam.looks_active",
            &[("name", &look.name), ("count", &shown)],
        ),
        None => tr("cam.looks_count", &[("count", &shown)]),
    };
    let cover = selected.and_then(|look| look.cover.clone());
    grouped_card(
        view()
            .flex_row()
            .items_center()
            .gap_3()
            .min_h(px(68.0))
            .px_4()
            .role(AccessibilityRole::Button)
            .label(format!("{}, {subtitle}", tr("tab.looks", &[])))
            .on_press(move || app.set_tab(Tab::Looks))
            .active(|s| s.opacity(0.72).bg(move || theme().accent))
            .child(cover.map(|cover| cover_view(&cover, 44.0, false)))
            .child(
                view()
                    .flex_1()
                    .gap(px(2.0))
                    .child(
                        text(tr("tab.looks", &[]))
                            .text_size(px(17.0))
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        text(subtitle)
                            .text_size(px(15.0))
                            .text_color(move || theme().muted_foreground),
                    ),
            )
            .child(
                text("›")
                    .text_size(px(22.0))
                    .text_color(move || theme().muted_foreground),
            ),
    )
    .into_element()
}

fn version_card(app: AppState) -> impl IntoElement {
    let package = app.package();
    let grkit = package
        .as_ref()
        .map(|p| p.installed.clone())
        .unwrap_or_default();
    let firmware = package
        .as_ref()
        .map(|p| p.firmware.clone())
        .unwrap_or_default();
    view()
        .flex_row()
        .gap_2()
        .child(version_tile(grkit, tr("connected.grkit", &[])))
        .child(version_tile(firmware, tr("connected.firmware", &[])))
}

fn version_tile(value: String, label: String) -> impl IntoElement {
    view()
        .flex_1()
        .gap(px(4.0))
        .p_3()
        .rounded(px(8.0))
        .bg(move || theme().muted)
        .child(
            text(value)
                .text_size(px(20.0))
                .font_weight(FontWeight::SEMIBOLD),
        )
        .child(
            text(label)
                .text_size(px(13.0))
                .text_color(move || theme().muted_foreground),
        )
}
