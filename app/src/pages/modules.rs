//! Modules tab: the camera's modules in ADJ order. Every title, value and
//! icon comes from the camera. Edit moves the modules that ADJ can order.

use nsh_base::theme;
use nsh_core::element::{ImageSrc, header::HeaderItem};
use nsh_core::prelude::*;

use crate::domain::modules::{Managed, ModuleRow, Summary, split_for_edit};
use crate::pages::looks::{
    ROW_GAP, RowGestures, drag_handle, edit_button, list, loading_card, offline_card,
    refresh_notice, reorder_list,
};
use crate::pages::recipes::center_card;
use crate::store::{AppState, ModulePart, ModulesBlock, Route, Tab};
use crate::strings::tr;
use crate::ui::ty::text;
use crate::ui::{camera_art, footnote, grouped_card, secondary_button, toggle_loading, token};

/// The least height of a module row; rows grow with their text.
const ROW_HEIGHT: f32 = 60.0;
const ROW_TILE: f32 = 32.0;

pub fn modules_root(app: AppState) -> impl IntoElement {
    let editing = app.modules.editing;
    crate::ui::app_page()
        .title(move || tr("modules.title", &[]))
        .header_large_title(!cfg!(target_os = "android"))
        .header_right_items(move || {
            if !can_edit(app) {
                return Vec::new();
            }
            let (key, title) = if editing.get() {
                ("done", "recipes.done")
            } else {
                ("edit", "recipes.edit")
            };
            vec![
                HeaderItem::new(tr(title, &[]))
                    .key(key)
                    .on_press(move || editing.set(!editing.get())),
            ]
        })
        .header_right(dynamic(move || can_edit(app).then(|| edit_button(editing))))
        .child(
            crate::ui::page_scroll()
                .flex_1()
                .bg(move || token("grouped"))
                .child(
                    view()
                        .px_6()
                        .pt_4()
                        .pb(px(crate::pages::look_pages::TAB_BAR_CLEARANCE))
                        .gap(px(ROW_GAP))
                        .child(dynamic(move || on_camera(app, editing.get()))),
                ),
        )
}

/// Edit shows with at least two modules that can trade places.
fn can_edit(app: AppState) -> bool {
    app.modules_block().is_none()
        && app.modules_writable()
        && app
            .module_rows()
            .is_some_and(|rows| rows.iter().filter(|r| r.state.orderable).count() > 1)
}

/// The art of a state without modules: the camera, faded.
fn faded_camera() -> AnyElement {
    view()
        .opacity(0.35)
        .child(camera_art(crate::domain::CameraModel::Gr3, 180.0, false))
        .into_element()
}

fn message_card(title: String, body: String) -> AnyElement {
    grouped_card(
        view()
            .p_6()
            .gap_3()
            .items_center()
            .child(faded_camera())
            .child(
                text(title)
                    .text_center()
                    .text_size(px(crate::ui::ty::HEADLINE))
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                text(body)
                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                    .line_height(px(22.0))
                    .text_center()
                    .text_color(move || theme().muted_foreground),
            ),
    )
    .into_element()
}

fn on_camera(app: AppState, editing: bool) -> AnyElement {
    let online = app.camera_online();
    if online && let Some(block) = app.modules_block() {
        return match block {
            ModulesBlock::Unsupported => message_card(
                tr("modules.unsupported_title", &[]),
                tr("modules.unsupported_body", &[]),
            ),
            ModulesBlock::AppTooOld => message_card(
                tr("modules.app_old_title", &[]),
                tr("modules.app_old_body", &[]),
            ),
        };
    }
    let rows = app.module_rows();
    if !online {
        return match rows {
            Some(rows) => view()
                .gap(px(ROW_GAP))
                .child(offline_card(app))
                .child(browse_list(app, rows, false))
                .into_element(),
            None => center_card(
                faded_camera(),
                tr("looks.no_camera_title", &[]),
                tr("modules.no_camera_body", &[]),
                tr("looks.connect", &[]),
                move || {
                    app.set_tab(Tab::Camera);
                    app.connect_saved();
                },
            )
            .into_element(),
        };
    }
    let Some(rows) = rows else {
        if app.modules_error().is_some() {
            return grouped_card(
                view()
                    .p_4()
                    .gap_3()
                    .child(text(tr("modules.read_failed", &[])).text_size(px(crate::ui::ty::BODY)))
                    .child(secondary_button(tr("looks.retry", &[]), move || {
                        app.refresh_modules()
                    })),
            )
            .into_element();
        }
        return loading_card(tr("modules.loading", &[])).into_element();
    };
    if rows.is_empty() {
        return message_card(
            tr("modules.empty_title", &[]),
            tr("modules.empty_body", &[]),
        );
    }
    let notice = dynamic(move || {
        app.modules_error().map(|_| {
            refresh_notice(app.modules_loading(), move || app.refresh_modules()).into_element()
        })
    });
    let content = if editing && can_edit(app) {
        edit_list(app, rows)
    } else {
        view()
            .gap_2()
            .child(browse_list(app, rows, app.modules_writable()))
            .child(
                (!app.grkit_paused() && !app.safe_mode())
                    .then(|| footnote(tr("modules.list_note", &[]))),
            )
            .into_element()
    };
    view()
        .gap(px(ROW_GAP))
        .child(notice)
        .child(state_notice(app))
        .child(draft_notice(app))
        .child(content)
        .child((!editing).then(|| install_section(app)).flatten())
        .into_element()
}

fn notice_card(title: String, body: Option<String>) -> AnyElement {
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
        .child(body.map(|body| {
            text(body)
                .text_size(px(crate::ui::ty::FOOTNOTE))
                .text_color(move || theme().muted_foreground)
        }))
        .into_element()
}

/// GRKit stops every module.
fn state_notice(app: AppState) -> Option<AnyElement> {
    if app.safe_mode() {
        Some(notice_card(
            tr("modules.safe", &[]),
            Some(tr("modules.safe_body", &[])),
        ))
    } else if app.grkit_paused() {
        Some(notice_card(
            tr("modules.grkit_off", &[]),
            Some(tr("modules.grkit_off_body", &[])),
        ))
    } else {
        None
    }
}

/// The module changes that wait for the next update.
fn draft_notice(app: AppState) -> Option<AnyElement> {
    let draft = app.module_draft();
    if draft.is_empty() {
        return None;
    }
    let action = tr("modules.view_update", &[]);
    Some(
        view()
            .flex_row()
            .items_center()
            .gap_3()
            .p_3()
            .pl_4()
            .rounded(px(10.0))
            .bg(move || theme().muted)
            .child(
                text(tr(
                    "modules.changes",
                    &[("count", &draft.count().to_string())],
                ))
                .flex_1()
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                view()
                    .min_h(px(36.0))
                    .px_4()
                    .rounded_full()
                    .items_center()
                    .justify_center()
                    .bg(move || theme().foreground)
                    .role(AccessibilityRole::Button)
                    .label(action.clone())
                    .on_press(move || app.open_software_update())
                    .active(|s| s.opacity(0.72))
                    .child(
                        text(action)
                            .text_size(px(crate::ui::ty::SUBHEADLINE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(move || theme().background),
                    ),
            )
            .into_element(),
    )
}

fn small_action(title: String, on_press: impl Fn() + 'static) -> AnyElement {
    view()
        .min_h(px(44.0))
        .px_3()
        .items_center()
        .justify_center()
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .on_press(on_press)
        .active(|s| s.opacity(0.72))
        .child(
            text(title)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD),
        )
        .into_element()
}

fn tag(title: String) -> AnyElement {
    view()
        .px_2()
        .py(px(3.0))
        .rounded(px(6.0))
        .bg(move || theme().muted)
        .child(
            text(title)
                .text_size(px(crate::ui::ty::FOOTNOTE))
                .text_color(move || theme().muted_foreground),
        )
        .into_element()
}

/// The official modules that the camera can add.
fn install_section(app: AppState) -> Option<AnyElement> {
    let draft = app.module_draft();
    let modules = app.installable_modules();
    if modules.is_empty() {
        return None;
    }
    let manageable = app.modules_manageable();
    let last = modules.len() - 1;
    let rows = modules
        .into_iter()
        .enumerate()
        .map(|(index, (id, title))| {
            let pending = draft.add.contains(&id);
            let toggle = id.clone();
            let trailing: Option<AnyElement> = if pending {
                Some(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .child(tag(tr("modules.pending_add", &[])))
                        .child(small_action(tr("modules.undo", &[]), move || {
                            app.toggle_module_add(&toggle)
                        }))
                        .into_element(),
                )
            } else {
                manageable.then(|| {
                    small_action(tr("modules.add", &[]), move || {
                        app.toggle_module_add(&toggle)
                    })
                })
            };
            view()
                .flex_row()
                .items_center()
                .gap_3()
                .min_h(px(ROW_HEIGHT))
                .pl_3()
                .pr_1()
                .bg(move || theme().card)
                .when(index != last, |st| {
                    st.border_b_1().border_b_color(move || theme().border)
                })
                .child(module_tile(app, id, ROW_TILE, false))
                .child(
                    text(title)
                        .flex_1()
                        .text_size(px(crate::ui::ty::CALLOUT))
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(trailing)
                .into_element()
        })
        .collect::<Vec<_>>();
    Some(
        view()
            .gap_2()
            .child(
                view()
                    .px_1()
                    .child(crate::ui::section_title(tr("modules.not_installed", &[]))),
            )
            .child(list(rows))
            .into_element(),
    )
}

/// The actions of a module, on a long press.
fn module_menu(app: AppState, row: ModuleRow) {
    let id = row.id().to_owned();
    let on = row.state.enabled;
    let can_delete = !row.state.custom && app.modules_manageable();
    let pending = app.module_draft().remove.contains(&id);
    let can_toggle = app.modules_editable() && !row.paused() && !row.safe();
    spawn(async move {
        let mut menu = crate::ui::held_menu(row.title().to_owned());
        menu = menu.action("open", tr("modules.open", &[]));
        if can_toggle {
            menu = menu.action(
                "toggle",
                tr(
                    if on {
                        "modules.turn_off"
                    } else {
                        "modules.turn_on"
                    },
                    &[],
                ),
            );
        }
        if can_delete {
            menu = if pending {
                menu.action("restore", tr("modules.undo", &[]))
            } else {
                menu.destructive("delete", tr("modules.delete", &[]))
            };
        }
        let answer = menu.cancel("cancel", tr("looks.cancel", &[])).show().await;
        match answer.ok().and_then(|a| a.action).as_deref() {
            Some("open") => app.push(Route::ModuleDetail(id.clone())),
            Some("toggle") => app.set_module_enabled(&id, !on),
            Some("delete" | "restore") => app.toggle_module_remove(&id),
            _ => {}
        }
    });
}

fn browse_list(app: AppState, rows: Vec<ModuleRow>, live: bool) -> AnyElement {
    let last = rows.len().saturating_sub(1);
    list(
        rows.into_iter()
            .enumerate()
            .map(|(index, row)| {
                let trailing = switch(app, &row, live);
                module_row(app, row, trailing, index == last, true).into_element()
            })
            .collect(),
    )
    .into_element()
}

/// Edit: the orderable modules move; the others show their place.
fn edit_list(app: AppState, rows: Vec<ModuleRow>) -> AnyElement {
    let (before, movable, after) = split_for_edit(rows);
    let fixed = |rows: Vec<ModuleRow>| {
        (!rows.is_empty()).then(|| {
            let last = rows.len() - 1;
            list(
                rows.into_iter()
                    .enumerate()
                    .map(|(index, row)| {
                        let place = place_label(&row);
                        module_row(
                            app,
                            row,
                            text(place)
                                .text_size(px(crate::ui::ty::FOOTNOTE))
                                .text_color(move || theme().muted_foreground)
                                .into_element(),
                            index == last,
                            false,
                        )
                        .into_element()
                    })
                    .collect(),
            )
        })
    };
    let count = movable.len();
    let cells = std::cell::RefCell::new(movable.into_iter().map(Some).collect::<Vec<_>>());
    let moving = reorder_list(
        count,
        app.modules_editable() && count > 1,
        move |from, to| app.move_module(from, to),
        move |index, gestures: Option<RowGestures>| {
            let row = cells.borrow_mut()[index].take().expect("each row once");
            let handle = drag_handle(row.title().to_owned(), gestures).into_element();
            module_row(app, row, handle, index + 1 == count, false).into_element()
        },
    );
    view()
        .gap_2()
        .child(fixed(before))
        .child(moving)
        .child(fixed(after))
        .child(footnote(tr("modules.edit_note", &[])))
        .into_element()
}

fn place_label(row: &ModuleRow) -> String {
    match row.state.place.as_str() {
        "toolbar" => tr("modules.place_toolbar", &[]),
        "hidden" => tr("modules.place_hidden", &[]),
        _ => String::new(),
    }
}

/// The switch of a row; a module that failed to load has none.
fn switch(app: AppState, row: &ModuleRow, live: bool) -> AnyElement {
    if row.failed() || row.safe() {
        return view().into_element();
    }
    let id = row.id().to_owned();
    if app.module_draft().remove.contains(&id) {
        let toggle = id.clone();
        return view()
            .flex_row()
            .items_center()
            .gap_1()
            .child(tag(tr("modules.pending_remove", &[])))
            .child(small_action(tr("modules.undo", &[]), move || {
                app.toggle_module_remove(&toggle)
            }))
            .into_element();
    }
    let on = row.state.enabled;
    let paused = row.paused();
    let custom = row.state.custom;
    let title = row.title().to_owned();
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(custom.then(|| tag(tr("modules.custom", &[]))))
        .child(dynamic(move || {
            let toggle_id = id.clone();
            toggle_loading(
                on,
                title.clone(),
                live && !paused && app.modules_editable(),
                app.module_part_busy(&id, &ModulePart::Enabled),
                move || app.set_module_enabled(&toggle_id, !on),
            )
        }))
        .into_element()
}

/// The one line under a module's title.
pub(crate) fn summary_text(app: AppState, row: &ModuleRow) -> String {
    if row.failed() {
        return tr("modules.failed", &[]);
    }
    if row.safe() || row.paused() {
        return String::new();
    }
    if row.managed().is_some() && !row.state.enabled {
        return tr("modules.off", &[]);
    }
    if row.needs_newer_app() {
        return tr("modules.needs_app", &[]);
    }
    match row.managed() {
        Some(Managed::Looks) => {
            // No Look in use means Off; an unread list says nothing.
            return app
                .looks_snapshot()
                .or_else(|| app.offline_looks())
                .map_or_else(String::new, |s| {
                    s.looks
                        .into_iter()
                        .find(|look| look.selected)
                        .map_or_else(|| tr("modules.not_used", &[]), |look| look.name)
                });
        }
        Some(Managed::Recipes) => {
            // Settings that match no recipe are not an Off value.
            return app
                .recipes_snapshot()
                .or_else(|| app.offline_recipes())
                .and_then(|s| s.recipes.into_iter().find(|r| r.current))
                .map_or_else(String::new, |item| app.recipe_name(&item));
        }
        None => {}
    }
    match row.summary() {
        Summary::Value(value) => value,
        Summary::NotUsed => tr("modules.not_used", &[]),
        Summary::None => String::new(),
    }
}

/// The module's icon on a neutral tile. A module without an icon shows the
/// first letter of its title.
pub(crate) fn module_tile(app: AppState, id: String, size: f32, faded: bool) -> AnyElement {
    let initial = app
        .module_title(&id)
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default();
    view()
        .size(px(size))
        .rounded(px(size * 0.22))
        .bg(rgb(0x353536))
        .items_center()
        .justify_center()
        .opacity(if faded { 0.45 } else { 1.0 })
        .child(dynamic(move || {
            match app.module_icon(&id) {
                Some(uri) => image(ImageSrc::from(uri))
                    .size(px(size * 0.875))
                    .object_fit(ResizeMode::Contain)
                    .into_element(),
                // Drawn on a fixed tile, as a cover is.
                None => text(initial.clone())
                    .allow_font_scaling(false)
                    .text_size(px(size * 0.45))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0xffffff))
                    .into_element(),
            }
        }))
        .into_element()
}

fn module_row(
    app: AppState,
    row: ModuleRow,
    trailing: AnyElement,
    last: bool,
    opens: bool,
) -> impl IntoElement {
    let summary = summary_text(app, &row);
    let title = row.title().to_owned();
    let id = row.id().to_owned();
    let off = !row.state.enabled;
    let pressable = opens && !row.failed() && !row.safe();
    let menu_row = row.clone();
    let label = if summary.is_empty() {
        title.clone()
    } else {
        format!("{title}, {summary}")
    };
    let route = Route::ModuleDetail(id.clone());
    view()
        .flex_row()
        .items_center()
        .min_h(px(ROW_HEIGHT))
        .pr_3()
        .gap_2()
        .bg(move || theme().card)
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .child(
            view()
                .flex_1()
                .min_h(px(ROW_HEIGHT))
                .py_1()
                .pl_3()
                .flex_row()
                .items_center()
                .gap_3()
                .role(AccessibilityRole::Button)
                .label(label)
                .when(pressable, |v| {
                    v.on_press(move || app.push(route.clone()))
                        .on_long_press(move || module_menu(app, menu_row.clone()))
                        .active(|s| s.opacity(0.72))
                })
                .child(module_tile(app, id, ROW_TILE, off))
                .child(
                    view()
                        .flex_1()
                        .gap(px(1.0))
                        .child(
                            text(title)
                                .text_size(px(crate::ui::ty::CALLOUT))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(move || {
                                    if off {
                                        theme().muted_foreground
                                    } else {
                                        theme().foreground
                                    }
                                }),
                        )
                        .child((!summary.is_empty()).then(|| {
                            text(summary)
                                .number_of_lines(1)
                                .text_size(px(crate::ui::ty::FOOTNOTE))
                                .text_color(move || theme().muted_foreground)
                        })),
                ),
        )
        .child(trailing)
}
