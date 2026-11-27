//! The Add page: official Looks that the camera does not have, and the
//! import of a file.

use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use crate::ui::ty::text;

use grkit_core::cover::{Cover, Scheme, Variant};

use crate::domain::{LookFilter, PendingChange};
use crate::store::Route;
use crate::store::native_looks::{Firmware, OfferedLook};
use crate::store::{AppState, CatalogItem, LookAction, Sheet};
use crate::strings::tr;
use crate::ui::PageTitle;
use crate::ui::{
    ButtonKind, ButtonSize, CaptionTone, CoverCell, TileTone, button, cover_cell, cover_grid,
    cover_grid_of, cover_thumb, glyph, icon_tile, list_note, list_row, list_section, notice_card,
    system_segmented, token,
};

pub fn add_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    crate::ui::app_page()
        .page_title(tr("looks.add_title", &[]), false)
        .on_dismiss(on_dismiss)
        .child(
            view().flex_1().bg(move || token("list-background")).child(
                crate::pages::page_scroll().flex_1().child(
                    view()
                        .px_4()
                        .pt_4()
                        .pb(px(crate::ui::TAB_BAR_CLEARANCE))
                        .gap(px(super::looks::ROW_GAP))
                        .child(dynamic(move || add_content(app))),
                ),
            ),
        )
}

fn add_content(app: AppState) -> impl IntoElement {
    let filter = app.looks_filter();
    let chips = system_segmented(
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
        .child(snapshot.as_ref().is_some_and(|s| s.camera_full()).then(|| {
            notice_card(tr("looks.camera_full_title", &[])).subtitle(tr("looks.add_full_body", &[]))
        }))
        .child(import_row(app, full))
        .child(unlock_section(app));
    let Some(items) = app.catalog_items() else {
        let Some(message) = app.catalog_error() else {
            return column
                .child(chips)
                .child(super::looks::loading_card(tr("looks.catalog_loading", &[])));
        };
        return column.child(chips).child(notice_card(message));
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
        .collect::<Vec<_>>();
    // Every official Look is on the camera: nothing to filter.
    if items.is_empty() {
        return column.child(list_note(tr("looks.catalog_all_added", &[])));
    }
    let scheme = snapshot
        .as_ref()
        .map(|s| s.official_covers)
        .unwrap_or_default();
    // One grid: the filter picks color, B&W, or both.
    let cells: Vec<_> = items
        .into_iter()
        .filter(|(item, _)| match filter {
            LookFilter::All => true,
            LookFilter::Color => !item.group.monochrome(),
            LookFilter::Mono => item.group.monochrome(),
        })
        .map(|(item, added)| catalog_cell(app, item, added, scheme))
        .collect();
    let column = column.child(chips);
    if cells.is_empty() {
        return column.child(list_note(tr("looks.catalog_kind_added", &[])));
    }
    column.child(cover_grid(cells))
}

/// The cover that a catalog Look gets on this card: the scheme's.
fn scheme_cover(item: &CatalogItem, scheme: Scheme) -> Cover {
    match item.look.film_cover() {
        Some(cover) if scheme.variant(item.look.monochrome) == Variant::FilmBox => cover.film_box(),
        _ => item.look.shown_cover(),
    }
}

/// An official Look: Add puts it on the camera; Undo takes back the one
/// just added. A full card still takes the press, which says why it fails.
fn catalog_cell(app: AppState, item: CatalogItem, added: bool, scheme: Scheme) -> CoverCell {
    let id = item.look.id.clone();
    let name = item.look.name.clone();
    let adding = app.catalog_adding(&id);
    let enabled = app.looks_blocked().is_none() && app.look_busy().is_none();
    let cell = cover_cell(scheme_cover(&item, scheme), name.clone())
        .photo(app.catalog_preview(&item).map(ImageSrc::from))
        .enabled(enabled || adding)
        .test_id(format!("catalog:{id}"));
    if added {
        let working = app.look_working(&id, LookAction::TakeBack);
        cell.caption(tr("looks.after_restart", &[]), CaptionTone::Pending)
            .undo(
                tr("looks.undo", &[]),
                tr("looks.added_named", &[("name", &name)]),
                working,
                move || app.take_back_look(id.clone()),
            )
            .enabled(enabled || working)
    } else {
        cell.add(
            tr("looks.add_named", &[("name", &name)]),
            adding,
            move || app.add_catalog_look(id.clone()),
        )
    }
}

/// The firmware files that unlock looks the camera lacks: a row for each,
/// with the cover of its first look.
fn unlock_section(app: AppState) -> Option<AnyElement> {
    let rows: Vec<_> = Firmware::ALL
        .into_iter()
        .filter_map(|firmware| {
            let looks = app.unlock_offered(firmware);
            let first = looks.first()?.cover.clone();
            Some(
                list_row(tr(firmware.row_key(), &[]))
                    .subtitle(unlock_summary(&looks))
                    .leading(cover_thumb(&first, 32.0, false))
                    .page(move || app.push(Route::LookUnlock(firmware))),
            )
        })
        .collect();
    if rows.is_empty() {
        return None;
    }
    Some(
        view()
            .gap_2()
            .child(
                text(tr("looks.unlock_section", &[]))
                    .px_4()
                    .text_size(px(crate::ui::ty::FOOTNOTE))
                    .line_height(px(18.0))
                    .text_color(move || token("list-secondary"))
                    .role(AccessibilityRole::Header),
            )
            .child(list_section(rows))
            .into_element(),
    )
}

/// What a firmware file unlocks: its one look by name, else how many color
/// and B&W looks.
fn unlock_summary(looks: &[OfferedLook]) -> String {
    if let [look] = looks {
        return look.name.to_owned();
    }
    let mono = looks.iter().filter(|look| look.monochrome).count();
    let color = looks.len() - mono;
    let key = match (color, mono) {
        (_, 0) => "looks.unlock_color",
        (0, _) => "looks.unlock_mono",
        _ => "looks.unlock_color_mono",
    };
    let (color, mono) = (color.to_string(), mono.to_string());
    tr(key, &[("color", &color), ("mono", &mono)])
}

/// Unlock the looks of `firmware`: choose the firmware file, choose its
/// looks, then put them on the camera.
pub fn unlock_page(
    app: AppState,
    firmware: Firmware,
    on_dismiss: impl Fn() + 'static,
) -> impl IntoElement {
    crate::ui::app_page()
        .page_title(tr(firmware.title_key(), &[]), false)
        .on_dismiss(move || {
            app.discard_native_looks();
            on_dismiss()
        })
        .child(
            view().flex_1().bg(move || token("list-background")).child(
                crate::pages::page_scroll().flex_1().child(
                    view()
                        .px_4()
                        .pt_4()
                        .pb(px(crate::ui::TAB_BAR_CLEARANCE))
                        .gap_6()
                        .child(dynamic(move || unlock_content(app, firmware))),
                ),
            ),
        )
}

/// The covers in rows of four on the unlock page.
const UNLOCK_COLUMNS: u16 = 4;
/// The width of the cover when a firmware file gives one look.
const SINGLE_COVER: f32 = 140.0;

fn unlock_content(app: AppState, firmware: Firmware) -> impl IntoElement {
    let found = app
        .native_looks_found()
        .filter(|found| found.firmware == firmware);
    // Before the file is read: the looks it gives. After: those it gave,
    // each with its check.
    let looks: Vec<(String, String, bool, Cover)> = match &found {
        Some(found) => found
            .looks
            .iter()
            .map(|l| (l.id.clone(), l.name.clone(), l.monochrome, l.cover.clone()))
            .collect(),
        None => app
            .unlock_offered(firmware)
            .into_iter()
            .map(|l| (l.id.to_owned(), l.name.to_owned(), l.monochrome, l.cover))
            .collect(),
    };
    let reading = found.is_some();
    // One look shows larger, in the middle.
    let single = looks.len() == 1;
    let sending = app.native_looks_sending();
    let group = |monochrome: bool| {
        let cells: Vec<_> = looks
            .iter()
            .filter(|(_, _, mono, _)| *mono == monochrome)
            .map(|(id, name, _, cover)| {
                let cell = cover_cell(cover.clone(), name.clone());
                let cell = if single { cell.centered() } else { cell };
                if reading {
                    let id = id.clone();
                    cell.check(app.native_look_chosen(&id), move || {
                        app.toggle_native_look(&id)
                    })
                    .enabled(!sending)
                } else {
                    cell
                }
            })
            .collect();
        (!cells.is_empty()).then_some(cells)
    };
    let groups: Vec<(&str, Vec<_>)> = [(false, "looks.filter.color"), (true, "looks.filter.mono")]
        .into_iter()
        .filter_map(|(mono, key)| group(mono).map(|cells| (key, cells)))
        .collect();
    // One kind of look needs no heading.
    let titled = groups.len() > 1;
    let mut column = view().gap_6();
    if let Some(found) = &found {
        let mut note = tr(
            "looks.firmware_read",
            &[("camera", firmware.camera()), ("version", &found.version)],
        );
        // The chosen looks that the camera has no room for stay on the card.
        let snapshot = app.looks_snapshot();
        let new = looks
            .iter()
            .filter(|(id, ..)| app.native_look_chosen(id))
            .filter(|(id, ..)| snapshot.as_ref().is_none_or(|s| s.get(id).is_none()))
            .count();
        match snapshot.and_then(|s| s.camera_room()) {
            Some(0) if new > 0 => note = format!("{note}{}", tr("looks.unlock_all_stored", &[])),
            Some(room) if new > room => {
                note = format!(
                    "{note}{}",
                    tr(
                        "looks.unlock_overflow",
                        &[
                            ("room", &room.to_string()),
                            ("rest", &(new - room).to_string())
                        ],
                    )
                )
            }
            _ => {}
        }
        column = column.child(
            text(note)
                .px_4()
                .text_size(px(crate::ui::ty::FOOTNOTE))
                .line_height(px(18.0))
                .text_color(move || token("list-secondary")),
        );
    }
    for (key, cells) in groups {
        column = column.child(
            view()
                .gap_2()
                .child(titled.then(|| {
                    text(tr(key, &[]))
                        .px_4()
                        .text_size(px(crate::ui::ty::FOOTNOTE))
                        .line_height(px(18.0))
                        .text_color(move || token("list-secondary"))
                        .role(AccessibilityRole::Header)
                }))
                .child(if single {
                    view()
                        .items_center()
                        .child(view().w(px(SINGLE_COVER)).child(cover_grid_of(1, cells)))
                        .into_element()
                } else {
                    cover_grid_of(UNLOCK_COLUMNS, cells)
                }),
        );
    }
    let buttons = if reading {
        let count = app.native_looks_chosen_count();
        view().gap_3().child(
            button(
                tr("looks.send_count", &[("count", &count.to_string())]),
                ButtonKind::Primary,
                ButtonSize::Large,
                move || app.send_native_looks(),
            )
            .enabled(count > 0)
            .loading(sending),
        )
    } else {
        view()
            .gap_3()
            .child(
                button(
                    tr("looks.choose_firmware", &[]),
                    ButtonKind::Primary,
                    ButtonSize::Large,
                    move || app.choose_firmware(firmware),
                )
                .loading(app.native_looks_reading()),
            )
            .child(button(
                tr("looks.open_ricoh", &[]),
                ButtonKind::Tinted,
                ButtonSize::Large,
                move || {
                    spawn(async move {
                        let _ = open_url(firmware.page()).await;
                    });
                },
            ))
    };
    column.child(buttons)
}

/// The row at the top that imports a file as a new Look. A full card
/// still takes the press, which says why it fails.
fn import_row(app: AppState, full: bool) -> impl IntoElement {
    let enabled = app.looks_blocked().is_none() && app.look_busy().is_none();
    let row = list_row(tr("looks.import_row", &[]))
        .subtitle(tr("looks.import_row_sub", &[]))
        .leading(icon_tile(glyph::PLUS, 32.0, TileTone::Tint));
    let row = if enabled {
        row.page(move || {
            if full {
                app.alert_look_error(crate::domain::LookError::CardFull)
            } else {
                app.set_sheet(Some(Sheet::ImportLook))
            }
        })
    } else {
        row
    };
    view()
        .opacity(if enabled && !full { 1.0 } else { 0.4 })
        .child(list_section([row]))
}
