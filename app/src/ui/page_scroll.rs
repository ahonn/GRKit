//! A page's scroll view. On iOS it always bounces, as system pages do.
//! Elsewhere it scrolls only when the content exceeds the space below
//! system chrome.
//!
//! A page named in `refresh` (`store::page_refresh`) reloads when it is
//! pulled down. The spinner shows from the pull until the reload ends.
//!
//! The root page of a tab names it in `top_of`: choosing the tab again
//! scrolls the page to its top (`AppState::reselect_tab`).
use std::time::Duration;

use nsh_core::prelude::*;

use crate::store::AppState;

/// How often a pulled page checks whether its reload ended.
const REFRESH_POLL: Duration = Duration::from_millis(200);
/// More than any inset above a page, so that a scroll to it ends at the top.
const SCROLL_TOP: f32 = 10_000.0;

#[derive(Debug, Clone, PartialEq, Props)]
pub struct PageScrollProps {
    pub children: Prop<Children>,
    /// The page that a pull reloads, or `None` for no pull to refresh.
    pub refresh: Prop<Option<String>>,
    /// The tab whose root page this is (`Tab::key`), or `None` for a
    /// pushed page.
    pub top_of: Prop<Option<String>>,
}

#[component]
pub fn PageScroll(props: PageScrollProps, cx: Scope<NoEvent>) -> impl IntoElement {
    let viewport = signal(0.0f32);
    let content = signal(0.0f32);
    let overflowing = move || {
        let insets = cx.insets();
        let available = (viewport.get() - insets.top.0 - insets.bottom.0).max(0.0);
        viewport.get() > 0.0 && content.get() > available + 1.0
    };
    let native = cfg!(target_os = "ios");
    let pulled = signal(false);
    let page = props.refresh.get();
    let app = use_context::<AppState>();
    let scroller = element_ref();
    if let (Some(tab), Some(app)) = (props.top_of.get(), app) {
        effect(move || {
            if app
                .scroll_top_request()
                .is_some_and(|(asked, _)| asked.key() == tab)
            {
                // The platforms clamp the offset to the top of the content,
                // below the bars and the insets.
                scroller.scroll_to(point(px(0.0), px(-SCROLL_TOP)), true);
            }
        });
    }
    let mut view_scroll = scroll().element_ref(&scroller);
    // A page that fits still takes a pull when it refreshes.
    let refreshes = page.is_some() && app.is_some();
    if let (Some(page), Some(app)) = (page, app) {
        view_scroll = view_scroll
            .refreshing(move || pulled.get())
            .on_refresh(move || {
                pulled.set(true);
                app.refresh_page(&page);
                let page = page.clone();
                spawn(async move {
                    loop {
                        timer(REFRESH_POLL).await;
                        if !app.page_refreshing(&page) {
                            break;
                        }
                    }
                    pulled.set(false);
                });
            });
    }
    view_scroll
        .flex_1()
        .bounces(native)
        .scrollable(move || native || refreshes || overflowing())
        .scrollbar(overflowing)
        .on_layout(move |bounds: &Bounds<Points>| viewport.set(bounds.size.height.0))
        .child(
            view()
                .w_full()
                .flex_shrink_0()
                .overflow_hidden()
                .on_layout(move |bounds: &Bounds<Points>| content.set(bounds.size.height.0))
                .child(props.children),
        )
}
