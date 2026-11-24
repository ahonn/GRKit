//! Camera tab: the first camera to add, the saved cameras, and the
//! connected camera with its update, modules, Looks and recipes.

use nsh_base::{alert_dialog, theme};
use nsh_core::element::ImageSrc;
use nsh_core::element::desktop::MenuItem;
use nsh_core::element::header::HeaderItem;
use nsh_core::prelude::*;

use crate::pages::look_pages::TAB_BAR_CLEARANCE;
use crate::ui::ty::text;

use crate::domain::{CameraModel, RememberedCamera};
use crate::store::{AppState, Route, Tab, is_up_to_date};
use crate::strings::tr;
use crate::ui::{
    camera_art, chevron, connection_button, cover_view, grouped_card, header_busy, header_buttons,
    header_icon, quiet_spinner, secondary_button, section_title, token, wordmark,
};

/// The height of the wordmark at the start of the camera tab's header.
const WORDMARK_HEIGHT: f32 = 26.0;
/// Three dots: More.
const MORE_PATH: &str = "M4 12a1 1 0 1 0 2 0a1 1 0 1 0-2 0M11 12a1 1 0 1 0 2 0a1 1 0 1 0-2 0M18 12a1 1 0 1 0 2 0a1 1 0 1 0-2 0";

pub fn camera_root(app: AppState) -> impl IntoElement {
    // The header of the other tabs, with the wordmark in place of the title.
    let page = crate::ui::app_page()
        .title(move || tr("cam.title", &[]))
        .header_title(view());
    // iOS 26 draws a bar item on glass, as a button: there the wordmark
    // heads the page instead, where the other tabs show their large title.
    let page = if cfg!(target_os = "ios") {
        page
    } else {
        page.header_left(wordmark(WORDMARK_HEIGHT))
    };
    page.header_right_items(move || native_header(app))
        .header_right(dynamic(move || header_actions(app)))
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

/// The wordmark at the head of the camera tab on iOS, in the place and at
/// the margin of a large title.
fn home_wordmark() -> Option<impl IntoElement> {
    cfg!(target_os = "ios").then(|| view().px_4().pt_2().child(wordmark(WORDMARK_HEIGHT)))
}

fn connecting_home() -> impl IntoElement {
    crate::ui::page_scroll()
        .top_of(Some(crate::store::Tab::Camera.key().to_owned()))
        .flex_1()
        .bg(move || token("grouped"))
        .child(home_wordmark())
        .child(
            view()
                .px_6()
                .pt_4()
                .gap_4()
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
        .top_of(Some(crate::store::Tab::Camera.key().to_owned()))
        .flex_1()
        .bg(move || token("connection-background"))
        .child(home_wordmark())
        .child(
            view()
                .px_6()
                .pt_4()
                .pb(px(TAB_BAR_CLEARANCE))
                .gap_6()
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
        .top_of(Some(crate::store::Tab::Camera.key().to_owned()))
        .flex_1()
        .bg(move || token("connection-background"))
        .child(home_wordmark())
        .child(
            view()
                .px_6()
                .pt_4()
                .pb(px(TAB_BAR_CLEARANCE))
                .gap_6()
                .child(camera_card(
                    camera.model,
                    format!("RICOH {}", camera.model.display_name()),
                    presence_line(app, camera.clone()).into_element(),
                    view()
                        .w_full()
                        .gap_3()
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
        .top_of(Some(crate::store::Tab::Camera.key().to_owned()))
        .refresh(Some(crate::store::page_refresh::CAMERA.to_owned()))
        .flex_1()
        .bg(move || token("grouped"))
        .child(home_wordmark())
        .child(
            view()
                .gap_4()
                .pt_4()
                .px_6()
                .pb(px(TAB_BAR_CLEARANCE))
                .child(grouped_card(
                    view().p_4().gap_4().child(
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
                                        text(tr("connected.status", &[]))
                                            .text_size(px(crate::ui::ty::SUBHEADLINE))
                                            .text_color(move || theme().muted_foreground),
                                    )
                                    .child(
                                        text(move || version_line(app))
                                            .text_size(px(crate::ui::ty::FOOTNOTE))
                                            .text_color(move || theme().muted_foreground),
                                    ),
                            ),
                    ),
                ))
                // Rows come and go, so each brings its own gap. Each has its
                // own dynamic: a list that finishes loading must not redraw
                // the row under a finger.
                .child(
                    view()
                        .mt(px(-16.0))
                        .child(spaced_row(move || update_card(app)))
                        .child(spaced_row(move || modules_card(app)))
                        .child(spaced_row(move || current_card(app))),
                ),
        )
}

/// A row of the camera tab with the gap above it, or nothing.
fn spaced_row(row: impl Fn() -> Option<AnyElement> + 'static) -> impl IntoElement {
    dynamic(move || row().map(|row| view().mt_4().child(row)))
}

/// A line of text that follows the state it reads.
type Line = Box<dyn Fn() -> String>;

/// One row about an update: one the phone can install, or one that waits
/// on the camera for its confirmation or a restart. Up to date, it still
/// opens Software Update; nothing before the camera's package is read.
fn update_card(app: AppState) -> Option<AnyElement> {
    use crate::domain::JournalState;
    let package = app.package();
    let available = package.as_ref().and_then(|p| p.available.clone());
    let version = match app.camera_journal() {
        Some(_) => app.pending_version().unwrap_or_default(),
        None => available.clone().unwrap_or_default(),
    };
    let named = |key: &str, args: &[(&str, &str)]| tr(key, args);
    let (title, subtitle, pending): (String, Option<Line>, bool) = match app.camera_journal() {
        Some(JournalState::Waiting) => (
            named("cam.update_waiting", &[("version", &version)]),
            Some(Box::new(|| tr("cam.update_waiting_sub", &[]))),
            true,
        ),
        Some(JournalState::Confirmed) => (
            named("cam.update_ready", &[("version", &version)]),
            Some(Box::new(|| tr("cam.update_ready_sub", &[]))),
            true,
        ),
        _ if available.is_some() && !package.as_ref().is_some_and(is_up_to_date) => (
            named("cam.update_available", &[("version", &version)]),
            Some(Box::new(|| tr("cam.update_available_sub", &[]))),
            false,
        ),
        _ if !app.module_draft().is_empty()
            && package
                .as_ref()
                .is_some_and(|p| p.available.as_deref() == Some(p.installed.as_str())) =>
        {
            (
                named(
                    "modules.changes",
                    &[("count", &app.module_draft().count().to_string())],
                ),
                Some(Box::new(|| tr("modules.view_update", &[]))),
                false,
            )
        }
        // Up to date, or no answer from the release check yet: the row
        // still opens Software Update, and says only what is known.
        _ if package.is_some() => {
            let known =
                package.as_ref().is_some_and(is_up_to_date) && app.release_problem().is_none();
            let subtitle: Option<Line> =
                known.then(|| Box::new(|| tr("software.up_to_date", &[])) as Line);
            (tr("software.title", &[]), subtitle, false)
        }
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
            .child(
                view()
                    .flex_1()
                    .gap(px(2.0))
                    .child(
                        text(title)
                            .text_size(px(crate::ui::ty::HEADLINE))
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(subtitle.map(|subtitle| {
                        text(subtitle)
                            .text_size(px(crate::ui::ty::SUBHEADLINE))
                            .text_color(move || theme().muted_foreground)
                    })),
            )
            // Its own dynamic: a check that ends must not redraw the row
            // under a finger.
            .child(dynamic(move || {
                app.release_checking().then(|| quiet_spinner(14.0))
            }))
            .child(chevron()),
    );
    Some(card.into_element())
}

/// What the camera uses now: the recipe that matches it and the Film Look in
/// use. Browsing and switching stay on their tabs, so a row opens the detail.
fn current_card(app: AppState) -> Option<AnyElement> {
    if app.grkit_paused() || app.safe_mode() {
        let (title, body) = if app.safe_mode() {
            ("modules.safe", "modules.safe_body")
        } else {
            ("modules.grkit_off", "modules.grkit_off_body")
        };
        return Some(status_notice(tr(title, &[]), tr(body, &[])));
    }
    let (recipes_on, looks_on) = (app.recipes_available(), app.looks_available());
    let recipes = app.recipes_snapshot();
    let looks = app.looks_snapshot();
    if !recipes_on && !looks_on {
        return None;
    }
    let mut rows = Vec::new();
    match &recipes {
        _ if !recipes_on => {}
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
        _ if !looks_on => {}
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
        if (recipes_on && recipes.is_none()) || (looks_on && looks.is_none()) {
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
            .child(grouped_card(view().children(rows)))
            .into_element(),
    )
}

/// The Modules row in a card of its own: the way to the Modules page. GRKit
/// off and safe mode stop every module, and the current card says so.
fn modules_card(app: AppState) -> Option<AnyElement> {
    if app.grkit_paused() || app.safe_mode() {
        return None;
    }
    modules_row(app).map(|row| grouped_card(row).into_element())
}

/// The modules that run, with their values. It opens the Modules page.
fn modules_row(app: AppState) -> Option<AnyElement> {
    let Some(rows) = app.module_rows() else {
        return Some(unread_modules_row(app));
    };
    let on: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.state.enabled
                && row.managed().is_none()
                && !row.failed()
                && !matches!(row.summary(), crate::domain::modules::Summary::NotUsed)
        })
        .collect();
    // The count of the modules in use, and their names on one line.
    let (title, subtitle) = if on.is_empty() {
        let count = rows.iter().filter(|row| row.state.enabled).count();
        (
            tr("modules.title", &[]),
            tr("modules.on_count", &[("count", &count.to_string())]),
        )
    } else {
        (
            tr("modules.in_use_count", &[("count", &on.len().to_string())]),
            on.iter()
                .map(|row| row.title())
                .collect::<Vec<_>>()
                .join(" · "),
        )
    };
    Some(current_row_lines(
        modules_tile(),
        title,
        subtitle,
        1,
        move || app.open_modules_page(),
    ))
}

/// The Modules row before the phone has the modules. It is the only way to
/// the Modules page, so it shows while they are read, after a failed read,
/// and when the camera cannot serve them.
fn unread_modules_row(app: AppState) -> AnyElement {
    use crate::store::ModulesBlock;
    let (loading, subtitle) = match app.modules_block() {
        Some(ModulesBlock::Unsupported) => (false, "modules.unsupported_title"),
        Some(ModulesBlock::AppTooOld) => (false, "modules.app_old_title"),
        None if app.modules_error().is_some() => (false, "modules.read_failed"),
        None => (true, ""),
    };
    let tile = if loading {
        view()
            .size(px(44.0))
            .items_center()
            .justify_center()
            .child(quiet_spinner(20.0))
            .into_element()
    } else {
        modules_tile()
    };
    // While loading, the spinner tile is the only sign; a screen reader
    // hears that the modules load.
    let subtitle = if loading {
        String::new()
    } else {
        tr(subtitle, &[])
    };
    let title = tr("modules.title", &[]);
    let label = if loading {
        format!("{}, {title}", tr("modules.loading", &[]))
    } else {
        format!("{subtitle}, {title}")
    };
    current_row_labeled(tile, title, subtitle, 2, label, move || {
        app.open_modules_page()
    })
}

/// The Modules page's sliders on the dark tile of a module.
fn modules_tile() -> AnyElement {
    const SLIDERS: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" stroke-width="1.9" stroke-linecap="round"><path d="M4 7h9M17 7h3M4 17h3M11 17h9"/><circle cx="15" cy="7" r="2.2"/><circle cx="9" cy="17" r="2.2"/></svg>"##;
    view()
        .size(px(44.0))
        .rounded(px(44.0 * 0.22))
        .bg(rgb(0x353536))
        .items_center()
        .justify_center()
        .child(svg(ImageSrc::svg_markup(SLIDERS)).size(px(26.0)))
        .into_element()
}

/// A state of GRKit that stops every module, in place of the current items.
fn status_notice(title: String, body: String) -> AnyElement {
    view()
        .gap(px(2.0))
        .p_4()
        .rounded(px(10.0))
        .bg(move || theme().muted)
        .child(
            text(title)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD),
        )
        .child(
            text(body)
                .text_size(px(crate::ui::ty::FOOTNOTE))
                .text_color(move || theme().muted_foreground),
        )
        .into_element()
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
    current_row_lines(tile, title, subtitle, 2, on_press)
}

/// A row of the current items whose subtitle takes at most `lines` lines.
fn current_row_lines(
    tile: AnyElement,
    title: String,
    subtitle: String,
    lines: u32,
    on_press: impl Fn() + 'static,
) -> AnyElement {
    let label = format!("{subtitle}, {title}");
    current_row_labeled(tile, title, subtitle, lines, label, on_press)
}

/// A current-items row with its own screen reader label. An empty subtitle
/// shows no line.
fn current_row_labeled(
    tile: AnyElement,
    title: String,
    subtitle: String,
    lines: u32,
    label: String,
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
        .label(label)
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
                .child((!subtitle.is_empty()).then(|| {
                    text(subtitle)
                        .number_of_lines(lines)
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                        .text_color(move || theme().muted_foreground)
                })),
        )
        .child(chevron())
        .into_element()
}

/// The versions on the camera, as text: Software Update opens from the
/// update row.
fn version_line(app: AppState) -> String {
    let Some(package) = app.package() else {
        return String::new();
    };
    format!(
        "{} {} · {} {}",
        tr("connected.grkit", &[]),
        package.installed,
        tr("connected.firmware", &[]),
        package.firmware
    )
}

/// More in the header (iOS): a native menu with Disconnect and Turn Off,
/// each after a confirm. It stays in place, disabled, while the camera
/// turns off.
fn native_header(app: AppState) -> Vec<HeaderItem> {
    if !app.camera_online() {
        return Vec::new();
    }
    vec![
        HeaderItem::symbol("ellipsis.circle")
            .key("more")
            .label(tr("cam.more", &[]))
            .disabled(app.powering_off())
            .menu([
                MenuItem::new(tr("cam.disconnect", &[]))
                    .symbol("wifi.slash")
                    .on_select(move || crate::pages::confirm_disconnect(app)),
                MenuItem::new(tr("cam.power", &[]))
                    .symbol("power")
                    .destructive(true)
                    .on_select(move || crate::pages::confirm_power_off(app)),
            ]),
    ]
}

/// More in the header (Android): Disconnect and Turn Off in a sheet. A
/// spinner takes its place while the camera turns off.
fn header_actions(app: AppState) -> Option<AnyElement> {
    if !app.camera_online() {
        return None;
    }
    let more = if app.powering_off() {
        header_busy(tr("cam.more", &[]))
    } else {
        header_icon(MORE_PATH, tr("cam.more", &[]), true, move || {
            connected_menu(app)
        })
    };
    Some(header_buttons([more]).into_element())
}

/// Disconnect and Turn Off, both after a confirm.
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
