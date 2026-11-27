//! GRKit Settings in the Settings tab, in the order and with the names of
//! the camera's MENU: Enable at Start, then the pages Shooting Settings,
//! Display Settings, and Shutdown Screen. It shows only with a connected
//! camera.

use nsh_core::prelude::*;

use crate::domain::camera_settings::{
    CameraSettings, DistanceUnit, SettingChange, SettingSwitch, StartMode,
};
use crate::domain::shutdown_picture::Crop;
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

/// A page of switches and choices, which follows the camera's settings,
/// with a `footer` under them that says what they do.
fn settings_page(
    app: AppState,
    title: &'static str,
    rows: impl Fn(AppState) -> Vec<ListRow> + 'static,
    footer: Option<&'static str>,
    on_dismiss: impl Fn() + 'static,
) -> impl IntoElement {
    crate::ui::app_page()
        .page_title(move || tr(title, &[]), false)
        .on_dismiss(on_dismiss)
        .child(
            crate::pages::page_scroll()
                .refresh(Some(crate::store::page_refresh::SETTINGS.to_owned()))
                .flex_1()
                .bg(move || token("list-background"))
                .child(
                    view()
                        .pt_4()
                        .px_4()
                        .pb(px(crate::ui::TAB_BAR_CLEARANCE))
                        .child(dynamic(move || {
                            let rows = rows(app);
                            (!rows.is_empty()).then(|| {
                                let section = list_section(rows);
                                match footer {
                                    Some(footer) => section.footer(tr(footer, &[])),
                                    None => section,
                                }
                            })
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
        Some("grkit.raw_note"),
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
        None,
        on_dismiss,
    )
}

/// Shutdown Screen: what the camera shows at power-off, its switches, and
/// the photo that replaces the picture of the card.
pub fn shutdown_page(app: AppState, on_dismiss: impl Fn() + 'static) -> impl IntoElement {
    app.refresh_shutdown_picture();
    let frame = signal(0.0f32);
    crate::ui::app_page()
        .page_title(move || tr("grkit.shutdown", &[]), false)
        .on_dismiss(on_dismiss)
        .child(
            crate::pages::page_scroll()
                .refresh(Some(crate::store::page_refresh::SETTINGS.to_owned()))
                .flex_1()
                .bg(move || token("list-background"))
                .child(
                    view()
                        .gap_5()
                        .pt_4()
                        .px_4()
                        .pb(px(crate::ui::TAB_BAR_CLEARANCE))
                        .child(dynamic(move || {
                            app.shutdown_picture_sent()
                                .then(|| sent_notice(tr("grkit.picture_sent", &[])))
                        }))
                        .child(dynamic(move || picture_preview(app, frame)))
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
                            (!rows.is_empty())
                                .then(|| list_section(rows).footer(tr("grkit.show_text_note", &[])))
                        }))
                        .child(dynamic(move || picture_buttons(app))),
                ),
        )
}

/// The line that says the camera has the new picture.
fn sent_notice(title: String) -> AnyElement {
    crate::ui::notice_card(title).into_element()
}

/// The value of a switch of the camera, when it has read and has it.
fn camera_switch(app: AppState, switch: SettingSwitch) -> Option<bool> {
    match app.camera_settings() {
        SettingsRead::Ready(settings) => settings.switch(switch),
        _ => None,
    }
}

/// A frame at the ratio of the camera's screen around `content`, with the
/// camera's text over it while Show Text is on. `frame` keeps its width,
/// which sizes the text.
fn screen_frame(app: AppState, frame: Signal<f32>, content: AnyElement) -> AnyElement {
    view()
        .w_full()
        .aspect_ratio(3.0 / 2.0)
        .rounded(px(8.0))
        .overflow_hidden()
        .bg(rgb(0x0B0B0B))
        .relative()
        .on_layout(move |bounds: &Bounds<Points>| frame.set(bounds.size.width.0))
        .child(content)
        .child(dynamic(move || {
            (camera_switch(app, SettingSwitch::ShowText) == Some(true))
                .then(|| camera_text(app, frame.get()))
                .flatten()
        }))
        .into_element()
}

/// The text that the camera draws over the picture, at its places on the
/// screen of 720 by 480: the shot counts on a dark bar, and the firmware
/// version. An approximation: the counts are placeholders.
fn camera_text(app: AppState, width: f32) -> Option<AnyElement> {
    if width <= 0.0 {
        return None;
    }
    let s = width / 720.0;
    let version = app.package().and_then(|p| firmware_number(&p.firmware));
    let label = |words: String| {
        text(words)
            .text_size(px(24.0 * s))
            .text_color(rgb(0xFFFFFF))
    };
    Some(
        view()
            .absolute()
            .left(px(0.0))
            .top(px(0.0))
            .size_full()
            .pointer_events(PointerEvents::None)
            .child(
                view()
                    .absolute()
                    .left(px(20.0 * s))
                    .top(px(340.0 * s))
                    .w(px(680.0 * s))
                    .h(px(40.0 * s))
                    .bg(rgba(0x00000073))
                    .items_center()
                    .justify_center()
                    .child(label("Today : 0000000 > Total: 0000000".to_owned())),
            )
            .child(version.map(|version| {
                view()
                    .absolute()
                    .right(px(10.0 * s))
                    .bottom(px(14.0 * s))
                    .child(label(format!("FW Ver.{version}")))
            }))
            .into_element(),
    )
}

/// The version number in the firmware name of the camera (`2.21`).
fn firmware_number(firmware: &str) -> Option<String> {
    let start = firmware.find(|c: char| c.is_ascii_digit())?;
    let rest = &firmware[start..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(rest.len());
    let number = rest[..end].trim_end_matches('.');
    number.contains('.').then(|| number.to_owned())
}

/// A picture that fills the frame.
fn picture_image(uri: String) -> AnyElement {
    image(nsh_core::element::ImageSrc::from(uri))
        .size_full()
        .object_fit(ResizeMode::Cover)
        .into_element()
}

/// What the camera shows at power-off: the card's picture while Custom
/// Picture is on, or else the camera's own, which the phone cannot read and
/// names instead.
fn picture_preview(app: AppState, frame: Signal<f32>) -> Option<AnyElement> {
    let custom = camera_switch(app, SettingSwitch::CustomPicture) != Some(false);
    let content = match app.shutdown_picture() {
        PictureRead::Picture(uri) if custom => picture_image(uri),
        PictureRead::Picture(_) | PictureRead::None => default_picture(),
        PictureRead::Unknown | PictureRead::Unsupported => return None,
    };
    Some(screen_frame(app, frame, content))
}

/// The name of the camera's own picture, in the middle of the frame.
fn default_picture() -> AnyElement {
    view()
        .size_full()
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
            .child(
                crate::ui::button(
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
                )
                .loading(app.shutdown_photo_reading()),
            )
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

/// The picked photo in the frame of the screen: dragging and pinching pick
/// the part that goes to the camera, and the button sends it.
pub fn crop_page(app: AppState, on_dismiss: impl Fn() + Clone + 'static) -> impl IntoElement {
    let done = on_dismiss.clone();
    let frame = signal(0.0f32);
    crate::ui::app_page()
        .page_title(move || tr("grkit.crop", &[]), false)
        .on_dismiss(on_dismiss)
        .child(
            view()
                .flex_1()
                .bg(move || token("list-background"))
                .pt_4()
                .px_4()
                .gap_3()
                .child(screen_frame(
                    app,
                    frame,
                    dynamic(move || crop_view(app, frame.get())).into_element(),
                ))
                .child(
                    text(tr("grkit.crop_hint", &[]))
                        .text_size(px(crate::ui::ty::FOOTNOTE))
                        .text_color(move || token("list-secondary")),
                )
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

/// The photo in a scroll view that zooms, at the size that covers a frame
/// `width` points wide, starting at its middle (`Crop::middle`).
fn crop_view(app: AppState, width: f32) -> Option<AnyElement> {
    let draft = app.shutdown_draft()?;
    if width <= 0.0 {
        return None;
    }
    let height = width * 2.0 / 3.0;
    let (photo_w, photo_h) = (draft.photo.width as f32, draft.photo.height as f32);
    let wide = photo_w * height > photo_h * width;
    let (content_w, content_h) = if wide {
        (height * photo_w / photo_h, height)
    } else {
        (width, width * photo_h / photo_w)
    };
    let start = Point {
        x: Points((content_w - width) / 2.0),
        y: Points((content_h - height) / 2.0),
    };
    let mut view = scroll()
        .size_full()
        .zoom_range(1.0, 4.0)
        .scrollbar(false)
        .content_offset(start)
        .on_scroll(move |event: &ScrollEvent| {
            let crop = Crop::shown(
                (event.offset.x.0, event.offset.y.0),
                (event.content_size.width.0, event.content_size.height.0),
                (event.size.width.0, event.size.height.0),
            );
            if let Some(crop) = crop {
                app.set_shutdown_crop(crop);
            }
        })
        .child(
            image(nsh_core::element::ImageSrc::from(draft.uri))
                .w(px(content_w))
                .h(px(content_h))
                .object_fit(ResizeMode::Cover),
        );
    if wide {
        view = view.horizontal();
    }
    Some(view.into_element())
}
