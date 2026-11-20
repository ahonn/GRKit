//! Short notices that leave by themselves: one line at the top of the
//! screen, read aloud as it shows, swiped away or gone after a few seconds.

use std::time::Duration;

use nsh_base::{
    Anchor, ToastManager, ToastMotion, ToastOptions, ToastStackState, ToastTransitionStatus, theme,
    toast, toast_stack,
};
use nsh_core::{prelude::*, time::now};

/// How long a notice stays.
const SHOWN_FOR: Duration = Duration::from_secs(4);
/// How often the lifecycle advances while a notice is up.
const TICK: Duration = Duration::from_millis(50);

type Shown = Vec<(u64, SharedString, ToastTransitionStatus)>;

/// The app's notices: made once with the app state, shown by
/// [`toast_layer`].
#[derive(Clone, Copy)]
pub struct Toasts {
    manager: Stored<ToastManager<u64, SharedString>>,
    shown: Signal<Shown>,
    state: ToastStackState,
    next_id: Stored<u64>,
    ticking: Stored<bool>,
}

impl Toasts {
    pub fn new() -> Self {
        Self {
            manager: stored(ToastManager::new(ToastMotion::sonner())),
            shown: signal(Vec::new()),
            state: ToastStackState::new(),
            next_id: stored(0),
            ticking: stored(false),
        }
    }

    /// Show `message`. Call under an owner that outlives the notice (the
    /// app root's), since the lifecycle runs as a task.
    pub fn show(self, message: impl Into<SharedString>) {
        let id = self.next_id.with(|n| {
            *n += 1;
            *n
        });
        self.manager.with(|m| {
            m.push(
                id,
                message.into(),
                ToastOptions::timeout(Some(SHOWN_FOR)),
                now(),
            )
        });
        self.sync();
        if self.ticking.with(|t| std::mem::replace(t, true)) {
            return;
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
    }

    fn sync(self) {
        self.shown.set(self.manager.with(|m| {
            m.visible(3)
                .map(|(id, text, status)| (*id, text.clone(), status))
                .collect()
        }));
    }
}

/// The notices over the screen, under the top safe area.
pub fn toast_layer(toasts: Toasts) -> impl IntoElement {
    dynamic(move || {
        toasts.shown.with(|s| !s.is_empty()).then(|| {
            view()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .pt_safe_area_plus(px(8.0))
                .px_4()
                .items_center()
                .child(
                    toast_stack()
                        .placement(Anchor::TopCenter)
                        .state(toasts.state)
                        .w_full()
                        .max_w(px(360.0))
                        .child(each(
                            toasts.shown,
                            |(id, _, _)| *id,
                            move |item, _| {
                                toast()
                                    .transition_status(move || item.get().2)
                                    .accessibility_label(move || Some(item.get().1))
                                    .on_dismiss_request(move || {
                                        toasts.manager.with(|m| m.dismiss(&item.get().0, now()));
                                        toasts.sync();
                                    })
                                    .rounded_full()
                                    .px_5()
                                    .py_3()
                                    .items_center()
                                    .bg(move || theme().foreground)
                                    .shadow_lg()
                                    .transition_all()
                                    .duration(200.0)
                                    .starting_class("opacity-0 -translate-y-4")
                                    .ending_class("opacity-0 scale-95")
                                    .child(
                                        text(move || item.get().1)
                                            .text_size(px(15.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(move || theme().background),
                                    )
                            },
                        )),
                )
        })
    })
}
