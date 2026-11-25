//! Up to five tabs (Camera, Modules, Looks, Recipes, Settings) with navigation
//! stacks kept mounted. The Looks and Recipes tabs show while their module
//! runs. Without a connected camera only Camera and Settings show. A restart
//! for changed Looks keeps the tabs of the camera.

use nsh_base::{nav_stack, theme};
use nsh_core::{
    mobile::{TabBarItem, tab_bar},
    platform::Os,
    prelude::*,
};

use crate::ui::ty::text;

use crate::pages::grkit_settings;
use crate::pages::{
    CoverTarget, add_page, camera_root, cover_page, import_page, look_page, looks_root,
    module_options_page, module_page, modules_page, preset_page, presets_page, recipe_page,
    recipes_root, release_history_page, saved_cameras_page, settings_root, software_page,
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

/// The camera tab while no camera is connected: Connect, with the Wi-Fi
/// symbol. See [`tab_item`].
const CONNECT_TAB: (&str, &str) = ("tab.connect", "wifi");

const TABS: &[(Tab, &str, &str)] = &[
    (Tab::Camera, "tab.camera", "camera"),
    (Tab::Looks, "tab.looks", LOOKS_ICON),
    (Tab::Recipes, "tab.recipes", RECIPES_ICON),
    (Tab::Settings, "tab.settings", "gearshape"),
];

pub fn shell(app: AppState) -> impl IntoElement {
    let tab = computed({
        let app = app.clone();
        move || app.tab()
    });
    let shown = computed({
        let app = app.clone();
        move || {
            TABS.iter()
                .map(|(t, _, _)| *t)
                .filter(|t| app.tab_available(*t))
                .collect::<Vec<_>>()
        }
    });
    let selected = move || {
        shown
            .get()
            .iter()
            .position(|t| *t == tab.get())
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
    // iOS 26 floats its tab bar over the content, which scrolls on under
    // it; Android's bar takes its own row below the content.
    let overlay = cfg!(target_os = "ios");
    let bar_height = signal(0.0f32);
    let lift = move || {
        let bar = if overlay { bar_height.get() } else { 0.0 };
        bar.max(app.toast_floor())
    };
    let stacks = stacks
        .relative()
        .child(crate::ui::toast_layer(app.toasts(), lift));
    let online = computed({
        let app = app.clone();
        move || app.camera_online()
    });
    let bar = dynamic(move || {
        let visible = shown.get();
        let online = online.get();
        let app = app.clone();
        let items = visible.clone();
        untracked(|| {
            tabs(&visible, online, selected, move |i| {
                if let Some(tab) = items.get(i as usize) {
                    if *tab == app.tab() {
                        app.reselect_tab(*tab);
                    } else {
                        app.set_tab(*tab);
                    }
                }
            })
        })
    });
    let root = view().flex_1().bg(move || theme().background).child(stacks);
    if overlay {
        root.relative().child(
            view()
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .on_layout(move |bounds: &Bounds<Points>| bar_height.set(bounds.size.height.0))
                .child(bar),
        )
    } else {
        root.child(bar)
    }
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
        Tab::Settings => settings_root(app).into_element(),
    }
}

fn pushed(app: AppState, tab: Tab, depth: usize, route: Route) -> AnyElement {
    let back = {
        let app = app.clone();
        move || app.pop_to(tab, depth)
    };
    match route {
        Route::SoftwareUpdate => software_page(app, back).into_element(),
        Route::Modules => modules_page(app, back).into_element(),
        Route::ReleaseHistory => release_history_page(app, back).into_element(),
        Route::LookDetail(id) => look_page(app, id, back).into_element(),
        Route::LookCover(id) => cover_page(app, CoverTarget::Look(id), back).into_element(),
        Route::LookAdd => add_page(app, back).into_element(),
        Route::LookImport => import_page(app, back).into_element(),
        Route::LookImportCover => cover_page(app, CoverTarget::Import, back).into_element(),
        Route::SavedCameras => saved_cameras_page(app, back).into_element(),
        Route::RecipeDetail(id) => recipe_page(app, id, back).into_element(),
        Route::RecipePresets => presets_page(app, back).into_element(),
        Route::RecipePreset(id) => preset_page(app, id, back).into_element(),
        Route::ModuleDetail(id) => module_page(app, id, back).into_element(),
        Route::ModuleOptions(id) => module_options_page(app, id, back).into_element(),
        Route::ShootingSettings => grkit_settings::shooting_page(app, back).into_element(),
        Route::DisplaySettings => grkit_settings::display_page(app, back).into_element(),
        Route::ShutdownScreen => grkit_settings::shutdown_page(app, back).into_element(),
        Route::ShutdownCrop => grkit_settings::crop_page(app, back).into_element(),
        Route::LookUnlock(firmware) => {
            crate::pages::unlock_page(app, firmware, back).into_element()
        }
    }
}

/// A tab's title key and symbol. The camera tab is Connect until a camera
/// connects, then Camera.
fn tab_item(
    tab: Tab,
    key: &'static str,
    symbol: &'static str,
    online: bool,
) -> (&'static str, &'static str) {
    if tab == Tab::Camera && !online {
        CONNECT_TAB
    } else {
        (key, symbol)
    }
}

fn tabs(
    visible: &[Tab],
    online: bool,
    selected: impl Fn() -> u32 + Copy + 'static,
    choose: impl Fn(u32) + 'static + Clone,
) -> AnyElement {
    let items: Vec<(Tab, &str, &str)> = TABS
        .iter()
        .filter(|(t, _, _)| visible.contains(t))
        .map(|&(t, key, symbol)| {
            let (key, symbol) = tab_item(t, key, symbol, online);
            (t, key, symbol)
        })
        .collect();
    if matches!(os(), Os::Ios | Os::Android) {
        let choose = choose.clone();
        return tab_bar()
            .items(
                items
                    .iter()
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
    for (i, (_, key, _)) in items.iter().enumerate() {
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
