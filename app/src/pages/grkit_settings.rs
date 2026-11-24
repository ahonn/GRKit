//! GRKit Settings in the Settings tab, in the order and with the names of
//! the camera's MENU: Enable at Start, then the pages Shooting Settings,
//! Display Settings, and Shutdown Screen. It shows only with a connected
//! camera.

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::domain::camera_settings::{
    CameraSettings, DistanceUnit, SettingChange, SettingSwitch, StartMode,
};
use crate::store::{AppState, PictureRead, Route, SettingsRead};
use crate::strings::tr;
use crate::ui::ty::text;
use crate::ui::{
    busy_state, control_row, grouped_card, quiet_spinner, section_title, toggle_loading, token,
};

/// The GRKit Settings group of the Settings tab, or nothing without a
/// connected camera that has read them.
pub fn grkit_section(app: AppState) -> Option<AnyElement> {
    if !app.camera_online() {
        return None;
    }
    let card = match app.camera_settings() {
        // While the camera answers, a row keeps the section's place, so
        // the page does not move when the settings come.
        SettingsRead::Unknown if app.camera_settings_loading() => {
            grouped_card(reading_row()).into_element()
        }
        SettingsRead::Unknown => return None,
        SettingsRead::Unsupported => grouped_card(update_row(app)).into_element(),
        SettingsRead::Ready(settings) => grouped_card(settings_rows(app, &settings)).into_element(),
    };
    Some(
        view()
            .gap_2()
            .child(section_title(tr("grkit.title", &[])))
            .child(card)
            .into_element(),
    )
}

/// The settings being read: a spinner in a row of the section's height.
fn reading_row() -> impl IntoElement {
    view()
        .min_h(px(52.0))
        .px_4()
        .flex_row()
        .items_center()
        .label(tr("grkit.title", &[]))
        .accessibility_state(busy_state(true))
        .child(quiet_spinner(18.0))
}

/// A GRKit from before the phone could change its settings.
fn update_row(app: AppState) -> impl IntoElement {
    let button = view()
        .min_h(px(32.0))
        .px_4()
        .rounded_full()
        .items_center()
        .justify_center()
        .bg(move || token("button"))
        .role(AccessibilityRole::Button)
        .label(tr("software.update", &[]))
        .on_press(move || app.open_software_update())
        .active(|s| s.opacity(0.72))
        .child(
            text(tr("software.update", &[]))
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || token("button-foreground")),
        );
    control_row(tr("modules.unsupported_title", &[]), button, true)
}

/// A row of GRKit Settings: its title, its value, and what it opens.
type Row = (String, Option<String>, Box<dyn Fn()>);

fn settings_rows(app: AppState, settings: &CameraSettings) -> impl IntoElement {
    let mut rows: Vec<Row> = Vec::new();
    if let Some(start) = settings.start {
        rows.push((
            tr("grkit.start", &[]),
            Some(start_text(start)),
            Box::new(move || start_menu(app, start)),
        ));
    }
    if settings.switch(SettingSwitch::UncroppedRaw).is_some() {
        rows.push((
            tr("grkit.shooting", &[]),
            None,
            Box::new(move || app.push(Route::ShootingSettings)),
        ));
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
        rows.push((
            tr("grkit.display", &[]),
            None,
            Box::new(move || app.push(Route::DisplaySettings)),
        ));
    }
    if [SettingSwitch::CustomPicture, SettingSwitch::ShowText]
        .into_iter()
        .any(|s| settings.switch(s).is_some())
    {
        rows.push((
            tr("grkit.shutdown", &[]),
            None,
            Box::new(move || app.push(Route::ShutdownScreen)),
        ));
    }
    let last = rows.len().saturating_sub(1);
    view().children(
        rows.into_iter()
            .enumerate()
            .map(|(index, (title, value, open))| nav_row(title, value, index == last, open)),
    )
}

/// A row that opens a page or a choice, with its value before the chevron.
fn nav_row(
    title: String,
    value: Option<String>,
    last: bool,
    on_press: Box<dyn Fn()>,
) -> AnyElement {
    let label = match &value {
        Some(value) => format!("{title}, {value}"),
        None => title.clone(),
    };
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .min_h(px(50.0))
        .px_4()
        .style_when(!last, |st| {
            st.border_b_1().border_b_color(move || theme().border)
        })
        .role(AccessibilityRole::Button)
        .label(label)
        .on_press(on_press)
        .active(|st| st.bg(move || theme().accent))
        .child(
            text(title)
                .text_size(px(crate::ui::ty::BODY))
                .flex_shrink(1.0),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(value.map(|value| {
                    text(value)
                        .text_size(px(crate::ui::ty::BODY))
                        .text_color(move || theme().muted_foreground)
                }))
                .child(crate::ui::chevron()),
        )
        .into_element()
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
) {
    spawn(async move {
        let mut menu = crate::ui::held_menu(title);
        for (index, (value, label)) in values.iter().enumerate() {
            let label = if *value == current {
                format!("✓ {label}")
            } else {
                label.clone()
            };
            menu = menu.action(index.to_string(), label);
        }
        let answer = menu.cancel("cancel", tr("looks.cancel", &[])).show().await;
        let chosen = answer
            .ok()
            .and_then(|a| a.action)
            .and_then(|action| action.parse::<usize>().ok())
            .and_then(|index| values.get(index).map(|(value, _)| *value));
        if let Some(value) = chosen.filter(|value| *value != current) {
            pick(value);
        }
    });
}

fn start_menu(app: AppState, current: StartMode) {
    let values = StartMode::ALL
        .into_iter()
        .map(|mode| (mode, start_text(mode)))
        .collect();
    choose(tr("grkit.start", &[]), values, current, move |mode| {
        app.set_camera_setting(SettingChange::Start(mode))
    });
}

/// A switch row of a settings page.
fn switch_row(app: AppState, switch: SettingSwitch, title: &str, last: bool) -> Option<AnyElement> {
    let SettingsRead::Ready(settings) = app.camera_settings() else {
        return None;
    };
    let on = settings.switch(switch)?;
    let saving =
        app.camera_setting_busy(|c| matches!(c, SettingChange::Switch(s, _) if s == switch));
    let title = tr(title, &[]);
    let toggle = toggle_loading(on, title.clone(), true, saving, move || {
        app.set_camera_setting(SettingChange::Switch(switch, !on))
    });
    Some(control_row(title, toggle, last).into_element())
}

fn unit_row(app: AppState, last: bool) -> Option<AnyElement> {
    let SettingsRead::Ready(settings) = app.camera_settings() else {
        return None;
    };
    let unit = settings.unit?;
    let open = Box::new(move || {
        let values = DistanceUnit::ALL
            .into_iter()
            .map(|unit| (unit, unit_text(unit)))
            .collect();
        choose(tr("grkit.unit", &[]), values, unit, move |unit| {
            app.set_camera_setting(SettingChange::Unit(unit))
        });
    });
    Some(nav_row(
        tr("grkit.unit", &[]),
        Some(unit_text(unit)),
        last,
        open,
    ))
}

/// A page of switches and choices, which follows the camera's settings.
fn settings_page(
    app: AppState,
    title: &'static str,
    rows: impl Fn(AppState) -> Vec<AnyElement> + 'static,
    on_dismiss: impl Fn() + 'static,
) -> impl IntoElement {
    crate::ui::app_page()
        .title(move || tr(title, &[]))
        .on_dismiss(on_dismiss)
        .child(
            crate::ui::page_scroll()
                .refresh(Some(crate::store::page_refresh::SETTINGS.to_owned()))
                .flex_1()
                .bg(move || token("grouped"))
                .child(
                    view()
                        .pt_4()
                        .px_6()
                        .pb(px(crate::pages::look_pages::TAB_BAR_CLEARANCE))
                        .child(dynamic(move || {
                            let rows = rows(app);
                            (!rows.is_empty()).then(|| grouped_card(view().children(rows)))
                        })),
                ),
        )
}

pub fn shooting_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    settings_page(
        app,
        "grkit.shooting",
        |app| {
            switch_row(app, SettingSwitch::UncroppedRaw, "grkit.raw", true)
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
                unit_row(app, false),
                switch_row(app, SettingSwitch::SimpleBattery, "grkit.battery", false),
                switch_row(
                    app,
                    SettingSwitch::BatteryPercent,
                    "grkit.battery_percent",
                    false,
                ),
                switch_row(
                    app,
                    SettingSwitch::PlaybackIcons,
                    "grkit.playback_icons",
                    true,
                ),
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
        .title(move || tr("grkit.shutdown", &[]))
        .on_dismiss(on_dismiss)
        .child(
            crate::ui::page_scroll()
                .refresh(Some(crate::store::page_refresh::SETTINGS.to_owned()))
                .flex_1()
                .bg(move || token("grouped"))
                .child(
                    view()
                        .gap_5()
                        .pt_4()
                        .px_6()
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
                            let rows: Vec<AnyElement> = [
                                has_picture
                                    .then(|| {
                                        switch_row(
                                            app,
                                            SettingSwitch::CustomPicture,
                                            "grkit.custom_picture",
                                            false,
                                        )
                                    })
                                    .flatten(),
                                switch_row(app, SettingSwitch::ShowText, "grkit.show_text", true),
                            ]
                            .into_iter()
                            .flatten()
                            .collect();
                            (!rows.is_empty()).then(|| grouped_card(view().children(rows)))
                        }))
                        .child(dynamic(move || picture_buttons(app))),
                ),
        )
}

/// The line that says the camera has the new picture.
fn sent_notice(title: String) -> AnyElement {
    view()
        .p_4()
        .rounded(px(10.0))
        .bg(move || theme().muted)
        .child(
            text(title)
                .text_size(px(crate::ui::ty::SUBHEADLINE))
                .font_weight(FontWeight::SEMIBOLD),
        )
        .into_element()
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
            .child(crate::ui::primary_button(
                tr(
                    if matches!(picture, PictureRead::Picture(_)) {
                        "grkit.change_photo"
                    } else {
                        "grkit.choose_photo"
                    },
                    &[],
                ),
                move || app.choose_shutdown_picture(),
            ))
            .child(matches!(picture, PictureRead::Picture(_)).then(|| {
                crate::ui::secondary_button_loading(
                    tr("grkit.restore_picture", &[]),
                    busy,
                    move || app.save_shutdown_picture(false, || {}),
                )
            }))
            .into_element(),
    )
}

/// The picture made from the picked photo, and the button that sends it.
pub fn crop_page(app: AppState, on_dismiss: impl Fn() + Clone + 'static) -> impl IntoElement {
    let done = on_dismiss.clone();
    crate::ui::app_page()
        .title(move || tr("grkit.crop", &[]))
        .on_dismiss(on_dismiss)
        .child(
            view()
                .flex_1()
                .bg(move || token("grouped"))
                .pt_4()
                .px_6()
                .gap_5()
                .child(dynamic(move || app.shutdown_draft().map(picture_view)))
                .child(dynamic(move || {
                    let done = done.clone();
                    crate::ui::primary_button_loading(
                        tr("looks.send", &[]),
                        app.shutdown_picture_busy(),
                        move || app.save_shutdown_picture(true, done.clone()),
                    )
                })),
        )
}
