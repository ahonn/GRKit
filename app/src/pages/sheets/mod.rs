//! Modal sheets for pairing, connecting, update progress, and errors.
//!
//! [`spec`] says, for each [`Sheet`], what it shows and how it closes. Two
//! overlays present it: a native sheet on iOS (`native`), and a sheet that
//! the app draws elsewhere (`drawn`).

mod connect;
#[cfg(not(target_os = "ios"))]
mod drawn;
#[cfg(target_os = "ios")]
mod native;
mod parts;
mod update;

use nsh_core::prelude::*;

#[cfg(not(target_os = "ios"))]
pub use drawn::overlay;
#[cfg(target_os = "ios")]
pub use native::overlay;
pub(crate) use update::update_error_copy;

use crate::store::{AppState, Sheet};

type Action = Box<dyn Fn()>;

/// What one sheet shows and how it closes.
// The drawn sheet closes on any swipe and draws its own buttons: only the
// native sheet reads the buttons, the swipe and the height.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
struct SheetSpec {
    body: Box<dyn FnOnce() -> AnyElement>,
    /// What the sheet's own way out does (close, Not Now, Cancel, Done),
    /// for a swipe and for its close button.
    dismiss: Action,
    /// The sheet has a close button, which does `dismiss`. iOS puts it in
    /// the system bar of its native sheet; elsewhere the body draws it.
    close: bool,
    /// The button at the top left, the twin of the close button.
    back: Option<Action>,
    /// A swipe closes the native sheet. Otherwise the sheet only reports
    /// the swipe: an update under way asks before it stops.
    swipe_closes: bool,
    /// The update sheet while its steps advance in place: the sheet keeps
    /// the height of its tallest step instead of jumping. The end of an
    /// update (installed, or an error) fits its own content, so it has no
    /// space left over from a taller step.
    keeps_height: bool,
}

impl SheetSpec {
    /// A sheet with a close button, which a swipe also closes.
    fn closable(dismiss: impl Fn() + 'static, body: impl FnOnce() -> AnyElement + 'static) -> Self {
        Self {
            body: Box::new(body),
            dismiss: Box::new(dismiss),
            close: true,
            back: None,
            swipe_closes: true,
            keeps_height: false,
        }
    }
}

/// What `sheet` shows and how it closes. Adding a sheet starts here.
fn spec(app: AppState, sheet: &Sheet) -> SheetSpec {
    match sheet.clone() {
        Sheet::Nearby => SheetSpec::closable(
            move || app.dismiss_nearby(),
            move || connect::nearby(app).into_element(),
        ),
        Sheet::Scan => SheetSpec {
            back: connect::scan_shows_card(app)
                .then(|| Box::new(move || app.pin_scan_list()) as Action),
            ..SheetSpec::closable(
                move || app.set_sheet(None),
                move || connect::scan(app).into_element(),
            )
        },
        Sheet::ImportLook => SheetSpec::closable(
            move || app.set_sheet(None),
            move || crate::pages::import_sheet(app).into_element(),
        ),
        Sheet::Connecting { step } => SheetSpec {
            close: false,
            ..SheetSpec::closable(
                move || app.cancel_connect(),
                move || connect::connecting(app, step).into_element(),
            )
        },
        Sheet::ConnectError(error) => SheetSpec::closable(
            move || app.dismiss_connect_error(),
            move || connect::connect_error_sheet(app, error).into_element(),
        ),
        Sheet::Sending | Sheet::Confirm | Sheet::Restart => {
            // An update that waits on the camera stays there, and its sheet
            // only closes; one under way asks first; a restart cannot be
            // stopped.
            let waits = app.update_waits_on_camera();
            let dismiss: Action = if waits {
                Box::new(move || app.hide_update_sheet())
            } else {
                Box::new(move || {
                    if !app.update_restarting() {
                        crate::pages::ask_cancel_update(app)
                    }
                })
            };
            SheetSpec {
                body: Box::new(move || update::update_sheet(app, None, None).into_element()),
                dismiss,
                close: waits,
                back: None,
                swipe_closes: waits,
                keeps_height: true,
            }
        }
        Sheet::Installed { version, .. } => SheetSpec::closable(
            move || app.dismiss_installed(),
            move || update::update_sheet(app, None, Some(version)).into_element(),
        ),
        Sheet::Error(error) => SheetSpec::closable(
            move || app.dismiss_error(),
            move || update::update_sheet(app, Some(error), None).into_element(),
        ),
        Sheet::InstallFailed => SheetSpec::closable(
            move || app.dismiss_install_failed(),
            move || update::install_failed(app).into_element(),
        ),
    }
}

/// The spec of the open sheet.
fn open_spec(app: AppState) -> Option<SheetSpec> {
    app.sheet().map(|sheet| spec(app, &sheet))
}

/// Every sheet closes with a swipe down or a press on the backdrop, as in
/// the camera app. A restart under way stays.
fn dismissible(app: AppState) -> bool {
    app.sheet().is_some() && !app.update_restarting()
}

/// The user swiped the open sheet away.
fn dismiss(app: AppState) {
    if let Some(spec) = open_spec(app) {
        (spec.dismiss)()
    }
}

/// The body of the open sheet, drawn again when the sheet changes.
fn content(app: AppState) -> View {
    view().flex_shrink_0().child(dynamic(move || {
        open_spec(app).map_or_else(|| view().into_element(), |spec| (spec.body)())
    }))
}
