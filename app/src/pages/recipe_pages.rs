//! The page of one recipe and the presets of the site.
//! The settings of a recipe show read-only: the camera saves recipes.

use nsh_core::prelude::*;

use crate::ui::ty::text;

use grkit_core::recipes::Look;

use crate::domain::recipes::{PresetItem, RecipeItem};
use crate::pages::recipe_text::{Group, Icon, Setting, State, groups};
use crate::pages::recipes::{confirm_delete_recipe, native_tile, recipe_summary, recipe_tile};
use crate::store::{AppState, RecipeAction, Route, native_type_name};
use crate::strings::tr;
use crate::ui::PageTitle;
use crate::ui::setting_icons::setting_icon;
use crate::ui::{
    ButtonKind, ButtonSize, ChipTone, ListRow, SECTION_RADIUS, button, cover_thumb, list_row,
    list_section, notice_card, quiet_spinner, row_chip, system_segmented, system_switch, token,
};

/// A page's scroll view around `content`. The scroll view stays while the
/// data changes: only the content follows it, so a change does not move
/// the page back to the top. `content` draws its own top and sides.
fn page_body<E: IntoElements>(content: impl Fn() -> E + 'static) -> impl IntoElement {
    crate::pages::page_scroll()
        .flex_1()
        .bg(move || token("list-background"))
        .child(
            view()
                .pb(px(crate::ui::TAB_BAR_CLEARANCE))
                .child(dynamic(content)),
        )
}

/// The part of a page under its top, with the margins of a list.
fn page_rest() -> View {
    view().px_4().pt(px(24.0)).gap(px(24.0))
}

/// The side of the icon at the start of a setting's row.
const SETTING_ICON: f32 = 28.0;

/// The settings of a recipe: a section of each group with the settings that
/// the recipe sets, then one section that names the settings it leaves
/// alone, turns off or sets to 0. `look_icon` draws the Film Look or Image
/// Control type, and `open_look` opens the Film Look's page.
fn settings_view(
    app: AppState,
    groups: Vec<Group>,
    look_icon: AnyElement,
    open_look: Option<String>,
) -> Vec<AnyElement> {
    let mut look_icon = Some(look_icon);
    let (mut keep, mut off, mut zero) = (Vec::new(), Vec::new(), Vec::new());
    let mut sections = Vec::new();
    for (title, settings) in groups {
        let mut rows = Vec::new();
        for setting in settings {
            match setting.state {
                State::Keep => keep.push(setting.name),
                State::Off => off.push(setting.name),
                State::Zero => zero.push(setting.name),
                State::Set => {
                    let icon = match setting.icon {
                        Icon::Look => look_icon.take(),
                        Icon::Module(id) => Some(module_icon(app, id, &setting.name)),
                        Icon::Glyph(icon) => Some(setting_icon(icon, SETTING_ICON - 4.0)),
                    };
                    let mut row = list_row(setting.name).value(setting.value);
                    if let Some(icon) = icon {
                        row = row.leading(
                            view()
                                .size(px(SETTING_ICON))
                                .items_center()
                                .justify_center()
                                .child(icon),
                        );
                    }
                    if setting.icon == Icon::Look
                        && let Some(id) = open_look.clone()
                    {
                        row = row.page(move || app.push(Route::LookDetail(id.clone())));
                    }
                    rows.push(row);
                }
            }
        }
        if !rows.is_empty() {
            sections.push(list_section(rows).header(title).into_element());
        }
    }
    let separator = tr("recipes.list_separator", &[]);
    let unchanged: Vec<ListRow> = [
        (tr("recipes.keep", &[]), keep.join(&separator)),
        (tr("recipes.unchanged_off", &[]), off.join(&separator)),
        (tr("recipes.unchanged_zero", &[]), zero.join(&separator)),
    ]
    .into_iter()
    .filter(|(_, names)| !names.is_empty())
    .map(|(title, names)| list_row(title).subtitle(names))
    .collect();
    if !unchanged.is_empty() {
        sections.push(
            list_section(unchanged)
                .header(tr("recipes.unchanged", &[]))
                .into_element(),
        );
    }
    sections
}

/// The pixel icon of GRKit module `id` from the camera, else the initial of
/// `name` on the module tile.
fn module_icon(app: AppState, id: &str, name: &str) -> AnyElement {
    let initial = name
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default();
    crate::ui::module_icon_tile(
        app.module_icon(id).as_deref(),
        &initial,
        SETTING_ICON,
        false,
    )
}

/// The top of a recipe or preset page: its tile, name, the line under it
/// and, for the current recipe, Current. No sample photo: the page of its
/// Film Look shows that, and the settings come first here.
fn page_top(tile: AnyElement, name: String, sub: String, current: bool) -> AnyElement {
    view()
        .px_4()
        .pt_4()
        .items_center()
        .gap(px(6.0))
        .child(view().pb(px(4.0)).child(tile))
        .child(
            text(name)
                .text_size(px(crate::ui::ty::TITLE1))
                .font_weight(FontWeight::BOLD)
                .text_center()
                .role(AccessibilityRole::Header),
        )
        .child(
            text(sub)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .text_center()
                .text_color(move || token("list-secondary")),
        )
        .child(current.then(|| row_chip(tr("recipes.current", &[]), ChipTone::Tint)))
        .into_element()
}

/// The Film Look of a recipe on the camera.
fn recipe_look(app: AppState, item: &RecipeItem) -> Option<crate::domain::Look> {
    item.effect_id.as_deref().and_then(|id| app.look(id))
}

/// The look setting of a recipe: its Film Look or its Image Control type.
fn look_row(item_look: &Look, effect: Option<String>, looks_on: bool) -> Setting {
    let (name, value, state) = match item_look {
        Look::Native(native) => (
            tr("recipes.image_control", &[]),
            native_type_name(native.image_type),
            State::Set,
        ),
        Look::Effect { .. } => (
            tr("recipes.film_look", &[]),
            effect.unwrap_or_else(|| {
                tr(
                    if looks_on {
                        "recipes.look_missing"
                    } else {
                        "recipes.needs_look"
                    },
                    &[],
                )
            }),
            State::Set,
        ),
        _ => (
            tr("recipes.film_look", &[]),
            tr("recipes.keep", &[]),
            State::Keep,
        ),
    };
    Setting {
        name,
        value,
        state,
        icon: Icon::Look,
    }
}

pub fn recipe_page(app: AppState, id: u32, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    // The page shows the name under the tile, so the bar has no title.
    crate::ui::photo_page(|| false)
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
    let open_look = recipe_look(app, &item).map(|look| look.id);
    let settings = settings_view(
        app,
        groups(
            &item.recipe,
            look_row(
                &item.recipe.look,
                item.effect_name.clone(),
                app.looks_available(),
            ),
        ),
        recipe_tile(app, &item, SETTING_ICON, false),
        open_look,
    );
    let rest = page_rest()
        .child((item.ready && (!item.current || applying)).then(|| {
            button(
                tr("recipes.apply", &[]),
                ButtonKind::Primary,
                ButtonSize::Large,
                move || app.apply_recipe(id),
            )
            .loading(applying)
        }))
        // The Look is on the card only: the recipe applies after the Look
        // is on the camera again.
        .child(
            (!item.ready && item.effect_stored)
                .then(|| crate::ui::list_note(tr("recipes.look_stored_note", &[])).text_center()),
        )
        .children(settings)
        .child(edit_steps(item.current));
    let mut shown: Vec<ListRow> =
        vec![
            list_row(tr("recipes.show_in_menu", &[])).trailing(system_switch(
                enabled != toggling,
                tr("recipes.show_in_menu", &[]),
                editable,
                toggling,
                move |on| app.set_recipe_enabled(id, on),
            )),
        ];
    if hidden {
        shown.push(list_row(tr("recipes.in_adj", &[])).value(tr("recipes.hidden", &[])));
    }
    shown.push(list_row(tr("recipes.rename", &[])).press(move || app.ask_rename_recipe(id)));
    let mut shown = list_section(shown);
    if hidden {
        shown = shown.footer(tr("recipes.hidden_note", &[]));
    }
    view()
        .child(page_top(
            recipe_tile(app, &item, HERO_TILE, false),
            name.clone(),
            sub,
            item.current,
        ))
        .child(
            rest.child(shown)
                .child(list_section([list_row(tr("recipes.delete", &[]))
                    .destructive()
                    .busy(deleting)
                    .press(move || confirm_delete_recipe(app, id, name.clone()))])),
        )
}

/// The width of the tile at the top of a recipe or preset page.
const HERO_TILE: f32 = 88.0;

/// How to change a recipe's settings: the camera saves recipes, so the
/// phone shows the steps. The current recipe's settings are already on the
/// camera, so its steps start at the camera.
fn edit_steps(current: bool) -> impl IntoElement {
    let keys: &[&'static str] = if current {
        &["recipes.step2", "recipes.step3"]
    } else {
        &["recipes.step1", "recipes.step2", "recipes.step3"]
    };
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
                    .bg(move || token("tint"))
                    .child(
                        text(n.to_string())
                            .text_size(px(crate::ui::ty::CAPTION))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xFFFFFF)),
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
        .rounded(px(SECTION_RADIUS))
        .py(px(14.0))
        .px_4()
        .gap(px(10.0))
        .bg(move || token("list-card"))
        .role(AccessibilityRole::Text)
        .label(format!(
            "{}: {}",
            tr("recipes.steps_title", &[]),
            keys.iter()
                .map(|k| tr(k, &[]))
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .child(
            text(tr("recipes.steps_title", &[]))
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD),
        )
        .children(
            keys.iter()
                .enumerate()
                .map(|(i, key)| step(i as u8 + 1, key).into_element()),
        )
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
        .page_title(move || tr("recipes.presets_title", &[]), false)
        .on_dismiss(on_dismiss)
        .child(page_body(move || {
            view().px_4().pt_4().child(presets_content(app, filter))
        }))
}

fn presets_content(app: AppState, filter: Signal<Filter>) -> impl IntoElement {
    let chosen = filter.get();
    let chips = system_segmented(
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
    let state: Option<AnyElement> = if none && app.release_checking() {
        Some(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .px_4()
                .child(quiet_spinner(16.0))
                .child(crate::ui::list_note(tr("recipes.presets_loading", &[])).px_0())
                .into_element(),
        )
    } else if none {
        Some(
            notice_card(tr("recipes.presets_offline", &[]))
                .action(button(
                    tr("looks.retry", &[]),
                    ButtonKind::Tinted,
                    ButtonSize::Small,
                    move || app.refresh_update_info(),
                ))
                .into_element(),
        )
    } else {
        None
    };
    let has_presets = !presets.is_empty();
    let rows = presets.into_iter().map(|preset| preset_row(app, preset));
    view()
        .gap(px(16.0))
        .child(chips)
        .child(crate::ui::list_note(tr(
            "recipes.presets_note",
            &[("model", &model)],
        )))
        .child(state)
        .child(has_presets.then(|| list_section(rows)))
}

/// The tile of a preset: the cover of its official Look, from the camera or,
/// before the preset adds it, from the site catalog; else its type.
fn preset_tile(app: AppState, preset: &PresetItem, size: f32) -> AnyElement {
    let look = preset.effect.as_deref().and_then(|id| app.look(id));
    match (&preset.recipe.look, look) {
        (_, Some(look)) => cover_thumb(&look.shown_cover(), size, false),
        (Look::Native(native), None) => {
            crate::ui::image_control_tile(native.image_type, size, false)
        }
        _ => match catalog_cover(app, preset) {
            Some(cover) => cover_thumb(&cover, size, false),
            None => native_tile(preset.recipe.name.clone(), size, false),
        },
    }
}

/// The cover of the official Look of `preset` in the site catalog.
fn catalog_cover(app: AppState, preset: &PresetItem) -> Option<grkit_core::cover::Cover> {
    let id = preset.effect.as_deref()?;
    app.catalog_items()?
        .into_iter()
        .find(|item| item.look.id == id)
        .map(|item| item.look.shown_cover())
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
    let language = crate::strings::app_language();
    // A preset without an author is GRKit's own, said in the same form.
    let author = preset.author.as_deref().unwrap_or("GRKit");
    let by = tr("recipes.by", &[("name", author)]);
    match preset.text_in(language) {
        Some(text) => format!("{by} · {text}"),
        None => by,
    }
}

/// A preset: its tile, name, author and words. It opens the preset; Add
/// puts it on the camera, with the official Look it needs.
fn preset_row(app: AppState, preset: PresetItem) -> ListRow {
    let added = app.preset_added(&preset);
    let busy = app.recipe_busy();
    let adding = busy.as_deref() == Some(preset.id.as_str());
    let missing = missing_look(app, &preset);
    let (id, open_id, name) = (
        preset.id.clone(),
        preset.id.clone(),
        preset.recipe.name.clone(),
    );
    let mut sub = preset_text(&preset);
    if let Some(look) = missing.filter(|_| !added) {
        sub = format!("{sub}\n{}", tr("recipes.adds_look", &[("look", &look)]));
    }
    let row = list_row(name.clone())
        .subtitle(sub)
        .leading(crate::ui::recipe_badge(
            preset_tile(app, &preset, 40.0),
            40.0,
            false,
        ))
        .page(move || app.push(Route::RecipePreset(open_id.clone())));
    if added {
        row.trailing(row_chip(tr("recipes.on_camera", &[]), ChipTone::Neutral))
    } else {
        row.trailing(
            button(
                tr("recipes.add", &[]),
                ButtonKind::Tinted,
                ButtonSize::Small,
                move || app.add_preset(id.clone()),
            )
            .label(tr("recipes.add_named", &[("name", &name)]))
            .enabled(busy.is_none())
            .loading(adding),
        )
    }
}

pub fn preset_page(app: AppState, id: String, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    let find = move |id: &str| app.recipe_presets().into_iter().find(|p| p.id == id);
    // The page shows the name under the tile, so the bar has no title.
    crate::ui::photo_page(|| false)
        .on_dismiss(on_dismiss)
        .child(page_body(move || {
            find(&id).map(|preset| preset_content(app, preset))
        }))
}

/// The Film Look of a preset on the camera.
fn preset_look(app: AppState, preset: &PresetItem) -> Option<crate::domain::Look> {
    preset.effect.as_deref().and_then(|id| app.look(id))
}

fn preset_content(app: AppState, preset: PresetItem) -> impl IntoElement {
    let added = app.preset_added(&preset);
    let missing = missing_look(app, &preset);
    let look = preset_look(app, &preset);
    let look_name = missing
        .clone()
        .or_else(|| look.as_ref().map(|l| l.name.clone()));
    let id = preset.id.clone();
    let adding = app.recipe_working(&id, RecipeAction::Add);
    let settings = settings_view(
        app,
        groups(
            &preset.recipe,
            look_row(&preset.recipe.look, look_name, true),
        ),
        preset_tile(app, &preset, SETTING_ICON),
        look.map(|look| look.id),
    );
    let rest = page_rest()
        .child(if added {
            crate::ui::list_note(tr("recipes.on_camera", &[]))
                .text_center()
                .into_element()
        } else {
            button(
                tr("recipes.add_to_camera", &[]),
                ButtonKind::Primary,
                ButtonSize::Large,
                move || app.add_preset(id.clone()),
            )
            .enabled(adding || app.recipe_busy().is_none())
            .loading(adding)
            .into_element()
        })
        .child(missing.map(|look| notice_card(tr("recipes.adds_look_note", &[("look", &look)]))))
        .children(settings);
    view()
        .child(page_top(
            preset_tile(app, &preset, HERO_TILE),
            preset.recipe.name.clone(),
            preset_text(&preset),
            false,
        ))
        .child(rest)
}
