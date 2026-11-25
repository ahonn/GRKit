//! Inset grouped lists: a section is a card of rows with an optional title
//! above it and a note below it. A row has an optional leading tile, a
//! title and subtitle, and what it shows and does at its end.

use nsh_core::prelude::*;

use crate::ui::ty::text;
use crate::ui::{Choices, busy_state, chevron, held_view, menu_row, quiet_spinner, token};

/// The corner radius of a section's card.
pub const SECTION_RADIUS: f32 = 22.0;

/// The least heights of a row: one line, one line beside a leading tile,
/// and a title with a subtitle.
pub const ROW_PLAIN: f32 = 48.0;
pub const ROW_LEADING: f32 = 52.0;
pub const ROW_TWO_LINES: f32 = 60.0;

/// What a row does when pressed, and the mark at its end that says so.
enum Action {
    None,
    /// Runs the action; no mark.
    Press(Box<dyn Fn()>),
    /// Opens a page: a chevron.
    Page(Box<dyn Fn()>),
    /// Opens a menu of values: up and down chevrons.
    Menu(Choices),
    /// Opens a page outside the app: an arrow out.
    External(Box<dyn Fn()>),
}

/// One row of a [`ListSection`].
pub struct ListRow {
    title: String,
    subtitle: Option<String>,
    value: Option<String>,
    leading: Option<AnyElement>,
    trailing: Option<AnyElement>,
    action: Action,
    busy: bool,
    destructive: bool,
    /// The subtitle in the tint: news to act on, such as an update.
    tinted_subtitle: bool,
    /// A gray bar this wide in place of a subtitle not read yet.
    placeholder: Option<f32>,
    test_id: Option<String>,
    held: Option<Box<dyn Fn() -> Option<Choices>>>,
}

/// A row titled `title`, which shows and does nothing else yet.
pub fn list_row(title: impl Into<String>) -> ListRow {
    ListRow {
        title: title.into(),
        subtitle: None,
        value: None,
        leading: None,
        trailing: None,
        action: Action::None,
        busy: false,
        destructive: false,
        tinted_subtitle: false,
        placeholder: None,
        test_id: None,
        held: None,
    }
}

impl ListRow {
    /// A second line under the title.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into()).filter(|s: &String| !s.is_empty());
        self
    }

    /// A gray bar `width` points wide in place of a subtitle that is not
    /// read yet: the row keeps the height it has with its subtitle.
    pub fn subtitle_placeholder(mut self, width: f32) -> Self {
        self.placeholder = Some(width);
        self
    }

    /// The subtitle in the tint, for news to act on, such as an update.
    pub fn tint_subtitle(mut self) -> Self {
        self.tinted_subtitle = true;
        self
    }

    /// A tile before the title, such as an icon, a cover or a module.
    pub fn leading(mut self, leading: impl IntoElement) -> Self {
        self.leading = Some(leading.into_element());
        self
    }

    /// A value at the end, in the secondary color.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into()).filter(|v: &String| !v.is_empty());
        self
    }

    /// A control at the end, such as a switch, a chip or a small button.
    pub fn trailing(mut self, trailing: impl IntoElement) -> Self {
        self.trailing = Some(trailing.into_element());
        self
    }

    /// Run `on_press` when the row is pressed.
    pub fn press(mut self, on_press: impl Fn() + 'static) -> Self {
        self.action = Action::Press(Box::new(on_press));
        self
    }

    /// Open a page when pressed: the row ends with a chevron.
    pub fn page(mut self, open: impl Fn() + 'static) -> Self {
        self.action = Action::Page(Box::new(open));
        self
    }

    /// Open `menu` when pressed: iOS's pull-down menu, the centered menu
    /// elsewhere. The row ends with up and down chevrons.
    pub fn menu(mut self, menu: Choices) -> Self {
        self.action = Action::Menu(menu);
        self
    }

    /// Open a page outside the app when pressed: the row ends with an
    /// arrow out.
    pub fn external(mut self, open: impl Fn() + 'static) -> Self {
        self.action = Action::External(Box::new(open));
        self
    }

    /// Work is under way: a spinner takes the place of the end mark, and
    /// the row cannot be pressed.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// The title is an action that deletes or removes something.
    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    pub fn test_id(mut self, id: impl Into<String>) -> Self {
        self.test_id = Some(id.into());
        self
    }

    /// A menu that a held finger opens: iOS's context menu, the centered
    /// menu elsewhere. `menu` is read again when the row's state changes.
    pub fn held(mut self, menu: impl Fn() -> Option<Choices> + 'static) -> Self {
        self.held = Some(Box::new(menu));
        self
    }

    /// The row as one of several on a card that the caller draws, such as
    /// a reorder list: a separator under it unless it is the `last`.
    pub fn into_row(self, last: bool) -> AnyElement {
        self.render(last)
    }

    /// The least height of the row for its content.
    fn min_height(&self) -> f32 {
        if self.subtitle.is_some() || self.placeholder.is_some() {
            ROW_TWO_LINES
        } else if self.leading.is_some() {
            ROW_LEADING
        } else {
            ROW_PLAIN
        }
    }

    /// What screen readers say for the row: its title, then its value or
    /// subtitle.
    fn label(&self) -> String {
        match self.value.as_ref().or(self.subtitle.as_ref()) {
            Some(detail) => format!("{}, {detail}", self.title),
            None if self.placeholder.is_some() => format!(
                "{}, {}",
                self.title,
                crate::strings::tr("loading.reading", &[])
            ),
            None => self.title.clone(),
        }
    }

    fn render(self, last: bool) -> AnyElement {
        let height = self.min_height();
        let label = self.label();
        let busy = self.busy;
        let destructive = self.destructive;
        let separator = move || token("list-separator");
        let mark: Option<AnyElement> = if busy {
            Some(quiet_spinner(18.0))
        } else {
            match &self.action {
                Action::Page(_) => Some(chevron().into_element()),
                Action::Menu(_) => Some(menu_mark()),
                Action::External(_) => Some(external_mark()),
                Action::Press(_) | Action::None => None,
            }
        };
        let words = view()
            .flex_1()
            .flex_shrink(1.0)
            .gap(px(1.0))
            .child(
                text(self.title.clone())
                    .text_size(px(crate::ui::ty::BODY))
                    .text_color(move || {
                        if destructive {
                            token("destructive-text")
                        } else {
                            nsh_base::theme().foreground
                        }
                    }),
            )
            .child(self.subtitle.clone().map(|subtitle| {
                let tinted = self.tinted_subtitle;
                text(subtitle)
                    .text_size(px(crate::ui::ty::FOOTNOTE))
                    .text_color(move || {
                        if tinted {
                            token("tint")
                        } else {
                            token("list-secondary")
                        }
                    })
            }))
            .child(
                self.placeholder
                    .filter(|_| self.subtitle.is_none())
                    .map(|width| {
                        // The line of a subtitle, with the bar in its middle.
                        view()
                            .h(px(18.0))
                            .justify_center()
                            .child(crate::ui::skeleton_bar(width, 10.0))
                    }),
            );
        let end = view()
            .flex_1()
            .self_stretch()
            .flex_row()
            .items_center()
            .gap(px(10.0))
            .pr_4()
            // Room above and below words that wrap or take two lines.
            .py(px(10.0))
            .min_h(px(height))
            .child(words)
            .child(self.value.clone().map(|value| {
                text(value)
                    .flex_shrink(1.0)
                    .text_size(px(crate::ui::ty::BODY))
                    .text_color(move || token("list-secondary"))
            }))
            .child(self.trailing)
            .child(mark)
            // Between rows only, from where the words start.
            .child((!last).then(|| {
                view()
                    .absolute()
                    .left(px(0.0))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .h(px(0.5))
                    .bg(separator)
                    .when_some(self.test_id.as_deref(), |v, id| {
                        v.test_id(&format!("{id}:separator"))
                    })
            }));
        let mut row = view()
            .flex_row()
            .items_center()
            .gap_3()
            .pl_4()
            .min_h(px(height))
            .label(label.clone())
            .accessibility_state(busy_state(
                busy || (self.placeholder.is_some() && self.subtitle.is_none()),
            ))
            .when_some(self.test_id.as_deref(), |v, id| v.test_id(id))
            .child(self.leading)
            .child(end);
        let pressable = !busy && !matches!(self.action, Action::None);
        if pressable {
            row = row.role(AccessibilityRole::Button);
        }
        // iOS lifts a held row over the screen for its menu: it needs a fill.
        let held = self.held;
        if held.is_some() {
            row = row.bg(move || token("list-card"));
        }
        let row = match self.action {
            Action::Menu(menu) => return menu_row(menu, !busy, label, pressed_look(row)),
            Action::Press(f) | Action::Page(f) | Action::External(f) if pressable => {
                pressed_look(row).on_press(f)
            }
            _ => row,
        };
        match held {
            Some(menu) => held_view(menu, row),
            None => row.into_element(),
        }
    }
}

/// How a pressable row answers a press: the system ripple on Android, a
/// gray fill elsewhere.
fn pressed_look(row: View) -> View {
    if cfg!(target_os = "android") {
        row.ripple(nsh_core::Ripple::default())
    } else {
        row.active(|st| st.bg(move || token("list-separator")))
    }
}

/// The end mark of a row that opens a menu of values.
fn menu_mark() -> AnyElement {
    svg(nsh_core::element::ImageSrc::svg_markup(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="16" viewBox="0 0 10 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M2 6l3-3 3 3M2 10l3 3 3-3"/></svg>"#,
    ))
    .w(px(10.0))
    .h(px(16.0))
    .text_color(move || token("list-secondary"))
    .into_element()
}

/// The end mark of a row that opens a page outside the app.
fn external_mark() -> AnyElement {
    svg(nsh_core::element::ImageSrc::svg_markup(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M5 2h7v7M12 2L3 11"/></svg>"#,
    ))
    .size(px(13.0))
    .text_color(move || token("list-chevron"))
    .into_element()
}

/// A row alone, as the last row of a card that the caller draws.
impl IntoElement for ListRow {
    fn into_element(self) -> AnyElement {
        self.render(true)
    }
}

/// A card of rows, with an optional title above it and a note below it.
pub struct ListSection {
    header: Option<String>,
    footer: Option<String>,
    /// A gray bar this wide in place of a footer not read yet.
    footer_placeholder: Option<f32>,
    rows: Vec<ListRow>,
}

pub fn list_section(rows: impl IntoIterator<Item = ListRow>) -> ListSection {
    ListSection {
        header: None,
        footer: None,
        footer_placeholder: None,
        rows: rows.into_iter().collect(),
    }
}

impl ListSection {
    pub fn header(mut self, header: impl Into<String>) -> Self {
        self.header = Some(header.into());
        self
    }

    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into()).filter(|f: &String| !f.is_empty());
        self
    }

    /// A gray bar `width` points wide in place of a footer that is not read
    /// yet, on the footer's line.
    pub fn footer_placeholder(mut self, width: f32) -> Self {
        self.footer_placeholder = Some(width);
        self
    }
}

/// The words of a section's header or footer, for a note outside a
/// section, such as under a grid.
pub fn list_note(copy: impl Into<String>) -> nsh_core::element::Text {
    text(copy.into())
        .px_4()
        .text_size(px(crate::ui::ty::FOOTNOTE))
        .line_height(px(18.0))
        .text_color(move || token("list-secondary"))
}

impl IntoElement for ListSection {
    fn into_element(self) -> AnyElement {
        let last = self.rows.len().saturating_sub(1);
        let note = list_note;
        let footer_bar = self.footer_placeholder.filter(|_| self.footer.is_none());
        view()
            .gap_2()
            .child(
                self.header
                    .map(|header| note(header).role(AccessibilityRole::Header)),
            )
            .child(
                view()
                    .rounded(px(SECTION_RADIUS))
                    .overflow_hidden()
                    .bg(move || token("list-card"))
                    .children(
                        self.rows
                            .into_iter()
                            .enumerate()
                            .map(move |(index, row)| row.render(index == last)),
                    ),
            )
            .child(self.footer.map(note))
            .child(footer_bar.map(|width| {
                view()
                    .px_4()
                    .h(px(18.0))
                    .justify_center()
                    .child(crate::ui::skeleton_bar(width, 10.0))
            }))
            .into_element()
    }
}
