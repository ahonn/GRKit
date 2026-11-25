//! The system's own controls: switches, segmented controls and menus. NSH
//! has them only on the phones (the segmented control and the menus only
//! on iOS); elsewhere the app's drawn controls stand in for them.

use std::rc::Rc;

use nsh_base::{MenuButtonItem, context_menu, menu_button};
use nsh_core::{
    mobile::{SegmentedControlItem, segmented_control, switch},
    prelude::*,
};

use crate::strings::tr;
use crate::ui::{Segment, held_menu, quiet_spinner, segmented, toggle_loading};

const IOS: bool = cfg!(target_os = "ios");
const PHONE: bool = cfg!(any(target_os = "ios", target_os = "android"));

/// The system's switch: `UISwitch` on iOS, the Material switch on Android.
///
/// `on` is the value to show: while `saving`, the value asked for, so the
/// switch does not move back until the camera answers. A spinner shows
/// beside it while `saving`. `flip` gets the value the user asks for.
pub fn system_switch(
    on: bool,
    label: String,
    enabled: bool,
    saving: bool,
    flip: impl Fn(bool) + 'static,
) -> AnyElement {
    if !PHONE {
        return toggle_loading(on, label, enabled, saving, move || flip(!on)).into_element();
    }
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(saving.then(|| quiet_spinner(18.0)))
        .child(
            switch()
                .checked(on)
                .disabled(!enabled)
                .accessibility_label(Some(SharedString::from(label)))
                .on_change(move |next: &bool| flip(*next)),
        )
        .into_element()
}

/// The system's segmented control on iOS; the drawn one elsewhere.
pub fn system_segmented(items: Vec<Segment>) -> AnyElement {
    if !IOS {
        return segmented(items).into_element();
    }
    let selected = items
        .iter()
        .position(|(_, selected, _)| *selected)
        .unwrap_or(items.len()) as u32;
    let segments = items
        .iter()
        .map(|(title, _, _)| SegmentedControlItem::new(title.clone()))
        .collect::<Vec<_>>();
    let actions = items
        .into_iter()
        .map(|(_, _, on_press)| on_press)
        .collect::<Vec<_>>();
    segmented_control()
        .items(segments)
        .selected_index(selected)
        .w_full()
        .on_change(move |index: &u32| {
            if let Some(action) = actions.get(*index as usize) {
                action();
            }
        })
        .into_element()
}

/// One entry of a [`Choices`] menu.
#[derive(Clone)]
pub struct Choice {
    id: String,
    label: String,
    checked: Option<bool>,
    destructive: bool,
}

impl Choice {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            checked: None,
            destructive: false,
        }
    }

    /// A value of a setting, checked when it is the current one.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }
}

/// A menu as data: iOS shows it as the system's menu, the other platforms
/// as the centered menu of [`held_menu`] with a Cancel.
#[derive(Clone)]
#[must_use]
pub struct Choices {
    title: String,
    choices: Vec<Choice>,
    pick: Rc<dyn Fn(&str)>,
}

impl Choices {
    pub fn new(
        title: impl Into<String>,
        choices: Vec<Choice>,
        pick: impl Fn(&str) + 'static,
    ) -> Self {
        Self {
            title: title.into(),
            choices,
            pick: Rc::new(pick),
        }
    }

    fn items(&self) -> Vec<MenuButtonItem> {
        self.choices
            .iter()
            .map(|choice| {
                let mut item = MenuButtonItem::new(choice.id.clone(), choice.label.clone())
                    .destructive(choice.destructive);
                if let Some(checked) = choice.checked {
                    item = item.checked(checked);
                }
                item
            })
            .collect()
    }

    /// The centered menu, which has no check marks: a mark before the
    /// label stands in for one.
    pub fn show(&self) {
        let menu = self.clone();
        spawn(async move {
            let mut alert = held_menu(menu.title.clone());
            for choice in &menu.choices {
                let label = if choice.checked == Some(true) {
                    format!("✓ {}", choice.label)
                } else {
                    choice.label.clone()
                };
                alert = if choice.destructive {
                    alert.destructive(choice.id.clone(), label)
                } else {
                    alert.action(choice.id.clone(), label)
                };
            }
            let answer = alert.cancel("cancel", tr("looks.cancel", &[])).show().await;
            if let Some(id) = answer.ok().and_then(|a| a.action) {
                (menu.pick)(&id);
            }
        });
    }
}

/// A row whose tap opens `menu`: iOS's pull-down menu from the row, the
/// centered menu elsewhere. A row that cannot change opens nothing. The
/// row keeps its own pressed style. On iOS the system's button covers the
/// row, so `label` is what VoiceOver says for it.
pub fn menu_row(menu: Choices, enabled: bool, label: String, row: View) -> AnyElement {
    if !enabled {
        return row.into_element();
    }
    if IOS {
        let pick = menu.pick.clone();
        return menu_button()
            .items(menu.items())
            .title(Some(SharedString::from(menu.title.clone())))
            .accessibility_label(Some(SharedString::from(label)))
            .on_select(move |id: &SharedString| pick(id))
            .child(row)
            .into_element();
    }
    row.on_press(move || menu.show()).into_element()
}

/// A view a held finger opens `menu` on: iOS's context menu, which lifts
/// the view over the blurred screen; the centered menu elsewhere. `menu`
/// is read again when the view's state changes.
pub fn held_view(menu: impl Fn() -> Option<Choices> + 'static, content: View) -> AnyElement {
    let menu = Rc::new(menu);
    if IOS {
        let items = menu.clone();
        let title = menu.clone();
        return context_menu()
            .items(Prop::bind(move || {
                items().map(|m| m.items()).unwrap_or_default()
            }))
            .title(Prop::bind(move || {
                title().map(|m| SharedString::from(m.title))
            }))
            .on_select(move |id: &SharedString| {
                if let Some(m) = menu() {
                    (m.pick)(id);
                }
            })
            .trigger(content)
            .into_element();
    }
    content
        .on_long_press(move || {
            if let Some(m) = menu() {
                m.show();
            }
        })
        .into_element()
}
