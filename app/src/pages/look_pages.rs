//! Pages pushed from the Looks tab: one Look, import.
//! The cover editor is in `cover_editor`.

use nsh_base::{alert_dialog, input, theme};
use nsh_core::prelude::*;

use crate::ui::ty::text;

use grkit_core::{color::Encoding, cover::Cover, fit::Grade};

use crate::domain::{Look, LookSource, looks::NAME_MAX};
use crate::store::{AppState, ImportDraft, LookAction, Route};
use crate::strings::tr;
use crate::ui::{
    action_row_loading, badge, control_row, cover_view, footnote, grouped_card, loading_content,
    primary_button, secondary_button, toggle_loading, token,
};

/// The side of the large cover of a Look page.
const PAGE_COVER: f32 = 148.0;
/// The side of the cover in a settings row.
const SMALL_COVER: f32 = 32.0;

/// The large cover at the top of a page, or a muted square without one.
fn page_cover(cover: Option<Cover>) -> impl IntoElement {
    view().items_center().py_2().child(match cover {
        Some(cover) => cover_view(&cover, PAGE_COVER, false),
        None => view()
            .size(px(PAGE_COVER))
            .rounded(px(10.0))
            .bg(move || theme().muted)
            .into_element(),
    })
}

/// A settings row that opens a page: its title, the value, and a chevron.
fn link_row(
    title: String,
    label: String,
    value: AnyElement,
    enabled: bool,
    loading: bool,
    last: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .role(AccessibilityRole::Button)
        .label(label)
        .accessibility_state(crate::ui::busy_state(loading))
        .when(enabled && !loading, |v| v.on_press(on_press))
        .child(control_row(
            title,
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(loading_content(loading, 16.0, token("tint"), value))
                .child(crate::ui::chevron()),
            last,
        ))
}

// --- One Look ---------------------------------------------------------------

pub fn look_page(app: AppState, id: String, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    let title_id = id.clone();
    crate::ui::app_page()
        .title(move || app.look(&title_id).map(|l| l.name).unwrap_or_default())
        .on_dismiss(on_dismiss)
        .child(dynamic(move || match app.look(&id) {
            Some(look) => look_content(app, look).into_element(),
            None => view().flex_1().bg(move || token("grouped")).into_element(),
        }))
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

    let mut settings = view().child(control_row(
        tr("looks.show_in_menu", &[]),
        // The camera shows or hides only a Look that it lists.
        toggle_loading(
            look.enabled,
            tr("looks.show_in_menu", &[]),
            editable && look.listed(),
            app.look_working(&look.id, LookAction::Toggle),
            {
                let (id, enabled) = (look.id.clone(), look.enabled);
                move || app.set_look_enabled(id.clone(), !enabled)
            },
        ),
        false,
    ));
    if look.editable() {
        let id = look.id.clone();
        settings = settings.child(link_row(
            tr("looks.name", &[]),
            format!("{}, {}", tr("looks.name", &[]), look.name),
            text(look.name.clone())
                .text_size(px(crate::ui::ty::BODY))
                .text_color(move || theme().muted_foreground)
                .into_element(),
            editable,
            app.look_working(&look.id, LookAction::Rename),
            false,
            move || app.ask_rename_look(id.clone()),
        ));
    }
    if look.editable() {
        let id = look.id.clone();
        settings = settings.child(link_row(
            tr("looks.cover", &[]),
            tr("looks.cover", &[]),
            cover_view(&look.shown_cover(), SMALL_COVER, false),
            editable,
            app.look_working(&look.id, LookAction::Cover)
                || app.look_working(&look.id, LookAction::DefaultCover),
            true,
            move || app.push(Route::LookCover(id.clone())),
        ));
    }
    let actions = view().child(action_row_loading(
        tr("looks.delete", &[]),
        true,
        true,
        app.look_working(&look.id, LookAction::Delete),
        {
            let look = look.clone();
            move || {
                if editable {
                    confirm_delete(app, look.clone())
                }
            }
        },
    ));

    view().flex_1().bg(move || token("grouped")).child(
        crate::ui::page_scroll().flex_1().child(
            view()
                .px_6()
                .pt_4()
                .pb(px(TAB_BAR_CLEARANCE))
                .gap_4()
                .child(page_cover(Some(look.shown_cover())))
                .child(
                    view()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .child(
                                    text(look.name.clone())
                                        .flex_shrink(1.0)
                                        .text_size(px(crate::ui::ty::TITLE2))
                                        .font_weight(FontWeight::BOLD),
                                )
                                .child(look.selected.then(|| badge(tr("looks.in_use", &[])))),
                        )
                        .child(
                            text(subtitle)
                                .text_size(px(crate::ui::ty::SUBHEADLINE))
                                .text_center()
                                .text_color(move || theme().muted_foreground),
                        ),
                )
                .child(
                    view()
                        .gap_2()
                        .child(grouped_card(settings))
                        .child(footnote(tr("looks.show_note", &[]))),
                )
                .child(view().gap_2().child(grouped_card(actions))),
        ),
    )
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
        .child(crate::ui::section_title(title.clone()))
        .child(
            view()
                .flex_row()
                .items_center()
                .min_h(px(50.0))
                .px_4()
                .gap_2()
                .rounded(px(10.0))
                .border_1()
                .border_color(crate::ui::control_border)
                .bg(move || theme().card)
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
                        .text_color(move || theme().muted_foreground),
                ),
        )
}

/// Room under scrolled content for the floating tab bar.
pub const TAB_BAR_CLEARANCE: f32 = 96.0;

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
        .title(move || tr("looks.new_title", &[]))
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
        return view().flex_1().bg(move || token("grouped")).into_element();
    };
    let base = view().flex_1().bg(move || token("grouped"));
    if let Some(error) = draft.error.clone() {
        let (title, body) = error.keys();
        return base
            .px_6()
            .pt_6()
            .child(grouped_card(
                view()
                    .p_4()
                    .gap_2()
                    .child(
                        text(tr(title, &[]))
                            .text_size(px(crate::ui::ty::HEADLINE))
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        text(tr(body, &[]))
                            .text_size(px(crate::ui::ty::SUBHEADLINE))
                            .line_height(px(22.0))
                            .text_color(move || theme().muted_foreground),
                    )
                    .child(secondary_button(tr("looks.choose_again", &[]), move || {
                        app.begin_import()
                    })),
            ))
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
    let file = view()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .min_h(px(50.0))
        .px_4()
        .role(AccessibilityRole::Button)
        .label(format!(
            "{}, {}",
            draft.file_name,
            tr("looks.change_file", &[])
        ))
        .on_press(move || app.begin_import())
        .active(|s| s.opacity(0.72))
        .child(
            text(draft.file_name.clone())
                .flex_1()
                .text_size(px(crate::ui::ty::BODY)),
        )
        .child(
            text(tr("looks.change_file", &[]))
                .text_size(px(crate::ui::ty::BODY))
                .text_color(move || token("link")),
        );
    let mut options = view();
    if !preset {
        options = options.child(link_row(
            tr("looks.input", &[]),
            format!("{}, {input_value}", tr("looks.input", &[])),
            text(input_value.clone())
                .text_size(px(crate::ui::ty::BODY))
                .text_color(move || theme().muted_foreground)
                .into_element(),
            !draft.sending,
            false,
            false,
            move || choose_input(app),
        ));
    }
    options = options.child(link_row(
        tr("looks.cover", &[]),
        tr("looks.cover", &[]),
        dynamic(move || {
            app.import_cover()
                .map(|cover| cover_view(&cover, SMALL_COVER, false))
        })
        .into_element(),
        true,
        false,
        true,
        move || app.push(Route::LookImportCover),
    ));
    base_scroll(
        view()
            .gap(px(22.0))
            .child(grouped_card(file))
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
                    .child(footnote(tr("looks.name_note", &[]))),
            )
            .child(
                view()
                    .gap_2()
                    .child(grouped_card(options))
                    .child((!preset).then(|| footnote(tr(input_note, &[])))),
            )
            .child(footnote(tr("looks.import_fit_note", &[])))
            .child(
                view()
                    .opacity(if working { 0.4 } else { 1.0 })
                    .child(primary_button(
                        tr(
                            if working {
                                "looks.converting_short"
                            } else {
                                "looks.import"
                            },
                            &[],
                        ),
                        move || app.finish_import(),
                    )),
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
        .bg(move || token("grouped"))
        .child(base_scroll(
            view()
                .gap(px(18.0))
                .child(dynamic(move || page_cover(app.import_cover())))
                .child(
                    view()
                        .gap(px(2.0))
                        .child(
                            text(draft.name.trim().to_owned())
                                .text_size(px(crate::ui::ty::TITLE2))
                                .font_weight(FontWeight::BOLD),
                        )
                        .child(
                            text(kind)
                                .text_size(px(crate::ui::ty::SUBHEADLINE))
                                .text_color(move || theme().muted_foreground),
                        ),
                )
                .child(
                    view()
                        .gap_2()
                        .child(grouped_card(control_row(
                            tr("looks.fit", &[]),
                            text(grade.letter())
                                .text_size(px(crate::ui::ty::BODY))
                                .text_color(move || theme().muted_foreground),
                            true,
                        )))
                        .child(footnote(grade_summary(grade))),
                ),
        ))
        .child(view().px_6().pb_6().gap_2().child(primary_button(
            tr(
                if sending {
                    "looks.sending"
                } else {
                    "looks.send"
                },
                &[],
            ),
            move || {
                if !sending {
                    app.send_import()
                }
            },
        )))
}

/// A scrolled page body on the grouped background.
fn base_scroll(content: impl IntoElement) -> impl IntoElement {
    view().flex_1().bg(move || token("grouped")).child(
        crate::ui::page_scroll()
            .flex_1()
            .child(view().px_6().pt_4().pb_8().child(content)),
    )
}
