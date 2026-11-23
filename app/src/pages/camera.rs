//! Camera tab: the first camera to add, the saved cameras, and the
//! connected camera with its update, Looks and recipes. Settings opens
//! from the top right.

use nsh_base::{alert_dialog, theme};
use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use crate::ui::ty::text;

use crate::domain::{CameraModel, RememberedCamera};
use crate::store::{AppState, Route, Tab, is_up_to_date};
use crate::strings::tr;
use crate::ui::{
    busy_state, camera_art, chevron, circle_button, connection_button, cover_view, gr_mark,
    grouped_card, loading_content, quiet_spinner, secondary_button, section_title, token,
    touch_target, wordmark,
};

/// The height of the wordmark at the top of the disconnected camera tab.
const WORDMARK_HEIGHT: f32 = 26.0;
/// A gear: Settings.
const SETTINGS_ICON: &str = "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2zM15 12a3 3 0 1 1-6 0a3 3 0 0 1 6 0";
/// Three dots: more actions.
const MORE_ICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="currentColor"><circle cx="5" cy="12" r="1.9"/><circle cx="12" cy="12" r="1.9"/><circle cx="19" cy="12" r="1.9"/></svg>"##;

pub fn camera_root(app: AppState) -> impl IntoElement {
    // The tab draws the wordmark itself, connected or not.
    crate::ui::app_page()
        .title(move || tr("cam.title", &[]))
        .header_visible(false)
        .child(dynamic(move || {
            if app.initial_connect() {
                return connecting_home(app).into_element();
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

/// The wordmark row at the top of the camera tab, with Settings at its
/// end. A spacer of the button's width keeps the wordmark centered.
fn brand_header(app: AppState) -> impl IntoElement {
    view()
        .flex_row()
        .min_h(px(44.0))
        .items_center()
        .justify_between()
        .child(view().size(px(touch_target())))
        .child(wordmark(WORDMARK_HEIGHT))
        .child(circle_button(
            SETTINGS_ICON,
            tr("settings.title", &[]),
            true,
            move || app.push(Route::Settings),
        ))
}

fn connecting_home(app: AppState) -> impl IntoElement {
    crate::ui::page_scroll()
        .flex_1()
        .bg(move || token("grouped"))
        .child(
            view()
                .px_6()
                .pt(px(72.0))
                .gap_4()
                .child(brand_header(app))
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
                        .text_size(px(crate::ui::ty::TITLE2))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(status),
        )
        .child(buttons)
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
                .child(brand_header(app))
                .child(camera_card(
                    CameraModel::Gr3,
                    tr("home.title", &[]),
                    muted_line(tr("connecting.bt", &[])).into_element(),
                    connection_button(tr("home.add", &[]), move || app.open_scan()).into_element(),
                    None,
                )),
        )
}

fn muted_line(copy: String) -> impl IntoElement {
    text(copy)
        .text_size(px(crate::ui::ty::SUBHEADLINE))
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
        let working = if app.camera_restarting() {
            Some(tr("restart.running_title", &[]))
        } else if app.auto_connecting() {
            Some(tr("cam.auto_connecting", &[]))
        } else {
            None
        };
        if let Some(status) = working {
            return view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(quiet_spinner(14.0))
                .child(muted_line(status))
                .into_element();
        }
        if seen_nearby(app, &camera) {
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    view()
                        .size(px(7.0))
                        .rounded_full()
                        .bg(move || token("online-dot")),
                )
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
                .child(brand_header(app))
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
                        .text_size(px(crate::ui::ty::HEADLINE))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    text(detail)
                        .text_size(px(crate::ui::ty::FOOTNOTE))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(if connected {
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    view()
                        .size(px(7.0))
                        .rounded_full()
                        .bg(move || token("online-dot")),
                )
                .child(
                    text(tr("cam.connected", &[]))
                        .text_size(px(crate::ui::ty::FOOTNOTE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || token("online")),
                )
                .into_element()
        } else if busy {
            view().into_element()
        } else {
            text(tr("nearby.connect", &[]))
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .text_color(move || token("link"))
                .into_element()
        })
}

/// The long-press menu of a saved camera: connect, or remove it.
pub fn camera_menu(app: AppState, camera: RememberedCamera) {
    let name = format!("RICOH {}", camera.model.display_name());
    spawn(async move {
        let mut sheet = crate::ui::held_menu(name.clone());
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
                .child(brand_header(app))
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
                                                .text_size(px(crate::ui::ty::TITLE3))
                                                .font_weight(FontWeight::SEMIBOLD),
                                        )
                                        .child(
                                            text(move || {
                                                app.remembered()
                                                    .map(|r| r.last_ssid)
                                                    .unwrap_or_else(|| tr("connected.status", &[]))
                                            })
                                            .text_size(px(crate::ui::ty::SUBHEADLINE))
                                            .text_color(move || theme().muted_foreground),
                                        ),
                                )
                                .child(dynamic(move || more_button(app))),
                        )
                        .child(dynamic(move || version_card(app))),
                ))
                // Rows come and go, so each brings its own gap. Each has its
                // own dynamic: a list that finishes loading must not redraw
                // the row under a finger.
                .child(
                    view()
                        .mt(px(-16.0))
                        .child(spaced_row(move || update_card(app)))
                        .child(spaced_row(move || current_card(app))),
                ),
        )
}

/// A row of the camera tab with the gap above it, or nothing.
fn spaced_row(row: impl Fn() -> Option<AnyElement> + 'static) -> impl IntoElement {
    dynamic(move || row().map(|row| view().mt_4().child(row)))
}

/// One row about an update: one the phone can install, or one that waits
/// on the camera for its confirmation or a restart. Nothing when GRKit is
/// up to date or the check has no answer.
fn update_card(app: AppState) -> Option<AnyElement> {
    use crate::domain::JournalState;
    let package = app.package();
    let available = package.as_ref().and_then(|p| p.available.clone());
    let version = match app.camera_journal() {
        Some(_) => app.pending_version().unwrap_or_default(),
        None => available.clone().unwrap_or_default(),
    };
    let named = |key: &str, args: &[(&str, &str)]| tr(key, args);
    let (title, subtitle, pending): (String, Box<dyn Fn() -> String>, bool) =
        match app.camera_journal() {
            Some(JournalState::Waiting) => (
                named("cam.update_waiting", &[("version", &version)]),
                Box::new(move || {
                    tr(
                        "cam.update_waiting_sub",
                        &[("time", &app.confirm_deadline_label())],
                    )
                }),
                true,
            ),
            Some(JournalState::Confirmed) => (
                named("cam.update_ready", &[("version", &version)]),
                Box::new(|| tr("cam.update_ready_sub", &[])),
                true,
            ),
            _ if available.is_some() && !package.as_ref().is_some_and(is_up_to_date) => (
                named("cam.update_available", &[("version", &version)]),
                Box::new(|| tr("cam.update_available_sub", &[])),
                false,
            ),
            _ => return None,
        };
    // Without a known version, the title names GRKit alone.
    let title = title.replace("GRKit  ", "GRKit ");
    let card = grouped_card(
        view()
            .flex_row()
            .items_center()
            .gap_3()
            .min_h(px(68.0))
            .px_4()
            .role(AccessibilityRole::Button)
            .label(title.clone())
            .on_press(move || {
                if pending {
                    app.open_pending_update()
                } else {
                    app.open_software_update()
                }
            })
            .active(|s| s.opacity(0.72).bg(move || theme().accent))
            .child(gr_mark(44.0))
            .child(
                view()
                    .flex_1()
                    .gap(px(2.0))
                    .child(
                        text(title)
                            .text_size(px(crate::ui::ty::HEADLINE))
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        text(subtitle)
                            .text_size(px(crate::ui::ty::SUBHEADLINE))
                            .text_color(move || theme().muted_foreground),
                    ),
            )
            .child(chevron()),
    );
    Some(card.into_element())
}

/// What the camera uses now: the recipe that matches it and the Film Look in
/// use. Browsing and switching stay on their tabs, so a row opens the detail.
fn current_card(app: AppState) -> Option<AnyElement> {
    let recipes = app.recipes_snapshot();
    let looks = app.looks_snapshot();
    let mut rows = Vec::new();
    match &recipes {
        Some(snapshot) => {
            if let Some(item) = snapshot.recipes.iter().find(|r| r.current) {
                let id = item.id;
                rows.push(current_row(
                    crate::pages::recipes::recipe_tile(app, item, 44.0, false),
                    app.recipe_name(item),
                    tr("tab.recipes", &[]),
                    move || open_detail(app, Tab::Recipes, Route::RecipeDetail(id)),
                ));
            }
        }
        None => rows.extend(unread_row(
            app,
            app.recipes_loading(),
            app.recipes_error().is_some(),
            "tab.recipes",
            "recipes.loading",
            "recipes.read_failed",
            Tab::Recipes,
        )),
    }
    match &looks {
        Some(snapshot) => {
            if let Some(look) = snapshot.looks.iter().find(|l| l.selected) {
                let id = look.id.clone();
                rows.push(current_row(
                    cover_view(&look.shown_cover(), 44.0, false),
                    look.name.clone(),
                    tr("tab.looks", &[]),
                    move || open_detail(app, Tab::Looks, Route::LookDetail(id.clone())),
                ));
            }
        }
        None => rows.extend(unread_row(
            app,
            app.looks_loading(),
            app.looks_error().is_some(),
            "tab.looks",
            "looks.loading",
            "looks.read_failed",
            Tab::Looks,
        )),
    }
    if rows.is_empty() {
        if recipes.is_none() || looks.is_none() {
            return None;
        }
        rows.push(
            view()
                .min_h(px(52.0))
                .justify_center()
                .px_4()
                .child(
                    text(tr("cam.current_none", &[]))
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                        .text_color(move || theme().muted_foreground),
                )
                .into_element(),
        );
    }
    let rows = rows.into_iter().enumerate().map(|(index, row)| {
        view()
            .when(index > 0, |v| {
                v.border_t_1().border_color(move || theme().border)
            })
            .child(row)
    });
    Some(
        view()
            .gap_2()
            .child(section_title(tr("cam.current", &[])))
            .child(grouped_card(view().children(rows)))
            .into_element(),
    )
}

/// Open a detail page on its tab, over the tab's list.
fn open_detail(app: AppState, tab: Tab, route: Route) {
    app.pop_to(tab, 0);
    app.set_tab(tab);
    app.push(route);
}

/// The list of a tab that the phone has not read: it is being read, or the
/// read failed and the tab offers another try.
fn unread_row(
    app: AppState,
    loading: bool,
    failed: bool,
    title: &'static str,
    reading: &'static str,
    read_failed: &'static str,
    tab: Tab,
) -> Option<AnyElement> {
    let (tile, subtitle) = if loading {
        let tile = view()
            .size(px(44.0))
            .items_center()
            .justify_center()
            .child(quiet_spinner(20.0))
            .into_element();
        (tile, tr(reading, &[]))
    } else if failed {
        (view().size(px(44.0)).into_element(), tr(read_failed, &[]))
    } else {
        return None;
    };
    Some(current_row(tile, tr(title, &[]), subtitle, move || {
        app.set_tab(tab)
    }))
}

/// A row of the card: a tile, a name, what it is.
fn current_row(
    tile: AnyElement,
    title: String,
    subtitle: String,
    on_press: impl Fn() + 'static,
) -> AnyElement {
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .min_h(px(68.0))
        .px_4()
        .py_2()
        .role(AccessibilityRole::Button)
        .label(format!("{subtitle}, {title}"))
        .on_press(on_press)
        .active(|s| s.opacity(0.72).bg(move || theme().accent))
        .child(tile)
        .child(
            view()
                .flex_1()
                .gap(px(2.0))
                .child(
                    text(title)
                        .text_size(px(crate::ui::ty::HEADLINE))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    text(subtitle)
                        .number_of_lines(2)
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(chevron())
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
    let grkit_label = tr("connected.grkit", &[]);
    // The GRKit tile opens Software Update and says so with a chevron.
    view()
        .flex_row()
        .gap_2()
        .child(
            version_tile(grkit.clone(), grkit_label.clone())
                .flex_row()
                .items_center()
                .justify_between()
                .role(AccessibilityRole::Button)
                .label(format!("{grkit_label} {grkit}"))
                .on_press(move || app.open_software_update())
                .active(|s| s.opacity(0.72))
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        // Its own dynamic: a check that ends must not redraw
                        // the tile under a finger.
                        .child(dynamic(move || {
                            app.release_checking().then(|| quiet_spinner(14.0))
                        }))
                        .child(chevron()),
                ),
        )
        .child(version_tile(firmware, tr("connected.firmware", &[])))
}

/// A version and what it is the version of.
fn version_tile(value: String, label: String) -> View {
    view()
        .flex_1()
        .p_3()
        .rounded(px(8.0))
        .border_1()
        .border_color(move || theme().border)
        .child(
            view()
                .gap(px(4.0))
                .child(
                    text(value)
                        .text_size(px(crate::ui::ty::TITLE3))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    text(label)
                        .text_size(px(crate::ui::ty::FOOTNOTE))
                        .text_color(move || theme().muted_foreground),
                ),
        )
}

/// The connected camera's other actions, in the system menu. It works in
/// place while the camera turns off.
fn more_button(app: AppState) -> impl IntoElement {
    let working = app.powering_off();
    view()
        .size(px(touch_target()))
        .items_center()
        .justify_center()
        .role(AccessibilityRole::Button)
        .label(tr("cam.more", &[]))
        .accessibility_state(busy_state(working))
        .when(!working, |v| {
            v.on_press(move || connected_menu(app))
                .active(|s| s.opacity(0.6))
        })
        .child(loading_content(
            working,
            20.0,
            theme().foreground,
            view()
                .text_color(move || theme().foreground)
                .child(svg(ImageSrc::svg_markup(MORE_ICON)).size(px(22.0))),
        ))
}

/// Disconnect and turn off the connected camera, both after a confirm.
fn connected_menu(app: AppState) {
    let name = format!("RICOH {}", app.model_name());
    spawn(async move {
        let answer = alert_dialog()
            .title(name)
            .action_sheet()
            .action("disconnect", tr("cam.disconnect", &[]))
            .destructive("power", tr("cam.power", &[]))
            .cancel("cancel", tr("cam.cancel", &[]))
            .show()
            .await;
        match answer.ok().and_then(|a| a.action).as_deref() {
            Some("disconnect") => crate::pages::confirm_disconnect(app),
            Some("power") => crate::pages::confirm_power_off(app),
            _ => {}
        }
    });
}
