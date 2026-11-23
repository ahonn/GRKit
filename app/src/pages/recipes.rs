//! Recipes tab: the camera's recipes in camera menu order. Browse applies
//! a recipe from its row; Edit turns recipes on or off and moves them.

use nsh_base::{alert_dialog, theme};
use nsh_core::element::header::HeaderItem;
use nsh_core::prelude::*;

use crate::ui::ty::text;

use grkit_core::recipes::Look;

use crate::domain::recipes::RecipeItem;
use crate::pages::looks::{
    ROW_COVER, ROW_GAP, ROW_HEIGHT, RowGestures, RowMode, drag_handle, edit_button, list,
    list_count, loading_card, offline_card, refresh_notice, reorder_list, round_check, section,
};
use crate::pages::recipe_text::{glow_level, grain_level};
use crate::store::{AppState, Route, Tab};
use crate::strings::tr;
use crate::ui::{
    badge, camera_art, circle_button, cover_view, grouped_card, primary_button, secondary_button,
    token,
};

const PLUS: &str = "M12 5v14M5 12h14";

pub fn recipes_root(app: AppState) -> impl IntoElement {
    // Edit shows the checks and the handles; Browse opens and applies.
    let editing = signal(false);
    crate::ui::app_page()
        .title(move || tr("recipes.title", &[]))
        .header_large_title(!cfg!(target_os = "android"))
        .header_right_items(move || {
            if !can_add(app) {
                return Vec::new();
            }
            if editing.get() {
                return vec![
                    HeaderItem::new(tr("recipes.done", &[]))
                        .key("done")
                        .on_press(move || editing.set(false)),
                ];
            }
            let mut items = vec![
                HeaderItem::symbol("plus")
                    .key("add")
                    .label(tr("recipes.add_title", &[]))
                    .on_press(move || app.open_recipe_presets()),
            ];
            if can_edit(app) {
                items.push(
                    HeaderItem::new(tr("recipes.edit", &[]))
                        .key("edit")
                        .on_press(move || editing.set(true)),
                );
            }
            items
        })
        .header_right(dynamic(move || {
            can_add(app).then(|| {
                view()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(can_edit(app).then(|| edit_button(editing)))
                    .child((!editing.get()).then(|| {
                        circle_button(PLUS, tr("recipes.add_title", &[]), true, move || {
                            app.open_recipe_presets()
                        })
                    }))
            })
        }))
        // The scroll view stays while the recipes change: only its content
        // follows them, so a change does not move the list back to the top.
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

fn can_add(app: AppState) -> bool {
    app.camera_online() && app.recipes_snapshot().is_some()
}

/// Edit shows with recipes to change.
fn can_edit(app: AppState) -> bool {
    can_add(app)
        && app
            .recipes_snapshot()
            .is_some_and(|s| !s.recipes.is_empty())
}

/// A card with art, a title, a note, and one button.
fn center_card(
    art: AnyElement,
    title: String,
    body: String,
    button: String,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    grouped_card(
        view()
            .p_6()
            .gap_4()
            .items_center()
            .child(art)
            .child(
                text(title)
                    .text_size(px(crate::ui::ty::HEADLINE))
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                text(body)
                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                    .line_height(px(22.0))
                    .text_center()
                    .text_color(move || theme().muted_foreground),
            )
            .child(view().w_full().child(primary_button(button, on_press))),
    )
}

const RECIPE_ART: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 90" width="150" height="112" fill="none" stroke="#9A9A9A" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"><rect x="30" y="8" width="60" height="74" rx="8"/><path d="M42 30h36M42 44h36M42 58h22"/></svg>"##;

fn on_camera(app: AppState, editing: bool) -> AnyElement {
    if !app.camera_online() {
        if let Some(snapshot) = app.offline_recipes() {
            return offline_recipes(app, snapshot).into_element();
        }
        return center_card(
            view()
                .opacity(0.35)
                .child(camera_art(crate::domain::CameraModel::Gr3, 180.0, false))
                .into_element(),
            tr("looks.no_camera_title", &[]),
            tr("recipes.no_camera_body", &[]),
            tr("looks.connect", &[]),
            move || {
                app.set_tab(Tab::Camera);
                app.connect_saved();
            },
        )
        .into_element();
    }
    let Some(snapshot) = app.recipes_snapshot() else {
        if app.recipes_error().is_some() {
            return grouped_card(
                view()
                    .p_4()
                    .gap_3()
                    .child(text(tr("recipes.read_failed", &[])).text_size(px(crate::ui::ty::BODY)))
                    .child(secondary_button(tr("looks.retry", &[]), move || {
                        app.refresh_recipes()
                    })),
            )
            .into_element();
        }
        return loading_card(tr("recipes.loading", &[])).into_element();
    };
    if snapshot.recipes.is_empty() {
        return center_card(
            image(nsh_core::element::ImageSrc::svg_markup(RECIPE_ART))
                .w(px(150.0))
                .h(px(112.0))
                .into_element(),
            tr("recipes.empty_title", &[]),
            tr("recipes.empty_body", &[]),
            tr("recipes.add_title", &[]),
            move || app.open_recipe_presets(),
        )
        .into_element();
    }
    let editable = app.recipe_busy().is_none();
    let mode = if editing {
        RowMode::Edit
    } else {
        RowMode::Browse
    };
    let (on, off): (Vec<RecipeItem>, Vec<RecipeItem>) =
        snapshot.recipes.iter().cloned().partition(|r| r.enabled);
    let count = on.len();
    let menu_list = if mode == RowMode::Edit {
        let rows = std::cell::RefCell::new(on.into_iter().map(Some).collect::<Vec<_>>());
        reorder_list(
            count,
            editable && count > 1,
            move |from, to| app.move_recipe(from, to),
            move |index, gestures| {
                let item = rows.borrow_mut()[index].take().expect("each row once");
                recipe_row(app, item, mode, editable, index + 1 == count, gestures).into_element()
            },
        )
        .into_element()
    } else {
        let last = count.saturating_sub(1);
        list(
            on.into_iter()
                .enumerate()
                .map(|(i, item)| {
                    recipe_row(app, item, mode, editable, i == last, None).into_element()
                })
                .collect(),
        )
        .into_element()
    };
    view()
        .gap(px(ROW_GAP))
        .child(dynamic(move || {
            app.recipes_error().map(|_| {
                refresh_notice(app.recipes_loading(), move || app.refresh_recipes()).into_element()
            })
        }))
        .child(section(
            tr("recipes.menu", &[]),
            list_count(count, snapshot.max_on),
            move || app.recipes_refreshing(),
            menu_list,
            (mode != RowMode::Edit).then(|| tr("recipes.menu_note", &[])),
        ))
        .child((!off.is_empty()).then(|| {
            let last = off.len() - 1;
            let n = off.len().to_string();
            let rows = off
                .into_iter()
                .enumerate()
                .map(|(i, item)| {
                    recipe_row(app, item, mode, editable, i == last, None).into_element()
                })
                .collect();
            section(
                tr("recipes.off", &[]),
                n,
                || false,
                list(rows),
                (mode == RowMode::Edit).then(|| tr("recipes.off_edit_note", &[])),
            )
        }))
        .into_element()
}

/// The recipes as the phone last read them, while no camera is connected:
/// read-only, with a way to connect.
fn offline_recipes(
    app: AppState,
    snapshot: crate::domain::recipes::RecipesSnapshot,
) -> impl IntoElement {
    let (on, off): (Vec<RecipeItem>, Vec<RecipeItem>) =
        snapshot.recipes.into_iter().partition(|r| r.enabled);
    let rows = |items: Vec<RecipeItem>| {
        let last = items.len().saturating_sub(1);
        list(
            items
                .into_iter()
                .enumerate()
                .map(|(i, item)| {
                    recipe_row(app, item, RowMode::ReadOnly, false, i == last, None).into_element()
                })
                .collect(),
        )
    };
    let on_count = list_count(on.len(), snapshot.max_on);
    let off_count = off.len().to_string();
    let has_off = !off.is_empty();
    view()
        .gap(px(ROW_GAP))
        .child(offline_card(app))
        .child(section(
            tr("recipes.menu", &[]),
            on_count,
            || false,
            rows(on),
            None,
        ))
        .child(
            has_off.then(|| section(tr("recipes.off", &[]), off_count, || false, rows(off), None)),
        )
}

/// The tile of a recipe: the cover of its Film Look, else the camera's icon
/// of its Image Control type.
pub fn recipe_tile(app: AppState, item: &RecipeItem, size: f32, faded: bool) -> AnyElement {
    if let Some(look) = item.effect_id.as_deref().and_then(|id| app.look(id)) {
        return cover_view(&look.shown_cover(), size, faded);
    }
    match &item.recipe.look {
        Look::Native(native) => crate::ui::image_control_tile(native.image_type, size, faded),
        _ => native_tile(
            item.effect_name.clone().unwrap_or_else(|| "?".into()),
            size,
            faded,
        ),
    }
}

pub fn native_tile(label: String, size: f32, faded: bool) -> AnyElement {
    view()
        .w(px(size))
        .h(px(size))
        .rounded(px(size / 7.0))
        .bg(rgb(0x3a3a3c))
        .items_center()
        .justify_center()
        .p(px(3.0))
        .opacity(if faded { 0.45 } else { 1.0 })
        .child(
            // Drawn on a fixed tile, as a cover is.
            text(label)
                .allow_font_scaling(false)
                .text_size(px((size / 4.0).max(10.0)))
                .font_weight(FontWeight::SEMIBOLD)
                .text_center()
                .number_of_lines(3)
                .text_color(rgb(0xffffff)),
        )
        .into_element()
}

/// The line under the name: the look, and its grain and glow.
pub fn recipe_summary(app: AppState, item: &RecipeItem) -> String {
    if !item.ready {
        return tr("recipes.look_missing", &[]);
    }
    let mut parts = vec![match &item.recipe.look {
        Look::Native(native) => crate::store::native_type_name(native.image_type),
        Look::Effect { .. } => item.effect_name.clone().unwrap_or_default(),
        _ => tr("recipes.look_kept", &[]),
    }];
    if let Some(grain) = item.recipe.grain.filter(|g| g.level > 0) {
        parts.push(format!(
            "{} {}",
            tr("recipes.grain", &[]),
            grain_level(grain.level)
        ));
    }
    if let Some(glow) = item.recipe.glow.filter(|g| g.halation > 0) {
        parts.push(format!(
            "{} {}",
            tr("recipes.halation", &[]),
            glow_level(glow.halation)
        ));
    }
    let _ = app;
    parts.join(" · ")
}

fn recipe_row(
    app: AppState,
    item: RecipeItem,
    mode: RowMode,
    editable: bool,
    last: bool,
    gestures: Option<RowGestures>,
) -> impl IntoElement {
    let name = app.recipe_name(&item);
    let summary = recipe_summary(app, &item);
    let off = !item.enabled;
    let (id, enabled, current, ready) = (item.id, item.enabled, item.current, item.ready);
    let working = app.recipe_row_busy(id);
    // In Browse, holding the row opens its menu; in Edit the handle moves it.
    let row_gestures = (mode == RowMode::Browse).then(|| {
        let menu_item = item.clone();
        RowGestures {
            long_press: Box::new(move || {
                if editable {
                    recipe_menu(app, menu_item.clone(), Box::new(|| {}))
                }
            }),
            release: None,
            handle: None,
        }
    });
    let mut label = format!("{name}, {summary}");
    if current {
        label = format!("{label}, {}", tr("recipes.current", &[]));
    }
    let toggle_label = tr(
        if enabled {
            "recipes.hide_named"
        } else {
            "recipes.show_named"
        },
        &[("name", &name)],
    );
    let apply_label = tr("recipes.apply_named", &[("name", &name)]);
    let handle_name = name.clone();
    view()
        .flex_row()
        .items_center()
        .min_h(px(ROW_HEIGHT))
        .pr(px(4.0))
        .bg(move || theme().card)
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .child((mode == RowMode::Edit).then(|| {
            round_check(enabled, toggle_label, editable, working, move || {
                app.set_recipe_enabled(id, !enabled)
            })
        }))
        .child({
            let v = view()
                .flex_1()
                .min_h(px(ROW_HEIGHT))
                .py_1()
                .flex_row()
                .items_center()
                .gap_3()
                .pl(px(if mode == RowMode::Edit { 4.0 } else { 12.0 }))
                .role(AccessibilityRole::Button)
                .label(label)
                .when(mode == RowMode::Browse, |v| {
                    v.on_press(move || app.push(Route::RecipeDetail(id)))
                        .active(|s| s.opacity(0.72))
                });
            match row_gestures {
                Some(g) => v.on_long_press(g.long_press),
                None => v,
            }
            .child(recipe_tile(app, &item, ROW_COVER, off))
            .child(
                view()
                    .flex_1()
                    .gap(px(2.0))
                    .child(
                        view()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(
                                text(name.clone())
                                    .flex_shrink(1.0)
                                    .text_size(px(crate::ui::ty::HEADLINE))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(move || {
                                        if off {
                                            theme().muted_foreground
                                        } else {
                                            theme().foreground
                                        }
                                    }),
                            )
                            .child(current.then(|| badge(tr("recipes.current", &[])))),
                    )
                    .child(
                        text(summary)
                            .number_of_lines(2)
                            .text_size(px(crate::ui::ty::FOOTNOTE))
                            .text_color(move || theme().muted_foreground),
                    ),
            )
        })
        .child(match mode {
            RowMode::Browse => crate::ui::pill_button_loading(
                tr("recipes.apply_short", &[]),
                apply_label,
                editable && ready,
                working,
                move || app.apply_recipe(id),
            )
            .into_element(),
            RowMode::Edit if gestures.is_some() => {
                drag_handle(handle_name, gestures).into_element()
            }
            _ => view().into_element(),
        })
}

/// The menu of a held row: apply, turn on or off, rename, delete.
pub fn recipe_menu(app: AppState, item: RecipeItem, closed: Box<dyn FnOnce()>) {
    if app.recipe_busy().is_some() {
        closed();
        return;
    }
    let name = app.recipe_name(&item);
    spawn(async move {
        let mut sheet = crate::ui::held_menu(name.clone());
        if item.ready {
            sheet = sheet.action("apply", tr("recipes.apply", &[]));
        }
        let answer = sheet
            .action(
                "toggle",
                tr(
                    if item.enabled {
                        "recipes.menu_hide"
                    } else {
                        "recipes.menu_show"
                    },
                    &[],
                ),
            )
            .action("rename", tr("recipes.rename", &[]))
            .destructive("delete", tr("recipes.delete", &[]))
            .cancel("cancel", tr("looks.cancel", &[]))
            .show()
            .await;
        closed();
        match answer.ok().and_then(|a| a.action).as_deref() {
            Some("apply") => app.apply_recipe(item.id),
            Some("toggle") => app.set_recipe_enabled(item.id, !item.enabled),
            Some("rename") => app.ask_rename_recipe(item.id),
            Some("delete") => confirm_delete_recipe(app, item.id, name),
            _ => {}
        }
    });
}

pub fn confirm_delete_recipe(app: AppState, id: u32, name: String) {
    spawn(async move {
        let answer = alert_dialog()
            .title(tr("recipes.delete_title", &[("name", &name)]))
            .description(tr("recipes.delete_body", &[]))
            .cancel("cancel", tr("looks.cancel", &[]))
            .destructive("delete", tr("looks.delete_action", &[]))
            .show()
            .await;
        if answer.ok().and_then(|a| a.action).as_deref() == Some("delete") {
            app.delete_recipe(id);
        }
    });
}
