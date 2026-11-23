//! The page of one recipe and the presets of the site.
//! The settings of a recipe show read-only: the camera saves recipes.

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::ui::ty::text;

use grkit_core::recipes::Look;

use crate::domain::recipes::{PresetItem, RecipeItem};
use crate::pages::recipe_text::{Group, groups};
use crate::pages::recipes::{confirm_delete_recipe, native_tile, recipe_summary, recipe_tile};
use crate::store::{AppState, RecipeAction, Route, native_type_name};
use crate::strings::tr;
use crate::ui::{
    action_row, action_row_loading, badge, control_row, cover_view, footnote, grouped_card,
    info_row, loading_content, primary_button_loading, quiet_spinner, secondary_button,
    section_title, segmented, toggle_loading, token,
};

/// A page's scroll view around `content`. The scroll view stays while the
/// data changes: only the content follows it, so a change does not move
/// the page back to the top.
fn page_body<E: IntoElements>(content: impl Fn() -> E + 'static) -> impl IntoElement {
    crate::ui::page_scroll()
        .flex_1()
        .bg(move || token("grouped"))
        .child(
            view()
                .px_6()
                .pt_4()
                .pb(px(crate::pages::look_pages::TAB_BAR_CLEARANCE))
                .gap(px(20.0))
                .child(dynamic(content)),
        )
}

fn group_view(group: Group) -> impl IntoElement {
    let (title, rows) = group;
    let last = rows.len().saturating_sub(1);
    let mut card = view();
    for (i, (name, value)) in rows.into_iter().enumerate() {
        card = card.child(info_row(name, value, i == last));
    }
    view()
        .gap_2()
        .child(view().px(px(2.0)).child(section_title(title)))
        .child(grouped_card(card))
}

/// The tile, name, and line under it at the top of a page.
fn hero(tile: AnyElement, name: String, sub: String, current: bool) -> impl IntoElement {
    view()
        .items_center()
        .gap_2()
        .child(view().pt_1().child(tile))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(name)
                        .flex_shrink(1.0)
                        .text_size(px(crate::ui::ty::TITLE2))
                        .font_weight(FontWeight::BOLD)
                        .text_center(),
                )
                .child(current.then(|| badge(tr("recipes.current", &[])))),
        )
        .child(
            text(sub)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .text_center()
                .text_color(move || theme().muted_foreground),
        )
}

/// The look row of a recipe: its Film Look or its Image Control type.
fn look_row(item_look: &Look, effect: Option<String>) -> (String, String) {
    match item_look {
        Look::Native(native) => (
            tr("recipes.image_control", &[]),
            native_type_name(native.image_type),
        ),
        Look::Effect { .. } => (
            tr("recipes.film_look", &[]),
            effect.unwrap_or_else(|| tr("recipes.look_missing", &[])),
        ),
        _ => (tr("recipes.film_look", &[]), tr("recipes.keep", &[])),
    }
}

pub fn recipe_page(app: AppState, id: u32, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    crate::ui::app_page()
        .title(move || {
            app.recipe(id)
                .map(|r| app.recipe_name(&r))
                .unwrap_or_default()
        })
        .on_dismiss(on_dismiss)
        .child(page_body(move || {
            app.recipe(id).map(|item| recipe_content(app, item))
        }))
}

fn recipe_content(app: AppState, item: RecipeItem) -> impl IntoElement {
    let name = app.recipe_name(&item);
    let editable = app.recipe_busy().is_none();
    let key = item.id.to_string();
    let applying = app.recipe_working(&key, RecipeAction::Apply);
    let toggling = app.recipe_working(&key, RecipeAction::Toggle);
    let deleting = app.recipe_working(&key, RecipeAction::Delete);
    let position = app.recipes_snapshot().and_then(|s| {
        s.recipes
            .iter()
            .filter(|r| r.enabled)
            .position(|r| r.id == item.id)
    });
    let sub = match position {
        Some(n) => tr("recipes.position", &[("n", &(n + 1).to_string())]),
        None => tr("recipes.is_off", &[]),
    };
    let sub = if item.ready {
        sub
    } else {
        recipe_summary(app, &item)
    };
    let id = item.id;
    let (enabled, hidden) = (item.enabled, item.recipe.hidden);
    let mut content = view()
        .gap(px(20.0))
        .child(hero(
            recipe_tile(app, &item, 120.0, false),
            name.clone(),
            sub,
            item.current,
        ))
        .child(item.ready.then(|| {
            primary_button_loading(tr("recipes.apply", &[]), applying, move || {
                app.apply_recipe(id)
            })
        }))
        .child(edit_steps());
    for group in groups(
        &item.recipe,
        look_row(&item.recipe.look, item.effect_name.clone()),
    ) {
        content = content.child(group_view(group));
    }
    let mut shown = view().child(control_row(
        tr("recipes.show_in_menu", &[]),
        toggle_loading(
            enabled,
            tr("recipes.show_in_menu", &[]),
            editable,
            toggling,
            move || app.set_recipe_enabled(id, !enabled),
        ),
        !hidden,
    ));
    if hidden {
        shown = shown.child(info_row(
            tr("recipes.in_adj", &[]),
            tr("recipes.hidden", &[]),
            true,
        ));
    }
    content
        .child(
            view()
                .gap_2()
                .child(grouped_card(shown))
                .child(hidden.then(|| footnote(tr("recipes.hidden_note", &[])))),
        )
        .child(grouped_card(
            view()
                .child(action_row(
                    tr("recipes.rename", &[]),
                    false,
                    false,
                    move || app.ask_rename_recipe(id),
                ))
                .child(action_row_loading(
                    tr("recipes.delete", &[]),
                    true,
                    true,
                    deleting,
                    move || confirm_delete_recipe(app, id, name.clone()),
                )),
        ))
}

/// How to change a recipe's settings: the camera saves recipes, so the
/// phone shows the steps.
fn edit_steps() -> impl IntoElement {
    let step = |n: u8, key: &'static str| {
        view()
            .flex_row()
            .items_start()
            .gap(px(10.0))
            .child(
                view()
                    .size(px(22.0))
                    .rounded_full()
                    .items_center()
                    .justify_center()
                    .bg(move || token("button"))
                    .child(
                        text(n.to_string())
                            .text_size(px(crate::ui::ty::CAPTION))
                            .font_weight(FontWeight::BOLD)
                            .text_color(move || token("button-foreground")),
                    ),
            )
            .child(
                text(tr(key, &[]))
                    .flex_1()
                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                    .line_height(px(22.0)),
            )
    };
    view()
        .rounded(px(10.0))
        .py_3()
        .px_4()
        .gap(px(10.0))
        .bg(move || theme().muted)
        .role(AccessibilityRole::Text)
        .label(format!(
            "{}: {}, {}, {}",
            tr("recipes.steps_title", &[]),
            tr("recipes.step1", &[]),
            tr("recipes.step2", &[]),
            tr("recipes.step3", &[])
        ))
        .child(
            text(tr("recipes.steps_title", &[]))
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD),
        )
        .child(step(1, "recipes.step1"))
        .child(step(2, "recipes.step2"))
        .child(step(3, "recipes.step3"))
}

/// Which presets show.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Color,
    Mono,
}

pub fn presets_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    let filter = signal(Filter::All);
    crate::ui::app_page()
        .title(move || tr("recipes.presets_title", &[]))
        .on_dismiss(on_dismiss)
        .child(page_body(move || presets_content(app, filter)))
}

fn presets_content(app: AppState, filter: Signal<Filter>) -> impl IntoElement {
    let chosen = filter.get();
    let chips = segmented(
        [
            (Filter::All, "looks.filter.all"),
            (Filter::Color, "looks.filter.color"),
            (Filter::Mono, "looks.filter.mono"),
        ]
        .into_iter()
        .map(|(value, key)| -> crate::ui::Segment {
            (
                tr(key, &[]),
                chosen == value,
                Box::new(move || filter.set(value)),
            )
        })
        .collect(),
    );
    let model = app
        .status()
        .map(|s| s.camera.model.display_name().to_owned())
        .unwrap_or_default();
    let all = app.recipe_presets();
    let none = all.is_empty();
    let presets: Vec<PresetItem> = all
        .into_iter()
        .filter(|p| match chosen {
            Filter::All => true,
            Filter::Color => !p.monochrome(),
            Filter::Mono => p.monochrome(),
        })
        .collect();
    let mut list = view().gap_3();
    if none && app.release_checking() {
        list = list.child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(quiet_spinner(16.0))
                .child(footnote(tr("recipes.presets_loading", &[]))),
        );
    } else if none {
        list = list
            .child(footnote(tr("recipes.presets_offline", &[])))
            .child(secondary_button(tr("looks.retry", &[]), move || {
                app.refresh_update_info()
            }));
    }
    for preset in presets {
        list = list.child(preset_card(app, preset));
    }
    view()
        .gap(px(16.0))
        .child(chips)
        .child(footnote(tr("recipes.presets_note", &[("model", &model)])))
        .child(list)
}

/// The tile of a preset: the cover of its official Look, else its type.
fn preset_tile(app: AppState, preset: &PresetItem, size: f32) -> AnyElement {
    let look = preset.effect.as_deref().and_then(|id| app.look(id));
    match (&preset.recipe.look, look) {
        (_, Some(look)) => cover_view(&look.shown_cover(), size, false),
        (Look::Native(native), None) => {
            crate::ui::image_control_tile(native.image_type, size, false)
        }
        _ => native_tile(preset.recipe.name.clone(), size, false),
    }
}

/// The official Look that a preset needs and the camera does not have.
fn missing_look(app: AppState, preset: &PresetItem) -> Option<String> {
    let id = preset.effect.as_deref()?;
    if app.look(id).is_some() {
        return None;
    }
    Some(
        app.catalog_items()
            .and_then(|items| {
                items
                    .into_iter()
                    .find(|i| i.look.id == id)
                    .map(|i| i.look.name)
            })
            .unwrap_or_else(|| look_title(id)),
    )
}

/// A readable name from a Look ID, while the catalog is not loaded.
fn look_title(id: &str) -> String {
    id.trim_end_matches("-v1")
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn preset_text(preset: &PresetItem) -> String {
    let language = if crate::strings::is_zh() { "zh" } else { "en" };
    let by = preset
        .author
        .as_ref()
        .map_or_else(|| "GRKit".to_owned(), |a| tr("recipes.by", &[("name", a)]));
    match preset.text_in(language) {
        Some(text) => format!("{by} · {text}"),
        None => by,
    }
}

fn preset_card(app: AppState, preset: PresetItem) -> impl IntoElement {
    let added = app.preset_added(&preset);
    let busy = app.recipe_busy();
    let adding = busy.as_deref() == Some(preset.id.as_str());
    let missing = missing_look(app, &preset);
    let (id, open_id, name) = (
        preset.id.clone(),
        preset.id.clone(),
        preset.recipe.name.clone(),
    );
    let trailing: AnyElement = if added {
        text(tr("recipes.on_camera", &[]))
            .text_size(px(crate::ui::ty::FOOTNOTE))
            .text_color(move || theme().muted_foreground)
            .into_element()
    } else {
        add_chip(
            tr("recipes.add", &[]),
            tr("recipes.add_named", &[("name", &name)]),
            busy.is_none(),
            adding,
            move || app.add_preset(id.clone()),
        )
        .into_element()
    };
    grouped_card(
        view()
            .flex_row()
            .items_center()
            .gap_3()
            .px(px(14.0))
            .py_3()
            .child(preset_tile(app, &preset, 40.0))
            .child(
                view()
                    .flex_1()
                    .gap(px(2.0))
                    .role(AccessibilityRole::Button)
                    .label(name.clone())
                    .on_press(move || app.push(Route::RecipePreset(open_id.clone())))
                    .active(|s| s.opacity(0.72))
                    .child(
                        text(name)
                            .text_size(px(crate::ui::ty::HEADLINE))
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        text(preset_text(&preset))
                            .text_size(px(crate::ui::ty::FOOTNOTE))
                            .line_height(px(18.0))
                            .text_color(move || theme().muted_foreground),
                    )
                    .child(missing.filter(|_| !added).map(|look| {
                        text(tr("recipes.adds_look", &[("look", &look)]))
                            .text_size(px(crate::ui::ty::FOOTNOTE))
                    })),
            )
            .child(trailing),
    )
}

fn add_chip(
    title: String,
    label: String,
    enabled: bool,
    loading: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .min_h(px(32.0))
        .px(px(14.0))
        .py_1()
        .hit_slop(crate::ui::vertical_hit_slop(32.0))
        .rounded_full()
        .justify_center()
        .bg(move || theme().foreground)
        .opacity(if enabled || loading { 1.0 } else { 0.4 })
        .role(AccessibilityRole::Button)
        .label(label)
        .accessibility_state(crate::ui::busy_state(loading))
        .when(enabled && !loading, |v| {
            v.on_press(on_press).active(|s| s.opacity(0.72))
        })
        .child(loading_content(
            loading,
            16.0,
            theme().background,
            text(title)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().background),
        ))
}

pub fn preset_page(app: AppState, id: String, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    let title_id = id.clone();
    let find = move |id: &str| app.recipe_presets().into_iter().find(|p| p.id == id);
    crate::ui::app_page()
        .title(move || find(&title_id).map(|p| p.recipe.name).unwrap_or_default())
        .on_dismiss(on_dismiss)
        .child(page_body(move || {
            find(&id).map(|preset| preset_content(app, preset))
        }))
}

fn preset_content(app: AppState, preset: PresetItem) -> impl IntoElement {
    let added = app.preset_added(&preset);
    let missing = missing_look(app, &preset);
    let look_name = missing.clone().or_else(|| {
        preset
            .effect
            .as_deref()
            .and_then(|id| app.look(id))
            .map(|l| l.name)
    });
    let id = preset.id.clone();
    let adding = app.recipe_working(&id, RecipeAction::Add);
    let mut content = view()
        .gap(px(20.0))
        .child(hero(
            preset_tile(app, &preset, 120.0),
            preset.recipe.name.clone(),
            preset_text(&preset),
            false,
        ))
        .child(missing.map(|look| {
            view()
                .rounded(px(10.0))
                .bg(move || theme().muted)
                .p(px(14.0))
                .child(
                    text(tr("recipes.adds_look_note", &[("look", &look)]))
                        .text_size(px(crate::ui::ty::SUBHEADLINE)),
                )
        }));
    for group in groups(&preset.recipe, look_row(&preset.recipe.look, look_name)) {
        content = content.child(group_view(group));
    }
    content.child(if added {
        footnote(tr("recipes.on_camera", &[])).into_element()
    } else {
        view()
            .opacity(if adding || app.recipe_busy().is_none() {
                1.0
            } else {
                0.4
            })
            .child(primary_button_loading(
                tr("recipes.add_to_camera", &[]),
                adding,
                move || {
                    if app.recipe_busy().is_none() {
                        app.add_preset(id.clone())
                    }
                },
            ))
            .into_element()
    })
}
