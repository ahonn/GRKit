//! A page's scroll view. On iOS it always bounces, as system pages do: a
//! large title collapses as the page scrolls and comes back only when the
//! page is pulled past its top, and a collapsed title leaves more room, so a
//! page that overflowed by little would otherwise stop scrolling. Elsewhere
//! it scrolls only when the content exceeds the space below system chrome.
use nsh_core::prelude::*;

#[derive(Debug, Clone, PartialEq, Props)]
pub struct PageScrollProps {
    pub children: Prop<Children>,
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
    scroll()
        .flex_1()
        .bounces(native)
        .scrollable(move || native || overflowing())
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
