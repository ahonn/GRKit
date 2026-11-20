//! Looks tab: the camera's Looks in camera menu order, and the covers of the
//! built-in Looks. The cover grid parts serve the Add page.

use nsh_base::{alert_dialog, nav_page, theme};
use nsh_core::AccessibilityActionDef;
use nsh_core::element::ImageSrc;
use nsh_core::element::header::HeaderItem;
use nsh_core::prelude::*;

use grkit_core::cover::Scheme;

use crate::domain::{CameraModel, CoverGroup, Look, LookSource, group_of};
use crate::store::{AppState, Route, Sheet, Tab};
use crate::strings::tr;
use crate::ui::{
    action_row, badge, camera_art, circle_button, control_row, cover_view, footnote, grouped_card,
    primary_button, secondary_button, token,
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
    nav_page()
        .title(move || tr("looks.title", &[]))
        .header_large_title(!cfg!(target_os = "android"))
        .header_right_items(move || native_add(app))
        .header_right(dynamic(move || {
            can_add(app).then(|| {
                circle_button(PLUS, tr("looks.add_title", &[]), true, move || {
                    app.open_add()
                })
            })
        }))
        .child(looks_content(app))
}

/// Add opens with a camera whose Looks the phone read.
fn can_add(app: AppState) -> bool {
    app.camera_online() && app.looks_blocked().is_none() && app.looks_snapshot().is_some()
}

/// Add in the system bar (iOS).
fn native_add(app: AppState) -> Vec<HeaderItem> {
    if !can_add(app) {
        return Vec::new();
    }
    vec![
        HeaderItem::symbol("plus")
            .key("add")
            .label(tr("looks.add_title", &[]))
            .on_press(move || app.open_add()),
    ]
}

/// The scroll view stays while the Looks change: only its content follows
/// them, so a change does not move the list back to the top.
fn looks_content(app: AppState) -> impl IntoElement {
    crate::ui::page_scroll()
        .flex_1()
        .bg(move || token("grouped"))
        .child(
            view()
                .px_6()
                .pt_4()
                .pb(px(crate::pages::look_pages::TAB_BAR_CLEARANCE))
                .gap(px(ROW_GAP))
                .child(dynamic(move || on_camera(app))),
        )
}

fn on_camera(app: AppState) -> AnyElement {
    if !app.camera_online() {
        return no_camera(app).into_element();
    }
    let Some(snapshot) = app.looks_snapshot() else {
        if app.looks_error().is_some() {
            return grouped_card(
                view()
                    .p_4()
                    .gap_3()
                    .child(text(tr("looks.read_failed", &[])).text_size(px(17.0)))
                    .child(secondary_button(tr("looks.retry", &[]), move || {
                        app.refresh_looks()
                    })),
            )
            .into_element();
        }
        return grouped_card(
            view().p_4().child(
                text(tr("looks.loading", &[]))
                    .text_size(px(15.0))
                    .text_color(move || theme().muted_foreground),
            ),
        )
        .into_element();
    };
    let blocked = app.looks_blocked().is_some();
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
    view()
        .gap(px(ROW_GAP))
        .child(blocked.then(|| banner(tr("looks.pending_title", &[]), "looks.pending_body")))
        .child((pending > 0).then(|| restart_banner(pending)))
        .child(section(
            tr("looks.menu", &[]),
            format!("{} / {}", shown.len(), snapshot.max_loaded),
            menu_list(app, shown, !blocked),
            tr("looks.menu_note", &[]),
        ))
        .child((!hidden.is_empty()).then(|| {
            let count = hidden.len().to_string();
            let last = hidden.len() - 1;
            let rows = hidden
                .into_iter()
                .enumerate()
                .map(|(index, look)| {
                    let gestures = RowGestures::menu(app, &look, !blocked);
                    look_row(app, look, !blocked, index == last, Some(gestures)).into_element()
                })
                .collect();
            section(
                tr("looks.off_section", &[]),
                count,
                list(rows),
                tr("looks.off_note", &[]),
            )
        }))
        .child(snapshot.has_built_in().then(|| covers_row(app)))
        .into_element()
}

/// A titled group of covers with its count and a note.
pub(crate) fn section(
    title: String,
    count: String,
    content: impl IntoElement,
    note: String,
) -> impl IntoElement {
    view()
        .gap_3()
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .px(px(2.0))
                .child(crate::ui::section_title(title))
                .child(
                    text(count)
                        .text_size(px(15.0))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(content)
        .child(footnote(note))
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

/// The height of a row of the Looks lists.
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

/// The shown Looks. A row held down lifts and follows the finger; the rows
/// it passes move aside, so the gap shows where it lands. The camera menu
/// takes the order where it is let go.
fn menu_list(app: AppState, looks: Vec<Look>, editable: bool) -> impl IntoElement {
    let count = looks.len();
    let movable = editable && count > 1;
    let menus: Vec<Look> = looks.clone();
    let rows = looks.into_iter().map(Some).collect::<Vec<_>>();
    let rows = std::cell::RefCell::new(rows);
    reorder_list(
        count,
        movable,
        move |from, to| app.move_shown_look(from, to),
        move |index, closed| look_menu_then(app, menus[index].clone(), closed),
        move |index, gestures| {
            let look = rows.borrow_mut()[index].take().expect("each row once");
            let gestures = gestures.unwrap_or_else(|| RowGestures::menu(app, &look, editable));
            look_row(app, look, editable, index + 1 == count, Some(gestures)).into_element()
        },
    )
}

/// Rows that a long press lifts and a drag moves: the rows it passes move
/// aside, so the gap shows where it lands, and `on_move` gets the place
/// where it is let go. Letting go without a drag calls `menu` with what to
/// do once the menu closes. `row` makes each row, with the gestures of a
/// movable list.
pub(crate) fn reorder_list(
    count: usize,
    movable: bool,
    on_move: impl Fn(usize, usize) + Copy + 'static,
    menu: impl Fn(usize, Box<dyn FnOnce()>) + Clone + 'static,
    row: impl Fn(usize, Option<RowGestures>) -> AnyElement,
) -> impl IntoElement {
    // The row that a long press lifted, which stays lifted until it is
    // dragged; the row being dragged, and how far.
    let armed = signal(None::<usize>);
    let dragging = signal(None::<usize>);
    // Whether the held row moved since it was lifted: then letting go drops
    // it instead of opening its menu.
    let dragged = signal(false);
    let offset = signal(0.0f32);
    let target = move || {
        dragging.get().map(|from| {
            let steps = (offset.get() / ROW_HEIGHT).round() as isize;
            (from as isize + steps).clamp(0, count as isize - 1) as usize
        })
    };
    let end = move || {
        armed.set(None);
        dragging.set(None);
        offset.set(0.0);
    };
    let rows = (0..count)
        .map(|index| {
            let held = move || armed.get() == Some(index);
            let lifted = move || dragging.get() == Some(index);
            // A row between the dragged row and its target moves one row
            // toward the place the dragged row left.
            let shift = move || match (dragging.get(), target()) {
                (Some(from), Some(to)) if from < index && index <= to => -ROW_HEIGHT,
                (Some(from), Some(to)) if to <= index && index < from => ROW_HEIGHT,
                _ => 0.0,
            };
            let gestures = movable.then(|| {
                let menu = menu.clone();
                RowGestures {
                    long_press: Box::new(move || {
                        dragged.set(false);
                        armed.set(Some(index))
                    }),
                    // Let go without a drag: the menu of the row. The row
                    // stays lifted, and can still be dragged, until the menu
                    // closes.
                    release: Some(Box::new(move || {
                        if armed.get() == Some(index) && !dragged.get() {
                            menu(
                                index,
                                Box::new(move || {
                                    if dragging.get().is_none() {
                                        armed.set(None)
                                    }
                                }),
                            );
                        }
                    })),
                }
            });
            view()
                .relative()
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
                    .claim_touches(TouchClaim::on_move().ask(move |request: &ClaimRequest| {
                        if armed.get() == Some(index) {
                            request.claim();
                        }
                    }))
                    .keep_touches(true)
                    .on_responder_grant(move |_: &ResponderEvent| {
                        dragged.set(true);
                        dragging.set(Some(index));
                        offset.set(0.0);
                    })
                    .on_responder_move(move |event: &ResponderEvent| {
                        offset.set(event.gesture.delta.y.0);
                    })
                    .on_responder_release(move |_: &ResponderEvent| {
                        let to = target().unwrap_or(index);
                        end();
                        on_move(index, to);
                    })
                    .on_responder_terminate(move |_: &ResponderEvent| end())
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
                        .opacity(move || if lifted() || held() { 1.0_f32 } else { 0.0_f32 }),
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
}

impl RowGestures {
    /// A row that does not move: holding it opens its menu.
    fn menu(app: AppState, look: &Look, editable: bool) -> Self {
        let look = look.clone();
        RowGestures {
            long_press: Box::new(move || {
                if editable {
                    look_menu(app, look.clone())
                }
            }),
            release: None,
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
        let mut sheet = alert_dialog().title(look.name.clone()).action_sheet();
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
            Some("rename") => app.set_sheet(Some(Sheet::RenameLook(look.id.clone()))),
            Some("cover") => app.push(Route::LookCover(look.id.clone())),
            Some("delete") => crate::pages::look_pages::confirm_delete(app, look.clone()),
            _ => {}
        }
    });
}

fn look_row(
    app: AppState,
    look: Look,
    editable: bool,
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
    let toggleable = editable && look.listed() && app.look_busy().is_none();
    let toggle_label = tr(
        if look.enabled {
            "looks.hide_named"
        } else {
            "looks.show_named"
        },
        &[("name", &look.name)],
    );
    let (toggle_id, enabled) = (look.id.clone(), look.enabled);
    view()
        .flex_row()
        .items_center()
        .h(px(ROW_HEIGHT))
        .pr(px(4.0))
        .bg(move || theme().card)
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .child(
            view()
                .flex_1()
                .h_full()
                .flex_row()
                .items_center()
                .gap_3()
                .pl(px(12.0))
                .role(AccessibilityRole::Button)
                .label(label)
                .on_press(move || app.push(Route::LookDetail(id.clone())))
                .when_some(gestures, |v, gestures| {
                    let v = v.on_long_press(gestures.long_press);
                    match gestures.release {
                        Some(release) => v.on_press_up(move |_: &PressEvent| release()),
                        None => v,
                    }
                })
                .active(|s| s.opacity(0.72))
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
                                        .number_of_lines(1)
                                        .text_size(px(17.0))
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
                            text(status).text_size(px(15.0)).text_color(move || {
                                if emphasized {
                                    token("tint")
                                } else {
                                    theme().muted_foreground
                                }
                            })
                        })),
                ),
        )
        .child(round_check(
            look.enabled,
            toggle_label,
            toggleable,
            move || app.set_look_enabled(toggle_id.clone(), !enabled),
        ))
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
                        .text_size(px(13.0))
                        .line_height(px(17.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_center(),
                )
                .child(
                    text(status)
                        .mt(px(-3.0))
                        .min_h(px(15.0))
                        .text_size(px(12.0))
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

/// A round control, 26 points drawn and 44 to press, at the top right corner
/// of a tile's cover when `corner`. `filled` is its color, or `None` for an
/// empty circle.
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
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .when(corner, |v| {
            // On the top right corner of the cover, which is centered.
            v.absolute()
                .top(px(-12.0))
                .left(relative(0.5))
                .ml(px(TILE_COVER / 2.0 - 34.0))
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
            ..AccessibilityState::default()
        })
        .opacity(if enabled && !dimmed { 1.0 } else { 0.4 })
        .when(enabled, |v| v.on_press(on_press))
        .child(
            view()
                .size(px(26.0))
                .rounded_full()
                .items_center()
                .justify_center()
                .map(|v| match filled {
                    Some(color) => v.border_2().border_color(rgb(0xFFFFFF)).bg(color),
                    None => v
                        .border(px(1.5))
                        .border_color(move || theme().border)
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
                .child(icon),
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
            .child(control_row(
                title,
                text("›")
                    .text_size(px(22.0))
                    .text_color(move || theme().muted_foreground),
                true,
            )),
    )
}

/// Cover Colors (push): the scheme of the built-in Looks without a cover of
/// their own, and a reset of the covers people changed, by group.
pub fn scheme_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    nav_page()
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
                scheme_option(app, scheme, scheme == current, enabled, i + 1 == count)
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
                                            view()
                                                .opacity(if enabled { 1.0 } else { 0.4 })
                                                .child(action_row(
                                                    tr(key, &[]),
                                                    false,
                                                    i + 1 == reset_count,
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
            ..AccessibilityState::default()
        })
        .opacity(if enabled { 1.0 } else { 0.4 })
        .when(enabled && !selected, |v| {
            v.on_press(move || app.set_cover_scheme(scheme))
                .active(|s| s.opacity(0.72))
        })
        .child(
            view()
                .flex_1()
                .gap(px(2.0))
                .child(text(title).text_size(px(17.0)))
                .child(
                    text(detail)
                        .text_size(px(13.0))
                        .line_height(px(18.0))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(selected.then(|| {
            view()
                .text_color(move || theme().foreground)
                .child(svg(ImageSrc::svg_markup(SCHEME_CHECK)).size(px(18.0)))
        }))
}

const SCHEME_CHECK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12.5 4.5 4.5L19 7.5"/></svg>"##;

/// New and changed Looks wait: the camera reads its Look files when it
/// starts.
fn restart_banner(count: usize) -> impl IntoElement {
    let title = match count {
        1 => tr("looks.restart_one", &[]),
        n => tr("looks.restart_many", &[("count", &n.to_string())]),
    };
    banner(title, "looks.restart_body")
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
                .text_size(px(15.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        )
        .child(
            text(tr(body, &[]))
                .text_size(px(13.0))
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
                    .text_size(px(17.0))
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                text(tr("looks.no_camera_body", &[]))
                    .text_size(px(15.0))
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
        view()
            .gap_3()
            .child(grouped_card(
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
            ))
            .child(footnote(tr("looks.import_note", &[]))),
    )
}
