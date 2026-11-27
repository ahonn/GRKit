//! Pages pushed from the Looks tab: one Look, import.
//! The cover editor is in `cover_editor`.

use nsh_base::{alert_dialog, input};
use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use crate::ui::ty::text;

use grkit_core::{color::Encoding, cover::Cover, fit::Grade};

use crate::domain::{Look, LookSource, looks::NAME_MAX};
use crate::store::{AppState, ImportDraft, LookAction, Route};
use crate::strings::tr;
use crate::ui::{
    ButtonKind, ButtonSize, ChipTone, ListRow, SECTION_RADIUS, button, chip, cover_thumb,
    detail_header, list_note, list_row, list_section, loading_content, notice_card, system_switch,
    token,
};
use crate::ui::{PageTitle, TAB_BAR_CLEARANCE};

/// The side of the large cover of the import result.
const PAGE_COVER: f32 = 148.0;
/// The side of the cover in a settings row.
const SMALL_COVER: f32 = 32.0;

/// The large cover at the top of a page, or an empty square without one.
fn page_cover(cover: Option<Cover>) -> impl IntoElement {
    view().items_center().py_2().child(match cover {
        Some(cover) => cover_thumb(&cover, PAGE_COVER, false),
        None => view()
            .size(px(PAGE_COVER))
            .rounded(px(10.0))
            .bg(move || token("list-card"))
            .into_element(),
    })
}

// --- One Look ---------------------------------------------------------------

pub fn look_page(app: AppState, id: String, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    let title_id = id.clone();
    // The photo reaches under the bar; the title shows on the page.
    crate::ui::photo_page(|| true)
        .title(move || app.look(&title_id).map(|l| l.name).unwrap_or_default())
        .on_dismiss(on_dismiss)
        .child(dynamic(move || match app.look(&id) {
            Some(look) => look_content(app, look).into_element(),
            None => view()
                .flex_1()
                .bg(move || token("list-background"))
                .into_element(),
        }))
}

/// Whether the camera has no room for one more Look, with the note that
/// says so when the camera reports how many it holds.
pub(crate) fn camera_room(app: AppState) -> (bool, Option<String>) {
    let snapshot = app.looks_snapshot();
    let full = snapshot.as_ref().is_some_and(|s| s.camera_full());
    let note = snapshot
        .as_ref()
        .and_then(|s| s.max_camera.map(|max| (s.looks.len(), max)))
        .filter(|_| full)
        .map(|(count, max)| {
            tr(
                "looks.camera_full_note",
                &[("count", &count.to_string()), ("max", &max.to_string())],
            )
        });
    (full, note)
}

fn look_content(app: AppState, look: Look) -> impl IntoElement {
    let busy = app.look_busy().is_some();
    let blocked = app.looks_blocked().is_some();
    let editable = !busy && !blocked;
    let kind = crate::pages::looks::look_kind(&look);
    let source = match look.source {
        LookSource::Official => tr("looks.official", &[]),
        LookSource::Custom => tr("looks.imported", &[]),
        LookSource::Native => tr("looks.native", &[]),
        LookSource::Unknown => tr("looks.unknown", &[]),
    };
    // Where it is from, its film group, and a change that waits.
    let mut subtitle = match look.source {
        LookSource::Official => format!("{source} · {kind}"),
        _ => source,
    };
    if look.pending.is_some() {
        subtitle = format!("{subtitle} · {}", tr("looks.after_restart", &[]));
    }
    if look.stored {
        subtitle = tr("looks.stored_subtitle", &[]);
    }

    let mut header = detail_header(look.name.clone())
        .subtitle(subtitle)
        .cover(look.shown_cover());
    if look.selected {
        header = header.chip(chip(tr("looks.in_use", &[]), ChipTone::Tint));
    }
    // The sample photo through the Look, once it is made.
    if let Some(photo) = app.look_preview(&look.id) {
        header = header.photo(ImageSrc::from(photo));
    }

    let delete = list_row(tr("looks.delete", &[]))
        .destructive()
        .busy(app.look_working(&look.id, LookAction::Delete));
    let delete = if editable {
        let look = look.clone();
        delete.press(move || confirm_delete(app, look.clone()))
    } else {
        delete
    };
    let page = |content: AnyElement| {
        view().flex_1().bg(move || token("list-background")).child(
            crate::pages::page_scroll().flex_1().child(
                view().pb(px(TAB_BAR_CLEARANCE)).child(header).child(
                    view()
                        .px_4()
                        .pt(px(24.0))
                        .gap(px(28.0))
                        .child(content)
                        .child(list_section([delete])),
                ),
            ),
        )
    };
    let snapshot = app.looks_snapshot();
    let moving = app.look_working(&look.id, LookAction::Move);
    // On the card only: the one action puts it back on the camera.
    if look.stored {
        let (full, full_note) = camera_room(app);
        let note = full_note.unwrap_or_else(|| tr("looks.move_to_camera_note", &[]));
        let id = look.id.clone();
        let action = view()
            .gap_2()
            .child(
                button(
                    tr("looks.move_to_camera", &[]),
                    ButtonKind::Primary,
                    ButtonSize::Large,
                    move || app.move_look(id.clone(), false, false),
                )
                .enabled(editable && !full)
                .loading(moving),
            )
            .child(list_note(note));
        return page(
            view()
                .gap(px(28.0))
                .child(action)
                .child(used_by_section(app, &look.id))
                .into_element(),
        );
    }

    // The camera shows or hides only a Look that it lists.
    let saving = app.look_working(&look.id, LookAction::Toggle);
    let toggle_id = look.id.clone();
    let mut settings: Vec<ListRow> =
        vec![
            list_row(tr("looks.show_in_menu", &[])).trailing(system_switch(
                look.enabled != saving,
                tr("looks.show_in_menu", &[]),
                editable && look.listed(),
                saving,
                move |on| app.set_look_enabled(toggle_id.clone(), on),
            )),
        ];
    if look.editable() {
        let id = look.id.clone();
        let row = list_row(tr("looks.name", &[]))
            .value(look.name.clone())
            .busy(app.look_working(&look.id, LookAction::Rename));
        settings.push(if editable {
            row.page(move || app.ask_rename_look(id.clone()))
        } else {
            row
        });
        let id = look.id.clone();
        let row = list_row(tr("looks.cover", &[]))
            .trailing(cover_thumb(&look.shown_cover(), SMALL_COVER, false))
            .busy(
                app.look_working(&look.id, LookAction::Cover)
                    || app.look_working(&look.id, LookAction::DefaultCover),
            );
        settings.push(if editable {
            row.page(move || app.push(Route::LookCover(id.clone())))
        } else {
            row
        });
    }
    // A camera that keeps Looks on the card only can take this one off.
    let to_card = snapshot
        .as_ref()
        .is_some_and(|s| s.max_camera.is_some())
        .then(|| {
            let row = list_row(tr("looks.move_to_card", &[])).busy(moving);
            let row = if editable {
                let look = look.clone();
                row.press(move || confirm_move_to_card(app, look.clone()))
            } else {
                row
            };
            list_section([row]).footer(tr("looks.move_to_card_note", &[]))
        });
    page(
        view()
            .gap(px(28.0))
            .child(list_section(settings).footer(tr("looks.show_note", &[])))
            .child(to_card)
            .into_element(),
    )
}

/// Ask before a Look leaves the camera when a recipe uses it or it is the
/// Look in use; else move it at once.
pub(crate) fn confirm_move_to_card(app: AppState, look: Look) {
    if look.used_by == 0 && !look.selected {
        app.move_look(look.id, true, false);
        return;
    }
    spawn(async move {
        let body = if look.used_by != 0 {
            move_blocked_body(&look)
        } else {
            tr("looks.move_active_body", &[])
        };
        let answer = alert_dialog()
            .title(tr("looks.move_title", &[("name", &look.name)]))
            .description(body)
            .cancel("cancel", tr("looks.cancel", &[]))
            .action("move", tr("looks.move_action", &[]))
            .show()
            .await;
        if matches!(answer, Ok(r) if r.action.as_deref() == Some("move")) {
            app.move_look(look.id.clone(), true, look.selected);
        }
    });
}

/// The recipes and dial modes that cannot apply while `look` is off the
/// camera.
fn move_blocked_body(look: &Look) -> String {
    let (names, more) = crate::domain::looks::use_labels(
        look,
        |name| tr("looks.use_recipe", &[("name", name)]),
        |mode| tr("looks.use_dial", &[("mode", &mode.to_string())]),
    );
    if names.is_empty() {
        return tr("looks.move_used_body", &[]);
    }
    let mut list = names.join(&tr("looks.use_separator", &[]));
    if more > 0 {
        let total = (names.len() + more).to_string();
        list += &tr(
            "looks.use_more",
            &[("more", &more.to_string()), ("total", &total)],
        );
    }
    capitalized(tr("looks.move_used_names", &[("names", &list)]))
}

/// Ask before deleting. A Look that recipes use cannot be deleted; the Look
/// in use is deleted with the camera switching to Off.
pub fn confirm_delete(app: AppState, look: Look) {
    spawn(async move {
        if look.used_by != 0 {
            let _ = alert_dialog()
                .title(tr("looks.cannot_delete_title", &[("name", &look.name)]))
                .description(delete_blocked_body(&look))
                .cancel("ok", tr("looks.ok", &[]))
                .show()
                .await;
            return;
        }
        let body = if look.selected {
            tr("looks.delete_active_body", &[])
        } else {
            tr("looks.delete_body", &[])
        };
        let answer = alert_dialog()
            .title(tr("looks.delete_title", &[("name", &look.name)]))
            .description(body)
            .cancel("cancel", tr("looks.cancel", &[]))
            .destructive("delete", tr("looks.delete_action", &[]))
            .show()
            .await;
        if matches!(answer, Ok(r) if r.action.as_deref() == Some("delete")) {
            app.delete_look(look.id.clone(), look.selected);
        }
    });
}

/// Why `look` cannot go: the recipes and dial modes that use it, by name,
/// when the camera names them.
pub fn delete_blocked_body(look: &Look) -> String {
    let (names, more) = crate::domain::looks::use_labels(
        look,
        |name| tr("looks.use_recipe", &[("name", name)]),
        |mode| tr("looks.use_dial", &[("mode", &mode.to_string())]),
    );
    if names.is_empty() {
        return tr("looks.cannot_delete_body", &[]);
    }
    let mut list = names.join(&tr("looks.use_separator", &[]));
    if more > 0 {
        let total = (names.len() + more).to_string();
        list += &tr(
            "looks.use_more",
            &[("more", &more.to_string()), ("total", &total)],
        );
    }
    capitalized(tr("looks.cannot_delete_names", &[("names", &list)]))
}

/// The text with its first letter in upper case (no effect on Chinese).
fn capitalized(text: String) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => text,
    }
}

/// A text button in the navigation bar, dimmed and inert when disabled.
pub(crate) fn header_button(
    title: String,
    enabled: bool,
    loading: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .min_h(px(44.0))
        .px_2()
        .justify_center()
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .accessibility_state(move || AccessibilityState {
            disabled: !enabled,
            busy: loading,
            ..AccessibilityState::default()
        })
        .opacity(if enabled || loading { 1.0 } else { 0.4 })
        .when(enabled && !loading, |v| v.on_press(on_press))
        .child(loading_content(
            loading,
            18.0,
            token("tint"),
            text(title)
                .text_size(px(crate::ui::ty::HEADLINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || token("link")),
        ))
}

fn name_field(
    title: String,
    name: Signal<String>,
    on_change: impl Fn(String) + 'static,
    on_submit: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .gap_2()
        .child(crate::ui::list_note(title.clone()).role(AccessibilityRole::Header))
        .child(
            view()
                .flex_row()
                .items_center()
                .min_h(px(48.0))
                .px_4()
                .gap_2()
                .rounded(px(SECTION_RADIUS))
                .bg(move || token("list-card"))
                .child(
                    input()
                        .flex_1()
                        .value(move || name.get())
                        .max_length(NAME_MAX as u32)
                        .auto_correct(false)
                        .label(title)
                        .text_size(px(crate::ui::ty::BODY))
                        .on_change(move |value: &str| on_change(value.to_owned()))
                        .on_submit(move |_: &str| on_submit()),
                )
                .child(
                    text(move || format!("{}/{NAME_MAX}", name.get().len()))
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                        .text_color(move || token("list-secondary")),
                ),
        )
}

// --- Import -------------------------------------------------------------------

/// The LUT inputs, in the order the site offers them.
const INPUTS: [Encoding; 7] = Encoding::ALL;

/// The name of a LUT input; Standard and sRGB with what they are.
fn input_name(input: Encoding, long: bool) -> String {
    match (input, long) {
        (Encoding::Standard, true) => tr("looks.input_standard", &[]),
        (Encoding::Srgb, true) => tr("looks.input_srgb", &[]),
        (Encoding::Standard, false) => "Standard".into(),
        (Encoding::Srgb, false) => "sRGB".into(),
        (Encoding::Linear, _) => "Linear".into(),
        (Encoding::Vlog, _) => "V-Log".into(),
        (Encoding::Flog, _) => "F-Log".into(),
        (Encoding::Flog2, _) => "F-Log2".into(),
        (Encoding::Flog2C, _) => "F-Log2 C".into(),
    }
}

fn grade_summary(grade: Grade) -> String {
    tr(&format!("looks.fit.{}", grade.letter()), &[])
}

pub fn import_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    crate::ui::app_page()
        .page_title(move || tr("looks.new_title", &[]), false)
        .header_right(dynamic(move || {
            let draft = app.import_draft()?;
            (draft.done && !draft.sending).then(|| {
                header_button(tr("looks.import_again", &[]), true, false, move || {
                    app.begin_import()
                })
            })
        }))
        .on_dismiss(move || {
            app.discard_import();
            on_dismiss();
        })
        .child(dynamic(move || import_content(app)))
}

fn import_content(app: AppState) -> AnyElement {
    let Some(draft) = app.import_draft() else {
        return view()
            .flex_1()
            .bg(move || token("list-background"))
            .into_element();
    };
    let base = view().flex_1().bg(move || token("list-background"));
    if let Some(error) = draft.error.clone() {
        let (title, body) = error.keys();
        return base
            .px_4()
            .pt_6()
            .child(
                notice_card(tr(title, &[]))
                    .subtitle(tr(body, &[]))
                    .action(button(
                        tr("looks.choose_again", &[]),
                        ButtonKind::Tinted,
                        ButtonSize::Small,
                        move || app.begin_import(),
                    )),
            )
            .into_element();
    }
    if draft.done {
        return import_result(app, draft).into_element();
    }
    import_form(app, draft).into_element()
}

/// The file, the name, the LUT input and the cover of the new Look.
fn import_form(app: AppState, draft: ImportDraft) -> impl IntoElement {
    let name = signal(draft.name.clone());
    let working = draft.working;
    let preset = draft.converted.is_some_and(|(input, _)| input.is_none());
    let input_value = match (draft.input, draft.converted) {
        (Some(input), _) => input_name(input, false),
        (None, Some((Some(guess), _))) => {
            tr("looks.input_auto", &[("input", &input_name(guess, false))])
        }
        _ => tr("looks.input_reading", &[]),
    };
    let input_note = match draft.input.or(draft.converted.and_then(|(input, _)| input)) {
        Some(input) if !input.is_display() => "looks.input_log_note",
        _ => "looks.input_note",
    };
    let file = list_row(draft.file_name.clone())
        .trailing(
            text(tr("looks.change_file", &[]))
                .text_size(px(crate::ui::ty::BODY))
                .text_color(move || token("tint")),
        )
        .press(move || app.begin_import());
    let mut options = Vec::new();
    if !preset {
        let row = list_row(tr("looks.input", &[])).value(input_value);
        options.push(if draft.sending {
            row
        } else {
            row.page(move || choose_input(app))
        });
    }
    options.push(
        list_row(tr("looks.cover", &[]))
            .trailing(dynamic(move || {
                app.import_cover()
                    .map(|cover| cover_thumb(&cover, SMALL_COVER, false))
            }))
            .page(move || app.push(Route::LookImportCover)),
    );
    let mut options = list_section(options);
    if !preset {
        options = options.footer(tr(input_note, &[]));
    }
    base_scroll(
        view()
            .gap(px(22.0))
            .child(list_section([file]))
            .child(
                view()
                    .gap_2()
                    .child(name_field(
                        tr("looks.import_name", &[]),
                        name,
                        move |value| {
                            name.set(value.clone());
                            app.set_import_name(value);
                        },
                        move || app.finish_import(),
                    ))
                    .child(crate::ui::list_note(tr("looks.name_note", &[]))),
            )
            .child(options)
            .child(crate::ui::list_note(tr("looks.import_fit_note", &[])))
            .child(
                button(
                    tr(
                        if working {
                            "looks.converting_short"
                        } else {
                            "looks.import"
                        },
                        &[],
                    ),
                    ButtonKind::Primary,
                    ButtonSize::Large,
                    move || app.finish_import(),
                )
                .enabled(!working),
            ),
    )
}

/// Choose the LUT input in an action sheet.
fn choose_input(app: AppState) {
    spawn(async move {
        let guess = app
            .import_draft()
            .and_then(|d| d.converted)
            .and_then(|(input, _)| input);
        let auto = match guess {
            Some(guess) => tr("looks.input_auto", &[("input", &input_name(guess, false))]),
            None => tr("looks.input_auto_plain", &[]),
        };
        let mut sheet = alert_dialog()
            .title(tr("looks.input", &[]))
            .action_sheet()
            .action("auto", auto);
        for input in INPUTS {
            sheet = sheet.action(input.name(), input_name(input, true));
        }
        let answer = sheet.cancel("cancel", tr("looks.cancel", &[])).show().await;
        let Ok(answer) = answer else { return };
        match answer.action.as_deref() {
            Some("auto") => app.set_import_input(None),
            Some(name) => {
                if let Ok(input) = name.parse::<Encoding>() {
                    app.set_import_input(Some(input));
                }
            }
            None => {}
        }
    });
}

/// The converted Look: its cover, name, input and grade, and the send.
fn import_result(app: AppState, draft: ImportDraft) -> impl IntoElement {
    let sending = draft.sending;
    let (input, grade) = draft.converted.unwrap_or((None, Grade::D));
    let kind = match input {
        Some(input) => tr(
            "looks.imported_with",
            &[("input", &input_name(input, false))],
        ),
        None => tr("looks.imported_preset", &[]),
    };
    view()
        .flex_1()
        .bg(move || token("list-background"))
        .child(base_scroll(
            view()
                .gap(px(18.0))
                .child(dynamic(move || page_cover(app.import_cover())))
                .child(
                    view()
                        .px_4()
                        .gap(px(2.0))
                        .child(
                            text(draft.name.trim().to_owned())
                                .text_size(px(crate::ui::ty::TITLE2))
                                .font_weight(FontWeight::BOLD),
                        )
                        .child(
                            text(kind)
                                .text_size(px(crate::ui::ty::SUBHEADLINE))
                                .text_color(move || token("list-secondary")),
                        ),
                )
                // How close the camera comes to the LUT, in words: a letter
                // grade would need a scale to read.
                .child(list_section([list_row(grade_summary(grade))]).header(tr("looks.fit", &[]))),
        ))
        .child(
            // Over the floating tab bar, as the main button of the Connect page.
            view().px_4().pb(px(TAB_BAR_CLEARANCE + 20.0)).child(
                button(
                    tr(
                        if sending {
                            "looks.sending"
                        } else {
                            "looks.send"
                        },
                        &[],
                    ),
                    ButtonKind::Primary,
                    ButtonSize::Large,
                    move || app.send_import(),
                )
                .enabled(!sending),
            ),
        )
}

/// A scrolled page body on the grouped background.
fn base_scroll(content: impl IntoElement) -> impl IntoElement {
    view().flex_1().bg(move || token("list-background")).child(
        crate::pages::page_scroll().flex_1().child(
            view()
                .px_4()
                .pt_4()
                .pb(px(TAB_BAR_CLEARANCE))
                .child(content),
        ),
    )
}

/// The recipes that use a Look on the card only: each opens, and none
/// applies until the Look is back on the camera.
fn used_by_section(app: AppState, look_id: &str) -> Option<AnyElement> {
    let recipes: Vec<_> = app
        .recipes_snapshot()?
        .recipes
        .into_iter()
        .filter(|item| item.effect_id.as_deref() == Some(look_id))
        .collect();
    if recipes.is_empty() {
        return None;
    }
    let note = if recipes.len() == 1 {
        "looks.used_by_note"
    } else {
        "looks.used_by_note_many"
    };
    let rows: Vec<ListRow> = recipes
        .into_iter()
        .map(|item| {
            let id = item.id;
            list_row(app.recipe_name(&item))
                .leading(crate::ui::recipe_badge(
                    crate::pages::recipes::recipe_tile(app, &item, 40.0, false),
                    40.0,
                    false,
                ))
                .page(move || app.push(Route::RecipeDetail(id)))
        })
        .collect();
    Some(
        list_section(rows)
            .header(tr("looks.used_by_section", &[]))
            .footer(tr(note, &[]))
            .into_element(),
    )
}
