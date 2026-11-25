//! Software Update page (push): the GRKit on the camera, the version the
//! phone can install, or why it cannot.

use nsh_core::prelude::*;

use crate::ui::ty::text;

use crate::domain::release_notes::{FOLDED_ITEMS, fold, history, items_of};
use crate::domain::{ReleaseProblem, UpdateKind, version_at_least};
use crate::store::{AppState, RELEASES_URL, Route, app_store_url};
use crate::strings::tr;
use crate::ui::PageTitle;
use crate::ui::{
    ButtonKind, ButtonSize, SECTION_RADIUS, TileTone, button, glyph, icon_tile, list_note,
    list_row, list_section, notice_card, quiet_spinner, text_link, token,
};

/// The camera maker's firmware download page.
const FIRMWARE_URL: &str = "https://www.ricoh-imaging.co.jp/english/support/digital/";

pub fn software_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    // Notes stay folded until "Show All", for as long as the page is open.
    let expanded = signal(false);
    crate::ui::app_page()
        .page_title(move || tr("software.title", &[]), false)
        .on_dismiss(on_dismiss)
        .child(dynamic(move || software_content(app, expanded)))
}

/// What the version card says.
pub(crate) enum CardState {
    Available(String),
    UpToDate,
    Checking,
    Problem(ReleaseProblem),
}

pub(crate) fn card_state(app: AppState) -> CardState {
    let package = app.package();
    let installed = package
        .as_ref()
        .map(|p| p.installed.clone())
        .unwrap_or_default();
    match package.as_ref().and_then(|p| p.available.clone()) {
        Some(latest) if !version_at_least(&installed, &latest) => CardState::Available(latest),
        Some(_) => CardState::UpToDate,
        None if app.release_checking() => CardState::Checking,
        None => match app.release_problem() {
            Some(problem) => CardState::Problem(problem),
            None if app.demo() => CardState::UpToDate,
            // No answer and no check under way: offer another try.
            None => CardState::Problem(ReleaseProblem::Network),
        },
    }
}

pub(crate) fn open(url: String) {
    spawn(async move {
        let _ = open_url(&url).await;
    });
}

fn software_content(app: AppState, expanded: Signal<bool>) -> impl IntoElement {
    let package = app.package();
    let installed = package
        .as_ref()
        .map(|p| p.installed.clone())
        .unwrap_or_default();
    let firmware = package
        .as_ref()
        .map(|p| p.firmware.clone())
        .unwrap_or_default();
    let model = format!("RICOH {}", app.model_name());
    let state = card_state(app);
    let refresh = move || app.refresh_update_info();
    let changes = app.module_changes();
    let on_release = package
        .as_ref()
        .is_some_and(|p| p.available.as_deref() == Some(p.installed.as_str()));
    let blocked = app.module_draft_blocked();
    let update_button = move || {
        button(
            tr("software.update", &[]),
            ButtonKind::Primary,
            ButtonSize::Medium,
            move || app.begin_update(UpdateKind::Install),
        )
        .enabled(!blocked)
        .into_element()
    };
    // An action below the page's content; `external` opens a page outside the app.
    let link_button = |title: String, external: bool, on_press: Box<dyn Fn()>| {
        text_link(title, external, on_press)
    };
    // A version the phone knows stays while it checks again or fails to.
    let checking = app.release_checking();
    let failed = app.release_problem().is_some();
    // The row: its version and line, a spinner, and the button at its end.
    // Under it: a caption and, below the info rows, a link button.
    let (version, subtitle, spinner, pill, caption, link): (
        String,
        String,
        bool,
        Option<AnyElement>,
        Option<String>,
        Option<AnyElement>,
    ) = match &state {
        // A check under way shows its spinner, not a line of its own (R5).
        CardState::Available(version) => (
            version.clone(),
            model.clone(),
            checking,
            Some(update_button()),
            Some(tr(
                if failed {
                    "software.check_failed"
                } else {
                    "software.confirm_note"
                },
                &[],
            )),
            failed.then(|| link_button(tr("software.retry", &[]), false, Box::new(refresh))),
        ),
        // The release is the installed version: a change of the modules
        // never installs an older GRKit.
        CardState::UpToDate if !changes.is_empty() && on_release => (
            installed.clone(),
            tr(
                "software.module_changes",
                &[("count", &changes.len().to_string()), ("model", &model)],
            ),
            checking,
            Some(update_button()),
            Some(tr("software.module_note", &[])),
            None,
        ),
        CardState::UpToDate => (
            installed.clone(),
            if checking {
                model.clone()
            } else if failed {
                tr("software.check_failed", &[])
            } else {
                tr("software.latest_sub", &[("model", &model)])
            },
            checking,
            None,
            failed.then(|| tr("software.check_failed_body", &[])),
            // A check under way shows its spinner in the card.
            (!checking).then(|| {
                link_button(
                    tr(
                        if failed {
                            "software.retry"
                        } else {
                            "software.check"
                        },
                        &[],
                    ),
                    false,
                    Box::new(refresh),
                )
            }),
        ),
        CardState::Checking => (installed.clone(), model.clone(), true, None, None, None),
        CardState::Problem(problem) => match problem {
            ReleaseProblem::Network => (
                installed.clone(),
                tr("software.check_failed", &[]),
                false,
                Some(
                    button(
                        tr("software.retry", &[]),
                        ButtonKind::Primary,
                        ButtonSize::Medium,
                        refresh,
                    )
                    .into_element(),
                ),
                Some(tr("software.check_failed_body", &[])),
                None,
            ),
            ReleaseProblem::FirmwareTooOld { required } => (
                installed.clone(),
                tr("software.firmware_old", &[]),
                false,
                None,
                Some(tr(
                    "software.firmware_old_body",
                    &[("model", &model), ("firmware", required)],
                )),
                Some(link_button(
                    tr("software.firmware_how", &[]),
                    true,
                    Box::new(|| open(FIRMWARE_URL.to_owned())),
                )),
            ),
            ReleaseProblem::FirmwareTooNew => (
                installed.clone(),
                tr("software.firmware_new", &[]),
                false,
                None,
                Some(tr("software.firmware_new_body", &[("firmware", &firmware)])),
                None,
            ),
            ReleaseProblem::AppTooOld => (
                installed.clone(),
                tr("software.app_old", &[]),
                false,
                None,
                Some(tr("software.app_old_body", &[])),
                Some(link_button(
                    tr("app.store", &[]),
                    true,
                    Box::new(|| open(app_store_url())),
                )),
            ),
        },
    };
    // The notes of the version on offer, or of the one installed.
    let notes = app.release_notes();
    let (items, notes_title) = match &state {
        CardState::Available(version) => (items_of(&notes, version), tr("software.whats_new", &[])),
        CardState::UpToDate => (
            items_of(&notes, &installed),
            tr("software.whats_new_here", &[]),
        ),
        _ => (Vec::new(), String::new()),
    };
    let has_history = !history(&notes).is_empty();

    let mut info = vec![list_row(tr("connected.firmware", &[])).value(firmware)];
    if has_history {
        info.push(
            list_row(tr("software.history", &[])).page(move || app.push(Route::ReleaseHistory)),
        );
    }

    view().flex_1().bg(move || token("list-background")).child(
        crate::ui::page_scroll()
            .refresh(Some(crate::store::page_refresh::SOFTWARE.to_owned()))
            .flex_1()
            .child(
                view()
                    .gap(px(24.0))
                    .px_4()
                    .pt_4()
                    // The end clears the floating tab bar of iOS.
                    .pb_safe_area_plus(px(if cfg!(target_os = "ios") { 96.0 } else { 32.0 }))
                    .child(match pending_state(app) {
                        // An update on the camera: where it is and the one
                        // step the person takes next.
                        Some(pending) => pending_card(app, pending).into_element(),
                        None => view()
                            .gap_2()
                            .child(
                                app.update_notice()
                                    .map(|error| view().pb_4().child(error_notice(error))),
                            )
                            .child(update_card(version, subtitle, spinner, pill))
                            .child(caption.map(list_note))
                            .into_element(),
                    })
                    .child(blocked.then(|| notice_card(tr("modules.film_alone", &[]))))
                    .child((!changes.is_empty()).then(|| module_changes_card(app, changes)))
                    // Its own dynamic: "Show All" redraws the notes alone and
                    // keeps the scroll position.
                    .child((!items.is_empty()).then(|| {
                        view()
                            .gap_2()
                            .child(list_note(notes_title).role(AccessibilityRole::Header))
                            .child(dynamic(move || notes_card(items.clone(), expanded)))
                    }))
                    .child(list_section(info))
                    .child(link),
            ),
    )
}

/// The modules that this update adds and removes.
fn module_changes_card(app: AppState, changes: Vec<(String, String, bool)>) -> impl IntoElement {
    let rows = changes.into_iter().map(|(id, title, add)| {
        list_row(title)
            .leading(crate::pages::modules::module_tile(app, id, 32.0, !add))
            .trailing(
                text(tr(
                    if add {
                        "modules.add"
                    } else {
                        "modules.delete_short"
                    },
                    &[],
                ))
                .text_size(px(crate::ui::ty::BODY))
                .text_color(move || {
                    if add {
                        token("list-secondary")
                    } else {
                        token("destructive-text")
                    }
                }),
            )
    });
    list_section(rows).header(tr("software.modules", &[]))
}

/// The notes of one version, folded after [`FOLDED_ITEMS`] items until
/// "Show All". The items are plain English text.
fn notes_card(items: Vec<String>, expanded: Signal<bool>) -> impl IntoElement {
    let (shown, more) = if expanded.get() {
        (&items[..], false)
    } else {
        fold(&items, FOLDED_ITEMS)
    };
    text_card()
        .gap_3()
        .child(bullets(shown.to_vec()))
        .child(more.then(|| {
            view()
                .flex_row()
                .child(text_link(tr("software.show_all", &[]), false, move || {
                    expanded.set(true)
                }))
        }))
}

/// A card of words, as wide as a list section.
fn text_card() -> View {
    view()
        .p_4()
        .rounded(px(SECTION_RADIUS))
        .bg(move || token("list-card"))
}

fn bullets(items: Vec<String>) -> impl IntoElement {
    view().gap_2().children(items.into_iter().map(|item| {
        view()
            .flex_row()
            .gap_2()
            .child(
                text("•")
                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                    .line_height(px(22.0))
                    .text_color(move || token("list-secondary")),
            )
            .child(
                text(item)
                    .flex_1()
                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                    .line_height(px(22.0)),
            )
    }))
}

/// Update History (push): the notes of every version in the release index,
/// newest first, and a link to every release on GitHub.
pub fn release_history_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    crate::ui::app_page()
        .page_title(move || tr("software.history", &[]), false)
        .on_dismiss(on_dismiss)
        .child(dynamic(move || {
            let versions = history(&app.release_notes());
            view().flex_1().bg(move || token("list-background")).child(
                crate::ui::page_scroll().flex_1().child(
                    view()
                        .gap_4()
                        .px_4()
                        .pt_4()
                        // Clears the floating tab bar of iOS.
                        .pb_safe_area_plus(px(if cfg!(target_os = "ios") { 96.0 } else { 32.0 }))
                        .children(versions.into_iter().map(|section| {
                            text_card()
                                .gap_3()
                                .child(
                                    text(format!("GRKit {}", section.version))
                                        .text_size(px(crate::ui::ty::HEADLINE))
                                        .font_weight(FontWeight::SEMIBOLD),
                                )
                                .child(bullets(section.items))
                        }))
                        .child(text_link(tr("software.history_github", &[]), true, || {
                            open(RELEASES_URL.to_owned())
                        })),
                ),
            )
        }))
}

/// The GRKit on the camera, or the version on offer, in one row: the
/// update mark, the version and its line, and the page's main button at its
/// end.
fn update_card(
    version: String,
    subtitle: String,
    spinner: bool,
    action: Option<AnyElement>,
) -> impl IntoElement {
    let title = if version.is_empty() {
        "GRKit".to_owned()
    } else {
        format!("GRKit {version}")
    };
    version_row(title, version_line(subtitle, spinner, false), action)
}

/// The row of the version card: the update mark, the title and its line,
/// and what ends the row.
fn version_row(title: String, line: AnyElement, end: Option<AnyElement>) -> View {
    view()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .px_4()
        .py(px(14.0))
        .rounded(px(SECTION_RADIUS))
        .bg(move || token("list-card"))
        .child(icon_tile(glyph::DOWNLOAD, 52.0, TileTone::Camera))
        .child(
            view()
                .flex_1()
                .gap(px(2.0))
                .child(
                    text(title)
                        .text_size(px(crate::ui::ty::HEADLINE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .role(AccessibilityRole::Header),
                )
                .child(line),
        )
        .child(end)
}

/// The line under the version: a spinner while something runs, in the
/// tint when the person has a step to take.
fn version_line(subtitle: String, spinner: bool, tint: bool) -> AnyElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .child(spinner.then(|| quiet_spinner(12.0)))
        .child(
            text(subtitle)
                .flex_shrink(1.0)
                .text_size(px(crate::ui::ty::FOOTNOTE))
                .text_color(move || {
                    if tint {
                        token("tint")
                    } else {
                        token("list-secondary")
                    }
                }),
        )
        .into_element()
}

/// Where an update on the camera is, after its send.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pending {
    /// It waits for its confirmation on the camera.
    Waiting,
    /// Confirmed: the next start of the camera installs it.
    Ready,
    /// The phone is turning the camera off and on.
    Restarting,
    /// The phone could not restart the camera: the person does.
    Manual,
}

fn pending_state(app: AppState) -> Option<Pending> {
    use crate::domain::{JournalState, UpdatePhase};
    match app.flow_phase() {
        UpdatePhase::Restarting { .. } => return Some(Pending::Restarting),
        UpdatePhase::ManualRestart { .. } => return Some(Pending::Manual),
        UpdatePhase::WaitingConfirm { .. } => return Some(Pending::Waiting),
        UpdatePhase::Confirmed { .. } | UpdatePhase::Restart { .. } => {
            return Some(Pending::Ready);
        }
        // The send has its own sheet.
        UpdatePhase::Downloading { .. } | UpdatePhase::Sending { .. } => return None,
        _ => {}
    }
    match app.camera_journal() {
        Some(JournalState::Waiting) => Some(Pending::Waiting),
        Some(JournalState::Confirmed) => Some(Pending::Ready),
        _ => None,
    }
}

/// The update on the camera, in the version card's row: where it is, in the
/// tint when the person has a step to take. The sheet has its steps and
/// buttons, so the row opens it.
fn pending_card(app: AppState, pending: Pending) -> impl IntoElement {
    let version = app.pending_version().unwrap_or_default();
    let title = tr(
        if pending == Pending::Waiting {
            "cam.update_waiting"
        } else {
            "cam.update_ready"
        },
        &[("version", &version)],
    )
    .replace("GRKit  ", "GRKit ");
    let subtitle = tr(
        match pending {
            Pending::Waiting => "cam.update_waiting_sub",
            Pending::Ready => "cam.update_ready_sub",
            Pending::Restarting => "restart.running_title",
            Pending::Manual => "software.restart_manual",
        },
        &[],
    );
    let restarting = pending == Pending::Restarting;
    let label = format!("{title}, {subtitle}");
    version_row(
        title,
        version_line(subtitle, restarting, !restarting),
        Some(crate::ui::chevron().into_element()),
    )
    .role(AccessibilityRole::Button)
    .label(label)
    .on_press(move || app.open_pending_update())
    .active(|s| s.opacity(0.72))
}

/// Why the last update stopped after its send.
fn error_notice(error: crate::domain::UpdateError) -> impl IntoElement {
    let (title, body) = super::sheets::update_error_copy(&error);
    notice_card(title).subtitle(body)
}
