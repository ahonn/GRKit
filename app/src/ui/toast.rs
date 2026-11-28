//! Short notices that leave by themselves: one line above the tab bar, read aloud as it shows, swiped away or gone after a few seconds. A dark capsule in both appearances, so it stands off the page.

use std::time::Duration;

use nsh_base::{
    Anchor, ToastManager, ToastMotion, ToastOptions, ToastStackState, ToastTransitionStatus, toast,
    toast_stack,
};
use nsh_core::{prelude::*, time::now};

use crate::ui::ty::text;

/// How long a notice stays.
const SHOWN_FOR: Duration = Duration::from_secs(4);
/// How often the lifecycle advances while a notice is up.
const TICK: Duration = Duration::from_millis(50);

/// A notice: its text, and the title of its action.
type Notice = (SharedString, Option<SharedString>);
type Shown = Vec<(u64, Notice, ToastTransitionStatus)>;
type Actions = Vec<(u64, std::rc::Rc<dyn Fn()>)>;

/// The app's notices: made once with the app state, shown by
/// [`toast_layer`].
#[derive(Clone, Copy)]
pub struct Toasts {
    manager: Stored<ToastManager<u64, Notice>>,
    actions: Stored<Actions>,
    shown: Signal<Shown>,
    state: ToastStackState,
    next_id: Stored<u64>,
    ticking: Stored<bool>,
}

impl Toasts {
    pub fn new() -> Self {
        Self {
            manager: stored(ToastManager::new(ToastMotion::sonner())),
            actions: stored(Vec::new()),
            shown: signal(Vec::new()),
            state: ToastStackState::new(),
            next_id: stored(0),
            ticking: stored(false),
        }
    }

    /// Show `message`. Call under an owner that outlives the notice (the
    /// app root's), since the lifecycle runs as a task.
    pub fn show(self, message: impl Into<SharedString>) {
        self.push((message.into(), None), Some(SHOWN_FOR));
    }

    /// Show `message` with a button that runs `action` and closes the
    /// notice. While a screen reader runs, the notice stays until the
    /// person acts on it or closes it: its time would run out before they
    /// reach it.
    pub fn show_action(
        self,
        message: impl Into<SharedString>,
        title: impl Into<SharedString>,
        action: impl Fn() + 'static,
    ) {
        let message = message.into();
        let title = title.into();
        announce_for_accessibility(format!("{message}, {title}"));
        let timeout = (!screen_reader()).then_some(SHOWN_FOR);
        let id = self.push((message, Some(title)), timeout);
        self.actions
            .with(|a| a.push((id, std::rc::Rc::new(action))));
    }

    fn push(self, notice: Notice, timeout: Option<Duration>) -> u64 {
        let id = self.next_id.with(|n| {
            *n += 1;
            *n
        });
        self.manager
            .with(|m| m.push(id, notice, ToastOptions::timeout(timeout), now()));
        self.sync();
        if self.ticking.with(|t| std::mem::replace(t, true)) {
            return id;
        }
        // Advance only while a notice is up; paused while a finger holds it.
        spawn(async move {
            loop {
                timer(TICK).await;
                let paused = self.state.is_expanded();
                if self.manager.with(|m| m.advance(now(), paused).changed) {
                    self.sync();
                }
                if self.shown.with(|s| s.is_empty()) {
                    self.ticking.with(|t| *t = false);
                    break;
                }
            }
        });
        id
    }

    /// Run the action of notice `id` and close it.
    fn act(self, id: u64) {
        let action = self
            .actions
            .with(|a| a.iter().find(|(i, _)| *i == id).map(|(_, f)| f.clone()));
        self.manager.with(|m| m.dismiss(&id, now()));
        self.sync();
        if let Some(action) = action {
            action();
        }
    }

    fn dismiss(self, id: u64) {
        self.manager.with(|m| m.dismiss(&id, now()));
        self.sync();
    }

    fn sync(self) {
        let ids: Vec<u64> = self
            .manager
            .with(|m| m.visible(3).map(|(id, _, _)| *id).collect());
        self.actions.with(|a| a.retain(|(id, _)| ids.contains(id)));
        self.shown.set(self.manager.with(|m| {
            m.visible(3)
                .map(|(id, text, status)| (*id, text.clone(), status))
                .collect()
        }));
    }
}

/// The notices at the bottom of the content, clear of the page header.
/// `lift`: the height of a tab bar that floats over the content's bottom.
pub fn toast_layer(toasts: Toasts, lift: impl Fn() -> f32 + 'static) -> impl IntoElement {
    dynamic(move || {
        let bottom = 12.0 + lift();
        toasts.shown.with(|s| !s.is_empty()).then(|| {
            view()
                .absolute()
                .bottom(px(bottom))
                .left_0()
                .right_0()
                .px_4()
                .items_center()
                .child(
                    toast_stack()
                        .placement(Anchor::BottomCenter)
                        .state(toasts.state)
                        .w_full()
                        .max_w(px(360.0))
                        .child(each(
                            toasts.shown,
                            |(id, _, _)| *id,
                            move |item, _| {
                                toast()
                                    .transition_status(move || item.get().2)
                                    // A notice with a button says itself, so its button
                                    // stays its own element for a screen reader: a
                                    // label would make the notice one element.
                                    .accessibility_label(move || {
                                        let (_, (message, action), _) = item.get();
                                        action.is_none().then_some(message)
                                    })
                                    .on_dismiss_request(move || toasts.dismiss(item.get().0))
                                    .rounded_full()
                                    .px_5()
                                    .py_3()
                                    .items_center()
                                    .bg(rgba(0x1E1E20EB))
                                    .shadow_lg()
                                    .transition_all()
                                    .duration(200.0)
                                    .starting_class("opacity-0 translate-y-4")
                                    .ending_class("opacity-0 scale-95")
                                    .flex_row()
                                    .gap(px(14.0))
                                    .child(
                                        // The button sits at the end.
                                        text(move || item.get().1.0)
                                            .flex_1()
                                            .text_size(px(crate::ui::ty::SUBHEADLINE))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(rgb(0xFFFFFF)),
                                    )
                                    .child(dynamic(move || {
                                        item.get().1.1.map(|title| {
                                            view()
                                                .min_h(px(36.0))
                                                .px(px(14.0))
                                                .my(px(-6.0))
                                                // 6 pt from the edge on each
                                                // side, as above and below.
                                                .mr(px(-14.0))
                                                .rounded_full()
                                                .justify_center()
                                                .bg(rgba(0xFFFFFF24))
                                                .hit_slop(crate::ui::vertical_hit_slop(36.0))
                                                .role(AccessibilityRole::Button)
                                                .label(title.to_string())
                                                .on_press(move || toasts.act(item.get().0))
                                                .child(
                                                    text(title)
                                                        .text_size(px(crate::ui::ty::SUBHEADLINE))
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(rgb(0xFFFFFF)),
                                                )
                                        })
                                    }))
                            },
                        )),
                )
        })
    })
}
