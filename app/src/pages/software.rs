//! Software Update page (push) for the install entry.

use nsh_base::{nav_page, theme};
use nsh_core::prelude::*;

use crate::domain::{UpdateKind, version_at_least};
use crate::store::AppState;
use crate::strings::tr;
use crate::ui::{gr_mark, grouped_card, info_row, primary_button, secondary_button, token};

pub fn software_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    nav_page()
        .title(move || tr("software.title", &[]))
        .on_dismiss(on_dismiss)
        .child(dynamic(move || software_content(app)))
}

fn software_content(app: AppState) -> impl IntoElement {
    let package = app.package();
    let up_to_date = package.as_ref().is_some_and(|p| {
        p.available
            .as_ref()
            .is_some_and(|latest| version_at_least(&p.installed, latest))
            || (app.demo() && p.available.is_none())
    });
    let available = package.as_ref().and_then(|p| p.available.clone());
    let can_update = available.is_some() && !up_to_date;
    let installed = package
        .as_ref()
        .map(|p| p.installed.clone())
        .unwrap_or_default();
    let notes = package
        .as_ref()
        .map(|p| p.release_notes.clone())
        .unwrap_or_default();
    let model = app.model_name();
    let mb = package
        .as_ref()
        .filter(|p| p.package_bytes > 0)
        .map(|p| format!("{:.1} MB", p.package_bytes as f64 / 1_000_000.0));

    view()
        .flex_1()
        .bg(move || token("grouped"))
        .child(
            crate::ui::page_scroll().flex_1().child(
                view()
                    .gap_5()
                    .px_6()
                    .pt_4()
                    .pb_8()
                    .child(if available.is_none() && app.release_checking() {
                        crate::ui::update_skeleton().into_element()
                    } else {
                        header_card(
                            if up_to_date {
                                installed.clone()
                            } else {
                                available.clone().unwrap_or_else(|| installed.clone())
                            },
                            if up_to_date {
                                tr("software.up_to_date", &[])
                            } else if available.is_none() {
                                tr("software.unavailable", &[])
                            } else {
                                mb.as_ref()
                                    .map(|size| format!("{model} · {size}"))
                                    .unwrap_or_else(|| model.clone())
                            },
                            notes,
                            up_to_date,
                        )
                        .into_element()
                    })
                    .child(if up_to_date {
                        view().into_element()
                    } else {
                        grouped_card(
                            view()
                                .child(info_row(
                                    tr("software.installed", &[]),
                                    installed.clone(),
                                    false,
                                ))
                                .child(
                                    view().px_4().py_3().child(
                                        text(move || tr("software.looks_stay", &[]))
                                            .text_size(px(13.0))
                                            .line_height(px(18.0))
                                            .text_color(move || theme().muted_foreground),
                                    ),
                                ),
                        )
                        .into_element()
                    })
                    .child((!app.demo()).then(|| {
                        grouped_card(
                            view()
                                .p_4()
                                .gap_2()
                                .child(if app.release_checking() {
                                    text(tr("software.checking", &[]))
                                        .text_size(px(15.0))
                                        .text_color(move || theme().muted_foreground)
                                        .into_element()
                                } else {
                                    secondary_button(tr("software.check", &[]), move || {
                                        app.refresh_update_info()
                                    })
                                    .into_element()
                                })
                                .child(app.release_error().map(|message| {
                                    text(message)
                                        .text_size(px(13.0))
                                        .line_height(px(18.0))
                                        .text_color(move || token("tint"))
                                })),
                        )
                    })),
            ),
        )
        .child(can_update.then(|| {
            view()
                .px_6()
                .pb_4()
                .gap_2()
                .child(primary_button(tr("software.update_now", &[]), {
                    let app = app.clone();
                    move || app.begin_update(UpdateKind::Install)
                }))
                .child(
                    text(move || tr("software.confirm_note", &[]))
                        .text_size(px(13.0))
                        .line_height(px(18.0))
                        .text_color(move || theme().muted_foreground)
                        .text_center(),
                )
        }))
}

fn header_card(
    version: String,
    subtitle: String,
    notes: String,
    up_to_date: bool,
) -> impl IntoElement {
    grouped_card(
        view()
            .p_4()
            .gap_3()
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(gr_mark(48.0))
                    .child(
                        view()
                            .flex_1()
                            .gap(px(4.0))
                            .child(
                                text(if version.is_empty() {
                                    "GRKit".into()
                                } else {
                                    format!("GRKit {version}")
                                })
                                .text_size(px(20.0))
                                .font_weight(FontWeight::BOLD),
                            )
                            .child(
                                text(subtitle)
                                    .text_size(px(14.0))
                                    .text_color(move || theme().muted_foreground),
                            ),
                    ),
            )
            .child((!up_to_date && !notes.is_empty()).then(|| {
                view().gap_2().child(
                    text(notes)
                        .text_size(px(15.0))
                        .line_height(px(22.0))
                        .text_color(move || theme().foreground),
                )
            })),
    )
}
