//! Modules page: the camera's modules in ADJ order. Every title, value and
//! icon comes from the camera. Edit moves the modules that ADJ can order.

use nsh_core::element::header::HeaderItem;
use nsh_core::prelude::*;

use crate::domain::modules::{
    Managed, ModuleLayout, ModulePlace, ModuleRow, Summary, split_for_edit,
};
use crate::pages::looks::{ROW_GAP, loading_card, no_camera, offline_card, refresh_notice};
use crate::store::{AppState, ModulePart, ModulesBlock, Route};
use crate::strings::tr;
use crate::ui::PageTitle;
use crate::ui::ty::text;
use crate::ui::{
    ButtonKind, ButtonSize, ChipTone, Choice, Choices, ListRow, RowGestures, button, check_circle,
    chip, drag_handle, empty_state, glyph, header_buttons, header_text, list_row, list_section,
    notice_card, reorder_list, system_switch, token,
};

/// The side of a module's tile in a row.
const ROW_TILE: f32 = 32.0;

pub fn modules_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    let editing = app.modules.editing;
    crate::ui::app_page()
        .page_title(move || tr("modules.title", &[]), false)
        .on_dismiss(on_dismiss)
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
        .header_right(dynamic(move || {
            can_edit(app).then(|| {
                let on = editing.get();
                let title = tr(if on { "recipes.done" } else { "recipes.edit" }, &[]);
                header_buttons([header_text(title, move || editing.set(!on))])
            })
        }))
        .child(
            crate::pages::page_scroll()
                .refresh(Some(crate::store::page_refresh::MODULES.to_owned()))
                .flex_1()
                .bg(move || token("list-background"))
                .child(
                    view()
                        .px_4()
                        .pt_4()
                        .pb(px(crate::ui::TAB_BAR_CLEARANCE))
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

fn on_camera(app: AppState, editing: bool) -> AnyElement {
    let online = app.camera_online();
    if online && let Some(block) = app.modules_block() {
        return match block {
            // The camera's GRKit takes a newer one over Wi-Fi: the way to
            // it is Software Update.
            ModulesBlock::Unsupported => {
                empty_state(glyph::DOWNLOAD, tr("modules.unsupported_title", &[]))
                    .body(tr("modules.unsupported_body", &[]))
                    .action(tr("software.title", &[]), move || {
                        app.open_software_update()
                    })
                    .into_element()
            }
            ModulesBlock::AppTooOld => {
                empty_state(glyph::DOWNLOAD, tr("modules.app_old_title", &[]))
                    .body(tr("modules.app_old_body", &[]))
                    .into_element()
            }
        };
    }
    let rows = app.module_rows();
    if !online {
        return match rows {
            Some(rows) => view()
                .gap(px(ROW_GAP))
                .child(offline_card(app))
                .child(browse_list(app, rows, false, None))
                .into_element(),
            None => no_camera(app, "modules.no_camera_body").into_element(),
        };
    }
    let Some(rows) = rows else {
        if app.modules_error().is_some() {
            return empty_state(glyph::WARNING, tr("modules.read_failed_title", &[]))
                .action(tr("looks.retry", &[]), move || app.refresh_modules())
                .into_element();
        }
        return loading_card(tr("modules.loading", &[])).into_element();
    };
    if rows.is_empty() {
        return empty_state(glyph::SLIDERS, tr("modules.empty_title", &[]))
            .body(tr("modules.empty_body", &[]))
            .into_element();
    }
    let notice = dynamic(move || {
        app.modules_error().map(|_| {
            refresh_notice(app.modules_loading(), move || app.refresh_modules()).into_element()
        })
    });
    let content = if editing && can_edit(app) {
        edit_list(app, rows)
    } else {
        let note = (!app.grkit_paused() && !app.safe_mode()).then(|| tr("modules.list_note", &[]));
        browse_list(app, rows, app.modules_writable(), note)
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

/// GRKit stops every module.
fn state_notice(app: AppState) -> Option<AnyElement> {
    let (title, body) = if app.safe_mode() {
        ("modules.safe", "modules.safe_body")
    } else if app.grkit_paused() {
        ("modules.grkit_off", "modules.grkit_off_body")
    } else {
        return None;
    };
    Some(
        notice_card(tr(title, &[]))
            .subtitle(tr(body, &[]))
            .into_element(),
    )
}

/// The module changes that wait for the next update.
fn draft_notice(app: AppState) -> Option<AnyElement> {
    let draft = app.module_draft();
    if draft.is_empty() {
        return None;
    }
    Some(
        notice_card(tr(
            "modules.changes",
            &[("count", &draft.count().to_string())],
        ))
        .emphasis()
        .action(button(
            tr("modules.view_update", &[]),
            ButtonKind::Primary,
            ButtonSize::Small,
            move || app.open_software_update(),
        ))
        .into_element(),
    )
}

/// A small action on a row, such as Add or Undo.
fn small_action(title: String, on_press: impl Fn() + 'static) -> AnyElement {
    button(title, ButtonKind::Tinted, ButtonSize::Small, on_press).into_element()
}

/// A change that waits for the update, and its Undo.
fn pending_end(title: String, on_undo: impl Fn() + 'static) -> AnyElement {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(chip(title, ChipTone::Pending))
        .child(small_action(tr("modules.undo", &[]), on_undo))
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
    let rows = modules.into_iter().map(|(id, title)| {
        let pending = draft.add.contains(&id);
        let toggle = id.clone();
        let row = list_row(title).leading(module_tile(app, id, ROW_TILE, false));
        if pending {
            row.trailing(pending_end(tr("modules.pending_add", &[]), move || {
                app.toggle_module_add(&toggle)
            }))
        } else if manageable {
            row.trailing(small_action(tr("modules.add", &[]), move || {
                app.toggle_module_add(&toggle)
            }))
        } else {
            row
        }
    });
    Some(
        list_section(rows)
            .header(tr("modules.not_installed", &[]))
            .into_element(),
    )
}

/// The actions of a module, on a long press.
fn module_menu(app: AppState, row: &ModuleRow) -> Choices {
    let id = row.id().to_owned();
    let on = row.state.enabled;
    let can_delete = !row.state.custom && app.modules_manageable();
    let pending = app.module_draft().remove.contains(&id);
    let can_toggle = app.modules_editable() && !row.paused() && !row.safe();
    let mut choices = vec![Choice::new("open", tr("modules.open", &[]))];
    if can_toggle {
        let toggle = if on {
            "modules.turn_off"
        } else {
            "modules.turn_on"
        };
        choices.push(Choice::new("toggle", tr(toggle, &[])));
    }
    if can_delete {
        choices.push(if pending {
            Choice::new("restore", tr("modules.undo", &[]))
        } else {
            Choice::new("delete", tr("modules.delete", &[])).destructive()
        });
    }
    Choices::new(
        row.title().to_owned(),
        choices,
        move |chosen| match chosen {
            "open" => app.push(Route::ModuleDetail(id.clone())),
            "toggle" => app.set_module_enabled(&id, !on),
            "delete" | "restore" => app.toggle_module_remove(&id),
            _ => {}
        },
    )
}

fn browse_list(
    app: AppState,
    rows: Vec<ModuleRow>,
    live: bool,
    note: Option<String>,
) -> AnyElement {
    // A module that failed to load has no page: a note under the list says
    // where the reason is.
    let failed = rows.iter().any(ModuleRow::failed);
    let rows = rows.into_iter().map(|row| {
        let trailing = switch(app, &row, live);
        module_row(app, row, trailing, true)
    });
    let section = list_section(rows);
    let section = match note {
        Some(note) => section.footer(note).into_element(),
        None => section.into_element(),
    };
    if !failed {
        return section;
    }
    view()
        .gap_2()
        .child(section)
        .child(crate::ui::list_note(tr("modules.failed_note", &[])))
        .into_element()
}

/// A row of a list whose rows a handle moves: drawn on its own card fill.
fn moving_row(row: ListRow, last: bool) -> AnyElement {
    view()
        .bg(move || token("list-card"))
        .child(row.into_row(last))
        .into_element()
}

/// Edit: the orderable modules move; the others show their place. A GRKit
/// that takes a whole layout edits the places of ADJ instead.
fn edit_list(app: AppState, rows: Vec<ModuleRow>) -> AnyElement {
    if let Some(layout) = app.module_layout() {
        return layout_edit(app, rows, layout);
    }
    let (before, movable, after) = split_for_edit(rows);
    let fixed = |rows: Vec<ModuleRow>| {
        (!rows.is_empty()).then(|| {
            list_section(rows.into_iter().map(|row| {
                let place = place_label(&row);
                module_row(
                    app,
                    row,
                    text(place)
                        .text_size(px(crate::ui::ty::FOOTNOTE))
                        .text_color(move || token("list-secondary"))
                        .into_element(),
                    false,
                )
            }))
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
            moving_row(module_row(app, row, handle, false), index + 1 == count)
        },
    );
    view()
        .gap_2()
        .child(fixed(before))
        .child(moving)
        .child(fixed(after))
        .child(crate::ui::list_note(tr("modules.edit_note", &[])))
        .into_element()
}

/// The places of ADJ, as MENU > ADJ Order has them: the toolbar, the GRK
/// root, and the hidden modules. A module moves within its place by its
/// handle, and to another place from its menu.
fn layout_edit(app: AppState, rows: Vec<ModuleRow>, layout: ModuleLayout) -> AnyElement {
    let max = app.toolbar_max();
    let full = layout.toolbar.len() >= max;
    let mut content = view().gap_5();
    for place in ModulePlace::ALL {
        let group: Vec<ModuleRow> = layout
            .ids(place)
            .iter()
            .filter_map(|id| rows.iter().find(|row| row.id() == id).cloned())
            .collect();
        if group.is_empty() {
            continue;
        }
        let count = group.len();
        let title = match place {
            ModulePlace::Toolbar => tr(
                "modules.group_toolbar",
                &[("count", &count.to_string()), ("max", &max.to_string())],
            ),
            ModulePlace::Root => tr("modules.group_root", &[]),
            ModulePlace::Hidden => tr("modules.group_hidden", &[]),
        };
        let cells = std::cell::RefCell::new(group.into_iter().map(Some).collect::<Vec<_>>());
        let moving = reorder_list(
            count,
            app.modules_editable() && count > 1,
            move |from, to| app.move_in_place(place, from, to),
            move |index, gestures: Option<RowGestures>| {
                let row = cells.borrow_mut()[index].take().expect("each row once");
                let id = row.id().to_owned();
                let name = row.title().to_owned();
                let handle = drag_handle(name.clone(), gestures).into_element();
                // As Film Looks' Edit: the check shows or hides the module,
                // the row moves it between the top row and the GRK tab.
                let shown = place != ModulePlace::Hidden;
                let check_id = id.clone();
                let check = check_circle(
                    shown,
                    tr(
                        if shown {
                            "modules.move_hidden"
                        } else {
                            "modules.move_root"
                        },
                        &[],
                    ),
                    app.modules_editable(),
                    false,
                    move || {
                        let to = if shown {
                            ModulePlace::Hidden
                        } else {
                            ModulePlace::Root
                        };
                        app.move_to_place(&check_id, to);
                    },
                );
                let summary = summary_text(app, &row);
                let off = !row.state.enabled;
                let leading = view()
                    .flex_row()
                    .items_center()
                    // The check's touch area reaches into the card's margin.
                    .ml(px(-10.0))
                    .gap(px(4.0))
                    .child(check)
                    .child(module_tile(app, id.clone(), ROW_TILE, off));
                let row = list_row(name.clone())
                    .subtitle(summary)
                    .leading(leading)
                    .trailing(handle)
                    .press(move || place_menu(app, id.clone(), name.clone(), place, full));
                moving_row(row, index + 1 == count)
            },
        );
        content = content.child(
            view()
                .gap_2()
                .child(crate::ui::list_note(title).role(AccessibilityRole::Header))
                .child(moving),
        );
    }
    content
        .child(crate::ui::list_note(tr("modules.layout_note", &[])))
        .into_element()
}

/// Move a shown module between the top row and the GRK tab; the check hides
/// it. The toolbar takes no more than its most.
fn place_menu(app: AppState, id: String, title: String, place: ModulePlace, full: bool) {
    if !app.modules_editable() {
        return;
    }
    spawn(async move {
        let mut menu = crate::ui::held_menu(title);
        if place != ModulePlace::Toolbar && !full {
            menu = menu.action("toolbar", tr("modules.move_toolbar", &[]));
        }
        if place != ModulePlace::Root {
            menu = menu.action("root", tr("modules.move_root", &[]));
        }
        let answer = menu.cancel("cancel", tr("looks.cancel", &[])).show().await;
        let chosen = match answer.ok().and_then(|a| a.action).as_deref() {
            Some("toolbar") => Some(ModulePlace::Toolbar),
            Some("root") => Some(ModulePlace::Root),
            _ => None,
        };
        if let Some(to) = chosen {
            app.move_to_place(&id, to);
        }
    });
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
        return pending_end(tr("modules.pending_remove", &[]), move || {
            app.toggle_module_remove(&toggle)
        });
    }
    let on = row.state.enabled;
    let paused = row.paused();
    let custom = row.state.custom;
    let title = row.title().to_owned();
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(custom.then(|| chip(tr("modules.custom", &[]), ChipTone::Neutral)))
        .child(dynamic(move || {
            let toggle_id = id.clone();
            let saving = app.module_part_busy(&id, &ModulePart::Enabled);
            system_switch(
                on != saving,
                title.clone(),
                live && !paused && app.modules_editable(),
                saving,
                move |on| app.set_module_enabled(&toggle_id, on),
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
    dynamic(move || {
        crate::ui::module_icon_tile(app.module_icon(&id).as_deref(), &initial, size, faded)
    })
    .into_element()
}

/// A module: its tile, title and summary, and `trailing`. When it
/// `opens`, a press opens the module and a held finger its menu.
fn module_row(app: AppState, row: ModuleRow, trailing: AnyElement, opens: bool) -> ListRow {
    let summary = summary_text(app, &row);
    let id = row.id().to_owned();
    let off = !row.state.enabled;
    let pressable = opens && !row.failed() && !row.safe();
    let list_row = list_row(row.title().to_owned())
        .subtitle(summary)
        .leading(module_tile(app, id.clone(), ROW_TILE, off))
        .trailing(trailing);
    if !pressable {
        return list_row;
    }
    let route = Route::ModuleDetail(id);
    list_row
        .press(move || app.push(route.clone()))
        .held(move || Some(module_menu(app, &row)))
}
