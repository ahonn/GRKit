//! The GRKit settings of the camera's MENU, which the Settings tab shows
//! while a camera is connected. A change applies on the camera at once. The
//! picture of Shutdown Screen applies at the camera's next start.

use nsh_core::prelude::*;

use super::AppState;
use crate::domain::LookError;
use crate::domain::camera_settings::{CameraSettings, SettingChange, demo_settings};
use crate::domain::modules::jpeg_data_uri;
use crate::domain::shutdown_picture::shutdown_picture;
use crate::platform::modules as remote;
use crate::strings::tr;

/// What the phone knows of the camera's GRKit settings.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingsRead {
    /// Not read in this connection.
    Unknown,
    /// The camera's GRKit is from before the phone could change them.
    Unsupported,
    Ready(CameraSettings),
}

/// The picture of Shutdown Screen on the card.
#[derive(Clone, Debug, PartialEq)]
pub enum PictureRead {
    Unknown,
    /// The camera's GRKit does not take a picture from the phone.
    Unsupported,
    /// The camera shows its own picture.
    None,
    /// A `data:` URI of the card's picture.
    Picture(String),
}

#[derive(Clone, Copy)]
pub struct CameraSettingsStore {
    read: Signal<SettingsRead>,
    loading: Signal<bool>,
    /// The change being saved.
    busy: Signal<Option<SettingChange>>,
    picture: Signal<PictureRead>,
    /// The file made from a picked photo, before it goes to the camera.
    draft: Signal<Option<Vec<u8>>>,
    picture_busy: Signal<bool>,
    /// The picture went to the camera in this visit of the page.
    picture_sent: Signal<bool>,
}

impl CameraSettingsStore {
    pub fn new() -> Self {
        Self {
            read: signal(SettingsRead::Unknown),
            loading: signal(false),
            busy: signal(None),
            picture: signal(PictureRead::Unknown),
            draft: signal(None),
            picture_busy: signal(false),
            picture_sent: signal(false),
        }
    }
}

impl AppState {
    pub fn camera_settings(self) -> SettingsRead {
        self.camera_settings.read.get()
    }

    pub fn camera_settings_loading(self) -> bool {
        self.camera_settings.loading.get()
    }

    /// A change of a setting like `change` is being saved.
    pub fn camera_setting_busy(self, same: impl Fn(SettingChange) -> bool) -> bool {
        self.camera_settings.busy.get().is_some_and(same)
    }

    /// Read the settings of the connected camera.
    pub fn refresh_camera_settings(self) {
        if self.demo() {
            if self.camera_online() {
                self.camera_settings
                    .read
                    .set(SettingsRead::Ready(demo_settings()));
            }
            return;
        }
        if !self.camera_online() || self.camera_settings.loading.get() {
            return;
        }
        let generation = self.connect_generation.get();
        self.camera_settings.loading.set(true);
        self.root.run(|| {
            spawn(async move {
                let result = remote::camera_settings().await;
                if self.connect_generation.get() != generation {
                    return;
                }
                self.camera_settings.loading.set(false);
                match result {
                    Ok(Some(settings)) if self.camera_settings.busy.get().is_none() => {
                        self.camera_settings.read.set(SettingsRead::Ready(settings));
                    }
                    Ok(Some(_)) => {}
                    Ok(None) => self.camera_settings.read.set(SettingsRead::Unsupported),
                    Err(error) => log::warn!("Could not read the GRKit settings: {error:?}"),
                }
            });
        });
    }

    /// Change one setting. It shows at once, and goes back if the camera
    /// refuses it.
    pub fn set_camera_setting(self, change: SettingChange) {
        let SettingsRead::Ready(before) = self.camera_settings.read.get() else {
            return;
        };
        if self.camera_settings.busy.get().is_some() {
            return;
        }
        self.camera_settings
            .read
            .set(SettingsRead::Ready(before.with(change)));
        if self.demo() {
            return;
        }
        self.camera_settings.busy.set(Some(change));
        let boot_id = self.modules_boot_id();
        let generation = self.connect_generation.get();
        self.root.run(|| {
            spawn(async move {
                let answer = remote::patch_camera_setting(&boot_id, change).await;
                if self.connect_generation.get() != generation {
                    return;
                }
                self.camera_settings.busy.set(None);
                match answer {
                    Ok(settings) => self.camera_settings.read.set(SettingsRead::Ready(settings)),
                    Err(error) => {
                        self.camera_settings.read.set(SettingsRead::Ready(before));
                        if error != LookError::ConnectionLost {
                            self.show_toast(tr("grkit.save_failed", &[]));
                        }
                    }
                }
            });
        });
    }

    /// A lost link: the next connection reads the settings again.
    pub(super) fn reset_camera_settings(self) {
        self.camera_settings.read.set(SettingsRead::Unknown);
        self.camera_settings.loading.set(false);
        self.camera_settings.busy.set(None);
        self.camera_settings.picture.set(PictureRead::Unknown);
        self.camera_settings.picture_busy.set(false);
    }

    pub fn shutdown_picture(self) -> PictureRead {
        self.camera_settings.picture.get()
    }

    /// The picture made from the picked photo, as a `data:` URI.
    pub fn shutdown_draft(self) -> Option<String> {
        self.camera_settings
            .draft
            .with(|draft| draft.as_deref().and_then(jpeg_data_uri))
    }

    pub fn shutdown_picture_busy(self) -> bool {
        self.camera_settings.picture_busy.get()
    }

    pub fn shutdown_picture_sent(self) -> bool {
        self.camera_settings.picture_sent.get()
    }

    /// Read the card's picture, as the Shutdown Screen page opens.
    pub fn refresh_shutdown_picture(self) {
        self.camera_settings.picture_sent.set(false);
        if self.demo() {
            self.camera_settings.picture.set(PictureRead::None);
            return;
        }
        if !self.camera_online() {
            return;
        }
        let generation = self.connect_generation.get();
        self.root.run(|| {
            spawn(async move {
                let read = match remote::shutdown_picture_writable().await {
                    Ok(false) => Ok(PictureRead::Unsupported),
                    Ok(true) => remote::shutdown_picture().await.map(|bytes| {
                        match bytes.as_deref().and_then(jpeg_data_uri) {
                            Some(uri) => PictureRead::Picture(uri),
                            None => PictureRead::None,
                        }
                    }),
                    Err(error) => Err(error),
                };
                if self.connect_generation.get() != generation {
                    return;
                }
                match read {
                    Ok(read) => self.camera_settings.picture.set(read),
                    Err(error) => log::warn!("Could not read the shutdown picture: {error:?}"),
                }
            });
        });
    }

    /// Pick a photo and make the picture from its middle; the crop page then
    /// shows it.
    pub fn choose_shutdown_picture(self) {
        self.root.run(|| {
            spawn(async move {
                let picked = pick_images(nsh_core::picker::PickRequest::new(false, 1)).await;
                let Some(file) = picked.into_iter().next() else {
                    return;
                };
                match shutdown_picture(file.data.bytes()) {
                    Ok(jpeg) => {
                        self.camera_settings.draft.set(Some(jpeg));
                        self.push(super::Route::ShutdownCrop);
                    }
                    Err(error) => {
                        log::warn!("Could not use the photo: {error}");
                        self.show_toast(tr("grkit.picture_failed", &[]));
                    }
                }
            });
        });
    }

    /// Send the picked picture to the camera, or remove the card's picture
    /// (`send: false`). `done` runs once the camera has the change.
    pub fn save_shutdown_picture(self, send: bool, done: impl Fn() + 'static) {
        if self.camera_settings.picture_busy.get() {
            return;
        }
        let jpeg = if send {
            match self.camera_settings.draft.get() {
                Some(jpeg) => Some(jpeg),
                None => return,
            }
        } else {
            None
        };
        let shown = match jpeg.as_deref().and_then(jpeg_data_uri) {
            Some(uri) => PictureRead::Picture(uri),
            None => PictureRead::None,
        };
        if self.demo() {
            self.camera_settings.picture.set(shown);
            self.camera_settings.picture_sent.set(send);
            done();
            return;
        }
        self.camera_settings.picture_busy.set(true);
        let boot_id = self.modules_boot_id();
        let generation = self.connect_generation.get();
        self.root.run(|| {
            spawn(async move {
                let answer = remote::put_shutdown_picture(&boot_id, jpeg).await;
                if self.connect_generation.get() != generation {
                    return;
                }
                self.camera_settings.picture_busy.set(false);
                match answer {
                    Ok(()) => {
                        self.camera_settings.picture.set(shown);
                        self.camera_settings.picture_sent.set(send);
                        self.camera_settings.draft.set(None);
                        done();
                    }
                    Err(error) if error != LookError::ConnectionLost => {
                        self.show_toast(tr("grkit.save_failed", &[]));
                    }
                    Err(_) => {}
                }
            });
        });
    }
}
