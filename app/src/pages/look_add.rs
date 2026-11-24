//! The Add page: official Looks that the camera does not have, and the
//! import of a file.

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::ui::ty::text;

use grkit_core::cover::{Cover, Scheme, Variant};

use super::looks::{TILE_COVER, corner_button, corner_icon, grid, tile};
use crate::domain::{LookFilter, PendingChange};
use crate::store::Route;
use crate::store::native_looks::{GR4_FIRMWARE_PAGE, gr4_looks};
use crate::store::{AppState, CatalogItem, LookAction, Sheet};
use crate::strings::tr;
use crate::ui::{grouped_card, section_title, segmented, token};

const PLUS: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3.4" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>"##;
const LOCK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="currentColor"><rect x="5" y="10.5" width="14" height="10" rx="2"/><path d="M8 10.5V8a4 4 0 0 1 8 0v2.5" fill="none" stroke="currentColor" stroke-width="2.6"/></svg>"##;
const CHECK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3.4" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12.5 4.5 4.5L19 7.5"/></svg>"##;

pub fn add_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    crate::ui::app_page()
        .title(tr("looks.add_title", &[]))
        .on_dismiss(on_dismiss)
        .child(
            view().flex_1().bg(move || token("grouped")).child(
                crate::ui::page_scroll().flex_1().child(
                    view()
                        .px_6()
                        .pt_4()
                        .pb(px(crate::pages::look_pages::TAB_BAR_CLEARANCE))
                        .gap(px(super::looks::ROW_GAP))
                        .child(dynamic(move || add_content(app))),
                ),
            ),
        )
}

fn add_content(app: AppState) -> impl IntoElement {
    let filter = app.looks_filter();
    let chips = segmented(
        [
            (LookFilter::All, "looks.filter.all"),
            (LookFilter::Color, "looks.filter.color"),
            (LookFilter::Mono, "looks.filter.mono"),
        ]
        .into_iter()
        .map(|(value, key)| -> crate::ui::Segment {
            (
                tr(key, &[]),
                filter == value,
                Box::new(move || app.set_looks_filter(value)),
            )
        })
        .collect(),
    );
    let snapshot = app.looks_snapshot();
    let full = snapshot.as_ref().is_some_and(|s| s.card_full());
    let column = view()
        .gap(px(super::looks::ROW_GAP))
        .child(import_row(app, full))
        .child(unlock_section(app))
        .child(chips);
    let Some(items) = app.catalog_items() else {
        let Some(message) = app.catalog_error() else {
            return column.child(super::looks::loading_card(tr("looks.catalog_loading", &[])));
        };
        return column.child(grouped_card(
            view().p_4().child(
                text(message)
                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                    .line_height(px(22.0))
                    .text_color(move || theme().muted_foreground),
            ),
        ));
    };
    // A Look added here stays, marked, until the page closes.
    let items: Vec<(CatalogItem, bool)> = items
        .into_iter()
        .filter_map(
            |item| match snapshot.as_ref().and_then(|s| s.get(&item.look.id)) {
                None => Some((item, false)),
                Some(look) if look.pending == Some(PendingChange::Add) => Some((item, true)),
                Some(_) => None,
            },
        )
        .collect();
    let scheme = snapshot
        .as_ref()
        .map(|s| s.official_covers)
        .unwrap_or_default();
    // One grid: the filter picks color, B&W, or both.
    let tiles: Vec<AnyElement> = items
        .iter()
        .filter(|(item, _)| match filter {
            LookFilter::All => true,
            LookFilter::Color => !item.group.monochrome(),
            LookFilter::Mono => item.group.monochrome(),
        })
        .map(|(item, added)| catalog_tile(app, item.clone(), *added, scheme, full).into_element())
        .collect();
    column.child(grid(tiles))
}

/// The cover that a catalog Look gets on this card: the scheme's.
fn scheme_cover(item: &CatalogItem, scheme: Scheme) -> Cover {
    match item.look.film_cover() {
        Some(cover) if scheme.variant(item.look.monochrome) == Variant::FilmBox => cover.film_box(),
        _ => item.look.shown_cover(),
    }
}

/// An official Look: the tile or its corner adds it, or takes back the one
/// just added.
fn catalog_tile(
    app: AppState,
    item: CatalogItem,
    added: bool,
    scheme: Scheme,
    full: bool,
) -> impl IntoElement {
    let id = item.look.id.clone();
    let name = item.look.name.clone();
    let adding = app.catalog_adding(&id);
    let working = adding || app.look_working(&id, LookAction::TakeBack);
    let enabled = app.looks_blocked().is_none() && app.look_busy().is_none() && !adding;
    // A full card greys the adds; pressing one says why.
    let dimmed = full && !added;
    let status = if adding {
        Some((tr("looks.adding", &[]), false))
    } else {
        added.then(|| (tr("looks.after_restart", &[]), false))
    };
    let label = match &status {
        Some((status, _)) => format!("{name}, {status}"),
        None => name.clone(),
    };
    let corner_label = tr(
        if added {
            "looks.added_named"
        } else {
            "looks.add_named"
        },
        &[("name", &name)],
    );
    // The whole tile does what its corner does, so a tap on the cover adds.
    let toggle = move || {
        if added {
            app.take_back_look(id.clone())
        } else {
            app.add_catalog_look(id.clone())
        }
    };
    let on_tile = toggle.clone();
    tile(
        crate::ui::cover_view(&scheme_cover(&item, scheme), TILE_COVER, false),
        name,
        status,
        label,
        enabled.then(|| Box::new(on_tile) as Box<dyn Fn()>),
        corner_button(
            true,
            Some(token("button")),
            corner_icon(if added { CHECK } else { PLUS }),
            corner_label,
            AccessibilityRole::Checkbox,
            Some(added),
            enabled,
            dimmed,
            working,
            toggle,
        ),
    )
}

/// The GR IV looks that a firmware file unlocks, while the camera lacks
/// one of them.
fn unlock_section(app: AppState) -> Option<AnyElement> {
    if !app.gr4_looks_offered() {
        return None;
    }
    let snapshot = app.looks_snapshot()?;
    let looks: Vec<_> = gr4_looks()
        .into_iter()
        .filter(|(id, _, _)| snapshot.get(id).is_none())
        .collect();
    if looks.is_empty() {
        return None;
    }
    let tiles = looks
        .into_iter()
        .map(|(_, name, cover)| {
            let open = move || app.push(Route::LookUnlock);
            tile(
                crate::ui::cover_view(&cover, TILE_COVER, false),
                name.to_owned(),
                None,
                name.to_owned(),
                Some(Box::new(open)),
                corner_button(
                    true,
                    Some(token("button")),
                    corner_icon(LOCK),
                    tr("looks.unlock_title", &[]),
                    AccessibilityRole::Button,
                    None,
                    true,
                    false,
                    false,
                    open,
                ),
            )
            .into_element()
        })
        .collect();
    Some(
        view()
            .gap_2()
            .child(
                view()
                    .px(px(2.0))
                    .child(section_title(tr("looks.unlock_section", &[]))),
            )
            .child(grid(tiles))
            .into_element(),
    )
}

/// Unlock the GR IV looks: choose the firmware file, then put them on the
/// camera.
pub fn unlock_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    crate::ui::app_page()
        .title(tr("looks.unlock_title", &[]))
        .on_dismiss(move || {
            app.discard_native_looks();
            on_dismiss()
        })
        .child(
            view().flex_1().bg(move || token("grouped")).child(
                crate::ui::page_scroll().flex_1().child(
                    view()
                        .px_6()
                        .pt_4()
                        .pb(px(crate::pages::look_pages::TAB_BAR_CLEARANCE))
                        .gap_6()
                        .child(dynamic(move || unlock_content(app))),
                ),
            ),
        )
}

fn unlock_content(app: AppState) -> impl IntoElement {
    let found = app.native_looks_found();
    let covers: Vec<(String, Cover)> = match &found {
        Some((_, looks)) => looks
            .iter()
            .map(|l| (l.name.clone(), l.cover.clone()))
            .collect(),
        None => gr4_looks()
            .into_iter()
            .map(|(_, name, cover)| (name.to_owned(), cover))
            .collect(),
    };
    let row = view()
        .flex_row()
        .justify_center()
        .gap_6()
        .children(covers.into_iter().map(|(name, cover)| {
            view()
                .items_center()
                .gap_2()
                .child(crate::ui::cover_view(&cover, 120.0, false))
                .child(
                    text(name)
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                        .font_weight(FontWeight::SEMIBOLD),
                )
        }));
    let buttons = if found.is_some() {
        view().gap_3().child(crate::ui::primary_button_loading(
            tr("looks.send", &[]),
            app.native_looks_sending(),
            move || app.send_native_looks(),
        ))
    } else {
        view()
            .gap_3()
            .child(crate::ui::primary_button_loading(
                tr("looks.choose_firmware", &[]),
                app.native_looks_reading(),
                move || app.choose_gr4_firmware(),
            ))
            .child(crate::ui::secondary_button(
                tr("looks.open_ricoh", &[]),
                || {
                    spawn(async move {
                        let _ = open_url(GR4_FIRMWARE_PAGE).await;
                    });
                },
            ))
    };
    view().gap_8().child(row).child(buttons)
}

/// The row at the top that imports a file as a new Look.
fn import_row(app: AppState, full: bool) -> impl IntoElement {
    let enabled = app.looks_blocked().is_none() && app.look_busy().is_none();
    grouped_card(
        view()
            .flex_row()
            .items_center()
            .gap_3()
            .min_h(px(64.0))
            .px_4()
            .role(AccessibilityRole::Button)
            .label(tr("looks.import_row", &[]))
            .opacity(if enabled && !full { 1.0 } else { 0.4 })
            .when(enabled, |v| {
                v.on_press(move || {
                    if full {
                        app.alert_look_error(crate::domain::LookError::CardFull)
                    } else {
                        app.set_sheet(Some(Sheet::ImportLook))
                    }
                })
                .active(|s| s.opacity(0.72))
            })
            .child(
                view()
                    .size(px(44.0))
                    .rounded(px(6.0))
                    .border(px(1.5))
                    .border_dashed()
                    .border_color(move || theme().border)
                    .items_center()
                    .justify_center()
                    .child(
                        text("+")
                            .text_size(px(crate::ui::ty::TITLE2))
                            .text_color(move || token("link")),
                    ),
            )
            .child(
                view()
                    .flex_1()
                    .gap(px(2.0))
                    .child(
                        text(tr("looks.import_row", &[]))
                            .text_size(px(crate::ui::ty::HEADLINE))
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        text(tr("looks.import_row_sub", &[]))
                            .text_size(px(crate::ui::ty::FOOTNOTE))
                            .text_color(move || theme().muted_foreground),
                    ),
            )
            .child(crate::ui::chevron()),
    )
}
