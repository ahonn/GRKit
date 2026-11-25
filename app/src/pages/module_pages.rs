//! The page of one module and the page of its ADJ options. Rows come from
//! the camera's catalog; a new value shows once the camera confirms it.

use nsh_core::prelude::*;

use crate::domain::modules::{CatalogSetting, Managed, ModuleChoices, ModuleRow};
use crate::pages::modules::{module_tile, summary_text};
use crate::store::{AppState, ModulePart, Route, Tab};
use crate::strings::tr;
use crate::ui::PageTitle;
use crate::ui::{
    Choice, Choices, RowGestures, check_circle, drag_handle, list, list_note, list_row,
    list_section, reorder_list, system_switch, token,
};

const HERO_TILE: f32 = 84.0;

fn page_body<E: IntoElements>(content: impl Fn() -> E + 'static) -> impl IntoElement {
    crate::ui::page_scroll()
        .flex_1()
        .bg(move || token("list-background"))
        .child(
            view()
                .px_4()
                .pt_4()
                .pb(px(crate::pages::look_pages::TAB_BAR_CLEARANCE))
                .gap(px(24.0))
                .child(dynamic(content)),
        )
}

pub fn module_page(app: AppState, id: String, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    let title_id = id.clone();
    crate::ui::app_page()
        .page_title(
            move || {
                app.module_row(&title_id)
                    .map(|row| row.title().to_owned())
                    .unwrap_or_default()
            },
            false,
        )
        .on_dismiss(on_dismiss)
        .child(page_body(move || {
            app.module_row(&id).map(|row| module_content(app, row))
        }))
}

fn module_content(app: AppState, row: ModuleRow) -> impl IntoElement {
    let id = row.id().to_owned();
    let managed = row.managed();
    let mut content = view()
        .gap(px(24.0))
        .child(view().items_center().pt_1().child(module_tile(
            app,
            id,
            HERO_TILE,
            !row.state.enabled,
        )))
        .child(enable_card(app, &row));
    if let Some(kind) = managed {
        let tab = match kind {
            Managed::Looks => Tab::Looks,
            Managed::Recipes => Tab::Recipes,
        };
        if app.tab_available(tab) {
            content = content.child(managed_card(app, &row, kind));
        }
        return content;
    }
    for (group, settings) in row.setting_groups() {
        let title = group.unwrap_or_else(|| tr("modules.settings", &[]));
        let rows = settings
            .into_iter()
            .map(|setting| setting_row(app, &row, setting));
        content = content.child(list_section(rows).header(title));
    }
    if let Some(options) = row.choices() {
        content = content.child(options_row(app, &row, &options));
    }
    content
}

fn enable_card(app: AppState, row: &ModuleRow) -> impl IntoElement {
    let id = row.id().to_owned();
    let on = row.state.enabled;
    let title = tr("modules.enabled", &[]);
    let saving = app.module_part_busy(&id, &ModulePart::Enabled);
    let toggle = system_switch(
        on != saving,
        title.clone(),
        app.modules_editable() && !row.paused() && !row.safe(),
        saving,
        move |on| app.set_module_enabled(&id, on),
    );
    let section = list_section([list_row(title).trailing(toggle)]);
    if row.settings().is_empty() {
        section
    } else {
        section.footer(tr("modules.enabled_note", &[]))
    }
}

/// A managed module: its current item, and the way to the tab that manages
/// it.
fn managed_card(app: AppState, row: &ModuleRow, kind: Managed) -> impl IntoElement {
    let (tab, action) = match kind {
        Managed::Looks => (Tab::Looks, tr("modules.manage_looks", &[])),
        Managed::Recipes => (Tab::Recipes, tr("modules.manage_recipes", &[])),
    };
    let current = Some(summary_text(app, row))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "—".to_owned());
    list_section([
        list_row(tr("modules.current", &[])).value(current),
        list_row(action).page(move || {
            app.pop_to(tab, 0);
            app.set_tab(tab);
        }),
    ])
}

/// A setting: its value, and the menu of its values while the camera takes
/// changes.
fn setting_row(app: AppState, row: &ModuleRow, setting: CatalogSetting) -> crate::ui::ListRow {
    let id = row.id().to_owned();
    let current = row.value_of(&setting);
    let shown = current
        .and_then(|v| setting.value_label(v))
        .unwrap_or_default()
        .to_owned();
    let saving = app.module_part_busy(&id, &ModulePart::Setting(setting.key.clone()));
    let list_row = list_row(setting.label.clone()).value(shown).busy(saving);
    if !app.modules_editable() {
        return list_row;
    }
    let setting = row.choosable(&setting);
    list_row.menu(value_menu(app, id, setting, current))
}

/// The values of a setting as a menu, the current one checked.
pub fn value_menu(
    app: AppState,
    id: String,
    setting: CatalogSetting,
    current: Option<i64>,
) -> Choices {
    let choices = setting
        .values
        .iter()
        .map(|value| {
            Choice::new(value.value.to_string(), value.label.clone())
                .checked(current == Some(value.value))
        })
        .collect();
    Choices::new(setting.label.clone(), choices, move |chosen| {
        if let Some(value) = chosen.parse::<i64>().ok().filter(|v| Some(*v) != current) {
            app.set_module_value(&id, &setting.key, value);
        }
    })
}

fn options_row(app: AppState, row: &ModuleRow, options: &ModuleChoices) -> AnyElement {
    let id = row.id().to_owned();
    let shown = options.choices.shown.len();
    let total = shown + options.choices.hidden.len();
    list_section([list_row(tr("modules.options", &[]))
        .value(format!("{shown} / {total}"))
        .page(move || app.push(Route::ModuleOptions(id.clone())))])
    .header(tr("modules.adj", &[]))
    .footer(tr("modules.options_note", &[]))
    .into_element()
}

pub fn module_options_page(
    app: AppState,
    id: String,
    on_dismiss: impl Fn() + 'static,
) -> impl IntoElement {
    crate::ui::app_page()
        .page_title(move || tr("modules.options", &[]), false)
        .on_dismiss(on_dismiss)
        .child(page_body(move || {
            app.module_row(&id)
                .and_then(|row| row.choices())
                .map(|options| options_content(app, id.clone(), options))
        }))
}

fn options_content(app: AppState, id: String, options: ModuleChoices) -> impl IntoElement {
    let ModuleChoices {
        setting,
        ordered,
        choices,
    } = options;
    let editable = app.modules_editable();
    let one_left = choices.shown.len() == 1;
    let count = choices.shown.len();
    let shown_rows = {
        let labels: Vec<(i64, String)> = choices
            .shown
            .iter()
            .map(|v| (*v, value_name(&setting, *v)))
            .collect();
        let row_id = id.clone();
        let make = move |index: usize, gestures: Option<RowGestures>| {
            let (value, name) = labels[index].clone();
            option_row(
                app,
                row_id.clone(),
                value,
                name,
                true,
                editable && !one_left,
                ordered,
                gestures,
                index + 1 == count,
            )
        };
        if ordered {
            // `reorder_list` takes a `Copy` callback.
            let move_id = stored(id.clone());
            reorder_list(
                count,
                editable && count > 1,
                move |from, to| move_id.with(|id| app.move_module_option(id, from, to)),
                make,
            )
            .into_element()
        } else {
            list((0..count).map(|index| make(index, None)).collect()).into_element()
        }
    };
    let hidden_count = choices.hidden.len();
    let hidden_rows = (hidden_count > 0).then(|| {
        let rows = choices
            .hidden
            .iter()
            .enumerate()
            .map(|(index, value)| {
                option_row(
                    app,
                    id.clone(),
                    *value,
                    value_name(&setting, *value),
                    false,
                    editable,
                    false,
                    None,
                    index + 1 == hidden_count,
                )
            })
            .collect();
        view()
            .gap_2()
            .child(list_note(tr("modules.not_shown", &[])).role(AccessibilityRole::Header))
            .child(list(rows))
    });
    let note = tr(
        if ordered {
            "modules.shown_note_ordered"
        } else {
            "modules.shown_note"
        },
        &[],
    );
    view()
        .gap(px(24.0))
        .child(
            view()
                .gap_2()
                .child(list_note(tr("modules.shown", &[])).role(AccessibilityRole::Header))
                .child(shown_rows)
                .child(list_note(note)),
        )
        .child(hidden_rows)
}

fn value_name(setting: &CatalogSetting, value: i64) -> String {
    setting
        .value_label(value)
        .map_or_else(|| value.to_string(), str::to_owned)
}

#[allow(clippy::too_many_arguments)]
fn option_row(
    app: AppState,
    id: String,
    value: i64,
    name: String,
    shown: bool,
    enabled: bool,
    movable: bool,
    gestures: Option<RowGestures>,
    last: bool,
) -> AnyElement {
    let check_label = tr(
        if shown {
            "modules.hide_named"
        } else {
            "modules.show_named"
        },
        &[("name", &name)],
    );
    let handle_name = name.clone();
    let working = app.module_part_busy(&id, &ModulePart::Option(value));
    let check = view()
        // The check's touch area reaches into the card's margin.
        .ml(px(-10.0))
        .child(check_circle(
            shown,
            check_label,
            enabled,
            working,
            move || app.toggle_module_option(&id, value),
        ));
    let mut row = list_row(name).leading(check);
    if movable && gestures.is_some() {
        row = row.trailing(drag_handle(handle_name, gestures));
    }
    view()
        .bg(move || token("list-card"))
        .child(row.into_row(last))
        .into_element()
}
