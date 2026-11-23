//! The page of one module and the page of its ADJ options. Rows come from
//! the camera's catalog; a new value shows once the camera confirms it.

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::domain::modules::{CatalogSetting, Managed, ModuleChoices, ModuleRow};
use crate::pages::looks::{ROW_HEIGHT, RowGestures, drag_handle, list, reorder_list, round_check};
use crate::pages::modules::{module_tile, summary_text};
use crate::store::{AppState, ModulePart, Route, Tab};
use crate::strings::tr;
use crate::ui::ty::text;
use crate::ui::{
    chevron, control_row, footnote, grouped_card, loading_content, section_title, toggle_loading,
    token,
};

const HERO_TILE: f32 = 84.0;

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

fn titled(title: Option<String>, body: impl IntoElement, note: Option<String>) -> AnyElement {
    view()
        .gap_2()
        .child(title.map(|t| view().px(px(2.0)).child(section_title(t))))
        .child(body)
        .child(note.map(footnote))
        .into_element()
}

pub fn module_page(app: AppState, id: String, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    let title_id = id.clone();
    crate::ui::app_page()
        .title(move || {
            app.module_row(&title_id)
                .map(|row| row.title().to_owned())
                .unwrap_or_default()
        })
        .on_dismiss(on_dismiss)
        .child(page_body(move || {
            app.module_row(&id).map(|row| module_content(app, row))
        }))
}

fn module_content(app: AppState, row: ModuleRow) -> impl IntoElement {
    let id = row.id().to_owned();
    let managed = row.managed();
    let mut content = view()
        .gap(px(20.0))
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
        let title = group.or_else(|| Some(tr("modules.settings", &[])));
        let last = settings.len() - 1;
        let mut card = view();
        for (index, setting) in settings.into_iter().enumerate() {
            card = card.child(setting_row(app, &row, setting, index == last));
        }
        content = content.child(titled(title, grouped_card(card), None));
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
    let toggle = toggle_loading(
        on,
        title.clone(),
        app.modules_editable() && !row.paused() && !row.safe(),
        saving,
        move || app.set_module_enabled(&id, !on),
    );
    let note = (!row.settings().is_empty()).then(|| tr("modules.enabled_note", &[]));
    titled(None, grouped_card(control_row(title, toggle, true)), note)
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
    let label = action.clone();
    titled(
        None,
        grouped_card(
            view()
                .child(control_row(
                    tr("modules.current", &[]),
                    text(current)
                        .flex_shrink(1.0)
                        .text_size(px(crate::ui::ty::BODY))
                        .text_color(move || theme().muted_foreground),
                    false,
                ))
                .child(
                    view()
                        .role(AccessibilityRole::Button)
                        .label(label)
                        .on_press(move || {
                            app.pop_to(tab, 0);
                            app.set_tab(tab);
                        })
                        .active(|s| s.opacity(0.72))
                        .child(control_row(action, chevron(), true)),
                ),
        ),
        None,
    )
}

fn setting_row(
    app: AppState,
    row: &ModuleRow,
    setting: CatalogSetting,
    last: bool,
) -> impl IntoElement {
    let id = row.id().to_owned();
    let current = row.value_of(&setting);
    let shown = current
        .and_then(|v| setting.value_label(v))
        .unwrap_or_default()
        .to_owned();
    let saving = app.module_part_busy(&id, &ModulePart::Setting(setting.key.clone()));
    let editable = app.modules_editable();
    let label = format!("{}, {shown}", setting.label);
    let title = setting.label.clone();
    let setting = row.choosable(&setting);
    let trailing = view()
        .flex_row()
        .flex_shrink(1.0)
        .items_center()
        .gap_2()
        .child(loading_content(
            saving,
            18.0,
            theme().muted_foreground,
            text(shown)
                .flex_shrink(1.0)
                .text_size(px(crate::ui::ty::BODY))
                .text_color(move || theme().muted_foreground),
        ))
        .child(editable.then(chevron));
    view()
        .role(AccessibilityRole::Button)
        .label(label)
        .accessibility_state(crate::ui::busy_state(saving))
        .when(editable, |v| {
            v.on_press(move || value_menu(app, id.clone(), setting.clone(), current))
                .active(|s| s.opacity(0.72))
        })
        .child(control_row(title, trailing, last))
}

/// The values of a setting in the system's centered menu.
pub fn value_menu(app: AppState, id: String, setting: CatalogSetting, current: Option<i64>) {
    spawn(async move {
        let mut menu = crate::ui::held_menu(setting.label.clone());
        for value in &setting.values {
            let label = if current == Some(value.value) {
                format!("✓ {}", value.label)
            } else {
                value.label.clone()
            };
            menu = menu.action(value.value.to_string(), label);
        }
        let answer = menu.cancel("cancel", tr("looks.cancel", &[])).show().await;
        let chosen = answer
            .ok()
            .and_then(|a| a.action)
            .and_then(|action| action.parse::<i64>().ok());
        if let Some(value) = chosen.filter(|v| Some(*v) != current) {
            app.set_module_value(&id, &setting.key, value);
        }
    });
}

fn options_row(app: AppState, row: &ModuleRow, options: &ModuleChoices) -> AnyElement {
    let id = row.id().to_owned();
    let title = tr("modules.options", &[]);
    let shown = options.choices.shown.len();
    let total = shown + options.choices.hidden.len();
    let count = format!("{shown} / {total}");
    let label = format!("{title}, {count}");
    let trailing = view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(
            text(count)
                .text_size(px(crate::ui::ty::BODY))
                .text_color(move || theme().muted_foreground),
        )
        .child(chevron());
    titled(
        Some(tr("modules.adj", &[])),
        grouped_card(
            view()
                .role(AccessibilityRole::Button)
                .label(label)
                .on_press(move || app.push(Route::ModuleOptions(id.clone())))
                .active(|s| s.opacity(0.72))
                .child(control_row(title, trailing, true)),
        ),
        Some(tr("modules.options_note", &[])),
    )
}

pub fn module_options_page(
    app: AppState,
    id: String,
    on_dismiss: impl Fn() + 'static,
) -> impl IntoElement {
    crate::ui::app_page()
        .title(move || tr("modules.options", &[]))
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
        titled(Some(tr("modules.not_shown", &[])), list(rows), None)
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
        .gap(px(20.0))
        .child(titled(
            Some(tr("modules.shown", &[])),
            shown_rows,
            Some(note),
        ))
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
    view()
        .flex_row()
        .items_center()
        .min_h(px(ROW_HEIGHT))
        .pr(px(4.0))
        .bg(move || theme().card)
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .child(round_check(
            shown,
            check_label,
            enabled,
            working,
            move || app.toggle_module_option(&id, value),
        ))
        .child(
            text(name)
                .flex_1()
                .text_size(px(crate::ui::ty::BODY))
                .text_color(move || {
                    if shown {
                        theme().foreground
                    } else {
                        theme().muted_foreground
                    }
                }),
        )
        .child((movable && gestures.is_some()).then(|| drag_handle(handle_name, gestures)))
        .into_element()
}
