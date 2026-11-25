//! Recipes tab: the camera's recipes in camera menu order. Browse applies
//! a recipe from its row; Edit turns recipes on or off and moves them.

use nsh_base::alert_dialog;
use nsh_core::element::header::HeaderItem;
use nsh_core::prelude::*;

use crate::ui::ty::text;

use grkit_core::recipes::Look;

use crate::domain::recipes::RecipeItem;
use crate::pages::looks::{
    ROW_COVER, ROW_GAP, RowMode, loading_card, no_camera, offline_card, refresh_notice, section,
};
use crate::pages::recipe_text::{glow_level, grain_level};
use crate::store::{AppState, Route, app_store_url};
use crate::strings::tr;
use crate::ui::PageTitle;
use crate::ui::{
    ButtonKind, ButtonSize, ChipTone, ListRow, RowGestures, button, check_circle, cover_thumb,
    drag_handle, empty_state, glyph, list_row, list_section, notice_card, reorder_list, row_chip,
    token,
};

pub fn recipes_root(app: AppState) -> impl IntoElement {
    // Edit shows the checks and the handles; Browse opens and applies.
    let editing = signal(false);
    crate::ui::app_page()
        .page_title(move || tr("recipes.title", &[]), true)
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
                items.push(crate::pages::looks::sort_item(editing));
            }
            items
        })
        .header_right(dynamic(move || {
            can_add(app).then(|| {
                crate::pages::looks::edit_add_buttons(
                    editing,
                    can_edit(app),
                    tr("recipes.add_title", &[]),
                    move || app.open_recipe_presets(),
                )
            })
        }))
        // The scroll view stays while the recipes change: only its content
        // follows them, so a change does not move the list back to the top.
        .child(
            crate::ui::page_scroll()
                .refresh(Some(crate::store::page_refresh::RECIPES.to_owned()))
                .top_of(Some(crate::store::Tab::Recipes.key().to_owned()))
                .flex_1()
                .bg(move || token("list-background"))
                .child(
                    view()
                        .px_4()
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

fn on_camera(app: AppState, editing: bool) -> AnyElement {
    if !app.camera_online() {
        if let Some(snapshot) = app.offline_recipes() {
            return offline_recipes(app, snapshot).into_element();
        }
        return no_camera(app, "recipes.no_camera_body").into_element();
    }
    let Some(snapshot) = app.recipes_snapshot() else {
        if app.recipes_error().is_some() {
            return empty_state(glyph::WARNING, tr("recipes.read_failed_title", &[]))
                .action(tr("looks.retry", &[]), move || app.refresh_recipes())
                .into_element();
        }
        return loading_card(tr("recipes.loading", &[])).into_element();
    };
    if snapshot.recipes.is_empty() && snapshot.unreadable == 0 {
        return empty_state(glyph::RECIPE, tr("recipes.empty_title", &[]))
            .body(tr("recipes.empty_body", &[]))
            .action(tr("recipes.add_title", &[]), move || {
                app.open_recipe_presets()
            })
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
                edit_row(app, item, editable, index + 1 == count, gestures)
            },
        )
        .into_element()
    } else {
        list_section(
            on.into_iter()
                .map(|item| recipe_row(app, item, mode, editable)),
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
        .child(look_off_notice(app, &snapshot))
        .child(section(
            tr("recipes.menu", &[]),
            move || app.recipes_refreshing(),
            menu_list,
            None,
        ))
        .child((!off.is_empty()).then(|| {
            let rows = if mode == RowMode::Edit {
                let last = off.len() - 1;
                crate::ui::list(
                    off.into_iter()
                        .enumerate()
                        .map(|(i, item)| edit_row(app, item, editable, i == last, None))
                        .collect(),
                )
                .into_element()
            } else {
                list_section(
                    off.into_iter()
                        .map(|item| recipe_row(app, item, mode, editable)),
                )
                .into_element()
            };
            section(
                tr("recipes.off", &[]),
                || false,
                rows,
                (mode == RowMode::Edit).then(|| tr("recipes.off_edit_note", &[])),
            )
        }))
        .child((snapshot.unreadable > 0).then(|| newer_app_row(snapshot.unreadable)))
        .into_element()
}

/// Film Look is off, and a recipe needs it.
fn look_off_notice(
    app: AppState,
    snapshot: &crate::domain::recipes::RecipesSnapshot,
) -> Option<AnyElement> {
    if app.looks_available() || snapshot.recipes.iter().all(|r| r.ready) {
        return None;
    }
    let id = app.film_look_off()?;
    Some(
        notice_card(tr("modules.looks_off", &[]))
            .action(button(
                tr("modules.enable", &[]),
                ButtonKind::Tinted,
                ButtonSize::Small,
                move || app.set_module_enabled(&id, true),
            ))
            .into_element(),
    )
}

/// The recipes of a later GRKit that this app leaves out.
fn newer_app_row(count: usize) -> impl IntoElement {
    notice_card(tr("recipes.needs_app", &[("count", &count.to_string())])).action(button(
        tr("recipes.needs_app_action", &[]),
        ButtonKind::Tinted,
        ButtonSize::Small,
        || crate::pages::software::open(app_store_url()),
    ))
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
        list_section(
            items
                .into_iter()
                .map(|item| recipe_row(app, item, RowMode::ReadOnly, false)),
        )
    };
    let has_off = !off.is_empty();
    view()
        .gap(px(ROW_GAP))
        .child(offline_card(app))
        .child(section(tr("recipes.menu", &[]), || false, rows(on), None))
        .child(has_off.then(|| section(tr("recipes.off", &[]), || false, rows(off), None)))
}

/// The tile of a recipe: the cover of its Film Look, else the camera's icon
/// of its Image Control type.
pub fn recipe_tile(app: AppState, item: &RecipeItem, size: f32, faded: bool) -> AnyElement {
    if let Some(look) = item.effect_id.as_deref().and_then(|id| app.look(id)) {
        return cover_thumb(&look.shown_cover(), size, faded);
    }
    match &item.recipe.look {
        Look::Native(native) => crate::ui::image_control_tile(native.image_type, size, faded),
        // The look is off or missing: the tile keeps the recipe's own words.
        _ => native_tile(
            item.effect_name
                .clone()
                .or_else(|| item.name.clone())
                .unwrap_or_default(),
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
            // Drawn on a fixed tile, as a cover is. The longest word fits
            // the tile's width, so a line breaks between words, not inside
            // one (CineStill, not CineSt / ill).
            text(label.clone())
                .allow_font_scaling(false)
                .text_size(px(native_tile_text(&label, size)))
                .font_weight(FontWeight::SEMIBOLD)
                .text_center()
                .number_of_lines(3)
                .text_color(rgb(0xffffff)),
        )
        .into_element()
}

/// The text size of a name tile `size` points wide: a quarter of the tile,
/// smaller when the longest word would not fit on one line (about 0.6 em
/// a letter in the semibold system font), and not under a sixth of it.
fn native_tile_text(label: &str, size: f32) -> f32 {
    let longest = label
        .split_whitespace()
        .map(|word| word.chars().count())
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    let fits = (size - 6.0) / (longest * 0.6);
    (size / 4.0).max(10.0).min(fits).max(size / 6.0)
}

/// The line under the name: the look, and its grain and glow.
pub fn recipe_summary(app: AppState, item: &RecipeItem) -> String {
    if !item.ready {
        return tr(
            if app.looks_available() {
                "recipes.look_missing"
            } else {
                "recipes.needs_look"
            },
            &[],
        );
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

/// A recipe in Browse or ReadOnly: its tile, name and summary. Browse
/// opens it, and applies it from its button; the current recipe has a chip
/// instead, as there is nothing to apply.
fn recipe_row(app: AppState, item: RecipeItem, mode: RowMode, editable: bool) -> ListRow {
    let name = app.recipe_name(&item);
    let summary = recipe_summary(app, &item);
    let (id, current, ready) = (item.id, item.current, item.ready);
    let working = app.recipe_row_busy(id);
    let row = list_row(name.clone())
        .subtitle(summary)
        .leading(crate::ui::recipe_badge(
            recipe_tile(app, &item, ROW_COVER, !item.enabled),
            ROW_COVER,
            !item.enabled,
        ))
        .test_id(format!("recipe:{id}"));
    if mode != RowMode::Browse {
        return row;
    }
    let end = if current && !working {
        row_chip(tr("recipes.current", &[]), ChipTone::Tint)
    } else {
        button(
            tr("recipes.apply_short", &[]),
            ButtonKind::Tinted,
            ButtonSize::Small,
            move || app.apply_recipe(id),
        )
        .label(tr("recipes.apply_named", &[("name", &name)]))
        .enabled(editable && ready)
        .loading(working)
        .into_element()
    };
    // The row opens the recipe: the chevron says so beside Apply.
    row.trailing(end)
        .page(move || app.push(Route::RecipeDetail(id)))
}

/// A recipe in Edit: the check that shows or hides it in the camera menu,
/// its tile and name, and the handle that moves it.
fn edit_row(
    app: AppState,
    item: RecipeItem,
    editable: bool,
    last: bool,
    gestures: Option<RowGestures>,
) -> AnyElement {
    let name = app.recipe_name(&item);
    let (id, enabled) = (item.id, item.enabled);
    let working = app.recipe_row_busy(id);
    let toggle_label = tr(
        if enabled {
            "recipes.hide_named"
        } else {
            "recipes.show_named"
        },
        &[("name", &name)],
    );
    let leading = view()
        .flex_row()
        .items_center()
        // The check's touch area reaches into the card's margin.
        .ml(px(-10.0))
        .gap(px(4.0))
        .child(check_circle(
            enabled,
            toggle_label,
            editable,
            working,
            move || app.set_recipe_enabled(id, !enabled),
        ))
        .child(crate::ui::recipe_badge(
            recipe_tile(app, &item, ROW_COVER, !enabled),
            ROW_COVER,
            !enabled,
        ));
    let mut row = list_row(name.clone())
        .subtitle(recipe_summary(app, &item))
        .leading(leading);
    if gestures.is_some() {
        row = row.trailing(drag_handle(name, gestures));
    }
    view()
        .bg(move || token("list-card"))
        .child(row.into_row(last))
        .into_element()
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
