//! GRKit Settings in the Settings tab, in the order and with the names of
//! the camera's MENU: Enable at Start, then the pages Shooting Settings,
//! Display Settings, and Shutdown Screen. It shows only with a connected
//! camera.

use nsh_core::prelude::*;

use crate::domain::camera_settings::{
    CameraSettings, DistanceUnit, SettingChange, SettingSwitch, StartMode,
};
use crate::store::{AppState, PictureRead, Route, SettingsRead};
use crate::strings::tr;
use crate::ui::PageTitle;
use crate::ui::ty::text;
use crate::ui::{
    ButtonKind, ButtonSize, Choice, Choices, ListRow, button, list_row, list_section,
    system_switch, token,
};

/// The GRKit Settings group of the Settings tab, or nothing without a
/// connected camera that has read them.
pub fn grkit_section(app: AppState) -> Option<AnyElement> {
    if !app.camera_online() {
        return None;
    }
    let rows =
        match app.camera_settings() {
            // While the camera answers, a row keeps the section's place, so
            // the page does not move when the settings come.
            SettingsRead::Unknown if app.camera_settings_loading() => {
                vec![list_row(tr("grkit.title", &[])).busy(true)]
            }
            SettingsRead::Unknown => return None,
            // A GRKit from before the phone could change its settings.
            SettingsRead::Unsupported => vec![
                list_row(tr("modules.unsupported_title", &[])).trailing(button(
                    tr("software.update", &[]),
                    ButtonKind::Tinted,
                    ButtonSize::Small,
                    move || app.open_software_update(),
                )),
            ],
            SettingsRead::Ready(settings) => settings_rows(app, &settings),
        };
    Some(list_section(rows).into_element())
}

/// The rows of GRKit Settings, in the order of the camera's MENU: Enable
/// at Start opens a menu of its values, the others open their pages.
fn settings_rows(app: AppState, settings: &CameraSettings) -> Vec<ListRow> {
    let mut rows = Vec::new();
    if let Some(start) = settings.start {
        rows.push(
            list_row(tr("grkit.start", &[]))
                .value(start_text(start))
                .menu(start_menu(app, start)),
        );
    }
    if settings.switch(SettingSwitch::UncroppedRaw).is_some() {
        rows.push(
            list_row(tr("grkit.shooting", &[])).page(move || app.push(Route::ShootingSettings)),
        );
    }
    if settings.unit.is_some()
        || [
            SettingSwitch::SimpleBattery,
            SettingSwitch::BatteryPercent,
            SettingSwitch::PlaybackIcons,
        ]
        .into_iter()
        .any(|s| settings.switch(s).is_some())
    {
        rows.push(
            list_row(tr("grkit.display", &[])).page(move || app.push(Route::DisplaySettings)),
        );
    }
    if [SettingSwitch::CustomPicture, SettingSwitch::ShowText]
        .into_iter()
        .any(|s| settings.switch(s).is_some())
    {
        rows.push(
            list_row(tr("grkit.shutdown", &[])).page(move || app.push(Route::ShutdownScreen)),
        );
    }
    rows
}

fn start_text(mode: StartMode) -> String {
    tr(
        match mode {
            StartMode::On => "grkit.on",
            StartMode::Off => "grkit.off",
            StartMode::Last => "grkit.keep_last",
        },
        &[],
    )
}

fn unit_text(unit: DistanceUnit) -> String {
    tr(
        match unit {
            DistanceUnit::Meters => "grkit.meters",
            DistanceUnit::Feet => "grkit.feet",
        },
        &[],
    )
}

/// The values of a setting as a menu, the current one checked.
fn choose<T: Copy + PartialEq + 'static>(
    title: String,
    values: Vec<(T, String)>,
    current: T,
    pick: impl Fn(T) + 'static,
) -> Choices {
    let choices = values
        .iter()
        .enumerate()
        .map(|(index, (value, label))| {
            Choice::new(index.to_string(), label.clone()).checked(*value == current)
        })
        .collect();
    Choices::new(title, choices, move |id| {
        let chosen = id
            .parse::<usize>()
            .ok()
            .and_then(|index| values.get(index).map(|(value, _)| *value));
        if let Some(value) = chosen.filter(|value| *value != current) {
            pick(value);
        }
    })
}

fn start_menu(app: AppState, current: StartMode) -> Choices {
    let values = StartMode::ALL
        .into_iter()
        .map(|mode| (mode, start_text(mode)))
        .collect();
    choose(tr("grkit.start", &[]), values, current, move |mode| {
        app.set_camera_setting(SettingChange::Start(mode))
    })
}

/// A switch row of a settings page.
fn switch_row(app: AppState, switch: SettingSwitch, title: &str) -> Option<ListRow> {
    let SettingsRead::Ready(settings) = app.camera_settings() else {
        return None;
    };
    let on = settings.switch(switch)?;
    let saving =
        app.camera_setting_busy(|c| matches!(c, SettingChange::Switch(s, _) if s == switch));
    let title = tr(title, &[]);
    // The store shows the value asked for while it saves.
    let toggle = system_switch(on, title.clone(), true, saving, move |on| {
        app.set_camera_setting(SettingChange::Switch(switch, on))
    });
    Some(list_row(title).trailing(toggle))
}

fn unit_row(app: AppState) -> Option<ListRow> {
    let SettingsRead::Ready(settings) = app.camera_settings() else {
        return None;
    };
    let unit = settings.unit?;
    let values = DistanceUnit::ALL
        .into_iter()
        .map(|unit| (unit, unit_text(unit)))
        .collect();
    let menu = choose(tr("grkit.unit", &[]), values, unit, move |unit| {
        app.set_camera_setting(SettingChange::Unit(unit))
    });
    Some(
        list_row(tr("grkit.unit", &[]))
            .value(unit_text(unit))
            .menu(menu),
    )
}

/// A page of switches and choices, which follows the camera's settings.
fn settings_page(
    app: AppState,
    title: &'static str,
    rows: impl Fn(AppState) -> Vec<ListRow> + 'static,
    on_dismiss: impl Fn() + 'static,
) -> impl IntoElement {
    crate::ui::app_page()
        .page_title(move || tr(title, &[]), false)
        .on_dismiss(on_dismiss)
        .child(
            crate::ui::page_scroll()
                .refresh(Some(crate::store::page_refresh::SETTINGS.to_owned()))
                .flex_1()
                .bg(move || token("list-background"))
                .child(
                    view()
                        .pt_4()
                        .px_4()
                        .pb(px(crate::pages::look_pages::TAB_BAR_CLEARANCE))
                        .child(dynamic(move || {
                            let rows = rows(app);
                            (!rows.is_empty()).then(|| list_section(rows))
                        })),
                ),
        )
}

pub fn shooting_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    settings_page(
        app,
        "grkit.shooting",
        |app| {
            switch_row(app, SettingSwitch::UncroppedRaw, "grkit.raw")
                .into_iter()
                .collect()
        },
        on_dismiss,
    )
}

pub fn display_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    settings_page(
        app,
        "grkit.display",
        |app| {
            let rows = [
                unit_row(app),
                switch_row(app, SettingSwitch::SimpleBattery, "grkit.battery"),
                switch_row(app, SettingSwitch::BatteryPercent, "grkit.battery_percent"),
                switch_row(app, SettingSwitch::PlaybackIcons, "grkit.playback_icons"),
            ];
            rows.into_iter().flatten().collect()
        },
        on_dismiss,
    )
}

/// Shutdown Screen: the picture of the card, its switches, and the photo
/// that replaces it.
pub fn shutdown_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    app.refresh_shutdown_picture();
    crate::ui::app_page()
        .page_title(move || tr("grkit.shutdown", &[]), false)
        .on_dismiss(on_dismiss)
        .child(
            crate::ui::page_scroll()
                .refresh(Some(crate::store::page_refresh::SETTINGS.to_owned()))
                .flex_1()
                .bg(move || token("list-background"))
                .child(
                    view()
                        .gap_5()
                        .pt_4()
                        .px_4()
                        .pb(px(crate::pages::look_pages::TAB_BAR_CLEARANCE))
                        .child(dynamic(move || {
                            app.shutdown_picture_sent()
                                .then(|| sent_notice(tr("grkit.picture_sent", &[])))
                        }))
                        .child(dynamic(move || picture_preview(app)))
                        .child(dynamic(move || {
                            // Without a picture on the card the switch would
                            // turn on nothing: choosing a photo comes first.
                            let has_picture =
                                matches!(app.shutdown_picture(), PictureRead::Picture(_));
                            let rows: Vec<ListRow> = [
                                has_picture
                                    .then(|| {
                                        switch_row(
                                            app,
                                            SettingSwitch::CustomPicture,
                                            "grkit.custom_picture",
                                        )
                                    })
                                    .flatten(),
                                switch_row(app, SettingSwitch::ShowText, "grkit.show_text"),
                            ]
                            .into_iter()
                            .flatten()
                            .collect();
                            (!rows.is_empty()).then(|| list_section(rows))
                        }))
                        .child(dynamic(move || picture_buttons(app))),
                ),
        )
}

/// The line that says the camera has the new picture.
fn sent_notice(title: String) -> AnyElement {
    crate::ui::notice_card(title).into_element()
}

/// The picture at the ratio of the camera's screen.
fn picture_view(uri: String) -> AnyElement {
    view()
        .w_full()
        .aspect_ratio(3.0 / 2.0)
        .rounded(px(8.0))
        .overflow_hidden()
        .bg(rgb(0x0B0B0B))
        .child(
            image(nsh_core::element::ImageSrc::from(uri))
                .size_full()
                .object_fit(ResizeMode::Cover),
        )
        .into_element()
}

/// The picture the camera shows at power-off: the card's, or, without one,
/// the camera's own, which the phone cannot read and names instead.
fn picture_preview(app: AppState) -> Option<AnyElement> {
    match app.shutdown_picture() {
        PictureRead::Picture(uri) => Some(picture_view(uri)),
        PictureRead::None => Some(default_picture_view()),
        PictureRead::Unknown | PictureRead::Unsupported => None,
    }
}

/// The frame of the picture with the name of the camera's own picture.
fn default_picture_view() -> AnyElement {
    view()
        .w_full()
        .aspect_ratio(3.0 / 2.0)
        .rounded(px(8.0))
        .bg(rgb(0x0B0B0B))
        .items_center()
        .justify_center()
        .child(
            text(tr("grkit.default_picture", &[]))
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .text_color(rgb(0xB8B8B8)),
        )
        .into_element()
}

fn picture_buttons(app: AppState) -> Option<AnyElement> {
    let picture = app.shutdown_picture();
    if matches!(picture, PictureRead::Unknown | PictureRead::Unsupported) {
        return None;
    }
    let busy = app.shutdown_picture_busy();
    Some(
        view()
            .gap_3()
            .child(crate::ui::button(
                tr(
                    if matches!(picture, PictureRead::Picture(_)) {
                        "grkit.change_photo"
                    } else {
                        "grkit.choose_photo"
                    },
                    &[],
                ),
                crate::ui::ButtonKind::Primary,
                crate::ui::ButtonSize::Large,
                move || app.choose_shutdown_picture(),
            ))
            .child(matches!(picture, PictureRead::Picture(_)).then(|| {
                crate::ui::button(
                    tr("grkit.restore_picture", &[]),
                    crate::ui::ButtonKind::Tinted,
                    crate::ui::ButtonSize::Large,
                    move || app.save_shutdown_picture(false, || {}),
                )
                .loading(busy)
            }))
            .into_element(),
    )
}

/// The picture made from the picked photo, and the button that sends it.
pub fn crop_page(app: AppState, on_dismiss: impl Fn() + Clone + 'static) -> impl IntoElement {
    let done = on_dismiss.clone();
    crate::ui::app_page()
        .page_title(move || tr("grkit.crop", &[]), false)
        .on_dismiss(on_dismiss)
        .child(
            view()
                .flex_1()
                .bg(move || token("list-background"))
                .pt_4()
                .px_4()
                .gap_5()
                .child(dynamic(move || app.shutdown_draft().map(picture_view)))
                .child(dynamic(move || {
                    let done = done.clone();
                    crate::ui::button(
                        tr("looks.send", &[]),
                        crate::ui::ButtonKind::Primary,
                        crate::ui::ButtonSize::Large,
                        move || app.save_shutdown_picture(true, done.clone()),
                    )
                    .loading(app.shutdown_picture_busy())
                })),
        )
}
