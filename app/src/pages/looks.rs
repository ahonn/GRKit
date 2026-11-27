//! Looks tab: the camera's Looks in camera menu order, and the covers of the
//! built-in Looks. The cover grid parts serve the Add page.

use nsh_base::alert_dialog;
use nsh_core::element::ImageSrc;
use nsh_core::element::header::HeaderItem;
use nsh_core::prelude::*;

use crate::ui::ty::text;

use crate::domain::{Look, LookSource, group_of};
use crate::store::{AppState, Route, Tab};
use crate::strings::tr;
use crate::ui::PageTitle;
use crate::ui::{
    ButtonKind, ButtonSize, CaptionTone, RowGestures, SECTION_RADIUS, button, check_circle,
    cover_cell, cover_grid, cover_grid_of, cover_thumb, drag_handle, empty_state, glyph,
    header_buttons, header_icon, header_text, list, list_row, list_section, notice_card,
    quiet_spinner, reorder_list, token,
};

const PLUS: &str = "M12 5v14M5 12h14";
/// Up and down arrows: sort.
const SORT: &str = "M7 4v16M3 8l4-4 4 4M17 20V4M13 16l4 4 4-4";
pub const ROW_GAP: f32 = 18.0;
/// The Looks on the card only show as covers alone, smaller than the
/// sample photos of the camera's Looks.
const STORED_COLUMNS: u16 = 4;

pub fn looks_root(app: AppState) -> impl IntoElement {
    // Edit shows the checks and the handles; Browse opens and applies.
    let editing = signal(false);
    crate::ui::app_page()
        .page_title(move || tr("looks.title", &[]), true)
        .header_right_items(move || native_header(app, editing))
        .header_right(dynamic(move || {
            can_add(app).then(|| {
                edit_add_buttons(
                    editing,
                    can_edit(app),
                    tr("looks.add_title", &[]),
                    move || app.open_add(),
                )
            })
        }))
        .child(looks_content(app, editing))
}

/// Add opens with a camera whose Looks the phone read.
fn can_add(app: AppState) -> bool {
    app.camera_online() && app.looks_blocked().is_none() && app.looks_snapshot().is_some()
}

/// Edit shows with Looks to change.
fn can_edit(app: AppState) -> bool {
    can_add(app) && app.looks_snapshot().is_some_and(|s| !s.looks.is_empty())
}

/// Edit (or Done) and Add in the system bar (iOS), listed from the
/// trailing edge.
fn native_header(app: AppState, editing: Signal<bool>) -> Vec<HeaderItem> {
    if !can_add(app) {
        return Vec::new();
    }
    if editing.get() {
        return vec![
            HeaderItem::new(tr("recipes.done", &[]))
                .key("done")
                .on_press(move || editing.set(false)),
        ];
    }
    let mut items = vec![
        HeaderItem::symbol("plus")
            .key("add")
            .label(tr("looks.add_title", &[]))
            .on_press(move || app.open_add()),
    ];
    if can_edit(app) {
        items.push(sort_item(editing));
    }
    items
}

/// Sort: the icon that shows the checks and the handles. Beside Add, iOS
/// draws the two icons as one group.
pub(crate) fn sort_item(editing: Signal<bool>) -> HeaderItem {
    HeaderItem::symbol("arrow.up.arrow.down")
        .key("edit")
        .label(tr("looks.sort", &[]))
        .on_press(move || editing.set(true))
}

/// Edit or Done and Add in the header (Android). Add leaves while
/// the list is edited.
pub(crate) fn edit_add_buttons(
    editing: Signal<bool>,
    editable: bool,
    add_label: String,
    on_add: impl Fn() + 'static,
) -> AnyElement {
    let on = editing.get();
    let mut buttons = Vec::new();
    if editable {
        buttons.push(if on {
            header_text(tr("recipes.done", &[]), move || editing.set(false))
        } else {
            header_icon(SORT, tr("looks.sort", &[]), true, move || editing.set(true))
        });
    }
    if !on {
        buttons.push(header_icon(PLUS, add_label, true, on_add));
    }
    header_buttons(buttons).into_element()
}

/// The scroll view stays while the Looks change: only its content follows
/// them, so a change does not move the list back to the top.
fn looks_content(app: AppState, editing: Signal<bool>) -> impl IntoElement {
    crate::pages::page_scroll()
        .refresh(Some(crate::store::page_refresh::LOOKS.to_owned()))
        .top_of(Some(crate::store::Tab::Looks.key().to_owned()))
        .flex_1()
        .bg(move || token("list-background"))
        .child(
            view()
                .px_4()
                .pt_4()
                .pb(px(crate::ui::TAB_BAR_CLEARANCE))
                .gap(px(ROW_GAP))
                .child(dynamic(move || on_camera(app, editing.get()))),
        )
}

fn on_camera(app: AppState, editing: bool) -> AnyElement {
    if !app.camera_online() {
        return match app.offline_looks() {
            Some(snapshot) => offline_looks(app, snapshot).into_element(),
            None => no_camera(app, "looks.no_camera_body").into_element(),
        };
    }
    let Some(snapshot) = app.looks_snapshot() else {
        if app.looks_error().is_some() {
            return empty_state(glyph::WARNING, tr("looks.read_failed_title", &[]))
                .action(tr("looks.retry", &[]), move || app.refresh_looks())
                .into_element();
        }
        return loading_card(tr("looks.loading", &[])).into_element();
    };
    let blocked = app.looks_blocked().is_some();
    let editable = !blocked;
    let mode = if editing && editable {
        RowMode::Edit
    } else {
        RowMode::Browse
    };
    let pending = snapshot.looks.iter().any(|look| look.pending.is_some());
    let (shown, hidden): (Vec<Look>, Vec<Look>) = snapshot
        .looks
        .iter()
        .cloned()
        .partition(|look| look.enabled);
    let shown = if mode == RowMode::Edit {
        edit_list(app, shown)
    } else {
        look_grid(app, shown, mode)
    };
    view()
        .gap(px(ROW_GAP))
        .child(dynamic(move || {
            app.looks_error().map(|_| {
                refresh_notice(app.looks_refreshing(), move || app.refresh_looks()).into_element()
            })
        }))
        .child(blocked.then(|| banner(tr("looks.pending_title", &[]), "looks.error.pending_body")))
        .child(pending.then(|| restart_banner(app, &snapshot.looks, editable)))
        .child(section_counted(
            tr("looks.menu", &[]),
            camera_count(&snapshot),
            move || app.looks_refreshing(),
            view()
                .gap_2()
                // A full camera says so under the count: moving Looks off
                // makes room.
                .child(snapshot.camera_full().then(|| {
                    text(tr("looks.camera_full_body", &[]))
                        .px_4()
                        .text_size(px(crate::ui::ty::FOOTNOTE))
                        .line_height(px(18.0))
                        .text_color(move || token("list-secondary"))
                }))
                .child(shown),
            None,
        ))
        .child((!hidden.is_empty()).then(|| {
            let content = if mode == RowMode::Edit {
                let last = hidden.len() - 1;
                list(
                    hidden
                        .into_iter()
                        .enumerate()
                        .map(|(index, look)| edit_row(app, look, index == last, None))
                        .collect(),
                )
                .into_element()
            } else {
                look_grid(app, hidden, mode)
            };
            section(
                tr("looks.off_section", &[]),
                || false,
                content,
                (mode == RowMode::Edit).then(|| tr("looks.off_edit_note", &[])),
            )
        }))
        .child(
            (mode != RowMode::Edit)
                .then(|| stored_section(app, snapshot.stored, mode))
                .flatten(),
        )
        .into_element()
}

/// How many Looks the camera holds of the most it takes, for a camera that
/// keeps Looks on the card only.
fn camera_count(snapshot: &crate::domain::LooksSnapshot) -> Option<Count> {
    snapshot.max_camera.map(|max| {
        let full = snapshot.camera_full();
        let words = tr(
            if full {
                "looks.camera_count_full"
            } else {
                "looks.camera_count"
            },
            &[
                ("count", &snapshot.looks.len().to_string()),
                ("max", &max.to_string()),
            ],
        );
        Count { words, alert: full }
    })
}

/// The count at the end of a section title. An alert count, such as a full
/// camera, takes the tint.
pub(crate) struct Count {
    pub words: String,
    pub alert: bool,
}

/// The Looks on the card only, as covers alone, four in a row.
fn stored_section(app: AppState, looks: Vec<Look>, mode: RowMode) -> Option<impl IntoElement> {
    if looks.is_empty() {
        return None;
    }
    let cells = looks.into_iter().map(|look| {
        let cell = cover_cell(look.shown_cover(), look.name.clone())
            .test_id(format!("stored:{}", look.id));
        if mode == RowMode::Browse {
            let id = look.id.clone();
            cell.open(move || app.push(Route::LookDetail(id.clone())))
        } else {
            cell
        }
    });
    Some(section(
        tr("looks.stored_section", &[]),
        || false,
        cover_grid_of(STORED_COLUMNS, cells),
        Some(tr("looks.stored_note", &[])),
    ))
}

/// The Looks as the phone last read them, while no camera is connected:
/// read-only, with a way to connect.
fn offline_looks(app: AppState, snapshot: crate::domain::LooksSnapshot) -> impl IntoElement {
    let count = camera_count(&snapshot);
    let stored = snapshot.stored;
    let (shown, hidden): (Vec<Look>, Vec<Look>) =
        snapshot.looks.into_iter().partition(|look| look.enabled);
    let has_hidden = !hidden.is_empty();
    view()
        .gap(px(ROW_GAP))
        .child(offline_card(app))
        .child(section_counted(
            tr("looks.menu", &[]),
            count,
            || false,
            look_grid(app, shown, RowMode::ReadOnly),
            None,
        ))
        .child(has_hidden.then(|| {
            section(
                tr("looks.off_section", &[]),
                || false,
                look_grid(app, hidden, RowMode::ReadOnly),
                None,
            )
        }))
        .child(stored_section(app, stored, RowMode::ReadOnly))
}

/// Not connected, and the list below is the last one read: Connect.
pub(crate) fn offline_card(app: AppState) -> impl IntoElement {
    notice_card(tr("looks.offline_title", &[]))
        .subtitle(tr("looks.offline_body", &[]))
        .action(dynamic(move || {
            button(
                tr("looks.connect", &[]),
                ButtonKind::Tinted,
                ButtonSize::Small,
                move || {
                    app.set_tab(Tab::Camera);
                    app.connect_saved();
                },
            )
            .loading(app.camera_restarting())
        }))
}

/// A list that is being read for the first time: a placeholder row with a
/// spinner and no copy. `message` is what a screen reader says.
pub(crate) fn loading_card(message: String) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .justify_center()
        .min_h(px(52.0))
        .rounded(px(SECTION_RADIUS))
        .bg(move || token("list-card"))
        .label(message)
        .child(quiet_spinner(16.0))
}

/// The last read of a list failed; the list below is the one read before.
pub(crate) fn refresh_notice(loading: bool, on_retry: impl Fn() + 'static) -> impl IntoElement {
    notice_card(tr("looks.refresh_failed", &[])).action(
        button(
            tr("looks.retry", &[]),
            ButtonKind::Tinted,
            ButtonSize::Small,
            on_retry,
        )
        .loading(loading),
    )
}

/// A titled group with a note, in the words of a list section's header and
/// footer.
pub(crate) fn section(
    title: String,
    working: impl Fn() -> bool + 'static,
    content: impl IntoElement,
    note: Option<String>,
) -> impl IntoElement {
    section_counted(title, None, working, content, note)
}

/// A [`section`] with a count at the end of its title line.
pub(crate) fn section_counted(
    title: String,
    count: Option<Count>,
    working: impl Fn() -> bool + 'static,
    content: impl IntoElement,
    note: Option<String>,
) -> impl IntoElement {
    let words = |copy: String| {
        text(copy)
            .text_size(px(crate::ui::ty::FOOTNOTE))
            .line_height(px(18.0))
            .text_color(move || token("list-secondary"))
    };
    view()
        .gap_2()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .px_4()
                .child(words(title).flex_1().role(AccessibilityRole::Header))
                // Its own dynamic: a read does not redraw the list.
                .child(dynamic(move || working().then(|| quiet_spinner(14.0))))
                .child(count.map(|count| {
                    let alert = count.alert;
                    words(count.words).when(alert, |words| words.text_color(move || token("tint")))
                })),
        )
        .child(content)
        .child(note.map(|note| words(note).px_4()))
}

/// The Looks of a group as covers. Browse opens a Look when it is
/// pressed; ReadOnly only shows them.
fn look_grid(app: AppState, looks: Vec<Look>, mode: RowMode) -> AnyElement {
    let cells = looks.into_iter().map(|look| {
        let mut cell = cover_cell(look.shown_cover(), look.name.clone())
            .photo(app.look_preview(&look.id).map(ImageSrc::from))
            .faded(!look.enabled)
            .test_id(format!("look:{}", look.id));
        if let Some((caption, tone)) = look_caption(&look) {
            cell = cell.caption(caption, tone);
        }
        if mode == RowMode::Browse {
            let id = look.id.clone();
            cell = cell.open(move || app.push(Route::LookDetail(id.clone())));
        }
        cell
    });
    cover_grid(cells)
}

/// What shows under a Look's name: a change that waits for a restart
/// first, then the Look in use, then a Look the camera did not load.
fn look_caption(look: &Look) -> Option<(String, CaptionTone)> {
    if look.pending.is_some() {
        Some((tr("looks.after_restart", &[]), CaptionTone::Pending))
    } else if look.selected && look.enabled {
        Some((tr("looks.in_use", &[]), CaptionTone::Current))
    } else if look.enabled && !look.loaded {
        Some((tr("looks.not_loaded", &[]), CaptionTone::Plain))
    } else {
        None
    }
}

/// The shown Looks in Edit. A row whose handle is held lifts and follows
/// the finger; the rows it passes move aside, so the gap shows where it
/// lands. The camera menu takes the order where it is let go.
fn edit_list(app: AppState, looks: Vec<Look>) -> AnyElement {
    let count = looks.len();
    let rows = looks.into_iter().map(Some).collect::<Vec<_>>();
    let rows = std::cell::RefCell::new(rows);
    reorder_list(
        count,
        count > 1 && app.look_busy().is_none(),
        move |from, to| app.move_shown_look(from, to),
        move |index, gestures| {
            let look = rows.borrow_mut()[index].take().expect("each row once");
            edit_row(app, look, index + 1 == count, gestures)
        },
    )
    .into_element()
}

/// A Look in Edit: the check that shows or hides it in the camera menu,
/// its cover and name, and the handle that moves it.
fn edit_row(app: AppState, look: Look, last: bool, gestures: Option<RowGestures>) -> AnyElement {
    // The camera shows or hides only a Look that it lists.
    let toggleable = look.listed() && app.look_busy().is_none();
    let working = app.look_row_busy(&look.id);
    let toggle_label = tr(
        if look.enabled {
            "looks.hide_named"
        } else {
            "looks.show_named"
        },
        &[("name", &look.name)],
    );
    let (id, enabled) = (look.id.clone(), look.enabled);
    let leading = view()
        .flex_row()
        .items_center()
        // The check's touch area reaches into the card's margin.
        .ml(px(-10.0))
        .gap(px(4.0))
        .child(check_circle(
            look.enabled,
            toggle_label,
            toggleable,
            working,
            move || app.set_look_enabled(id.clone(), !enabled),
        ))
        .child(cover_thumb(&look.shown_cover(), ROW_COVER, !look.enabled));
    let mut row = list_row(look.name.clone()).leading(leading);
    if let Some((caption, _)) = look_caption(&look) {
        row = row.subtitle(caption);
    }
    if gestures.is_some() {
        row = row.trailing(drag_handle(look.name.clone(), gestures));
    }
    view()
        .bg(move || token("list-card"))
        .child(row.into_row(last))
        .into_element()
}

/// The side of the cover in a row of an edited list.
pub(crate) const ROW_COVER: f32 = 40.0;

/// What a list row offers: Browse opens the item and holds for its menu;
/// Edit shows the check and the drag handle; ReadOnly only shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowMode {
    Browse,
    Edit,
    ReadOnly,
}

/// The most pending changes that the restart notice lists by name.
const RESTART_LISTED: usize = 5;

/// New and changed Looks wait: the camera reads its Look files when it
/// starts. The notice lists each change, and Restart Camera turns the camera
/// off and on, then connects again.
fn restart_banner(app: AppState, looks: &[Look], enabled: bool) -> impl IntoElement {
    let pending: Vec<&Look> = looks.iter().filter(|look| look.pending.is_some()).collect();
    let title = match pending.len() {
        1 => tr("looks.restart_one", &[]),
        n => tr("looks.restart_many", &[("count", &n.to_string())]),
    };
    let mut items: Vec<String> = pending
        .iter()
        .take(RESTART_LISTED)
        .map(|look| {
            tr(
                "looks.pending_item",
                &[("name", &look.name), ("change", &pending_change(look))],
            )
        })
        .collect();
    if pending.len() > RESTART_LISTED {
        let more = (pending.len() - RESTART_LISTED).to_string();
        items.push(tr("looks.pending_more", &[("count", &more)]));
    }
    notice_card(title).items(items).action_below(
        button(
            tr("looks.restart_now", &[]),
            ButtonKind::Primary,
            ButtonSize::Small,
            move || confirm_restart(app),
        )
        .enabled(enabled)
        .loading(app.camera_restarting()),
    )
}

/// The change of a Look that waits for the restart, in words: put back,
/// added, or what changed in it.
fn pending_change(look: &Look) -> String {
    use crate::domain::looks::{LookEdit, PendingChange};
    if look.pending == Some(PendingChange::Add) {
        let key = if look.edits.contains(&LookEdit::Place) {
            "looks.change.put_back"
        } else {
            "looks.change.added"
        };
        return tr(key, &[]);
    }
    let words: Vec<String> = look
        .edits
        .iter()
        .filter_map(|edit| match edit {
            LookEdit::Name => Some("looks.change.renamed"),
            LookEdit::Cover => Some("looks.change.cover"),
            LookEdit::Content => Some("looks.change.content"),
            LookEdit::Place | LookEdit::Unknown => None,
        })
        .map(|key| tr(key, &[]))
        .collect();
    if words.is_empty() {
        tr("looks.change.changed", &[])
    } else {
        words.join(&tr("looks.change.join", &[]))
    }
}

fn confirm_restart(app: AppState) {
    spawn(async move {
        let answer = alert_dialog()
            .title(tr("looks.restart_confirm_title", &[]))
            .description(tr("looks.restart_confirm_body", &[]))
            .cancel("cancel", tr("looks.cancel", &[]))
            .action("restart", tr("looks.restart_confirm", &[]))
            .show()
            .await;
        if answer.ok().and_then(|a| a.action).as_deref() == Some("restart") {
            app.restart_camera_now();
        }
    });
}

fn banner(title: String, body: &'static str) -> impl IntoElement {
    notice_card(title).subtitle(tr(body, &[]))
}

/// The kind of a Look on the camera: its film group, or where it came from.
pub fn look_kind(look: &Look) -> String {
    match look.source {
        LookSource::Official => tr(group_of(&look.id, false).label_key(), &[]),
        LookSource::Custom => tr("looks.imported", &[]),
        LookSource::Native => tr("looks.native", &[]),
        LookSource::Unknown => tr("looks.unknown", &[]),
    }
}

/// No camera connected and no list read before: what the page shows once
/// connected (`body`, a string key), and Connect.
pub(crate) fn no_camera(app: AppState, body: &str) -> impl IntoElement {
    empty_state(glyph::CAMERA, tr("looks.no_camera_title", &[]))
        .body(tr(body, &[]))
        .action(tr("looks.connect", &[]), move || {
            app.set_tab(Tab::Camera);
            app.connect_saved();
        })
}

/// The sheet behind the Import tile: what kind of file to import.
pub fn import_sheet(app: AppState) -> impl IntoElement {
    // The two kinds of file as rows of the shared lists.
    let option = move |title: String, ext: String| {
        list_row(title)
            .subtitle(ext)
            .page(move || app.begin_import())
    };
    crate::ui::sheet_shell(
        tr("looks.import_title", &[]),
        Some(tr("looks.import_body", &[])),
        None,
        true,
        move || app.set_sheet(None),
        list_section([
            option(tr("looks.import_lut", &[]), tr("looks.import_lut_ext", &[])),
            option(
                tr("looks.import_preset", &[]),
                tr("looks.import_preset_ext", &[]),
            ),
        ]),
    )
}
