//! Camera tab: the first camera to add, the saved cameras, and the
//! connected camera with its update, modules, Looks and recipes.

use nsh_base::alert_dialog;
use nsh_core::element::ImageSrc;
use nsh_core::element::desktop::MenuItem;
use nsh_core::element::header::HeaderItem;
use nsh_core::prelude::*;

use crate::pages::look_pages::TAB_BAR_CLEARANCE;
use crate::pages::software::{CardState, card_state};
use crate::ui::ty::text;

use crate::domain::{CameraModel, ReleaseProblem, RememberedCamera};
use crate::store::{AppState, Route, Tab, is_up_to_date};
use crate::strings::tr;
use crate::ui::PageTitle;
use crate::ui::{
    ButtonKind, ButtonSize, Choice, Choices, ListRow, TileTone, button, camera_art, glyph,
    header_busy, header_buttons, header_icon, held_view, icon_tile, list_row, list_section,
    notice_card, photo_hero, quiet_spinner, status_line, token,
};

/// Three dots: More.
const MORE_PATH: &str = "M4 12a1 1 0 1 0 2 0a1 1 0 1 0-2 0M11 12a1 1 0 1 0 2 0a1 1 0 1 0-2 0M18 12a1 1 0 1 0 2 0a1 1 0 1 0-2 0";

/// The camera tab: Connect while no camera is connected, then the camera.
pub fn camera_root(app: AppState) -> impl IntoElement {
    let online = move || app.camera_online() && app.status().is_some() && !app.initial_connect();
    crate::ui::app_page()
        .page_title(
            move || match app.status().filter(|_| online()) {
                Some(status) => format!("RICOH {}", status.camera.model.display_name()),
                // The Connect tab names the page: the bar stays empty.
                None => String::new(),
            },
            true,
        )
        // Without a camera the page has no bar to see: a see-through one,
        // as iOS brings a hidden bar back only with a page that comes in.
        // The first connect shows the connected page's layout under the
        // same bar, so the page does not move when the camera answers.
        .header_transparent(move || !online() && !app.initial_connect())
        // The back button of the pages above is the arrow alone, not the
        // camera's name.
        .header_back_title("")
        .header_right_items(move || native_header(app))
        .header_right(dynamic(move || header_actions(app)))
        .child({
            // The page stays while the status refreshes: only a connect, a
            // disconnect or another model makes it again.
            let model = computed(move || {
                (!app.initial_connect() && app.camera_online())
                    .then(|| app.status().map(|s| s.camera.model))
                    .flatten()
            });
            dynamic(move || {
                if app.initial_connect() {
                    return connecting_home().into_element();
                }
                if model.get().is_some() {
                    return untracked(|| connected(app).into_element());
                }
                match app.remembered() {
                    Some(camera) => untracked(|| saved_home(app, camera).into_element()),
                    None => empty_home(app).into_element(),
                }
            })
        })
}

/// The first connect to a saved camera: the connected page's layout with
/// nothing read yet, so the page does not move when the camera answers.
fn connecting_home() -> impl IntoElement {
    crate::ui::page_scroll()
        .top_of(Some(crate::store::Tab::Camera.key().to_owned()))
        .flex_1()
        .bg(move || token("list-background"))
        .child(
            view()
                .gap(px(24.0))
                .pt_4()
                .px_4()
                .pb(px(TAB_BAR_CLEARANCE))
                .child(current_card_frame(
                    crate::ui::hero_skeleton(),
                    Some(crate::ui::cover_row_skeleton()),
                ))
                .child(
                    list_section([
                        list_row(tr("software.title", &[]))
                            .leading(software_tile())
                            .subtitle_placeholder(SOFTWARE_BAR)
                            .trailing(crate::ui::chevron()),
                        list_row(tr("modules.title", &[]))
                            .leading(modules_tile())
                            .subtitle_placeholder(MODULES_BAR)
                            .trailing(crate::ui::chevron()),
                    ])
                    .footer_placeholder(VERSIONS_BAR),
                ),
        )
}

/// The widths of the bars in place of the words that the camera tab reads:
/// about the length of the words.
const SOFTWARE_BAR: f32 = 110.0;
const MODULES_BAR: f32 = 150.0;
const VERSIONS_BAR: f32 = 150.0;

/// The Connect page: the camera, its title and a line under it in the
/// middle, and the actions at the foot, within reach of the thumb. The bar
/// over the page is see-through, so the page keeps clear of the status bar
/// only.
fn connect_frame(
    app: AppState,
    art: impl IntoElement,
    title: String,
    line: impl IntoElement,
    actions: impl IntoElement,
) -> impl IntoElement {
    view()
        .flex_1()
        .bg(move || token("list-background"))
        .px_5()
        .pt(move || px(safe_area().top.0))
        .pb(px(TAB_BAR_CLEARANCE + 20.0))
        .child(
            view()
                .flex_1()
                .items_center()
                .justify_center()
                .gap(px(20.0))
                .child(art)
                .child(
                    text(title)
                        .mt(px(12.0))
                        .text_size(px(30.0))
                        .font_weight(FontWeight::BOLD)
                        .text_center()
                        .role(AccessibilityRole::Header),
                )
                .child(line),
        )
        .child(
            // Notices show above the buttons, not over them.
            view()
                .on_layout(move |bounds: &Bounds<Points>| {
                    app.set_connect_foot(TAB_BAR_CLEARANCE + 20.0 + bounds.size.height.0)
                })
                .child(actions),
        )
}

/// No camera saved yet: the camera, what to do on it first, and Add Camera.
fn empty_home(app: AppState) -> impl IntoElement {
    connect_frame(
        app,
        camera_art(CameraModel::Gr3, 290.0, false),
        tr("home.title", &[]),
        text(tr(
            "home.pair_first",
            &[("path", &tr("cam.path_common", &[]))],
        ))
        .text_size(px(crate::ui::ty::SUBHEADLINE))
        .line_height(px(22.0))
        .text_center()
        .text_color(move || token("list-secondary")),
        button(
            tr("home.add", &[]),
            ButtonKind::Primary,
            ButtonSize::Large,
            move || app.open_scan(),
        ),
    )
}

/// Whether the nearby watch sees this camera powered on.
fn seen_nearby(app: AppState, camera: &RememberedCamera) -> bool {
    app.nearby_cameras()
        .iter()
        .any(|c| c.powered_on && c.peripheral_id.eq_ignore_ascii_case(&camera.peripheral_id))
}

/// "On · Nearby" with a green dot, "Not connected", or the connect under way.
fn presence_line(app: AppState, camera: RememberedCamera) -> impl IntoElement {
    dynamic(move || {
        let working = if app.camera_restarting() {
            Some(tr("restart.running_title", &[]))
        } else if app.auto_connecting() {
            Some(tr("cam.auto_connecting", &[]))
        } else {
            None
        };
        if let Some(status) = working {
            return view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(quiet_spinner(14.0))
                .child(status_line(status, false))
                .into_element();
        }
        if seen_nearby(app, &camera) {
            status_line(tr("cam.nearby", &[]), true)
        } else {
            status_line(tr("connected.offline", &[]), false)
        }
    })
}

/// The most recent saved camera while none is connected: Connect, and Add
/// Camera under it. Press and hold the camera for its menu; the other saved
/// cameras are in Settings.
fn saved_home(app: AppState, camera: RememberedCamera) -> impl IntoElement {
    let menu_camera = camera.clone();
    let art = held_view(
        move || Some(camera_menu(app, &menu_camera)),
        view().child(camera_art(camera.model, 290.0, false)),
    );
    connect_frame(
        app,
        art,
        format!("RICOH {}", camera.model.display_name()),
        presence_line(app, camera),
        view()
            .gap(px(12.0))
            .child(button(
                tr("nearby.connect", &[]),
                ButtonKind::Primary,
                ButtonSize::Large,
                move || app.connect_saved(),
            ))
            .child(button(
                tr("home.add", &[]),
                ButtonKind::Tinted,
                ButtonSize::Large,
                move || app.open_scan(),
            )),
    )
}

/// One saved camera: tap to connect, press and hold for its menu. While
/// another camera is connected it only shows.
pub fn saved_row(app: AppState, camera: RememberedCamera, connected: bool) -> ListRow {
    let name = format!("RICOH {}", camera.model.display_name());
    let id = camera.peripheral_id.clone();
    let menu_camera = camera.clone();
    let busy = app.camera_online();
    let mut row = list_row(name)
        .subtitle(camera.last_ssid.clone())
        .leading(camera_art(camera.model, 48.0, false));
    if connected {
        row = row.trailing(status_line(tr("cam.connected", &[]), true));
    } else {
        row = row.held(move || Some(camera_menu(app, &menu_camera)));
    }
    if !busy {
        row = row
            .trailing(
                text(tr("nearby.connect", &[]))
                    .text_size(px(crate::ui::ty::BODY))
                    .text_color(move || token("tint")),
            )
            .press(move || app.connect_camera(&id));
    }
    row
}

/// The long-press menu of a saved camera: connect, or remove it.
pub fn camera_menu(app: AppState, camera: &RememberedCamera) -> Choices {
    let name = format!("RICOH {}", camera.model.display_name());
    let mut choices = Vec::new();
    if !app.camera_online() {
        choices.push(Choice::new("connect", tr("nearby.connect", &[])));
    }
    choices.push(Choice::new("remove", tr("cam.remove", &[])).destructive());
    let camera = camera.clone();
    Choices::new(name.clone(), choices, move |chosen| match chosen {
        "connect" => app.connect_camera(&camera.peripheral_id),
        "remove" => {
            let (camera, name) = (camera.clone(), name.clone());
            spawn(async move { confirm_remove(app, camera, name).await });
        }
        _ => {}
    })
}

async fn confirm_remove(app: AppState, camera: RememberedCamera, name: String) {
    let answer = alert_dialog()
        .title(tr("cam.remove", &[]))
        .description(tr("cam.remove_body", &[("name", &name)]))
        .cancel("cancel", tr("cam.cancel", &[]))
        .destructive("remove", tr("cam.remove_confirm", &[]))
        .show()
        .await;
    if answer.ok().and_then(|a| a.action).as_deref() == Some("remove") {
        app.forget_camera(&camera.peripheral_id);
    }
}

fn connected(app: AppState) -> impl IntoElement {
    crate::ui::page_scroll()
        .top_of(Some(crate::store::Tab::Camera.key().to_owned()))
        .refresh(Some(crate::store::page_refresh::CAMERA.to_owned()))
        .flex_1()
        .bg(move || token("list-background"))
        .child(
            view()
                .gap(px(24.0))
                .pt_4()
                .px_4()
                .pb(px(TAB_BAR_CLEARANCE))
                // Each part has its own dynamic: a list that finishes
                // loading must not redraw another part under a finger.
                .child(dynamic(move || current_card(app)))
                .child(dynamic(move || grkit_card(app))),
        )
}

/// What the camera's update says, and whether it waits on the camera.
struct UpdateState {
    title: String,
    subtitle: Option<String>,
    pending: bool,
    /// An update that the phone can install now.
    available: bool,
}

/// The update state: one the phone can install, one that waits on the
/// camera for its confirmation or a restart, module changes to send, or
/// up to date. Nothing before the camera's package is read.
fn update_state(app: AppState) -> Option<UpdateState> {
    use crate::domain::JournalState;
    let package = app.package();
    let available = package.as_ref().and_then(|p| p.available.clone());
    let version = match app.camera_journal() {
        Some(_) => app.pending_version().unwrap_or_default(),
        None => available.clone().unwrap_or_default(),
    };
    let named = |key: &str, args: &[(&str, &str)]| tr(key, args);
    let state = |title: String, subtitle: Option<String>, pending: bool, available: bool| {
        // Without a known version, the title names GRKit alone.
        let title = title.replace("GRKit  ", "GRKit ");
        Some(UpdateState {
            title,
            subtitle,
            pending,
            available,
        })
    };
    match app.camera_journal() {
        Some(JournalState::Waiting) => state(
            named("cam.update_waiting", &[("version", &version)]),
            Some(tr("cam.update_waiting_sub", &[])),
            true,
            false,
        ),
        Some(JournalState::Confirmed) => state(
            named("cam.update_ready", &[("version", &version)]),
            Some(tr("cam.update_ready_sub", &[])),
            true,
            false,
        ),
        _ if available.is_some() && !package.as_ref().is_some_and(is_up_to_date) => state(
            named("cam.update_available", &[("version", &version)]),
            None,
            false,
            true,
        ),
        _ if !app.module_draft().is_empty()
            && package
                .as_ref()
                .is_some_and(|p| p.available.as_deref() == Some(p.installed.as_str())) =>
        {
            state(
                named(
                    "modules.changes",
                    &[("count", &app.module_draft().count().to_string())],
                ),
                Some(tr("modules.view_update", &[])),
                false,
                false,
            )
        }
        // Up to date, or no answer from the release check yet: the row
        // still opens Software Update, and says what its card says.
        _ if package.is_some() => {
            let known =
                matches!(card_state(app), CardState::UpToDate) && app.release_problem().is_none();
            state(
                tr("software.title", &[]),
                known.then(|| tr("software.up_to_date", &[])),
                false,
                false,
            )
        }
        _ => None,
    }
}

/// The Software Update row. An update to install, or one that waits on
/// the camera for its confirmation or a restart, shows as red text with a
/// dot; one that waits on the camera opens Software Update with its steps
/// over it.
fn software_row(app: AppState) -> Option<ListRow> {
    let mut row = list_row(tr("software.title", &[])).leading(software_tile());
    let Some(update) = update_state(app) else {
        // The camera's versions are not read yet: the row holds its place.
        return Some(
            row.subtitle_placeholder(SOFTWARE_BAR)
                .page(move || app.open_software_update()),
        );
    };
    let pending = update.pending;
    row = row.page(move || {
        app.open_software_update();
        if pending {
            app.open_pending_update();
        }
    });
    if update.available || update.pending {
        // The version and where it is; its sheet has the steps.
        row = row.subtitle(update.title).tint_subtitle().trailing(
            view()
                .size(px(8.0))
                .rounded_full()
                .bg(move || token("tint")),
        );
    } else if update.title != tr("software.title", &[]) {
        row = row.subtitle(update.title);
    } else if let Some(subtitle) = update.subtitle {
        row = row.subtitle(subtitle);
    } else if let Some(problem) = match card_state(app) {
        // No release known and no check under way.
        CardState::Problem(problem) => Some(problem),
        CardState::Checking => None,
        _ => app.release_problem().filter(|_| !app.release_checking()),
    } {
        // Why the check gave no answer, as Software Update says it.
        row = row.subtitle(tr(
            match problem {
                ReleaseProblem::Network => "software.check_failed",
                ReleaseProblem::FirmwareTooOld { .. } => "software.firmware_old",
                ReleaseProblem::FirmwareTooNew => "software.firmware_new",
                ReleaseProblem::AppTooOld => "software.app_old",
            },
            &[],
        ));
    } else {
        // The first check is under way: the line holds its place.
        row = row.subtitle_placeholder(SOFTWARE_BAR);
    }
    Some(row)
}

/// The Software Update row's tile: the update mark, white on the dark tile
/// of the camera.
fn software_tile() -> AnyElement {
    icon_tile(glyph::DOWNLOAD, 32.0, TileTone::Camera)
}

/// Software Update and Modules, with the versions on the camera under them.
/// The versions name GRKit, so the group has no title.
fn grkit_card(app: AppState) -> Option<AnyElement> {
    let rows: Vec<ListRow> = [software_row(app), modules_row(app)]
        .into_iter()
        .flatten()
        .collect();
    if rows.is_empty() {
        return None;
    }
    Some(
        list_section(rows)
            .footer(version_line(app))
            .footer_placeholder(VERSIONS_BAR)
            .into_element(),
    )
}

/// What the camera uses now: the Look in use on the sample photo and the
/// recipe that matches the camera. Browsing and switching stay on their
/// tabs, so the card opens the detail.
fn current_card(app: AppState) -> Option<AnyElement> {
    if app.grkit_paused() || app.safe_mode() {
        let (title, body) = if app.safe_mode() {
            ("modules.safe", "modules.safe_body")
        } else {
            ("modules.grkit_off", "modules.grkit_off_body")
        };
        return Some(
            notice_card(tr(title, &[]))
                .subtitle(tr(body, &[]))
                .into_element(),
        );
    }
    let (recipes_on, looks_on) = (app.recipes_available(), app.looks_available());
    if !recipes_on && !looks_on {
        return None;
    }
    let recipes = app.recipes_snapshot();
    let looks = app.looks_snapshot();
    // A list that the phone has not read is being read, or its read failed.
    // Its part of the card holds its place until then.
    let recipes_unread = recipes_on && recipes.is_none();
    let looks_unread = looks_on && looks.is_none();
    let recipes_failed = recipes_unread && !app.recipes_loading() && app.recipes_error().is_some();
    let looks_failed = looks_unread && !app.looks_loading() && app.looks_error().is_some();
    let recipe = recipes
        .as_ref()
        .filter(|_| recipes_on)
        .and_then(|s| s.recipes.iter().find(|r| r.current).cloned());
    let look = looks
        .as_ref()
        .filter(|_| looks_on)
        .and_then(|s| s.looks.iter().find(|l| l.selected).cloned());
    let recipe_row = |item: &crate::domain::recipes::RecipeItem| {
        let id = item.id;
        list_row(app.recipe_name(item))
            .subtitle(tr("cam.current_recipe", &[]))
            .leading(crate::ui::recipe_badge(
                crate::pages::recipes::recipe_tile(app, item, 40.0, false),
                40.0,
                false,
            ))
            .page(move || open_detail(app, Tab::Recipes, Route::RecipeDetail(id)))
            .into_element()
    };
    // The photo shows the Look in use, else the recipe in use: it waits for
    // the Looks, and for the recipes while no Look is in use.
    let hero: AnyElement = if looks_unread {
        if looks_failed {
            failed_hero(tr("looks.read_failed", &[]), move || app.refresh_looks())
        } else {
            crate::ui::hero_skeleton()
        }
    } else if look.is_none() && recipes_unread {
        if recipes_failed {
            failed_hero(tr("recipes.read_failed", &[]), move || {
                app.refresh_recipes()
            })
        } else {
            crate::ui::hero_skeleton()
        }
    } else {
        match (&look, &recipe) {
            // The Look in use on the sample.
            (Some(look), _) => {
                let id = look.id.clone();
                let mut hero = photo_hero(tr("cam.current_look", &[]), look.name.clone())
                    .cover(look.shown_cover())
                    .open(move || open_detail(app, Tab::Looks, Route::LookDetail(id.clone())));
                if let Some(photo) = app.look_preview(&look.id) {
                    hero = hero.photo(ImageSrc::from(photo));
                }
                hero.into_element()
            }
            // A recipe of the camera's own Image Control: its icon, and no Look.
            (None, Some(item)) => {
                let id = item.id;
                photo_hero(tr("cam.current_recipe", &[]), app.recipe_name(item))
                    .center(crate::pages::recipes::recipe_tile(app, item, 112.0, false))
                    .open(move || open_detail(app, Tab::Recipes, Route::RecipeDetail(id)))
                    .into_element()
            }
            // Neither: the camera's own colors on the sample.
            (None, None) => {
                let mut hero = photo_hero(String::new(), tr("cam.current_none", &[])).empty_mark();
                if let Some(photo) = app.standard_preview() {
                    hero = hero.photo(ImageSrc::from(photo));
                }
                hero.into_element()
            }
        }
    };
    let row: Option<AnyElement> = if recipes_unread {
        match (recipes_failed, look.is_some() || looks_unread) {
            // Under a Look: the recipe row says why it is missing.
            (true, true) => Some(
                list_row(tr("tab.recipes", &[]))
                    .subtitle(tr("recipes.read_failed", &[]))
                    .leading(cover_box())
                    .press(move || app.set_tab(Tab::Recipes))
                    .into_element(),
            ),
            // Without a Look, the photo says it.
            (true, false) => None,
            (false, _) => Some(crate::ui::cover_row_skeleton()),
        }
    } else {
        match (&look, &recipe) {
            (Some(_), recipe) => recipe.as_ref().map(recipe_row),
            (None, Some(_)) => looks_on.then(|| {
                list_row(tr("tab.looks", &[]))
                    .value(tr("modules.not_used", &[]))
                    .into_element()
            }),
            (None, None) => None,
        }
    };
    Some(current_card_frame(hero, row))
}

/// The card of what the camera uses: the photo, and a row under it.
fn current_card_frame(hero: AnyElement, row: Option<AnyElement>) -> AnyElement {
    view()
        .rounded(px(crate::ui::SECTION_RADIUS))
        .overflow_hidden()
        .bg(move || token("list-card"))
        .child(hero)
        .child(row)
        .into_element()
}

/// The photo's place when a list could not be read: why, and another try.
fn failed_hero(message: String, retry: impl Fn() + 'static) -> AnyElement {
    view()
        .w_full()
        .h(px(crate::ui::HERO_HEIGHT))
        .bg(move || token("skeleton-area"))
        .items_center()
        .justify_center()
        .gap_3()
        .px(px(24.0))
        .child(
            text(message)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .line_height(px(20.0))
                .text_center()
                .text_color(move || token("list-secondary")),
        )
        .child(button(
            tr("looks.retry", &[]),
            ButtonKind::Tinted,
            ButtonSize::Small,
            retry,
        ))
        .into_element()
}

/// The place of a cover in a row whose list is not read.
fn cover_box() -> AnyElement {
    view()
        .w(px(40.0))
        .h(px(40.0 * 34.0 / 40.0))
        .rounded(px(3.0))
        .bg(move || token("skeleton"))
        .into_element()
}

/// The modules that run, with their values. It opens the Modules page.
/// GRKit off and safe mode stop every module, and the current card says so.
fn modules_row(app: AppState) -> Option<ListRow> {
    if app.grkit_paused() || app.safe_mode() {
        return None;
    }
    let Some(rows) = app.module_rows() else {
        return Some(unread_modules_row(app));
    };
    // The modules in use by name; while none is, the ones that are on.
    let names = |used: bool| {
        rows.iter()
            .filter(|row| {
                row.state.enabled
                    && row.managed().is_none()
                    && !row.failed()
                    && (!used || !matches!(row.summary(), crate::domain::modules::Summary::NotUsed))
            })
            .map(|row| row.title())
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let in_use = names(true);
    let subtitle = if in_use.is_empty() {
        names(false)
    } else {
        in_use
    };
    Some(
        list_row(tr("modules.title", &[]))
            .subtitle(subtitle)
            .leading(modules_tile())
            .page(move || app.open_modules_page()),
    )
}

/// The Modules row before the phone has the modules. It is the only way to
/// the Modules page, so it shows while they are read, after a failed read,
/// and when the camera cannot serve them.
fn unread_modules_row(app: AppState) -> ListRow {
    use crate::store::ModulesBlock;
    let (loading, subtitle) = match app.modules_block() {
        Some(ModulesBlock::Unsupported) => (false, "modules.unsupported_title"),
        Some(ModulesBlock::AppTooOld) => (false, "modules.app_old_title"),
        None if app.modules_error().is_some() => (false, "modules.read_failed"),
        None => (true, ""),
    };
    let mut row = list_row(tr("modules.title", &[]))
        .leading(modules_tile())
        .page(move || app.open_modules_page());
    if loading {
        // While loading, the line holds its place.
        row = row.subtitle_placeholder(MODULES_BAR);
    } else {
        row = row.subtitle(tr(subtitle, &[]));
    }
    row
}

/// The Modules row's tile: sliders, white on the dark tile of the camera,
/// as the Software Update row's.
fn modules_tile() -> AnyElement {
    icon_tile(glyph::SLIDERS, 32.0, TileTone::Camera)
}

/// Open a detail page on its tab, over the tab's list.
fn open_detail(app: AppState, tab: Tab, route: Route) {
    app.pop_to(tab, 0);
    app.set_tab(tab);
    app.push(route);
}

/// The versions on the camera, as text: Software Update opens from the
/// update row.
fn version_line(app: AppState) -> String {
    let Some(package) = app.package() else {
        return String::new();
    };
    format!(
        "{} {} · {} {}",
        tr("connected.grkit", &[]),
        package.installed,
        tr("connected.firmware", &[]),
        package.firmware
    )
}

/// More in the header (iOS): a native menu with Disconnect and Turn Off,
/// each after a confirm. It stays in place, disabled, while the camera
/// turns off.
fn native_header(app: AppState) -> Vec<HeaderItem> {
    if !app.camera_online() {
        return Vec::new();
    }
    vec![
        HeaderItem::symbol("ellipsis.circle")
            .key("more")
            .label(tr("cam.more", &[]))
            .disabled(app.powering_off())
            .menu([
                MenuItem::new(tr("cam.disconnect", &[]))
                    .symbol("wifi.slash")
                    .on_select(move || crate::pages::confirm_disconnect(app)),
                MenuItem::new(tr("cam.power", &[]))
                    .symbol("power")
                    .destructive(true)
                    .on_select(move || crate::pages::confirm_power_off(app)),
            ]),
    ]
}

/// More in the header (Android): Disconnect and Turn Off in a sheet. A
/// spinner takes its place while the camera turns off.
fn header_actions(app: AppState) -> Option<AnyElement> {
    if !app.camera_online() {
        return None;
    }
    let more = if app.powering_off() {
        header_busy(tr("cam.more", &[]))
    } else {
        header_icon(MORE_PATH, tr("cam.more", &[]), true, move || {
            connected_menu(app)
        })
    };
    Some(header_buttons([more]).into_element())
}

/// Disconnect and Turn Off, both after a confirm.
fn connected_menu(app: AppState) {
    let name = format!("RICOH {}", app.model_name());
    spawn(async move {
        let answer = alert_dialog()
            .title(name)
            .action_sheet()
            .action("disconnect", tr("cam.disconnect", &[]))
            .destructive("power", tr("cam.power", &[]))
            .cancel("cancel", tr("cam.cancel", &[]))
            .show()
            .await;
        match answer.ok().and_then(|a| a.action).as_deref() {
            Some("disconnect") => crate::pages::confirm_disconnect(app),
            Some("power") => crate::pages::confirm_power_off(app),
            _ => {}
        }
    });
}
