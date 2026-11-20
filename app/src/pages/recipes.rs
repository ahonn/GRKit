//! Recipes tab: the camera's recipes in camera menu order, which the phone
//! turns on or off, moves, renames, deletes, and applies.

use nsh_base::{alert_dialog, nav_page, theme};
use nsh_core::element::header::HeaderItem;
use nsh_core::prelude::*;

use grkit_core::recipes::Look;

use crate::domain::recipes::RecipeItem;
use crate::pages::looks::{
    ROW_COVER, ROW_GAP, ROW_HEIGHT, RowGestures, list, reorder_list, round_check, section,
};
use crate::pages::recipe_text::{glow_level, grain_level};
use crate::store::{AppState, Route, Sheet, Tab};
use crate::strings::tr;
use crate::ui::{
    badge, camera_art, circle_button, cover_view, grouped_card, primary_button, secondary_button,
    token,
};

const PLUS: &str = "M12 5v14M5 12h14";

pub fn recipes_root(app: AppState) -> impl IntoElement {
    nav_page()
        .title(move || tr("recipes.title", &[]))
        .header_large_title(!cfg!(target_os = "android"))
        .header_right_items(move || {
            if !can_add(app) {
                return Vec::new();
            }
            vec![
                HeaderItem::symbol("plus")
                    .key("add")
                    .label(tr("recipes.add_title", &[]))
                    .on_press(move || app.open_recipe_presets()),
            ]
        })
        .header_right(dynamic(move || {
            can_add(app).then(|| {
                circle_button(PLUS, tr("recipes.add_title", &[]), true, move || {
                    app.open_recipe_presets()
                })
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
                        .child(dynamic(move || on_camera(app))),
                ),
        )
}

fn can_add(app: AppState) -> bool {
    app.camera_online() && app.recipes_snapshot().is_some()
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
                    .text_size(px(17.0))
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                text(body)
                    .text_size(px(15.0))
                    .line_height(px(22.0))
                    .text_center()
                    .text_color(move || theme().muted_foreground),
            )
            .child(view().w_full().child(primary_button(button, on_press))),
    )
}

const RECIPE_ART: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 90" width="150" height="112" fill="none" stroke="#9A9A9A" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"><rect x="30" y="8" width="60" height="74" rx="8"/><path d="M42 30h36M42 44h36M42 58h22"/></svg>"##;

fn on_camera(app: AppState) -> AnyElement {
    if !app.camera_online() {
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
                    .child(text(tr("recipes.read_failed", &[])).text_size(px(17.0)))
                    .child(secondary_button(tr("looks.retry", &[]), move || {
                        app.refresh_recipes()
                    })),
            )
            .into_element();
        }
        return grouped_card(
            view().p_4().child(
                text(tr("looks.loading", &[]))
                    .text_size(px(15.0))
                    .text_color(move || theme().muted_foreground),
            ),
        )
        .into_element();
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
    let (on, off): (Vec<RecipeItem>, Vec<RecipeItem>) =
        snapshot.recipes.iter().cloned().partition(|r| r.enabled);
    let count = on.len();
    let menus = on.clone();
    let rows = std::cell::RefCell::new(on.into_iter().map(Some).collect::<Vec<_>>());
    let menu_list = reorder_list(
        count,
        editable && count > 1,
        move |from, to| app.move_recipe(from, to),
        move |index, closed| recipe_menu(app, menus[index].clone(), closed),
        move |index, gestures| {
            let item = rows.borrow_mut()[index].take().expect("each row once");
            recipe_row(app, item, editable, index + 1 == count, gestures).into_element()
        },
    );
    view()
        .gap(px(ROW_GAP))
        .child(section(
            tr("recipes.menu", &[]),
            format!("{count} / {}", snapshot.max_on),
            menu_list,
            tr("recipes.menu_note", &[]),
        ))
        .child((!off.is_empty()).then(|| {
            let last = off.len() - 1;
            let n = off.len().to_string();
            let rows = off
                .into_iter()
                .enumerate()
                .map(|(i, item)| recipe_row(app, item, editable, i == last, None).into_element())
                .collect();
            section(
                tr("recipes.off", &[]),
                n,
                list(rows),
                tr("recipes.off_note", &[]),
            )
        }))
        .into_element()
}

/// The tile of a recipe: the cover of its Film Look, else a dark square
/// with its Image Control type.
pub fn recipe_tile(app: AppState, item: &RecipeItem, size: f32, faded: bool) -> AnyElement {
    if let Some(look) = item.effect_id.as_deref().and_then(|id| app.look(id)) {
        return cover_view(&look.shown_cover(), size, faded);
    }
    let label = match &item.recipe.look {
        Look::Native(native) => crate::store::native_type_short(native.image_type),
        _ => item.effect_name.clone().unwrap_or_else(|| "?".into()),
    };
    native_tile(label, size, faded)
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
            text(label)
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
    editable: bool,
    last: bool,
    gestures: Option<RowGestures>,
) -> impl IntoElement {
    let name = app.recipe_name(&item);
    let summary = recipe_summary(app, &item);
    let off = !item.enabled;
    let (id, enabled, current) = (item.id, item.enabled, item.current);
    let gestures = gestures.unwrap_or_else(|| {
        let menu_item = item.clone();
        RowGestures {
            long_press: Box::new(move || {
                if editable {
                    recipe_menu(app, menu_item.clone(), Box::new(|| {}))
                }
            }),
            release: None,
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
    view()
        .flex_row()
        .items_center()
        .h(px(ROW_HEIGHT))
        .pr(px(4.0))
        .bg(move || theme().card)
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .child({
            let v = view()
                .flex_1()
                .h_full()
                .flex_row()
                .items_center()
                .gap_3()
                .pl(px(12.0))
                .role(AccessibilityRole::Button)
                .label(label)
                .on_press(move || app.push(Route::RecipeDetail(id)))
                .on_long_press(gestures.long_press);
            match gestures.release {
                Some(release) => v.on_press_up(move |_: &PressEvent| release()),
                None => v,
            }
            .active(|s| s.opacity(0.72))
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
                                    .number_of_lines(1)
                                    .text_size(px(17.0))
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
                            .number_of_lines(1)
                            .text_size(px(13.0))
                            .text_color(move || theme().muted_foreground),
                    ),
            )
        })
        .child(round_check(enabled, toggle_label, editable, move || {
            app.set_recipe_enabled(id, !enabled)
        }))
}

/// The menu of a held row: apply, turn on or off, rename, delete.
pub fn recipe_menu(app: AppState, item: RecipeItem, closed: Box<dyn FnOnce()>) {
    if app.recipe_busy().is_some() {
        closed();
        return;
    }
    let name = app.recipe_name(&item);
    spawn(async move {
        let mut sheet = alert_dialog().title(name.clone()).action_sheet();
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
            Some("rename") => app.set_sheet(Some(Sheet::RenameRecipe(item.id))),
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
