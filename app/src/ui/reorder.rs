//! Lists whose rows a handle moves (the Edit mode of Film Looks, Recipes
//! and Modules), the handle itself, and the check that shows or hides an
//! item in the camera menu.

use nsh_core::AccessibilityActionDef;
use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use crate::strings::tr;
use crate::ui::{busy_state, quiet_spinner, token};

/// A row's height until it measures itself: rows grow with their text.
const ROW_ESTIMATE: f32 = 64.0;

/// Rows on a card. Its background shows where a dragged row will land.
pub fn list(rows: Vec<AnyElement>) -> impl IntoElement {
    let mut card = view()
        .rounded(px(crate::ui::list::SECTION_RADIUS))
        .overflow_hidden()
        .bg(move || token("list-separator"));
    for row in rows {
        card = card.child(row);
    }
    card
}

/// Rows that a handle moves in Edit: the dragged row follows the finger,
/// the rows it passes move aside, so the gap shows where it lands, and
/// `on_move` gets the place where it is let go. `row` makes each row, with
/// the handle's drag when the list is movable.
pub fn reorder_list(
    count: usize,
    movable: bool,
    on_move: impl Fn(usize, usize) + Copy + 'static,
    row: impl Fn(usize, Option<RowGestures>) -> AnyElement,
) -> impl IntoElement {
    // The row being dragged, and how far.
    let dragging = signal(None::<usize>);
    let offset = signal(0.0f32);
    // Rows grow with the text size, each by its own text.
    let heights = signal(vec![ROW_ESTIMATE; count]);
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
                        .bg(move || token("list-card"))
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

/// What a held row does: lift it for a drag, or open its menu, and what
/// letting go does.
pub struct RowGestures {
    pub long_press: Box<dyn Fn()>,
    pub release: Option<Box<dyn Fn()>>,
    /// The drag of a row's handle in Edit, which moves the row.
    pub handle: Option<HandleDrag>,
}

/// What a handle's touch does to its row: it is lifted, follows the
/// finger by `moved`, and lands where it is let go.
pub struct HandleDrag {
    pub grant: Box<dyn Fn()>,
    pub moved: Box<dyn Fn(f32)>,
    pub release: Box<dyn Fn()>,
    pub terminate: Box<dyn Fn()>,
}

/// Three lines: hold and drag to move the row.
const HANDLE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="14" viewBox="0 0 20 14" fill="none" stroke="#000000" stroke-width="2" stroke-linecap="round"><path d="M2 2h16M2 7h16M2 12h16"/></svg>"##;

/// The handle at the end of a row in Edit. It takes the touch as the
/// finger goes down, before a scroll view can, so a drag that starts on
/// it moves the row at once.
pub fn drag_handle(name: String, gestures: Option<RowGestures>) -> impl IntoElement {
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
                .tint_color(move || token("list-secondary")),
        )
}

/// Whether an item shows in the camera menu, in Edit: a filled check in the
/// tint, or an empty ring. A spinner takes its place while it changes.
pub fn check_circle(
    on: bool,
    label: String,
    enabled: bool,
    loading: bool,
    on_press: impl Fn() + 'static,
) -> AnyElement {
    let mark: AnyElement = if loading {
        quiet_spinner(18.0)
    } else if on {
        view()
            .size(px(24.0))
            .rounded_full()
            .items_center()
            .justify_center()
            .bg(move || token("tint-fill"))
            .child(
                svg(ImageSrc::svg_markup(
                    r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" stroke-width="3.4" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.5l4.5 4.5L19 7"/></svg>"##,
                ))
                .size(px(13.0)),
            )
            .into_element()
    } else {
        view()
            .size(px(24.0))
            .rounded_full()
            .border(px(1.5))
            .border_color(move || token("list-chevron"))
            .into_element()
    };
    let pressable = enabled && !loading;
    view()
        .size(px(crate::ui::touch_target()))
        .flex_shrink(0.0)
        .items_center()
        .justify_center()
        .role(AccessibilityRole::Checkbox)
        .label(label)
        .accessibility_state(move || AccessibilityState {
            checked: Some(if on {
                nsh_core::Checked::True
            } else {
                nsh_core::Checked::False
            }),
            ..busy_state(loading)()
        })
        .disabled(!pressable)
        .opacity(if enabled || loading { 1.0 } else { 0.4 })
        .when(pressable, |v| v.on_press(on_press))
        .child(mark)
        .into_element()
}
