//! The film looks of GR IV, made on the phone from a GR IV firmware file that
//! the person downloaded from Ricoh (`grkit_core::gr4_looks`). The camera
//! gets them as Looks, which it lists after it starts again.

use grkit_core::cover::Cover;
use nsh_core::picker::FilePickRequest;
use nsh_core::prelude::*;

use super::looks::off_thread;
use super::{AppState, LookAction};
use crate::domain::{CameraModel, look_media};
use crate::platform::looks as remote;
use crate::strings::tr;

/// Ricoh's GR IV page, which gives the firmware with its license.
pub const GR4_FIRMWARE_PAGE: &str =
    "https://www.ricoh-imaging.co.jp/english/support/digital/gr4_s.html";
/// The largest file the picker takes: an official GR IV zip has about 35 MB.
const MAX_FIRMWARE_BYTES: u64 = 64 * 1024 * 1024;

/// A look made from the firmware file, ready for the camera.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeLook {
    pub id: String,
    pub name: String,
    pub cover: Cover,
    pub grf: Vec<u8>,
}

#[derive(Clone, Copy)]
pub struct NativeLooksStore {
    /// The looks of the read file, and the version of the firmware.
    found: Signal<Option<(String, Vec<NativeLook>)>>,
    reading: Signal<bool>,
    sending: Signal<bool>,
}

impl Default for NativeLooksStore {
    fn default() -> Self {
        Self {
            found: signal(None),
            reading: signal(false),
            sending: signal(false),
        }
    }
}

/// The GR IV looks that the app offers, with their covers: the color looks,
/// which have a native cover (`cover::native_cover`).
pub fn gr4_looks() -> Vec<(&'static str, &'static str, Cover)> {
    grkit_core::gr4_looks::LOOKS
        .iter()
        .filter_map(|look| {
            grkit_core::cover::native_cover(look.effect_id)
                .map(|cover| (look.effect_id, look.name, cover.film_box()))
        })
        .collect()
}

/// Read the looks of a GR IV firmware zip or firmware file.
fn read_gr4(bytes: &[u8]) -> Result<(String, Vec<NativeLook>), String> {
    let image = grkit_core::gr4_looks::Image::read(bytes).map_err(|e| e.to_string())?;
    let offered = gr4_looks();
    let mut looks = Vec::new();
    for definition in image.definitions().map_err(|e| e.to_string())? {
        let grf =
            grkit_core::effects::compile_definition(&definition).map_err(|e| e.to_string())?;
        let (effect, _) = grkit_core::effects::decode(&grf).map_err(|e| e.to_string())?;
        let Some((_, name, cover)) = offered.iter().find(|(id, _, _)| *id == effect.id) else {
            continue;
        };
        let grf = look_media::with_cover(&grf, cover).map_err(|e| e.to_string())?;
        looks.push(NativeLook {
            id: effect.id,
            name: (*name).to_owned(),
            cover: cover.clone(),
            grf,
        });
    }
    Ok((image.source.version.to_owned(), looks))
}

impl AppState {
    /// The GR IV looks suit this camera: GR IV has them built in, and their
    /// data follows the Standard of GR III and GR IIIx.
    pub fn gr4_looks_offered(self) -> bool {
        self.status().is_some_and(|status| {
            matches!(status.camera.model, CameraModel::Gr3 | CameraModel::Gr3x)
        })
    }

    pub fn native_looks_found(self) -> Option<(String, Vec<NativeLook>)> {
        self.native_looks.found.get()
    }

    pub fn native_looks_reading(self) -> bool {
        self.native_looks.reading.get()
    }

    pub fn native_looks_sending(self) -> bool {
        self.native_looks.sending.get()
    }

    /// Pick the firmware file and read its looks on a worker thread.
    pub fn choose_gr4_firmware(self) {
        if self.native_looks.reading.get() {
            return;
        }
        self.root.run(|| {
            spawn(async move {
                let request = FilePickRequest::new(false, 1).max_size(MAX_FIRMWARE_BYTES);
                let Some(file) = pick_files(request).await.into_iter().next() else {
                    return;
                };
                self.native_looks.reading.set(true);
                let bytes = file.data.bytes().to_vec();
                let read = off_thread(move || read_gr4(&bytes)).await;
                self.native_looks.reading.set(false);
                match read {
                    Some(Ok(found)) if !found.1.is_empty() => {
                        self.native_looks.found.set(Some(found));
                    }
                    other => {
                        if let Some(Err(error)) = other {
                            log::warn!("Could not read the firmware file: {error}");
                        }
                        self.show_toast(tr("looks.firmware_wrong", &[]));
                    }
                }
            });
        });
    }

    /// Send the looks of the read file to the camera, one after another.
    pub fn send_native_looks(self) {
        let Some((_, looks)) = self.native_looks.found.get() else {
            return;
        };
        if self.native_looks.sending.get() {
            return;
        }
        self.native_looks.sending.set(true);
        self.send_native_look(looks, 0);
    }

    fn send_native_look(self, looks: Vec<NativeLook>, index: usize) {
        let Some(look) = looks.get(index).cloned() else {
            self.native_looks.sending.set(false);
            self.native_looks.found.set(None);
            self.pop_to(super::Tab::Looks, 0);
            return;
        };
        let demo_grf = look.grf.clone();
        let demo_cover = look.cover.clone();
        self.change(
            look.id.clone(),
            LookAction::Add,
            move |fake| fake.put(&demo_grf, true, Some(demo_cover)),
            move |context| async move {
                remote::put_content(&context, &look.id, look.grf, Some(&look.cover)).await
            },
            move |app, ok| {
                if ok {
                    app.send_native_look(looks, index + 1);
                } else {
                    app.native_looks.sending.set(false);
                }
            },
        );
    }

    /// Leave the unlock without sending.
    pub fn discard_native_looks(self) {
        self.native_looks.found.set(None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gr4_looks_have_film_box_covers_and_a_wrong_file_reads_as_an_error() {
        let looks = gr4_looks();
        assert_eq!(looks.len(), 2);
        assert!(looks.iter().all(|(_, _, cover)| cover.fits()));
        assert!(read_gr4(b"not a firmware file").is_err());
    }
}
