//! A grid of Looks: each cell is a cover, or the sample photo with the Look
//! and its cover in a corner, with the Look's name under it and a short
//! caption or an Add button.

use grkit_core::cover::Cover;
use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use crate::ui::ty::text;
use crate::ui::{
    ButtonKind, ButtonSize, busy_state, button, cover_fill, cover_thumb, quiet_spinner, token,
};

/// The side of the cover in the corner of a photo.
const CORNER_COVER: f32 = 30.0;

/// The columns of a grid on a phone.
pub const GRID_COLUMNS: u16 = 3;
/// The space between cells across, and between rows.
pub const GRID_GAP: f32 = 12.0;
pub const GRID_ROW_GAP: f32 = 18.0;

/// The color of a cell's caption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptionTone {
    /// The Look in use: the tint, with a dot.
    Current,
    /// A change that waits for a restart.
    Pending,
    /// A plain fact, such as Negative or B&W.
    Plain,
}

/// What a cell offers under its name.
enum CellAction {
    None,
    /// A button: Add, or its spinner while `adding`. `title` replaces the
    /// grid's Add title, such as Undo for a Look just added.
    Add {
        title: Option<String>,
        label: String,
        adding: bool,
        on_add: Box<dyn Fn()>,
    },
    /// A check in the corner: the Look is `on` the list of what goes on.
    Check {
        on: bool,
        on_toggle: Box<dyn Fn()>,
    },
}

/// One Look of a [`cover_grid`].
pub struct CoverCell {
    cover: Cover,
    /// The sample photo with the Look: `Some(None)` while it is made.
    photo: Option<Option<ImageSrc>>,
    name: String,
    caption: Option<(String, CaptionTone)>,
    action: CellAction,
    open: Option<Box<dyn Fn()>>,
    faded: bool,
    enabled: bool,
    /// The name in the middle, under a cover alone on its row.
    centered: bool,
    test_id: Option<String>,
}

/// A cell of the Look `name` with its cover.
pub fn cover_cell(cover: Cover, name: impl Into<String>) -> CoverCell {
    CoverCell {
        cover,
        photo: None,
        name: name.into(),
        caption: None,
        action: CellAction::None,
        open: None,
        faded: false,
        enabled: true,
        centered: false,
        test_id: None,
    }
}

impl CoverCell {
    /// Show the sample photo with the Look, the cover in its lower left
    /// corner. `None` while the photo is made, or when the phone cannot make
    /// it: the whole cover in the photo's frame.
    pub fn photo(mut self, photo: Option<ImageSrc>) -> Self {
        self.photo = Some(photo);
        self
    }

    /// A short line under the name.
    pub fn caption(mut self, caption: impl Into<String>, tone: CaptionTone) -> Self {
        self.caption = Some((caption.into(), tone)).filter(|(c, _)| !c.is_empty());
        self
    }

    /// The cell adds the Look when pressed, said as `label` (Add Portra 400):
    /// a plus in the corner of the photo, its spinner while `adding`.
    pub fn add(
        mut self,
        label: impl Into<String>,
        adding: bool,
        on_add: impl Fn() + 'static,
    ) -> Self {
        self.action = CellAction::Add {
            title: None,
            label: label.into(),
            adding,
            on_add: Box::new(on_add),
        };
        self
    }

    /// An Undo button under the name, titled `title` and said as `label`
    /// (Take back Portra 400); its spinner while `working`.
    pub fn undo(
        mut self,
        title: impl Into<String>,
        label: impl Into<String>,
        working: bool,
        on_undo: impl Fn() + 'static,
    ) -> Self {
        self.action = CellAction::Add {
            title: Some(title.into()),
            label: label.into(),
            adding: working,
            on_add: Box::new(on_undo),
        };
        self
    }

    /// The cell puts the Look in or out of a choice when pressed: a check
    /// in the lower right corner of the cover, which fades while `on` is
    /// false.
    pub fn check(mut self, on: bool, on_toggle: impl Fn() + 'static) -> Self {
        self.action = CellAction::Check {
            on,
            on_toggle: Box::new(on_toggle),
        };
        self.faded = !on;
        self
    }

    /// Whether the cell and its button take a press.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Open the Look when the cover or name is pressed.
    pub fn open(mut self, open: impl Fn() + 'static) -> Self {
        self.open = Some(Box::new(open));
        self
    }

    /// The Look is not in the camera's menu.
    pub fn faded(mut self, faded: bool) -> Self {
        self.faded = faded;
        self
    }

    /// The name in the middle, under a cover alone on its row.
    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    pub fn test_id(mut self, id: impl Into<String>) -> Self {
        self.test_id = Some(id.into());
        self
    }

    fn label(&self) -> String {
        match &self.caption {
            Some((caption, _)) => format!("{}, {caption}", self.name),
            None => self.name.clone(),
        }
    }

    fn render(self) -> AnyElement {
        let label = self.label();
        let caption = self.caption.map(|(copy, tone)| {
            let color = move || match tone {
                CaptionTone::Current => token("tint"),
                CaptionTone::Pending => token("pending"),
                CaptionTone::Plain => token("list-secondary"),
            };
            view()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .child((tone == CaptionTone::Current).then(|| {
                    view()
                        .size(px(6.0))
                        .rounded_full()
                        .bg(move || token("tint"))
                }))
                .child(
                    text(copy)
                        .text_size(px(crate::ui::ty::CAPTION))
                        .font_weight(if tone == CaptionTone::Current {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(color),
                )
        });
        let enabled = self.enabled;
        let centered = self.centered;
        let checks = matches!(self.action, CellAction::Check { .. });
        let mut check = None;
        // An Add is the whole cell; an Undo stays a button under it.
        let (add, button_action) = match self.action {
            CellAction::Check { on, on_toggle } => {
                check = Some((on, on_toggle));
                (None, None)
            }
            CellAction::Add {
                title: None,
                label,
                adding,
                on_add,
            } => (Some((label, adding, on_add)), None),
            CellAction::Add {
                title: Some(title),
                label,
                adding,
                on_add,
            } => (
                None,
                Some(
                    button(title, ButtonKind::Tinted, ButtonSize::Small, on_add)
                        .label(label)
                        .enabled(enabled)
                        .loading(adding)
                        .into_element(),
                ),
            ),
            CellAction::None => (None, None),
        };
        let frame = match self.photo {
            Some(photo) => photo_frame(&self.cover, photo, self.faded),
            None => cover_fill(&self.cover, self.faded),
        };
        let frame = view()
            .child(frame)
            .child(add.as_ref().map(|(_, adding, _)| add_mark(*adding)))
            .child(check.as_ref().map(|(on, _)| check_mark(*on)));
        let mut body = view()
            .gap(px(8.0))
            .label(match &add {
                Some((add_label, ..)) => add_label.clone(),
                None => label,
            })
            .when_some(self.test_id.as_deref(), |v, id| v.test_id(id))
            .child(frame)
            .child(
                view()
                    .gap(px(1.0))
                    .px(px(2.0))
                    .child(
                        // Two lines: a long name or a large text size keeps
                        // the words that tell the Looks apart.
                        text(self.name)
                            .when(centered, |t| t.text_center())
                            .number_of_lines(2)
                            .text_size(px(crate::ui::ty::FOOTNOTE))
                            .font_weight(FontWeight::MEDIUM),
                    )
                    .child(caption),
            );
        // A Look being added says so, and takes no press until it ends.
        if let Some((_, adding, _)) = &add {
            body = body
                .accessibility_state(busy_state(*adding))
                .disabled(*adding || !enabled);
        }
        if let Some((on, _)) = &check {
            let on = *on;
            body = body
                .role(AccessibilityRole::Checkbox)
                .accessibility_state(move || AccessibilityState {
                    checked: Some(if on {
                        nsh_core::Checked::True
                    } else {
                        nsh_core::Checked::False
                    }),
                    ..AccessibilityState::default()
                });
        }
        let press: Option<Box<dyn Fn()>> = match (add, check) {
            (Some((_, adding, on_add)), _) => (!adding).then_some(on_add),
            (None, Some((_, on_toggle))) => Some(on_toggle),
            (None, None) => self.open,
        };
        if let Some(press) = press.filter(|_| enabled) {
            if !checks {
                body = body.role(AccessibilityRole::Button);
            }
            body = body.on_press(press).active(|s| s.opacity(0.72));
        }
        view()
            .gap(px(8.0))
            .child(body)
            .child(button_action)
            .into_element()
    }
}

/// The mark of a cell that adds its Look: a plus in the upper right corner of
/// the photo, its spinner while the Look is added.
fn add_mark(adding: bool) -> AnyElement {
    view()
        .absolute()
        .top(px(6.0))
        .right(px(6.0))
        .size(px(24.0))
        .rounded_full()
        .items_center()
        .justify_center()
        .bg(rgba(0xFFFFFFE6))
        .shadow([BoxShadow {
            offset_x: Points(0.0),
            offset_y: Points(1.0),
            blur: Points(4.0),
            spread: Points(0.0),
            color: rgba(0x00000040).into(),
            inset: false,
        }])
        .child(if adding {
            quiet_spinner(14.0)
        } else {
            svg(ImageSrc::svg_markup(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round"><path d="M12 6v12M6 12h12"/></svg>"#,
            ))
            .size(px(14.0))
            .text_color(move || token("tint"))
            .into_element()
        })
        .into_element()
}

/// The check of a cell in a choice, in the lower right corner of the cover:
/// the tint with a white check while on, else a white ring.
fn check_mark(on: bool) -> AnyElement {
    let mark = view()
        .absolute()
        .bottom(px(4.0))
        .right(px(4.0))
        .size(px(22.0))
        .rounded_full()
        .items_center()
        .justify_center();
    if on {
        mark.bg(move || token("tint-fill"))
            .child(
                svg(ImageSrc::svg_markup(
                    r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" stroke-width="3.4" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.5l4.5 4.5L19 7"/></svg>"##,
                ))
                .size(px(12.0)),
            )
            .into_element()
    } else {
        mark.border(px(1.5))
            .border_color(rgb(0xFFFFFF))
            .bg(rgba(0x00000033))
            .into_element()
    }
}

/// The sample photo at 3:2, with `cover` small in its lower left corner.
fn photo_frame(cover: &Cover, photo: Option<ImageSrc>, faded: bool) -> AnyElement {
    let Some(photo) = photo else {
        // Without the photo the cover alone tells the Looks apart.
        return view()
            .w_full()
            .aspect_ratio(3.0 / 2.0)
            .rounded(px(8.0))
            .overflow_hidden()
            .bg(move || token("skeleton-area"))
            .opacity(if faded { 0.45 } else { 1.0 })
            .items_center()
            .justify_center()
            .child(cover_thumb(cover, 64.0, false))
            .into_element();
    };
    view()
        .w_full()
        .aspect_ratio(3.0 / 2.0)
        .rounded(px(8.0))
        .overflow_hidden()
        .bg(rgb(0x1C1C1E))
        .opacity(if faded { 0.45 } else { 1.0 })
        .child(
            image(photo)
                .absolute()
                .inset_0()
                .object_fit(ResizeMode::Cover),
        )
        .child(
            view()
                .absolute()
                .left(px(6.0))
                .bottom(px(6.0))
                .rounded(px(2.0))
                .shadow([BoxShadow {
                    offset_x: Points(0.0),
                    offset_y: Points(1.0),
                    blur: Points(4.0),
                    spread: Points(0.0),
                    color: rgba(0x00000059).into(),
                    inset: false,
                }])
                .child(cover_thumb(cover, CORNER_COVER, false)),
        )
        .into_element()
}

/// `cells` in rows of [`GRID_COLUMNS`].
pub fn cover_grid(cells: impl IntoIterator<Item = CoverCell>) -> AnyElement {
    cover_grid_of(GRID_COLUMNS, cells)
}

/// `cells` in rows of `columns`: four for covers alone, which read at a
/// smaller size than the sample photos.
pub fn cover_grid_of(columns: u16, cells: impl IntoIterator<Item = CoverCell>) -> AnyElement {
    let mut grid = view().grid().grid_cols(columns).gap(px(GRID_GAP));
    for cell in cells {
        grid = grid.child(view().pb(px(GRID_ROW_GAP - GRID_GAP)).child(cell.render()));
    }
    grid.into_element()
}
