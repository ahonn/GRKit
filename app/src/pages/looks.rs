//! Looks tab: the camera's Looks in camera menu order, and the covers of the
//! built-in Looks. The cover grid parts serve the Add page.

use nsh_base::{nav_page, theme};
use nsh_core::AccessibilityActionDef;
use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use grkit_core::cover::Scheme;

use crate::domain::{CameraModel, CoverGroup, Look, LookSource, LooksSnapshot, group_of};
use crate::store::{AppState, Route, Tab};
use crate::strings::tr;
use crate::ui::{
    action_row, camera_art, circle_button, control_row, cover_view, footnote, grouped_card,
    page_title, primary_button, secondary_button, segmented, token,
};

const PLUS: &str = "M12 5v14M5 12h14";
/// The side of a cover in the grid of the Add page.
pub const TILE_COVER: f32 = 92.0;
/// Columns of a cover grid.
pub const COLUMNS: usize = 3;
pub const COLUMN_GAP: f32 = 12.0;
pub const ROW_GAP: f32 = 18.0;
const CHECK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" stroke-width="3.4" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12.5 4.5 4.5L19 7.5"/></svg>"##;

pub fn looks_root(app: AppState) -> impl IntoElement {
    nav_page()
        .header_visible(false)
        .child(dynamic(move || looks_content(app)))
}

fn looks_content(app: AppState) -> impl IntoElement {
    let connected = app.camera_online();
    let can_add = connected && app.looks_blocked().is_none() && app.looks_snapshot().is_some();
    let plus = circle_button(PLUS, tr("looks.add_title", &[]), can_add, move || {
        app.open_add()
    })
    .into_element();
    crate::ui::page_scroll()
        .flex_1()
        .bg(move || token("grouped"))
        .child(
            view()
                .px_6()
                .pt(px(72.0))
                .pb(px(crate::pages::look_pages::TAB_BAR_CLEARANCE))
                .gap(px(ROW_GAP))
                .child(page_title(
                    tr("looks.title", &[]),
                    connected.then_some(plus),
                ))
                .child(on_camera(app)),
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
            format!("{} / {}", shown.len(), snapshot.max_listed),
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
                    look_row(app, look, !blocked, index == last, None).into_element()
                })
                .collect();
            section(
                tr("looks.off_section", &[]),
                count,
                list(rows),
                tr("looks.off_note", &[]),
            )
        }))
        .child(
            snapshot
                .has_built_in()
                .then(|| covers(app, &snapshot, !blocked)),
        )
        .into_element()
}

/// A titled group of covers with its count and a note.
fn section(
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
const ROW_HEIGHT: f32 = 64.0;
/// The side of the cover in a row.
const ROW_COVER: f32 = 44.0;

/// Rows on a card. Its background shows where a dragged row will land.
fn list(rows: Vec<AnyElement>) -> impl IntoElement {
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
    // The row that a long press lifted, which stays lifted until it is
    // dragged; the row being dragged, and how far.
    let armed = signal(None::<usize>);
    let dragging = signal(None::<usize>);
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
    let rows = looks
        .into_iter()
        .enumerate()
        .map(|(index, look)| {
            let held = move || armed.get() == Some(index);
            let lifted = move || dragging.get() == Some(index);
            // A row between the dragged row and its target moves one row
            // toward the place the dragged row left.
            let shift = move || match (dragging.get(), target()) {
                (Some(from), Some(to)) if from < index && index <= to => -ROW_HEIGHT,
                (Some(from), Some(to)) if to <= index && index < from => ROW_HEIGHT,
                _ => 0.0,
            };
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
                        "move_earlier" => app.move_shown_look(index, index.saturating_sub(1)),
                        "move_later" => app.move_shown_look(index, (index + 1).min(count - 1)),
                        _ => {}
                    })
                    .claim_touches(TouchClaim::on_move().ask(move |request: &ClaimRequest| {
                        if armed.get() == Some(index) {
                            request.claim();
                        }
                    }))
                    .keep_touches(true)
                    .on_responder_grant(move |_: &ResponderEvent| {
                        dragging.set(Some(index));
                        offset.set(0.0);
                    })
                    .on_responder_move(move |event: &ResponderEvent| {
                        offset.set(event.gesture.delta.y.0);
                    })
                    .on_responder_release(move |_: &ResponderEvent| {
                        let to = target().unwrap_or(index);
                        end();
                        app.move_shown_look(index, to);
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
                .child(look_row(
                    app,
                    look,
                    editable,
                    index + 1 == count,
                    movable.then(|| Box::new(move || armed.set(Some(index))) as Box<dyn Fn()>),
                ))
                .into_element()
        })
        .collect();
    list(rows)
}

/// What a row says under the name, and whether it is in the tint.
fn look_status(look: &Look) -> Option<(String, bool)> {
    if look.selected && look.enabled {
        Some((tr("looks.in_use", &[]), true))
    } else if look.pending.is_some() {
        Some((tr("looks.after_restart", &[]), false))
    } else if look.enabled && !look.loaded {
        Some((tr("looks.not_loaded", &[]), false))
    } else {
        None
    }
}

/// A Look on the camera: its cover, name and state, which opens the Look,
/// and the check that shows or hides it in the camera menu.
fn look_row(
    app: AppState,
    look: Look,
    editable: bool,
    last: bool,
    on_long_press: Option<Box<dyn Fn()>>,
) -> impl IntoElement {
    let id = look.id.clone();
    let off = !look.enabled;
    let (status, emphasized) = look_status(&look).unwrap_or_default();
    let label = match status.is_empty() {
        true => look.name.clone(),
        false => format!("{}, {status}", look.name),
    };
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
                .when_some(on_long_press, |v, on_long_press| {
                    v.on_long_press(on_long_press)
                })
                .active(|s| s.opacity(0.72))
                .child(cover_view(&look.shown_cover(), ROW_COVER, off))
                .child(
                    view()
                        .flex_1()
                        .gap(px(2.0))
                        .child(
                            text(look.name.clone())
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
        .opacity(if enabled { 1.0 } else { 0.4 })
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

/// A white stroke icon for a corner button.
pub fn corner_icon(svg: &str) -> AnyElement {
    image(ImageSrc::svg_markup(svg))
        .size(px(13.0))
        .into_element()
}

/// Shown in the camera menu: a filled check; hidden: an empty circle.
fn round_check(
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
        on.then(|| token("tint")),
        icon,
        label,
        AccessibilityRole::Checkbox,
        Some(on),
        enabled,
        on_press,
    )
}

/// The covers of the built-in Looks: the scheme of the Looks without an
/// individual cover, and a reset of the individual covers of each group.
fn covers(app: AppState, snapshot: &LooksSnapshot, writable: bool) -> impl IntoElement {
    let enabled = writable && app.look_busy().is_none();
    let current = snapshot.official_covers;
    let schemes = [Scheme::Gray, Scheme::ColorFilmBox, Scheme::FilmBox]
        .into_iter()
        .map(|scheme| -> crate::ui::Segment {
            (
                tr(&format!("looks.covers_scheme.{}", scheme.id()), &[]),
                scheme == current,
                Box::new(move || {
                    if enabled {
                        app.set_cover_scheme(scheme)
                    }
                }),
            )
        })
        .collect();
    let resets: Vec<(CoverGroup, &str)> = [
        (CoverGroup::Color, "looks.covers_reset_color"),
        (CoverGroup::Mono, "looks.covers_reset_mono"),
    ]
    .into_iter()
    .filter(|(group, _)| snapshot.has_individual_covers(*group))
    .collect();
    let count = resets.len();
    let mut rows = view().child(control_row(
        tr("looks.covers_film_box", &[]),
        view()
            .w(px(180.0))
            .opacity(if enabled { 1.0 } else { 0.4 })
            .child(segmented(schemes)),
        count == 0,
    ));
    for (index, (group, key)) in resets.into_iter().enumerate() {
        rows = rows.child(
            view()
                .opacity(if enabled { 1.0 } else { 0.4 })
                .child(action_row(
                    tr(key, &[]),
                    false,
                    index + 1 == count,
                    move || {
                        if enabled {
                            app.reset_covers(group)
                        }
                    },
                )),
        );
    }
    view()
        .gap_2()
        .child(crate::ui::section_title(tr("looks.covers", &[])))
        .child(grouped_card(rows))
        .child(footnote(tr("looks.covers_note", &[])))
}

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
        .bg(move || token("warning-soft"))
        .role(AccessibilityRole::Text)
        .child(
            text(title)
                .text_size(px(15.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || token("warning")),
        )
        .child(
            text(tr(body, &[]))
                .text_size(px(13.0))
                .line_height(px(18.0))
                .text_color(move || token("warning")),
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

fn no_camera(app: AppState) -> impl IntoElement {
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
