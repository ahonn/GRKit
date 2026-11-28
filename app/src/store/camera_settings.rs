//! The GRKit settings of the camera's MENU, which the Settings tab shows
//! while a camera is connected. A change applies on the camera at once. A new
//! picture of Shutdown Screen shows at the camera's next power-off.

use nsh_core::prelude::*;

use super::AppState;
use crate::domain::LookError;
use std::rc::Rc;

use crate::domain::camera_settings::{CameraSettings, SettingChange, SettingSwitch};
use crate::domain::images::jpeg_data_uri;
use crate::domain::shutdown_picture::{Crop, MAX_SIDE, Photo, PhotoError};
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

/// A picked photo for the picture of Shutdown Screen, before it goes to the
/// camera.
#[derive(Clone)]
pub struct PictureDraft {
    pub photo: Rc<Photo>,
    /// A `data:` URI of the photo, for the crop page.
    pub uri: String,
}

impl PartialEq for PictureDraft {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.photo, &other.photo)
    }
}

#[derive(Clone, Copy)]
pub struct CameraSettingsStore {
    read: Signal<SettingsRead>,
    loading: Signal<bool>,
    /// The change being saved.
    busy: Signal<Option<SettingChange>>,
    picture: Signal<PictureRead>,
    draft: Signal<Option<PictureDraft>>,
    /// The part of the draft that the crop page shows; `None`: its middle.
    crop: Signal<Option<Crop>>,
    /// The phone reads a picked photo.
    picture_reading: Signal<bool>,
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
            crop: signal(None),
            picture_reading: signal(false),
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
        if !self.camera_online() || self.camera_settings.loading.get() {
            return;
        }
        let generation = self.connect_generation.get();
        self.camera_settings.loading.set(true);
        let read = self.api().camera_settings();
        self.run(async move {
            let result = read.await;
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
        self.camera_settings.busy.set(Some(change));
        let generation = self.connect_generation.get();
        let answer = self
            .api()
            .change_camera_setting(&self.modules_boot_id(), change);
        self.run(async move {
            let answer = answer.await;
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

    /// The picked photo, for the crop page.
    pub fn shutdown_draft(self) -> Option<PictureDraft> {
        self.camera_settings.draft.get()
    }

    /// The crop page shows `crop` of the draft.
    pub fn set_shutdown_crop(self, crop: Crop) {
        self.camera_settings.crop.set(Some(crop));
    }

    pub fn shutdown_photo_reading(self) -> bool {
        self.camera_settings.picture_reading.get()
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
        if !self.camera_online() {
            return;
        }
        let generation = self.connect_generation.get();
        let api = self.api();
        self.run(async move {
            let read = match api.shutdown_picture_writable().await {
                Ok(false) => Ok(PictureRead::Unsupported),
                Ok(true) => api.shutdown_picture().await.map(|bytes| {
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
    }

    /// Pick a photo and read it off the engine thread; the crop page then
    /// shows it.
    pub fn choose_shutdown_picture(self) {
        if self.camera_settings.picture_reading.get() {
            return;
        }
        self.root.run(|| {
            spawn(async move {
                // The picker scales a large photo down, with its orientation
                // applied, and turns HEIC into JPEG.
                let mut request = nsh_core::picker::PickRequest::new(false, 1);
                request.max_dimension = Some(DevicePixels(MAX_SIDE as i32));
                let picked = pick_images(request).await;
                let Some(file) = picked.into_iter().next() else {
                    return;
                };
                self.camera_settings.picture_reading.set(true);
                let bytes = file.data.bytes().to_vec();
                let read = blocking(move || {
                    Photo::decode(&bytes).map(|photo| {
                        let preview = photo.preview();
                        (photo, preview)
                    })
                })
                .await
                .unwrap_or(Err(PhotoError::Unreadable));
                self.camera_settings.picture_reading.set(false);
                match read {
                    Ok((photo, preview)) => {
                        let Some(uri) = jpeg_data_uri(&preview) else {
                            self.show_toast(tr("grkit.picture_failed", &[]));
                            return;
                        };
                        self.camera_settings.draft.set(Some(PictureDraft {
                            photo: Rc::new(photo),
                            uri,
                        }));
                        self.camera_settings.crop.set(None);
                        self.push(super::Route::ShutdownCrop);
                    }
                    Err(PhotoError::Format) => self.show_toast(tr("grkit.picture_format", &[])),
                    Err(PhotoError::Unreadable) => self.show_toast(tr("grkit.picture_failed", &[])),
                }
            });
        });
    }

    /// Send the shown part of the picked photo to the camera, or remove the
    /// card's picture (`send: false`). `done` runs once the camera has the
    /// change. A sent picture turns Custom Picture on, so that the camera
    /// shows it.
    pub fn save_shutdown_picture(self, send: bool, done: impl Fn() + 'static) {
        if self.camera_settings.picture_busy.get() {
            return;
        }
        let jpeg = if send {
            let Some(draft) = self.camera_settings.draft.get() else {
                return;
            };
            let photo = &draft.photo;
            let crop = self
                .camera_settings
                .crop
                .get()
                .unwrap_or_else(|| Crop::middle(photo.width, photo.height));
            match photo.picture(crop) {
                Ok(jpeg) => Some(jpeg),
                Err(error) => {
                    log::warn!("Could not make the picture: {error}");
                    self.show_toast(tr("grkit.picture_failed", &[]));
                    return;
                }
            }
        } else {
            None
        };
        let shown = match jpeg.as_deref().and_then(jpeg_data_uri) {
            Some(uri) => PictureRead::Picture(uri),
            None => PictureRead::None,
        };
        self.camera_settings.picture_busy.set(true);
        let generation = self.connect_generation.get();
        let answer = self
            .api()
            .put_shutdown_picture(&self.modules_boot_id(), jpeg);
        self.run(async move {
            let answer = answer.await;
            if self.connect_generation.get() != generation {
                return;
            }
            self.camera_settings.picture_busy.set(false);
            match answer {
                Ok(()) => {
                    self.camera_settings.picture.set(shown);
                    self.camera_settings.picture_sent.set(send);
                    self.camera_settings.draft.set(None);
                    let custom_off = matches!(
                        self.camera_settings.read.get(),
                        SettingsRead::Ready(settings)
                            if settings.switch(SettingSwitch::CustomPicture) == Some(false)
                    );
                    if send && custom_off {
                        self.set_camera_setting(SettingChange::Switch(
                            SettingSwitch::CustomPicture,
                            true,
                        ));
                    }
                    done();
                }
                Err(error) if error != LookError::ConnectionLost => {
                    self.show_toast(tr("grkit.save_failed", &[]));
                }
                Err(_) => {}
            }
        });
    }
}

/// Demo screens (`dev::screens`).
#[cfg(demo)]
impl AppState {
    /// `photo` is picked, and the crop page shows its middle.
    pub(crate) fn seed_shutdown_draft(self, photo: Photo, uri: String) {
        self.camera_settings.draft.set(Some(PictureDraft {
            photo: Rc::new(photo),
            uri,
        }));
        self.camera_settings.crop.set(None);
    }

    /// The card has the picture `uri`, which went to the camera in this
    /// visit of the page.
    pub(crate) fn seed_shutdown_sent(self, uri: String) {
        self.camera_settings.picture.set(PictureRead::Picture(uri));
        self.camera_settings.picture_sent.set(true);
    }
}
