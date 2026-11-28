//! The Settings tab: the cameras, GRKit Settings of the connected camera,
//! About, and More from the Maker (GR Snap).

use nsh_base::alert_dialog;
use nsh_core::prelude::*;

use super::camera::saved_row;
use crate::domain::look_preview;
use crate::store::{
    APP_BUILD, APP_VERSION, AppState, GR_SNAP_URL, HELP_URL, PRIVACY_URL, Route, rate_url,
};
use crate::strings::tr;
use crate::ui::PageTitle;
use crate::ui::{ListRow, camera_art, gr_snap_icon, list_row, list_section, token};

/// The Settings tab: the cameras, About, and More from the Maker.
/// Disconnect and turn off are on the camera card.
pub fn settings_root(app: AppState) -> impl IntoElement {
    crate::ui::app_page()
        .page_title(move || tr("settings.title", &[]), true)
        .child(
            crate::pages::page_scroll()
                .refresh(Some(crate::store::page_refresh::SETTINGS.to_owned()))
                .top_of(Some(crate::store::Tab::Settings.key().to_owned()))
                .flex_1()
                .bg(move || token("list-background"))
                .child(
                    view()
                        .gap(px(24.0))
                        .pt_4()
                        .px_4()
                        .pb(px(crate::ui::TAB_BAR_CLEARANCE))
                        .child(dynamic(move || camera_section(app)))
                        .child(dynamic(move || super::grkit_settings::grkit_section(app)))
                        .child(about_section())
                        .child(
                            list_section([gr_snap_row()]).header(tr("settings.more_maker", &[])),
                        ),
                ),
        )
}

/// The app's version and the pages about it outside the app.
fn about_section() -> impl IntoElement {
    let open = |url: &'static str| {
        move || {
            spawn(async move {
                let _ = open_url(url).await;
            });
        }
    };
    let mut rows = vec![
        list_row(tr("settings.version", &[])).value(format!("{APP_VERSION} ({APP_BUILD})")),
        list_row(tr("cam.help", &[])).external(open(HELP_URL)),
    ];
    // The stores rate an app on iOS only from inside it.
    if !cfg!(target_os = "android") {
        rows.push(list_row(tr("settings.rate", &[])).external(move || {
            spawn(async move {
                let _ = open_url(rate_url()).await;
            });
        }));
    }
    rows.push(list_row(tr("settings.privacy", &[])).external(open(PRIVACY_URL)));
    // The Looks show on this photo: its author and license.
    rows.push(
        list_row(tr("settings.sample_photo", &[]))
            .subtitle(look_preview::SAMPLE_CREDIT)
            .external(open(look_preview::SAMPLE_SOURCE)),
    );
    list_section(rows).header(tr("settings.about", &[]))
}

pub fn confirm_disconnect(app: AppState) {
    spawn(async move {
        let answer = alert_dialog()
            .title(tr("cam.disconnect_title", &[]))
            .description(tr("cam.disconnect_body", &[]))
            .cancel("cancel", tr("cam.cancel", &[]))
            .destructive("disconnect", tr("cam.disconnect_confirm", &[]))
            .show()
            .await;
        if answer.ok().and_then(|a| a.action).as_deref() == Some("disconnect") {
            app.disconnect_camera();
        }
    });
}

pub fn confirm_power_off(app: AppState) {
    spawn(async move {
        let answer = alert_dialog()
            .title(tr("cam.power_title", &[]))
            .cancel("cancel", tr("cam.cancel", &[]))
            .destructive("power", tr("cam.power_confirm", &[]))
            .show()
            .await;
        if answer.ok().and_then(|a| a.action).as_deref() == Some("power") {
            app.power_off_camera();
        }
    });
}

/// The way to the saved cameras, with the art of the most recent one and
/// the models of all.
fn camera_section(app: AppState) -> Option<AnyElement> {
    let remembered = app.remembered();
    let online = app.status().filter(|_| app.camera_online());
    // The connected camera, else the one used last.
    let model = online
        .as_ref()
        .map(|s| s.camera.model)
        .or_else(|| remembered.as_ref().map(|c| c.model))?;
    let status = match (&online, &remembered) {
        (Some(_), Some(camera)) if !camera.last_ssid.is_empty() => {
            format!("{} · {}", tr("cam.connected", &[]), camera.last_ssid)
        }
        (Some(_), _) => tr("cam.connected", &[]),
        (None, _) => tr("connected.offline", &[]),
    };
    let mut rows: Vec<ListRow> = vec![
        list_row(format!("RICOH {}", model.display_name()))
            .subtitle(status)
            .leading(camera_art(model, 48.0, false)),
    ];
    let count = app.saved_cameras().len();
    if count > 0 {
        rows.push(
            list_row(tr("cam.saved", &[]))
                .value(count.to_string())
                .page(move || app.push(Route::SavedCameras)),
        );
    }
    Some(
        list_section(rows)
            .header(tr("cam.settings_group", &[]))
            .into_element(),
    )
}

/// The saved cameras: tap to connect, press and hold to remove.
pub fn saved_cameras_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    crate::ui::app_page()
        .page_title(move || tr("cam.saved", &[]), false)
        .on_dismiss(on_dismiss)
        .child(
            crate::pages::page_scroll()
                .flex_1()
                .bg(move || token("list-background"))
                .child(
                    view()
                        .pt_4()
                        .px_4()
                        .pb(px(crate::ui::TAB_BAR_CLEARANCE))
                        .child(dynamic(move || {
                            let online = app.camera_online();
                            let rows = app.saved_cameras().into_iter().enumerate().map(
                                |(index, camera)| saved_row(app, camera, online && index == 0),
                            );
                            list_section(rows).footer(tr("cam.saved_note", &[]))
                        })),
                ),
        )
}

/// GR Snap: the row opens its page.
fn gr_snap_row() -> ListRow {
    list_row(tr("settings.gr_snap_name", &[]))
        .subtitle(tr("settings.gr_snap_desc", &[]))
        .leading(
            image(gr_snap_icon())
                .size(px(36.0))
                .rounded(px(9.0))
                .object_fit(ResizeMode::Cover),
        )
        .external(|| {
            spawn(async move {
                let _ = open_url(GR_SNAP_URL).await;
            });
        })
}
