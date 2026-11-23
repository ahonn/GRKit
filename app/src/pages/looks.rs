//! Looks tab: the camera's Looks in camera menu order, and the covers of the
//! built-in Looks. The cover grid parts serve the Add page.

use nsh_base::{alert_dialog, theme};
use nsh_core::AccessibilityActionDef;
use nsh_core::element::ImageSrc;
use nsh_core::element::header::HeaderItem;
use nsh_core::prelude::*;

use crate::ui::ty::text;

use grkit_core::cover::Scheme;

use crate::domain::{CameraModel, CoverGroup, Look, LookSource, group_of};
use crate::store::{AppState, LookAction, Route, Tab};
use crate::strings::tr;
use crate::ui::{
    action_row_loading, badge, camera_art, circle_button, control_row, cover_view, footnote,
    grouped_card, loading_content, pill_button_loading, primary_button, quiet_spinner,
    secondary_button, token,
};

const PLUS: &str = "M12 5v14M5 12h14";
/// The side of a cover in the grid of the Add page.
pub const TILE_COVER: f32 = 92.0;
/// Columns of a cover grid.
pub const COLUMNS: usize = 3;
pub const COLUMN_GAP: f32 = 12.0;
pub const ROW_GAP: f32 = 18.0;
const CHECK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3.4" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12.5 4.5 4.5L19 7.5"/></svg>"##;

pub fn looks_root(app: AppState) -> impl IntoElement {
    // Edit shows the checks and the handles; Browse opens and applies.
    let editing = signal(false);
    crate::ui::app_page()
        .title(move || tr("looks.title", &[]))
        .header_large_title(!cfg!(target_os = "android"))
        .header_right_items(move || native_header(app, editing))
        .header_right(dynamic(move || {
            can_add(app).then(|| {
                view()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(can_edit(app).then(|| edit_button(editing)))
                    .child((!editing.get()).then(|| {
                        circle_button(PLUS, tr("looks.add_title", &[]), true, move || {
                            app.open_add()
                        })
                    }))
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
        items.push(
            HeaderItem::new(tr("recipes.edit", &[]))
                .key("edit")
                .on_press(move || editing.set(true)),
        );
    }
    items
}

/// Edit or Done in the header (Android). It has the fill and outline of
/// the Add button.
pub(crate) fn edit_button(editing: Signal<bool>) -> impl IntoElement {
    dynamic(move || {
        let on = editing.get();
        let title = tr(if on { "recipes.done" } else { "recipes.edit" }, &[]);
        view()
            .min_h(px(crate::ui::touch_target()))
            .px_4()
            .rounded_full()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(crate::ui::control_border)
            .bg(move || theme().card)
            .role(AccessibilityRole::Button)
            .label(title.clone())
            .on_press(move || editing.set(!on))
            .active(|s| s.opacity(0.72))
            .child(
                text(title)
                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(move || theme().foreground),
            )
    })
}

/// The scroll view stays while the Looks change: only its content follows
/// them, so a change does not move the list back to the top.
fn looks_content(app: AppState, editing: Signal<bool>) -> impl IntoElement {
    crate::ui::page_scroll()
        .flex_1()
        .bg(move || token("grouped"))
        .child(
            view()
                .px_6()
                .pt_4()
                .pb(px(crate::pages::look_pages::TAB_BAR_CLEARANCE))
                .gap(px(ROW_GAP))
                .child(dynamic(move || on_camera(app, editing.get()))),
        )
}

fn on_camera(app: AppState, editing: bool) -> AnyElement {
    if !app.camera_online() {
        return match app.offline_looks() {
            Some(snapshot) => offline_looks(app, snapshot).into_element(),
            None => no_camera(app).into_element(),
        };
    }
    let Some(snapshot) = app.looks_snapshot() else {
        if app.looks_error().is_some() {
            return grouped_card(
                view()
                    .p_4()
                    .gap_3()
                    .child(text(tr("looks.read_failed", &[])).text_size(px(crate::ui::ty::BODY)))
                    .child(secondary_button(tr("looks.retry", &[]), move || {
                        app.refresh_looks()
                    })),
            )
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
    let pending = snapshot
        .looks
        .iter()
        .filter(|look| look.pending.is_some())
        .count();
    let (shown, hidden): (Vec<Look>, Vec<Look>) = snapshot
        .looks
        .iter()
        .cloned()
        .partition(|look| look.enabled);
    let shown_count = list_count(shown.len(), snapshot.max_loaded);
    view()
        .gap(px(ROW_GAP))
        .child(dynamic(move || {
            app.looks_error().map(|_| {
                refresh_notice(app.looks_refreshing(), move || app.refresh_looks()).into_element()
            })
        }))
        .child(blocked.then(|| banner(tr("looks.pending_title", &[]), "looks.error.pending_body")))
        .child((pending > 0).then(|| restart_banner(app, pending, editable)))
        .child(section(
            tr("looks.menu", &[]),
            shown_count,
            move || app.looks_refreshing(),
            menu_list(app, shown, mode),
            (mode != RowMode::Edit).then(|| tr("looks.menu_note", &[])),
        ))
        .child((!hidden.is_empty()).then(|| {
            let count = hidden.len().to_string();
            let last = hidden.len() - 1;
            let rows = hidden
                .into_iter()
                .enumerate()
                .map(|(index, look)| {
                    let gestures = (mode == RowMode::Browse).then(|| RowGestures::menu(app, &look));
                    look_row(app, look, mode, index == last, gestures).into_element()
                })
                .collect();
            section(
                tr("looks.off_section", &[]),
                count,
                || false,
                list(rows),
                (mode == RowMode::Edit).then(|| tr("looks.off_edit_note", &[])),
            )
        }))
        .child((snapshot.has_built_in() && mode == RowMode::Browse).then(|| covers_row(app)))
        .into_element()
}

/// The Looks as the phone last read them, while no camera is connected:
/// read-only, with a way to connect.
fn offline_looks(app: AppState, snapshot: crate::domain::LooksSnapshot) -> impl IntoElement {
    let max = snapshot.max_loaded;
    let (shown, hidden): (Vec<Look>, Vec<Look>) =
        snapshot.looks.into_iter().partition(|look| look.enabled);
    let rows = |looks: Vec<Look>| {
        let last = looks.len().saturating_sub(1);
        list(
            looks
                .into_iter()
                .enumerate()
                .map(|(index, look)| {
                    look_row(app, look, RowMode::ReadOnly, index == last, None).into_element()
                })
                .collect(),
        )
    };
    let shown_count = list_count(shown.len(), max);
    let hidden_count = hidden.len().to_string();
    let has_hidden = !hidden.is_empty();
    view()
        .gap(px(ROW_GAP))
        .child(offline_card(app))
        .child(section(
            tr("looks.menu", &[]),
            shown_count,
            || false,
            rows(shown),
            None,
        ))
        .child(has_hidden.then(|| {
            section(
                tr("looks.off_section", &[]),
                hidden_count,
                || false,
                rows(hidden),
                None,
            )
        }))
}

/// Not connected, and the list below is the last one read: Connect.
pub(crate) fn offline_card(app: AppState) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .rounded(px(10.0))
        .border_1()
        .border_color(move || theme().border)
        .bg(move || theme().card)
        .py_3()
        .pl_4()
        .pr_3()
        .child(
            view()
                .flex_1()
                .gap(px(2.0))
                .child(
                    text(tr("looks.offline_title", &[]))
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    text(tr("looks.offline_body", &[]))
                        .text_size(px(crate::ui::ty::FOOTNOTE))
                        .line_height(px(18.0))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(dynamic(move || {
            pill_button_loading(
                tr("looks.connect", &[]),
                tr("looks.connect", &[]),
                true,
                app.camera_restarting(),
                move || {
                    app.set_tab(Tab::Camera);
                    app.connect_saved();
                },
            )
        }))
}

/// A list that is being read for the first time.
pub(crate) fn loading_card(message: String) -> impl IntoElement {
    grouped_card(
        view()
            .flex_row()
            .items_center()
            .gap_2()
            .p_4()
            .child(quiet_spinner(16.0))
            .child(
                text(message)
                    .flex_1()
                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                    .text_color(move || theme().muted_foreground),
            ),
    )
}

/// The last read of a list failed; the list below is the one read before.
pub(crate) fn refresh_notice(loading: bool, on_retry: impl Fn() + 'static) -> impl IntoElement {
    let retry = tr("looks.retry", &[]);
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .rounded(px(10.0))
        .border_1()
        .border_color(move || theme().border)
        .bg(move || theme().card)
        .py_1()
        .pl_4()
        .pr_2()
        .child(
            text(tr("looks.refresh_failed", &[]))
                .flex_1()
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .text_color(move || theme().muted_foreground),
        )
        .child(pill_button_loading(
            retry.clone(),
            retry,
            true,
            loading,
            on_retry,
        ))
}

/// The count of a camera menu group: how many, and the most it shows.
pub(crate) fn list_count(count: usize, max: usize) -> String {
    tr(
        "looks.count",
        &[("count", &count.to_string()), ("max", &max.to_string())],
    )
}

/// A titled group of covers with its count and a note.
pub(crate) fn section(
    title: String,
    count: String,
    working: impl Fn() -> bool + 'static,
    content: impl IntoElement,
    note: Option<String>,
) -> impl IntoElement {
    view()
        .gap_3()
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .px(px(2.0))
                .child(view().flex_1().child(crate::ui::section_title(title)))
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        // Its own dynamic: a read does not redraw the list.
                        .child(dynamic(move || working().then(|| quiet_spinner(14.0))))
                        .child(
                            text(count)
                                .text_size(px(crate::ui::ty::SUBHEADLINE))
                                .text_color(move || theme().muted_foreground),
                        ),
                ),
        )
        .child(content)
        .child(note.map(footnote))
}

/// Rows of `COLUMNS` tiles, each column the same width.
pub fn grid(tiles: Vec<AnyElement>) -> impl IntoElement {
    let mut rows = view().gap(px(ROW_GAP));
    let mut tiles = tiles.into_iter().peekable();
    while tiles.peek().is_some() {
        let mut row = view().flex_row().gap(px(COLUMN_GAP));
        for _ in 0..COLUMNS {
            row = row.child(view().flex_1().child(tiles.next()));
        }
        rows = rows.child(row);
    }
    rows
}

/// The least height of a row of the Looks lists; rows grow with their text.
pub(crate) const ROW_HEIGHT: f32 = 64.0;
/// The side of the cover in a row.
pub(crate) const ROW_COVER: f32 = 44.0;

/// Rows on a card. Its background shows where a dragged row will land.
pub(crate) fn list(rows: Vec<AnyElement>) -> impl IntoElement {
    let mut card = view()
        .rounded(px(10.0))
        .border_1()
        .border_color(move || theme().border)
        .overflow_hidden()
        .bg(move || theme().muted);
    for row in rows {
        card = card.child(row);
    }
    card
}

/// The shown Looks. In Edit, a row whose handle is held lifts and follows
/// the finger; the rows it passes move aside, so the gap shows where it
/// lands. The camera menu takes the order where it is let go. In Browse,
/// holding a row opens its menu.
fn menu_list(app: AppState, looks: Vec<Look>, mode: RowMode) -> AnyElement {
    let count = looks.len();
    if mode != RowMode::Edit {
        let last = count.saturating_sub(1);
        let rows = looks
            .into_iter()
            .enumerate()
            .map(|(index, look)| {
                let gestures = RowGestures::menu(app, &look);
                look_row(app, look, mode, index == last, Some(gestures)).into_element()
            })
            .collect();
        return list(rows).into_element();
    }
    let rows = looks.into_iter().map(Some).collect::<Vec<_>>();
    let rows = std::cell::RefCell::new(rows);
    reorder_list(
        count,
        count > 1 && app.look_busy().is_none(),
        move |from, to| app.move_shown_look(from, to),
        move |index, gestures| {
            let look = rows.borrow_mut()[index].take().expect("each row once");
            look_row(app, look, RowMode::Edit, index + 1 == count, gestures).into_element()
        },
    )
    .into_element()
}

/// Rows that a handle moves in Edit: the dragged row follows the finger,
/// the rows it passes move aside, so the gap shows where it lands, and
/// `on_move` gets the place where it is let go. `row` makes each row, with
/// the handle's drag when the list is movable.
pub(crate) fn reorder_list(
    count: usize,
    movable: bool,
    on_move: impl Fn(usize, usize) + Copy + 'static,
    row: impl Fn(usize, Option<RowGestures>) -> AnyElement,
) -> impl IntoElement {
    // The row being dragged, and how far.
    let dragging = signal(None::<usize>);
    let offset = signal(0.0f32);
    // Rows grow with the text size, each by its own text.
    let heights = signal(vec![ROW_HEIGHT; count]);
    let target = move || {
        dragging.get().map(|from| {
            let moved = offset.get();
            heights.with(|heights| crate::domain::looks::reorder_target(heights, from, moved))
        })
    };
    let dragged_height = move |from: usize| heights.with(|heights| heights[from]);
    let end = move || {
        dragging.set(None);
        offset.set(0.0);
    };
    let rows = (0..count)
        .map(|index| {
            let lifted = move || dragging.get() == Some(index);
            // A row between the dragged row and its target moves one row
            // toward the place the dragged row left.
            let shift = move || match (dragging.get(), target()) {
                (Some(from), Some(to)) if from < index && index <= to => -dragged_height(from),
                (Some(from), Some(to)) if to <= index && index < from => dragged_height(from),
                _ => 0.0,
            };
            let gestures = movable.then(|| RowGestures {
                long_press: Box::new(|| {}),
                release: None,
                handle: Some(HandleDrag {
                    grant: Box::new(move || {
                        dragging.set(Some(index));
                        offset.set(0.0);
                    }),
                    moved: Box::new(move |dy| offset.set(dy)),
                    release: Box::new(move || {
                        let to = target().unwrap_or(index);
                        end();
                        on_move(index, to);
                    }),
                    terminate: Box::new(end),
                }),
            });
            view()
                .relative()
                .on_layout(move |bounds: &Bounds<Points>| {
                    let height = bounds.size.height.0;
                    if height > 0.0 && heights.with(|h| h[index] != height) {
                        heights.update(|h| h[index] = height);
                    }
                })
                .translate_y(move || px(if lifted() { offset.get() } else { shift() }))
                .transition_transform()
                .duration(move || if lifted() { 0.0_f32 } else { 180.0_f32 })
                .z_index(move || if lifted() { 10 } else { 0 })
                .when(movable, |v| {
                    v.accessibility_actions(vec![
                        AccessibilityActionDef {
                            name: "move_earlier".into(),
                            label: Some(tr("looks.move_up", &[]).into()),
                        },
                        AccessibilityActionDef {
                            name: "move_later".into(),
                            label: Some(tr("looks.move_down", &[]).into()),
                        },
                    ])
                    .on_accessibility_action(move |action: &str| match action {
                        "move_earlier" => on_move(index, index.saturating_sub(1)),
                        "move_later" => on_move(index, (index + 1).min(count - 1)),
                        _ => {}
                    })
                })
                // The shadow of a lifted row.
                .child(
                    view()
                        .absolute()
                        .inset_0()
                        .bg(move || theme().card)
                        .shadow([BoxShadow {
                            offset_x: Points(0.0),
                            offset_y: Points(4.0),
                            blur: Points(14.0),
                            spread: Points(0.0),
                            color: rgba(0x00000033).into(),
                            inset: false,
                        }])
                        .opacity(move || if lifted() { 1.0_f32 } else { 0.0_f32 }),
                )
                .child(row(index, gestures))
                .into_element()
        })
        .collect();
    list(rows)
}

/// What a row says under the name. The Look in use has a chip instead.
fn look_status(look: &Look) -> Option<(String, bool)> {
    if look.pending.is_some() {
        Some((tr("looks.after_restart", &[]), false))
    } else if look.enabled && !look.loaded {
        Some((tr("looks.not_loaded", &[]), false))
    } else {
        None
    }
}

/// A Look on the camera: its cover, name and state, which opens the Look,
/// and the check that shows or hides it in the camera menu.
/// What a held row does: lift it for a drag, or open its menu, and what
/// letting go does.
pub(crate) struct RowGestures {
    pub long_press: Box<dyn Fn()>,
    pub release: Option<Box<dyn Fn()>>,
    /// The drag of a row's handle in Edit, which moves the row.
    pub handle: Option<HandleDrag>,
}

/// What a handle's touch does to its row: it is lifted, follows the
/// finger by `moved`, and lands where it is let go.
pub(crate) struct HandleDrag {
    pub grant: Box<dyn Fn()>,
    pub moved: Box<dyn Fn(f32)>,
    pub release: Box<dyn Fn()>,
    pub terminate: Box<dyn Fn()>,
}

impl RowGestures {
    /// A row that does not move: holding it opens its menu.
    fn menu(app: AppState, look: &Look) -> Self {
        let look = look.clone();
        RowGestures {
            long_press: Box::new(move || look_menu(app, look.clone())),
            release: None,
            handle: None,
        }
    }
}

/// The menu of a held row: show or hide it, rename, cover, delete.
pub fn look_menu(app: AppState, look: Look) {
    look_menu_then(app, look, || {});
}

/// The menu of a held row, and `closed` once it closes.
fn look_menu_then(app: AppState, look: Look, closed: impl FnOnce() + 'static) {
    if app.look_busy().is_some() || app.looks_blocked().is_some() {
        closed();
        return;
    }
    spawn(async move {
        let mut sheet = crate::ui::held_menu(look.name.clone());
        if look.listed() {
            sheet = sheet.action(
                "toggle",
                tr(
                    if look.enabled {
                        "looks.menu_hide"
                    } else {
                        "looks.menu_show"
                    },
                    &[],
                ),
            );
        }
        if look.editable() {
            sheet = sheet
                .action("rename", tr("looks.rename_title", &[]))
                .action("cover", tr("looks.menu_cover", &[]));
        }
        let answer = sheet
            .destructive("delete", tr("looks.delete", &[]))
            .cancel("cancel", tr("looks.cancel", &[]))
            .show()
            .await;
        closed();
        match answer.ok().and_then(|a| a.action).as_deref() {
            Some("toggle") => app.set_look_enabled(look.id.clone(), !look.enabled),
            Some("rename") => app.ask_rename_look(look.id.clone()),
            Some("cover") => app.push(Route::LookCover(look.id.clone())),
            Some("delete") => crate::pages::look_pages::confirm_delete(app, look.clone()),
            _ => {}
        }
    });
}

fn look_row(
    app: AppState,
    look: Look,
    mode: RowMode,
    last: bool,
    gestures: Option<RowGestures>,
) -> impl IntoElement {
    let id = look.id.clone();
    let off = !look.enabled;
    let in_use = look.selected && look.enabled;
    let (status, emphasized) = look_status(&look).unwrap_or_default();
    let mut label = look.name.clone();
    for part in [
        in_use.then(|| tr("looks.in_use", &[])),
        (!status.is_empty()).then(|| status.clone()),
    ]
    .into_iter()
    .flatten()
    {
        label = format!("{label}, {part}");
    }
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
    let (toggle_id, enabled) = (look.id.clone(), look.enabled);
    let handle_label = look.name.clone();
    // In Edit the handle moves the row; in Browse holding the row opens its
    // menu.
    let (row_gestures, handle_gestures) = match mode {
        RowMode::Edit => (None, gestures),
        _ => (gestures, None),
    };
    let movable = handle_gestures.is_some();
    view()
        .flex_row()
        .items_center()
        .min_h(px(ROW_HEIGHT))
        .pr(px(4.0))
        .bg(move || theme().card)
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .child((mode == RowMode::Edit).then(|| {
            round_check(look.enabled, toggle_label, toggleable, working, move || {
                app.set_look_enabled(toggle_id.clone(), !enabled)
            })
        }))
        .child(
            view()
                .flex_1()
                .min_h(px(ROW_HEIGHT))
                .py_1()
                .flex_row()
                .items_center()
                .gap_3()
                .pl(px(if mode == RowMode::Edit { 4.0 } else { 12.0 }))
                .role(AccessibilityRole::Button)
                .label(label)
                .when(mode == RowMode::Browse, |v| {
                    v.on_press(move || app.push(Route::LookDetail(id.clone())))
                        .active(|s| s.opacity(0.72))
                })
                .when_some(row_gestures, |v, gestures| {
                    let v = v.on_long_press(gestures.long_press);
                    match gestures.release {
                        Some(release) => v.on_press_up(move |_: &PressEvent| release()),
                        None => v,
                    }
                })
                .child(cover_view(&look.shown_cover(), ROW_COVER, off))
                .child(
                    view()
                        .flex_1()
                        .gap(px(2.0))
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .child(
                                    text(look.name.clone())
                                        .flex_shrink(1.0)
                                        .text_size(px(crate::ui::ty::HEADLINE))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(move || {
                                            if off {
                                                theme().muted_foreground
                                            } else {
                                                theme().foreground
                                            }
                                        }),
                                )
                                .child(in_use.then(|| badge(tr("looks.in_use", &[])))),
                        )
                        .child((!status.is_empty()).then(|| {
                            text(status)
                                .text_size(px(crate::ui::ty::SUBHEADLINE))
                                .text_color(move || {
                                    if emphasized {
                                        token("tint")
                                    } else {
                                        theme().muted_foreground
                                    }
                                })
                        })),
                ),
        )
        .child(match mode {
            // The width of the chevron's column, so the row does not move.
            RowMode::Browse if working => view()
                .w(px(32.0))
                .items_center()
                .child(quiet_spinner(14.0))
                .into_element(),
            RowMode::Browse => view().px_3().child(crate::ui::chevron()).into_element(),
            RowMode::Edit if movable => drag_handle(handle_label, handle_gestures).into_element(),
            _ => view().into_element(),
        })
}

/// What a list row offers: Browse opens the item and holds for its menu;
/// Edit shows the check and the drag handle; ReadOnly only shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowMode {
    Browse,
    Edit,
    ReadOnly,
}

/// Three lines: hold and drag to move the row.
const HANDLE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="14" viewBox="0 0 20 14" fill="none" stroke="#000000" stroke-width="2" stroke-linecap="round"><path d="M2 2h16M2 7h16M2 12h16"/></svg>"##;

/// The handle at the end of a row in Edit. It takes the touch as the
/// finger goes down, before a scroll view can, so a drag that starts on
/// it moves the row at once.
pub(crate) fn drag_handle(name: String, gestures: Option<RowGestures>) -> impl IntoElement {
    let drag = gestures.and_then(|g| g.handle);
    view()
        .size(px(crate::ui::touch_target()))
        .items_center()
        .justify_center()
        .role(AccessibilityRole::Button)
        .label(tr("looks.drag_named", &[("name", &name)]))
        .when_some(drag, |v, drag| {
            let HandleDrag {
                grant,
                moved,
                release,
                terminate,
            } = drag;
            v.claim_touches(TouchClaim::on_start())
                .keep_touches(true)
                .on_responder_grant(move |_: &ResponderEvent| grant())
                .on_responder_move(move |event: &ResponderEvent| moved(event.gesture.delta.y.0))
                .on_responder_release(move |_: &ResponderEvent| release())
                .on_responder_terminate(move |_: &ResponderEvent| terminate())
        })
        .child(
            image(ImageSrc::svg_markup(HANDLE))
                .w(px(20.0))
                .h(px(14.0))
                .tint_color(move || theme().muted_foreground),
        )
}

/// A cover with its name and an optional status line, and a control at its
/// top right corner.
pub fn tile(
    cover: AnyElement,
    name: String,
    status: Option<(String, bool)>,
    label: String,
    on_press: Option<Box<dyn Fn()>>,
    corner: impl IntoElement,
) -> impl IntoElement {
    let (status, emphasized) = status.unwrap_or_default();
    view()
        .relative()
        .items_center()
        .child(
            view()
                .items_center()
                .gap(px(6.0))
                .role(AccessibilityRole::Button)
                .label(label)
                .when_some(on_press, |v, on_press| {
                    v.on_press(on_press).active(|s| s.opacity(0.72))
                })
                .child(cover)
                .child(
                    text(name)
                        .text_size(px(crate::ui::ty::FOOTNOTE))
                        .line_height(px(17.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_center(),
                )
                .child(
                    text(status)
                        .mt(px(-3.0))
                        .min_h(px(15.0))
                        .text_size(px(crate::ui::ty::CAPTION))
                        .line_height(px(15.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_center()
                        .text_color(move || {
                            if emphasized {
                                token("tint")
                            } else {
                                theme().muted_foreground
                            }
                        }),
                ),
        )
        .child(corner)
}

/// A round control, 44 points to press, drawn at 26 points, or at 22 centered
/// on the top right corner of a tile's cover when `corner`, clear of the cover's
/// brand. `filled` is its color, or `None` for an empty circle.
#[allow(clippy::too_many_arguments)]
pub fn corner_button(
    corner: bool,
    filled: Option<AdaptiveColor>,
    icon: AnyElement,
    label: String,
    role: AccessibilityRole,
    checked: Option<bool>,
    enabled: bool,
    dimmed: bool,
    loading: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let spinner_tint: AdaptiveColor = if filled.is_some() {
        token("button-foreground")
    } else {
        rgb(0x6E6E73).into()
    };
    view()
        .when(corner, |v| {
            // The cover is centered, and its film box starts 3/40 below its top.
            v.absolute()
                .top(px(-21.0))
                .left(relative(0.5))
                .ml(px(TILE_COVER / 2.0 - 22.0))
        })
        .size(px(44.0))
        .items_center()
        .justify_center()
        .role(role)
        .label(label)
        .accessibility_state(move || AccessibilityState {
            checked: checked.map(|on| {
                if on {
                    nsh_core::Checked::True
                } else {
                    nsh_core::Checked::False
                }
            }),
            disabled: !enabled,
            busy: loading,
            ..AccessibilityState::default()
        })
        .opacity(if loading || (enabled && !dimmed) {
            1.0
        } else {
            0.4
        })
        .when(enabled, |v| v.on_press(on_press))
        .child(
            view()
                .size(px(if corner { 22.0 } else { 26.0 }))
                .rounded_full()
                .items_center()
                .justify_center()
                .map(|v| match filled {
                    Some(color) => v.border_2().border_color(rgb(0xFFFFFF)).bg(color),
                    None => v
                        .border(px(1.5))
                        .border_color(crate::ui::control_border)
                        .bg(rgba(0xFFFFFFEB)),
                })
                .shadow([BoxShadow {
                    offset_x: Points(0.0),
                    offset_y: Points(1.0),
                    blur: Points(3.0),
                    spread: Points(0.0),
                    color: rgba(0x0000002E).into(),
                    inset: false,
                }])
                .child(loading_content(loading, 13.0, spinner_tint, icon)),
        )
}

/// A stroke icon for a corner button, in the color of text on a button.
pub fn corner_icon(markup: &str) -> AnyElement {
    view()
        .text_color(move || token("button-foreground"))
        .child(svg(ImageSrc::svg_markup(markup)).size(px(13.0)))
        .into_element()
}

/// Shown in the camera menu: a filled check; hidden: an empty circle.
pub(crate) fn round_check(
    on: bool,
    label: String,
    enabled: bool,
    loading: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let icon = if on {
        corner_icon(CHECK)
    } else {
        view().into_element()
    };
    corner_button(
        false,
        on.then(|| token("button")),
        icon,
        label,
        AccessibilityRole::Checkbox,
        Some(on),
        enabled,
        false,
        loading,
        on_press,
    )
}

/// The row that opens the cover colors of the built-in Looks.
fn covers_row(app: AppState) -> impl IntoElement {
    let title = tr("looks.covers_row", &[]);
    grouped_card(
        view()
            .role(AccessibilityRole::Button)
            .label(title.clone())
            .on_press(move || app.push(Route::LookScheme))
            .active(|s| s.opacity(0.72))
            .child(control_row(title, crate::ui::chevron(), true)),
    )
}

/// Cover Colors (push): the scheme of the built-in Looks without a cover of
/// their own, and a reset of the covers people changed, by group.
pub fn scheme_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    crate::ui::app_page()
        .title(move || tr("looks.covers_row", &[]))
        .on_dismiss(on_dismiss)
        .child(dynamic(move || {
            let Some(snapshot) = app.looks_snapshot() else {
                return view().flex_1().bg(move || token("grouped")).into_element();
            };
            let enabled = app.look_busy().is_none() && app.looks_blocked().is_none();
            let current = snapshot.official_covers;
            let schemes = [Scheme::Gray, Scheme::ColorFilmBox, Scheme::FilmBox];
            let count = schemes.len();
            let options = view().children(schemes.into_iter().enumerate().map(|(i, scheme)| {
                let working =
                    app.look_working(crate::store::COVERS_KEY, LookAction::Scheme(scheme));
                scheme_option(
                    app,
                    scheme,
                    scheme == current,
                    enabled,
                    working,
                    i + 1 == count,
                )
                .into_element()
            }));
            let resets: Vec<(CoverGroup, &str)> = [
                (CoverGroup::Color, "looks.covers_reset_color"),
                (CoverGroup::Mono, "looks.covers_reset_mono"),
            ]
            .into_iter()
            .filter(|(group, _)| snapshot.has_individual_covers(*group))
            .collect();
            let reset_count = resets.len();
            view()
                .flex_1()
                .bg(move || token("grouped"))
                .child(
                    crate::ui::page_scroll().flex_1().child(
                        view()
                            .px_6()
                            .pt_4()
                            .pb(px(crate::pages::look_pages::TAB_BAR_CLEARANCE))
                            .gap_5()
                            .child(
                                view()
                                    .gap_2()
                                    .child(grouped_card(options))
                                    .child(footnote(tr("looks.covers_note", &[]))),
                            )
                            .child((reset_count > 0).then(|| {
                                view()
                                    .gap_2()
                                    .child(crate::ui::section_title(tr("looks.covers_custom", &[])))
                                    .child(grouped_card(view().children(
                                        resets.into_iter().enumerate().map(|(i, (group, key))| {
                                            let working = app.look_working(
                                                crate::store::COVERS_KEY,
                                                LookAction::Reset(group),
                                            );
                                            view()
                                                .opacity(if enabled || working { 1.0 } else { 0.4 })
                                                .child(action_row_loading(
                                                    tr(key, &[]),
                                                    false,
                                                    i + 1 == reset_count,
                                                    working,
                                                    move || {
                                                        if enabled {
                                                            app.reset_covers(group)
                                                        }
                                                    },
                                                ))
                                                .into_element()
                                        }),
                                    )))
                                    .child(footnote(tr("looks.covers_custom_note", &[])))
                            })),
                    ),
                )
                .into_element()
        }))
}

/// One scheme: its name, what it does, and a check when it is the card's.
fn scheme_option(
    app: AppState,
    scheme: Scheme,
    selected: bool,
    enabled: bool,
    working: bool,
    last: bool,
) -> impl IntoElement {
    let title = tr(&format!("looks.covers_scheme.{}", scheme.id()), &[]);
    let detail = tr(&format!("looks.covers_desc.{}", scheme.id()), &[]);
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .min_h(px(64.0))
        .px_4()
        .py_2()
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .role(AccessibilityRole::Radio)
        .label(format!("{title}, {detail}"))
        .accessibility_state(move || AccessibilityState {
            selected,
            disabled: !enabled,
            busy: working,
            ..AccessibilityState::default()
        })
        .opacity(if enabled || working { 1.0 } else { 0.4 })
        .when(enabled && !selected, |v| {
            v.on_press(move || app.set_cover_scheme(scheme))
                .active(|s| s.opacity(0.72))
        })
        .child(
            view()
                .flex_1()
                .gap(px(2.0))
                .child(text(title).text_size(px(crate::ui::ty::BODY)))
                .child(
                    text(detail)
                        .text_size(px(crate::ui::ty::FOOTNOTE))
                        .line_height(px(18.0))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(working.then(|| quiet_spinner(18.0)))
        .child((selected && !working).then(|| {
            view()
                .text_color(move || theme().foreground)
                .child(svg(ImageSrc::svg_markup(SCHEME_CHECK)).size(px(18.0)))
        }))
}

const SCHEME_CHECK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12.5 4.5 4.5L19 7.5"/></svg>"##;

/// New and changed Looks wait: the camera reads its Look files when it
/// starts. Restart Camera turns it off and on, then connects again.
fn restart_banner(app: AppState, count: usize, enabled: bool) -> impl IntoElement {
    let title = match count {
        1 => tr("looks.restart_one", &[]),
        n => tr("looks.restart_many", &[("count", &n.to_string())]),
    };
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .rounded(px(10.0))
        .py_3()
        .pl_3()
        .pr_2()
        .bg(move || theme().muted)
        .child(
            view()
                .flex_1()
                .gap(px(2.0))
                .role(AccessibilityRole::Text)
                .child(
                    text(title)
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().foreground),
                )
                .child(
                    text(tr("looks.restart_body", &[]))
                        .text_size(px(crate::ui::ty::FOOTNOTE))
                        .line_height(px(18.0))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(restart_button(app, enabled))
}

/// Restart Camera, in the fill of a button, after a confirm.
fn restart_button(app: AppState, enabled: bool) -> impl IntoElement {
    let title = tr("looks.restart_now", &[]);
    let restarting = app.camera_restarting();
    let pressable = enabled && !restarting;
    view()
        .min_h(px(crate::ui::touch_target()))
        .justify_center()
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .accessibility_state(crate::ui::busy_state(restarting))
        .opacity(if enabled || restarting { 1.0 } else { 0.4 })
        .when(pressable, |v| {
            v.on_press(move || confirm_restart(app))
                .active(|s| s.opacity(0.72))
        })
        .child(
            view()
                .min_h(px(32.0))
                .px(px(14.0))
                .py_1()
                .rounded_full()
                .items_center()
                .justify_center()
                .bg(move || token("button"))
                .child(loading_content(
                    restarting,
                    16.0,
                    token("button-foreground"),
                    text(title)
                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || token("button-foreground")),
                )),
        )
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
    view()
        .rounded(px(10.0))
        .p_3()
        .gap(px(2.0))
        .bg(move || theme().muted)
        .role(AccessibilityRole::Text)
        .child(
            text(title)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        )
        .child(
            text(tr(body, &[]))
                .text_size(px(crate::ui::ty::FOOTNOTE))
                .line_height(px(18.0))
                .text_color(move || theme().muted_foreground),
        )
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

pub(crate) fn no_camera(app: AppState) -> impl IntoElement {
    grouped_card(
        view()
            .p_6()
            .gap_4()
            .items_center()
            .child(
                view()
                    .opacity(0.35)
                    .child(camera_art(CameraModel::Gr3, 180.0, false)),
            )
            .child(
                text(tr("looks.no_camera_title", &[]))
                    .text_size(px(crate::ui::ty::HEADLINE))
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                text(tr("looks.no_camera_body", &[]))
                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                    .line_height(px(22.0))
                    .text_center()
                    .text_color(move || theme().muted_foreground),
            )
            .child(
                view()
                    .w_full()
                    .child(primary_button(tr("looks.connect", &[]), move || {
                        app.set_tab(Tab::Camera);
                        app.connect_saved();
                    })),
            ),
    )
}

/// The sheet behind the Import tile: what kind of file to import.
pub fn import_sheet(app: AppState) -> impl IntoElement {
    let option = move |title: String, ext: String| {
        crate::ui::chevron_row(title, Some(ext), move || app.begin_import())
    };
    crate::ui::sheet_shell(
        tr("looks.import_title", &[]),
        Some(tr("looks.import_body", &[])),
        None,
        true,
        move || app.set_sheet(None),
        view().gap_3().child(grouped_card(
            view()
                .child(
                    view()
                        .border_b_1()
                        .border_b_color(move || theme().border)
                        .child(option(
                            tr("looks.import_lut", &[]),
                            tr("looks.import_lut_ext", &[]),
                        )),
                )
                .child(option(
                    tr("looks.import_preset", &[]),
                    tr("looks.import_preset_ext", &[]),
                )),
        )),
    )
}
