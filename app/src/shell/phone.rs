//! Three tabs (Camera, Looks, Recipes) with navigation stacks kept mounted.
//! Settings opens from the camera tab.

use nsh_base::{nav_stack, theme};
use nsh_core::{
    mobile::{TabBarItem, tab_bar},
    platform::Os,
    prelude::*,
};

use crate::ui::ty::text;

use crate::pages::{
    CoverTarget, add_page, camera_root, cover_page, import_page, look_page, looks_root,
    preset_page, presets_page, recipe_page, recipes_root, release_history_page, saved_cameras_page,
    scheme_page, settings_page, software_page,
};
use crate::store::{AppState, Route, Tab};
use crate::strings::tr;

/// An SF Symbol on iOS; the app's drawable on Android.
const LOOKS_ICON: &str = if cfg!(target_os = "android") {
    "camera_filters"
} else {
    "camera.filters"
};

/// An SF Symbol on iOS; the app's drawable on Android.
const RECIPES_ICON: &str = if cfg!(target_os = "android") {
    "recipes"
} else {
    "list.bullet.rectangle.portrait"
};

const TABS: &[(Tab, &str, &str)] = &[
    (Tab::Camera, "tab.camera", "camera"),
    (Tab::Looks, "tab.looks", LOOKS_ICON),
    (Tab::Recipes, "tab.recipes", RECIPES_ICON),
];

pub fn shell(app: AppState) -> impl IntoElement {
    let tab = computed({
        let app = app.clone();
        move || app.tab()
    });
    let selected = move || {
        TABS.iter()
            .position(|(t, _, _)| *t == tab.get())
            .unwrap_or(0) as u32
    };
    let mut stacks = view().flex_1();
    for &(t, _, _) in TABS {
        let hidden = move || tab.get() != t;
        stacks = stacks.child(
            view()
                .flex_1()
                .style_when(hidden, |st| st.hidden())
                .child(tab_stack(app.clone(), t)),
        );
    }
    let stacks = stacks
        .relative()
        .child(crate::ui::toast_layer(app.toasts()));
    view()
        .flex_1()
        .bg(move || theme().background)
        .child(stacks)
        .child(tabs(selected, {
            let app = app.clone();
            move |i| app.set_tab(TABS[i as usize].0)
        }))
}

fn tab_stack(app: AppState, tab: Tab) -> AnyElement {
    let mut stack = nav_stack().flex_1().child(root(app.clone(), tab));
    for depth in 0..4 {
        let at = computed({
            let app = app.clone();
            move || app.route_at(tab, depth)
        });
        let app = app.clone();
        stack = stack.child(dynamic(move || {
            at.get()
                .map(|route| untracked(|| pushed(app.clone(), tab, depth, route)))
        }));
    }
    stack.into_element()
}

fn root(app: AppState, tab: Tab) -> AnyElement {
    match tab {
        Tab::Camera => camera_root(app).into_element(),
        Tab::Looks => looks_root(app).into_element(),
        Tab::Recipes => recipes_root(app).into_element(),
    }
}

fn pushed(app: AppState, tab: Tab, depth: usize, route: Route) -> AnyElement {
    let back = {
        let app = app.clone();
        move || app.pop_to(tab, depth)
    };
    match route {
        Route::Settings => settings_page(app, back).into_element(),
        Route::SoftwareUpdate => software_page(app, back).into_element(),
        Route::ReleaseHistory => release_history_page(app, back).into_element(),
        Route::LookDetail(id) => look_page(app, id, back).into_element(),
        Route::LookCover(id) => cover_page(app, CoverTarget::Look(id), back).into_element(),
        Route::LookAdd => add_page(app, back).into_element(),
        Route::LookImport => import_page(app, back).into_element(),
        Route::LookImportCover => cover_page(app, CoverTarget::Import, back).into_element(),
        Route::SavedCameras => saved_cameras_page(app, back).into_element(),
        Route::LookScheme => scheme_page(app, back).into_element(),
        Route::RecipeDetail(id) => recipe_page(app, id, back).into_element(),
        Route::RecipePresets => presets_page(app, back).into_element(),
        Route::RecipePreset(id) => preset_page(app, id, back).into_element(),
    }
}

fn tabs(
    selected: impl Fn() -> u32 + Copy + 'static,
    choose: impl Fn(u32) + 'static + Clone,
) -> AnyElement {
    if matches!(os(), Os::Ios | Os::Android) {
        let choose = choose.clone();
        return tab_bar()
            .items(
                TABS.iter()
                    .map(|(_, key, symbol)| TabBarItem::new(tr(key, &[])).icon(*symbol))
                    .collect::<Vec<_>>(),
            )
            .selected_index(selected)
            .on_change(move |i: &u32| choose(*i))
            .into_element();
    }
    let mut row = view()
        .flex_row()
        .border_t_1()
        .border_t_color(move || theme().border)
        .bg(move || theme().card)
        .pb_safe_area();
    for (i, (_, key, _)) in TABS.iter().enumerate() {
        let i = i as u32;
        let title = tr(key, &[]);
        let choose = choose.clone();
        row = row.child(
            view()
                .flex_1()
                .min_h(px(49.0))
                .items_center()
                .justify_center()
                .role(AccessibilityRole::Button)
                .label(title.clone())
                .on_press(move || choose(i))
                .child(
                    text(title)
                        .text_size(px(crate::ui::ty::CAPTION))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || {
                            if selected() == i {
                                theme().primary
                            } else {
                                theme().muted_foreground
                            }
                        }),
                ),
        );
    }
    row.into_element()
}
