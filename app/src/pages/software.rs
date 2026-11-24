//! Software Update page (push): the GRKit on the camera, the version the
//! phone can install, or why it cannot.

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::ui::ty::text;

use crate::domain::release_notes::{FOLDED_ITEMS, fold, history, items_of};
use crate::domain::{ReleaseProblem, UpdateKind, version_at_least};
use crate::store::{AppState, RELEASES_URL, Route, app_store_url};
use crate::strings::tr;
use crate::ui::{
    chevron_row, grouped_card, info_row, quiet_spinner, secondary_button, secondary_button_loading,
    section_title, token,
};

/// The camera maker's firmware download page.
const FIRMWARE_URL: &str = "https://www.ricoh-imaging.co.jp/english/support/digital/";

pub fn software_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    // Notes stay folded until "Show All", for as long as the page is open.
    let expanded = signal(false);
    crate::ui::app_page()
        .title(move || tr("software.title", &[]))
        .on_dismiss(on_dismiss)
        .child(dynamic(move || software_content(app, expanded)))
}

/// What the version card says.
enum CardState {
    Available(String),
    UpToDate,
    Checking,
    Problem(ReleaseProblem),
}

fn card_state(app: AppState) -> CardState {
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
        if blocked {
            disabled_pill(tr("software.update", &[]))
        } else {
            pill_button(tr("software.update", &[]), move || {
                app.begin_update(UpdateKind::Install)
            })
        }
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
            tr(
                "software.current",
                &[("version", &installed), ("model", &model)],
            ),
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
            failed.then(|| secondary_button(tr("software.retry", &[]), refresh).into_element()),
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
                tr(
                    "software.current",
                    &[("version", &installed), ("model", &model)],
                )
            } else if failed {
                tr("software.check_failed", &[])
            } else {
                tr("software.latest_sub", &[("model", &model)])
            },
            checking,
            None,
            failed.then(|| tr("software.check_failed_body", &[])),
            Some(
                secondary_button_loading(
                    tr(
                        if failed {
                            "software.retry"
                        } else {
                            "software.check"
                        },
                        &[],
                    ),
                    checking,
                    refresh,
                )
                .into_element(),
            ),
        ),
        CardState::Checking => (
            installed.clone(),
            tr(
                "software.current",
                &[("version", &installed), ("model", &model)],
            ),
            true,
            None,
            None,
            None,
        ),
        CardState::Problem(problem) => match problem {
            ReleaseProblem::Network => (
                installed.clone(),
                tr("software.check_failed", &[]),
                false,
                Some(pill_button(tr("software.retry", &[]), refresh)),
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
                Some(
                    secondary_button(tr("software.firmware_how", &[]), || {
                        open(FIRMWARE_URL.to_owned())
                    })
                    .into_element(),
                ),
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
                Some(
                    secondary_button(tr("app.store", &[]), || open(app_store_url())).into_element(),
                ),
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

    view().flex_1().bg(move || token("grouped")).child(
        crate::ui::page_scroll()
            .refresh(Some(crate::store::page_refresh::SOFTWARE.to_owned()))
            .flex_1()
            .child(
                view()
                    .gap_5()
                    .px_6()
                    .pt_4()
                    // The end clears the floating tab bar of iOS.
                    .pb_safe_area_plus(px(if cfg!(target_os = "ios") { 96.0 } else { 32.0 }))
                    .child(match pending_state(app) {
                        // An update on the camera: where it is and the one
                        // step the person takes next.
                        Some(pending) => pending_card(app, pending).into_element(),
                        None => view()
                            .gap_5()
                            .child(app.update_notice().map(notice_card))
                            .child(update_row(version, subtitle, spinner, pill))
                            .child(caption.map(|copy| {
                                text(copy)
                                    .mt(px(-8.0))
                                    .px_1()
                                    .text_size(px(crate::ui::ty::FOOTNOTE))
                                    .line_height(px(18.0))
                                    .text_color(move || theme().muted_foreground)
                            }))
                            .into_element(),
                    })
                    .child(blocked.then(blocked_notice))
                    .child((!changes.is_empty()).then(|| module_changes_card(app, changes)))
                    // Its own dynamic: "Show All" redraws the notes alone and
                    // keeps the scroll position.
                    .child((!items.is_empty()).then(|| {
                        view()
                            .gap_2()
                            .child(view().px_1().child(section_title(notes_title)))
                            .child(dynamic(move || notes_card(items.clone(), expanded)))
                    }))
                    .child(grouped_card(
                        view()
                            .child(info_row(
                                tr("connected.firmware", &[]),
                                firmware,
                                !has_history,
                            ))
                            .child(
                                has_history
                                    .then(|| history_row(move || app.push(Route::ReleaseHistory))),
                            ),
                    ))
                    .child(link),
            ),
    )
}

fn blocked_notice() -> impl IntoElement {
    view()
        .p_4()
        .rounded(px(10.0))
        .bg(move || theme().muted)
        .child(
            text(tr("modules.film_alone", &[]))
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD),
        )
}

/// The modules that this update adds and removes.
fn module_changes_card(app: AppState, changes: Vec<(String, String, bool)>) -> impl IntoElement {
    let last = changes.len().saturating_sub(1);
    let rows = changes
        .into_iter()
        .enumerate()
        .map(|(index, (id, title, add))| {
            view()
                .flex_row()
                .items_center()
                .gap_3()
                .min_h(px(54.0))
                .px_3()
                .when(index != last, |v| {
                    v.border_b_1().border_b_color(move || theme().border)
                })
                .child(crate::pages::modules::module_tile(app, id, 32.0, !add))
                .child(
                    text(title)
                        .flex_1()
                        .text_size(px(crate::ui::ty::CALLOUT))
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(
                    text(tr(
                        if add {
                            "modules.add"
                        } else {
                            "modules.delete_short"
                        },
                        &[],
                    ))
                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                    .text_color(move || {
                        if add {
                            theme().muted_foreground
                        } else {
                            theme().destructive
                        }
                    }),
                )
        });
    view()
        .gap_2()
        .child(
            view()
                .px_1()
                .child(section_title(tr("software.modules", &[]))),
        )
        .child(grouped_card(view().children(rows)))
}

/// The update button while the module changes cannot install.
fn disabled_pill(title: String) -> AnyElement {
    view()
        .min_h(px(36.0))
        .px_4()
        .rounded_full()
        .items_center()
        .justify_center()
        .bg(move || theme().muted)
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .accessibility_state(AccessibilityState {
            disabled: true,
            ..Default::default()
        })
        .child(
            text(title)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().muted_foreground),
        )
        .into_element()
}

/// The small filled button at the end of the update row.
fn pill_button(title: String, on_press: impl Fn() + 'static) -> AnyElement {
    view()
        .min_h(px(36.0))
        .px_4()
        .hit_slop(crate::ui::vertical_hit_slop(36.0))
        .rounded_full()
        .items_center()
        .justify_center()
        .bg(move || theme().foreground)
        .role(AccessibilityRole::Button)
        .label(title.clone())
        .on_press(on_press)
        .active(|s| s.opacity(0.72))
        .child(
            text(title)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().background),
        )
        .into_element()
}

/// The notes of one version, folded after [`FOLDED_ITEMS`] items until
/// "Show All". The items are plain English text.
fn notes_card(items: Vec<String>, expanded: Signal<bool>) -> impl IntoElement {
    let (shown, more) = if expanded.get() {
        (&items[..], false)
    } else {
        fold(&items, FOLDED_ITEMS)
    };
    grouped_card(
        view()
            .p_4()
            .gap_2()
            .child(bullets(shown.to_vec()))
            .child(more.then(|| {
                view().flex_row().child(crate::ui::pill_button_loading(
                    tr("software.show_all", &[]),
                    tr("software.show_all", &[]),
                    true,
                    false,
                    move || expanded.set(true),
                ))
            })),
    )
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
                    .text_color(move || theme().muted_foreground),
            )
            .child(
                text(item)
                    .flex_1()
                    .text_size(px(crate::ui::ty::SUBHEADLINE))
                    .line_height(px(22.0))
                    .text_color(move || theme().foreground),
            )
    }))
}

/// The row that opens Update History.
fn history_row(on_press: impl Fn() + 'static) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .min_h(px(50.0))
        .px_4()
        .role(AccessibilityRole::Button)
        .label(tr("software.history", &[]))
        .on_press(on_press)
        .active(|s| s.opacity(0.72).bg(move || theme().accent))
        .child(text(tr("software.history", &[])).text_size(px(crate::ui::ty::BODY)))
        .child(crate::ui::chevron())
}

/// Update History (push): the notes of every version in the release index,
/// newest first, and a link to every release on GitHub.
pub fn release_history_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    crate::ui::app_page()
        .title(move || tr("software.history", &[]))
        .on_dismiss(on_dismiss)
        .child(dynamic(move || {
            let versions = history(&app.release_notes());
            view().flex_1().bg(move || token("grouped")).child(
                crate::ui::page_scroll().flex_1().child(
                    view()
                        .gap_5()
                        .px_6()
                        .pt_4()
                        // Clears the floating tab bar of iOS.
                        .pb_safe_area_plus(px(if cfg!(target_os = "ios") { 96.0 } else { 32.0 }))
                        .children(versions.into_iter().map(|section| {
                            grouped_card(
                                view()
                                    .p_4()
                                    .gap_3()
                                    .child(
                                        text(format!("GRKit {}", section.version))
                                            .text_size(px(crate::ui::ty::HEADLINE))
                                            .font_weight(FontWeight::SEMIBOLD),
                                    )
                                    .child(bullets(section.items)),
                            )
                        }))
                        .child(secondary_button(tr("software.history_github", &[]), || {
                            open(RELEASES_URL.to_owned())
                        })),
                ),
            )
        }))
}

fn update_row(
    version: String,
    subtitle: String,
    spinner: bool,
    pill: Option<AnyElement>,
) -> impl IntoElement {
    grouped_card(
        view()
            .flex_row()
            .items_center()
            .gap_3()
            .px_4()
            .py_3()
            .child(
                view()
                    .flex_1()
                    .gap(px(2.0))
                    .child(
                        text(if version.is_empty() {
                            "GRKit".into()
                        } else {
                            format!("GRKit {version}")
                        })
                        .text_size(px(crate::ui::ty::HEADLINE))
                        .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        view()
                            .flex_row()
                            .items_center()
                            .gap(px(6.0))
                            .child(spinner.then(|| quiet_spinner(14.0)))
                            .child(
                                text(subtitle)
                                    .flex_shrink(1.0)
                                    .text_size(px(crate::ui::ty::FOOTNOTE))
                                    .text_color(move || theme().muted_foreground),
                            ),
                    ),
            )
            .child(pill),
    )
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

/// The update on the camera: where it is. The sheet has its steps and
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
    grouped_card(chevron_row(title, Some(subtitle), move || {
        app.open_pending_update()
    }))
}

/// Why the last update stopped after its send.
fn notice_card(error: crate::domain::UpdateError) -> impl IntoElement {
    let (title, body) = super::sheets::update_error_copy(&error);
    view()
        .gap_1()
        .p_4()
        .rounded(px(10.0))
        .bg(move || theme().muted)
        .child(
            text(title)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD),
        )
        .child(
            text(body)
                .text_size(px(crate::ui::ty::FOOTNOTE))
                .line_height(px(18.0))
                .text_color(move || theme().muted_foreground),
        )
}
